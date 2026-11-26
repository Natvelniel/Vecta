use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::task::JoinSet;

use crate::core::native_scraper::client::{
    clean_html_to_text, deterministic_external_id, HttpFetchError, ResilientHttpClient,
};
use crate::core::native_scraper::portals::compose_structured_description;
use crate::core::worker::WorkerScrapedJob;

const KARIRHUB_DETAIL_CAP: usize = 12;

pub fn parse_karirhub_detail_json(body: &Value) -> Option<String> {
    let data = body.get("data").unwrap_or(body);
    let desc_html = data.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let qual_html = data
        .get("qualification")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let skills = data
        .get("skills")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.get("name").and_then(|n| n.as_str()).or_else(|| x.as_str()))
                .map(|s| format!("- {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    let mut meta = Vec::new();
    if let Some(exp) = data.get("min_year_experience").and_then(|v| v.as_i64()) {
        if exp > 0 {
            meta.push(format!("Pengalaman Minimum: {exp} tahun"));
        }
    }
    if let Some(quota) = data.get("quota").and_then(|v| v.as_i64()) {
        if quota > 0 {
            meta.push(format!("Kebutuhan Kuota: {quota} orang"));
        }
    }
    if let Some(ind) = data
        .get("employer")
        .and_then(|e| e.get("industry"))
        .and_then(|i| i.get("name"))
        .and_then(|v| v.as_str())
    {
        if !ind.trim().is_empty() {
            meta.push(format!("Industri: {ind}"));
        }
    }

    let composed = compose_structured_description(&[
        ("Deskripsi Pekerjaan", clean_html_to_text(desc_html)),
        ("Kualifikasi", clean_html_to_text(qual_html)),
        ("Keahlian Terkait", skills),
        ("Informasi Peran", meta.join("\n")),
    ]);
    if composed.chars().count() >= 40 {
        Some(composed)
    } else {
        None
    }
}

pub async fn fetch_karirhub_detail(
    http: &ResilientHttpClient,
    id: &str,
) -> Result<Option<String>, HttpFetchError> {
    let clean_id = id.strip_prefix("kh-").unwrap_or(id);
    if clean_id.is_empty() {
        return Ok(None);
    }
    let url = format!("https://api.kemnaker.go.id/karirhub/catalogue/v1/vacancies/{clean_id}");
    let body = http
        .get_json("api.kemnaker.go.id", &url, &[("Accept", "application/json")], &[])
        .await?;
    Ok(parse_karirhub_detail_json(&body))
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
    let mut detail_targets: Vec<(usize, String)> = Vec::new();

    for page in 1..=pages {
        if page > 1 {
            tokio::time::sleep(Duration::from_millis(240)).await;
        }
        let body = match http
            .get_json(
                "api.kemnaker.go.id",
                "https://api.kemnaker.go.id/karirhub/catalogue/v1/industrial-vacancies",
                &[("Accept", "application/json")],
                &[
                    ("page", page.to_string()),
                    ("limit", page_size.to_string()),
                    ("keyword", query.into()),
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

        let Some(items) = body.get("data").and_then(|v| v.as_array()) else {
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
                .get("company_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Perusahaan Terverifikasi Kemnaker")
                .trim()
                .to_string();
            let loc = item
                .get("city_name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(location)
                .to_string();
            let id_str = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            let industry = item
                .get("industry_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let func_name = item
                .get("job_function_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let job_type = item
                .get("job_type_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let skills_txt = item
                .get("skills")
                .and_then(|s| s.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.get("name").and_then(|n| n.as_str()).or_else(|| x.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            let summary = compose_structured_description(&[
                (
                    "Ringkasan Peran",
                    [
                        (!industry.is_empty()).then(|| format!("Industri: {industry}")),
                        (!func_name.is_empty()).then(|| format!("Fungsi: {func_name}")),
                        (!job_type.is_empty()).then(|| format!("Tipe Kerja: {job_type}")),
                        (!skills_txt.is_empty()).then(|| format!("Skills: {skills_txt}")),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join("\n"),
                ),
            ]);

            let min_sal = item
                .get("min_salary_amount")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let max_sal = item
                .get("max_salary_amount")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let salary_text = if min_sal > 0.0 && max_sal >= min_sal {
                Some(format!("IDR {:.0} - {:.0}", min_sal, max_sal))
            } else if min_sal > 0.0 {
                Some(format!("IDR {:.0}", min_sal))
            } else {
                None
            };

            let url = item
                .get("platform_link")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| s.starts_with("http"))
                .map(str::to_string)
                .or_else(|| {
                    (!id_str.is_empty()).then(|| {
                        format!("https://karirhub.kemnaker.go.id/lowongan-dalam-negeri/lowongan/{id_str}")
                    })
                });

            let external_id = if !id_str.is_empty() {
                format!("kh-{id_str}")
            } else {
                deterministic_external_id("karirhub", &company, &title, &loc)
            };

            let idx = out.len();
            if !id_str.is_empty() && detail_targets.len() < KARIRHUB_DETAIL_CAP {
                detail_targets.push((idx, id_str));
            }

            out.push(WorkerScrapedJob {
                source: "karirhub".into(),
                external_id,
                url,
                title: title.clone(),
                company: company.clone(),
                location: Some(loc),
                raw_description: if summary.is_empty() {
                    format!("Lowongan resmi Karirhub Kemnaker RI: {title} di {company}")
                } else {
                    summary
                },
                posted_at: item
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                salary_text,
            });
        }

        if items.len() < page_size as usize {
            break;
        }
    }

    for chunk in detail_targets.chunks(4) {
        let mut set = JoinSet::new();
        for (idx, kid) in chunk {
            let idx = *idx;
            let kid = kid.clone();
            let client = Arc::clone(&http);
            set.spawn(async move {
                let res = fetch_karirhub_detail(&client, &kid).await.ok().flatten();
                (idx, res)
            });
        }
        while let Some(Ok((idx, Some(full_desc)))) = set.join_next().await {
            if let Some(job) = out.get_mut(idx) {
                if full_desc.chars().count() > job.raw_description.chars().count() {
                    job.raw_description = full_desc;
                }
            }
        }
    }

    Ok(out)
}
