# Vecta Frontend GUI Design Specification

**Date:** 2026-11-24  
**Scope:** Complete Vue 3 + Tauri 2.0 Frontend GUI (`apps/client/`) connecting to the verified 22-command Rust Core Engine (`apps/client/src/lib/bindings.ts`) and local Python Scraper Worker.  
**Authoritative Functional Spec:** `product_requirements_document_prd_vecta.md` (PRD v2.0)  
**Authoritative Visual System:** `VECTA_DESIGN_SYSTEM.md` (Obsidian Navy x Cobalt Blue, Satoshi, `6/10/15/20px` radii, selective `#9B8CFA` AI accent)  
**Product Truth:** `PRODUCT.md`

---

## 1. Intent & Why

Vecta's backend (SQLite WAL persistence, OS Keychain BYOK storage, `< 100ms` hybrid matching engine, Gemini v1beta structured contracts A–D, Python FastAPI multi-portal scraper worker, and 22 typed `tauri-specta` IPC commands) is complete and verified on branch `Development`. Right now, `apps/client/dist/` is an empty shell, so the desktop application cannot yet be launched with a graphical interface.

This design specifies the full **Operate-mode** frontend workspace in `apps/client/` so users can run Vecta natively on Desktop (`1440×900` default, responsive down to `390×844` mobile layout at the `1024px` breakpoint) and complete the entire career intelligence loop end-to-end: **Guided BYOK + Story-to-Chips Onboarding → Ranked Jobs Split-Pane → On-Demand Cached Gap Analysis → Dynamic Competency Map → 3-Phase Career Roadmap → Auto-Add Skill & Instant Local Match Recalculation**.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
- Ship all **6 PRD screens** (`OnboardingStoryView`, `DashboardView`, `JobsFeedView`, `CompetencyMapView`, `RoadmapView`, `SettingsView`) plus a **Guided 2-Step First-Run** flow (Step 1: Gemini API Key → Step 2: Story-to-Chips, with an explicit `"Lewati ke Dashboard"` escape hatch).
- Honor all **3 curated themes** pinned in PRD Section 4.2 (`paper-ink` default light, `swiss-mono` dark, `warm-stone` warm light) with **Zero-FOUC** boot via inline `<script>` in `index.html` and zero hardcoded Tailwind palette colors.
- Bind exclusively to the **22 auto-generated IPC commands** in `apps/client/src/lib/bindings.ts` with strict TypeScript safety (`pnpm typecheck` / `vue-tsc --noEmit` = 0 errors).
- Bundle `Plus Jakarta Sans` and `JetBrains Mono` (`tabular-nums` for scores, percentages, counts, and hours) locally via `@fontsource/*` so the desktop app works cleanly offline.
- Deliver purposeful, restrained motion (`motion-v` + `@formkit/auto-animate`) honoring `prefers-reduced-motion: reduce`.
- Make the desktop app immediately launchable via `pnpm tauri dev` (and buildable via `pnpm build` + `cargo build`) while enabling repeatable visual inspection across all 3 themes and both Desktop (`1440×900`) and Mobile (`390×844`) viewports.

### Non-Goals
- **Local DB export/reset IPC commands:** Explicitly deferred from the strict 22-command MVP scope; `SettingsView` will contain the 3 active cards (Gemini API Key BYOK, 3-Theme Selector, Python Worker URL & Health Check) without dead/fake export buttons.
- **Phase 2 features (PRD Section 3):** Turso/LibSQL cloud sync (FR-09), Chrome Extension (FR-10), Radar Chart / side-by-side role comparison / interactive CV editor (FR-11), GitHub Actions Cron.
- **Mobile native packaging (`tauri android build` / iOS):** Responsive `< 1024px` mobile webview layout is built now per PRD Section 4.4, while Android/iOS native builds remain Milestone 4.

### Anti-Goals (Failure even if technically working)
- Any neon glow, generic purple/blue gradient, glassmorphism bloat, or hardcoded Tailwind color utility (`bg-slate-*`, `text-blue-*`, `border-zinc-*`) in core components — every visual surface must consume semantic tokens from `src/styles/themes.css`.
- Calling Gemini API, Python Worker HTTP endpoints, or SQLite directly from Vue.
- Writing the Gemini API key to `localStorage`, `sessionStorage`, Pinia persisted state, or console logs.
- Losing typed user career story text or pasted manual job descriptions when an IPC/network error occurs.
- Crowded 4-box dashboards or modals used as a lazy crutch where inline controls work better.
- Flash of unstyled/wrong-theme white screen (FOUC) when reopening the app in `swiss-mono`.

---

## 3. Binding Constraints & Technical Realities

1. **Tauri 2.0 IPC Contract (`apps/client/src/lib/bindings.ts`):**
   - Never hand-edit `bindings.ts`.
   - Every command returns `Result<T, AppError>` (`{ status: "ok", data: T } | { status: "error", error: AppError }`) where `AppError = { code: ErrorCode, message: string, retryable: boolean }`.
   - Because `specta-typescript 0.0.15` exports Rust `f64` fields as `number | null` (e.g. `final_score`, `vector_score`, `skill_overlap_score`, `level_score`, `demand_percentage`, `progress_percentage`), the frontend IPC adapter must normalize numeric fields with safe `?? 0` guards so view components work with clean `number` values and `vue-tsc` stays at 0 errors.
   - All 7 error codes (`API_KEY_MISSING`, `API_KEY_INVALID`, `GEMINI_RATE_LIMITED`, `GEMINI_PARSE_FAILED`, `SCRAPER_UNREACHABLE`, `DB_ERROR`, `VALIDATION_ERROR`) must map to clear Bahasa Indonesia recovery UX per PRD Section 9.2.
2. **Tauri 2.0 Runtime, Opener Plugin & Capabilities (`apps/client/src-tauri/`):**
   - Update `tauri.conf.json` `build` block to add `devUrl: "http://localhost:1420"`, `beforeDevCommand: "pnpm dev"`, `beforeBuildCommand: "pnpm build"`, keeping `frontendDist: "../dist"`.
   - Add official `tauri-plugin-opener` to `src-tauri/Cargo.toml` + register `.plugin(tauri_plugin_opener::init())` in `src-tauri/src/lib.rs`, plus `@tauri-apps/plugin-opener` on the frontend (keeps the exact 22 custom `tauri-specta` IPC commands unchanged). Expose `openExternalUrl(url)` in `src/lib/ipc.ts` (uses `openUrl` in Tauri runtime, falls back to `window.open(..., "_blank", "noopener,noreferrer")` in browser preview mode) so Google AI Studio and job portal links open cleanly in the user's default OS browser with existing sessions intact.
   - Add `apps/client/src-tauri/capabilities/default.json` granting `core:default` and scoped `opener:allow-open-url` (`https://*`) for the `main` window.
   - Note on Windows `pnpm`: Node `v24.15.0` and Corepack `0.34.6` are installed; enable/prepare `pnpm` via Corepack during scaffold setup.
3. **Language & Copy:**
   - All UI labels, headings, helper copy, empty states, toasts, and error banners in natural, concise **Bahasa Indonesia**; canonical technical skills remain standard English (`Rust`, `SQLite`, `Docker`, `Vue.js`).
4. **Impeccable Design-System Mode & Contract:**
   - All 6 screens are **Operate mode** (code-led build; familiarity, scanability, state completeness, and precision outrank decoration).
   - The visual world is **pinned whole** by PRD Section 4 (`paper-ink`, `swiss-mono`, `warm-stone` tokens + `Plus Jakarta Sans` + `JetBrains Mono`). At completion, `DESIGN.md` and `.impeccable/design.json` will be generated from the built system.

---

## 4. Approaches Considered & Selected

### Approach 1 (Selected): Layered IPC Adapter + Semantic Tailwind v4 Token Bridge + Dev Preview Harness
- **Structure:**
  - `src/lib/ipc.ts` wraps `bindings.ts` to unwrap `Result<T, AppError>`, normalize `number | null` floats into guaranteed `number` domain types, classify `AppError` into `IpcError`, and broadcast global `API_KEY_MISSING` events cleanly.
  - 5 Pinia stores (`settings`, `profile`, `jobs`, `competency`, `roadmap`) own async state, loading/error flags, and cross-store invalidation (e.g., `quickAddUserSkill` automatically refreshes `profile`, `jobs`, and `competency` stores).
  - `src/styles/themes.css` defines the exact PRD Section 4.2 CSS variables and maps them to Tailwind v4 `@theme` + `shadcn-vue` semantic tokens (`--color-canvas`, `--color-surface`, `--color-elevated`, `--color-accent`, `--color-match`, `--color-gap`, `--color-critical`, `--color-adjacent`).
  - Optional browser-only preview mock (`src/lib/mockIpc.ts`) activates **strictly** when `import.meta.env.DEV && import.meta.env.VITE_MOCK_IPC === "1" && !("__TAURI_INTERNALS__" in window)`, enabling automated visual inspection in Chromium across all 3 themes & 2 viewports without affecting production or `pnpm tauri dev` (which always hits real Rust Core).
- **Trade-offs:** Adds ~150 lines for the adapter and dev-only preview fixture, but eliminates duplicated null/error boilerplate across 6 views, guarantees clean `vue-tsc` types, and enables rigorous visual + interactive verification across all 3 themes and breakpoints.

### Approach 2 (Not Selected): Direct `bindings.ts` in Pinia Stores + Tauri-Only Manual Dev
- **Structure:** Pinia stores call `commands.*` directly, each store/view handles `Result` + `?? 0` + `API_KEY_MISSING` inline, and UI is checked only manually inside `pnpm tauri dev`.
- **Why rejected:** Duplicates `number | null` and `AppError` handling across 5 stores and 6 views, increases drift risk when updating error handling, and prevents automated multi-theme / multi-viewport screenshot verification.

---

## 5. System Architecture & File Structure (`apps/client/`)

```text
apps/client/
├── index.html                        # Inline Anti-FOUC script setting documentElement.dataset.theme
├── package.json                      # Vue 3.5, Vite, TS, vue-tsc, Pinia, Vue Router, Tailwind v4, shadcn-vue, lucide-vue-next, motion-v, @formkit/auto-animate, @tauri-apps/api, @fontsource/*, vitest
├── vite.config.ts                    # Port 1420 (strictPort), @tailwindcss/vite, @ -> ./src
├── tsconfig.json                     # Strict TS + Vue bundler resolution
├── components.json                   # shadcn-vue config pointing to semantic CSS vars
├── src/
│   ├── main.ts                       # Mounts Pinia, Router, @formkit/auto-animate, local fonts, themes.css
│   ├── App.vue                       # Bootstrap gate (theme + API key + active profile) + AppShell / First-Run layout
│   ├── styles/
│   │   └── themes.css                # PRD Section 4.2 exact tokens (paper-ink, swiss-mono, warm-stone) + Tailwind v4 @theme bridge + reduced-motion
│   ├── composables/
│   │   ├── useTheme.ts               # Reactive theme state synced to dataset.theme & localStorage['vecta:theme']
│   │   └── useToast.ts               # Global toast queue with optional interactive CTA button (e.g., Roadmap auto-add skill)
│   ├── lib/
│   │   ├── bindings.ts               # Auto-generated by tauri-specta (NEVER hand-edited)
│   │   ├── ipc.ts                    # Result unwrapper, f64 null-normalizer, IpcError class, Tauri vs VITE_MOCK_IPC switch
│   │   ├── mockIpc.ts                # Dev-only realistic Bahasa Indonesia fixtures for visual audit in browser (tree-shaken out in prod)
│   │   ├── format.ts                 # Tabular numeric formatters (percent, score 0-100, hours, Indonesian relative dates)
│   │   └── utils.ts                  # cn() helper (clsx + tailwind-merge) for shadcn-vue components
│   ├── router/
│   │   └── index.ts                  # Routes: /onboarding, /dashboard, /jobs, /competency, /roadmap, /settings + smart / redirect
│   ├── stores/
│   │   ├── settings.ts               # apiKeyStatus, workerHealth, workerUrl, save/delete key, checkWorkerHealth
│   │   ├── profile.ts                # activeProfile, storyDraft, draftText (preserved on error!), parse/save/toggleKeyword/addCustomKeyword
│   │   ├── jobs.ts                   # rankedJobs, selectedJobId, selectedJobDetail, filters, scraping state, ingestManual, runScrape, toggleSave, generateGapAnalysis
│   │   ├── competency.ts             # availableRoles, selectedRole, competencyMap, categoryFilter, gapOnlyToggle
│   │   └── roadmap.ts                # roadmaps, activeRoadmapId, activeRoadmapDetail, generateRoadmap, updateStepStatus, quickAddUserSkill
│   ├── components/
│   │   ├── ui/                       # Clean shadcn-vue primitives (Button, Input, Textarea, Badge, Switch, Dialog, Tooltip, Skeleton, Select) styled purely via semantic tokens
│   │   ├── layout/
│   │   │   ├── AppShell.vue          # Desktop >=1024px: 240px sidebar + canvas; Mobile <1024px: 52px top bar (brand + ThemeSwitcher + Settings/key dot) + canvas + 5-icon bottom nav + API-key reminder banner + ToastContainer
│   │   │   ├── AppSidebar.vue        # Brand mark, 5 nav items, compact 3-swatch ThemeSwitcher, Settings link + key/worker status dot
│   │   │   ├── MobileBottomNav.vue   # 5-icon clean bottom bar (<1024px)
│   │   │   ├── ThemeSwitcher.vue     # Supports variant="compact" (sidebar/mobile header swatches) and variant="cards" (Settings visual mini previews)
│   │   │   └── ToastContainer.vue    # Top-right / bottom-right accessible toast stack with interactive action button
│   │   ├── profile/
│   │   │   ├── SkillChip.vue         # Neutral --bg-elevated chip, 6px proficiency/status dot, 1-click level cycle, evidence tooltip, remove 'x'
│   │   │   └── KeywordChipGroup.vue  # Supports mode="draft" (unsaved onboarding) and mode="live" (immediate toggleScrapeKeyword / addCustomKeyword with 0 Gemini calls)
│   │   ├── jobs/
│   │   │   ├── JobCard.vue           # Compact master-list row: tabular score badge, title, company, work_mode, count-based status dots (matched/missing counts from JobMatchSummary), bookmark icon
│   │   │   ├── MatchRing.vue         # SVG circular gauge animating stroke-dashoffset (380ms --ease-vecta) + tabular-nums count-up
│   │   │   ├── GapAnalysisPanel.vue  # Cached/on-demand Contract B panel: verdict headline, strengths, prioritized gaps (high/medium/low), CV tailoring tips, Refresh button
│   │   │   ├── ManualJobModal.vue    # Accessible dialog for pasting job description (min 30 chars, max 12,000 live counter) + optional title/URL
│   │   │   └── ScrapeKeywordsPopover.vue # Compact popover on JobsFeedView header wrapping KeywordChipGroup in mode="live" for 0-Gemini-call keyword toggling & custom role additions
│   │   ├── competency/
│   │   │   └── SkillDemandRow.vue    # Skill name, cluster badge, horizontal percentage bar, tabular % + job count, owned vs gap status indicator
│   │   └── roadmap/
│   │       └── RoadmapTimeline.vue   # 3-phase vertical timeline (Fundamental -> Praktik -> Proyek Mini) with spring check, copyable learning keywords, mini project box, estimated hours
│   └── views/
│       ├── OnboardingStoryView.vue   # Guided 2-step first-run (API Key -> Story-to-Chips) OR direct Profile Story editor
│       ├── DashboardView.vue         # 3 KPI cards + 2-column Top 5 Matches & Next Learning Steps
│       ├── JobsFeedView.vue          # Header filters/actions + 380px Master-Detail Split-Pane (stacked detail w/ back button on Mobile)
│       ├── CompetencyMapView.vue     # Role picker + 4-cluster filter + "Hanya Tampilkan Gap" FLIP toggle + CTA to generate roadmap
│       ├── RoadmapView.vue           # Roadmap switcher + active RoadmapTimeline + inline "Buat Roadmap Baru" trigger
│       └── SettingsView.vue          # Single-column max-w-2xl: Gemini BYOK card, 3-Theme preview cards, Python Worker URL & health check
└── src-tauri/
    ├── Cargo.toml                    # Add official tauri-plugin-opener (0 changes to the 22 custom IPC commands)
    ├── src/lib.rs                    # Register .plugin(tauri_plugin_opener::init())
    ├── tauri.conf.json               # Add devUrl, beforeDevCommand, beforeBuildCommand
    └── capabilities/
        └── default.json              # Tauri 2.0 core:default + scoped opener:allow-open-url (https://*)
```

---

## 6. Key User Flows & Screen Specifications

### 6.1 App Bootstrap & Guided 2-Step First-Run Flow
1. `index.html` inline `<script>` sets `document.documentElement.dataset.theme` from `localStorage['vecta:theme']` (fallback `'paper-ink'`) synchronously before any CSS/JS executes (**Zero-FOUC**).
2. `App.vue` bootstraps `settingsStore.loadApiKeyStatus()` and `profileStore.loadActiveProfile()` in parallel.
3. **Smart entry routing (`/`):**
   - If `!hasKey && !activeProfile && !sessionStorage.getItem('vecta:skipped_onboarding')` → redirect to `/onboarding` in **Guided First-Run Mode** (clean centered layout without sidebar clutter, top bar with Vecta wordmark and `"Lewati ke Dashboard"` button).
     - **Step 1 — Hubungkan Gemini API Key:** Single focused card explaining BYOK local keychain privacy, link to Google AI Studio, password input, `"Tes & Simpan Key"` CTA. Upon valid key confirmation, smoothly advances to Step 2.
     - **Step 2 — Ceritakan Profilmu (Story-to-Chips):** Centered `max-w-3xl` writing canvas with 3 one-click preset story buttons (*Software Dev*, *Data Analyst*, *Hybrid Tech*), live character count (`minimal 30 karakter`), `"Rapikan & Petakan Profil"` CTA → staggered `30ms` reveal of summary, `SkillChip` group (`v-auto-animate`, 1-click level cycle, add custom skill), and `KeywordChipGroup` (`Primary Fit` / `Adjacent Fit` / custom Enter add) → `"Simpan Profil & Cari Lowongan"` CTA saves atomically and routes straight to `/jobs`.
   - Clicking `"Lewati ke Dashboard"` sets `sessionStorage['vecta:skipped_onboarding'] = '1'` and routes to `/dashboard` (where a calm top banner reminds the user to configure their API key).
   - Returning users (`hasKey || activeProfile`) land directly on `/dashboard`. Visiting `/onboarding` via sidebar `"Profil Cerita"` opens the Story-to-Chips editor inside `AppShell` pre-filled with the active profile's raw story, skills, and keywords.
   - **Live Keyword Persistence vs Draft Editing (Profile State Machine):**
     - **First-time unsaved profile (`activeProfile === null`):** `KeywordChipGroup` runs in `mode="draft"`; toggling or adding custom keywords mutates local draft state and persists atomically on `"Simpan Profil & Cari Lowongan"` (`saveConfirmedProfile`).
     - **Existing saved profile (`activeProfile !== null`):**
       - Toggling existing saved keywords (`is_active`) or adding new `custom` keywords calls `toggleScrapeKeyword` / `addCustomKeyword` immediately (**0 Gemini calls**, preserving `last_scraped_at`), available both in `OnboardingStoryView` (`mode="live"`) and inside the quick `"Keyword"` popover on `JobsFeedView`.
       - Editing raw story text, summary, experience level, or skills marks the profile draft dirty and saves explicitly via `"Simpan Perubahan Profil"` (`saveConfirmedProfile`), which re-embeds the profile and recalculates matches.
       - Re-running `"Rapikan & Petakan Profil"` (`parseUserStory`) on an existing profile replaces AI-suggested `primary_fit`/`adjacent_fit` drafts while **preserving and merging** any existing `fit_type === 'custom'` keywords (deduplicated case-insensitively by `role_label`) so user-added roles are never silently wiped.

### 6.1.1 Responsive Mobile Chrome (`< 1024px`, Builder-Check Call)
- Below `1024px`, `AppSidebar` hides and `AppShell` renders:
  - A compact **`52px` sticky mobile top bar** (`bg-[var(--bg-surface)]`, hairline bottom border) containing the Vecta brand mark on the left and compact 3-swatch `ThemeSwitcher` + Settings icon button (carrying the API key status dot) on the right.
  - Scrollable main canvas (`pb-20` to clear the bottom bar).
  - Fixed `MobileBottomNav` with 5 clean icon tabs (`Dashboard`, `Profil`, `Lowongan`, `Peta`, `Roadmap`).
- This ensures `/settings`, `ThemeSwitcher`, and key status remain reachable at `390×844` without crowding or dropping any of the 5 PRD bottom-nav items.

### 6.2 `DashboardView` (`/dashboard`)
- **Top row (3 compact KPI cards):**
  1. `Lowongan Cocok (≥ 75%)` — count in `JetBrains Mono tabular-nums` + total lowongan tersimpan.
  2. `Progres Roadmap` — active roadmap percentage + completed/total steps.
  3. `Total Skill Terverifikasi` — count from `activeProfile.skills.length`.
- **Bottom 2-column layout (single column on `< 1024px`):**
  - Left: **Top 5 Lowongan Paling Cocok** (clicking navigates to `/jobs?jobId=<id>`).
  - Right: **Langkah Belajar Berikutnya** (up to 4 next `todo`/`in_progress` steps from the latest roadmap, with 1-click jump to `/roadmap`).
- **Empty states:** Actionable 1-click CTAs (`"Mulai Ceritakan Profilmu"` if no profile; `"Ambil Lowongan Pertama"` / `"Tempel Lowongan Manual"` if 0 jobs).

### 6.3 `JobsFeedView` (`/jobs`)
- **Top action bar:** Role filter dropdown (`Semua Peran` + `availableRoles`), search input (title/company), `"Tersimpan"` filter chip, `"Keyword"` popover (`ScrapeKeywordsPopover` — shows active count badge and lets users toggle/add scrape keywords immediately with 0 Gemini calls), `"+ Tempel Manual"` button (opens `ManualJobModal`), and `"Refresh Lowongan"` split-control (with source toggles `LinkedIn`, `Glints`, `JobStreet` and thin top progress bar during scrape while keeping existing list readable).
- **Desktop Split-Pane (`>= 1024px`):** Left `380px` scrollable `JobCard` list sorted by `final_score DESC`; right fluid detail pane. Each `JobCard` renders strictly from `JobMatchSummary` (1 fast query, zero N+1): tabular score badge, title, company, work mode badge, compact count-based status dot indicators (`● {matched_skills_count} cocok · ● {missing_skills_count} gap` in `tabular-nums`, builder-check call), and bookmark toggle icon. Full named skill chips render in the right `JobMatchDetail` pane.
- **Mobile (`< 1024px`):** Full-width list; tapping a `JobCard` shows the detail pane with a sticky `"← Kembali ke Daftar"` bar offset cleanly below the `52px` mobile top bar (`top-[52px]`).
- **Detail Pane Contents:**
  - Header: `MatchRing` (`380ms --ease-vecta` SVG stroke + count-up `tabular-nums` percentage), title, company, location, work mode & experience level badges, bookmark toggle (`toggleSaveJob`), external job link (if URL exists).
  - Sub-score micro-bar row: `Vektor (40%)`, `Skill (45%)`, `Level (15%)` in `tabular-nums`.
  - Skill Breakdown: Neutral `SkillChip` lists separated into **Skill Cocok** (`--status-match` dot) and **Skill Belum Dimiliki** (`--status-gap` for nice-to-have, `--status-critical` for mandatory).
  - `GapAnalysisPanel`: If `ai_gap_analysis` is cached in SQLite, renders immediately (badge `"Tersimpan di cache lokal"` + `"Analisis Ulang"` button); if `null`, shows clean prompt card with `"Analisis Gap Mendalam"` CTA (`generateGapAnalysis`).
  - Action row: `"Buat Roadmap dari Lowongan Ini"` with inline hours/week selector (`4 / 6 / 10 jam/minggu`) → calls `generateLearningRoadmap` and navigates to `/roadmap`.
  - Cleaned Job Description collapsible/readable section (`max-w-[70ch]`).
- **Partial-failure & offline resilience:** If worker returns warnings (e.g. Glints `RATE_LIMITED` while LinkedIn succeeded), display a dismissible warning toast/banner listing partial warnings while still refreshing the newly ingested jobs. If `SCRAPER_UNREACHABLE`, show inline helper banner with copyable `uv run uvicorn app.main:app --host 127.0.0.1 --port 8000` command.

### 6.4 `CompetencyMapView` (`/competency`)
- **Header:** Role selector dropdown (`listAvailableRoles` with job count per role) + `"Buat Roadmap dari Gap Peran Ini"` primary CTA (with inline hours/week popover; gracefully disabled with tooltip if user has 0 missing skills for the role).
- **Controls row:** 4-cluster segmented filter (`Semua`, `Core Hard`, `Tools & Tech`, `Soft Skill`, `Domain`) + `"Hanya Tampilkan Gap"` toggle switch.
- **Advisory banner:** Shown when `map.is_small_sample === true` (`total_jobs < 5`): *"Sampel kecil ({total_jobs} lowongan): akurasi peta akan meningkat setelah lebih banyak lowongan masuk."*
- **Skill Demand List (`v-auto-animate`):** Ordered by `demand_percentage DESC`, rendering `SkillDemandRow` with horizontal fill bar (`--status-match` if owned, `--status-gap` if missing), tabular `%` + `{demand_count}/{total_jobs} lowongan`, and proficiency pill if owned.

### 6.5 `RoadmapView` (`/roadmap`)
- **Header & Selector:** Saved roadmaps switcher (dropdown/pills showing title, target role, and `tabular-nums` completion `%`) + `"Buat Roadmap Baru"` inline popover (pick role from `availableRoles` + `hours_per_week`).
- **Summary Strip:** Roadmap title, target role badge, estimated weeks, overall progress bar (`completed_steps / total_steps`).
- **`RoadmapTimeline`:** Vertical 3-phase layout (`Fase 1: Fundamental`, `Fase 2: Praktik`, `Fase 3: Proyek Mini`) connected by a vertical line that fills with `--accent-primary` as steps complete.
  - Each step shows interactive checkbox/status (`todo` → `in_progress` → `done` with spring micro-check), `topic_title`, `skill_name` neutral pill, `estimated_hours` in `JetBrains Mono`, copyable `learning_keywords` chips (clicking copies keyword + brief feedback), and `mini_project_idea` callout.
  - **Auto-Sync Loop (FR-07):** When marking a step `done` returns `suggest_add_skill: SkillRef`, show an interactive toast: *"Selesai belajar {canonical_name}! Tambahkan ke profilmu?"* with button `"Tambahkan ke Profil"`. Clicking executes `quickAddUserSkill(userId, skillId)` (0 Gemini calls), refreshes `profile`, `jobs`, and `competency` stores, and shows confirmation toast *"Skill ditambahkan & skor lowongan diperbarui."*

### 6.6 `SettingsView` (`/settings`)
- Single calm column (`max-w-2xl`) with 3 cards:
  1. **Gemini API Key (BYOK):** Masked status badge (`AIza...9xQp` + `"Terhubung"` dot) or empty state with link to Google AI Studio (`https://aistudio.google.com/apikey`), password input with show/hide toggle, `"Tes & Simpan"` button with inline spinner, and `"Hapus Kunci"` destructive secondary button.
  2. **Tema Antarmuka:** `ThemeSwitcher` (`variant="cards"`) showing live-styled mini UI previews of `paper-ink`, `swiss-mono`, and `warm-stone` with instant zero-reload switching.
  3. **Python Scraper Worker:** Worker URL input (default `http://127.0.0.1:8000`), `"Cek Koneksi"` button calling `checkWorkerHealth(url)` (which persists the verified URL in Rust `AppState` for scraping), status indicator (`Online v0.1.0` + available sources chips, or offline guide with copyable `uv run uvicorn app.main:app --host 127.0.0.1 --port 8000`).

---

## 7. Cross-Store Data Flow & Error Handling

| Error Code | Global / Local Handling |
| --- | --- |
| `API_KEY_MISSING` | Opens/highlights calm banner with 1-click button to `/settings` (or Step 1 if in onboarding). |
| `API_KEY_INVALID` | Inline error on Settings key input or banner guiding user to verify key in Google AI Studio. |
| `GEMINI_RATE_LIMITED` | Recoverable alert/toast with `"Coba Lagi"` action; never clears user input. |
| `GEMINI_PARSE_FAILED` | Recoverable alert with `"Ulangi"` action; preserves draft text. |
| `SCRAPER_UNREACHABLE` | Inline worker startup guide (`uv run uvicorn ...`) + `"Cek Ulang"` button; manual paste stays usable. |
| `DB_ERROR` | Friendly Bahasa Indonesia error toast; never crashes view. |
| `VALIDATION_ERROR` | Highlights relevant form field (e.g. story `< 30` chars, empty keyword, 0 missing skills for roadmap). |

---

## 8. Decisions Left to the Builder

- Exact component prop ergonomics inside `src/components/ui/` (standard `shadcn-vue` Radix/Reka-UI primitives styled via semantic CSS tokens).
- Exact Vitest mock setup for Pinia store and IPC adapter unit tests.
- Exact breakpoint CSS grid column widths inside `DashboardView` and `CompetencyMapView` as long as PRD Section 4.1 (`max 2 main columns on desktop`, calm single focus, `240px` sidebar, `380px` master list) is preserved.
- Exact micro-copy wording in Indonesian helper subtitles and the 3 sample story presets (realistic Indonesian tech career stories of 250–400 chars covering Software Dev, Data Analyst, and Hybrid Tech).

---

## 9. Verification & Definition of Done

1. `pnpm typecheck` (`vue-tsc --noEmit`) passes with **0 errors**.
2. `pnpm test` (Vitest unit tests for `ipc.ts`, `useTheme.ts`, `format.ts`, and core Pinia stores) passes **100%**.
3. `pnpm build` succeeds cleanly producing `apps/client/dist/`.
4. Rust `cargo test` (52 existing tests) + `cargo build` continue to pass with **0 warnings** after `tauri.conf.json` & `capabilities/default.json` updates.
5. Live visual & interactive verification across **all 3 themes** (`paper-ink`, `swiss-mono`, `warm-stone`) at both **Desktop (`1440×900`)** and **Mobile (`390×844`)** viewports, plus `impeccable detect --json`, `impeccable-finish-reviewer`, and `DESIGN.md` / `.impeccable/design.json` documentation generation.
6. Smoke-verified launch on Windows Desktop via `pnpm tauri dev`.
