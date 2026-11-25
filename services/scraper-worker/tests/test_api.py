from pathlib import Path

from fastapi.testclient import TestClient

from app.main import create_app
from app.pipeline.schemas import ScrapedJob, SourceName
from app.scrapers.base import BaseScraper, ScraperError


class _StubScraper(BaseScraper):
    def __init__(self, source: SourceName, jobs_by_query: dict[str, list[ScrapedJob]]) -> None:
        self.source_name: SourceName = source
        self._jobs_by_query = jobs_by_query

    async def scrape(self, query: str, location: str, limit: int, **kwargs: object) -> list[ScrapedJob]:
        return self._jobs_by_query.get(query, [])[:limit]


class _FailingScraper(BaseScraper):
    def __init__(self, source: SourceName, code: str, message: str) -> None:
        self.source_name: SourceName = source
        self._code = code
        self._message = message

    async def scrape(self, query: str, location: str, limit: int, **kwargs: object) -> list[ScrapedJob]:
        raise ScraperError(self._code, self._message)


def _sample_job(source: SourceName, external_id: str, title: str) -> ScrapedJob:
    return ScrapedJob(
        source=source,
        external_id=external_id,
        url=f"https://example.com/{source}/{external_id}",
        title=title,
        company="PT Vecta Demo",
        location="Jakarta, Indonesia",
        raw_description=f"Looking for {title} experienced in Python and SQL.",
    )


def test_health_endpoint_returns_expected_sources() -> None:
    client = TestClient(create_app())
    response = client.get("/health")

    assert response.status_code == 200
    assert response.json() == {
        "status": "ok",
        "version": "0.1.0",
        "available_sources": ["linkedin", "jobstreet", "indeed", "glints"],
    }


def test_scrape_endpoint_merges_and_deduplicates_across_queries() -> None:
    shared_li = _sample_job("linkedin", "li-1", "Backend Engineer")
    second_li = _sample_job("linkedin", "li-2", "Rust Developer")
    glints_job = _sample_job("glints", "gl-1", "Data Analyst")

    app = create_app(
        scrapers={
            "linkedin": _StubScraper(
                "linkedin",
                {
                    "Backend": [shared_li, second_li],
                    "Rust": [shared_li],
                },
            ),
            "glints": _StubScraper(
                "glints",
                {
                    "Backend": [glints_job],
                    "Rust": [],
                },
            ),
        }
    )
    client = TestClient(app)

    response = client.post(
        "/api/v1/scrape",
        json={
            "queries": ["Backend", "Rust"],
            "sources": ["linkedin", "glints"],
            "location": "Indonesia",
            "limit_per_query": 15,
        },
    )

    assert response.status_code == 200
    body = response.json()
    assert len(body["jobs"]) == 3
    assert body["stats"]["total_fetched"] == 4
    assert body["stats"]["deduplicated_in_batch"] == 1
    assert isinstance(body["stats"]["duration_ms"], int)
    assert body["warnings"] == []


def test_scrape_endpoint_partial_failure_returns_200_with_warnings() -> None:
    li_1 = _sample_job("linkedin", "li-10", "Software Developer")
    li_2 = _sample_job("linkedin", "li-11", "Junior Backend Engineer")

    app = create_app(
        scrapers={
            "linkedin": _StubScraper("linkedin", {"Software Developer": [li_1, li_2]}),
            "glints": _FailingScraper(
                "glints",
                "RATE_LIMITED",
                "Glints mengembalikan HTTP 429 pada query pertama; hasil parsial tetap dikembalikan.",
            ),
        }
    )
    client = TestClient(app)

    response = client.post(
        "/api/v1/scrape",
        json={
            "queries": ["Software Developer"],
            "sources": ["linkedin", "glints"],
            "location": "Indonesia",
            "limit_per_query": 10,
        },
    )

    assert response.status_code == 200
    body = response.json()
    assert len(body["jobs"]) == 2
    assert len(body["warnings"]) == 1
    assert body["warnings"][0]["source"] == "glints"
    assert body["warnings"][0]["code"] == "RATE_LIMITED"


def test_scrape_endpoint_rejects_empty_queries() -> None:
    client = TestClient(create_app())
    response = client.post(
        "/api/v1/scrape",
        json={
            "queries": [],
            "sources": ["linkedin"],
            "location": "Indonesia",
            "limit_per_query": 10,
        },
    )
    assert response.status_code == 422


class _ExplodingScraper(BaseScraper):
    def __init__(self, source: SourceName) -> None:
        self.source_name: SourceName = source

    async def scrape(self, query: str, location: str, limit: int, **kwargs: object) -> list[ScrapedJob]:
        raise RuntimeError("/internal/secret/path/traceback-leak")


def test_scrape_endpoint_rejects_excessive_queries_and_sanitizes_unexpected_errors() -> None:
    app = create_app(scrapers={"linkedin": _ExplodingScraper("linkedin")})
    client = TestClient(app)

    too_many = client.post(
        "/api/v1/scrape",
        json={
            "queries": [f"role-{i}" for i in range(16)],
            "sources": ["linkedin"],
            "location": "Indonesia",
            "limit_per_query": 5,
        },
    )
    assert too_many.status_code == 422

    too_long = client.post(
        "/api/v1/scrape",
        json={
            "queries": ["a" * 121],
            "sources": ["linkedin"],
            "location": "Indonesia",
            "limit_per_query": 5,
        },
    )
    assert too_long.status_code == 422

    resp = client.post(
        "/api/v1/scrape",
        json={
            "queries": ["Rust"],
            "sources": ["linkedin"],
            "location": "Indonesia",
            "limit_per_query": 5,
        },
    )
    assert resp.status_code == 200
    warnings = resp.json()["warnings"]
    assert len(warnings) == 1
    assert warnings[0]["code"] == "UNEXPECTED_ERROR"
    assert "/internal/secret/path" not in warnings[0]["message"]
    assert "traceback-leak" not in warnings[0]["message"]


def test_worker_contains_no_gemini_api_key_references() -> None:
    app_root = Path(__file__).resolve().parent.parent / "app"
    py_files = list(app_root.rglob("*.py"))
    assert py_files, "Expected Python modules under app/"
    for file_path in py_files:
        content = file_path.read_text(encoding="utf-8")
        assert "GEMINI_API_KEY" not in content
        assert "x-goog-api-key" not in content.lower()
        assert "generativelanguage.googleapis.com" not in content.lower()
