use tauri::State;
use vecta_taxonomy::TaxonomyEngine;

use crate::core::gemini::RoadmapPromptInput;
use crate::core::roadmap_local::generate_local_roadmap;
use crate::error::AppError;
use crate::models::roadmap::{RoadmapDetail, RoadmapSummary, StepStatus, StepUpdateResult};
use crate::state::AppState;

pub async fn generate_learning_roadmap_impl(
    state: &AppState,
    user_id: String,
    target_role: String,
    source_job_id: Option<String>,
    hours_per_week: u32,
) -> Result<RoadmapDetail, AppError> {
    let role = target_role.trim();
    if role.is_empty() {
        return Err(AppError::validation_error(
            "Peran target roadmap tidak boleh kosong.",
        ));
    }
    if hours_per_week == 0 {
        return Err(AppError::validation_error(
            "Alokasi jam belajar per minggu minimal 1 jam.",
        ));
    }

    let profile = state
        .db
        .get_profile_by_id(&user_id)?
        .ok_or_else(|| {
            AppError::validation_error(format!("Profil pengguna '{user_id}' tidak ditemukan."))
        })?;

    let clean_source_job_id = source_job_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let (missing_demands, source_job_title) = match clean_source_job_id {
        Some(jid) => {
            let missing = state.db.get_missing_skills_for_job(&user_id, jid)?;
            let title = state
                .db
                .get_job_detail(&user_id, jid)
                .ok()
                .map(|d| format!("{} di {}", d.title, d.company));
            (missing, title)
        }
        None => {
            let missing = state.db.get_top_missing_skills_for_role(&user_id, role, 8)?;
            (missing, None)
        }
    };

    let missing_skill_names: Vec<String> = missing_demands
        .into_iter()
        .map(|m| m.canonical_name)
        .collect();

    if missing_skill_names.is_empty() {
        let msg = if clean_source_job_id.is_some() {
            "Tidak ada skill gap untuk lowongan ini: Anda sudah menguasai seluruh skill yang terdeteksi.".to_string()
        } else {
            format!(
                "Tidak ada skill gap untuk peran '{role}'. Pastikan sudah ada lowongan untuk peran tersebut atau Anda sudah menguasai seluruh skill utamanya."
            )
        };
        return Err(AppError::validation_error(msg));
    }

    let generated = match state.require_api_key() {
        Ok(api_key) => {
            let prompt_input = RoadmapPromptInput {
                target_role: role.to_string(),
                user_level: profile.experience_level.as_str().to_string(),
                user_summary: profile.summary.unwrap_or_default(),
                missing_skills: missing_skill_names.clone(),
                hours_per_week,
                source_job_title,
            };
            match state.gemini.generate_roadmap(&api_key, &prompt_input).await {
                Ok(ai_roadmap) => ai_roadmap,
                Err(_) => generate_local_roadmap(
                    role,
                    &missing_skill_names,
                    hours_per_week,
                    TaxonomyEngine::global(),
                ),
            }
        }
        Err(_) => generate_local_roadmap(
            role,
            &missing_skill_names,
            hours_per_week,
            TaxonomyEngine::global(),
        ),
    };

    state
        .db
        .insert_roadmap_with_steps(&user_id, clean_source_job_id, &generated)
}

pub fn list_roadmaps_impl(
    state: &AppState,
    user_id: String,
) -> Result<Vec<RoadmapSummary>, AppError> {
    state.db.list_roadmaps(&user_id)
}

pub fn get_roadmap_detail_impl(
    state: &AppState,
    roadmap_id: String,
) -> Result<RoadmapDetail, AppError> {
    state.db.get_roadmap_detail(&roadmap_id)
}

pub fn update_roadmap_step_status_impl(
    state: &AppState,
    step_id: String,
    status: StepStatus,
) -> Result<StepUpdateResult, AppError> {
    state.db.update_roadmap_step_status(&step_id, status)
}

pub fn quick_add_user_skill_impl(
    state: &AppState,
    user_id: String,
    skill_id: i64,
) -> Result<(), AppError> {
    state.db.quick_add_user_skill_row(&user_id, skill_id)?;
    state.recalculate_all_matches_for_user(&user_id)
}

#[tauri::command]
#[specta::specta]
pub async fn generate_learning_roadmap(
    state: State<'_, AppState>,
    user_id: String,
    target_role: String,
    source_job_id: Option<String>,
    hours_per_week: u32,
) -> Result<RoadmapDetail, AppError> {
    generate_learning_roadmap_impl(&state, user_id, target_role, source_job_id, hours_per_week)
        .await
}

#[tauri::command]
#[specta::specta]
pub fn list_roadmaps(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<RoadmapSummary>, AppError> {
    list_roadmaps_impl(&state, user_id)
}

#[tauri::command]
#[specta::specta]
pub fn get_roadmap_detail(
    state: State<'_, AppState>,
    roadmap_id: String,
) -> Result<RoadmapDetail, AppError> {
    get_roadmap_detail_impl(&state, roadmap_id)
}

#[tauri::command]
#[specta::specta]
pub fn update_roadmap_step_status(
    state: State<'_, AppState>,
    step_id: String,
    status: StepStatus,
) -> Result<StepUpdateResult, AppError> {
    update_roadmap_step_status_impl(&state, step_id, status)
}

#[tauri::command]
#[specta::specta]
pub fn quick_add_user_skill(
    state: State<'_, AppState>,
    user_id: String,
    skill_id: i64,
) -> Result<(), AppError> {
    quick_add_user_skill_impl(&state, user_id, skill_id)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use super::*;
    use crate::commands::jobs::{get_job_detail_impl, ingest_manual_job_impl};
    use crate::commands::profile::save_confirmed_profile_impl;
    use crate::models::profile::{
        ConfirmedProfileInput, ConfirmedSkillInput, ExperienceLevel, Proficiency, SkillCategory,
    };
    use crate::state::test_support::{build_test_state, MockGemini, MockWorker};

    async fn setup_user_and_job(state: &AppState) -> (String, String) {
        let profile = save_confirmed_profile_impl(
            state,
            ConfirmedProfileInput {
                id: Some("user-rm".into()),
                profile_name: "Primary".into(),
                target_roles: vec!["Backend Engineer".into()],
                raw_story_text: "Berpengalaman membuat REST API dengan Rust dan SQLite.".into(),
                summary: "Junior Rust Developer".into(),
                experience_level: ExperienceLevel::Junior,
                skills: vec![ConfirmedSkillInput {
                    canonical_name: "Rust".into(),
                    category: SkillCategory::CoreHard,
                    proficiency: Proficiency::Intermediate,
                    evidence: None,
                }],
                keywords: vec![],
            },
        )
        .await
        .expect("seed user");

        let job = ingest_manual_job_impl(
            state,
            profile.id.clone(),
            Some("Backend Engineer".into()),
            None,
            "Dibutuhkan Backend Engineer yang menguasai Rust dan Docker untuk deployment.".into(),
        )
        .await
        .expect("seed job");

        (profile.id, job.job_id)
    }

    #[tokio::test]
    async fn generate_roadmap_saves_header_and_linked_steps() {
        let gemini = Arc::new(MockGemini::default());
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::clone(&gemini),
            Arc::new(MockWorker::default()),
        );
        let (user_id, _job_id) = setup_user_and_job(&state).await;

        let detail = generate_learning_roadmap_impl(
            &state,
            user_id.clone(),
            "Backend Engineer".into(),
            None,
            8,
        )
        .await
        .expect("generate roadmap");

        assert_eq!(detail.steps.len(), 2);
        assert!(detail.steps[0].skill_id.is_some());
        assert_eq!(detail.steps[0].phase_order, 1);
        assert_eq!(detail.steps[1].phase_order, 2);

        let list = list_roadmaps_impl(&state, user_id).expect("list roadmaps");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].total_steps, 2);
        assert_eq!(list[0].completed_steps, 0);
        assert!((list[0].progress_percentage - 0.0).abs() < 1e-6);

        let fetched = get_roadmap_detail_impl(&state, detail.id.clone()).expect("get detail");
        assert_eq!(fetched.id, detail.id);
    }

    #[tokio::test]
    async fn update_step_status_done_suggests_unowned_skill_and_quick_add_recalculates_matches() {
        let gemini = Arc::new(MockGemini::default());
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::clone(&gemini),
            Arc::new(MockWorker::default()),
        );
        let (user_id, job_id) = setup_user_and_job(&state).await;

        let score_before = get_job_detail_impl(&state, user_id.clone(), job_id.clone())
            .expect("before")
            .final_score;

        let roadmap = generate_learning_roadmap_impl(
            &state,
            user_id.clone(),
            "Backend Engineer".into(),
            Some(job_id.clone()),
            6,
        )
        .await
        .expect("generate roadmap");

        let step1_id = roadmap.steps[0].id.clone();
        let step2_id = roadmap.steps[1].id.clone();

        let update1 = update_roadmap_step_status_impl(&state, step1_id, StepStatus::Done)
            .expect("mark done");
        assert_eq!(update1.status, StepStatus::Done);
        assert!(update1.completed_at.is_some());
        let suggested = update1
            .suggest_add_skill
            .expect("should suggest adding unowned Docker skill");
        assert_eq!(suggested.canonical_name, "Docker");

        let embed_calls_before = gemini.embed_calls.load(Ordering::SeqCst);
        quick_add_user_skill_impl(&state, user_id.clone(), suggested.skill_id)
            .expect("quick add skill");
        assert_eq!(
            gemini.embed_calls.load(Ordering::SeqCst),
            embed_calls_before,
            "quick_add_user_skill must recalculate locally with 0 Gemini calls"
        );

        let score_after = get_job_detail_impl(&state, user_id.clone(), job_id)
            .expect("after")
            .final_score;
        assert!(
            score_after > score_before,
            "score should jump after adding missing skill ({score_before} -> {score_after})"
        );

        let update2 = update_roadmap_step_status_impl(&state, step2_id, StepStatus::Done)
            .expect("mark step 2 done");
        assert_eq!(update2.suggest_add_skill, None);
    }

    #[tokio::test]
    async fn generate_roadmap_rejects_when_no_missing_skills_exist_without_calling_gemini() {
        use crate::error::ErrorCode;

        let gemini = Arc::new(MockGemini::default());
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::clone(&gemini),
            Arc::new(MockWorker::default()),
        );
        let (user_id, _job_id) = setup_user_and_job(&state).await;

        let err = generate_learning_roadmap_impl(
            &state,
            user_id,
            "Role Tanpa Lowongan".into(),
            None,
            6,
        )
        .await
        .expect_err("empty missing skills must return ValidationError");
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert_eq!(
            gemini.roadmap_calls.load(Ordering::SeqCst),
            0,
            "must NOT call Gemini when missing skill list is empty"
        );
    }

    #[tokio::test]
    async fn generate_roadmap_succeeds_locally_without_api_key() {
        use crate::core::db::NewJobRecord;
        use crate::models::job::{
            EnrichmentStatus, ExtractedJobSkill, JobSource, RequirementType, WorkMode,
        };

        let gemini = Arc::new(MockGemini::default());
        let state = build_test_state(
            None,
            Arc::clone(&gemini),
            Arc::new(MockWorker::default()),
        );
        let profile = save_confirmed_profile_impl(
            &state,
            ConfirmedProfileInput {
                id: Some("user-rm-local".into()),
                profile_name: "Primary Local".into(),
                target_roles: vec!["Backend Engineer".into()],
                raw_story_text: "Berpengalaman membuat REST API dengan Rust dan SQLite.".into(),
                summary: "Junior Rust Developer".into(),
                experience_level: ExperienceLevel::Junior,
                skills: vec![ConfirmedSkillInput {
                    canonical_name: "Rust".into(),
                    category: SkillCategory::CoreHard,
                    proficiency: Proficiency::Intermediate,
                    evidence: None,
                }],
                keywords: vec![],
            },
        )
        .await
        .expect("seed local user");

        let job_id = "job-local-rm".to_string();
        state
            .db
            .insert_job_with_skills(
                &NewJobRecord {
                    id: job_id.clone(),
                    source: JobSource::Kalibrr,
                    external_id: "kal-local-rm".into(),
                    url: None,
                    title: "Backend Engineer".into(),
                    normalized_role: "Backend Engineer".into(),
                    company: "PT Lokal".into(),
                    location: Some("Jakarta".into()),
                    work_mode: WorkMode::Remote,
                    experience_level: ExperienceLevel::Junior,
                    raw_description: "Dibutuhkan Backend Engineer Rust dan Docker.".into(),
                    extracted_summary_json: None,
                    embedding: None,
                    enrichment_status: EnrichmentStatus::LocalOnly,
                    posted_at: None,
                    salary_text: None,
                    domain_slug: "informatika_data".into(),
                    quality_score: 1.0,
                    red_flags_json: "[]".into(),
                    also_available_on_json: "[]".into(),
                    embedding_model: None,
                },
                &[ExtractedJobSkill {
                    canonical_name: "Docker".into(),
                    category: SkillCategory::ToolsTech,
                    requirement_type: RequirementType::Mandatory,
                    matched_aliases: vec![],
                }],
            )
            .expect("seed local job");

        let detail = generate_learning_roadmap_impl(
            &state,
            profile.id,
            "Backend Engineer".into(),
            Some(job_id),
            6,
        )
        .await
        .expect("local roadmap must succeed with 0 API keys");

        assert_eq!(gemini.roadmap_calls.load(Ordering::SeqCst), 0);
        assert!(detail.title.contains("Lokal · 0 Token"));
        assert!(!detail.steps.is_empty());
        assert!(detail.steps[0].skill_id.is_some());
    }
}
