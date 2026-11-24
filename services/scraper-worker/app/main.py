import time
from collections.abc import Mapping

from fastapi import FastAPI

from app.config import WorkerSettings
from app.pipeline.cleaner import deduplicate_jobs
from app.pipeline.schemas import (
    HealthResponse,
    ScrapedJob,
    ScrapeRequest,
    ScrapeResponse,
    ScrapeStats,
    ScrapeWarning,
    SourceName,
)
from app.scrapers.base import BaseScraper, ScraperError
from app.scrapers.glints_api import GlintsApiScraper
from app.scrapers.jobstreet_api import JobStreetApiScraper
from app.scrapers.linkedin_guest import LinkedInGuestScraper

_AVAILABLE_SOURCES: list[SourceName] = ["linkedin", "glints", "jobstreet"]


def create_app(
    scrapers: Mapping[str, BaseScraper] | None = None,
    settings: WorkerSettings | None = None,
) -> FastAPI:
    cfg = settings or WorkerSettings.from_env()
    registry: dict[str, BaseScraper] = (
        dict(scrapers)
        if scrapers is not None
        else {
            "linkedin": LinkedInGuestScraper(cfg),
            "glints": GlintsApiScraper(cfg),
            "jobstreet": JobStreetApiScraper(cfg),
        }
    )

    api = FastAPI(title="Vecta Scraper Worker", version=cfg.version)

    @api.get("/health", response_model=HealthResponse)
    async def health() -> HealthResponse:
        return HealthResponse(
            status="ok",
            version=cfg.version,
            available_sources=list(_AVAILABLE_SOURCES),
        )

    @api.post("/api/v1/scrape", response_model=ScrapeResponse)
    async def scrape(request: ScrapeRequest) -> ScrapeResponse:
        started = time.perf_counter()
        collected: list[ScrapedJob] = []
        warnings: list[ScrapeWarning] = []

        for source in request.sources:
            adapter = registry.get(source)
            if adapter is None:
                warnings.append(
                    ScrapeWarning(
                        source=source,
                        code="UNSUPPORTED_SOURCE",
                        message=f"Adapter untuk sumber '{source}' tidak terdaftar.",
                    )
                )
                continue

            for query in request.queries:
                trimmed = query.strip()
                if not trimmed:
                    continue
                try:
                    fetched = await adapter.scrape(
                        query=trimmed,
                        location=request.location,
                        limit=request.limit_per_query,
                    )
                    collected.extend(fetched)
                except ScraperError as exc:
                    warnings.append(
                        ScrapeWarning(source=source, code=exc.code, message=exc.message)
                    )
                except Exception as exc:  # noqa: BLE001
                    warnings.append(
                        ScrapeWarning(
                            source=source,
                            code="UNEXPECTED_ERROR",
                            message=f"Kesalahan tak terduga pada sumber '{source}' untuk query '{trimmed}': {exc}",
                        )
                    )

        total_fetched = len(collected)
        unique_jobs, dedup_count = deduplicate_jobs(collected)
        duration_ms = int((time.perf_counter() - started) * 1000)

        return ScrapeResponse(
            jobs=unique_jobs,
            stats=ScrapeStats(
                total_fetched=total_fetched,
                deduplicated_in_batch=dedup_count,
                duration_ms=duration_ms,
            ),
            warnings=warnings,
        )

    return api


app = create_app()
