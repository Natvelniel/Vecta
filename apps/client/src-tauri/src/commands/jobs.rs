use std::time::Duration;

use sha2::{Digest, Sha256};
use tauri::State;
use uuid::Uuid;

use crate::core::config::{MAX_KNOWN_SKILLS_CONTEXT, MIN_MANUAL_JOB_CHARS};
use crate::core::db::NewJobRecord;
use crate::core::gemini::{EmbeddingTaskType, GapAnalysisPromptInput};
use crate::core::matching::{
    compute_hybrid_match, encode_embedding, format_job_embedding_text, to_match_upsert,
};
use crate::core::worker::WorkerScrapeRequest;
use crate::error::{AppError, ErrorCode};
use crate::models::job::{
    GapAnalysis, JobFilterInput, JobMatchDetail, JobMatchSummary, JobSource, RequirementType,
    ScrapeRunReport, ScrapeWarningItem,
};
use crate::state::AppState;

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

    let profile = state
        .db
        .get_profile_for_matching(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;

    let clean_url = url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let external_id = compute_manual_external_id(clean_url.as_deref(), trimmed_text);

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
        .extract_job(&api_key, trimmed_text, &known_skills)
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
        raw_description: trimmed_text.to_string(),
        extracted_summary_json: Some(extracted_json),
        embedding: Some(embedding_bytes),
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

    let worker_req = WorkerScrapeRequest {
        queries,
        sources: clean_sources,
        location: "Indonesia".to_string(),
        limit_per_query: limit_per_query.clamp(1, 50),
    };
    let worker_resp = state.worker.scrape(&state.worker_url, &worker_req).await?;

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
                    code: format!("{:?}", err.code),
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
                    code: format!("{:?}", err.code),
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
        let new_job = NewJobRecord {
            id: job_id.clone(),
            source,
            external_id: scraped_job.external_id,
            url: scraped_job.url,
            title: scraped_job.title,
            normalized_role: extracted.normalized_role.clone(),
            company: scraped_job.company,
            location: scraped_job.location,
            work_mode: extracted.work_mode,
            experience_level: extracted.experience_level,
            raw_description: scraped_job.raw_description,
            extracted_summary_json: Some(extracted_json),
            embedding: embedding_bytes,
        };

        let job_match_rec = state.db.insert_job_with_skills(&new_job, &extracted.skills)?;
        let computed = compute_hybrid_match(&profile, &job_match_rec);
        let upsert = to_match_upsert(&user_id, &job_id, &computed)?;
        state.db.upsert_job_matches(&[upsert])?;
        newly_ingested += 1;
    }

    state.db.touch_active_keywords_scraped(&user_id)?;

    Ok(ScrapeRunReport {
        total_fetched,
        newly_ingested,
        skipped_duplicates,
        failed_extractions,
        warnings,
    })
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
                },
                WorkerScrapedJob {
                    source: "glints".into(),
                    external_id: "gl-202".into(),
                    url: Some("https://glints.com/jobs/202".into()),
                    title: "Systems Developer".into(),
                    company: "PT Beta".into(),
                    location: Some("Bandung".into()),
                    raw_description: "Membangun desktop engine lokal dengan Rust dan Docker.".into(),
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
}
