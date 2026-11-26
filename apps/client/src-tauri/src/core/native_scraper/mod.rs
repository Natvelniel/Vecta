use std::sync::Arc;

use async_trait::async_trait;

use crate::core::native_scraper::client::ResilientHttpClient;
use crate::core::native_scraper::orchestrator::run_concurrent_scrape;
use crate::core::native_scraper::portals::{jobstreet, karirhub, linkedin};
use crate::core::worker::{WorkerClient, WorkerScrapeRequest, WorkerScrapeResponse};
use crate::error::AppError;
use crate::models::profile::WorkerHealth;

pub mod client;
pub mod orchestrator;
pub mod portals;
pub mod quality;

pub struct NativeScraperClient {
    http: Arc<ResilientHttpClient>,
}

impl NativeScraperClient {
    pub fn new() -> Result<Self, AppError> {
        let http = ResilientHttpClient::new().map_err(|e| {
            eprintln!("[vecta::native_scraper] HTTP client init failed: {e}");
            AppError::db_error("Gagal menginisialisasi HTTP client scraper bawaan.")
        })?;
        Ok(Self {
            http: Arc::new(http),
        })
    }

    /// Fetches full structured description on-demand for an existing job whose stored text is sparse.
    pub async fn fetch_full_description(
        &self,
        source: &str,
        external_id: &str,
        url: Option<&str>,
    ) -> Result<Option<String>, String> {
        let res = match source {
            "jobstreet" => {
                let target = url.unwrap_or(external_id);
                jobstreet::fetch_jobstreet_detail(&self.http, target).await
            }
            "linkedin" => {
                let target = url.unwrap_or(external_id);
                linkedin::fetch_linkedin_detail(&self.http, target).await
            }
            "karirhub" => karirhub::fetch_karirhub_detail(&self.http, external_id).await,
            _ => return Ok(None),
        };
        res.map_err(|e| e.message().to_string())
    }
}

#[async_trait]
impl WorkerClient for NativeScraperClient {
    async fn check_health(&self, _base_url: &str) -> Result<WorkerHealth, AppError> {
        Ok(WorkerHealth {
            status: "ok".into(),
            version: "native-rust-4.0".into(),
            available_sources: vec![
                "jobstreet".into(),
                "linkedin".into(),
                "kalibrr".into(),
                "dealls".into(),
                "karirhub".into(),
                "remotive".into(),
            ],
        })
    }

    async fn scrape(
        &self,
        _base_url: &str,
        request: &WorkerScrapeRequest,
    ) -> Result<WorkerScrapeResponse, AppError> {
        Ok(run_concurrent_scrape(Arc::clone(&self.http), request).await)
    }
}
