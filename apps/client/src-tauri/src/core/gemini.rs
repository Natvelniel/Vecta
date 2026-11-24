use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::config::{
    EMBEDDING_DIMENSIONS, GEMINI_BACKOFF_BASE_MS, GEMINI_BASE_URL, GEMINI_EMBEDDING_MODEL,
    GEMINI_GENERATION_MODEL, GEMINI_MAX_RETRIES, MAX_KNOWN_SKILLS_CONTEXT, TEMP_CONTRACT_A,
    TEMP_CONTRACT_B, TEMP_CONTRACT_C, TEMP_CONTRACT_D,
};
use crate::error::AppError;
use crate::models::job::{GapAnalysis, JobExtractionResult};
use crate::models::profile::StoryProfileDraft;
use crate::models::roadmap::GeneratedRoadmap;

const SCHEMA_A_RAW: &str =
    include_str!("../../../../../shared/schemas/job_extraction.schema.json");
const SCHEMA_B_RAW: &str =
    include_str!("../../../../../shared/schemas/gap_analysis.schema.json");
const SCHEMA_C_RAW: &str = include_str!("../../../../../shared/schemas/roadmap.schema.json");
const SCHEMA_D_RAW: &str =
    include_str!("../../../../../shared/schemas/story_profile.schema.json");

fn schema_a() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(SCHEMA_A_RAW).expect("valid job_extraction schema"))
}

fn schema_b() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(SCHEMA_B_RAW).expect("valid gap_analysis schema"))
}

fn schema_c() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(SCHEMA_C_RAW).expect("valid roadmap schema"))
}

fn schema_d() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(SCHEMA_D_RAW).expect("valid story_profile schema"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingTaskType {
    RetrievalDocument,
    RetrievalQuery,
}

impl EmbeddingTaskType {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::RetrievalDocument => "RETRIEVAL_DOCUMENT",
            Self::RetrievalQuery => "RETRIEVAL_QUERY",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GapAnalysisPromptInput {
    pub user_summary: String,
    pub user_level: String,
    pub user_skills: Vec<String>,
    pub job_title: String,
    pub job_company: String,
    pub job_role: String,
    pub job_description: String,
    pub matched_skills: Vec<String>,
    pub missing_skills: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoadmapPromptInput {
    pub target_role: String,
    pub user_level: String,
    pub user_summary: String,
    pub missing_skills: Vec<String>,
    pub hours_per_week: u32,
    pub source_job_title: Option<String>,
}

#[async_trait]
pub trait GeminiProvider: Send + Sync {
    async fn test_api_key(&self, api_key: &str) -> Result<(), AppError>;
    async fn extract_job(
        &self,
        api_key: &str,
        raw_description: &str,
        known_skills: &[String],
    ) -> Result<JobExtractionResult, AppError>;
    async fn analyze_gap(
        &self,
        api_key: &str,
        context: &GapAnalysisPromptInput,
    ) -> Result<GapAnalysis, AppError>;
    async fn generate_roadmap(
        &self,
        api_key: &str,
        context: &RoadmapPromptInput,
    ) -> Result<GeneratedRoadmap, AppError>;
    async fn parse_user_story(
        &self,
        api_key: &str,
        raw_story: &str,
        known_skills: &[String],
    ) -> Result<StoryProfileDraft, AppError>;
    async fn embed_text(
        &self,
        api_key: &str,
        text: &str,
        task_type: EmbeddingTaskType,
    ) -> Result<Vec<f32>, AppError>;
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    #[serde(default)]
    parts: Vec<ContentPart>,
}

#[derive(Deserialize)]
struct ContentPart {
    text: Option<String>,
    #[serde(default)]
    thought: bool,
}

#[derive(Deserialize)]
struct EmbedContentResponse {
    embedding: Option<ContentEmbedding>,
}

#[derive(Deserialize)]
struct ContentEmbedding {
    values: Vec<f32>,
}

pub struct HttpGeminiClient {
    http: Client,
    base_url: String,
    backoff_base_ms: u64,
}

impl HttpGeminiClient {
    pub fn new() -> Result<Self, AppError> {
        Self::with_base_url(GEMINI_BASE_URL.to_string(), GEMINI_BACKOFF_BASE_MS)
    }

    pub fn with_base_url(base_url: String, backoff_base_ms: u64) -> Result<Self, AppError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|e| AppError::db_error(format!("Gagal menginisialisasi HTTP client: {e}")))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            backoff_base_ms,
        })
    }

    async fn send_with_retry(
        &self,
        method: Method,
        path: &str,
        api_key: &str,
        body: Option<&Value>,
    ) -> Result<String, AppError> {
        let api_key_header = reqwest::header::HeaderValue::from_str(api_key).map_err(|_| {
            AppError::api_key_invalid(
                "Format Gemini API Key tidak valid (mengandung karakter tidak sah).",
            )
        })?;
        let url = format!("{}{}", self.base_url, path);

        for attempt in 0..=GEMINI_MAX_RETRIES {
            let mut req = self
                .http
                .request(method.clone(), &url)
                .header("x-goog-api-key", api_key_header.clone())
                .header("content-type", "application/json");
            if let Some(payload) = body {
                req = req.json(payload);
            }

            let response = match req.send().await {
                Ok(resp) => resp,
                Err(_) if attempt < GEMINI_MAX_RETRIES => {
                    self.sleep_backoff(attempt).await;
                    continue;
                }
                Err(_) => {
                    return Err(AppError::gemini_rate_limited(
                        "Gagal menghubungi Google Gemini API. Periksa koneksi internet Anda lalu coba lagi.",
                    ));
                }
            };

            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            if status.is_success() {
                return Ok(text);
            }

            if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
                return Err(AppError::api_key_invalid(
                    "Gemini API Key tidak valid atau ditolak (HTTP 401/403). Periksa kunci Anda di Google AI Studio.",
                ));
            }

            if status == StatusCode::BAD_REQUEST {
                let lower = text.to_lowercase();
                let is_key_error = method == Method::GET
                    || lower.contains("api_key_invalid")
                    || lower.contains("api key not valid")
                    || lower.contains("invalid api key");
                if is_key_error {
                    return Err(AppError::api_key_invalid(
                        "Gemini API Key tidak valid atau ditolak (HTTP 400). Periksa kunci Anda di Google AI Studio.",
                    ));
                }
                return Err(AppError::validation_error(
                    "Permintaan ke Gemini API ditolak (HTTP 400). Periksa panjang atau format teks input.",
                ));
            }

            if matches!(
                status,
                StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE
            ) {
                if attempt < GEMINI_MAX_RETRIES {
                    self.sleep_backoff(attempt).await;
                    continue;
                }
                return Err(AppError::gemini_rate_limited(format!(
                    "Kuota atau kapasitas Gemini API penuh (HTTP {}). Tunggu beberapa saat lalu coba lagi.",
                    status.as_u16()
                )));
            }

            return Err(AppError::gemini_parse_failed(format!(
                "Gemini API mengembalikan status tak terduga (HTTP {}).",
                status.as_u16()
            )));
        }

        Err(AppError::gemini_rate_limited(
            "Permintaan ke Gemini API melebihi batas percobaan ulang.",
        ))
    }

    async fn sleep_backoff(&self, attempt: usize) {
        let exp = self.backoff_base_ms.saturating_mul(1u64 << attempt.min(6));
        let jitter_bound = (self.backoff_base_ms / 4).max(1);
        let jitter = (Utc::now().timestamp_subsec_millis() as u64) % jitter_bound;
        tokio::time::sleep(Duration::from_millis(exp.saturating_add(jitter))).await;
    }

    async fn generate_structured<T: DeserializeOwned>(
        &self,
        api_key: &str,
        prompt: &str,
        schema: &Value,
        temperature: f32,
    ) -> Result<T, AppError> {
        let path = format!("/models/{GEMINI_GENERATION_MODEL}:generateContent");
        let payload = json!({
            "contents": [
                {
                    "role": "user",
                    "parts": [{ "text": prompt }]
                }
            ],
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseSchema": schema,
                "temperature": temperature
            }
        });

        let raw_response = self
            .send_with_retry(Method::POST, &path, api_key, Some(&payload))
            .await?;

        let envelope: GenerateContentResponse =
            serde_json::from_str(&raw_response).map_err(|e| {
                AppError::gemini_parse_failed(format!(
                    "Gagal memparsing envelope respons Gemini: {e}"
                ))
            })?;

        let json_text = envelope
            .candidates
            .into_iter()
            .filter_map(|c| c.content)
            .flat_map(|c| c.parts)
            .filter(|p| !p.thought)
            .find_map(|p| p.text)
            .ok_or_else(|| {
                AppError::gemini_parse_failed("Respons Gemini tidak berisi kandidat teks JSON.")
            })?;

        parse_structured_json(&json_text)
    }
}

#[async_trait]
impl GeminiProvider for HttpGeminiClient {
    async fn test_api_key(&self, api_key: &str) -> Result<(), AppError> {
        let path = format!("/models/{GEMINI_GENERATION_MODEL}");
        self.send_with_retry(Method::GET, &path, api_key, None)
            .await?;
        Ok(())
    }

    async fn extract_job(
        &self,
        api_key: &str,
        raw_description: &str,
        known_skills: &[String],
    ) -> Result<JobExtractionResult, AppError> {
        let known_slice = &known_skills[..known_skills.len().min(MAX_KNOWN_SKILLS_CONTEXT)];
        let known_list = if known_slice.is_empty() {
            "(belum ada kamus skill)".to_string()
        } else {
            known_slice.join(", ")
        };

        let prompt = format!(
            "Kamu adalah mesin ekstraksi kompetensi lowongan kerja Vecta.\n\
             Tugas: Ekstrak peran baku (bahasa Inggris, misal 'Backend Engineer'), level pengalaman, mode kerja, ringkasan peran singkat, dan seluruh skill yang diminta (mandatory vs nice_to_have).\n\
             WAJIB gunakan nama baku dari daftar known_canonical_skills jika semantiknya sama untuk mencegah fragmentasi sinonim:\n\
             known_canonical_skills: [{known_list}]\n\n\
             Deskripsi lowongan:\n{raw_description}"
        );

        self.generate_structured(api_key, &prompt, schema_a(), TEMP_CONTRACT_A)
            .await
    }

    async fn analyze_gap(
        &self,
        api_key: &str,
        ctx: &GapAnalysisPromptInput,
    ) -> Result<GapAnalysis, AppError> {
        let prompt = format!(
            "Kamu adalah konsultan karier teknis Vecta. Berikan analisis kesenjangan skill (gap analysis) dan tips penyesuaian CV dalam Bahasa Indonesia yang objektif, ringkas, dan langsung pada poin tindakan.\n\n\
             Profil Kandidat:\n\
             - Level: {}\n\
             - Ringkasan: {}\n\
             - Skill Dimiliki: {}\n\n\
             Lowongan Target:\n\
             - Judul: {} di {} (Peran: {})\n\
             - Skill Cocok: {}\n\
             - Skill Belum Dimiliki: {}\n\
             - Deskripsi: {}",
            ctx.user_level,
            ctx.user_summary,
            ctx.user_skills.join(", "),
            ctx.job_title,
            ctx.job_company,
            ctx.job_role,
            ctx.matched_skills.join(", "),
            ctx.missing_skills.join(", "),
            ctx.job_description
        );

        self.generate_structured(api_key, &prompt, schema_b(), TEMP_CONTRACT_B)
            .await
    }

    async fn generate_roadmap(
        &self,
        api_key: &str,
        ctx: &RoadmapPromptInput,
    ) -> Result<GeneratedRoadmap, AppError> {
        let source_ctx = ctx
            .source_job_title
            .as_deref()
            .map(|t| format!("Khusus untuk lowongan target: {t}\n"))
            .unwrap_or_default();

        let prompt = format!(
            "Kamu adalah arsitek kurikulum karier Vecta. Buat roadmap belajar bertahap dalam Bahasa Indonesia untuk peran target '{}'.\n\
             {source_ctx}\
             Aturan WAJIB:\n\
             1. Urutkan langkah dalam 3 fase: phase_order 1 ('Fundamental'), 2 ('Praktik'), 3 ('Proyek Mini').\n\
             2. DILARANG menyertakan tautan URL mentah (rawan mati). Isi 'learning_keywords' dengan kata kunci pencarian yang abadi.\n\
             3. Sesuaikan 'total_estimated_weeks' dengan alokasi {} jam per minggu.\n\
             4. Isi 'target_skill' dengan nama skill baku yang tepat dari daftar gap berikut.\n\n\
             Profil Pengguna (Level {}): {}\n\
             Prioritas Skill Gap: {}",
            ctx.target_role,
            ctx.hours_per_week,
            ctx.user_level,
            ctx.user_summary,
            ctx.missing_skills.join(", ")
        );

        self.generate_structured(api_key, &prompt, schema_c(), TEMP_CONTRACT_C)
            .await
    }

    async fn parse_user_story(
        &self,
        api_key: &str,
        raw_story: &str,
        known_skills: &[String],
    ) -> Result<StoryProfileDraft, AppError> {
        let known_slice = &known_skills[..known_skills.len().min(MAX_KNOWN_SKILLS_CONTEXT)];
        let known_list = if known_slice.is_empty() {
            "(belum ada kamus skill)".to_string()
        } else {
            known_slice.join(", ")
        };

        let prompt = format!(
            "Kamu adalah AI profil karier Vecta.\n\
             Ubah cerita bebas pengguna menjadi:\n\
             1. 'professional_summary' yang rapi dan ringkas.\n\
             2. 'estimated_experience_level'.\n\
             3. 'extracted_skills': ekstrak skill eksplisit maupun implisit ke nama baku bahasa Inggris (cocokkan dengan known_canonical_skills jika relevan), taksir proficiency dari bukti cerita.\n\
             4. 'recommended_keywords': 3 sampai 5 rekomendasi peran target terbagi 'primary_fit' dan 'adjacent_fit' lengkap dengan alasan singkat dan scrape_queries.\n\n\
             known_canonical_skills: [{known_list}]\n\n\
             Cerita Pengguna:\n{raw_story}"
        );

        self.generate_structured(api_key, &prompt, schema_d(), TEMP_CONTRACT_D)
            .await
    }

    async fn embed_text(
        &self,
        api_key: &str,
        text: &str,
        task_type: EmbeddingTaskType,
    ) -> Result<Vec<f32>, AppError> {
        let path = format!("/models/{GEMINI_EMBEDDING_MODEL}:embedContent");
        let payload = json!({
            "model": format!("models/{GEMINI_EMBEDDING_MODEL}"),
            "content": {
                "parts": [{ "text": text }]
            },
            "taskType": task_type.as_api_str(),
            "outputDimensionality": EMBEDDING_DIMENSIONS
        });

        let raw_response = self
            .send_with_retry(Method::POST, &path, api_key, Some(&payload))
            .await?;

        let parsed: EmbedContentResponse = serde_json::from_str(&raw_response).map_err(|e| {
            AppError::gemini_parse_failed(format!("Gagal memparsing respons embedding Gemini: {e}"))
        })?;

        let values = parsed
            .embedding
            .map(|e| e.values)
            .filter(|v| v.len() == EMBEDDING_DIMENSIONS && v.iter().all(|x| x.is_finite()))
            .ok_or_else(|| {
                AppError::gemini_parse_failed(format!(
                    "Respons embedding Gemini tidak valid (harus berisi tepat {EMBEDDING_DIMENSIONS} nilai finite)."
                ))
            })?;

        Ok(values)
    }
}

fn parse_structured_json<T: DeserializeOwned>(raw_json: &str) -> Result<T, AppError> {
    let trimmed = raw_json.trim();
    let cleaned = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);

    serde_json::from_str(cleaned).map_err(|e| {
        AppError::gemini_parse_failed(format!(
            "Respons JSON Gemini tidak memenuhi skema kontrak: {e}"
        ))
    })
}

pub fn parse_contract_a(raw_json: &str) -> Result<JobExtractionResult, AppError> {
    parse_structured_json(raw_json)
}

pub fn parse_contract_b(raw_json: &str) -> Result<GapAnalysis, AppError> {
    parse_structured_json(raw_json)
}

pub fn parse_contract_c(raw_json: &str) -> Result<GeneratedRoadmap, AppError> {
    parse_structured_json(raw_json)
}

pub fn parse_contract_d(raw_json: &str) -> Result<StoryProfileDraft, AppError> {
    parse_structured_json(raw_json)
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;
    use crate::error::ErrorCode;
    use crate::models::job::{GapSeverity, RequirementType, WorkMode};
    use crate::models::profile::{ExperienceLevel, FitType, Proficiency, SkillCategory};

    #[test]
    fn deserialize_contract_a_job_extraction_fixture() {
        let json = r#"{
            "normalized_role": "Backend Engineer",
            "experience_level": "junior",
            "work_mode": "hybrid",
            "role_summary": "Build scalable REST APIs and maintain PostgreSQL.",
            "skills": [
                {
                    "canonical_name": "Rust",
                    "category": "core_hard",
                    "requirement_type": "mandatory"
                },
                {
                    "canonical_name": "Docker",
                    "category": "tools_tech",
                    "requirement_type": "nice_to_have",
                    "matched_aliases": ["Docker Compose"]
                }
            ]
        }"#;

        let parsed = parse_contract_a(json).expect("contract A should parse");
        assert_eq!(parsed.normalized_role, "Backend Engineer");
        assert_eq!(parsed.experience_level, ExperienceLevel::Junior);
        assert_eq!(parsed.work_mode, WorkMode::Hybrid);
        assert_eq!(parsed.skills.len(), 2);
        assert!(parsed.skills[0].matched_aliases.is_empty());
        assert_eq!(parsed.skills[1].requirement_type, RequirementType::NiceToHave);
        assert_eq!(parsed.skills[1].matched_aliases, vec!["Docker Compose"]);
    }

    #[test]
    fn deserialize_contract_b_gap_analysis_fixture() {
        let json = r#"{
            "verdict_headline": "Kandidat kuat di backend lokal, perlu penguatan containerization.",
            "strength_highlights": ["Pengalaman langsung membangun API Rust & SQLite"],
            "prioritized_gaps": [
                {"skill": "Docker", "severity": "high", "reason": "Dibutuhkan untuk CI/CD produksi"}
            ],
            "cv_tailoring_tips": ["Sorot metrik latensi di proyek Vecta"]
        }"#;

        let parsed = parse_contract_b(json).expect("contract B should parse");
        assert_eq!(parsed.prioritized_gaps.len(), 1);
        assert_eq!(parsed.prioritized_gaps[0].severity, GapSeverity::High);
    }

    #[test]
    fn deserialize_contract_c_roadmap_fixture() {
        let json = r#"{
            "title": "Roadmap Backend Engineer 4 Minggu",
            "target_role": "Backend Engineer",
            "total_estimated_weeks": 4,
            "steps": [
                {
                    "phase_order": 1,
                    "phase_name": "Fundamental",
                    "target_skill": "Docker",
                    "topic_title": "Dasar Image & Container",
                    "learning_keywords": ["dockerfile multi-stage", "docker compose local dev"],
                    "mini_project_idea": "Containerize FastAPI worker dengan healthcheck",
                    "estimated_hours": 6
                }
            ]
        }"#;

        let parsed = parse_contract_c(json).expect("contract C should parse");
        assert_eq!(parsed.total_estimated_weeks, 4);
        assert_eq!(parsed.steps[0].phase_name, "Fundamental");
        assert_eq!(parsed.steps[0].target_skill, "Docker");
    }

    #[test]
    fn deserialize_contract_d_story_profile_fixture() {
        let json = r#"{
            "professional_summary": "Junior Software Developer berpengalaman di Rust, Vue, dan Python.",
            "estimated_experience_level": "junior",
            "extracted_skills": [
                {
                    "canonical_name": "Rust",
                    "category": "core_hard",
                    "proficiency": "intermediate",
                    "evidence": "Membangun core Tauri untuk Vecta"
                }
            ],
            "recommended_keywords": [
                {
                    "role_label": "Backend Developer",
                    "fit_type": "primary_fit",
                    "match_reason": "Kuat di Rust dan SQLite",
                    "scrape_queries": ["Backend Developer", "Rust Developer"]
                },
                {
                    "role_label": "Fullstack Engineer",
                    "fit_type": "adjacent_fit",
                    "match_reason": "Paham integrasi frontend-backend",
                    "scrape_queries": ["Fullstack Engineer"]
                }
            ]
        }"#;

        let parsed = parse_contract_d(json).expect("contract D should parse");
        assert_eq!(parsed.extracted_skills[0].category, SkillCategory::CoreHard);
        assert_eq!(
            parsed.extracted_skills[0].proficiency,
            Proficiency::Intermediate
        );
        assert_eq!(parsed.recommended_keywords[0].fit_type, FitType::PrimaryFit);
        assert_eq!(
            parsed.recommended_keywords[1].fit_type,
            FitType::AdjacentFit
        );
    }

    #[test]
    fn invalid_or_corrupt_json_maps_to_gemini_parse_failed() {
        let err1 = parse_contract_a("{broken json").expect_err("corrupt JSON must fail");
        assert_eq!(err1.code, ErrorCode::GeminiParseFailed);
        assert!(err1.retryable);

        let err2 = parse_contract_d(r#"{"professional_summary":"only one field"}"#)
            .expect_err("missing required fields must fail");
        assert_eq!(err2.code, ErrorCode::GeminiParseFailed);
        assert!(err2.retryable);
    }

    async fn spawn_mock_http_server(
        responses: Vec<(u16, String)>,
    ) -> (String, Arc<AtomicUsize>, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let count = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));

        let count_clone = Arc::clone(&count);
        let req_clone = Arc::clone(&requests);
        tokio::spawn(async move {
            for (status, body) in responses {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let mut buf = vec![0u8; 16 * 1024];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let raw_req = String::from_utf8_lossy(&buf[..n]).to_string();
                req_clone.lock().unwrap().push(raw_req);
                count_clone.fetch_add(1, Ordering::SeqCst);

                let resp = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });

        (format!("http://{addr}"), count, requests)
    }

    #[tokio::test]
    async fn http_client_uses_header_not_query_param() {
        let vals = vec![0.25f32; 768];
        let embed_body = serde_json::json!({ "embedding": { "values": vals } }).to_string();
        let (base_url, _count, requests) = spawn_mock_http_server(vec![
            (200, r#"{"name":"models/gemini-2.5-flash"}"#.into()),
            (200, embed_body),
        ])
        .await;

        let client = HttpGeminiClient::with_base_url(base_url, 1).expect("client");
        let key = "AIzaSyTESTSECRETHEADERONLY9999";

        client.test_api_key(key).await.expect("test_api_key ok");
        let vec = client
            .embed_text(key, "Rust Developer", EmbeddingTaskType::RetrievalQuery)
            .await
            .expect("embed ok");
        assert_eq!(vec.len(), 768);

        let recorded = requests.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        for req in recorded.iter() {
            let first_line = req.lines().next().unwrap_or("");
            assert!(
                !first_line.contains("key="),
                "API key must NEVER appear in URL query string: {first_line}"
            );
            assert!(
                req.to_lowercase()
                    .contains(&format!("x-goog-api-key: {}", key.to_lowercase())),
                "Expected x-goog-api-key header"
            );
        }
        assert!(recorded[1].contains("\"outputDimensionality\":768"));
        assert!(recorded[1].contains("\"taskType\":\"RETRIEVAL_QUERY\""));
    }

    #[tokio::test]
    async fn http_client_retries_429_and_maps_exhausted_to_rate_limited() {
        let (url_ok, count_ok, _) = spawn_mock_http_server(vec![
            (429, r#"{"error":{"message":"quota"}}"#.into()),
            (429, r#"{"error":{"message":"quota"}}"#.into()),
            (200, r#"{"name":"models/gemini-2.5-flash"}"#.into()),
        ])
        .await;
        let client_ok = HttpGeminiClient::with_base_url(url_ok, 1).expect("client");
        client_ok
            .test_api_key("AIzaSyRETRYOK1234")
            .await
            .expect("should succeed on 3rd attempt");
        assert_eq!(count_ok.load(Ordering::SeqCst), 3);

        let (url_fail, count_fail, _) = spawn_mock_http_server(vec![
            (429, "{}".into()),
            (429, "{}".into()),
            (429, "{}".into()),
            (429, "{}".into()),
        ])
        .await;
        let client_fail = HttpGeminiClient::with_base_url(url_fail, 1).expect("client");
        let err = client_fail
            .test_api_key("AIzaSyRETRYFAIL1234")
            .await
            .expect_err("exhausted retries must fail");
        assert_eq!(err.code, ErrorCode::GeminiRateLimited);
        assert!(err.retryable);
        assert_eq!(count_fail.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn http_client_maps_401_to_api_key_invalid_without_retry() {
        let (url, count, _) =
            spawn_mock_http_server(vec![(401, r#"{"error":{"message":"bad key"}}"#.into())]).await;
        let client = HttpGeminiClient::with_base_url(url, 1).expect("client");
        let err = client
            .test_api_key("AIzaSyINVALIDKEY9999")
            .await
            .expect_err("401 must fail immediately");
        assert_eq!(err.code, ErrorCode::ApiKeyInvalid);
        assert!(!err.retryable);
        assert!(!err.message.contains("AIzaSyINVALIDKEY9999"));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn http_client_rejects_invalid_header_api_key_immediately_without_retry() {
        use std::time::Instant;

        let (url, count, _) = spawn_mock_http_server(vec![]).await;
        let client = HttpGeminiClient::with_base_url(url, 150).expect("client");
        let start = Instant::now();
        let err = client
            .test_api_key("AIzaSyBAD\nNEWLINEKEY")
            .await
            .expect_err("newline in API key must fail fast");
        assert_eq!(err.code, ErrorCode::ApiKeyInvalid);
        assert!(!err.retryable);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(
            start.elapsed().as_millis() < 200,
            "Invalid header key must not sleep through retry backoff"
        );
    }

    #[tokio::test]
    async fn http_client_maps_non_key_400_on_generate_to_validation_error_not_api_key_invalid() {
        let (url, count, _) = spawn_mock_http_server(vec![
            (
                400,
                r#"{"error":{"code":400,"message":"Request payload size exceeds the limit","status":"INVALID_ARGUMENT"}}"#
                    .into(),
            ),
            (
                400,
                r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT"}}"#
                    .into(),
            ),
        ])
        .await;
        let client = HttpGeminiClient::with_base_url(url, 1).expect("client");

        let payload_err = client
            .extract_job("AIzaSyVALIDKEY1234", "some long jd", &[])
            .await
            .expect_err("non-key 400 must fail");
        assert_eq!(payload_err.code, ErrorCode::ValidationError);

        let key_err = client
            .extract_job("AIzaSyBADKEY1234", "some jd", &[])
            .await
            .expect_err("API key 400 must fail");
        assert_eq!(key_err.code, ErrorCode::ApiKeyInvalid);
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn generate_structured_skips_thought_parts_from_gemini_25_flash() {
        let response_body = serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        { "text": "Let me carefully analyze this job description first...", "thought": true },
                        { "text": r#"{"normalized_role":"Backend Engineer","experience_level":"junior","work_mode":"remote","role_summary":"Membangun REST API.","skills":[]}"# }
                    ]
                }
            }]
        })
        .to_string();

        let (url, _, _) = spawn_mock_http_server(vec![(200, response_body)]).await;
        let client = HttpGeminiClient::with_base_url(url, 1).expect("client");
        let parsed = client
            .extract_job("AIzaSyVALIDKEY1234", "Backend Engineer Rust", &[])
            .await
            .expect("must skip thought:true part and parse JSON part");
        assert_eq!(parsed.normalized_role, "Backend Engineer");
    }

    #[tokio::test]
    async fn embed_text_rejects_wrong_dimension_vector() {
        let bad_embed_body =
            serde_json::json!({ "embedding": { "values": [0.1f32, 0.2, 0.3] } }).to_string();
        let (url, _, _) = spawn_mock_http_server(vec![(200, bad_embed_body)]).await;
        let client = HttpGeminiClient::with_base_url(url, 1).expect("client");
        let err = client
            .embed_text("AIzaSyVALIDKEY1234", "Rust", EmbeddingTaskType::RetrievalQuery)
            .await
            .expect_err("wrong embedding dimension must fail");
        assert_eq!(err.code, ErrorCode::GeminiParseFailed);
    }
}
