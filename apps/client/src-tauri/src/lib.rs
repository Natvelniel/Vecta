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
        // Settings & Offline AI (11)
        settings::get_api_key_status,
        settings::save_and_test_api_key,
        settings::delete_api_key,
        settings::check_worker_health,
        settings::get_search_preferences,
        settings::save_search_preferences,
        settings::get_local_ai_status,
        settings::get_scraper_mode,
        settings::set_scraper_mode,
        settings::export_vecta_backup,
        settings::import_vecta_backup,
        // Profile & Keywords (9)
        profile::parse_user_story,
        profile::save_confirmed_profile,
        profile::get_active_profile,
        profile::toggle_scrape_keyword,
        profile::add_custom_keyword,
        profile::list_prodi_roles_catalog,
        profile::list_user_profiles,
        profile::switch_active_profile,
        profile::delete_user_profile,
        // CV Import, Master CV & ATS PDF Export (5)
        cv::import_cv_pdf,
        cv::parse_story_local,
        cv::get_master_cv,
        cv::save_master_cv,
        cv::export_tailored_ats_pdf,
        // Jobs, Decoupled AI Queue, Tracker, Watchlists, Apply Copilot & Interview Prep (15)
        jobs::ingest_manual_job,
        jobs::run_scrape_for_active_keywords,
        jobs::scrape_and_store_jobs,
        jobs::rerank_top_jobs,
        jobs::run_ai_matching_queue,
        jobs::enrich_job_with_ai,
        jobs::list_ranked_jobs,
        jobs::get_job_detail,
        jobs::toggle_save_job,
        jobs::update_job_application,
        jobs::generate_gap_analysis,
        jobs::generate_apply_kit,
        jobs::generate_interview_prep,
        jobs::list_saved_watchlists,
        jobs::save_watchlist,
        jobs::delete_watchlist,
        // Competency Map & Market Analytics (5)
        competency::list_available_roles,
        competency::get_competency_map,
        competency::simulate_skill_impact,
        competency::get_salary_benchmark,
        competency::get_company_snapshot,
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

#[cfg(target_os = "android")]
fn extract_android_neural_assets(app_data_dir: &std::path::Path) -> Option<PathBuf> {
    use std::fs::{self, File};
    use std::io::{self, Read};

    let dest_dir = app_data_dir.join("models").join("neural-v2");
    if let Err(e) = fs::create_dir_all(&dest_dir) {
        eprintln!("[vecta::android_assets] mkdir failed: {e}");
        return None;
    }

    let maps = fs::read_to_string("/proc/self/maps").ok()?;
    let apk_path = maps
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let pos = trimmed.find('/')?;
            let p = trimmed[pos..].trim();
            if p.ends_with(".apk") && std::path::Path::new(p).exists() {
                Some(p.to_string())
            } else {
                None
            }
        })
        .next()?;

    let apk_file = File::open(&apk_path).ok()?;
    let mut archive = zip::ZipArchive::new(apk_file).ok()?;
    let mut extracted_any = false;

    for idx in 0..archive.len() {
        let mut entry = archive.by_index(idx).ok()?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let rel = name
            .strip_prefix("assets/resources/models/neural-v2/")
            .or_else(|| name.strip_prefix("assets/models/neural-v2/"));
        let Some(fname) = rel else {
            continue;
        };
        if fname.contains('/') || fname.contains('\\') || fname.contains("..") {
            continue;
        }
        let out_path = dest_dir.join(fname);
        let expected_size = entry.size();
        if let Ok(meta) = fs::metadata(&out_path) {
            if meta.len() == expected_size && expected_size > 0 {
                extracted_any = true;
                continue;
            }
        }
        let tmp_path = dest_dir.join(format!("{fname}.tmp"));
        if let Ok(mut out_f) = File::create(&tmp_path) {
            if io::copy(&mut entry, &mut out_f).is_ok() {
                let _ = fs::rename(&tmp_path, &out_path);
                extracted_any = true;
            } else {
                let _ = fs::remove_file(&tmp_path);
            }
        }
    }

    if extracted_any || dest_dir.join("e5-base.vtxr").exists() {
        Some(dest_dir)
    } else {
        None
    }
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
            #[cfg(target_os = "android")]
            {
                if extract_android_neural_assets(&data_dir).is_some() {
                    resource_dirs.push(data_dir.clone());
                }
            }
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

            #[cfg(target_os = "android")]
            let export_dir = data_dir.join("Vecta_CVs");

            #[cfg(not(target_os = "android"))]
            let export_dir = app
                .path()
                .document_dir()
                .unwrap_or_else(|_| data_dir.clone())
                .join("Vecta_CVs");

            let state = AppState::production(db, &data_dir, resource_dirs, export_dir)
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
            "export_vecta_backup",
            "import_vecta_backup",
            "parse_user_story",
            "save_confirmed_profile",
            "get_active_profile",
            "toggle_scrape_keyword",
            "add_custom_keyword",
            "list_prodi_roles_catalog",
            "list_user_profiles",
            "switch_active_profile",
            "delete_user_profile",
            "import_cv_pdf",
            "parse_story_local",
            "get_master_cv",
            "save_master_cv",
            "export_tailored_ats_pdf",
            "ingest_manual_job",
            "run_scrape_for_active_keywords",
            "scrape_and_store_jobs",
            "rerank_top_jobs",
            "run_ai_matching_queue",
            "enrich_job_with_ai",
            "list_ranked_jobs",
            "get_job_detail",
            "toggle_save_job",
            "update_job_application",
            "generate_gap_analysis",
            "generate_apply_kit",
            "generate_interview_prep",
            "list_saved_watchlists",
            "save_watchlist",
            "delete_watchlist",
            "list_available_roles",
            "get_competency_map",
            "simulate_skill_impact",
            "get_salary_benchmark",
            "get_company_snapshot",
            "generate_learning_roadmap",
            "list_roadmaps",
            "get_roadmap_detail",
            "update_roadmap_step_status",
            "quick_add_user_skill",
        ];

        assert_eq!(expected_commands.len(), 51);
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
