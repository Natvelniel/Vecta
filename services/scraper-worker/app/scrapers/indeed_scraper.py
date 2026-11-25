import asyncio
from typing import Any

from jobspy import scrape_jobs

from app.config import WorkerSettings
from app.pipeline.cleaner import (
    clean_html_description,
    make_deterministic_external_id,
    sanitize_external_url,
)
from app.pipeline.schemas import ScrapedJob, SourceName
from app.scrapers.base import BaseScraper, ScraperError


def _safe_str(value: Any) -> str:
    if value is None:
        return ""
    text = str(value).strip()
    return "" if text.lower() in {"nan", "none", "null", "nat"} else text


def _format_salary(record: dict[str, Any]) -> str | None:
    min_amt = _safe_str(record.get("min_amount"))
    max_amt = _safe_str(record.get("max_amount"))
    currency = _safe_str(record.get("currency")) or "IDR"
    interval = _safe_str(record.get("interval"))
    if not min_amt and not max_amt:
        return None
    if min_amt and max_amt:
        base = f"{currency} {min_amt} - {max_amt}"
    else:
        base = f"{currency} {min_amt or max_amt}"
    return f"{base}/{interval}" if interval else base


class IndeedScraper(BaseScraper):
    """Public Indeed Indonesia job search adapter powered by python-jobspy."""

    source_name: SourceName = "indeed"

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
        wanted = min(max(1, limit) * max(1, min(max_pages, 5)), 100)
        kwargs: dict[str, Any] = {
            "site_name": ["indeed"],
            "search_term": query,
            "location": location,
            "country_indeed": "Indonesia",
            "results_wanted": wanted,
        }
        if posted_within_days is not None:
            kwargs["hours_old"] = posted_within_days * 24
        if work_mode == "remote":
            kwargs["is_remote"] = True

        try:
            df = await asyncio.to_thread(scrape_jobs, **kwargs)
        except Exception as exc:  # noqa: BLE001
            msg = str(exc)
            if "429" in msg or "rate" in msg.lower() or "too many" in msg.lower():
                raise ScraperError(
                    "RATE_LIMITED",
                    f"Indeed membatasi permintaan untuk query '{query}'.",
                ) from exc
            raise ScraperError(
                "UPSTREAM_ERROR",
                f"Gagal mengambil data Indeed untuk query '{query}'.",
            ) from exc

        if df is None or getattr(df, "empty", True):
            return []

        results: list[ScrapedJob] = []
        for record in df.to_dict(orient="records"):
            title = _safe_str(record.get("title"))
            company = _safe_str(record.get("company")) or "Unknown Company"
            job_location = _safe_str(record.get("location")) or location
            url = sanitize_external_url(_safe_str(record.get("job_url")) or None)
            raw_desc = clean_html_description(
                _safe_str(record.get("description")),
                max_chars=self._settings.max_description_chars,
            )
            if not title or not raw_desc:
                continue

            raw_id = _safe_str(record.get("id"))
            if raw_id:
                external_id = raw_id if raw_id.startswith("in-") else f"in-{raw_id}"
            else:
                external_id = make_deterministic_external_id(
                    self.source_name, company, title, job_location
                )

            posted_at = _safe_str(record.get("date_posted")) or None
            salary_text = _format_salary(record)

            results.append(
                ScrapedJob(
                    source=self.source_name,
                    external_id=external_id,
                    url=url,
                    title=title,
                    company=company,
                    location=job_location,
                    raw_description=raw_desc,
                    posted_at=posted_at,
                    salary_text=salary_text,
                )
            )
        return results
