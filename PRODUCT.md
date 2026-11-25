# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Vue 3 (^3.5) + TypeScript + Vite + Vue Router (^4) + Pinia (^2) + Tailwind CSS v4 + @tauri-apps/api (^2) + @tauri-apps/plugin-opener (^2) + Vitest, bundled inside Tauri 2.0 (`apps/client/`). All backend access goes strictly through the 22 auto-generated `tauri-specta` IPC commands in `apps/client/src/lib/bindings.ts` plus official `tauri-plugin-opener` for opening external URLs in the OS browser.

## Users

Job seekers and early-to-mid career professionals in Indonesia (including the author as primary daily user and portfolio showcase audience) who know the projects they have built, struggle with rigid HR keyword forms, want objective visibility into what skills the Indonesian job market actually demands, and want a clear path from skill gaps to actionable learning steps.

## Product Purpose

Vecta is a local-first AI career intelligence desktop (and responsive mobile) application that turns freeform skill stories into a structured competency profile, real-time market competency maps, sub-100ms local hybrid job matching, on-demand gap analysis with CV tailoring tips, and step-by-step learning roadmaps. Success means a user can go from a single paragraph of freeform career story to ranked real jobs, clear skill gaps, and tracked learning progress in one calm workspace without form fatigue, cloud lock-in, or unnecessary AI token burn.

## Positioning

Local-first BYOK (Bring Your Own Key — Google Gemini stored solely in OS Keychain) career workbench that separates deterministic local intelligence (Rust + SQLite WAL + 768-dim vector/skill/level hybrid matching in < 100ms at zero token cost) from deliberate, cached on-demand AI extraction and reasoning, paired with a stateless local Python scraper worker that never touches user API keys.

## Operating Context

- Runs as a Tauri 2.0 desktop application (`1440x900` default, responsive down to `390x844` mobile breakpoint at `1024px`) communicating exclusively via typed Tauri IPC with the Rust Core engine and local SQLite database.
- Optional companion Python FastAPI Scraper Worker (`http://127.0.0.1:8000`, configurable in Settings) fetches jobs from LinkedIn Guest, Glints, and JobStreet with partial-failure resilience; manual job text/URL paste always works offline from portals without the worker.
- First-run flow guides new users through a 2-step onboarding (Step 1: Gemini BYOK API Key setup & test; Step 2: Story-to-Chips profile creation) with an explicit "Lewati ke Dashboard" option so users can explore freely first.
- All user-facing copy, labels, errors, and AI insights (Gap Analysis & Roadmap) are in Bahasa Indonesia, while canonical technical skill names remain standard English (e.g., `Rust`, `SQLite`, `Docker`).

## Capabilities and Constraints

- **Strict 22-command IPC boundary:** Frontend must never call Gemini API, Python Worker, or SQLite directly; all persistence and AI orchestration use the 22 commands in `apps/client/src/lib/bindings.ts` (`Result<T, AppError>` with 7 standardized `SCREAMING_SNAKE_CASE` error codes).
- **Local DB export/reset deferred:** Not part of the current 22-command MVP scope.
- **Zero-secrets rule:** Gemini API key is never written to `localStorage`, SQLite, or logs; UI displays only masked suffix (`AIza...9xQp`).
- **Cached & quota-aware UX:** Gap analysis loads from SQLite cache unless user explicitly requests refresh; re-scraping duplicates burns 0 Gemini calls; `quick_add_user_skill` recalculates all matches locally with 0 Gemini calls.
- **Phase 2 explicitly out of scope:** Turso/LibSQL sync (FR-09), Chrome Extension (FR-10), Radar Chart / side-by-side role comparison / interactive CV editor (FR-11), GitHub Actions Cron.
- **Type-safety note:** `tauri-specta` (`specta-typescript 0.0.15`) exports Rust `f64` fields as `number | null` in `bindings.ts`; frontend stores/formatters must guard numeric fields safely (`?? 0`) so `pnpm typecheck` (`vue-tsc --noEmit`) passes with 0 errors.

## Brand Commitments

- Product name: **Vecta**.
- Authoritative visual system pinned in `VECTA_DESIGN_SYSTEM.md`: **Obsidian Navy x Cobalt Blue** dark-first theme (`#111827` primary canvas, `#172235` subordinate `240px` sidebar, `#1A2333` elevated cards, `#243249` hover, `#4169E1`/`#6F8FFF`/`#3154C5`/`#243B75` Cobalt Blue brand accent, `#9B8CFA` violet strictly reserved for AI-specific micro-markers, semantic colors `#34C38F`/`#F0B85A`/`#F07178`/`#55A6E8`), self-hosted **Satoshi** typeface with `tabular-nums`, `4px` spacing scale, and `6/10/15/20px` geometric corner radius system driven by reusable CSS custom properties in `src/styles/tokens.css`.

## Evidence on Hand

- Authoritative functional spec: `product_requirements_document_prd_vecta.md` (PRD v2.0).
- Authoritative visual spec: `VECTA_DESIGN_SYSTEM.md`.
- Working, tested backend on branch `Development`: 52 Rust tests passing, 14 Python scraper worker tests passing, 22 typed IPC commands exported to `apps/client/src/lib/bindings.ts`.
- Three sample story presets defined in PRD Section 4.4 (`Software Dev`, `Data Analyst`, `Hybrid Tech`) for `OnboardingStoryView` empty state; no fabricated user metrics, testimonials, or fake live job listings should be shipped as real data.

## Product Principles

1. **Calm single-focus workspace:** One clear task per screen with generous structure; avoid crowded multi-widget dashboards.
2. **Evidence before AI spend:** Show deterministic local match scores, matched/missing skills, and market demand percentages immediately; spend BYOK Gemini tokens only on deliberate, cached user actions.
3. **Never lose user input or trust:** Preserve typed career stories and manual job text across errors; distinguish recoverable warnings (partial scrape failure, offline worker) from blocking errors with clear Bahasa Indonesia recovery paths.
4. **Close the loop from insight to action:** Every screen connects directly to the next step: Story -> Ranked Jobs -> Gap Analysis -> Roadmap -> Skill Added -> Recalculated Match Score.

## Accessibility & Inclusion

- Respect `prefers-reduced-motion: reduce` globally across all CSS transitions and interactions.
- Full keyboard accessibility (`Tab`, `Enter`, `Space`, `Escape`), visible focus rings (`--brand-hover`), and readable contrast at Desktop (`1440x900`), compact desktop (`1024x768`), and Mobile (`390x844`) viewports.
