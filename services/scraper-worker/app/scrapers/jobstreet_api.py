import asyncio
from typing import Any

import httpx

from app.config import WorkerSettings
from app.pipeline.cleaner import (
    clean_html_description,
    make_deterministic_external_id,
    sanitize_external_url,
)
from app.pipeline.schemas import ScrapedJob, SourceName
from app.scrapers.base import BaseScraper, ScraperError

_JOBSTREET_SEARCH_URL = "https://id.jobstreet.com/api/jobsearch/v5/search"


class JobStreetApiScraper(BaseScraper):
    """Public JobStreet Indonesia (SEEK v5) multi-page JSON search API adapter."""

    source_name: SourceName = "jobstreet"

    def __init__(self, settings: WorkerSettings) -> None:
        self._settings = settings

    async def scrape(
        self,
        query: str,
        location: str,
        limit: int,
        *,
        max_pages: int = 1,
        posted_within_days: int | None = None,
        work_mode: str = "any",
    ) -> list[ScrapedJob]:
        headers = {
            "User-Agent": self._settings.user_agent,
            "Accept": "application/json",
            "Referer": "https://id.jobstreet.com/",
        }
        pages_to_fetch = max(1, min(max_pages, 5))
        jobs: list[ScrapedJob] = []

        async with httpx.AsyncClient(timeout=self._settings.http_timeout_sec) as client:
            for page in range(1, pages_to_fetch + 1):
                if page > 1:
                    await asyncio.sleep(0.35)

                params: dict[str, Any] = {
                    "siteKey": "ID-Main",
                    "keywords": query,
                    "where": location,
                    "page": page,
                    "pageSize": limit,
                    "locale": "id-ID",
                }
                if posted_within_days is not None:
                    params["daterange"] = posted_within_days
                if work_mode == "remote":
                    params["workarrangement"] = 3
                elif work_mode == "hybrid":
                    params["workarrangement"] = 2
                elif work_mode == "onsite":
                    params["workarrangement"] = 1

                try:
                    response = await client.get(
                        _JOBSTREET_SEARCH_URL, params=params, headers=headers
                    )
                except httpx.TimeoutException as exc:
                    if jobs:
                        break
                    raise ScraperError(
                        "TIMEOUT", f"JobStreet timeout saat mengambil query '{query}'."
                    ) from exc
                except httpx.HTTPError as exc:
                    if jobs:
                        break
                    raise ScraperError(
                        "UPSTREAM_ERROR", f"Koneksi JobStreet gagal untuk query '{query}': {exc}"
                    ) from exc

                if response.status_code == 429:
                    if jobs:
                        break
                    raise ScraperError(
                        "RATE_LIMITED",
                        f"JobStreet mengembalikan HTTP 429 pada query '{query}'; hasil parsial tetap dikembalikan.",
                    )
                if response.status_code >= 400:
                    if jobs:
                        break
                    raise ScraperError(
                        "UPSTREAM_ERROR",
                        f"JobStreet mengembalikan HTTP {response.status_code} untuk query '{query}'.",
                    )

                try:
                    body: dict[str, Any] = response.json()
                    items = body.get("data") or []
                except Exception as exc:  # noqa: BLE001
                    if jobs:
                        break
                    raise ScraperError(
                        "PARSE_ERROR", f"Respons JSON JobStreet tidak dapat diparsing: {exc}"
                    ) from exc

                if not items:
                    break

                for item in items[:limit]:
                    title = str(item.get("title") or "").strip()
                    advertiser = item.get("advertiser") or {}
                    company = str(
                        advertiser.get("description")
                        or item.get("companyName")
                        or "Unknown Company"
                    ).strip()
                    locations = item.get("locations") or []
                    if locations and isinstance(locations, list):
                        job_location = str(locations[0].get("label") or location).strip()
                    else:
                        job_location = str(item.get("location") or location).strip()

                    teaser = str(item.get("teaser") or "")
                    bullets = item.get("bulletPoints") or []
                    bullet_html = "".join(f"<li>{b}</li>" for b in bullets if b)
                    combined_html = f"<p>{teaser}</p><ul>{bullet_html}</ul>"
                    raw_desc = clean_html_description(
                        combined_html, max_chars=self._settings.max_description_chars
                    )
                    if not title or not raw_desc:
                        continue

                    raw_id = str(item.get("id") or "").strip()
                    external_id = (
                        f"js-{raw_id}"
                        if raw_id
                        else make_deterministic_external_id(
                            self.source_name, company, title, job_location
                        )
                    )
                    url = sanitize_external_url(
                        f"https://id.jobstreet.com/id/job/{raw_id}" if raw_id else None
                    )
                    salary_raw = str(item.get("salaryLabel") or item.get("salary") or "").strip()
                    posted_raw = str(item.get("listingDate") or "").strip()

                    jobs.append(
                        ScrapedJob(
                            source=self.source_name,
                            external_id=external_id,
                            url=url,
                            title=title,
                            company=company,
                            location=job_location,
                            raw_description=raw_desc,
                            posted_at=posted_raw or None,
                            salary_text=salary_raw or None,
                        )
                    )

                if len(items) < limit:
                    break
        return jobs
