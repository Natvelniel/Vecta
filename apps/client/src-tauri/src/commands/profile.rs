use tauri::State;

use crate::core::config::{MAX_KNOWN_SKILLS_CONTEXT, MIN_STORY_CHARS};
use crate::core::gemini::EmbeddingTaskType;
use crate::core::matching::{encode_embedding, format_profile_embedding_text};
use crate::error::AppError;
use crate::models::profile::{
    ConfirmedProfileInput, ScrapeKeywordItem, StoryProfileDraft, UserProfileDetail,
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

    let api_key = state.require_api_key()?;
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

    let embedding_vec = state
        .gemini
        .embed_text(&api_key, &embed_input, EmbeddingTaskType::RetrievalQuery)
        .await?;
    let embedding_bytes = encode_embedding(&embedding_vec);

    let saved = state
        .db
        .save_profile_atomic(&input, Some(&embedding_bytes))?;
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
    state: State<'_, AppState>,
    input: ConfirmedProfileInput,
) -> Result<UserProfileDetail, AppError> {
    save_confirmed_profile_impl(&state, input).await
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
