use tauri::State;

use crate::error::AppError;
use crate::models::competency::{CompetencyMapResponse, RoleJobCount};
use crate::state::AppState;

pub fn list_available_roles_impl(state: &AppState) -> Result<Vec<RoleJobCount>, AppError> {
    state.db.list_available_roles()
}

pub fn get_competency_map_impl(
    state: &AppState,
    user_id: String,
    target_role: String,
) -> Result<CompetencyMapResponse, AppError> {
    if target_role.trim().is_empty() {
        return Err(AppError::validation_error(
            "Peran target tidak boleh kosong.",
        ));
    }
    state.db.get_competency_map(&user_id, &target_role)
}

#[tauri::command]
#[specta::specta]
pub fn list_available_roles(state: State<'_, AppState>) -> Result<Vec<RoleJobCount>, AppError> {
    list_available_roles_impl(&state)
}

#[tauri::command]
#[specta::specta]
pub fn get_competency_map(
    state: State<'_, AppState>,
    user_id: String,
    target_role: String,
) -> Result<CompetencyMapResponse, AppError> {
    get_competency_map_impl(&state, user_id, target_role)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::commands::profile::save_confirmed_profile_impl;
    use crate::core::db::NewJobRecord;
    use crate::models::job::{ExtractedJobSkill, JobSource, RequirementType, WorkMode};
    use crate::models::profile::{
        ConfirmedProfileInput, ConfirmedSkillInput, ExperienceLevel, Proficiency, SkillCategory,
    };
    use crate::state::test_support::{build_test_state, MockGemini, MockWorker};

    #[tokio::test]
    async fn competency_commands_list_roles_and_map_ownership() {
        let state = build_test_state(
            Some("AIzaSyTESTKEY1234"),
            Arc::new(MockGemini::default()),
            Arc::new(MockWorker::default()),
        );
        let profile = save_confirmed_profile_impl(
            &state,
            ConfirmedProfileInput {
                id: Some("u1".into()),
                profile_name: "Primary".into(),
                target_roles: vec!["Backend Engineer".into()],
                raw_story_text: "Berpengalaman membuat layanan backend Rust dan SQLite lokal."
                    .into(),
                summary: "Rust Dev".into(),
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
        .expect("save profile");

        for (id, role) in [
            ("j1", "Backend Engineer"),
            ("j2", "Backend Engineer"),
            ("j3", "Data Analyst"),
        ] {
            state
                .db
                .insert_job_with_skills(
                    &NewJobRecord {
                        id: id.into(),
                        source: JobSource::Linkedin,
                        external_id: format!("ext-{id}"),
                        url: None,
                        title: role.into(),
                        normalized_role: role.into(),
                        company: "PT Vecta".into(),
                        location: None,
                        work_mode: WorkMode::Remote,
                        experience_level: ExperienceLevel::Junior,
                        raw_description: "desc".into(),
                        extracted_summary_json: None,
                        embedding: None,
                    },
                    &[
                        ExtractedJobSkill {
                            canonical_name: "Rust".into(),
                            category: SkillCategory::CoreHard,
                            requirement_type: RequirementType::Mandatory,
                            matched_aliases: vec![],
                        },
                        ExtractedJobSkill {
                            canonical_name: "Docker".into(),
                            category: SkillCategory::ToolsTech,
                            requirement_type: RequirementType::Mandatory,
                            matched_aliases: vec![],
                        },
                    ],
                )
                .expect("seed job");
        }

        let roles = list_available_roles_impl(&state).expect("roles");
        assert_eq!(roles.len(), 2);
        assert_eq!(roles[0].normalized_role, "Backend Engineer");
        assert_eq!(roles[0].job_count, 2);

        let map = get_competency_map_impl(&state, profile.id, "Backend Engineer".into())
            .expect("competency map");
        assert_eq!(map.total_jobs, 2);
        let rust = map
            .skills
            .iter()
            .find(|s| s.canonical_name == "Rust")
            .unwrap();
        let docker = map
            .skills
            .iter()
            .find(|s| s.canonical_name == "Docker")
            .unwrap();
        assert!(rust.is_owned_by_user);
        assert!(!docker.is_owned_by_user);
    }
}
