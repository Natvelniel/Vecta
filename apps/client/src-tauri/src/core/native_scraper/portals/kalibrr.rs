use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::core::native_scraper::client::{
    clean_html_to_text, deterministic_external_id, HttpFetchError, ResilientHttpClient,
};
use crate::core::native_scraper::portals::compose_structured_description;
use crate::core::worker::WorkerScrapedJob;

pub fn parse_kalibrr_response(body: &Value, fallback_location: &str) -> Vec<WorkerScrapedJob> {
    if body.get("from_alternative").and_then(|v| v.as_bool()) == Some(true) {
        return vec![];
    }
    let Some(jobs) = body.get("jobs").and_then(|v| v.as_array()) else {
        return vec![];
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
            .unwrap_or(fallback_location)
            .to_string();

        let desc_html = item.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let qual_html = item
            .get("qualifications")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let perks_text = item
            .get("perks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| p.as_str())
                    .map(|s| format!("- {s}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let company_desc = item
            .get("company")
            .and_then(|c| c.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut info_lines = Vec::new();
        if item.get("is_work_from_home").and_then(|v| v.as_bool()) == Some(true) {
            info_lines.push("Mode Kerja: Remote / WFH".to_string());
        } else if item.get("is_hybrid").and_then(|v| v.as_bool()) == Some(true) {
            info_lines.push("Mode Kerja: Hybrid".to_string());
        }
        if item.get("is_open_to_fresh_grads").and_then(|v| v.as_bool()) == Some(true) {
            info_lines.push("Terbuka untuk Fresh Graduate".to_string());
        }
        if let Some(edu) = item.get("education_level").and_then(|v| v.as_str()) {
            if !edu.trim().is_empty() {
                info_lines.push(format!("Pendidikan: {edu}"));
            }
        }

        let combined = compose_structured_description(&[
            ("Deskripsi Pekerjaan", clean_html_to_text(desc_html)),
            ("Kualifikasi", clean_html_to_text(qual_html)),
            (
                "Benefit & Info Tambahan",
                [perks_text, info_lines.join("\n")]
                    .into_iter()
                    .filter(|s| !s.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            ("Tentang Perusahaan", clean_html_to_text(company_desc)),
        ]);
        if combined.chars().count() < 20 {
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
            (Some(lo), Some(hi)) if lo > 0.0 && hi > 0.0 => Some(format!("IDR {:.0} - {:.0}", lo, hi)),
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
    out
}

pub fn filter_query_relevant_jobs(query: &str, jobs: Vec<WorkerScrapedJob>) -> Vec<WorkerScrapedJob> {
    const GENERIC_TOKENS: &[&str] = &[
        "full", "time", "part", "staff", "staf", "senior", "junior", "intern", "magang",
        "specialist", "officer", "associate", "manager", "lead", "head", "kerja", "lowongan",
        "indonesia", "remote", "hybrid", "onsite",
    ];
    let q_lower = query.trim().to_lowercase();
    let all_tokens: Vec<&str> = q_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .collect();
    if all_tokens.len() < 2 {
        return jobs;
    }
    let core_tokens: Vec<&str> = all_tokens
        .iter()
        .copied()
        .filter(|t| !GENERIC_TOKENS.contains(t))
        .collect();
    if core_tokens.is_empty() {
        return jobs;
    }

    jobs.into_iter()
        .filter(|job| {
            let haystack = format!(
                "{} {}",
                job.title.to_lowercase(),
                job.raw_description.to_lowercase()
            );
            core_tokens.iter().any(|tok| {
                haystack
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|w| w == *tok)
            })
        })
        .collect()
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
        let offset = (page - 1) * page_size;
        let params = [
            ("limit", page_size.to_string()),
            ("offset", offset.to_string()),
            ("text", query.into()),
            ("country", "Indonesia".into()),
        ];

        let primary = http
            .get_json(
                "www.kalibrr.com",
                "https://www.kalibrr.com/kjs/job_board/search",
                &[("Accept", "application/json")],
                &params,
            )
            .await;

        let body = match primary {
            Ok(v) => v,
            Err(first_err) if first_err.is_transport() => {
                match http
                    .get_json(
                        "www.kalibrr.id",
                        "https://www.kalibrr.id/kjs/job_board/search",
                        &[("Accept", "application/json")],
                        &params,
                    )
                    .await
                {
                    Ok(v) => v,
                    Err(second_err) => {
                        if !out.is_empty() {
                            break;
                        }
                        return Err(second_err);
                    }
                }
            }
            Err(err) => {
                if !out.is_empty() {
                    break;
                }
                return Err(err);
            }
        };

        let page_jobs = parse_kalibrr_response(&body, location);
        let raw_count = page_jobs.len();
        out.extend(filter_query_relevant_jobs(query, page_jobs));
        if raw_count < page_size as usize {
            break;
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_rejects_full_time_tutor_noise_for_full_stack_developer() {
        let jobs = vec![
            WorkerScrapedJob {
                source: "kalibrr".into(),
                external_id: "kb-1".into(),
                url: None,
                title: "Full Time Pengajar UTBK Numerasi".into(),
                company: "Ruangguru".into(),
                location: Some("Pontianak".into()),
                raw_description: "Mengajar siswa UTBK Matematika dan skolastik.".into(),
                posted_at: None,
                salary_text: None,
            },
            WorkerScrapedJob {
                source: "kalibrr".into(),
                external_id: "kb-2".into(),
                url: None,
                title: "Full Stack Web Developer".into(),
                company: "PT Teknologi".into(),
                location: Some("Jakarta".into()),
                raw_description: "Membangun aplikasi React dan Laravel.".into(),
                posted_at: None,
                salary_text: None,
            },
        ];

        let filtered = filter_query_relevant_jobs("Full Stack Developer", jobs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].external_id, "kb-2");
    }
}
