use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::core::native_scraper::client::{PortalCircuitBreaker, ResilientHttpClient};
use crate::core::native_scraper::portals::{
    dealls, jobstreet, kalibrr, karirhub, linkedin, remotive,
};
use crate::core::worker::{
    WorkerScrapeRequest, WorkerScrapeResponse, WorkerScrapeStats, WorkerScrapedJob,
};
use crate::models::job::{PortalScrapeStatus, ScrapeWarningItem};

const MAX_CONCURRENT_WORKERS: usize = 4;

struct UnitOutcome {
    source: String,
    latency_ms: u64,
    jobs: Vec<WorkerScrapedJob>,
    transport_error: Option<String>,
    warning: Option<ScrapeWarningItem>,
}

/// Runs all `(source, query)` scrape jobs across a bounded concurrent worker pool (max 4 workers),
/// with a per-portal circuit breaker and telemetry output (`PortalScrapeStatus`).
pub async fn run_concurrent_scrape(
    http: Arc<ResilientHttpClient>,
    request: &WorkerScrapeRequest,
) -> WorkerScrapeResponse {
    let started = Instant::now();
    let breaker = Arc::new(PortalCircuitBreaker::new(2));
    let pool_sem = Arc::new(Semaphore::new(MAX_CONCURRENT_WORKERS));

    let mut tasks = Vec::new();
    for query in &request.queries {
        for source in &request.sources {
            tasks.push((source.clone(), query.clone()));
        }
    }

    let mut set = JoinSet::new();
    for (source, query) in tasks {
        let http = Arc::clone(&http);
        let breaker = Arc::clone(&breaker);
        let permit_sem = Arc::clone(&pool_sem);
        let location = request.location.clone();
        let limit = request.limit_per_query;
        let max_pages = request.max_pages;
        let posted_within_days = request.posted_within_days;
        let work_mode = request.work_mode.clone();

        set.spawn(async move {
            let _permit = permit_sem.acquire_owned().await.expect("pool semaphore open");
            let unit_start = Instant::now();

            if breaker.is_open(&source).await {
                return UnitOutcome {
                    source: source.clone(),
                    latency_ms: 0,
                    jobs: vec![],
                    transport_error: Some("Circuit breaker aktif untuk portal ini.".into()),
                    warning: None,
                };
            }

            let res = match source.as_str() {
                "jobstreet" => {
                    jobstreet::scrape(
                        http,
                        &query,
                        &location,
                        limit,
                        max_pages,
                        posted_within_days,
                        &work_mode,
                    )
                    .await
                }
                "linkedin" => {
                    linkedin::scrape(
                        http,
                        &query,
                        &location,
                        limit,
                        max_pages,
                        posted_within_days,
                        &work_mode,
                    )
                    .await
                }
                "kalibrr" => kalibrr::scrape(http, &query, &location, limit, max_pages).await,
                "dealls" => dealls::scrape(http, &query, &location, limit, max_pages).await,
                "karirhub" => karirhub::scrape(http, &query, &location, limit, max_pages).await,
                "remotive" => remotive::scrape(http, &query, limit).await,
                other => {
                    return UnitOutcome {
                        source: source.clone(),
                        latency_ms: unit_start.elapsed().as_millis() as u64,
                        jobs: vec![],
                        transport_error: None,
                        warning: Some(ScrapeWarningItem {
                            source: source.clone(),
                            code: "NATIVE_PORTAL_UNSUPPORTED".into(),
                            message: format!(
                                "Portal '{other}' hanya tersedia melalui Python Worker opsional."
                            ),
                        }),
                    };
                }
            };

            let latency_ms = unit_start.elapsed().as_millis() as u64;
            match res {
                Ok(jobs) => {
                    breaker.record_success(&source).await;
                    UnitOutcome {
                        source,
                        latency_ms,
                        jobs,
                        transport_error: None,
                        warning: None,
                    }
                }
                Err(err) => {
                    let is_transport = err.is_transport();
                    if is_transport {
                        breaker.record_transport_failure(&source).await;
                    }
                    let msg = format!("{source} ('{query}'): {}", err.message());
                    UnitOutcome {
                        source: source.clone(),
                        latency_ms,
                        jobs: vec![],
                        transport_error: is_transport.then(|| msg.clone()),
                        warning: Some(ScrapeWarningItem {
                            source,
                            code: if is_transport {
                                "NATIVE_TRANSPORT_WARN".into()
                            } else {
                                "NATIVE_HTTP_WARN".into()
                            },
                            message: msg,
                        }),
                    }
                }
            }
        });
    }

    let mut all_jobs = Vec::new();
    let mut warnings = Vec::new();
    let mut metrics: HashMap<String, (u32, u32, u64)> = HashMap::new();
    // Tuple: (ok_units, err_units, max_latency_ms)
    for src in &request.sources {
        metrics.entry(src.clone()).or_insert((0, 0, 0));
    }
    let mut found_per_source: HashMap<String, u32> = HashMap::new();

    while let Some(joined) = set.join_next().await {
        let Ok(outcome) = joined else {
            continue;
        };
        let entry = metrics.entry(outcome.source.clone()).or_insert((0, 0, 0));
        entry.2 = entry.2.max(outcome.latency_ms);
        if outcome.warning.is_some() || outcome.transport_error.is_some() {
            entry.1 += 1;
            if let Some(w) = outcome.warning {
                warnings.push(w);
            }
        } else {
            entry.0 += 1;
            let count = outcome.jobs.len() as u32;
            *found_per_source.entry(outcome.source.clone()).or_insert(0) += count;
            all_jobs.extend(outcome.jobs);
        }
    }

    let total_fetched = all_jobs.len() as u32;
    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(all_jobs.len());
    for job in all_jobs {
        if seen.insert((job.source.clone(), job.external_id.clone())) {
            deduped.push(job);
        }
    }
    let deduplicated_in_batch = total_fetched.saturating_sub(deduped.len() as u32);

    let mut portal_statuses: Vec<PortalScrapeStatus> = Vec::with_capacity(request.sources.len());
    for src in &request.sources {
        let (ok_cnt, err_cnt, latency_ms) = metrics.get(src).copied().unwrap_or((0, 0, 0));
        let jobs_found = found_per_source.get(src).copied().unwrap_or(0);
        let status = if ok_cnt > 0 && err_cnt == 0 {
            if latency_ms >= 6_000 {
                "degraded"
            } else {
                "ok"
            }
        } else if ok_cnt > 0 {
            "degraded"
        } else if err_cnt > 0 {
            "unreachable"
        } else {
            "ok"
        };
        portal_statuses.push(PortalScrapeStatus {
            source: src.clone(),
            status: status.into(),
            jobs_found,
            latency_ms,
        });
    }

    WorkerScrapeResponse {
        jobs: deduped,
        stats: WorkerScrapeStats {
            total_fetched,
            deduplicated_in_batch,
            duration_ms: started.elapsed().as_millis() as u64,
        },
        warnings,
        portal_statuses,
    }
}
