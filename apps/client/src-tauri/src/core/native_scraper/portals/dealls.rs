use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::core::native_scraper::client::{
    clean_html_to_text, deterministic_external_id, HttpFetchError, ResilientHttpClient,
};
use crate::core::native_scraper::portals::compose_structured_description;
use crate::core::worker::WorkerScrapedJob;

pub fn parse_dealls_response(body: &Value, fallback_location: &str) -> Vec<WorkerScrapedJob> {
    let Some(docs) = body
        .get("data")
        .and_then(|d| d.get("docs"))
        .and_then(|v| v.as_array())
    else {
        return vec![];
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
        let sector = item
            .get("company")
            .and_then(|c| c.get("sector"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let loc = item
            .get("city")
            .and_then(|c| c.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or(fallback_location)
            .to_string();
        let skills_txt = item
            .get("skills")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.get("name").and_then(|n| n.as_str()))
                    .map(|s| format!("- {s}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let workplace = item
            .get("workplaceType")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let emp_types = item
            .get("employmentTypes")
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();

        let mut pref_lines = Vec::new();
        if !workplace.is_empty() {
            pref_lines.push(format!("Mode Kerja: {workplace}"));
        }
        if !emp_types.is_empty() {
            pref_lines.push(format!("Tipe Kontrak: {emp_types}"));
        }
        if !sector.is_empty() {
            pref_lines.push(format!("Sektor Industri: {sector}"));
        }
        if let Some(cp) = item.get("candidatePreference") {
            if let Some(major) = cp.get("major").and_then(|v| v.as_str()) {
                if !major.trim().is_empty() {
                    pref_lines.push(format!("Jurusan: {major}"));
                }
            }
            if let Some(gpa) = cp.get("minimumGpa").and_then(|v| v.as_f64()) {
                if gpa > 0.0 {
                    pref_lines.push(format!("IPK Minimum: {gpa:.2}"));
                }
            }
            if let Some(age) = cp.get("age") {
                if age.get("restriction").and_then(|v| v.as_bool()) == Some(true) {
                    let min_a = age.get("min").and_then(|v| v.as_i64());
                    let max_a = age.get("max").and_then(|v| v.as_i64());
                    if let (Some(a), Some(b)) = (min_a, max_a) {
                        pref_lines.push(format!("Rentang Usia: {a}-{b} tahun"));
                    }
                }
            }
        }

        let raw_req = item
            .get("requirements")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("description").and_then(|v| v.as_str()))
            .unwrap_or("");

        let desc = compose_structured_description(&[
            ("Ringkasan Peran", pref_lines.join("\n")),
            ("Kualifikasi", clean_html_to_text(raw_req)),
            ("Keahlian yang Dicari", skills_txt),
        ]);
        let final_desc = if desc.trim().is_empty() {
            format!("Lowongan {title} di {company} ({loc})")
        } else {
            desc
        };

        let slug = item.get("slug").and_then(|v| v.as_str()).unwrap_or("");
        let id_str = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let url = if !slug.is_empty() {
            Some(format!("https://dealls.com/loker/{slug}"))
        } else {
            None
        };
        let min_s = item
            .get("salaryRange")
            .and_then(|s| s.get("start"))
            .and_then(|v| v.as_i64());
        let max_s = item
            .get("salaryRange")
            .and_then(|s| s.get("end"))
            .and_then(|v| v.as_i64());
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
            raw_description: final_desc,
            posted_at: item
                .get("publishedAt")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            salary_text,
        });
    }
    out
}

pub async fn scrape(
    http: Arc<ResilientHttpClient>,
    query: &str,
    location: &str,
    limit: u32,
    max_pages: u32,
) -> Result<Vec<WorkerScrapedJob>, HttpFetchError> {
    let pages = max_pages.clamp(1, 3);
    let page_size = limit.clamp(1, 30);
    let mut out = Vec::new();

    for page in 1..=pages {
        if page > 1 {
            tokio::time::sleep(Duration::from_millis(240)).await;
        }
        let body = match http
            .get_json(
                "api.sejutacita.id",
                "https://api.sejutacita.id/v1/explore-job/job",
                &[("Accept", "application/json")],
                &[
                    ("page", page.to_string()),
                    ("limit", page_size.to_string()),
                    ("search", query.into()),
                    ("published", "true".into()),
                ],
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

        let page_jobs = parse_dealls_response(&body, location);
        let count = page_jobs.len();
        out.extend(page_jobs);
        if count < page_size as usize {
            break;
        }
    }

    Ok(out)
}
