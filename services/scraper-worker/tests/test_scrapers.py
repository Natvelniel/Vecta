import json
from typing import Any

import httpx
import pytest

from app.config import WorkerSettings
from app.scrapers.base import ScraperError
from app.scrapers.glints_api import GlintsApiScraper
from app.scrapers.indeed_scraper import IndeedScraper
from app.scrapers.jobstreet_api import JobStreetApiScraper
from app.scrapers.linkedin_guest import LinkedInGuestScraper


_REAL_ASYNC_CLIENT = httpx.AsyncClient


def _patch_async_client(monkeypatch: pytest.MonkeyPatch, handler: Any) -> None:
    transport = httpx.MockTransport(handler)

    def _factory(*args: Any, **kwargs: Any) -> httpx.AsyncClient:
        kwargs["transport"] = transport
        return _REAL_ASYNC_CLIENT(*args, **kwargs)

    monkeypatch.setattr(httpx, "AsyncClient", _factory)


@pytest.mark.asyncio
async def test_glints_parses_draftjs_blocks_and_skips_missing_description(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    draftjs_payload = json.dumps(
        {
            "blocks": [
                {"text": "Join our backend engineering team.", "type": "unstyled"},
                {"text": "Strong Rust and SQLite skills", "type": "unordered-list-item"},
                {"text": "Experience with FastAPI", "type": "unordered-list-item"},
            ],
            "entityMap": {},
        }
    )

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={
                "data": {
                    "searchJobsV3": {
                        "jobsInPage": [
                            {
                                "id": "gl-abc-1",
                                "title": "Backend Engineer",
                                "descriptionJsonString": draftjs_payload,
                                "company": {"name": "PT Glints Demo"},
                                "city": {"name": "Jakarta"},
                                "country": {"name": "Indonesia"},
                            },
                            {
                                "id": "gl-empty-2",
                                "title": "Empty Desc Role",
                                "descriptionJsonString": None,
                                "company": {"name": "PT Empty"},
                                "city": {"name": "Bandung"},
                                "country": {"name": "Indonesia"},
                            },
                        ]
                    }
                }
            },
        )

    _patch_async_client(monkeypatch, handler)
    scraper = GlintsApiScraper(WorkerSettings())
    jobs = await scraper.scrape("Backend Engineer", "Indonesia", 10)

    assert len(jobs) == 1, "Listing without description must be skipped, not fallback to title"
    assert jobs[0].external_id == "gl-gl-abc-1"
    assert jobs[0].company == "PT Glints Demo"
    assert jobs[0].location == "Jakarta, Indonesia"
    assert '"blocks"' not in jobs[0].raw_description
    assert "Join our backend engineering team." in jobs[0].raw_description
    assert "- Strong Rust and SQLite skills" in jobs[0].raw_description
    assert "- Experience with FastAPI" in jobs[0].raw_description


@pytest.mark.asyncio
async def test_glints_graphql_null_data_with_errors_raises_upstream_error(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={
                "data": None,
                "errors": [{"message": "Upstream GraphQL temporarily unavailable"}],
            },
        )

    _patch_async_client(monkeypatch, handler)
    scraper = GlintsApiScraper(WorkerSettings())
    with pytest.raises(ScraperError) as exc_info:
        await scraper.scrape("Rust Developer", "Indonesia", 5)

    assert exc_info.value.code == "UPSTREAM_ERROR"
    assert "Upstream GraphQL temporarily unavailable" in exc_info.value.message


@pytest.mark.asyncio
async def test_glints_maps_429_and_timeout(monkeypatch: pytest.MonkeyPatch) -> None:
    _patch_async_client(monkeypatch, lambda _req: httpx.Response(429, text="rate limited"))
    scraper = GlintsApiScraper(WorkerSettings())
    with pytest.raises(ScraperError) as exc_429:
        await scraper.scrape("Rust", "Indonesia", 5)
    assert exc_429.value.code == "RATE_LIMITED"

    def raise_timeout(request: httpx.Request) -> httpx.Response:
        raise httpx.ReadTimeout("timed out", request=request)

    _patch_async_client(monkeypatch, raise_timeout)
    with pytest.raises(ScraperError) as exc_timeout:
        await scraper.scrape("Rust", "Indonesia", 5)
    assert exc_timeout.value.code == "TIMEOUT"


@pytest.mark.asyncio
async def test_jobstreet_parses_valid_response_and_maps_errors(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={
                "data": [
                    {
                        "id": "889911",
                        "title": "Software Engineer",
                        "advertiser": {"description": "PT Nusantara Tech"},
                        "locations": [{"label": "Jakarta Selatan"}],
                        "teaser": "Build desktop and API systems.",
                        "bulletPoints": ["Rust & SQLite", "Remote friendly"],
                    },
                    {
                        "id": "889912",
                        "title": "No Desc Job",
                        "advertiser": {"description": "PT Blank"},
                        "locations": [],
                        "teaser": "",
                        "bulletPoints": [],
                    },
                ]
            },
        )

    _patch_async_client(monkeypatch, handler)
    scraper = JobStreetApiScraper(WorkerSettings())
    jobs = await scraper.scrape("Software Engineer", "Indonesia", 10)

    assert len(jobs) == 1
    assert jobs[0].external_id == "js-889911"
    assert jobs[0].company == "PT Nusantara Tech"
    assert jobs[0].location == "Jakarta Selatan"
    assert jobs[0].url == "https://id.jobstreet.com/id/job/889911"
    assert "Build desktop and API systems." in jobs[0].raw_description
    assert "- Rust & SQLite" in jobs[0].raw_description

    _patch_async_client(monkeypatch, lambda _req: httpx.Response(429, text="too many"))
    with pytest.raises(ScraperError) as exc_429:
        await scraper.scrape("Software Engineer", "Indonesia", 5)
    assert exc_429.value.code == "RATE_LIMITED"

    def raise_timeout(request: httpx.Request) -> httpx.Response:
        raise httpx.ConnectTimeout("timeout", request=request)

    _patch_async_client(monkeypatch, raise_timeout)
    with pytest.raises(ScraperError) as exc_to:
        await scraper.scrape("Software Engineer", "Indonesia", 5)
    assert exc_to.value.code == "TIMEOUT"


@pytest.mark.asyncio
async def test_linkedin_guest_parses_dataframe_skips_empty_desc_and_maps_429(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class _FakeDF:
        empty = False

        def to_dict(self, orient: str = "records") -> list[dict[str, Any]]:
            return [
                {
                    "id": "123456",
                    "title": "Backend Rust Engineer",
                    "company": "PT Vecta",
                    "location": "Jakarta",
                    "job_url": "https://www.linkedin.com/jobs/view/123456",
                    "description": "<p>Build fast systems</p><ul><li>Rust</li></ul>",
                },
                {
                    "id": "123457",
                    "title": "Empty Description Role",
                    "company": "PT Skip",
                    "location": "Jakarta",
                    "job_url": None,
                    "description": "nan",
                },
            ]

    monkeypatch.setattr(
        "app.scrapers.linkedin_guest.scrape_jobs",
        lambda **kwargs: _FakeDF(),
    )
    settings = WorkerSettings(linkedin_jitter_min_sec=0.0, linkedin_jitter_max_sec=0.0)
    scraper = LinkedInGuestScraper(settings)
    jobs = await scraper.scrape("Backend Rust", "Indonesia", 5)

    assert len(jobs) == 1
    assert jobs[0].external_id == "li-123456"
    assert jobs[0].company == "PT Vecta"
    assert "- Rust" in jobs[0].raw_description

    def fail_429(**kwargs: Any) -> Any:
        raise RuntimeError("HTTP 429 Too Many Requests from LinkedIn")

    monkeypatch.setattr("app.scrapers.linkedin_guest.scrape_jobs", fail_429)
    with pytest.raises(ScraperError) as exc_429:
        await scraper.scrape("Backend Rust", "Indonesia", 5)
    assert exc_429.value.code == "RATE_LIMITED"


@pytest.mark.asyncio
async def test_jobstreet_supports_multipage_and_extracts_salary_and_date(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    seen_pages: list[str] = []

    def handler(request: httpx.Request) -> httpx.Response:
        page = request.url.params.get("page", "1")
        seen_pages.append(page)
        if page == "1":
            return httpx.Response(
                200,
                json={
                    "data": [
                        {
                            "id": "p1-1",
                            "title": "Laravel Developer",
                            "advertiser": {"description": "PT Satu"},
                            "locations": [{"label": "Bandung"}],
                            "teaser": "Build Laravel APIs",
                            "bulletPoints": ["PHP 8", "MySQL"],
                            "salaryLabel": "Rp 10.000.000 - Rp 15.000.000",
                            "listingDate": "2026-11-24T05:00:00Z",
                        },
                        {
                            "id": "p1-2",
                            "title": "PHP Engineer",
                            "advertiser": {"description": "PT Dua"},
                            "locations": [{"label": "Bandung"}],
                            "teaser": "Maintain backend systems",
                            "bulletPoints": ["Laravel"],
                        },
                    ]
                },
            )
        return httpx.Response(
            200,
            json={
                "data": [
                    {
                        "id": "p2-1",
                        "title": "Backend Engineer",
                        "advertiser": {"description": "PT Tiga"},
                        "locations": [{"label": "Bandung"}],
                        "teaser": "Scale REST APIs",
                        "bulletPoints": ["PostgreSQL"],
                    }
                ]
            },
        )

    _patch_async_client(monkeypatch, handler)
    scraper = JobStreetApiScraper(WorkerSettings())
    jobs = await scraper.scrape(
        "Laravel", "Bandung", 2, max_pages=3, posted_within_days=7, work_mode="remote"
    )

    assert seen_pages == ["1", "2"]
    assert len(jobs) == 3
    assert jobs[0].salary_text == "Rp 10.000.000 - Rp 15.000.000"
    assert jobs[0].posted_at == "2026-11-24T05:00:00Z"


@pytest.mark.asyncio
async def test_indeed_scraper_parses_records_and_sanitizes_urls(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class _FakeDF:
        empty = False

        def to_dict(self, orient: str = "records") -> list[dict[str, Any]]:
            return [
                {
                    "id": "abc999",
                    "title": "Backend PHP Developer",
                    "company": "PT Indeed Demo",
                    "location": "Bandung, Jawa Barat",
                    "job_url": "https://id.indeed.com/viewjob?jk=abc999",
                    "description": "<p>Membutuhkan developer PHP Laravel dan MySQL.</p>",
                    "date_posted": "2026-11-23",
                    "min_amount": 8000000,
                    "max_amount": 12000000,
                    "currency": "IDR",
                    "interval": "monthly",
                },
                {
                    "id": "badurl",
                    "title": "Suspicious URL Job",
                    "company": "PT Unsafe",
                    "location": "Jakarta",
                    "job_url": "javascript:alert(1)",
                    "description": "<p>Valid description text here</p>",
                },
            ]

    monkeypatch.setattr("app.scrapers.indeed_scraper.scrape_jobs", lambda **kwargs: _FakeDF())
    scraper = IndeedScraper(WorkerSettings())
    jobs = await scraper.scrape("PHP Developer", "Bandung", 10, max_pages=2)

    assert len(jobs) == 2
    assert jobs[0].source == "indeed"
    assert jobs[0].external_id == "in-abc999"
    assert jobs[0].url == "https://id.indeed.com/viewjob?jk=abc999"
    assert jobs[0].salary_text == "IDR 8000000 - 12000000/monthly"
    assert jobs[0].posted_at == "2026-11-23"
    assert jobs[1].url is None
