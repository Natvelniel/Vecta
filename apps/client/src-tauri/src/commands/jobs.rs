use std::collections::HashMap;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, State};
use url::Url;
use uuid::Uuid;

pub(crate) fn sanitize_external_url(raw: Option<&str>) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = Url::parse(trimmed).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    if parsed.host_str().map(|h| h.trim().is_empty()).unwrap_or(true) {
        return None;
    }
    Some(parsed.into())
}

use crate::core::config::{
    MAX_KNOWN_SKILLS_CONTEXT, MAX_MANUAL_JOB_CHARS, MIN_MANUAL_JOB_CHARS,
};
use crate::core::db::NewJobRecord;
use crate::core::gemini::{
    ApplyKitPromptInput, BatchJobInput, EmbeddingTaskType, GapAnalysisPromptInput,
};
use crate::core::local_embedder::LOCAL_EMBEDDING_MODEL_ID;
use crate::core::local_matcher::{
    compute_local_keyword_relevance, extract_universal_job_metadata, infer_experience_level,
    infer_work_mode,
};
use crate::core::matching::{
    compute_hybrid_match, compute_local_hybrid_match, cosine_similarity, encode_embedding,
    format_job_embedding_text, to_match_upsert,
};
use crate::core::native_scraper::quality::{assess_job_quality, deduplicate_cross_portal};
use crate::core::worker::WorkerScrapeRequest;
use crate::error::{AppError, ErrorCode};
use crate::models::job::{
    AiMatchRunReport, ApplicationStatus, ApplyKitResult, EnrichmentStatus, GapAnalysis,
    JobExtractionResult, JobFilterInput, JobMatchDetail, JobMatchSummary, JobSource,
    RequirementType, ScrapeProgressEvent, ScrapeRunReport, ScrapeWarningItem, SearchPreferences,
};
use crate::state::AppState;

const AI_EXTRACT_BATCH_SIZE: usize = 5;

fn emit_progress(
    app: Option<&AppHandle>,
    stage: &str,
    message: impl Into<String>,
    current: u32,
    total: u32,
) {
    if let Some(handle) = app {
        let _ = handle.emit(
            "scrape-progress",
            ScrapeProgressEvent {
                stage: stage.to_string(),
                message: message.into(),
                current,
                total,
            },
        );
    }
}

fn compute_manual_external_id(url: Option<&str>, raw_text: &str) -> String {
    let mut hasher = Sha256::new();
    let payload = format!(
        "manual|{}|{}",
        url.unwrap_or("").trim().to_lowercase(),
        raw_text.trim()
    );
    hasher.update(payload.as_bytes());
    let digest = hasher.finalize();
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

pub async fn ingest_manual_job_impl(
    state: &AppState,
    user_id: String,
    title_hint: Option<String>,
    url: Option<String>,
    raw_text: String,
) -> Result<JobMatchDetail, AppError> {
    let trimmed_text = raw_text.trim();
    if trimmed_text.chars().count() < MIN_MANUAL_JOB_CHARS {
        return Err(AppError::validation_error(format!(
            "Teks deskripsi lowongan minimal {MIN_MANUAL_JOB_CHARS} karakter."
        )));
    }
    let capped_text: String = trimmed_text.chars().take(MAX_MANUAL_JOB_CHARS).collect();

    let profile = state
        .db
        .get_profile_for_matching(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;

    let clean_url = sanitize_external_url(url.as_deref());
    let external_id = compute_manual_external_id(clean_url.as_deref(), &capped_text);

    // Reuse existing manual job if identical text/URL was already ingested
    if let Some(existing_job_id) = state
        .db
        .job_exists_by_source_external_id(JobSource::Manual, &external_id)?
    {
        if let Some(job_rec) = state.db.load_job_for_matching(&existing_job_id)? {
            let computed = compute_hybrid_match(&profile, &job_rec);
            let upsert = to_match_upsert(&user_id, &existing_job_id, &computed)?;
            state.db.upsert_job_matches(&[upsert])?;
            return state.db.get_job_detail(&user_id, &existing_job_id);
        }
    }

    let api_key = state.require_api_key()?;
    let known_skills = state.db.get_top_canonical_skills(MAX_KNOWN_SKILLS_CONTEXT)?;
    let extracted = state
        .gemini
        .extract_job(&api_key, &capped_text, &known_skills)
        .await?;

    let title = title_hint
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| extracted.normalized_role.clone());

    let mandatory_names: Vec<String> = extracted
        .skills
        .iter()
        .filter(|s| s.requirement_type == RequirementType::Mandatory)
        .map(|s| s.canonical_name.clone())
        .collect();
    let nice_names: Vec<String> = extracted
        .skills
        .iter()
        .filter(|s| s.requirement_type == RequirementType::NiceToHave)
        .map(|s| s.canonical_name.clone())
        .collect();

    let embed_text = format_job_embedding_text(
        &extracted.normalized_role,
        extracted.experience_level,
        &title,
        &extracted.role_summary,
        &mandatory_names,
        &nice_names,
    );
    let embedding_vec = state
        .gemini
        .embed_text(&api_key, &embed_text, EmbeddingTaskType::RetrievalDocument)
        .await?;
    let embedding_bytes = encode_embedding(&embedding_vec);

    let extracted_json = serde_json::to_string(&extracted)
        .map_err(|e| AppError::db_error(format!("Gagal serialisasi hasil ekstraksi: {e}")))?;

    let job_id = Uuid::new_v4().to_string();
    let quality = assess_job_quality(&title, "Manual Input", &capped_text, None, clean_url.as_deref());
    let red_flags_json = serde_json::to_string(&quality.red_flags).unwrap_or_else(|_| "[]".into());
    let tax_probe = vecta_taxonomy::TaxonomyEngine::global().extract_job(&title, &capped_text, None);
    let new_job = NewJobRecord {
        id: job_id.clone(),
        source: JobSource::Manual,
        external_id,
        url: clean_url,
        title,
        normalized_role: extracted.normalized_role.clone(),
        company: "Manual Input".to_string(),
        location: None,
        work_mode: extracted.work_mode,
        experience_level: extracted.experience_level,
        raw_description: capped_text,
        extracted_summary_json: Some(extracted_json),
        embedding: Some(embedding_bytes),
        enrichment_status: EnrichmentStatus::AiEnriched,
        posted_at: None,
        salary_text: None,
        domain_slug: tax_probe.dominant_domain,
        quality_score: quality.quality_score,
        red_flags_json,
        also_available_on_json: "[]".into(),
        embedding_model: Some("gemini:embedding-001".into()),
    };

    let job_match_rec = state.db.insert_job_with_skills(&new_job, &extracted.skills)?;
    let computed = compute_hybrid_match(&profile, &job_match_rec);
    let upsert = to_match_upsert(&user_id, &job_id, &computed)?;
    state.db.upsert_job_matches(&[upsert])?;

    state.db.get_job_detail(&user_id, &job_id)
}

pub async fn run_scrape_for_active_keywords_impl(
    state: &AppState,
    user_id: String,
    sources: Vec<String>,
    limit_per_query: u32,
) -> Result<ScrapeRunReport, AppError> {
    let api_key = state.require_api_key()?;
    let profile = state
        .db
        .get_profile_for_matching(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;

    let queries = state.db.list_active_scrape_queries(&user_id)?;
    if queries.is_empty() {
        return Err(AppError::validation_error(
            "Tidak ada keyword scraping aktif. Aktifkan minimal satu peran terlebih dahulu.",
        ));
    }

    let clean_sources: Vec<String> = sources
        .into_iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if clean_sources.is_empty() {
        return Err(AppError::validation_error(
            "Pilih minimal satu sumber portal lowongan.",
        ));
    }
    for src in &clean_sources {
        if !matches!(src.as_str(), "linkedin" | "glints" | "jobstreet" | "indeed") {
            return Err(AppError::validation_error(format!(
                "Sumber portal '{src}' tidak didukung. Gunakan: jobstreet, linkedin, indeed, atau glints."
            )));
        }
    }

    let _scrape_guard = state.try_acquire_scrape_guard()?;
    let worker_url = state.get_worker_url();

    let worker_req = WorkerScrapeRequest {
        queries,
        sources: clean_sources,
        location: "Indonesia".to_string(),
        limit_per_query: limit_per_query.clamp(1, 50),
        max_pages: 1,
        posted_within_days: None,
        work_mode: "any".to_string(),
    };
    let worker_resp = state.worker.scrape(&worker_url, &worker_req).await?;

    let total_fetched = worker_resp.jobs.len() as u32;
    let mut newly_ingested = 0u32;
    let mut skipped_duplicates = 0u32;
    let mut failed_extractions = 0u32;
    let mut warnings = worker_resp.warnings;
    let mut gemini_calls_made = 0u32;

    for scraped_job in worker_resp.jobs {
        let source = JobSource::from_db_str(&scraped_job.source);
        if let Some(existing_job_id) = state
            .db
            .job_exists_by_source_external_id(source, &scraped_job.external_id)?
        {
            skipped_duplicates += 1;
            // Ensure current user has a local match score for existing job (0 Gemini calls)
            if let Some(job_rec) = state.db.load_job_for_matching(&existing_job_id)? {
                let computed = compute_hybrid_match(&profile, &job_rec);
                let upsert = to_match_upsert(&user_id, &existing_job_id, &computed)?;
                state.db.upsert_job_matches(&[upsert])?;
            }
            continue;
        }

        if state.scrape_pacing_ms > 0 && gemini_calls_made > 0 {
            tokio::time::sleep(Duration::from_millis(state.scrape_pacing_ms)).await;
        }
        gemini_calls_made += 1;

        let known_skills = state.db.get_top_canonical_skills(MAX_KNOWN_SKILLS_CONTEXT)?;
        let extracted = match state
            .gemini
            .extract_job(&api_key, &scraped_job.raw_description, &known_skills)
            .await
        {
            Ok(res) => res,
            Err(err) if err.code == ErrorCode::ApiKeyInvalid => return Err(err),
            Err(err) => {
                failed_extractions += 1;
                warnings.push(ScrapeWarningItem {
                    source: scraped_job.source.clone(),
                    code: err.code.as_str().to_string(),
                    message: format!("Gagal ekstraksi '{}': {}", scraped_job.title, err.message),
                });
                continue;
            }
        };

        let mandatory_names: Vec<String> = extracted
            .skills
            .iter()
            .filter(|s| s.requirement_type == RequirementType::Mandatory)
            .map(|s| s.canonical_name.clone())
            .collect();
        let nice_names: Vec<String> = extracted
            .skills
            .iter()
            .filter(|s| s.requirement_type == RequirementType::NiceToHave)
            .map(|s| s.canonical_name.clone())
            .collect();

        let embed_text = format_job_embedding_text(
            &extracted.normalized_role,
            extracted.experience_level,
            &scraped_job.title,
            &extracted.role_summary,
            &mandatory_names,
            &nice_names,
        );

        let embedding_bytes = match state
            .gemini
            .embed_text(&api_key, &embed_text, EmbeddingTaskType::RetrievalDocument)
            .await
        {
            Ok(vec) => Some(encode_embedding(&vec)),
            Err(err) if err.code == ErrorCode::ApiKeyInvalid => return Err(err),
            Err(err) => {
                warnings.push(ScrapeWarningItem {
                    source: scraped_job.source.clone(),
                    code: err.code.as_str().to_string(),
                    message: format!(
                        "Embedding gagal untuk '{}', skor memakai bobot skill+level: {}",
                        scraped_job.title, err.message
                    ),
                });
                None
            }
        };

        let extracted_json = serde_json::to_string(&extracted)
            .map_err(|e| AppError::db_error(format!("Gagal serialisasi hasil ekstraksi: {e}")))?;

        let job_id = Uuid::new_v4().to_string();
        let quality = assess_job_quality(
            &scraped_job.title,
            &scraped_job.company,
            &scraped_job.raw_description,
            scraped_job.salary_text.as_deref(),
            scraped_job.url.as_deref(),
        );
        let red_flags_json = serde_json::to_string(&quality.red_flags).unwrap_or_else(|_| "[]".into());
        let tax_probe = vecta_taxonomy::TaxonomyEngine::global().extract_job(
            &scraped_job.title,
            &scraped_job.raw_description,
            None,
        );
        let new_job = NewJobRecord {
            id: job_id.clone(),
            source,
            external_id: scraped_job.external_id,
            url: sanitize_external_url(scraped_job.url.as_deref()),
            title: scraped_job.title,
            normalized_role: extracted.normalized_role.clone(),
            company: scraped_job.company,
            location: scraped_job.location,
            work_mode: extracted.work_mode,
            experience_level: extracted.experience_level,
            raw_description: scraped_job.raw_description,
            extracted_summary_json: Some(extracted_json),
            embedding: embedding_bytes,
            enrichment_status: EnrichmentStatus::AiEnriched,
            posted_at: scraped_job.posted_at,
            salary_text: scraped_job.salary_text,
            domain_slug: tax_probe.dominant_domain,
            quality_score: quality.quality_score,
            red_flags_json,
            also_available_on_json: "[]".into(),
            embedding_model: Some("gemini:embedding-001".into()),
        };

        let job_match_rec = state.db.insert_job_with_skills(&new_job, &extracted.skills)?;
        let computed = compute_hybrid_match(&profile, &job_match_rec);
        let upsert = to_match_upsert(&user_id, &job_id, &computed)?;
        state.db.upsert_job_matches(&[upsert])?;
        newly_ingested += 1;
    }

    state.db.touch_active_keywords_scraped(&user_id)?;
    let pending_local_count = state.db.count_pending_local_jobs(&user_id).unwrap_or(0);

    Ok(ScrapeRunReport {
        total_fetched,
        newly_ingested,
        skipped_duplicates,
        failed_extractions,
        pending_local_count,
        warnings,
    })
}

/// Stage 1 (Decoupled, 0 Gemini tokens):
/// Scrape multi-page jobs from Python Worker using full user preferences, store them cleanly
/// into SQLite as `local_only`, extract skills from the local dictionary, and compute instant
/// local match rankings in < 30ms.
pub async fn scrape_and_store_jobs_impl(
    state: &AppState,
    app: Option<&AppHandle>,
    user_id: String,
    prefs: SearchPreferences,
) -> Result<ScrapeRunReport, AppError> {
    let profile_match = state
        .db
        .get_profile_for_matching(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;
    let profile_detail = state
        .db
        .get_profile_by_id(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;

    // Combine custom queries from SearchPreferences with active profile keywords
    let mut queries: Vec<String> = prefs
        .custom_queries
        .iter()
        .map(|q| q.trim().to_string())
        .filter(|q| !q.is_empty())
        .collect();
    if queries.is_empty() {
        queries = state.db.list_active_scrape_queries(&user_id)?;
    }
    if queries.is_empty() {
        return Err(AppError::validation_error(
            "Tambahkan minimal satu kata kunci pencarian atau aktifkan peran di Profil Cerita.",
        ));
    }
    queries.truncate(15);

    let clean_sources: Vec<String> = prefs
        .sources
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| {
            matches!(
                s.as_str(),
                "linkedin" | "jobstreet" | "indeed" | "kalibrr" | "dealls" | "glints"
            )
        })
        .collect();
    if clean_sources.is_empty() {
        return Err(AppError::validation_error(
            "Pilih minimal satu portal lowongan (JobStreet, LinkedIn, Kalibrr, Dealls, Indeed, atau Glints).",
        ));
    }

    let location = if prefs.location.trim().is_empty() {
        "Indonesia".to_string()
    } else {
        prefs.location.trim().chars().take(80).collect()
    };

    let _scrape_guard = state.try_acquire_scrape_guard()?;
    let worker_url = state.get_worker_url();

    emit_progress(
        app,
        "scraping",
        format!(
            "Menarik lowongan dari {} ({}, {} halaman/query)...",
            clean_sources.join(", "),
            location,
            prefs.max_pages.clamp(1, 5)
        ),
        0,
        queries.len() as u32,
    );

    let worker_req = WorkerScrapeRequest {
        queries: queries.clone(),
        sources: clean_sources,
        location: location.clone(),
        limit_per_query: prefs.limit_per_query.clamp(5, 50),
        max_pages: prefs.max_pages.clamp(1, 5),
        posted_within_days: prefs.posted_within_days.map(|d| d.clamp(1, 90)),
        work_mode: prefs.work_mode.clone(),
    };

    let worker_resp = state.active_scraper().scrape(&worker_url, &worker_req).await?;

    // Cross-portal deduplication (e.g. same PT + title on JobStreet & LinkedIn -> merged card with alternate links)
    let raw_fetched = worker_resp.jobs.len() as u32;
    let merged_jobs = deduplicate_cross_portal(worker_resp.jobs, |j| {
        (
            j.company.clone(),
            j.title.clone(),
            j.location.clone().unwrap_or_default(),
            j.source.clone(),
            j.url.clone(),
            j.raw_description.clone(),
        )
    });
    let cross_portal_dupes = raw_fetched.saturating_sub(merged_jobs.len() as u32);

    // Ensure 14-domain Indonesian skill taxonomy exists in SQLite for 0-token local extraction
    state.db.ensure_seed_skills()?;
    let skill_catalog = state.db.list_all_skills_for_local_matching()?;
    let user_skill_names: Vec<String> = profile_detail
        .skills
        .iter()
        .map(|s| s.canonical_name.clone())
        .collect();
    let user_summary = profile_detail.summary.unwrap_or_default();
    let fallback_query = queries.first().map(String::as_str);

    // Compute offline multilingual user embedding once (< 0.3ms) when local AI pack is bundled
    let local_profile_vec = if state.local_embedder.is_ready() {
        let profile_text = format!(
            "{} {} {}",
            profile_detail.target_roles.join(" "),
            user_summary,
            user_skill_names.join(" ")
        );
        state.local_embedder.embed_one(&profile_text)
    } else {
        None
    };

    let total_fetched = raw_fetched;
    let mut newly_ingested = 0u32;
    let mut skipped_duplicates = cross_portal_dupes;
    let mut upserts = Vec::new();

    emit_progress(
        app,
        "local_matching",
        if local_profile_vec.is_some() {
            format!(
                "Menyimpan & menilai {} lowongan dengan AI Semantik Offline + Taksonomi 14 Prodi (0 token)...",
                merged_jobs.len()
            )
        } else {
            format!(
                "Menyimpan & menilai {} lowongan dengan Taksonomi 14 Prodi lokal (0 token)...",
                merged_jobs.len()
            )
        },
        0,
        merged_jobs.len() as u32,
    );

    for (idx, (scraped_job, alternate_portals)) in merged_jobs.into_iter().enumerate() {
        let source = JobSource::from_db_str(&scraped_job.source);
        if let Some(existing_job_id) = state
            .db
            .job_exists_by_source_external_id(source, &scraped_job.external_id)?
        {
            skipped_duplicates += 1;
            if let Some(job_rec) = state.db.load_job_for_matching(&existing_job_id)? {
                let computed = if job_rec.embedding.is_some() {
                    compute_hybrid_match(&profile_match, &job_rec)
                } else {
                    let local_ctx = compute_local_keyword_relevance(
                        &user_summary,
                        &profile_detail.target_roles,
                        &user_skill_names,
                        &scraped_job.title,
                        &scraped_job.raw_description,
                        Some(&location),
                        scraped_job.location.as_deref(),
                    );
                    compute_local_hybrid_match(&profile_match, &job_rec, local_ctx)
                };
                upserts.push(to_match_upsert(&user_id, &existing_job_id, &computed)?);
            }
            continue;
        }

        let work_mode = infer_work_mode(
            &scraped_job.title,
            scraped_job.location.as_deref(),
            &scraped_job.raw_description,
        );
        let experience_level =
            infer_experience_level(&scraped_job.title, &scraped_job.raw_description);
        let universal = extract_universal_job_metadata(
            &scraped_job.title,
            &scraped_job.raw_description,
            fallback_query,
            &skill_catalog,
        );
        let clean_url = sanitize_external_url(scraped_job.url.as_deref());
        let quality = assess_job_quality(
            &scraped_job.title,
            &scraped_job.company,
            &scraped_job.raw_description,
            scraped_job.salary_text.as_deref(),
            clean_url.as_deref(),
        );
        let red_flags_json =
            serde_json::to_string(&quality.red_flags).unwrap_or_else(|_| "[]".into());
        let also_available_on_json =
            serde_json::to_string(&alternate_portals).unwrap_or_else(|_| "[]".into());

        // Pure-Rust offline multilingual semantic vector (< 0.3ms, 0 API key, 101 languages)
        let (local_job_vec, embedding_bytes, embedding_model) = if local_profile_vec.is_some() {
            let job_text = format!("{} {}", scraped_job.title, scraped_job.raw_description);
            match state.local_embedder.embed_one(&job_text) {
                Some(v) => {
                    let bytes = encode_embedding(&v);
                    (Some(v), Some(bytes), Some(LOCAL_EMBEDDING_MODEL_ID.to_string()))
                }
                None => (None, None, None),
            }
        } else {
            (None, None, None)
        };

        let job_id = Uuid::new_v4().to_string();
        let new_job = NewJobRecord {
            id: job_id.clone(),
            source,
            external_id: scraped_job.external_id,
            url: clean_url,
            title: scraped_job.title.clone(),
            normalized_role: universal.normalized_role,
            company: scraped_job.company,
            location: scraped_job.location.clone(),
            work_mode,
            experience_level,
            raw_description: scraped_job.raw_description.clone(),
            extracted_summary_json: None,
            embedding: embedding_bytes,
            enrichment_status: EnrichmentStatus::LocalOnly,
            posted_at: scraped_job.posted_at,
            salary_text: scraped_job.salary_text,
            domain_slug: universal.dominant_domain,
            quality_score: quality.quality_score,
            red_flags_json,
            also_available_on_json,
            embedding_model,
        };

        let job_match_rec = state
            .db
            .insert_job_with_skill_ids(&new_job, &universal.skill_ids)?;
        let kw_ctx = compute_local_keyword_relevance(
            &user_summary,
            &profile_detail.target_roles,
            &user_skill_names,
            &scraped_job.title,
            &scraped_job.raw_description,
            Some(&location),
            scraped_job.location.as_deref(),
        );
        let semantic_ctx = match (&local_profile_vec, &local_job_vec) {
            (Some(u), Some(j)) => cosine_similarity(u, j)
                .map(|sim| (0.70 * (sim as f64) + 0.30 * kw_ctx).clamp(0.0, 1.0))
                .unwrap_or(kw_ctx),
            _ => kw_ctx,
        };
        let computed = compute_local_hybrid_match(&profile_match, &job_match_rec, semantic_ctx);
        upserts.push(to_match_upsert(&user_id, &job_id, &computed)?);
        newly_ingested += 1;

        if (idx + 1) % 20 == 0 || idx + 1 == total_fetched as usize {
            emit_progress(
                app,
                "local_matching",
                format!("Menyimpan & menilai lokal {}/{}...", idx + 1, total_fetched),
                (idx + 1) as u32,
                total_fetched,
            );
        }
    }

    if !upserts.is_empty() {
        state.db.upsert_job_matches(&upserts)?;
    }
    state.db.touch_active_keywords_scraped(&user_id)?;
    let pending_local_count = state.db.count_pending_local_jobs(&user_id)?;

    emit_progress(
        app,
        "done",
        format!("Selesai: {newly_ingested} lowongan baru tersimpan rapi (0 token Gemini)."),
        total_fetched,
        total_fetched,
    );

    Ok(ScrapeRunReport {
        total_fetched,
        newly_ingested,
        skipped_duplicates,
        failed_extractions: 0,
        pending_local_count,
        warnings: worker_resp.warnings,
    })
}

/// Stage 2 (Decoupled, Resumable, Batched + RPM Limited):
/// Reads pending `local_only` jobs (excluding dismissed ones) from SQLite, extracts 5 jobs/call
/// via `extract_jobs_batch` + embeds via `batchEmbedContents`, enforces the user's RPM cap,
/// and upgrades them to `ai_enriched`.
pub async fn run_ai_matching_queue_impl(
    state: &AppState,
    app: Option<&AppHandle>,
    user_id: String,
    max_jobs: u32,
    rpm_limit: u32,
) -> Result<AiMatchRunReport, AppError> {
    let api_key = state.require_api_key()?;
    let profile = state
        .db
        .get_profile_for_matching(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;

    let _guard = state.try_acquire_scrape_guard()?;
    let limit = max_jobs.clamp(1, 100) as usize;
    let safe_rpm = rpm_limit.clamp(1, 60);

    let pending = state.db.list_pending_local_jobs_for_ai(&user_id, limit)?;
    let total = pending.len() as u32;
    if pending.is_empty() {
        return Ok(AiMatchRunReport {
            processed_jobs: 0,
            enriched_jobs: 0,
            failed_jobs: 0,
            remaining_local_jobs: 0,
            api_calls_used: 0,
            warnings: vec![],
        });
    }

    let mut enriched_jobs = 0u32;
    let mut failed_jobs = 0u32;
    let mut api_calls_used = 0u32;
    let mut warnings = Vec::new();
    let mut processed = 0u32;

    for chunk in pending.chunks(AI_EXTRACT_BATCH_SIZE) {
        let wait_before = state.rpm_limiter.pending_wait(safe_rpm).await;
        if !wait_before.is_zero() {
            emit_progress(
                app,
                "rate_limit_wait",
                format!(
                    "Menunggu jeda aman kuota RPM ({} detik) agar tidak terkena limit 429...",
                    wait_before.as_secs().max(1)
                ),
                processed,
                total,
            );
        }
        state.rpm_limiter.acquire(safe_rpm).await;

        emit_progress(
            app,
            "ai_batch_extract",
            format!(
                "AI Matching batch {}/{} lowongan (hemat kuota: {} lowongan/request)...",
                (processed + chunk.len() as u32).min(total),
                total,
                chunk.len()
            ),
            processed,
            total,
        );

        let known_skills = state.db.get_top_canonical_skills(MAX_KNOWN_SKILLS_CONTEXT)?;
        let batch_inputs: Vec<BatchJobInput> = chunk
            .iter()
            .map(|row| BatchJobInput {
                job_id: row.job_id.clone(),
                title: row.title.clone(),
                raw_description: row.raw_description.clone(),
            })
            .collect();

        api_calls_used += 1;
        let extracted_items = match state
            .gemini
            .extract_jobs_batch(&api_key, &batch_inputs, &known_skills)
            .await
        {
            Ok(items) => items,
            Err(err) if err.code == ErrorCode::ApiKeyInvalid => return Err(err),
            Err(err) => {
                failed_jobs += chunk.len() as u32;
                processed += chunk.len() as u32;
                warnings.push(ScrapeWarningItem {
                    source: "gemini".into(),
                    code: err.code.as_str().to_string(),
                    message: format!("Batch ekstraksi gagal ({} lowongan): {}", chunk.len(), err.message),
                });
                if err.code == ErrorCode::GeminiRateLimited {
                    break;
                }
                continue;
            }
        };

        let mut extracted_by_id: HashMap<String, JobExtractionResult> = HashMap::new();
        for item in extracted_items {
            extracted_by_id.insert(
                item.job_id,
                JobExtractionResult {
                    normalized_role: item.normalized_role,
                    experience_level: item.experience_level,
                    work_mode: item.work_mode,
                    role_summary: item.role_summary,
                    skills: item.skills,
                },
            );
        }

        // Prepare batch embedding texts for all successfully extracted jobs in this chunk
        let mut ready_rows = Vec::new();
        let mut embed_texts = Vec::new();
        for row in chunk {
            let Some(ext) = extracted_by_id.remove(&row.job_id) else {
                failed_jobs += 1;
                warnings.push(ScrapeWarningItem {
                    source: row.source.clone(),
                    code: "GEMINI_PARSE_FAILED".into(),
                    message: format!("Model melewatkan '{}' pada batch ekstraksi.", row.title),
                });
                continue;
            };
            let mandatory_names: Vec<String> = ext
                .skills
                .iter()
                .filter(|s| s.requirement_type == RequirementType::Mandatory)
                .map(|s| s.canonical_name.clone())
                .collect();
            let nice_names: Vec<String> = ext
                .skills
                .iter()
                .filter(|s| s.requirement_type == RequirementType::NiceToHave)
                .map(|s| s.canonical_name.clone())
                .collect();
            let embed_text = format_job_embedding_text(
                &ext.normalized_role,
                ext.experience_level,
                &row.title,
                &ext.role_summary,
                &mandatory_names,
                &nice_names,
            );
            embed_texts.push(embed_text);
            ready_rows.push((row, ext));
        }

        if !ready_rows.is_empty() {
            let wait_emb = state.rpm_limiter.pending_wait(safe_rpm).await;
            if !wait_emb.is_zero() {
                emit_progress(
                    app,
                    "rate_limit_wait",
                    format!(
                        "Menunggu jeda aman RPM sebelum batch embedding ({} detik)...",
                        wait_emb.as_secs().max(1)
                    ),
                    processed,
                    total,
                );
            }
            state.rpm_limiter.acquire(safe_rpm).await;
            api_calls_used += 1;

            let embeddings_opt: Option<Vec<Vec<f32>>> = match state
                .gemini
                .embed_texts_batch(&api_key, &embed_texts, EmbeddingTaskType::RetrievalDocument)
                .await
            {
                Ok(vecs) => Some(vecs),
                Err(err) if err.code == ErrorCode::ApiKeyInvalid => return Err(err),
                Err(err) => {
                    warnings.push(ScrapeWarningItem {
                        source: "gemini".into(),
                        code: err.code.as_str().to_string(),
                        message: format!(
                            "Batch embedding gagal, skor sementara memakai skill+level: {}",
                            err.message
                        ),
                    });
                    None
                }
            };

            let mut upserts = Vec::with_capacity(ready_rows.len());
            for (idx, (row, ext)) in ready_rows.into_iter().enumerate() {
                let emb_bytes = embeddings_opt
                    .as_ref()
                    .and_then(|v| v.get(idx))
                    .map(|vec| encode_embedding(vec));
                let extracted_json = serde_json::to_string(&ext)
                    .map_err(|_| AppError::db_error("Gagal serialisasi ekstraksi AI."))?;
                let job_match_rec = state.db.apply_ai_enrichment_to_job(
                    &row.job_id,
                    &ext,
                    &extracted_json,
                    emb_bytes.as_deref(),
                )?;
                let computed = compute_hybrid_match(&profile, &job_match_rec);
                upserts.push(to_match_upsert(&user_id, &row.job_id, &computed)?);
                enriched_jobs += 1;
            }
            if !upserts.is_empty() {
                state.db.upsert_job_matches(&upserts)?;
            }
        }

        processed += chunk.len() as u32;
        emit_progress(
            app,
            "ai_batch_extract",
            format!("Selesai memperkaya {enriched_jobs}/{total} lowongan dengan AI..."),
            processed,
            total,
        );
    }

    let remaining_local_jobs = state.db.count_pending_local_jobs(&user_id)?;
    emit_progress(
        app,
        "done",
        format!(
            "AI Matching selesai: {enriched_jobs} lowongan diperkaya ({api_calls_used} panggilan API)."
        ),
        total,
        total,
    );

    Ok(AiMatchRunReport {
        processed_jobs: processed,
        enriched_jobs,
        failed_jobs,
        remaining_local_jobs,
        api_calls_used,
        warnings,
    })
}

/// On-demand AI enrichment for a single `local_only` job.
pub async fn enrich_job_with_ai_impl(
    state: &AppState,
    user_id: String,
    job_id: String,
) -> Result<JobMatchDetail, AppError> {
    let api_key = state.require_api_key()?;
    let profile = state
        .db
        .get_profile_for_matching(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;
    let row = state
        .db
        .load_job_for_ai_enrichment(&job_id)?
        .ok_or_else(|| AppError::validation_error(format!("Lowongan '{job_id}' tidak ditemukan.")))?;

    let known_skills = state.db.get_top_canonical_skills(MAX_KNOWN_SKILLS_CONTEXT)?;
    state.rpm_limiter.acquire(15).await;
    let extracted = state
        .gemini
        .extract_job(&api_key, &row.raw_description, &known_skills)
        .await?;

    let mandatory_names: Vec<String> = extracted
        .skills
        .iter()
        .filter(|s| s.requirement_type == RequirementType::Mandatory)
        .map(|s| s.canonical_name.clone())
        .collect();
    let nice_names: Vec<String> = extracted
        .skills
        .iter()
        .filter(|s| s.requirement_type == RequirementType::NiceToHave)
        .map(|s| s.canonical_name.clone())
        .collect();
    let embed_text = format_job_embedding_text(
        &extracted.normalized_role,
        extracted.experience_level,
        &row.title,
        &extracted.role_summary,
        &mandatory_names,
        &nice_names,
    );

    state.rpm_limiter.acquire(15).await;
    let emb_vec = state
        .gemini
        .embed_text(&api_key, &embed_text, EmbeddingTaskType::RetrievalDocument)
        .await?;
    let emb_bytes = encode_embedding(&emb_vec);

    let extracted_json = serde_json::to_string(&extracted)
        .map_err(|_| AppError::db_error("Gagal serialisasi ekstraksi AI."))?;
    let job_match_rec = state.db.apply_ai_enrichment_to_job(
        &job_id,
        &extracted,
        &extracted_json,
        Some(&emb_bytes),
    )?;
    let computed = compute_hybrid_match(&profile, &job_match_rec);
    let upsert = to_match_upsert(&user_id, &job_id, &computed)?;
    state.db.upsert_job_matches(&[upsert])?;

    state.db.get_job_detail(&user_id, &job_id)
}

pub fn update_job_application_impl(
    state: &AppState,
    user_id: String,
    job_id: String,
    status: ApplicationStatus,
    notes: Option<String>,
) -> Result<JobMatchDetail, AppError> {
    state
        .db
        .update_job_application(&user_id, &job_id, status, notes.as_deref())
}

pub async fn generate_apply_kit_impl(
    state: &AppState,
    user_id: String,
    job_id: String,
    language: String,
    force_refresh: bool,
) -> Result<ApplyKitResult, AppError> {
    let clean_lang = if language.trim().eq_ignore_ascii_case("en") {
        "en".to_string()
    } else {
        "id".to_string()
    };

    if !force_refresh {
        if let Some(cached) = state.db.get_cached_apply_kit(&user_id, &job_id)? {
            if cached.language == clean_lang {
                return Ok(cached);
            }
        }
    }

    let api_key = state.require_api_key()?;
    let profile = state
        .db
        .get_profile_by_id(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;
    let detail = state.db.get_job_detail(&user_id, &job_id)?;

    let prompt_input = ApplyKitPromptInput {
        language: clean_lang,
        user_summary: profile.summary.unwrap_or_default(),
        user_level: profile.experience_level.as_str().to_string(),
        user_skills: profile
            .skills
            .iter()
            .map(|s| format!("{} ({})", s.canonical_name, s.proficiency.as_str()))
            .collect(),
        job_title: detail.title,
        job_company: detail.company,
        job_role: detail.normalized_role,
        job_description: detail.raw_description,
        matched_skills: detail
            .matched_skills
            .iter()
            .map(|s| s.canonical_name.clone())
            .collect(),
        missing_skills: detail
            .missing_skills
            .iter()
            .map(|s| s.canonical_name.clone())
            .collect(),
    };

    let kit = state
        .gemini
        .generate_apply_kit(&api_key, &prompt_input)
        .await?;
    state.db.save_apply_kit(&user_id, &job_id, &kit)?;
    Ok(kit)
}

pub fn list_ranked_jobs_impl(
    state: &AppState,
    user_id: String,
    filter: JobFilterInput,
) -> Result<Vec<JobMatchSummary>, AppError> {
    state.db.list_ranked_jobs(&user_id, &filter)
}

pub fn get_job_detail_impl(
    state: &AppState,
    user_id: String,
    job_id: String,
) -> Result<JobMatchDetail, AppError> {
    state.db.get_job_detail(&user_id, &job_id)
}

pub fn toggle_save_job_impl(
    state: &AppState,
    user_id: String,
    job_id: String,
) -> Result<bool, AppError> {
    state.db.toggle_save_job(&user_id, &job_id)
}

pub async fn generate_gap_analysis_impl(
    state: &AppState,
    user_id: String,
    job_id: String,
    force_refresh: bool,
) -> Result<GapAnalysis, AppError> {
    if !force_refresh {
        if let Some(cached) = state.db.get_cached_gap_analysis(&user_id, &job_id)? {
            return Ok(cached);
        }
    }

    let api_key = state.require_api_key()?;
    let profile = state
        .db
        .get_profile_by_id(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;
    let job_detail = state.db.get_job_detail(&user_id, &job_id)?;

    let prompt_input = GapAnalysisPromptInput {
        user_summary: profile.summary.unwrap_or_default(),
        user_level: profile.experience_level.as_str().to_string(),
        user_skills: profile
            .skills
            .iter()
            .map(|s| format!("{} ({})", s.canonical_name, s.proficiency.as_str()))
            .collect(),
        job_title: job_detail.title,
        job_company: job_detail.company,
        job_role: job_detail.normalized_role,
        job_description: job_detail.raw_description,
        matched_skills: job_detail
            .matched_skills
            .iter()
            .map(|s| s.canonical_name.clone())
            .collect(),
        missing_skills: job_detail
            .missing_skills
            .iter()
            .map(|s| s.canonical_name.clone())
            .collect(),
    };

    let analysis = state.gemini.analyze_gap(&api_key, &prompt_input).await?;
    state.db.save_gap_analysis(&user_id, &job_id, &analysis)?;
    Ok(analysis)
}

#[tauri::command]
#[specta::specta]
pub async fn ingest_manual_job(
    state: State<'_, AppState>,
    user_id: String,
    title_hint: Option<String>,
    url: Option<String>,
    raw_text: String,
) -> Result<JobMatchDetail, AppError> {
    ingest_manual_job_impl(&state, user_id, title_hint, url, raw_text).await
}

#[tauri::command]
#[specta::specta]
pub async fn run_scrape_for_active_keywords(
    state: State<'_, AppState>,
    user_id: String,
    sources: Vec<String>,
    limit_per_query: u32,
) -> Result<ScrapeRunReport, AppError> {
    run_scrape_for_active_keywords_impl(&state, user_id, sources, limit_per_query).await
}

#[tauri::command]
#[specta::specta]
pub fn list_ranked_jobs(
    state: State<'_, AppState>,
    user_id: String,
    filter: JobFilterInput,
) -> Result<Vec<JobMatchSummary>, AppError> {
    list_ranked_jobs_impl(&state, user_id, filter)
}

#[tauri::command]
#[specta::specta]
pub fn get_job_detail(
    state: State<'_, AppState>,
    user_id: String,
    job_id: String,
) -> Result<JobMatchDetail, AppError> {
    get_job_detail_impl(&state, user_id, job_id)
}

#[tauri::command]
#[specta::specta]
pub fn toggle_save_job(
    state: State<'_, AppState>,
    user_id: String,
    job_id: String,
) -> Result<bool, AppError> {
    toggle_save_job_impl(&state, user_id, job_id)
}

#[tauri::command]
#[specta::specta]
pub async fn generate_gap_analysis(
    state: State<'_, AppState>,
    user_id: String,
    job_id: String,
    force_refresh: bool,
) -> Result<GapAnalysis, AppError> {
    generate_gap_analysis_impl(&state, user_id, job_id, force_refresh).await
}

#[tauri::command]
#[specta::specta]
pub async fn scrape_and_store_jobs(
    app: AppHandle,
    state: State<'_, AppState>,
    user_id: String,
    prefs: SearchPreferences,
) -> Result<ScrapeRunReport, AppError> {
    scrape_and_store_jobs_impl(&state, Some(&app), user_id, prefs).await
}

#[tauri::command]
#[specta::specta]
pub async fn run_ai_matching_queue(
    app: AppHandle,
    state: State<'_, AppState>,
    user_id: String,
    max_jobs: u32,
    rpm_limit: u32,
) -> Result<AiMatchRunReport, AppError> {
    run_ai_matching_queue_impl(&state, Some(&app), user_id, max_jobs, rpm_limit).await
}

#[tauri::command]
#[specta::specta]
pub async fn enrich_job_with_ai(
    state: State<'_, AppState>,
    user_id: String,
    job_id: String,
) -> Result<JobMatchDetail, AppError> {
    enrich_job_with_ai_impl(&state, user_id, job_id).await
}

#[tauri::command]
#[specta::specta]
pub fn update_job_application(
    state: State<'_, AppState>,
    user_id: String,
    job_id: String,
    status: ApplicationStatus,
    notes: Option<String>,
) -> Result<JobMatchDetail, AppError> {
    update_job_application_impl(&state, user_id, job_id, status, notes)
}

#[tauri::command]
#[specta::specta]
pub async fn generate_apply_kit(
    state: State<'_, AppState>,
    user_id: String,
    job_id: String,
    language: String,
    force_refresh: bool,
) -> Result<ApplyKitResult, AppError> {
    generate_apply_kit_impl(&state, user_id, job_id, language, force_refresh).await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use super::*;
    use crate::commands::profile::save_confirmed_profile_impl;
    use crate::core::worker::{WorkerScrapeResponse, WorkerScrapeStats, WorkerScrapedJob};
    use crate::models::job::{JobSource, ScrapeWarningItem};
    use crate::models::profile::{
        ConfirmedKeywordInput, ConfirmedProfileInput, ConfirmedSkillInput, ExperienceLevel,
        FitType, Proficiency, SkillCategory,
    };
    use crate::state::test_support::{build_test_state, MockGemini, MockWorker};

    async fn seed_user(state: &AppState) -> String {
        let profile = save_confirmed_profile_impl(
            state,
            ConfirmedProfileInput {
                id: Some("user-1".into()),
                profile_name: "Primary".into(),
                target_roles: vec!["Backend Engineer".into()],
                raw_story_text:
                    "Saya membangun layanan backend Rust dan SQLite untuk aplikasi desktop.".into(),
                summary: "Junior Rust Backend Developer".into(),
                experience_level: ExperienceLevel::Junior,
                skills: vec![ConfirmedSkillInput {
                    canonical_name: "Rust".into(),
                    category: SkillCategory::CoreHard,
                    proficiency: Proficiency::Intermediate,
                    evidence: Some("Vecta Core".into()),
                }],
                keywords: vec![ConfirmedKeywordInput {
                    role_label: "Backend Engineer".into(),
                    fit_type: FitType::PrimaryFit,
                    scrape_queries: vec!["Backend Engineer".into()],
                    match_reason: Some("Match Rust".into()),
                    is_active: true,
                }],
            },
        )
        .await
        .expect("seed user");
        profile.id
    }

    #[tokio::test]
    async fn ingest_manual_job_extracts_embeds_and_scores() {
        let gemini = Arc::new(MockGemini::default());
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::clone(&gemini),
            Arc::new(MockWorker::default()),
        );
        let user_id = seed_user(&state).await;

        let detail = ingest_manual_job_impl(
            &state,
            user_id,
            Some("Junior Rust Backend".into()),
            Some("https://example.com/jobs/1".into()),
            "Kami mencari Junior Backend Engineer berpengalaman Rust dan Docker untuk tim inti."
                .into(),
        )
        .await
        .expect("ingest manual job");

        assert_eq!(detail.source, JobSource::Manual);
        assert_eq!(detail.title, "Junior Rust Backend");
        assert!(detail.final_score > 0.7);
        assert_eq!(detail.matched_skills.len(), 1);
        assert_eq!(detail.missing_skills.len(), 1);
        assert_eq!(gemini.extract_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn run_scrape_skips_existing_jobs_without_gemini_calls() {
        let gemini = Arc::new(MockGemini::default());
        let worker = Arc::new(MockWorker::default());
        *worker.response.lock().unwrap() = Ok(WorkerScrapeResponse {
            jobs: vec![
                WorkerScrapedJob {
                    source: "linkedin".into(),
                    external_id: "li-101".into(),
                    url: Some("https://linkedin.com/jobs/view/101".into()),
                    title: "Rust Backend Engineer".into(),
                    company: "PT Alpha".into(),
                    location: Some("Jakarta".into()),
                    raw_description: "Mencari backend engineer dengan kemampuan Rust dan SQLite."
                        .into(),
                    posted_at: None,
                    salary_text: None,
                },
                WorkerScrapedJob {
                    source: "glints".into(),
                    external_id: "gl-202".into(),
                    url: Some("https://glints.com/jobs/202".into()),
                    title: "Systems Developer".into(),
                    company: "PT Beta".into(),
                    location: Some("Bandung".into()),
                    raw_description: "Membangun desktop engine lokal dengan Rust dan Docker.".into(),
                    posted_at: None,
                    salary_text: None,
                },
            ],
            stats: WorkerScrapeStats {
                total_fetched: 2,
                deduplicated_in_batch: 0,
                duration_ms: 120,
            },
            warnings: vec![],
        });

        let state = build_test_state(Some("AIzaSyTESTKEY1234"), Arc::clone(&gemini), worker);
        let user_id = seed_user(&state).await;
        let embed_before = gemini.embed_calls.load(Ordering::SeqCst);

        let first = run_scrape_for_active_keywords_impl(
            &state,
            user_id.clone(),
            vec!["linkedin".into(), "glints".into()],
            10,
        )
        .await
        .expect("first scrape");
        assert_eq!(first.total_fetched, 2);
        assert_eq!(first.newly_ingested, 2);
        assert_eq!(first.skipped_duplicates, 0);
        assert_eq!(gemini.extract_calls.load(Ordering::SeqCst), 2);
        assert_eq!(gemini.embed_calls.load(Ordering::SeqCst) - embed_before, 2);

        let second = run_scrape_for_active_keywords_impl(
            &state,
            user_id,
            vec!["linkedin".into(), "glints".into()],
            10,
        )
        .await
        .expect("second scrape");
        assert_eq!(second.newly_ingested, 0);
        assert_eq!(second.skipped_duplicates, 2);
        assert_eq!(gemini.extract_calls.load(Ordering::SeqCst), 2);
        assert_eq!(gemini.embed_calls.load(Ordering::SeqCst) - embed_before, 2);
    }

    #[tokio::test]
    async fn run_scrape_preserves_partial_warnings_from_worker() {
        let gemini = Arc::new(MockGemini::default());
        let worker = Arc::new(MockWorker::default());
        *worker.response.lock().unwrap() = Ok(WorkerScrapeResponse {
            jobs: vec![WorkerScrapedJob {
                source: "linkedin".into(),
                external_id: "li-999".into(),
                url: None,
                title: "Backend Developer".into(),
                company: "PT Gamma".into(),
                location: Some("Jakarta".into()),
                raw_description: "Membangun layanan API dengan Rust dan SQLite secara lokal."
                    .into(),
                posted_at: None,
                salary_text: None,
            }],
            stats: WorkerScrapeStats {
                total_fetched: 1,
                deduplicated_in_batch: 0,
                duration_ms: 50,
            },
            warnings: vec![ScrapeWarningItem {
                source: "glints".into(),
                code: "RATE_LIMITED".into(),
                message: "HTTP 429 pada Glints".into(),
            }],
        });

        let state = build_test_state(Some("AIzaSyTESTKEY1234"), gemini, worker);
        let user_id = seed_user(&state).await;

        let report = run_scrape_for_active_keywords_impl(
            &state,
            user_id,
            vec!["linkedin".into(), "glints".into()],
            10,
        )
        .await
        .expect("partial scrape succeeds");
        assert_eq!(report.newly_ingested, 1);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "RATE_LIMITED");
    }

    #[tokio::test]
    async fn list_ranked_jobs_orders_by_final_score_desc_and_filters() {
        let gemini = Arc::new(MockGemini::default());
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::clone(&gemini),
            Arc::new(MockWorker::default()),
        );
        let user_id = seed_user(&state).await;

        let j1 = ingest_manual_job_impl(
            &state,
            user_id.clone(),
            Some("Role A".into()),
            None,
            "Deskripsi lowongan pertama untuk Backend Engineer dengan Rust dan Docker.".into(),
        )
        .await
        .expect("j1");
        toggle_save_job_impl(&state, user_id.clone(), j1.job_id.clone()).expect("save j1");

        let _j2 = ingest_manual_job_impl(
            &state,
            user_id.clone(),
            Some("Role B".into()),
            None,
            "Deskripsi lowongan kedua yang berbeda untuk tim infrastruktur cloud backend.".into(),
        )
        .await
        .expect("j2");

        let all = list_ranked_jobs_impl(&state, user_id.clone(), JobFilterInput::default())
            .expect("list all");
        assert_eq!(all.len(), 2);
        assert!(all[0].final_score >= all[1].final_score);

        let saved_only = list_ranked_jobs_impl(
            &state,
            user_id,
            JobFilterInput {
                saved_only: Some(true),
                ..Default::default()
            },
        )
        .expect("saved only");
        assert_eq!(saved_only.len(), 1);
        assert_eq!(saved_only[0].job_id, j1.job_id);
    }

    #[tokio::test]
    async fn generate_gap_analysis_caches_result_and_supports_force_refresh() {
        let gemini = Arc::new(MockGemini::default());
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::clone(&gemini),
            Arc::new(MockWorker::default()),
        );
        let user_id = seed_user(&state).await;

        let job = ingest_manual_job_impl(
            &state,
            user_id.clone(),
            Some("Backend Dev".into()),
            None,
            "Lowongan backend engineer dengan pengalaman Rust, SQLite, dan Docker container.".into(),
        )
        .await
        .expect("ingest");

        let first = generate_gap_analysis_impl(&state, user_id.clone(), job.job_id.clone(), false)
            .await
            .expect("first gap");
        assert_eq!(gemini.gap_calls.load(Ordering::SeqCst), 1);

        let cached = generate_gap_analysis_impl(&state, user_id.clone(), job.job_id.clone(), false)
            .await
            .expect("cached gap");
        assert_eq!(cached, first);
        assert_eq!(
            gemini.gap_calls.load(Ordering::SeqCst),
            1,
            "cached gap analysis must NOT call Gemini again"
        );

        let _refreshed = generate_gap_analysis_impl(&state, user_id, job.job_id, true)
            .await
            .expect("force refresh");
        assert_eq!(gemini.gap_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn ingest_manual_job_caps_oversized_raw_text_to_12000_chars() {
        let gemini = Arc::new(MockGemini::default());
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::clone(&gemini),
            Arc::new(MockWorker::default()),
        );
        let user_id = seed_user(&state).await;

        let oversized = "a".repeat(20_000);
        let detail = ingest_manual_job_impl(
            &state,
            user_id,
            Some("Big JD".into()),
            None,
            oversized,
        )
        .await
        .expect("oversized manual JD should be capped and ingested");

        assert_eq!(detail.raw_description.chars().count(), 12_000);
        let sent_to_gemini = gemini
            .last_extract_description
            .lock()
            .unwrap()
            .clone()
            .expect("extract called");
        assert_eq!(sent_to_gemini.chars().count(), 12_000);
    }

    #[tokio::test]
    async fn run_scrape_rejects_unknown_source_emits_screaming_snake_warning_and_blocks_concurrent_run(
    ) {
        let gemini = Arc::new(MockGemini::default());
        let worker = Arc::new(MockWorker::default());
        *worker.response.lock().unwrap() = Ok(WorkerScrapeResponse {
            jobs: vec![WorkerScrapedJob {
                source: "linkedin".into(),
                external_id: "li-warn-1".into(),
                url: None,
                title: "Backend Dev".into(),
                company: "PT Warn".into(),
                location: Some("Jakarta".into()),
                raw_description: "Membangun layanan API menggunakan Rust dan SQLite lokal.".into(),
                posted_at: None,
                salary_text: None,
            }],
            stats: WorkerScrapeStats {
                total_fetched: 1,
                deduplicated_in_batch: 0,
                duration_ms: 10,
            },
            warnings: vec![],
        });

        let state = Arc::new(build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::clone(&gemini),
            Arc::clone(&worker),
        ));
        let user_id = seed_user(&state).await;

        // 1. Unknown source must be rejected as ValidationError
        let bad_src_err = run_scrape_for_active_keywords_impl(
            &state,
            user_id.clone(),
            vec!["unknown_portal".into()],
            5,
        )
        .await
        .expect_err("unknown source must fail");
        assert_eq!(bad_src_err.code, ErrorCode::ValidationError);

        // 2. Extraction warning code must be SCREAMING_SNAKE_CASE
        *gemini.fail_extract_error.lock().unwrap() =
            Some(AppError::gemini_rate_limited("429 from mock"));
        let report = run_scrape_for_active_keywords_impl(
            &state,
            user_id.clone(),
            vec!["linkedin".into()],
            5,
        )
        .await
        .expect("scrape completes with warning");
        assert_eq!(report.failed_extractions, 1);
        assert_eq!(report.warnings[0].code, "GEMINI_RATE_LIMITED");
        *gemini.fail_extract_error.lock().unwrap() = None;

        // 3. Concurrent double-click scrape must fail fast with ValidationError
        worker.scrape_delay_ms.store(120, Ordering::SeqCst);
        let s1 = Arc::clone(&state);
        let u1 = user_id.clone();
        let handle = tokio::spawn(async move {
            run_scrape_for_active_keywords_impl(&s1, u1, vec!["linkedin".into()], 5).await
        });
        tokio::time::sleep(Duration::from_millis(25)).await;
        let concurrent_err =
            run_scrape_for_active_keywords_impl(&state, user_id, vec!["linkedin".into()], 5)
                .await
                .expect_err("concurrent scrape must be rejected");
        assert_eq!(concurrent_err.code, ErrorCode::ValidationError);
        let first_res = handle.await.expect("join").expect("first scrape succeeds");
        assert_eq!(first_res.newly_ingested, 1);
    }

    #[test]
    fn sanitize_external_url_allows_http_https_and_drops_dangerous_schemes() {
        assert_eq!(
            sanitize_external_url(Some("https://linkedin.com/jobs/view/123")),
            Some("https://linkedin.com/jobs/view/123".into())
        );
        assert_eq!(
            sanitize_external_url(Some("  http://example.com/job  ")),
            Some("http://example.com/job".into())
        );
        assert_eq!(sanitize_external_url(Some("javascript:alert(1)")), None);
        assert_eq!(sanitize_external_url(Some("file:///etc/passwd")), None);
        assert_eq!(sanitize_external_url(Some("data:text/html,<h1>x</h1>")), None);
        assert_eq!(sanitize_external_url(Some("   ")), None);
        assert_eq!(sanitize_external_url(None), None);
    }

    #[tokio::test]
    async fn decoupled_stage1_stores_jobs_with_zero_gemini_calls_and_stage2_enriches_resumably() {
        let gemini = Arc::new(MockGemini::default());
        let worker = Arc::new(MockWorker::default());
        *worker.response.lock().unwrap() = Ok(WorkerScrapeResponse {
            jobs: vec![
                WorkerScrapedJob {
                    source: "jobstreet".into(),
                    external_id: "js-dec-1".into(),
                    url: Some("https://id.jobstreet.com/id/job/1".into()),
                    title: "Junior Backend Rust Engineer".into(),
                    company: "PT Lokal Satu".into(),
                    location: Some("Bandung".into()),
                    raw_description: "Dicari backend engineer Rust, SQLite, dan Docker secara remote."
                        .into(),
                    posted_at: Some("2026-11-24".into()),
                    salary_text: Some("Rp 10jt - 15jt".into()),
                },
                WorkerScrapedJob {
                    source: "indeed".into(),
                    external_id: "in-dec-2".into(),
                    url: Some("https://id.indeed.com/viewjob?jk=2".into()),
                    title: "Systems Engineer".into(),
                    company: "PT Lokal Dua".into(),
                    location: Some("Jakarta".into()),
                    raw_description: "Membangun CLI Rust dengan SQLite.".into(),
                    posted_at: None,
                    salary_text: None,
                },
            ],
            stats: WorkerScrapeStats {
                total_fetched: 2,
                deduplicated_in_batch: 0,
                duration_ms: 15,
            },
            warnings: vec![],
        });

        let state = build_test_state(None, Arc::clone(&gemini), Arc::clone(&worker));
        // Seed user WITH temporary key, then delete key to prove Stage 1 works with 0 key & 0 tokens!
        state.secrets.set_api_key("AIzaSySEED1234").expect("set");
        let user_id = seed_user(&state).await;
        let calls_after_seed = gemini.extract_calls.load(Ordering::SeqCst);
        let embeds_after_seed = gemini.embed_calls.load(Ordering::SeqCst);
        state.secrets.delete_api_key().expect("clear key");

        let prefs = SearchPreferences {
            location: "Bandung".into(),
            sources: vec!["jobstreet".into(), "indeed".into()],
            limit_per_query: 20,
            max_pages: 2,
            posted_within_days: Some(7),
            work_mode: "any".into(),
            custom_queries: vec!["Rust Engineer".into()],
            ai_enrich_enabled: false,
            ai_enrich_max_jobs: 10,
            ai_rpm_limit: 10,
        };

        // Stage 1: MUST succeed WITHOUT an API key and with 0 Gemini calls
        let report = scrape_and_store_jobs_impl(&state, None, user_id.clone(), prefs)
            .await
            .expect("stage 1 must succeed with 0 API key");
        assert_eq!(report.newly_ingested, 2);
        assert_eq!(report.pending_local_count, 2);
        assert_eq!(gemini.extract_calls.load(Ordering::SeqCst), calls_after_seed);
        assert_eq!(gemini.embed_calls.load(Ordering::SeqCst), embeds_after_seed);

        let listed = list_ranked_jobs_impl(&state, user_id.clone(), JobFilterInput::default())
            .expect("list local jobs");
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|j| j.enrichment_status == EnrichmentStatus::LocalOnly));
        assert!(listed[0].final_score > 0.0);

        // Dismiss second job BEFORE running AI -> Stage 2 must NOT waste tokens on dismissed jobs
        let dismissed = update_job_application_impl(
            &state,
            user_id.clone(),
            listed[1].job_id.clone(),
            ApplicationStatus::Dismissed,
            Some("Lokasi kurang cocok".into()),
        )
        .expect("dismiss");
        assert_eq!(dismissed.application_status, ApplicationStatus::Dismissed);

        let visible_after_dismiss =
            list_ranked_jobs_impl(&state, user_id.clone(), JobFilterInput::default())
                .expect("visible list");
        assert_eq!(visible_after_dismiss.len(), 1);

        // Stage 2: Restore API key and enrich remaining active queue (only 1 non-dismissed job!)
        state.secrets.set_api_key("AIzaSyTESTKEY1234").expect("restore key");
        let ai_report = run_ai_matching_queue_impl(&state, None, user_id.clone(), 10, 10)
            .await
            .expect("stage 2 ok");
        assert_eq!(ai_report.enriched_jobs, 1);
        assert_eq!(ai_report.remaining_local_jobs, 0);
        assert_eq!(ai_report.api_calls_used, 2); // 1 batch extract + 1 batch embed

        // Phase 3 & 4: Track Applied + Generate cached Apply Copilot kit
        let applied = update_job_application_impl(
            &state,
            user_id.clone(),
            visible_after_dismiss[0].job_id.clone(),
            ApplicationStatus::Applied,
            Some("Kirim CV via JobStreet".into()),
        )
        .expect("applied");
        assert_eq!(applied.application_status, ApplicationStatus::Applied);
        assert!(applied.applied_at.is_some());
        assert_eq!(applied.user_notes.as_deref(), Some("Kirim CV via JobStreet"));

        let kit = generate_apply_kit_impl(
            &state,
            user_id.clone(),
            visible_after_dismiss[0].job_id.clone(),
            "id".into(),
            false,
        )
        .await
        .expect("apply kit");
        assert_eq!(kit.language, "id");
        assert!(!kit.cv_bullets.is_empty());
        assert!(!kit.cover_note.is_empty());
    }
}
