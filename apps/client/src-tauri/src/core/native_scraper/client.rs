use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::{Client, StatusCode};
use scraper::Html;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, Semaphore};

pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
pub const MAX_DESC_CHARS: usize = 12_000;

#[derive(Debug, Clone)]
pub enum HttpFetchError {
    Transport(String),
    HttpStatus(u16, String),
}

impl HttpFetchError {
    pub fn is_transport(&self) -> bool {
        matches!(self, Self::Transport(_))
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Transport(m) | Self::HttpStatus(_, m) => m,
        }
    }
}

pub fn deterministic_external_id(source: &str, company: &str, title: &str, location: &str) -> String {
    let mut hasher = Sha256::new();
    let payload = format!(
        "{}|{}|{}|{}",
        source.trim().to_lowercase(),
        company.trim().to_lowercase(),
        title.trim().to_lowercase(),
        location.trim().to_lowercase()
    );
    hasher.update(payload.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    format!("{source}-{hex}")
}

/// Converts HTML fragment into clean, readable plain text while preserving bullets and line structure.
pub fn clean_html_to_text(raw: &str) -> String {
    let normalized_html = raw
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n")
        .replace("</h1>", "\n")
        .replace("</h2>", "\n")
        .replace("</h3>", "\n")
        .replace("</h4>", "\n")
        .replace("<li>", "\n- ")
        .replace("</li>", "\n");

    let fragment = Html::parse_fragment(&normalized_html);
    let mut raw_text = String::with_capacity(raw.len().min(MAX_DESC_CHARS));
    for chunk in fragment.root_element().text() {
        raw_text.push_str(chunk);
    }

    let mut out = String::with_capacity(raw_text.len().min(MAX_DESC_CHARS));
    let mut prev_was_empty = false;
    for line in raw_text.lines() {
        let trimmed = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.is_empty() {
            if !prev_was_empty && !out.is_empty() {
                out.push('\n');
                prev_was_empty = true;
            }
            continue;
        }
        prev_was_empty = false;
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&trimmed);
        if out.chars().count() >= MAX_DESC_CHARS {
            break;
        }
    }
    out.chars().take(MAX_DESC_CHARS).collect()
}

fn jitter_ms(max_jitter: u64) -> u64 {
    if max_jitter == 0 {
        return 0;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(17);
    nanos % max_jitter
}

/// Dual-stack HTTP client hardened for mobile cellular/Wi-Fi networks (HTTP/2 + automatic HTTP/1.1 fallback).
pub struct ResilientHttpClient {
    primary: Client,
    h1_fallback: Client,
    host_semaphores: Mutex<HashMap<String, Arc<Semaphore>>>,
}

impl ResilientHttpClient {
    pub fn new() -> Result<Self, String> {
        let primary = Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(18))
            .tcp_keepalive(Duration::from_secs(30))
            .pool_max_idle_per_host(6)
            .user_agent(DEFAULT_USER_AGENT)
            .build()
            .map_err(|e| format!("Primary HTTP client init failed: {e}"))?;

        let h1_fallback = Client::builder()
            .http1_only()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .tcp_keepalive(Duration::from_secs(30))
            .pool_max_idle_per_host(4)
            .user_agent(DEFAULT_USER_AGENT)
            .build()
            .map_err(|e| format!("Fallback HTTP/1.1 client init failed: {e}"))?;

        Ok(Self {
            primary,
            h1_fallback,
            host_semaphores: Mutex::new(HashMap::new()),
        })
    }

    async fn acquire_host_permit(&self, host_key: &str) -> tokio::sync::OwnedSemaphorePermit {
        let sem = {
            let mut map = self.host_semaphores.lock().await;
            Arc::clone(
                map.entry(host_key.to_string())
                    .or_insert_with(|| Arc::new(Semaphore::new(2))),
            )
        };
        sem.acquire_owned()
            .await
            .expect("host semaphore must remain open")
    }

    async fn execute_once(
        client: &Client,
        url: &str,
        headers: &[(&str, &str)],
        query: &[(&str, String)],
    ) -> Result<String, (HttpFetchError, Option<u64>)> {
        let mut req = client
            .get(url)
            .header("Accept-Language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7");
        for &(k, v) in headers {
            req = req.header(k, v);
        }
        if !query.is_empty() {
            req = req.query(query);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| (HttpFetchError::Transport(e.to_string()), None))?;

        let status = resp.status();
        if !status.is_success() {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|secs| secs.clamp(1, 3) * 1000);
            return Err((
                HttpFetchError::HttpStatus(status.as_u16(), format!("HTTP {}", status.as_u16())),
                retry_after,
            ));
        }

        resp.text()
            .await
            .map_err(|e| (HttpFetchError::Transport(e.to_string()), None))
    }

    pub async fn get_text(
        &self,
        host_key: &str,
        url: &str,
        headers: &[(&str, &str)],
        query: &[(&str, String)],
    ) -> Result<String, HttpFetchError> {
        let _permit = self.acquire_host_permit(host_key).await;

        match Self::execute_once(&self.primary, url, headers, query).await {
            Ok(text) => Ok(text),
            Err((HttpFetchError::Transport(first_err), _)) => {
                // Retry on HTTP/1.1 fallback to bypass Android mobile HTTP/2 ALPN or TLS resets
                tokio::time::sleep(Duration::from_millis(320 + jitter_ms(180))).await;
                match Self::execute_once(&self.h1_fallback, url, headers, query).await {
                    Ok(text) => Ok(text),
                    Err((HttpFetchError::Transport(second_err), _)) => Err(HttpFetchError::Transport(
                        format!("{first_err}; h1 fallback: {second_err}"),
                    )),
                    Err((other, _)) => Err(other),
                }
            }
            Err((HttpFetchError::HttpStatus(code, _msg), retry_ms))
                if matches!(
                    StatusCode::from_u16(code).ok(),
                    Some(
                        StatusCode::TOO_MANY_REQUESTS
                            | StatusCode::BAD_GATEWAY
                            | StatusCode::SERVICE_UNAVAILABLE
                            | StatusCode::GATEWAY_TIMEOUT
                    )
                ) =>
            {
                let delay = retry_ms.unwrap_or(600) + jitter_ms(200);
                tokio::time::sleep(Duration::from_millis(delay)).await;
                Self::execute_once(&self.primary, url, headers, query)
                    .await
                    .map_err(|(e, _)| e)
            }
            Err((err, _)) => Err(err),
        }
    }

    pub async fn get_json(
        &self,
        host_key: &str,
        url: &str,
        headers: &[(&str, &str)],
        query: &[(&str, String)],
    ) -> Result<Value, HttpFetchError> {
        let text = self.get_text(host_key, url, headers, query).await?;
        serde_json::from_str::<Value>(&text)
            .map_err(|e| HttpFetchError::Transport(format!("JSON parse error: {e}")))
    }
}

/// Tracks per-portal consecutive transport failures during a scrape run to avoid repeated timeouts.
pub struct PortalCircuitBreaker {
    failures: Mutex<HashMap<String, Arc<(AtomicUsize, AtomicBool)>>>,
    threshold: usize,
}

impl PortalCircuitBreaker {
    pub fn new(threshold: usize) -> Self {
        Self {
            failures: Mutex::new(HashMap::new()),
            threshold: threshold.max(1),
        }
    }

    async fn slot(&self, source: &str) -> Arc<(AtomicUsize, AtomicBool)> {
        let mut map = self.failures.lock().await;
        Arc::clone(
            map.entry(source.to_string())
                .or_insert_with(|| Arc::new((AtomicUsize::new(0), AtomicBool::new(false)))),
        )
    }

    pub async fn is_open(&self, source: &str) -> bool {
        let slot = self.slot(source).await;
        slot.1.load(Ordering::Relaxed)
    }

    pub async fn record_success(&self, source: &str) {
        let slot = self.slot(source).await;
        slot.0.store(0, Ordering::Relaxed);
        slot.1.store(false, Ordering::Relaxed);
    }

    pub async fn record_transport_failure(&self, source: &str) -> bool {
        let slot = self.slot(source).await;
        let next = slot.0.fetch_add(1, Ordering::Relaxed) + 1;
        if next >= self.threshold {
            slot.1.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn clean_html_preserves_bullets_and_headings() {
        let html = "<p><strong>Deskripsi:</strong></p><ul><li>Membangun API Rust</li><li>Kelola SQLite</li></ul>";
        let out = clean_html_to_text(html);
        assert!(out.contains("Deskripsi:"));
        assert!(out.contains("- Membangun API Rust"));
        assert!(out.contains("- Kelola SQLite"));
    }

    #[tokio::test]
    async fn resilient_client_retries_503_once_and_succeeds() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            for attempt in 0..2 {
                if let Ok((mut sock, _)) = listener.accept().await {
                    let mut buf = vec![0u8; 2048];
                    let _ = sock.read(&mut buf).await;
                    let resp = if attempt == 0 {
                        "HTTP/1.1 503 Service Unavailable\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                    } else {
                        let body = r#"{"ok":true}"#;
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )
                    };
                    let _ = sock.write_all(resp.as_bytes()).await;
                    let _ = sock.shutdown().await;
                }
            }
        });

        let client = ResilientHttpClient::new().expect("client");
        let val = client
            .get_json("test-retry", &format!("http://{addr}/test"), &[], &[])
            .await
            .expect("retry succeeds on second attempt");
        assert_eq!(val.get("ok").and_then(|v| v.as_bool()), Some(true));
    }

    #[tokio::test]
    async fn circuit_breaker_opens_after_threshold_transport_failures() {
        let cb = PortalCircuitBreaker::new(2);
        assert!(!cb.is_open("kalibrr").await);
        assert!(!cb.record_transport_failure("kalibrr").await);
        assert!(!cb.is_open("kalibrr").await);
        assert!(cb.record_transport_failure("kalibrr").await);
        assert!(cb.is_open("kalibrr").await);
        cb.record_success("kalibrr").await;
        assert!(!cb.is_open("kalibrr").await);
    }

    #[tokio::test]
    async fn concurrent_orchestrator_fans_out_and_emits_portal_statuses() {
        let http = Arc::new(ResilientHttpClient::new().expect("client"));
        let req = crate::core::worker::WorkerScrapeRequest {
            queries: vec!["Rust".into()],
            sources: vec!["glints".into(), "unknown_x".into()],
            location: "Indonesia".into(),
            limit_per_query: 5,
            max_pages: 1,
            posted_within_days: None,
            work_mode: "any".into(),
        };
        let resp = crate::core::native_scraper::orchestrator::run_concurrent_scrape(http, &req).await;
        assert_eq!(resp.portal_statuses.len(), 2);
        assert_eq!(resp.warnings.len(), 2);
    }
}
