from typing import Literal

from pydantic import BaseModel, Field

SourceName = Literal["linkedin", "glints", "jobstreet"]


class HealthResponse(BaseModel):
    status: Literal["ok"] = "ok"
    version: str
    available_sources: list[SourceName]


class ScrapeRequest(BaseModel):
    queries: list[str] = Field(min_length=1)
    sources: list[SourceName] = Field(min_length=1)
    location: str = "Indonesia"
    limit_per_query: int = Field(default=15, ge=1, le=50)


class ScrapedJob(BaseModel):
    source: SourceName
    external_id: str
    url: str | None = None
    title: str
    company: str
    location: str | None = None
    raw_description: str


class ScrapeStats(BaseModel):
    total_fetched: int
    deduplicated_in_batch: int
    duration_ms: int


class ScrapeWarning(BaseModel):
    source: str
    code: str
    message: str


class ScrapeResponse(BaseModel):
    jobs: list[ScrapedJob]
    stats: ScrapeStats
    warnings: list[ScrapeWarning]
