use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::core::worker::{
    WorkerClient, WorkerScrapeRequest, WorkerScrapeResponse, WorkerScrapeStats, WorkerScrapedJob,
};
use crate::error::AppError;
use crate::models::job::ScrapeWarningItem;
use crate::models::profile::WorkerHealth;

pub mod quality;

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const MAX_DESC_CHARS: usize = 12_000;

fn deterministic_external_id(source: &str, company: &str, title: &str, location: &str) -> String {
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

fn clean_html_to_text(raw: &str) -> String {
    let fragment = Html::parse_fragment(raw);
    let mut out = String::with_capacity(raw.len().min(MAX_DESC_CHARS));
    for chunk in fragment.root_element().text() {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(trimmed);
        if out.len() >= MAX_DESC_CHARS {
            break;
        }
    }
    out.chars().take(MAX_DESC_CHARS).collect()
}

pub struct NativeScraperClient {
    http: Client,
}

impl NativeScraperClient {
    pub fn new() -> Result<Self, AppError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(DEFAULT_USER_AGENT)
            .build()
            .map_err(|e| {
                eprintln!("[vecta::native_scraper] HTTP client init failed: {e}");
                AppError::db_error("Gagal menginisialisasi HTTP client scraper bawaan.")
            })?;
        Ok(Self { http })
    }

    async fn scrape_jobstreet(
        &self,
        query: &str,
        location: &str,
        limit: u32,
        max_pages: u32,
        posted_within_days: Option<u32>,
        work_mode: &str,
    ) -> Result<Vec<WorkerScrapedJob>, String> {
        let pages = max_pages.clamp(1, 5);
        let page_size = limit.clamp(1, 50);
        let mut out = Vec::new();

        for page in 1..=pages {
            if page > 1 {
                tokio::time::sleep(Duration::from_millis(320)).await;
            }
            let mut params: Vec<(&str, String)> = vec![
                ("siteKey", "ID-Main".into()),
                ("keywords", query.into()),
                ("where", location.into()),
                ("page", page.to_string()),
                ("pageSize", page_size.to_string()),
                ("locale", "id-ID".into()),
            ];
            if let Some(days) = posted_within_days {
                params.push(("daterange", days.to_string()));
            }
            match work_mode {
                "remote" => params.push(("workarrangement", "3".into())),
                "hybrid" => params.push(("workarrangement", "2".into())),
                "onsite" => params.push(("workarrangement", "1".into())),
                _ => {}
            }

            let resp = self
                .http
                .get("https://id.jobstreet.com/api/jobsearch/v5/search")
                .header("Accept", "application/json")
                .header("Referer", "https://id.jobstreet.com/")
                .query(&params)
                .send()
                .await
                .map_err(|e| format!("JobStreet koneksi gagal untuk '{query}': {e}"))?;

            if !resp.status().is_success() {
                if !out.is_empty() {
                    break;
                }
                return Err(format!(
                    "JobStreet HTTP {} untuk query '{query}'.",
                    resp.status().as_u16()
                ));
            }

            let body: Value = resp
                .json()
                .await
                .map_err(|e| format!("JobStreet JSON invalid: {e}"))?;
            let Some(items) = body.get("data").and_then(|d| d.as_array()) else {
                break;
            };
            if items.is_empty() {
                break;
            }

            for item in items.iter().take(page_size as usize) {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if title.is_empty() {
                    continue;
                }
                let company = item
                    .get("advertiser")
                    .and_then(|a| a.get("description"))
                    .and_then(|v| v.as_str())
                    .or_else(|| item.get("companyName").and_then(|v| v.as_str()))
                    .unwrap_or("Unknown Company")
                    .trim()
                    .to_string();
                let loc = item
                    .get("locations")
                    .and_then(|l| l.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|first| first.get("label"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| location.to_string());

                let teaser = item.get("teaser").and_then(|v| v.as_str()).unwrap_or("");
                let bullets = item
                    .get("bulletPoints")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|b| b.as_str())
                            .map(|s| format!("- {s}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                let combined_desc = clean_html_to_text(&format!("{teaser}\n{bullets}"));
                if combined_desc.len() < 15 {
                    continue;
                }

                let raw_id = item
                    .get("id")
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        _ => String::new(),
                    })
                    .unwrap_or_default();
                let external_id = if !raw_id.is_empty() {
                    format!("js-{raw_id}")
                } else {
                    deterministic_external_id("jobstreet", &company, &title, &loc)
                };
                let url = if !raw_id.is_empty() {
                    Some(format!("https://id.jobstreet.com/id/job/{raw_id}"))
                } else {
                    None
                };
                let salary_text = item
                    .get("salaryLabel")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                let posted_at = item
                    .get("listingDate")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                out.push(WorkerScrapedJob {
                    source: "jobstreet".into(),
                    external_id,
                    url,
                    title,
                    company,
                    location: Some(loc),
                    raw_description: combined_desc,
                    posted_at,
                    salary_text,
                });
            }
        }
        Ok(out)
    }

    async fn scrape_linkedin_guest(
        &self,
        query: &str,
        location: &str,
        limit: u32,
        max_pages: u32,
        posted_within_days: Option<u32>,
        work_mode: &str,
    ) -> Result<Vec<WorkerScrapedJob>, String> {
        let pages = max_pages.clamp(1, 3);
        let mut out = Vec::new();
        let card_sel = Selector::parse("li").unwrap();
        let title_sel = Selector::parse("h3.base-search-card__title").unwrap();
        let comp_sel = Selector::parse("h4.base-search-card__subtitle").unwrap();
        let loc_sel = Selector::parse("span.job-search-card__location").unwrap();
        let link_sel = Selector::parse("a.base-card__full-link").unwrap();
        let date_sel = Selector::parse("time").unwrap();
        let desc_sel = Selector::parse("div.show-more-less-html__markup").unwrap();

        for page in 0..pages {
            if page > 0 {
                tokio::time::sleep(Duration::from_millis(600)).await;
            }
            let start = page * 25;
            let mut params: Vec<(&str, String)> = vec![
                ("keywords", query.into()),
                ("location", location.into()),
                ("start", start.to_string()),
            ];
            if let Some(days) = posted_within_days {
                params.push(("f_TPR", format!("r{}", days * 86_400)));
            }
            match work_mode {
                "remote" => params.push(("f_WT", "2".into())),
                "hybrid" => params.push(("f_WT", "3".into())),
                "onsite" => params.push(("f_WT", "1".into())),
                _ => {}
            }

            let resp = self
                .http
                .get("https://www.linkedin.com/jobs-guest/jobs/api/seeMoreJobPostings/search")
                .query(&params)
                .send()
                .await
                .map_err(|e| format!("LinkedIn Guest koneksi gagal untuk '{query}': {e}"))?;

            if !resp.status().is_success() {
                if !out.is_empty() {
                    break;
                }
                return Err(format!(
                    "LinkedIn Guest HTTP {} untuk query '{query}'.",
                    resp.status().as_u16()
                ));
            }
            let html_text = resp.text().await.unwrap_or_default();
            let parsed_cards: Vec<(String, String, String, Option<String>, Option<String>)> = {
                let doc = Html::parse_document(&html_text);
                let mut cards = Vec::new();
                for li in doc.select(&card_sel) {
                    let title = li
                        .select(&title_sel)
                        .next()
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .unwrap_or_default();
                    if title.is_empty() {
                        continue;
                    }
                    let company = li
                        .select(&comp_sel)
                        .next()
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "Unknown Company".into());
                    let job_loc = li
                        .select(&loc_sel)
                        .next()
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| location.to_string());
                    let job_url = li
                        .select(&link_sel)
                        .next()
                        .and_then(|e| e.value().attr("href"))
                        .map(|s| s.split('?').next().unwrap_or(s).to_string());
                    let posted = li
                        .select(&date_sel)
                        .next()
                        .and_then(|e| e.value().attr("datetime"))
                        .map(|s| s.to_string());
                    cards.push((title, company, job_loc, job_url, posted));
                    if cards.len() >= limit.min(20) as usize {
                        break;
                    }
                }
                cards
            };

            if parsed_cards.is_empty() {
                break;
            }

            for (idx, (title, company, job_loc, job_url, posted)) in
                parsed_cards.into_iter().enumerate()
            {
                let job_numeric_id = job_url.as_deref().and_then(|u| {
                    u.trim_end_matches('/')
                        .rsplit('-')
                        .next()
                        .filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty())
                        .map(str::to_string)
                });

                // Fetch full guest description for top 10 listings per page to respect rate limits
                let mut desc = format!("{title} at {company} ({job_loc})");
                if idx < 10 {
                    if let Some(ref lid) = job_numeric_id {
                        tokio::time::sleep(Duration::from_millis(220)).await;
                        if let Ok(detail_resp) = self
                            .http
                            .get(format!(
                                "https://www.linkedin.com/jobs-guest/jobs/api/jobPosting/{lid}"
                            ))
                            .send()
                            .await
                        {
                            if detail_resp.status().is_success() {
                                if let Ok(detail_html) = detail_resp.text().await {
                                    let ddoc = Html::parse_document(&detail_html);
                                    if let Some(el) = ddoc.select(&desc_sel).next() {
                                        let cleaned = clean_html_to_text(&el.inner_html());
                                        if cleaned.len() >= 30 {
                                            desc = cleaned;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let external_id = job_numeric_id
                    .map(|id| format!("li-{id}"))
                    .unwrap_or_else(|| deterministic_external_id("linkedin", &company, &title, &job_loc));

                out.push(WorkerScrapedJob {
                    source: "linkedin".into(),
                    external_id,
                    url: job_url,
                    title,
                    company,
                    location: Some(job_loc),
                    raw_description: desc,
                    posted_at: posted,
                    salary_text: None,
                });
            }
        }
        Ok(out)
    }

    async fn scrape_kalibrr(
        &self,
        query: &str,
        location: &str,
        limit: u32,
    ) -> Result<Vec<WorkerScrapedJob>, String> {
        let resp = self
            .http
            .get("https://www.kalibrr.com/kjs/job_board/search")
            .header("Accept", "application/json")
            .query(&[
                ("limit", limit.clamp(1, 30).to_string()),
                ("offset", "0".into()),
                ("text", query.into()),
                ("country", "Indonesia".into()),
            ])
            .send()
            .await
            .map_err(|e| format!("Kalibrr koneksi gagal untuk '{query}': {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Kalibrr HTTP {} untuk query '{query}'.",
                resp.status().as_u16()
            ));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("Kalibrr JSON invalid: {e}"))?;
        let Some(jobs) = body.get("jobs").and_then(|v| v.as_array()) else {
            return Ok(vec![]);
        };

        let mut out = Vec::new();
        for item in jobs {
            let title = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if title.is_empty() {
                continue;
            }
            let company = item
                .get("company_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Company")
                .trim()
                .to_string();
            let loc = item
                .get("google_location")
                .and_then(|g| g.get("address_components"))
                .and_then(|a| a.get("city"))
                .and_then(|v| v.as_str())
                .unwrap_or(location)
                .to_string();
            let desc_html = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let qual_html = item
                .get("qualifications")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let combined = clean_html_to_text(&format!(
                "Deskripsi Pekerjaan:\n{desc_html}\nKualifikasi:\n{qual_html}"
            ));
            if combined.len() < 20 {
                continue;
            }
            let id_str = item
                .get("id")
                .map(|v| v.to_string().trim_matches('"').to_string())
                .unwrap_or_default();
            let slug = item.get("slug").and_then(|v| v.as_str()).unwrap_or("");
            let company_code = item
                .get("company")
                .and_then(|c| c.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("company");
            let url = if !id_str.is_empty() && !slug.is_empty() {
                Some(format!(
                    "https://www.kalibrr.com/c/{company_code}/jobs/{id_str}/{slug}"
                ))
            } else {
                None
            };
            let min_sal = item.get("base_salary").and_then(|v| v.as_f64());
            let max_sal = item.get("maximum_salary").and_then(|v| v.as_f64());
            let salary_text = match (min_sal, max_sal) {
                (Some(lo), Some(hi)) if lo > 0.0 && hi > 0.0 => {
                    Some(format!("IDR {:.0} - {:.0}", lo, hi))
                }
                (Some(lo), _) if lo > 0.0 => Some(format!("IDR {:.0}", lo)),
                _ => None,
            };

            out.push(WorkerScrapedJob {
                source: "kalibrr".into(),
                external_id: if !id_str.is_empty() {
                    format!("kb-{id_str}")
                } else {
                    deterministic_external_id("kalibrr", &company, &title, &loc)
                },
                url,
                title,
                company,
                location: Some(loc),
                raw_description: combined,
                posted_at: item
                    .get("activation_date")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                salary_text,
            });
        }
        Ok(out)
    }

    async fn scrape_dealls(
        &self,
        query: &str,
        location: &str,
        limit: u32,
    ) -> Result<Vec<WorkerScrapedJob>, String> {
        let resp = self
            .http
            .get("https://api.sejutacita.id/v1/explore-job/job")
            .header("Accept", "application/json")
            .query(&[
                ("page", "1".into()),
                ("limit", limit.clamp(1, 30).to_string()),
                ("search", query.into()),
                ("published", "true".into()),
            ])
            .send()
            .await
            .map_err(|e| format!("Dealls koneksi gagal untuk '{query}': {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Dealls HTTP {} untuk query '{query}'.",
                resp.status().as_u16()
            ));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("Dealls JSON invalid: {e}"))?;
        let Some(docs) = body
            .get("data")
            .and_then(|d| d.get("docs"))
            .and_then(|v| v.as_array())
        else {
            return Ok(vec![]);
        };

        let mut out = Vec::new();
        for item in docs {
            let title = item
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if title.is_empty() {
                continue;
            }
            let company = item
                .get("company")
                .and_then(|c| c.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Company")
                .trim()
                .to_string();
            let loc = item
                .get("city")
                .and_then(|c| c.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or(location)
                .to_string();
            let skills_txt = item
                .get("skills")
                .and_then(|s| s.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.get("name").and_then(|n| n.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let desc = clean_html_to_text(&format!(
                "Kualifikasi:\n{}\nSkills: {skills_txt}",
                item.get("requirements")
                    .and_then(|v| v.as_str())
                    .or_else(|| item.get("description").and_then(|v| v.as_str()))
                    .unwrap_or(&title)
            ));
            let slug = item.get("slug").and_then(|v| v.as_str()).unwrap_or("");
            let id_str = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let url = if !slug.is_empty() {
                Some(format!("https://dealls.com/loker/{slug}"))
            } else {
                None
            };
            let min_s = item.get("salaryRange").and_then(|s| s.get("start")).and_then(|v| v.as_i64());
            let max_s = item.get("salaryRange").and_then(|s| s.get("end")).and_then(|v| v.as_i64());
            let salary_text = match (min_s, max_s) {
                (Some(lo), Some(hi)) if lo > 0 && hi > 0 => Some(format!("IDR {lo} - {hi}")),
                _ => None,
            };

            out.push(WorkerScrapedJob {
                source: "dealls".into(),
                external_id: if !id_str.is_empty() {
                    format!("dl-{id_str}")
                } else {
                    deterministic_external_id("dealls", &company, &title, &loc)
                },
                url,
                title,
                company,
                location: Some(loc),
                raw_description: desc,
                posted_at: item
                    .get("publishedAt")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                salary_text,
            });
        }
        Ok(out)
    }

    async fn scrape_indeed_rss(
        &self,
        query: &str,
        location: &str,
        limit: u32,
    ) -> Result<Vec<WorkerScrapedJob>, String> {
        let resp = self
            .http
            .get("https://id.indeed.com/rss")
            .query(&[("q", query), ("l", location)])
            .send()
            .await
            .map_err(|e| format!("Indeed RSS koneksi gagal untuk '{query}': {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Indeed membatasi akses langsung (HTTP {}); gunakan JobStreet/LinkedIn/Kalibrr/Dealls atau Python Worker opsional.",
                resp.status().as_u16()
            ));
        }

        let xml = resp.text().await.unwrap_or_default();
        let doc = Html::parse_fragment(&xml);
        let item_sel = Selector::parse("item").unwrap();
        let title_sel = Selector::parse("title").unwrap();
        let link_sel = Selector::parse("link").unwrap();
        let desc_sel = Selector::parse("description").unwrap();
        let source_sel = Selector::parse("source").unwrap();

        let mut out = Vec::new();
        for item in doc.select(&item_sel).take(limit.min(30) as usize) {
            let title = item
                .select(&title_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            if title.is_empty() {
                continue;
            }
            let company = item
                .select(&source_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Unknown Company".into());
            let url = item
                .select(&link_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| s.starts_with("http"));
            let desc = item
                .select(&desc_sel)
                .next()
                .map(|e| clean_html_to_text(&e.text().collect::<String>()))
                .unwrap_or_default();
            if desc.len() < 20 {
                continue;
            }
            let external_id = deterministic_external_id("indeed", &company, &title, location);
            out.push(WorkerScrapedJob {
                source: "indeed".into(),
                external_id,
                url,
                title,
                company,
                location: Some(location.to_string()),
                raw_description: desc,
                posted_at: None,
                salary_text: None,
            });
        }
        Ok(out)
    }
}

#[async_trait]
impl WorkerClient for NativeScraperClient {
    async fn check_health(&self, _base_url: &str) -> Result<WorkerHealth, AppError> {
        Ok(WorkerHealth {
            status: "ok".into(),
            version: "native-rust-3.0".into(),
            available_sources: vec![
                "jobstreet".into(),
                "linkedin".into(),
                "kalibrr".into(),
                "dealls".into(),
                "indeed".into(),
            ],
        })
    }

    async fn scrape(
        &self,
        _base_url: &str,
        request: &WorkerScrapeRequest,
    ) -> Result<WorkerScrapeResponse, AppError> {
        let started = Instant::now();
        let mut all_jobs = Vec::new();
        let mut warnings = Vec::new();

        for query in &request.queries {
            for source in &request.sources {
                let res = match source.as_str() {
                    "jobstreet" => {
                        self.scrape_jobstreet(
                            query,
                            &request.location,
                            request.limit_per_query,
                            request.max_pages,
                            request.posted_within_days,
                            &request.work_mode,
                        )
                        .await
                    }
                    "linkedin" => {
                        self.scrape_linkedin_guest(
                            query,
                            &request.location,
                            request.limit_per_query,
                            request.max_pages,
                            request.posted_within_days,
                            &request.work_mode,
                        )
                        .await
                    }
                    "kalibrr" => {
                        self.scrape_kalibrr(query, &request.location, request.limit_per_query)
                            .await
                    }
                    "dealls" => {
                        self.scrape_dealls(query, &request.location, request.limit_per_query)
                            .await
                    }
                    "indeed" => {
                        self.scrape_indeed_rss(query, &request.location, request.limit_per_query)
                            .await
                    }
                    other => Err(format!(
                        "Portal '{other}' hanya tersedia melalui Python Worker opsional."
                    )),
                };

                match res {
                    Ok(mut found) => all_jobs.append(&mut found),
                    Err(msg) => warnings.push(ScrapeWarningItem {
                        source: source.clone(),
                        code: "NATIVE_SCRAPER_WARN".into(),
                        message: msg,
                    }),
                }
            }
        }

        let total_fetched = all_jobs.len() as u32;
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::with_capacity(all_jobs.len());
        for job in all_jobs {
            if seen.insert((job.source.clone(), job.external_id.clone())) {
                deduped.push(job);
            }
        }
        let deduplicated_in_batch = total_fetched.saturating_sub(deduped.len() as u32);

        Ok(WorkerScrapeResponse {
            jobs: deduped,
            stats: WorkerScrapeStats {
                total_fetched,
                deduplicated_in_batch,
                duration_ms: started.elapsed().as_millis() as u64,
            },
            warnings,
        })
    }
}
