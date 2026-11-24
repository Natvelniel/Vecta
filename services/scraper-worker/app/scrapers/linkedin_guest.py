import asyncio
import random
from typing import Any

from jobspy import scrape_jobs

from app.config import WorkerSettings
from app.pipeline.cleaner import clean_html_description, make_deterministic_external_id
from app.pipeline.schemas import ScrapedJob, SourceName
from app.scrapers.base import BaseScraper, ScraperError


def _safe_str(value: Any) -> str:
    if value is None:
        return ""
    text = str(value).strip()
    return "" if text.lower() in {"nan", "none", "null"} else text


class LinkedInGuestScraper(BaseScraper):
    """Public guest LinkedIn scraper powered by python-jobspy without user credentials."""

    source_name: SourceName = "linkedin"

    def __init__(self, settings: WorkerSettings) -> None:
        self._settings = settings
        self._has_fetched_once = False

    async def scrape(self, query: str, location: str, limit: int) -> list[ScrapedJob]:
        if self._has_fetched_once and self._settings.linkedin_jitter_max_sec > 0:
            delay = random.uniform(
                self._settings.linkedin_jitter_min_sec,
                self._settings.linkedin_jitter_max_sec,
            )
            await asyncio.sleep(delay)
        self._has_fetched_once = True

        try:
            df = await asyncio.to_thread(
                scrape_jobs,
                site_name=["linkedin"],
                search_term=query,
                location=location,
                results_wanted=limit,
                linkedin_fetch_description=True,
            )
        except Exception as exc:  # noqa: BLE001
            msg = str(exc)
            if "429" in msg or "rate" in msg.lower() or "too many" in msg.lower():
                raise ScraperError(
                    "RATE_LIMITED",
                    f"LinkedIn membatasi permintaan untuk query '{query}': {msg}",
                ) from exc
            raise ScraperError(
                "UPSTREAM_ERROR",
                f"Gagal mengambil data LinkedIn untuk query '{query}': {msg}",
            ) from exc

        if df is None or getattr(df, "empty", True):
            return []

        results: list[ScrapedJob] = []
        for record in df.to_dict(orient="records"):
            title = _safe_str(record.get("title"))
            company = _safe_str(record.get("company")) or "Unknown Company"
            job_location = _safe_str(record.get("location")) or location
            url = _safe_str(record.get("job_url")) or None
            raw_desc = clean_html_description(
                _safe_str(record.get("description")),
                max_chars=self._settings.max_description_chars,
            )
            if not title or not raw_desc:
                continue

            raw_id = _safe_str(record.get("id"))
            if raw_id:
                external_id = raw_id if raw_id.startswith("li-") else f"li-{raw_id}"
            else:
                external_id = make_deterministic_external_id(
                    self.source_name, company, title, job_location
                )

            results.append(
                ScrapedJob(
                    source=self.source_name,
                    external_id=external_id,
                    url=url,
                    title=title,
                    company=company,
                    location=job_location,
                    raw_description=raw_desc,
                )
            )
        return results
