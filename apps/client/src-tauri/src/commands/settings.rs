use tauri::State;

use crate::core::keychain::build_api_key_status;
use crate::error::AppError;
use crate::models::profile::{ApiKeyStatus, WorkerHealth};
use crate::state::AppState;

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
    let url = worker_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&state.worker_url);
    state.worker.check_health(url).await
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
}
