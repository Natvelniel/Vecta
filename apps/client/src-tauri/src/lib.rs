pub mod commands;
pub mod core;
pub mod error;
pub mod models;
pub mod state;

use specta_typescript::Typescript;
use tauri::Manager;
use tauri_specta::{collect_commands, Builder, ErrorHandlingMode};

use crate::commands::{competency, jobs, profile, roadmap, settings};
use crate::core::db::Database;
use crate::state::AppState;

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .error_handling(ErrorHandlingMode::Result)
        .dangerously_cast_bigints_to_number()
        .commands(collect_commands![
        // Settings (4)
        settings::get_api_key_status,
        settings::save_and_test_api_key,
        settings::delete_api_key,
        settings::check_worker_health,
        // Profile & Keywords (5)
        profile::parse_user_story,
        profile::save_confirmed_profile,
        profile::get_active_profile,
        profile::toggle_scrape_keyword,
        profile::add_custom_keyword,
        // Jobs & Gap Analysis (6)
        jobs::ingest_manual_job,
        jobs::run_scrape_for_active_keywords,
        jobs::list_ranked_jobs,
        jobs::get_job_detail,
        jobs::toggle_save_job,
        jobs::generate_gap_analysis,
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
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve Tauri app_data_dir");
            let db_path = data_dir.join("vecta.db");
            let db = Database::open(&db_path).expect("failed to initialize Vecta SQLite database");
            let state = AppState::production(db).expect("failed to initialize Vecta AppState");
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
    fn export_typescript_bindings_contains_all_22_commands() {
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
            "parse_user_story",
            "save_confirmed_profile",
            "get_active_profile",
            "toggle_scrape_keyword",
            "add_custom_keyword",
            "ingest_manual_job",
            "run_scrape_for_active_keywords",
            "list_ranked_jobs",
            "get_job_detail",
            "toggle_save_job",
            "generate_gap_analysis",
            "list_available_roles",
            "get_competency_map",
            "generate_learning_roadmap",
            "list_roadmaps",
            "get_roadmap_detail",
            "update_roadmap_step_status",
            "quick_add_user_skill",
        ];

        assert_eq!(expected_commands.len(), 22);
        for cmd in expected_commands {
            assert!(
                content.contains(cmd),
                "bindings.ts missing IPC command '{cmd}'"
            );
        }
    }
}
