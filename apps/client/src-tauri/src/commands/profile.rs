use tauri::State;

use crate::core::config::{MAX_KNOWN_SKILLS_CONTEXT, MIN_STORY_CHARS};
use crate::core::gemini::EmbeddingTaskType;
use crate::core::matching::{encode_embedding, encode_embedding_i8, format_profile_embedding_text};
use crate::core::reranker::spawn_background_rerank;
use crate::error::AppError;
use crate::models::profile::{
    ConfirmedProfileInput, ProdiRoleItem, ProfileSummaryItem, ScrapeKeywordItem, StoryProfileDraft,
    UserProfileDetail,
};
use crate::state::AppState;

pub async fn parse_user_story_impl(
    state: &AppState,
    raw_story: String,
) -> Result<StoryProfileDraft, AppError> {
    let trimmed = raw_story.trim();
    if trimmed.chars().count() < MIN_STORY_CHARS {
        return Err(AppError::validation_error(format!(
            "Cerita kemampuan minimal {MIN_STORY_CHARS} karakter agar AI dapat memetakan profil secara akurat."
        )));
    }

    let api_key = state.require_api_key()?;
    let known_skills = state.db.get_top_canonical_skills(MAX_KNOWN_SKILLS_CONTEXT)?;
    state
        .gemini
        .parse_user_story(&api_key, trimmed, &known_skills)
        .await
}

pub async fn save_confirmed_profile_impl(
    state: &AppState,
    input: ConfirmedProfileInput,
) -> Result<UserProfileDetail, AppError> {
    if input.profile_name.trim().is_empty() {
        return Err(AppError::validation_error(
            "Nama profil tidak boleh kosong.",
        ));
    }

    let skill_pairs: Vec<(String, String)> = input
        .skills
        .iter()
        .map(|s| (s.canonical_name.clone(), s.proficiency.as_str().to_string()))
        .collect();
    let embed_input = format_profile_embedding_text(
        &input.target_roles,
        input.experience_level,
        &input.summary,
        &skill_pairs,
    );

    // Zero-API-key ready: use Gemini embedding if BYOK key exists, otherwise compute offline multilingual embedding locally.
    let embedding_bytes = match state.require_api_key() {
        Ok(api_key) => {
            let embedding_vec = state
                .gemini
                .embed_text(&api_key, &embed_input, EmbeddingTaskType::RetrievalQuery)
                .await?;
            Some(encode_embedding(&embedding_vec))
        }
        Err(_) => state
            .local_embedder
            .embed_query(&embed_input)
            .map(|v| encode_embedding(&v)),
    };

    let saved = state
        .db
        .save_profile_atomic(&input, embedding_bytes.as_deref())?;

    if state.local_embedder.is_neural_v2() {
        let role_text = input.target_roles.join(", ");
        let skill_text = input
            .skills
            .iter()
            .map(|s| s.canonical_name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let story_text = if input.raw_story_text.trim().is_empty() {
            input.summary.as_str()
        } else {
            input.raw_story_text.as_str()
        };

        let emb_role = if role_text.trim().is_empty() {
            None
        } else {
            state
                .local_embedder
                .embed_query(&role_text)
                .map(|v| encode_embedding_i8(&v))
        };
        let emb_skills = if skill_text.trim().is_empty() {
            None
        } else {
            state
                .local_embedder
                .embed_query(&skill_text)
                .map(|v| encode_embedding_i8(&v))
        };
        let emb_story = if story_text.trim().is_empty() {
            None
        } else {
            state
                .local_embedder
                .embed_query(story_text)
                .map(|v| encode_embedding_i8(&v))
        };
        let dom_probe = format!("{} {} {}", role_text, input.summary, skill_text);
        let dominant_domain =
            vecta_taxonomy::TaxonomyEngine::global().extract_from_profile_text(&dom_probe).dominant_domain;

        let _ = state.db.update_profile_multifacet_embeddings(
            &saved.id,
            emb_role.as_deref(),
            emb_skills.as_deref(),
            emb_story.as_deref(),
            Some(dominant_domain.as_str()),
        );
    }

    state.recalculate_all_matches_for_user(&saved.id)?;
    Ok(saved)
}

pub fn get_active_profile_impl(state: &AppState) -> Result<Option<UserProfileDetail>, AppError> {
    state.db.get_active_profile()
}

pub fn toggle_scrape_keyword_impl(
    state: &AppState,
    keyword_id: i64,
    is_active: bool,
) -> Result<(), AppError> {
    state.db.toggle_scrape_keyword(keyword_id, is_active)
}

pub fn add_custom_keyword_impl(
    state: &AppState,
    user_id: String,
    role_label: String,
    queries: Vec<String>,
) -> Result<ScrapeKeywordItem, AppError> {
    state.db.add_custom_keyword(&user_id, &role_label, &queries)
}

pub fn list_prodi_roles_catalog_impl(
    query: Option<String>,
) -> Result<Vec<ProdiRoleItem>, AppError> {
    let q = query.unwrap_or_default().trim().to_lowercase();
    let catalog = vecta_taxonomy::TaxonomyEngine::global().roles_catalog();
    let items: Vec<ProdiRoleItem> = catalog
        .iter()
        .filter(|entry| {
            if q.is_empty() {
                return true;
            }
            entry.prodi_label.to_lowercase().contains(&q)
                || entry.role_title.to_lowercase().contains(&q)
                || entry.domain.to_lowercase().contains(&q)
        })
        .map(|entry| ProdiRoleItem {
            id: entry.id.clone(),
            prodi_label: entry.prodi_label.clone(),
            domain: entry.domain.clone(),
            role_title: entry.role_title.clone(),
            mandatory_skills: entry.mandatory_skills.clone(),
            preferred_skills: entry.preferred_skills.clone(),
            certifications: entry.certifications.clone(),
            scrape_queries: entry.scrape_queries.clone(),
        })
        .collect();
    Ok(items)
}

pub fn list_user_profiles_impl(state: &AppState) -> Result<Vec<ProfileSummaryItem>, AppError> {
    state.db.list_user_profiles()
}

pub fn switch_active_profile_impl(
    state: &AppState,
    profile_id: String,
) -> Result<UserProfileDetail, AppError> {
    let detail = state.db.switch_active_profile(&profile_id)?;
    state.recalculate_all_matches_for_user(&detail.id)?;
    Ok(detail)
}

pub fn delete_user_profile_impl(
    state: &AppState,
    profile_id: String,
) -> Result<UserProfileDetail, AppError> {
    let remaining_active = state.db.delete_user_profile(&profile_id)?;
    state.recalculate_all_matches_for_user(&remaining_active.id)?;
    Ok(remaining_active)
}

#[tauri::command]
#[specta::specta]
pub async fn parse_user_story(
    state: State<'_, AppState>,
    raw_story: String,
) -> Result<StoryProfileDraft, AppError> {
    parse_user_story_impl(&state, raw_story).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_confirmed_profile(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: ConfirmedProfileInput,
) -> Result<UserProfileDetail, AppError> {
    let saved = save_confirmed_profile_impl(&state, input).await?;
    if state.local_embedder.has_reranker() {
        spawn_background_rerank(app, saved.id.clone(), 25);
    }
    Ok(saved)
}

#[tauri::command]
#[specta::specta]
pub fn get_active_profile(
    state: State<'_, AppState>,
) -> Result<Option<UserProfileDetail>, AppError> {
    get_active_profile_impl(&state)
}

#[tauri::command]
#[specta::specta]
pub fn toggle_scrape_keyword(
    state: State<'_, AppState>,
    keyword_id: i64,
    is_active: bool,
) -> Result<(), AppError> {
    toggle_scrape_keyword_impl(&state, keyword_id, is_active)
}

#[tauri::command]
#[specta::specta]
pub fn add_custom_keyword(
    state: State<'_, AppState>,
    user_id: String,
    role_label: String,
    queries: Vec<String>,
) -> Result<ScrapeKeywordItem, AppError> {
    add_custom_keyword_impl(&state, user_id, role_label, queries)
}

#[tauri::command]
#[specta::specta]
pub fn list_prodi_roles_catalog(query: Option<String>) -> Result<Vec<ProdiRoleItem>, AppError> {
    list_prodi_roles_catalog_impl(query)
}

#[tauri::command]
#[specta::specta]
pub fn list_user_profiles(state: State<'_, AppState>) -> Result<Vec<ProfileSummaryItem>, AppError> {
    list_user_profiles_impl(&state)
}

#[tauri::command]
#[specta::specta]
pub fn switch_active_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<UserProfileDetail, AppError> {
    switch_active_profile_impl(&state, profile_id)
}

#[tauri::command]
#[specta::specta]
pub fn delete_user_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<UserProfileDetail, AppError> {
    delete_user_profile_impl(&state, profile_id)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::core::db::NewJobRecord;
    use crate::core::matching::encode_embedding;
    use crate::error::ErrorCode;
    use crate::models::job::{
        ExtractedJobSkill, JobFilterInput, JobSource, RequirementType, WorkMode,
    };
    use crate::models::profile::{
        ConfirmedKeywordInput, ConfirmedSkillInput, ExperienceLevel, FitType, Proficiency,
        SkillCategory,
    };
    use crate::state::test_support::{build_test_state, MockGemini, MockWorker};

    #[tokio::test]
    async fn parse_user_story_enforces_min_30_chars_and_requires_key() {
        let state_no_key = build_test_state(
            None,
            Arc::new(MockGemini::default()),
            Arc::new(MockWorker::default()),
        );

        let err_short = parse_user_story_impl(&state_no_key, "terlalu pendek".into())
            .await
            .expect_err("short story must fail validation");
        assert_eq!(err_short.code, ErrorCode::ValidationError);

        let err_no_key = parse_user_story_impl(
            &state_no_key,
            "Saya sudah membangun beberapa proyek backend Rust dan SQLite selama 1 tahun.".into(),
        )
        .await
        .expect_err("missing key must fail");
        assert_eq!(err_no_key.code, ErrorCode::ApiKeyMissing);

        let state_ok = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::new(MockGemini::default()),
            Arc::new(MockWorker::default()),
        );
        let draft = parse_user_story_impl(
            &state_ok,
            "Saya sudah membangun beberapa proyek backend Rust dan SQLite selama 1 tahun.".into(),
        )
        .await
        .expect("parse story succeeds");
        assert!(!draft.extracted_skills.is_empty());
        assert_eq!(
            get_active_profile_impl(&state_ok).expect("active profile"),
            None,
            "parse_user_story must NOT persist to DB"
        );
    }

    #[tokio::test]
    async fn save_confirmed_profile_persists_atomically_embeds_and_recalculates_matches() {
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::new(MockGemini::default()),
            Arc::new(MockWorker::default()),
        );

        let job_emb = encode_embedding(&vec![0.1f32; 768]);
        for idx in 1..=2 {
            state
                .db
                .insert_job_with_skills(
                    &NewJobRecord {
                        id: format!("job-{idx}"),
                        source: JobSource::Linkedin,
                        external_id: format!("li-{idx}"),
                        url: None,
                        title: format!("Backend Engineer #{idx}"),
                        normalized_role: "Backend Engineer".into(),
                        company: "PT Vecta".into(),
                        location: Some("Jakarta".into()),
                        work_mode: WorkMode::Hybrid,
                        experience_level: ExperienceLevel::Junior,
                        raw_description: "Rust SQLite API".into(),
                        extracted_summary_json: None,
                        embedding: Some(job_emb.clone()),
                        enrichment_status: crate::models::job::EnrichmentStatus::AiEnriched,
                        posted_at: None,
                        salary_text: None,
                        domain_slug: "informatika_data".into(),
                        quality_score: 1.0,
                        red_flags_json: "[]".into(),
                        also_available_on_json: "[]".into(),
                        embedding_model: None,
                    },
                    &[ExtractedJobSkill {
                        canonical_name: "Rust".into(),
                        category: SkillCategory::CoreHard,
                        requirement_type: RequirementType::Mandatory,
                        matched_aliases: vec![],
                    }],
                )
                .expect("seed job");
        }

        let saved = save_confirmed_profile_impl(
            &state,
            ConfirmedProfileInput {
                id: None,
                profile_name: "Profil Utama".into(),
                target_roles: vec!["Backend Engineer".into()],
                raw_story_text: "Saya berpengalaman membangun REST API dengan Rust dan SQLite."
                    .into(),
                summary: "Junior Rust Backend Developer".into(),
                experience_level: ExperienceLevel::Junior,
                skills: vec![ConfirmedSkillInput {
                    canonical_name: "Rust".into(),
                    category: SkillCategory::CoreHard,
                    proficiency: Proficiency::Intermediate,
                    evidence: Some("Vecta".into()),
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
        .expect("save profile");

        assert!(saved.has_embedding);
        let rec = state
            .db
            .get_profile_for_matching(&saved.id)
            .expect("match record")
            .expect("exists");
        assert_eq!(rec.embedding.as_ref().map(|b| b.len()), Some(768 * 4));

        let ranked = state
            .db
            .list_ranked_jobs(&saved.id, &JobFilterInput::default())
            .expect("ranked jobs");
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].final_score > 0.9);
    }

    #[tokio::test]
    async fn add_custom_keyword_and_toggle_updates_active_flag() {
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::new(MockGemini::default()),
            Arc::new(MockWorker::default()),
        );
        let saved = save_confirmed_profile_impl(
            &state,
            ConfirmedProfileInput {
                id: None,
                profile_name: "Profil".into(),
                target_roles: vec!["Backend Engineer".into()],
                raw_story_text: "Cerita pengalaman pengembangan perangkat lunak lokal.".into(),
                summary: "Backend Dev".into(),
                experience_level: ExperienceLevel::Junior,
                skills: vec![],
                keywords: vec![],
            },
        )
        .await
        .expect("save");

        let kw = add_custom_keyword_impl(
            &state,
            saved.id.clone(),
            "Cloud Security Engineer".into(),
            vec![],
        )
        .expect("add custom keyword");
        assert_eq!(kw.fit_type, FitType::Custom);
        assert!(kw.is_active);
        assert_eq!(kw.scrape_queries, vec!["Cloud Security Engineer"]);

        toggle_scrape_keyword_impl(&state, kw.id, false).expect("toggle off");
        let active = get_active_profile_impl(&state)
            .expect("get active")
            .expect("exists");
        assert!(!active.keywords[0].is_active);
    }
}
