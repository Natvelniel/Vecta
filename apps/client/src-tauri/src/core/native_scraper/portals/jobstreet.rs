use std::sync::Arc;
use std::time::Duration;

use scraper::{Html, Selector};
use serde_json::Value;
use tokio::task::JoinSet;

use crate::core::native_scraper::client::{
    clean_html_to_text, deterministic_external_id, HttpFetchError, ResilientHttpClient,
};
use crate::core::native_scraper::portals::compose_structured_description;
use crate::core::worker::WorkerScrapedJob;

const DETAIL_ENRICH_CAP: usize = 12;

pub fn parse_jobstreet_detail_html(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    if let Ok(sel) = Selector::parse(r#"[data-automation="jobAdDetails"]"#) {
        if let Some(el) = doc.select(&sel).next() {
            let text = clean_html_to_text(&el.inner_html());
            if text.chars().count() >= 80 {
                return Some(text);
            }
        }
    }

    if let Ok(ld_sel) = Selector::parse(r#"script[type="application/ld+json"]"#) {
        for script in doc.select(&ld_sel) {
            let raw = script.text().collect::<String>();
            if let Ok(val) = serde_json::from_str::<Value>(&raw) {
                if let Some(desc) = val.get("description").and_then(|v| v.as_str()) {
                    let text = clean_html_to_text(desc);
                    if text.chars().count() >= 80 {
                        return Some(text);
                    }
                }
            }
        }
    }
    None
}

pub async fn fetch_jobstreet_detail(
    http: &ResilientHttpClient,
    id_or_url: &str,
) -> Result<Option<String>, HttpFetchError> {
    let target_url = if id_or_url.starts_with("http") {
        id_or_url.to_string()
    } else {
        let clean_id = id_or_url.strip_prefix("js-").unwrap_or(id_or_url);
        format!("https://id.jobstreet.com/id/job/{clean_id}")
    };
    let html = http
        .get_text(
            "id.jobstreet.com",
            &target_url,
            &[
                ("Accept", "text/html,application/xhtml+xml;q=0.9,*/*;q=0.8"),
                ("Referer", "https://id.jobstreet.com/"),
            ],
            &[],
        )
        .await?;
    Ok(parse_jobstreet_detail_html(&html))
}

pub async fn scrape(
    http: Arc<ResilientHttpClient>,
    query: &str,
    location: &str,
    limit: u32,
    max_pages: u32,
    posted_within_days: Option<u32>,
    work_mode: &str,
) -> Result<Vec<WorkerScrapedJob>, HttpFetchError> {
    let pages = max_pages.clamp(1, 5);
    let page_size = limit.clamp(1, 50);
    let mut out = Vec::new();
    let mut detail_targets: Vec<(usize, String, String)> = Vec::new();

    for page in 1..=pages {
        if page > 1 {
            tokio::time::sleep(Duration::from_millis(260)).await;
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

        let body = match http
            .get_json(
                "id.jobstreet.com",
                "https://id.jobstreet.com/api/jobsearch/v5/search",
                &[
                    ("Accept", "application/json"),
                    ("Referer", "https://id.jobstreet.com/"),
                ],
                &params,
            )
            .await
        {
            Ok(v) => v,
            Err(err) => {
                if !out.is_empty() {
                    break;
                }
                return Err(err);
            }
        };

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

            let work_types = item
                .get("workTypes")
                .and_then(|w| w.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let arrangement = item
                .get("workArrangements")
                .and_then(|w| w.get("data"))
                .and_then(|d| d.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| a.get("label").and_then(|l| l.get("text")).and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let category = item
                .get("classifications")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .map(|first| {
                    let cls = first
                        .get("classification")
                        .and_then(|c| c.get("description"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let sub = first
                        .get("subclassification")
                        .and_then(|c| c.get("description"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    [cls, sub]
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" - ")
                })
                .unwrap_or_default();

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

            let meta_line = [
                if !category.is_empty() {
                    format!("Bidang: {category}")
                } else {
                    String::new()
                },
                if !work_types.is_empty() {
                    format!("Tipe Kerja: {work_types}")
                } else {
                    String::new()
                },
                if !arrangement.is_empty() {
                    format!("Pengaturan Lokasi: {arrangement}")
                } else {
                    String::new()
                },
            ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

            let summary_desc = compose_structured_description(&[
                ("Ringkasan Peran", clean_html_to_text(&format!("{teaser}\n{bullets}"))),
                ("Informasi Tambahan", meta_line),
            ]);
            if summary_desc.chars().count() < 15 {
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

            let idx = out.len();
            if !raw_id.is_empty() && detail_targets.len() < DETAIL_ENRICH_CAP {
                detail_targets.push((idx, raw_id, meta_line_backup(&category, &work_types, &arrangement)));
            }

            out.push(WorkerScrapedJob {
                source: "jobstreet".into(),
                external_id,
                url,
                title,
                company,
                location: Some(loc),
                raw_description: summary_desc,
                posted_at,
                salary_text,
            });
        }
    }

    // Fetch full SSR job descriptions in bounded concurrent chunks (4 at a time)
    for chunk in detail_targets.chunks(4) {
        let mut set = JoinSet::new();
        for (idx, raw_id, meta_block) in chunk {
            let idx = *idx;
            let raw_id = raw_id.clone();
            let meta_block = meta_block.clone();
            let client = Arc::clone(&http);
            set.spawn(async move {
                let res = fetch_jobstreet_detail(&client, &raw_id).await.ok().flatten();
                (idx, res, meta_block)
            });
        }
        while let Some(Ok((idx, Some(full_body), meta_block))) = set.join_next().await {
            if let Some(job) = out.get_mut(idx) {
                let enriched = compose_structured_description(&[
                    ("Deskripsi Pekerjaan", full_body),
                    ("Informasi Peran", meta_block),
                ]);
                if enriched.chars().count() > job.raw_description.chars().count() {
                    job.raw_description = enriched;
                }
            }
        }
    }

    Ok(out)
}

fn meta_line_backup(category: &str, work_types: &str, arrangement: &str) -> String {
    [
        (!category.is_empty()).then(|| format!("Bidang: {category}")),
        (!work_types.is_empty()).then(|| format!("Tipe Kerja: {work_types}")),
        (!arrangement.is_empty()).then(|| format!("Pengaturan Lokasi: {arrangement}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_jobstreet_detail_extracts_job_ad_details_div() {
        let html = r#"<html><body><div data-automation="jobAdDetails"><div><p>Kami mencari Backend Engineer.</p><ul><li>Menguasai Rust dan SQLite</li><li>Berpengalaman API REST</li></ul></div></div></body></html>"#;
        let parsed = parse_jobstreet_detail_html(html).expect("must extract jobAdDetails");
        assert!(parsed.contains("Kami mencari Backend Engineer."));
        assert!(parsed.contains("- Menguasai Rust dan SQLite"));
    }
}
