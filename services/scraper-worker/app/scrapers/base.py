from abc import ABC, abstractmethod

from app.pipeline.schemas import ScrapedJob, SourceName


class ScraperError(Exception):
    """Domain error raised by individual portal scrapers for partial-failure reporting."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


class BaseScraper(ABC):
    source_name: SourceName

    @abstractmethod
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
        """Collect and clean job listings for a single search query."""
