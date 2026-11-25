import hashlib

from app.pipeline.cleaner import (
    clean_html_description,
    deduplicate_jobs,
    make_deterministic_external_id,
    sanitize_external_url,
)
from app.pipeline.schemas import ScrapedJob


def test_clean_html_strips_scripts_styles_and_converts_list_items_to_bullets() -> None:
    raw_html = """
    <html>
      <head>
        <style>.hidden { display: none; }</style>
        <script>console.log('tracking');</script>
      </head>
      <body>
        <div>
          <h2>Backend Engineer</h2>
          <p>We are   hiring    engineers.</p>
          <ul>
            <li>Build APIs with   Rust</li>
            <li>Maintain SQLite &amp; FastAPI services</li>
          </ul>
          <p>Great   culture!</p>
        </div>
      </body>
    </html>
    """
    cleaned = clean_html_description(raw_html)

    assert "console.log" not in cleaned
    assert "display: none" not in cleaned
    assert "<" not in cleaned and ">" not in cleaned
    assert "- Build APIs with Rust" in cleaned
    assert "- Maintain SQLite & FastAPI services" in cleaned
    assert "We are hiring engineers." in cleaned
    assert "\n\n\n" not in cleaned


def test_clean_html_truncates_at_12000_chars() -> None:
    raw_html = "<p>" + ("x" * 15_000) + "</p>"
    cleaned = clean_html_description(raw_html)

    assert len(cleaned) == 12_000


def test_deterministic_external_id_matches_sha256_spec() -> None:
    expected = hashlib.sha256(
        "glints|pt contoh teknologi|junior backend developer|jakarta, indonesia".encode("utf-8")
    ).hexdigest()[:16]

    actual_a = make_deterministic_external_id(
        source="glints",
        company="  PT Contoh Teknologi ",
        title="Junior Backend Developer  ",
        location="  Jakarta, Indonesia",
    )
    actual_b = make_deterministic_external_id(
        source="glints",
        company="pt contoh teknologi",
        title="junior backend developer",
        location="jakarta, indonesia",
    )

    assert len(actual_a) == 16
    assert actual_a == expected
    assert actual_a == actual_b


def test_deduplicate_jobs_preserves_order_and_counts_duplicates() -> None:
    j1 = ScrapedJob(
        source="linkedin",
        external_id="li-100",
        url="https://linkedin.com/jobs/view/100",
        title="Backend Dev",
        company="Acme",
        location="Jakarta",
        raw_description="Role A",
    )
    j1_dup = ScrapedJob(
        source="linkedin",
        external_id="li-100",
        url="https://linkedin.com/jobs/view/100",
        title="Backend Dev",
        company="Acme",
        location="Jakarta",
        raw_description="Role A duplicate",
    )
    j2_other_source = ScrapedJob(
        source="glints",
        external_id="li-100",
        url="https://glints.com/jobs/100",
        title="Backend Dev",
        company="Acme",
        location="Jakarta",
        raw_description="Role A on glints",
    )

    unique, dup_count = deduplicate_jobs([j1, j1_dup, j2_other_source])

    assert dup_count == 1
    assert len(unique) == 2
    assert unique[0].raw_description == "Role A"
    assert unique[1].source == "glints"


def test_sanitize_external_url_allows_only_http_and_https() -> None:
    assert sanitize_external_url("https://linkedin.com/jobs/1") == "https://linkedin.com/jobs/1"
    assert sanitize_external_url("  http://glints.com/job/2  ") == "http://glints.com/job/2"
    assert sanitize_external_url("javascript:alert(1)") is None
    assert sanitize_external_url("file:///etc/passwd") is None
    assert sanitize_external_url("data:text/html,hi") is None
    assert sanitize_external_url("https://") is None
    assert sanitize_external_url("   ") is None
    assert sanitize_external_url(None) is None
