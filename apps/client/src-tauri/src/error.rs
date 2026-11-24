use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ApiKeyMissing,
    ApiKeyInvalid,
    GeminiRateLimited,
    GeminiParseFailed,
    ScraperUnreachable,
    DbError,
    ValidationError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Error)]
#[error("[{code:?}] {message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl AppError {
    pub fn api_key_missing() -> Self {
        Self {
            code: ErrorCode::ApiKeyMissing,
            message: "Gemini API Key belum diatur. Silakan tambahkan kunci di Pengaturan.".into(),
            retryable: false,
        }
    }

    pub fn api_key_invalid(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ApiKeyInvalid,
            message: message.into(),
            retryable: false,
        }
    }

    pub fn gemini_rate_limited(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::GeminiRateLimited,
            message: message.into(),
            retryable: true,
        }
    }

    pub fn gemini_parse_failed(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::GeminiParseFailed,
            message: message.into(),
            retryable: true,
        }
    }

    pub fn scraper_unreachable(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ScraperUnreachable,
            message: message.into(),
            retryable: true,
        }
    }

    pub fn db_error(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::DbError,
            message: message.into(),
            retryable: false,
        }
    }

    pub fn validation_error(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ValidationError,
            message: message.into(),
            retryable: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_taxonomy_matches_prd_table() {
        let cases = vec![
            (AppError::api_key_missing(), "API_KEY_MISSING", false),
            (AppError::api_key_invalid("bad"), "API_KEY_INVALID", false),
            (AppError::gemini_rate_limited("429"), "GEMINI_RATE_LIMITED", true),
            (AppError::gemini_parse_failed("json"), "GEMINI_PARSE_FAILED", true),
            (AppError::scraper_unreachable("down"), "SCRAPER_UNREACHABLE", true),
            (AppError::db_error("constraint"), "DB_ERROR", false),
            (AppError::validation_error("short"), "VALIDATION_ERROR", false),
        ];

        for (err, expected_code, expected_retryable) in cases {
            let val = serde_json::to_value(&err).expect("AppError should serialize");
            assert_eq!(val["code"], expected_code);
            assert_eq!(val["retryable"], expected_retryable);
            assert!(val["message"].is_string());
            assert!(!val["message"].as_str().unwrap_or_default().is_empty());
        }
    }
}
