import hashlib
import re
from urllib.parse import urlparse

from bs4 import BeautifulSoup

from app.pipeline.schemas import ScrapedJob

_HORIZONTAL_SPACES = re.compile(r"[ \t\r\f\v]+")
_BLOCK_TAGS = ("p", "div", "section", "article", "h1", "h2", "h3", "h4", "h5", "h6", "br", "tr")


def sanitize_external_url(raw_url: str | None) -> str | None:
    """Allow only absolute http/https URLs with a valid network host; reject all other schemes."""
    if not raw_url:
        return None
    trimmed = raw_url.strip()
    if not trimmed:
        return None
    try:
        parsed = urlparse(trimmed)
    except ValueError:
        return None
    if parsed.scheme.lower() not in {"http", "https"} or not parsed.netloc:
        return None
    return trimmed


def clean_html_description(raw_html: str, max_chars: int = 12_000) -> str:
    """Strip HTML/scripts/styles, convert <li> to bullet lines, normalize whitespace, and cap length."""
    if not raw_html:
        return ""

    soup = BeautifulSoup(raw_html, "html.parser")

    for tag in soup(["script", "style", "noscript", "svg"]):
        tag.decompose()

    for li in soup.find_all("li"):
        bullet_text = _HORIZONTAL_SPACES.sub(" ", li.get_text(" ", strip=True)).strip()
        if bullet_text:
            li.replace_with(soup.new_string(f"\n- {bullet_text}\n"))
        else:
            li.decompose()

    for tag_name in _BLOCK_TAGS:
        for tag in soup.find_all(tag_name):
            if tag_name == "br":
                tag.replace_with(soup.new_string("\n"))
            else:
                tag.insert_before(soup.new_string("\n"))
                tag.insert_after(soup.new_string("\n"))

    raw_text = soup.get_text(separator=" ")
    lines: list[str] = []
    for line in raw_text.splitlines():
        normalized = _HORIZONTAL_SPACES.sub(" ", line).strip()
        if normalized:
            lines.append(normalized)

    cleaned = "\n".join(lines)
    if len(cleaned) > max_chars:
        return cleaned[:max_chars]
    return cleaned


def make_deterministic_external_id(source: str, company: str, title: str, location: str) -> str:
    """Compute PRD Section 5.3 deterministic 16-hex-char SHA-256 fallback external_id."""
    normalized_key = (
        f"{source}|{company.lower().strip()}|{title.lower().strip()}|{location.lower().strip()}"
    )
    return hashlib.sha256(normalized_key.encode("utf-8")).hexdigest()[:16]


def deduplicate_jobs(jobs: list[ScrapedJob]) -> tuple[list[ScrapedJob], int]:
    """Deduplicate jobs in-batch by (source, external_id), preserving first occurrence order."""
    seen: set[tuple[str, str]] = set()
    unique: list[ScrapedJob] = []
    duplicates = 0

    for job in jobs:
        key = (job.source, job.external_id)
        if key in seen:
            duplicates += 1
            continue
        seen.add(key)
        unique.append(job)

    return unique, duplicates
