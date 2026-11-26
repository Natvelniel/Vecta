use std::sync::Arc;

use serde_json::Value;

use crate::core::native_scraper::client::{
    clean_html_to_text, deterministic_external_id, HttpFetchError, ResilientHttpClient,
};
use crate::core::native_scraper::portals::compose_structured_description;
use crate::core::worker::WorkerScrapedJob;

pub fn parse_remotive_response(body: &Value) -> Vec<WorkerScrapedJob> {
    let Some(jobs) = body.get("jobs").and_then(|v| v.as_array()) else {
        return vec![];
    };

    let mut out = Vec::new();
    for item in jobs {
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
            .unwrap_or("Remote Company")
            .trim()
            .to_string();
        let req_loc = item
            .get("candidate_required_location")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Worldwide (Remote)")
            .to_string();
        let category = item
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let job_type = item
            .get("job_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let tags = item
            .get("tags")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| format!("- {s}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let raw_html = item.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let desc = compose_structured_description(&[
            (
                "Informasi Peran",
                [
                    (!category.is_empty()).then(|| format!("Kategori: {category}")),
                    (!job_type.is_empty()).then(|| format!("Tipe Kerja: {job_type}")),
                    Some(format!("Wilayah Kandidat: {req_loc}")),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("\n"),
            ),
            ("Deskripsi Pekerjaan", clean_html_to_text(raw_html)),
            ("Keahlian & Tags", tags),
        ]);
        if desc.chars().count() < 30 {
            continue;
        }
        let id_str = item
            .get("id")
            .map(|v| v.to_string().trim_matches('"').to_string())
            .unwrap_or_default();
        let salary_text = item
            .get("salary")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        out.push(WorkerScrapedJob {
            source: "remotive".into(),
            external_id: if !id_str.is_empty() {
                format!("rm-{id_str}")
            } else {
                deterministic_external_id("remotive", &company, &title, &req_loc)
            },
            url: item
                .get("url")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            title,
            company,
            location: Some(format!("Remote ({req_loc})")),
            raw_description: desc,
            posted_at: item
                .get("publication_date")
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
    limit: u32,
) -> Result<Vec<WorkerScrapedJob>, HttpFetchError> {
    let body = http
        .get_json(
            "remotive.com",
            "https://remotive.com/api/remote-jobs",
            &[("Accept", "application/json")],
            &[
                ("search", query.into()),
                ("limit", limit.clamp(1, 25).to_string()),
            ],
        )
        .await?;
    Ok(parse_remotive_response(&body))
}
