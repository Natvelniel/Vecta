use tauri::State;

use crate::core::keychain::build_api_key_status;
use crate::core::local_embedder::LocalAiModelStatus;
use crate::core::worker::validate_worker_url;
use crate::error::AppError;
use crate::models::job::SearchPreferences;
use crate::models::profile::{ApiKeyStatus, VectaBackupReport, VectaImportReport, WorkerHealth};
use crate::state::AppState;

const SETTING_SEARCH_PREFS: &str = "search_preferences";

fn normalize_search_preferences(mut prefs: SearchPreferences) -> SearchPreferences {
    let mut clean_locs: Vec<String> = prefs
        .locations
        .into_iter()
        .map(|l| l.trim().chars().take(60).collect::<String>())
        .filter(|l| !l.is_empty())
        .collect();
    if clean_locs.is_empty() {
        let loc = prefs.location.trim();
        clean_locs.push(if loc.is_empty() {
            "Indonesia".into()
        } else {
            loc.chars().take(60).collect()
        });
    }
    let mut seen_loc = std::collections::HashSet::new();
    clean_locs.retain(|l| seen_loc.insert(l.to_lowercase()));
    clean_locs.truncate(5);
    prefs.location = clean_locs[0].clone();
    prefs.locations = clean_locs;

    let valid_sources: Vec<String> = prefs
        .sources
        .into_iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| {
            matches!(
                s.as_str(),
                "linkedin" | "jobstreet" | "indeed" | "kalibrr" | "dealls" | "glints"
            )
        })
        .collect();
    prefs.sources = if valid_sources.is_empty() {
        vec!["jobstreet".into(), "linkedin".into(), "kalibrr".into()]
    } else {
        valid_sources
    };

    prefs.limit_per_query = prefs.limit_per_query.clamp(5, 50);
    prefs.max_pages = prefs.max_pages.clamp(1, 5);
    prefs.posted_within_days = prefs.posted_within_days.map(|d| d.clamp(1, 90));
    prefs.work_mode = match prefs.work_mode.trim().to_lowercase().as_str() {
        "remote" => "remote".into(),
        "hybrid" => "hybrid".into(),
        "onsite" => "onsite".into(),
        _ => "any".into(),
    };
    prefs.custom_queries = prefs
        .custom_queries
        .into_iter()
        .map(|q| q.trim().chars().take(120).collect::<String>())
        .filter(|q| !q.is_empty())
        .take(15)
        .collect();
    prefs.ai_enrich_max_jobs = prefs.ai_enrich_max_jobs.clamp(1, 100);
    prefs.ai_rpm_limit = prefs.ai_rpm_limit.clamp(1, 60);
    prefs
}

pub fn get_search_preferences_impl(state: &AppState) -> Result<SearchPreferences, AppError> {
    let raw = state.db.get_app_setting(SETTING_SEARCH_PREFS)?;
    let prefs = raw
        .as_deref()
        .and_then(|s| serde_json::from_str::<SearchPreferences>(s).ok())
        .unwrap_or_default();
    Ok(normalize_search_preferences(prefs))
}

pub fn save_search_preferences_impl(
    state: &AppState,
    prefs: SearchPreferences,
) -> Result<SearchPreferences, AppError> {
    let clean = normalize_search_preferences(prefs);
    let json = serde_json::to_string(&clean)
        .map_err(|_| AppError::validation_error("Preferensi pencarian tidak dapat disimpan."))?;
    state.db.set_app_setting(SETTING_SEARCH_PREFS, &json)?;
    Ok(clean)
}

pub fn get_api_key_status_impl(state: &AppState) -> Result<ApiKeyStatus, AppError> {
    let key = state.secrets.get_api_key()?;
    Ok(build_api_key_status(key.as_deref()))
}

pub async fn save_and_test_api_key_impl(
    state: &AppState,
    api_key: String,
) -> Result<ApiKeyStatus, AppError> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation_error(
            "Gemini API Key tidak boleh kosong.",
        ));
    }

    state.gemini.test_api_key(trimmed).await?;
    state.secrets.set_api_key(trimmed)?;
    Ok(build_api_key_status(Some(trimmed)))
}

pub fn delete_api_key_impl(state: &AppState) -> Result<(), AppError> {
    state.secrets.delete_api_key()
}

pub async fn check_worker_health_impl(
    state: &AppState,
    worker_url: Option<String>,
) -> Result<WorkerHealth, AppError> {
    let custom = worker_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('/').to_string());

    let target_url = match &custom {
        Some(url) => validate_worker_url(url)?,
        None => state.get_worker_url(),
    };

    let health = state.active_scraper().check_health(&target_url).await?;
    if custom.is_some() {
        state.set_worker_url(target_url)?;
    }
    Ok(health)
}

pub fn get_local_ai_status_impl(state: &AppState) -> LocalAiModelStatus {
    state.local_embedder.status()
}

pub fn get_scraper_mode_impl(state: &AppState) -> String {
    state.get_scraper_mode()
}

pub fn set_scraper_mode_impl(state: &AppState, mode: String) -> Result<String, AppError> {
    state.set_scraper_mode(&mode)
}

pub fn export_vecta_backup_impl(state: &AppState) -> Result<VectaBackupReport, AppError> {
    state.db.export_backup_json()
}

pub fn import_vecta_backup_impl(
    state: &AppState,
    backup_json: String,
) -> Result<VectaImportReport, AppError> {
    let report = state.db.import_backup_json(&backup_json)?;
    if let Ok(Some(prof)) = state.db.get_active_profile() {
        let _ = state.recalculate_all_matches_for_user(&prof.id);
    }
    Ok(report)
}

#[tauri::command]
#[specta::specta]
pub fn export_vecta_backup(state: State<'_, AppState>) -> Result<VectaBackupReport, AppError> {
    export_vecta_backup_impl(&state)
}

#[tauri::command]
#[specta::specta]
pub fn import_vecta_backup(
    state: State<'_, AppState>,
    backup_json: String,
) -> Result<VectaImportReport, AppError> {
    import_vecta_backup_impl(&state, backup_json)
}

#[tauri::command]
#[specta::specta]
pub fn get_local_ai_status(state: State<'_, AppState>) -> Result<LocalAiModelStatus, AppError> {
    Ok(get_local_ai_status_impl(&state))
}

#[tauri::command]
#[specta::specta]
pub fn get_scraper_mode(state: State<'_, AppState>) -> Result<String, AppError> {
    Ok(get_scraper_mode_impl(&state))
}

#[tauri::command]
#[specta::specta]
pub fn set_scraper_mode(state: State<'_, AppState>, mode: String) -> Result<String, AppError> {
    set_scraper_mode_impl(&state, mode)
}

#[tauri::command]
#[specta::specta]
pub fn get_api_key_status(state: State<'_, AppState>) -> Result<ApiKeyStatus, AppError> {
    get_api_key_status_impl(&state)
}

#[tauri::command]
#[specta::specta]
pub async fn save_and_test_api_key(
    state: State<'_, AppState>,
    api_key: String,
) -> Result<ApiKeyStatus, AppError> {
    save_and_test_api_key_impl(&state, api_key).await
}

#[tauri::command]
#[specta::specta]
pub fn delete_api_key(state: State<'_, AppState>) -> Result<(), AppError> {
    delete_api_key_impl(&state)
}

#[tauri::command]
#[specta::specta]
pub async fn check_worker_health(
    state: State<'_, AppState>,
    worker_url: Option<String>,
) -> Result<WorkerHealth, AppError> {
    check_worker_health_impl(&state, worker_url).await
}

#[tauri::command]
#[specta::specta]
pub fn get_search_preferences(state: State<'_, AppState>) -> Result<SearchPreferences, AppError> {
    get_search_preferences_impl(&state)
}

#[tauri::command]
#[specta::specta]
pub fn save_search_preferences(
    state: State<'_, AppState>,
    prefs: SearchPreferences,
) -> Result<SearchPreferences, AppError> {
    save_search_preferences_impl(&state, prefs)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::error::ErrorCode;
    use crate::state::test_support::{build_test_state, MockGemini, MockWorker};

    #[tokio::test]
    async fn save_and_test_api_key_rejects_invalid_without_persisting() {
        let mut mock = MockGemini::default();
        mock.fail_key_validation = true;
        let state = build_test_state(None, Arc::new(mock), Arc::new(MockWorker::default()));

        let err = save_and_test_api_key_impl(&state, "AIzaSyBADKEY1234".into())
            .await
            .expect_err("invalid key must fail");
        assert_eq!(err.code, ErrorCode::ApiKeyInvalid);

        let status = get_api_key_status_impl(&state).expect("status");
        assert!(!status.has_key);
        assert_eq!(status.masked_suffix, None);
    }

    #[tokio::test]
    async fn save_and_test_api_key_persists_valid_and_returns_masked() {
        let state = build_test_state(
            None,
            Arc::new(MockGemini::default()),
            Arc::new(MockWorker::default()),
        );

        let status = save_and_test_api_key_impl(&state, "AIzaSyVALIDSECRETKEY9xQp".into())
            .await
            .expect("valid key succeeds");
        assert!(status.has_key);
        assert_eq!(status.masked_suffix.as_deref(), Some("AIza...9xQp"));

        delete_api_key_impl(&state).expect("delete");
        assert!(!get_api_key_status_impl(&state).expect("after delete").has_key);

        let health = check_worker_health_impl(&state, None)
            .await
            .expect("worker health ok");
        assert_eq!(health.status, "ok");
    }

    #[tokio::test]
    async fn check_worker_health_updates_active_worker_url_and_rejects_invalid_scheme() {
        let worker = Arc::new(MockWorker::default());
        let state = build_test_state(
            Some("AIzaSyTEST1234"),
            Arc::new(MockGemini::default()),
            Arc::clone(&worker),
        );

        let bad_err = check_worker_health_impl(&state, Some("ftp://127.0.0.1:8001".into()))
            .await
            .expect_err("non-http scheme must be rejected");
        assert_eq!(bad_err.code, ErrorCode::ValidationError);

        let ssrf_err = check_worker_health_impl(
            &state,
            Some("http://169.254.169.254/latest/meta-data".into()),
        )
        .await
        .expect_err("non-loopback host must be rejected");
        assert_eq!(ssrf_err.code, ErrorCode::ValidationError);

        check_worker_health_impl(&state, Some("http://127.0.0.1:8001/".into()))
            .await
            .expect("custom http url health ok");
        assert_eq!(
            worker.last_health_url.lock().unwrap().as_deref(),
            Some("http://127.0.0.1:8001")
        );
        assert_eq!(
            state
                .db
                .get_app_setting("worker_url")
                .expect("read persisted worker_url")
                .as_deref(),
            Some("http://127.0.0.1:8001")
        );

        check_worker_health_impl(&state, None)
            .await
            .expect("subsequent default check should use updated url");
        assert_eq!(
            worker.last_health_url.lock().unwrap().as_deref(),
            Some("http://127.0.0.1:8001")
        );
    }
}
