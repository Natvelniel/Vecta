# Security Policy

## Local-First Security Architecture

Vecta is engineered so that sensitive career data never leaves the user's device without explicit user action:

- **Credential Storage:** Optional Google Gemini API keys are stored via the host OS Credential Manager (`keyring` on Windows, macOS, and Linux) and are never persisted in SQLite, localStorage, or log files.
- **Database Isolation:** All profiles, Master CV records, scraped vacancies, match scores, and personal notes live locally in SQLite (WAL mode) under the OS application data directory.
- **Anti-SSRF Controls:** Optional local scraper worker URLs are strictly restricted to loopback hosts (`127.0.0.1`, `localhost`, `::1`) with HTTP redirects disabled and credential-bearing URLs rejected.
- **External Link & File Safety:** Outbound links allow only `http://` and `https://` schemes. Local PDF imports enforce `%PDF-` magic-byte verification and a strict 10 MB size ceiling.
- **No Telemetry:** Vecta sends zero analytics or usage beacons. Network traffic occurs strictly when the user triggers job scraping or invokes an explicit optional Gemini AI action.

## Reporting a Vulnerability

If you discover a security issue in Vecta, please **do not** open a public issue with exploit details. Instead, report it privately via GitHub Security Advisories on this repository or contact the maintainer directly. Please include reproduction steps, affected component (`src-tauri`, `vecta-taxonomy`, `apps/client`, or `scraper-worker`), and potential impact.
