use std::time::Duration;

use async_trait::async_trait;
use reqwest::redirect::Policy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::{Host, Url};

use crate::error::AppError;
use crate::models::job::{PortalScrapeStatus, ScrapeWarningItem};
use crate::models::profile::WorkerHealth;

/// Validate and normalize the Python Scraper Worker base URL.
/// Prevents SSRF by restricting connections strictly to loopback hosts (localhost, 127.0.0.1, ::1).
pub fn validate_worker_url(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    let parsed = Url::parse(trimmed).map_err(|_| {
        AppError::validation_error(
            "URL Python Scraper Worker tidak valid. Gunakan alamat lokal seperti http://127.0.0.1:8000.",
        )
    })?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::validation_error(
            "URL Python Scraper Worker harus menggunakan skema http:// atau https://.",
        ));
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(AppError::validation_error(
            "URL Python Scraper Worker tidak boleh mengandung kredensial autentikasi.",
        ));
    }

    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(AppError::validation_error(
            "URL Python Scraper Worker tidak boleh mengandung query string atau fragment.",
        ));
    }

    let is_loopback = match parsed.host() {
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };

    if !is_loopback {
        return Err(AppError::validation_error(
            "Demi keamanan (anti-SSRF), Python Scraper Worker hanya diizinkan pada host lokal (127.0.0.1, localhost, atau ::1).",
        ));
    }

    Ok(parsed.as_str().trim_end_matches('/').to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerScrapeRequest {
    pub queries: Vec<String>,
    pub sources: Vec<String>,
    pub location: String,
    pub limit_per_query: u32,
    #[serde(default = "default_max_pages")]
    pub max_pages: u32,
    #[serde(default)]
    pub posted_within_days: Option<u32>,
    #[serde(default = "default_work_mode")]
    pub work_mode: String,
}

fn default_max_pages() -> u32 {
    2
}

fn default_work_mode() -> String {
    "any".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerScrapedJob {
    pub source: String,
    pub external_id: String,
    pub url: Option<String>,
    pub title: String,
    pub company: String,
    pub location: Option<String>,
    pub raw_description: String,
    #[serde(default)]
    pub posted_at: Option<String>,
    #[serde(default)]
    pub salary_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerScrapeStats {
    pub total_fetched: u32,
    pub deduplicated_in_batch: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerScrapeResponse {
    pub jobs: Vec<WorkerScrapedJob>,
    pub stats: WorkerScrapeStats,
    pub warnings: Vec<ScrapeWarningItem>,
    #[serde(default)]
    pub portal_statuses: Vec<PortalScrapeStatus>,
}

#[async_trait]
pub trait WorkerClient: Send + Sync {
    async fn check_health(&self, base_url: &str) -> Result<WorkerHealth, AppError>;
    async fn scrape(
        &self,
        base_url: &str,
        request: &WorkerScrapeRequest,
    ) -> Result<WorkerScrapeResponse, AppError>;
}

pub struct HttpWorkerClient {
    health_http: Client,
    scrape_http: Client,
}

impl HttpWorkerClient {
    pub fn new() -> Result<Self, AppError> {
        let health_http = Client::builder()
            .timeout(Duration::from_secs(5))
            .redirect(Policy::none())
            .build()
            .map_err(|e| {
                eprintln!("[vecta::worker] failed to build health HTTP client: {e}");
                AppError::db_error("Gagal menginisialisasi HTTP client untuk Python Worker.")
            })?;
        let scrape_http = Client::builder()
            .timeout(Duration::from_secs(120))
            .redirect(Policy::none())
            .build()
            .map_err(|e| {
                eprintln!("[vecta::worker] failed to build scrape HTTP client: {e}");
                AppError::db_error("Gagal menginisialisasi HTTP client untuk Python Worker.")
            })?;
        Ok(Self {
            health_http,
            scrape_http,
        })
    }
}

#[async_trait]
impl WorkerClient for HttpWorkerClient {
    async fn check_health(&self, base_url: &str) -> Result<WorkerHealth, AppError> {
        let safe_base = validate_worker_url(base_url)?;
        let url = format!("{safe_base}/health");
        let resp = self.health_http.get(&url).send().await.map_err(|_| {
            AppError::scraper_unreachable(
                "Python Scraper Worker tidak dapat dijangkau. Jalankan: uv run uvicorn app.main:app --host 127.0.0.1 --port 8000",
            )
        })?;

        if resp.status().is_redirection() {
            return Err(AppError::scraper_unreachable(
                "Python Scraper Worker mengembalikan redirect HTTP yang diblokir demi keamanan.",
            ));
        }

        if !resp.status().is_success() {
            return Err(AppError::scraper_unreachable(format!(
                "Python Scraper Worker mengembalikan HTTP {}.",
                resp.status().as_u16()
            )));
        }

        resp.json::<WorkerHealth>().await.map_err(|e| {
            eprintln!("[vecta::worker] invalid /health response: {e}");
            AppError::scraper_unreachable("Respons /health Python Worker tidak valid.")
        })
    }

    async fn scrape(
        &self,
        base_url: &str,
        request: &WorkerScrapeRequest,
    ) -> Result<WorkerScrapeResponse, AppError> {
        let safe_base = validate_worker_url(base_url)?;
        let url = format!("{safe_base}/api/v1/scrape");
        let resp = self
            .scrape_http
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(|_| {
                AppError::scraper_unreachable(
                    "Python Scraper Worker tidak dapat dijangkau saat scraping. Jalankan: uv run uvicorn app.main:app --host 127.0.0.1 --port 8000",
                )
            })?;

        if resp.status().is_redirection() {
            return Err(AppError::scraper_unreachable(
                "Endpoint scraping Python Worker mengembalikan redirect HTTP yang diblokir demi keamanan.",
            ));
        }

        if matches!(
            resp.status(),
            reqwest::StatusCode::BAD_REQUEST | reqwest::StatusCode::UNPROCESSABLE_ENTITY
        ) {
            return Err(AppError::validation_error(format!(
                "Parameter scraping ditolak oleh Python Worker (HTTP {}).",
                resp.status().as_u16()
            )));
        }

        if !resp.status().is_success() {
            return Err(AppError::scraper_unreachable(format!(
                "Endpoint scraping Python Worker gagal dengan HTTP {}.",
                resp.status().as_u16()
            )));
        }

        resp.json::<WorkerScrapeResponse>().await.map_err(|e| {
            eprintln!("[vecta::worker] invalid /api/v1/scrape response: {e}");
            AppError::scraper_unreachable(
                "Respons /api/v1/scrape Python Worker tidak dapat diparsing.",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;
    use crate::error::ErrorCode;

    #[tokio::test]
    async fn http_worker_client_maps_422_to_validation_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = vec![0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"detail":"invalid source"}"#;
                let resp = format!(
                    "HTTP/1.1 422 Unprocessable Entity\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });

        let client = HttpWorkerClient::new().expect("client");
        let err = client
            .scrape(
                &format!("http://{addr}"),
                &WorkerScrapeRequest {
                    queries: vec!["Rust".into()],
                    sources: vec!["invalid".into()],
                    location: "Indonesia".into(),
                    limit_per_query: 5,
                    max_pages: 1,
                    posted_within_days: None,
                    work_mode: "any".into(),
                },
            )
            .await
            .expect_err("422 must fail");
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert!(!err.retryable);
    }

    #[test]
    fn validate_worker_url_blocks_ssrf_and_allows_loopback() {
        assert_eq!(
            validate_worker_url("http://127.0.0.1:8000/").expect("ipv4 loopback"),
            "http://127.0.0.1:8000"
        );
        assert_eq!(
            validate_worker_url("http://localhost:8001").expect("localhost"),
            "http://localhost:8001"
        );
        assert_eq!(
            validate_worker_url("http://[::1]:8000").expect("ipv6 loopback"),
            "http://[::1]:8000"
        );

        for bad in [
            "http://169.254.169.254/latest/meta-data",
            "http://192.168.1.1:8000",
            "https://example.com",
            "ftp://127.0.0.1:8000",
            "http://user:pass@127.0.0.1:8000",
            "http://127.0.0.1:8000?redirect=1",
        ] {
            let err = validate_worker_url(bad).expect_err(&format!("must reject {bad}"));
            assert_eq!(err.code, ErrorCode::ValidationError);
        }
    }

    #[tokio::test]
    async fn http_worker_client_blocks_http_redirects() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = vec![0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let resp = "HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });

        let client = HttpWorkerClient::new().expect("client");
        let err = client
            .check_health(&format!("http://{addr}"))
            .await
            .expect_err("302 redirect must be rejected");
        assert_eq!(err.code, ErrorCode::ScraperUnreachable);
    }
}
