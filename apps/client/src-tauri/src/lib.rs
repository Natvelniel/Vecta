pub mod commands;
pub mod core;
pub mod error;
pub mod models;
pub mod state;

use std::path::PathBuf;

use specta_typescript::Typescript;
use tauri::Manager;
use tauri_specta::{collect_commands, Builder, ErrorHandlingMode};

use crate::commands::{competency, cv, jobs, profile, roadmap, settings};
use crate::core::db::Database;
use crate::state::AppState;

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .error_handling(ErrorHandlingMode::Result)
        .dangerously_cast_bigints_to_number()
        .commands(collect_commands![
        // Settings & Offline AI (9)
        settings::get_api_key_status,
        settings::save_and_test_api_key,
        settings::delete_api_key,
        settings::check_worker_health,
        settings::get_search_preferences,
        settings::save_search_preferences,
        settings::get_local_ai_status,
        settings::get_scraper_mode,
        settings::set_scraper_mode,
        // Profile & Keywords (5)
        profile::parse_user_story,
        profile::save_confirmed_profile,
        profile::get_active_profile,
        profile::toggle_scrape_keyword,
        profile::add_custom_keyword,
        // CV Import, Master CV & ATS PDF Export (5)
        cv::import_cv_pdf,
        cv::parse_story_local,
        cv::get_master_cv,
        cv::save_master_cv,
        cv::export_tailored_ats_pdf,
        // Jobs, Decoupled AI Queue, Tracker & Apply Copilot (11)
        jobs::ingest_manual_job,
        jobs::run_scrape_for_active_keywords,
        jobs::scrape_and_store_jobs,
        jobs::run_ai_matching_queue,
        jobs::enrich_job_with_ai,
        jobs::list_ranked_jobs,
        jobs::get_job_detail,
        jobs::toggle_save_job,
        jobs::update_job_application,
        jobs::generate_gap_analysis,
        jobs::generate_apply_kit,
        // Competency Map (2)
        competency::list_available_roles,
        competency::get_competency_map,
        // Roadmap (5)
        roadmap::generate_learning_roadmap,
        roadmap::list_roadmaps,
        roadmap::get_roadmap_detail,
        roadmap::update_roadmap_step_status,
        roadmap::quick_add_user_skill,
    ])
}

pub fn typescript_exporter() -> Typescript {
    Typescript::default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    builder
        .export(typescript_exporter(), "../src/lib/bindings.ts")
        .expect("Failed to export TypeScript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve Tauri app_data_dir");
            let db_path = data_dir.join("vecta.db");
            let db = Database::open(&db_path).expect("failed to initialize Vecta SQLite database");

            let mut resource_dirs: Vec<PathBuf> = Vec::new();
            if let Ok(res_dir) = app.path().resource_dir() {
                resource_dirs.push(res_dir.join("resources"));
                resource_dirs.push(res_dir);
            }
            if let Ok(exe) = std::env::current_exe() {
                if let Some(parent) = exe.parent() {
                    resource_dirs.push(parent.join("resources"));
                }
            }
            resource_dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources"));

            let export_dir = app
                .path()
                .document_dir()
                .unwrap_or_else(|_| data_dir.clone())
                .join("Vecta_CVs");

            let state = AppState::production(db, resource_dirs, export_dir)
                .expect("failed to initialize Vecta AppState");
            app.manage(state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn export_typescript_bindings_contains_all_v3_commands() {
        let bindings_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../src/lib/bindings.ts");

        specta_builder()
            .export(typescript_exporter(), &bindings_path)
            .expect("tauri-specta export must succeed");

        let content =
            std::fs::read_to_string(&bindings_path).expect("generated bindings.ts must be readable");

        let expected_commands = [
            "get_api_key_status",
            "save_and_test_api_key",
            "delete_api_key",
            "check_worker_health",
            "get_search_preferences",
            "save_search_preferences",
            "get_local_ai_status",
            "get_scraper_mode",
            "set_scraper_mode",
            "parse_user_story",
            "save_confirmed_profile",
            "get_active_profile",
            "toggle_scrape_keyword",
            "add_custom_keyword",
            "import_cv_pdf",
            "parse_story_local",
            "get_master_cv",
            "save_master_cv",
            "export_tailored_ats_pdf",
            "ingest_manual_job",
            "run_scrape_for_active_keywords",
            "scrape_and_store_jobs",
            "run_ai_matching_queue",
            "enrich_job_with_ai",
            "list_ranked_jobs",
            "get_job_detail",
            "toggle_save_job",
            "update_job_application",
            "generate_gap_analysis",
            "generate_apply_kit",
            "list_available_roles",
            "get_competency_map",
            "generate_learning_roadmap",
            "list_roadmaps",
            "get_roadmap_detail",
            "update_roadmap_step_status",
            "quick_add_user_skill",
        ];

        assert_eq!(expected_commands.len(), 37);
        for cmd in expected_commands {
            assert!(
                content.contains(cmd),
                "bindings.ts missing IPC command '{cmd}'"
            );
        }
    }

    #[test]
    fn build_rs_gates_windows_manifest_on_target_env_not_host_cfg() {
        let build_rs = include_str!("../build.rs");
        assert!(
            !build_rs.contains("#[cfg(target_os = \"windows\")]"),
            "build.rs must check CARGO_CFG_TARGET_OS/ENV, not host #[cfg(target_os)]"
        );
        assert!(build_rs.contains("CARGO_CFG_TARGET_OS"));
        assert!(build_rs.contains("CARGO_CFG_TARGET_ENV"));
    }
}
