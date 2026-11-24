import json
from typing import Any

import httpx

from app.config import WorkerSettings
from app.pipeline.cleaner import clean_html_description, make_deterministic_external_id
from app.pipeline.schemas import ScrapedJob, SourceName
from app.scrapers.base import BaseScraper, ScraperError


def _extract_glints_description(item: dict[str, Any], max_chars: int) -> str:
    raw_value = item.get("descriptionJsonString") or item.get("description")
    if not isinstance(raw_value, str) or not raw_value.strip():
        return ""

    stripped = raw_value.strip()
    if stripped.startswith("{"):
        try:
            parsed = json.loads(stripped)
        except json.JSONDecodeError:
            parsed = None
        if isinstance(parsed, dict) and isinstance(parsed.get("blocks"), list):
            lines: list[str] = []
            for block in parsed["blocks"]:
                if not isinstance(block, dict):
                    continue
                text = str(block.get("text") or "").strip()
                if not text:
                    continue
                block_type = str(block.get("type") or "")
                if block_type in {"unordered-list-item", "ordered-list-item"}:
                    lines.append(f"- {text}")
                else:
                    lines.append(text)
            if lines:
                return clean_html_description("\n".join(lines), max_chars=max_chars)
            return ""

    return clean_html_description(stripped, max_chars=max_chars)

_GLINTS_GRAPHQL_URL = "https://glints.com/api/v2-cms/graphql"
_GLINTS_SEARCH_QUERY = """
query searchJobsV3($data: JobSearchConditionInput!) {
  searchJobsV3(data: $data) {
    jobsInPage {
      id
      title
      descriptionJsonString
      company {
        name
      }
      city {
        name
      }
      country {
        name
      }
    }
  }
}
"""


class GlintsApiScraper(BaseScraper):
    """Public Glints GraphQL job search adapter using httpx.AsyncClient."""

    source_name: SourceName = "glints"

    def __init__(self, settings: WorkerSettings) -> None:
        self._settings = settings

    async def scrape(self, query: str, location: str, limit: int) -> list[ScrapedJob]:
        headers = {
            "User-Agent": self._settings.user_agent,
            "Accept": "application/json",
            "Content-Type": "application/json",
            "Origin": "https://glints.com",
            "Referer": "https://glints.com/id/opportunities/jobs/explore",
        }
        payload = {
            "operationName": "searchJobsV3",
            "variables": {
                "data": {
                    "SearchTerm": query,
                    "CountryCode": "ID",
                    "limit": limit,
                    "offset": 0,
                    "includeExternalJobs": True,
                }
            },
            "query": _GLINTS_SEARCH_QUERY,
        }

        try:
            async with httpx.AsyncClient(timeout=self._settings.http_timeout_sec) as client:
                response = await client.post(_GLINTS_GRAPHQL_URL, json=payload, headers=headers)
        except httpx.TimeoutException as exc:
            raise ScraperError(
                "TIMEOUT", f"Glints timeout saat mengambil query '{query}'."
            ) from exc
        except httpx.HTTPError as exc:
            raise ScraperError(
                "UPSTREAM_ERROR", f"Koneksi Glints gagal untuk query '{query}': {exc}"
            ) from exc

        if response.status_code == 429:
            raise ScraperError(
                "RATE_LIMITED",
                f"Glints mengembalikan HTTP 429 pada query '{query}'; hasil parsial tetap dikembalikan.",
            )
        if response.status_code >= 400:
            raise ScraperError(
                "UPSTREAM_ERROR",
                f"Glints mengembalikan HTTP {response.status_code} untuk query '{query}'.",
            )

        try:
            body: Any = response.json()
        except Exception as exc:  # noqa: BLE001
            raise ScraperError(
                "PARSE_ERROR", f"Respons JSON Glints tidak dapat diparsing: {exc}"
            ) from exc

        if not isinstance(body, dict):
            raise ScraperError("PARSE_ERROR", "Respons JSON Glints bukan objek.")

        errors = body.get("errors")
        if errors:
            first_msg = (
                errors[0].get("message")
                if isinstance(errors, list) and errors and isinstance(errors[0], dict)
                else str(errors)
            )
            raise ScraperError(
                "UPSTREAM_ERROR",
                f"Glints GraphQL mengembalikan error untuk query '{query}': {first_msg}",
            )

        data_node = body.get("data") or {}
        search_node = data_node.get("searchJobsV3") if isinstance(data_node, dict) else None
        items = (search_node.get("jobsInPage") if isinstance(search_node, dict) else None) or []

        jobs: list[ScrapedJob] = []
        for item in items[:limit]:
            if not isinstance(item, dict):
                continue
            title = str(item.get("title") or "").strip()
            company = str((item.get("company") or {}).get("name") or "Unknown Company").strip()
            city = str((item.get("city") or {}).get("name") or "").strip()
            country = str((item.get("country") or {}).get("name") or location).strip()
            job_location = ", ".join(part for part in (city, country) if part) or location
            raw_desc = _extract_glints_description(
                item, max_chars=self._settings.max_description_chars
            )
            if not title or not raw_desc:
                continue

            raw_id = str(item.get("id") or "").strip()
            external_id = (
                f"gl-{raw_id}"
                if raw_id
                else make_deterministic_external_id(self.source_name, company, title, job_location)
            )
            url = f"https://glints.com/id/opportunities/jobs/{raw_id}" if raw_id else None

            jobs.append(
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
        return jobs
