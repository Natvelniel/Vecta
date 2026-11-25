pub const GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
pub const GEMINI_GENERATION_MODEL: &str = "gemini-3.5-flash-lite";
pub const GEMINI_EMBEDDING_MODEL: &str = "gemini-embedding-001";
pub const EMBEDDING_DIMENSIONS: usize = 768;

pub const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:8000";

pub const GEMINI_MAX_RETRIES: usize = 3;
pub const GEMINI_BACKOFF_BASE_MS: u64 = 1_000;
pub const SCRAPE_GEMINI_PACING_MS: u64 = 350;

pub const TEMP_CONTRACT_A: f32 = 0.1;
pub const TEMP_CONTRACT_B: f32 = 0.3;
pub const TEMP_CONTRACT_C: f32 = 0.3;
pub const TEMP_CONTRACT_D: f32 = 0.1;

pub const KEYCHAIN_SERVICE: &str = "vecta";
pub const KEYCHAIN_USER: &str = "gemini_api_key";

pub const MIN_STORY_CHARS: usize = 30;
pub const MIN_MANUAL_JOB_CHARS: usize = 30;
pub const MAX_MANUAL_JOB_CHARS: usize = 12_000;
pub const MAX_KNOWN_SKILLS_CONTEXT: usize = 50;
