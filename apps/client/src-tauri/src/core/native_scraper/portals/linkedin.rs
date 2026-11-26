use std::sync::Arc;
use std::time::Duration;

use scraper::{Html, Selector};
use tokio::task::JoinSet;

use crate::core::native_scraper::client::{
    clean_html_to_text, deterministic_external_id, HttpFetchError, ResilientHttpClient,
};
use crate::core::native_scraper::portals::compose_structured_description;
use crate::core::worker::WorkerScrapedJob;

const LINKEDIN_DETAIL_CAP: usize = 12;

pub fn parse_linkedin_detail_html(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let desc_sel = Selector::parse("div.show-more-less-html__markup").ok()?;
    let body = doc
        .select(&desc_sel)
        .next()
        .map(|el| clean_html_to_text(&el.inner_html()))
        .unwrap_or_default();

    let mut criteria_lines = Vec::new();
    if let (Ok(item_sel), Ok(sub_sel), Ok(val_sel)) = (
        Selector::parse("li.description__job-criteria-item"),
        Selector::parse("h3.description__job-criteria-subheader"),
        Selector::parse("span.description__job-criteria-text"),
    ) {
        for item in doc.select(&item_sel) {
            let k = item
                .select(&sub_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            let v = item
                .select(&val_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            if !k.is_empty() && !v.is_empty() {
                criteria_lines.push(format!("{k}: {v}"));
            }
        }
    }

    let composed = compose_structured_description(&[
        ("Deskripsi Pekerjaan", body),
        ("Informasi Peran", criteria_lines.join("\n")),
    ]);
    if composed.chars().count() >= 40 {
        Some(composed)
    } else {
        None
    }
}

pub async fn fetch_linkedin_detail(
    http: &ResilientHttpClient,
    id_or_url: &str,
) -> Result<Option<String>, HttpFetchError> {
    let numeric_id = if id_or_url.starts_with("http") {
        id_or_url
            .trim_end_matches('/')
            .rsplit('-')
            .next()
            .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(id_or_url)
    } else {
        id_or_url.strip_prefix("li-").unwrap_or(id_or_url)
    };
    if !numeric_id.chars().all(|c| c.is_ascii_digit()) || numeric_id.is_empty() {
        return Ok(None);
    }
    let url = format!("https://www.linkedin.com/jobs-guest/jobs/api/jobPosting/{numeric_id}");
    let html = http
        .get_text(
            "www.linkedin.com",
            &url,
            &[("Accept", "text/html,application/xhtml+xml;q=0.9,*/*;q=0.8")],
            &[],
        )
        .await?;
    Ok(parse_linkedin_detail_html(&html))
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
    let pages = max_pages.clamp(1, 3);
    let mut out = Vec::new();
    let card_sel = Selector::parse("li").unwrap();
    let title_sel = Selector::parse("h3.base-search-card__title").unwrap();
    let comp_sel = Selector::parse("h4.base-search-card__subtitle").unwrap();
    let loc_sel = Selector::parse("span.job-search-card__location").unwrap();
    let link_sel = Selector::parse("a.base-card__full-link").unwrap();
    let date_sel = Selector::parse("time").unwrap();

    let mut detail_targets: Vec<(usize, String)> = Vec::new();

    for page in 0..pages {
        if page > 0 {
            tokio::time::sleep(Duration::from_millis(450)).await;
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

        let html_text = match http
            .get_text(
                "www.linkedin.com",
                "https://www.linkedin.com/jobs-guest/jobs/api/seeMoreJobPostings/search",
                &[("Accept", "text/html,application/xhtml+xml;q=0.9,*/*;q=0.8")],
                &params,
            )
            .await
        {
            Ok(h) => h,
            Err(err) => {
                if !out.is_empty() {
                    break;
                }
                return Err(err);
            }
        };

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

        for (title, company, job_loc, job_url, posted) in parsed_cards {
            let job_numeric_id = job_url.as_deref().and_then(|u| {
                u.trim_end_matches('/')
                    .rsplit('-')
                    .next()
                    .filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty())
                    .map(str::to_string)
            });

            let external_id = job_numeric_id
                .as_ref()
                .map(|id| format!("li-{id}"))
                .unwrap_or_else(|| deterministic_external_id("linkedin", &company, &title, &job_loc));

            let idx = out.len();
            if let Some(lid) = job_numeric_id {
                if detail_targets.len() < LINKEDIN_DETAIL_CAP {
                    detail_targets.push((idx, lid));
                }
            }

            out.push(WorkerScrapedJob {
                source: "linkedin".into(),
                external_id,
                url: job_url,
                title: title.clone(),
                company: company.clone(),
                location: Some(job_loc.clone()),
                raw_description: format!("{title} at {company} ({job_loc})"),
                posted_at: posted,
                salary_text: None,
            });
        }
    }

    // Polite bounded concurrent pool (3 at a time) for LinkedIn Guest job detail pages
    for chunk in detail_targets.chunks(3) {
        let mut set = JoinSet::new();
        for (idx, lid) in chunk {
            let idx = *idx;
            let lid = lid.clone();
            let client = Arc::clone(&http);
            set.spawn(async move {
                let res = fetch_linkedin_detail(&client, &lid).await.ok().flatten();
                (idx, res)
            });
        }
        while let Some(Ok((idx, Some(full_desc)))) = set.join_next().await {
            if let Some(job) = out.get_mut(idx) {
                job.raw_description = full_desc;
            }
        }
        tokio::time::sleep(Duration::from_millis(160)).await;
    }

    Ok(out)
}
