use std::fmt;
use std::sync::Mutex;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use keyring::Entry;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use crate::core::config::{KEYCHAIN_SERVICE, KEYCHAIN_USER};
use crate::error::AppError;
use crate::models::profile::ApiKeyStatus;

pub trait SecretStore: Send + Sync {
    fn get_api_key(&self) -> Result<Option<String>, AppError>;
    fn set_api_key(&self, key: &str) -> Result<(), AppError>;
    fn delete_api_key(&self) -> Result<(), AppError>;
}

#[derive(Debug, Default)]
pub struct OsKeyringStore;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl OsKeyringStore {
    fn entry() -> Result<Entry, AppError> {
        Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER).map_err(|e| {
            eprintln!("[vecta::keychain] failed to open credential entry: {e}");
            AppError::db_error(
                "Gagal mengakses OS Credential Manager. Pastikan layanan kunci sistem tersedia.",
            )
        })
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl SecretStore for OsKeyringStore {
    fn get_api_key(&self) -> Result<Option<String>, AppError> {
        let entry = Self::entry()?;
        match entry.get_password() {
            Ok(secret) => {
                let trimmed = secret.trim().to_string();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(trimmed))
                }
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => {
                eprintln!("[vecta::keychain] failed to read credential: {e}");
                Err(AppError::db_error(
                    "Gagal membaca Gemini API Key dari OS Credential Manager.",
                ))
            }
        }
    }

    fn set_api_key(&self, key: &str) -> Result<(), AppError> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation_error("API Key tidak boleh kosong."));
        }
        let entry = Self::entry()?;
        entry.set_password(trimmed).map_err(|e| {
            eprintln!("[vecta::keychain] failed to store credential: {e}");
            AppError::db_error("Gagal menyimpan Gemini API Key ke OS Credential Manager.")
        })
    }

    fn delete_api_key(&self) -> Result<(), AppError> {
        let entry = Self::entry()?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => {
                eprintln!("[vecta::keychain] failed to delete credential: {e}");
                Err(AppError::db_error(
                    "Gagal menghapus Gemini API Key dari OS Credential Manager.",
                ))
            }
        }
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn mobile_session_key() -> &'static Mutex<Option<String>> {
    static MOBILE_KEY: std::sync::OnceLock<Mutex<Option<String>>> = std::sync::OnceLock::new();
    MOBILE_KEY.get_or_init(|| Mutex::new(None))
}

#[cfg(any(target_os = "android", target_os = "ios"))]
impl SecretStore for OsKeyringStore {
    fn get_api_key(&self) -> Result<Option<String>, AppError> {
        let guard = mobile_session_key()
            .lock()
            .map_err(|_| AppError::db_error("SecretStore mutex poisoned"))?;
        Ok(guard.clone())
    }
    fn set_api_key(&self, key: &str) -> Result<(), AppError> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation_error("API Key tidak boleh kosong."));
        }
        let mut guard = mobile_session_key()
            .lock()
            .map_err(|_| AppError::db_error("SecretStore mutex poisoned"))?;
        *guard = Some(trimmed.to_string());
        Ok(())
    }
    fn delete_api_key(&self) -> Result<(), AppError> {
        let mut guard = mobile_session_key()
            .lock()
            .map_err(|_| AppError::db_error("SecretStore mutex poisoned"))?;
        *guard = None;
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemorySecretStore {
    inner: Mutex<Option<String>>,
}

impl InMemorySecretStore {
    pub fn new(initial: Option<String>) -> Self {
        Self {
            inner: Mutex::new(initial),
        }
    }
}

impl fmt::Debug for InMemorySecretStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let has_key = self.inner.lock().map(|g| g.is_some()).unwrap_or(false);
        f.debug_struct("InMemorySecretStore")
            .field("has_key", &has_key)
            .finish()
    }
}

impl SecretStore for InMemorySecretStore {
    fn get_api_key(&self) -> Result<Option<String>, AppError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| AppError::db_error("SecretStore mutex poisoned"))?;
        Ok(guard.clone())
    }

    fn set_api_key(&self, key: &str) -> Result<(), AppError> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation_error("API Key tidak boleh kosong."));
        }
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| AppError::db_error("SecretStore mutex poisoned"))?;
        *guard = Some(trimmed.to_string());
        Ok(())
    }

    fn delete_api_key(&self) -> Result<(), AppError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| AppError::db_error("SecretStore mutex poisoned"))?;
        *guard = None;
        Ok(())
    }
}

pub fn mask_api_key(key: &str) -> String {
    let chars: Vec<char> = key.trim().chars().collect();
    let len = chars.len();
    if len >= 8 {
        let prefix: String = chars[..4].iter().collect();
        let suffix: String = chars[len - 4..].iter().collect();
        format!("{prefix}...{suffix}")
    } else if len > 4 {
        let suffix: String = chars[len - 4..].iter().collect();
        format!("...{suffix}")
    } else {
        "****".to_string()
    }
}

pub fn build_api_key_status(key: Option<&str>) -> ApiKeyStatus {
    match key.map(str::trim).filter(|k| !k.is_empty()) {
        Some(k) => ApiKeyStatus {
            has_key: true,
            masked_suffix: Some(mask_api_key(k)),
        },
        None => ApiKeyStatus {
            has_key: false,
            masked_suffix: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_store_lifecycle() {
        let store = InMemorySecretStore::new(None);
        assert_eq!(store.get_api_key().expect("empty get"), None);
        assert_eq!(
            build_api_key_status(None),
            ApiKeyStatus {
                has_key: false,
                masked_suffix: None,
            }
        );

        store
            .set_api_key("AIzaSyDUMMYSECRETKEY12349xQp")
            .expect("set key");
        assert_eq!(
            store.get_api_key().expect("get key").as_deref(),
            Some("AIzaSyDUMMYSECRETKEY12349xQp")
        );

        store.delete_api_key().expect("first delete");
        assert_eq!(store.get_api_key().expect("after delete"), None);
        store
            .delete_api_key()
            .expect("idempotent delete on missing key");
    }

    #[test]
    fn masked_key_never_exposes_middle() {
        let secret = "AIzaSyDUMMYSECRETKEY12349xQp";
        let masked = mask_api_key(secret);
        assert_eq!(masked, "AIza...9xQp");
        assert!(!masked.contains("DUMMYSECRETKEY"));
        assert_eq!(mask_api_key("1234"), "****");
        assert_eq!(mask_api_key("abcdef"), "...cdef");

        let status = build_api_key_status(Some(secret));
        assert!(status.has_key);
        assert_eq!(status.masked_suffix.as_deref(), Some("AIza...9xQp"));

        let store = InMemorySecretStore::new(Some(secret.to_string()));
        let debug_repr = format!("{store:?}");
        assert!(!debug_repr.contains(secret));
        assert!(!debug_repr.contains("DUMMYSECRETKEY"));
    }
}
