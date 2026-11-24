use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::job::ScrapeWarningItem;
use crate::models::profile::WorkerHealth;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerScrapeRequest {
    pub queries: Vec<String>,
    pub sources: Vec<String>,
    pub location: String,
    pub limit_per_query: u32,
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
            .build()
            .map_err(|e| AppError::db_error(format!("Gagal membuat HTTP client worker: {e}")))?;
        let scrape_http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::db_error(format!("Gagal membuat HTTP client worker: {e}")))?;
        Ok(Self {
            health_http,
            scrape_http,
        })
    }
}

#[async_trait]
impl WorkerClient for HttpWorkerClient {
    async fn check_health(&self, base_url: &str) -> Result<WorkerHealth, AppError> {
        let url = format!("{}/health", base_url.trim_end_matches('/'));
        let resp = self.health_http.get(&url).send().await.map_err(|_| {
            AppError::scraper_unreachable(
                "Python Scraper Worker tidak dapat dijangkau. Jalankan: uv run uvicorn app.main:app --host 127.0.0.1 --port 8000",
            )
        })?;

        if !resp.status().is_success() {
            return Err(AppError::scraper_unreachable(format!(
                "Python Scraper Worker mengembalikan HTTP {}.",
                resp.status().as_u16()
            )));
        }

        resp.json::<WorkerHealth>().await.map_err(|e| {
            AppError::scraper_unreachable(format!(
                "Respons /health Python Worker tidak valid: {e}"
            ))
        })
    }

    async fn scrape(
        &self,
        base_url: &str,
        request: &WorkerScrapeRequest,
    ) -> Result<WorkerScrapeResponse, AppError> {
        let url = format!("{}/api/v1/scrape", base_url.trim_end_matches('/'));
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
            AppError::scraper_unreachable(format!(
                "Respons /api/v1/scrape Python Worker tidak dapat diparsing: {e}"
            ))
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
                },
            )
            .await
            .expect_err("422 must fail");
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert!(!err.retryable);
    }
}
