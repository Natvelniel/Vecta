import os
from dataclasses import dataclass


@dataclass(frozen=True)
class WorkerSettings:
    version: str = "0.1.0"
    http_timeout_sec: float = 15.0
    max_description_chars: int = 12_000
    linkedin_jitter_min_sec: float = 3.0
    linkedin_jitter_max_sec: float = 8.0
    user_agent: str = (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
        "AppleWebKit/537.36 (KHTML, like Gecko) "
        "Chrome/131.0.0.0 Safari/537.36"
    )

    @classmethod
    def from_env(cls) -> "WorkerSettings":
        return cls(
            http_timeout_sec=float(os.getenv("VECTA_HTTP_TIMEOUT_SEC", "15.0")),
            max_description_chars=int(os.getenv("VECTA_MAX_DESC_CHARS", "12000")),
            linkedin_jitter_min_sec=float(os.getenv("VECTA_LINKEDIN_JITTER_MIN", "3.0")),
            linkedin_jitter_max_sec=float(os.getenv("VECTA_LINKEDIN_JITTER_MAX", "8.0")),
        )
