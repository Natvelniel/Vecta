from typing import Annotated, Literal

from pydantic import BaseModel, Field

SourceName = Literal["linkedin", "jobstreet", "indeed", "glints"]
WorkModePref = Literal["any", "remote", "hybrid", "onsite"]
QueryText = Annotated[str, Field(min_length=1, max_length=120)]


class HealthResponse(BaseModel):
    status: Literal["ok"] = "ok"
    version: str
    available_sources: list[SourceName]


class ScrapeRequest(BaseModel):
    queries: list[QueryText] = Field(min_length=1, max_length=15)
    sources: list[SourceName] = Field(min_length=1, max_length=4)
    location: str = Field(default="Indonesia", max_length=80)
    limit_per_query: int = Field(default=20, ge=1, le=50)
    max_pages: int = Field(default=2, ge=1, le=5)
    posted_within_days: int | None = Field(default=None, ge=1, le=90)
    work_mode: WorkModePref = "any"


class ScrapedJob(BaseModel):
    source: SourceName
    external_id: str
    url: str | None = None
    title: str
    company: str
    location: str | None = None
    raw_description: str
    posted_at: str | None = None
    salary_text: str | None = None


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
