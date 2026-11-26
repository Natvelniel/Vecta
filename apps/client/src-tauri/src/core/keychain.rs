use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use keyring::Entry;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use crate::core::config::{KEYCHAIN_SERVICE, KEYCHAIN_USER};
use crate::error::AppError;
use crate::models::profile::ApiKeyStatus;

pub trait SecretStore: Send + Sync {
    fn get_api_key(&self) -> Result<Option<String>, AppError>;
    fn set_api_key(&self, key: &str) -> Result<(), AppError>;
    fn delete_api_key(&self) -> Result<(), AppError>;
}

const VAULT_MAGIC: &[u8; 4] = b"VVLT";
const VAULT_VERSION: u8 = 1;
const VAULT_KEY_FILE: &str = ".vecta_vault.key";
const VAULT_DATA_FILE: &str = ".vecta_vault.bin";

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) {}

fn derive_keystream_block(master_key: &[u8; 32], nonce: &[u8; 16], counter: u32) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(master_key);
    hasher.update(b"VECTA_VAULT_ENC_V1");
    hasher.update(nonce);
    hasher.update(counter.to_le_bytes());
    let digest = hasher.finalize();
    let mut block = [0u8; 32];
    block.copy_from_slice(&digest);
    block
}

fn xor_with_keystream(master_key: &[u8; 32], nonce: &[u8; 16], input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    for (block_idx, chunk) in input.chunks(32).enumerate() {
        let block = derive_keystream_block(master_key, nonce, block_idx as u32);
        for (b, k) in chunk.iter().zip(block.iter()) {
            out.push(b ^ k);
        }
    }
    out
}

fn compute_vault_mac(master_key: &[u8; 32], nonce: &[u8; 16], ciphertext: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(master_key);
    hasher.update(b"VECTA_VAULT_MAC_V1");
    hasher.update(nonce);
    hasher.update(ciphertext);
    let digest = hasher.finalize();
    let mut tag = [0u8; 32];
    tag.copy_from_slice(&digest);
    tag
}

/// Encrypted app-private file vault for mobile targets where desktop `keyring` is unavailable.
pub struct FileVaultSecretStore {
    dir: PathBuf,
    cache: Mutex<Option<String>>,
}

impl fmt::Debug for FileVaultSecretStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let has_key = self.cache.lock().map(|g| g.is_some()).unwrap_or(false);
        f.debug_struct("FileVaultSecretStore")
            .field("dir", &self.dir)
            .field("has_key", &has_key)
            .finish()
    }
}

impl FileVaultSecretStore {
    pub fn new(dir: &Path) -> Self {
        let _ = fs::create_dir_all(dir);
        let store = Self {
            dir: dir.to_path_buf(),
            cache: Mutex::new(None),
        };
        if let Ok(Some(existing)) = store.read_from_disk() {
            if let Ok(mut guard) = store.cache.lock() {
                *guard = Some(existing);
            }
        }
        store
    }

    fn key_path(&self) -> PathBuf {
        self.dir.join(VAULT_KEY_FILE)
    }

    fn data_path(&self) -> PathBuf {
        self.dir.join(VAULT_DATA_FILE)
    }

    fn load_or_create_master_key(&self) -> Result<[u8; 32], AppError> {
        let path = self.key_path();
        if path.exists() {
            let bytes = fs::read(&path)
                .map_err(|_| AppError::db_error("Gagal membaca kunci vault aplikasi."))?;
            if bytes.len() != 32 {
                return Err(AppError::db_error("Format kunci vault aplikasi tidak valid."));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }

        fs::create_dir_all(&self.dir)
            .map_err(|_| AppError::db_error("Gagal membuat direktori vault aplikasi."))?;
        let mut key = [0u8; 32];
        key[..16].copy_from_slice(Uuid::new_v4().as_bytes());
        key[16..].copy_from_slice(Uuid::new_v4().as_bytes());
        let tmp = self.dir.join(".vecta_vault.key.tmp");
        fs::write(&tmp, &key)
            .map_err(|_| AppError::db_error("Gagal menulis kunci vault aplikasi."))?;
        set_owner_only_permissions(&tmp);
        fs::rename(&tmp, &path)
            .map_err(|_| AppError::db_error("Gagal menyimpan kunci vault aplikasi."))?;
        set_owner_only_permissions(&path);
        Ok(key)
    }

    fn read_from_disk(&self) -> Result<Option<String>, AppError> {
        let data_path = self.data_path();
        if !data_path.exists() {
            return Ok(None);
        }
        let raw = fs::read(&data_path)
            .map_err(|_| AppError::db_error("Gagal membaca file vault API Key."))?;
        // Header: 4B magic + 1B version + 16B nonce + 32B MAC = 53 bytes minimum
        if raw.len() < 53 || &raw[0..4] != VAULT_MAGIC || raw[4] != VAULT_VERSION {
            return Err(AppError::db_error("Integritas vault API Key tidak valid."));
        }
        let mut nonce = [0u8; 16];
        nonce.copy_from_slice(&raw[5..21]);
        let mac_start = raw.len() - 32;
        let ciphertext = &raw[21..mac_start];
        let stored_mac = &raw[mac_start..];

        let master_key = self.load_or_create_master_key()?;
        let expected_mac = compute_vault_mac(&master_key, &nonce, ciphertext);
        let diff = stored_mac
            .iter()
            .zip(expected_mac.iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b));
        if diff != 0 {
            return Err(AppError::db_error(
                "Autentikasi vault API Key gagal. Silakan masukkan ulang API Key.",
            ));
        }

        let plain = xor_with_keystream(&master_key, &nonce, ciphertext);
        let text = String::from_utf8(plain)
            .map_err(|_| AppError::db_error("Encoding API Key dalam vault rusak."))?;
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed))
        }
    }
}

impl SecretStore for FileVaultSecretStore {
    fn get_api_key(&self) -> Result<Option<String>, AppError> {
        {
            let guard = self
                .cache
                .lock()
                .map_err(|_| AppError::db_error("Vault mutex poisoned"))?;
            if let Some(ref cached) = *guard {
                return Ok(Some(cached.clone()));
            }
        }

        let disk = self.read_from_disk()?;
        if let Some(ref key) = disk {
            let mut guard = self
                .cache
                .lock()
                .map_err(|_| AppError::db_error("Vault mutex poisoned"))?;
            *guard = Some(key.clone());
        }
        Ok(disk)
    }

    fn set_api_key(&self, key: &str) -> Result<(), AppError> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation_error("API Key tidak boleh kosong."));
        }
        let master_key = self.load_or_create_master_key()?;
        let mut nonce = [0u8; 16];
        nonce.copy_from_slice(Uuid::new_v4().as_bytes());
        let ciphertext = xor_with_keystream(&master_key, &nonce, trimmed.as_bytes());
        let mac = compute_vault_mac(&master_key, &nonce, &ciphertext);

        let mut payload = Vec::with_capacity(53 + ciphertext.len());
        payload.extend_from_slice(VAULT_MAGIC);
        payload.push(VAULT_VERSION);
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);
        payload.extend_from_slice(&mac);

        let tmp = self.dir.join(".vecta_vault.bin.tmp");
        fs::write(&tmp, &payload)
            .map_err(|_| AppError::db_error("Gagal menulis API Key ke vault aplikasi."))?;
        set_owner_only_permissions(&tmp);
        let dest = self.data_path();
        fs::rename(&tmp, &dest)
            .map_err(|_| AppError::db_error("Gagal menyimpan API Key ke vault aplikasi."))?;
        set_owner_only_permissions(&dest);

        let mut guard = self
            .cache
            .lock()
            .map_err(|_| AppError::db_error("Vault mutex poisoned"))?;
        *guard = Some(trimmed.to_string());
        Ok(())
    }

    fn delete_api_key(&self) -> Result<(), AppError> {
        match fs::remove_file(self.data_path()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(AppError::db_error("Gagal menghapus file vault API Key.")),
        }
        let mut guard = self
            .cache
            .lock()
            .map_err(|_| AppError::db_error("Vault mutex poisoned"))?;
        *guard = None;
        Ok(())
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[derive(Debug, Default)]
pub struct OsKeyringStore;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl OsKeyringStore {
    pub fn new(_data_dir: &Path) -> Self {
        Self
    }

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
#[derive(Debug)]
pub struct OsKeyringStore {
    vault: FileVaultSecretStore,
}

#[cfg(any(target_os = "android", target_os = "ios"))]
impl OsKeyringStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            vault: FileVaultSecretStore::new(data_dir),
        }
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
impl SecretStore for OsKeyringStore {
    fn get_api_key(&self) -> Result<Option<String>, AppError> {
        self.vault.get_api_key()
    }
    fn set_api_key(&self, key: &str) -> Result<(), AppError> {
        self.vault.set_api_key(key)
    }
    fn delete_api_key(&self) -> Result<(), AppError> {
        self.vault.delete_api_key()
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

    #[test]
    fn file_vault_persists_across_restarts_and_never_writes_plaintext() {
        let tmp = std::env::temp_dir().join(format!("vecta_vault_persist_{}", Uuid::new_v4()));
        let secret = "AIzaSyPERSISTTESTKEY987654321";

        {
            let store = FileVaultSecretStore::new(&tmp);
            assert_eq!(store.get_api_key().expect("initial empty"), None);
            store.set_api_key(secret).expect("save secret");
            assert_eq!(store.get_api_key().expect("cached read").as_deref(), Some(secret));
        }

        let raw = fs::read(tmp.join(VAULT_DATA_FILE)).expect("vault binary exists");
        assert!(!raw.windows(6).any(|w| w == b"AIzaSy"));
        assert!(!raw.windows(14).any(|w| w == b"PERSISTTESTKEY"));

        // Fresh instance simulates complete app restart
        let restarted = FileVaultSecretStore::new(&tmp);
        assert_eq!(
            restarted.get_api_key().expect("persisted read after restart").as_deref(),
            Some(secret)
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn file_vault_detects_tampering_and_supports_delete() {
        let tmp = std::env::temp_dir().join(format!("vecta_vault_tamper_{}", Uuid::new_v4()));
        let store = FileVaultSecretStore::new(&tmp);
        store.set_api_key("AIzaSyTAMPERCHECK12345").expect("save");

        let data_path = tmp.join(VAULT_DATA_FILE);
        let mut raw = fs::read(&data_path).expect("read vault");
        let last = raw.len() - 1;
        raw[last] ^= 0xFF;
        fs::write(&data_path, &raw).expect("corrupt vault");

        let tampered_store = FileVaultSecretStore::new(&tmp);
        assert!(tampered_store.get_api_key().is_err(), "tampered vault must reject read");

        // Re-save clean key, delete, then simulate restart
        tampered_store.set_api_key("AIzaSyCLEANKEY999").expect("re-save");
        tampered_store.delete_api_key().expect("delete");
        assert_eq!(tampered_store.get_api_key().expect("deleted read"), None);

        let after_delete_restart = FileVaultSecretStore::new(&tmp);
        assert_eq!(after_delete_restart.get_api_key().expect("empty after delete"), None);
        let _ = fs::remove_dir_all(&tmp);
    }
}
