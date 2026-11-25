# Vecta Comprehensive System, UI/UX, and Cybersecurity Audit Report

**Tanggal Audit:** 2026-11-25  
**Cakupan:** Seluruh stack Vecta (`apps/client` Vue 3 + TypeScript + Tailwind v4, `apps/client/src-tauri` Rust Core + SQLite WAL + OS Keychain + Tauri 2.0, `services/scraper-worker` Python 3.13 FastAPI).  
**Branch:** `Development`  
**Acuan Desain & Produk:** `VECTA_DESIGN_SYSTEM.md` & `product_requirements_document_prd_vecta.md` (PRD v2.0)

---

## 1. Ringkasan Eksekutif & Status Pengujian

Seluruh lapisan aplikasi (Frontend Vue 3, Rust Core + Tauri 2.0 desktop binary, dan Python Scraper Worker) telah selesai dibangun, diuji secara otomatis, dan diaudit secara visual serta keamanan kode sumber.

| Lapisan | Perintah Verifikasi | Hasil | Catatan |
| --- | --- | --- | --- |
| **Frontend Type Safety** | `npm run typecheck` (`vue-tsc --noEmit`) | **PASS (`0` error)** | Strict TS aktif, guard `number \| null` untuk ekspor `f64` Rust |
| **Frontend Logic & Store Tests** | `npm test` (`vitest run`) | **PASS (`8/8` test)** | Mencakup IPC unwrapper, null-float guard, URL scheme guard, dan 5 Pinia store end-to-end |
| **Frontend Production Bundle** | `npm run build` (`vite build`) | **PASS (`1.4s`)** | `dist/` ter-build bersih dengan font lokal `Satoshi` (`400/500/700`), CSS `8.05 kB` gzip, JS `65.16 kB` gzip |
| **UI Slop & Pattern Detector** | `impeccable detect --json apps/client/src` | **PASS (`0` temuan)** | `RC=0 []` |
| **Rust Core & Tauri 2.0 Build** | `cargo test && cargo build` | **PASS (`52/52` test, `0` warning)** | Termasuk performa matching 1.000 lowongan `< 100ms`, transaksi atomik, dan plugin `tauri-plugin-opener` |
| **Python Scraper Worker** | `uv run pytest -v` | **PASS (`14/14` test)** | Cleaner HTML, deterministik SHA-256 ID, dedup, zero-key verification, 3 portal adapter |
| **Audit Visual Live App** | 8 tangkapan layar Chromium (`1440x900`, `1024x768`, `390x844`) | **PASS** | Seluruh 6 layar (`Dashboard`, `Profil Cerita`, `Lowongan`, `Peta Kompetensi`, `Roadmap`, `Pengaturan`) lolos inspeksi visual |

---

## 2. Audit Sistem & Logika Bisnis

### 2.1 Yang Sudah Berjalan Sesuai Kontrak
- **Batas 22 IPC Command Terjaga Ketat:** Seluruh komunikasi frontend menuju Rust melewati `apps/client/src/lib/ipc.ts` yang membungkus `apps/client/src/lib/bindings.ts` tanpa panggilan langsung ke Gemini, Python Worker, maupun SQLite.
- **Closed-Loop Career Intelligence Terverifikasi:** Alur `Profil Cerita -> Simpan Profil -> Ranking Lowongan Lokal -> Analisis Gap Ber-cache -> Peta Kompetensi -> Roadmap 3 Fase -> Selesai Langkah -> Tambah Skill Lokal (0 token Gemini) -> Hitung Ulang Skor` teruji di kedua sisi (`cargo test` dan `src/stores/stores.test.ts`).
- **Isolasi Error & Ketahanan Partial Failure:** Semua 7 kode error (`API_KEY_MISSING`, `API_KEY_INVALID`, `GEMINI_RATE_LIMITED`, `GEMINI_PARSE_FAILED`, `SCRAPER_UNREACHABLE`, `DB_ERROR`, `VALIDATION_ERROR`) dipetakan ke pesan Bahasa Indonesia; input cerita pengguna dan teks lowongan manual tidak hilang saat terjadi error jaringan/kuota.

### 2.2 Temuan Logika untuk Perbaikan Ke Depan

| ID | Prioritas | Lokasi | Temuan | Rekomendasi Perbaikan |
| --- | --- | --- | --- | --- |
| **SYS-01** | **Medium** | `apps/client/src-tauri/src/state.rs:27,52-55` | `worker_url` hanya disimpan di `RwLock<String>` in-memory. Jika pengguna mengubah URL Python Worker di `SettingsView`, nilai akan kembali ke `http://127.0.0.1:8000` saat aplikasi dibuka ulang. | Tambahkan tabel key-value `app_settings` di migrasi SQLite `002_app_settings.sql` untuk menyimpan preferensi non-rahasia (`worker_url`, `hours_per_week` default). |
| **SYS-02** | **Medium** | `apps/client/src/views/JobsFeedView.vue` & `core/db.rs:862` | Backend sudah menyediakan filter `search_query`, `work_mode`, `source`, `min_score`, dan `offset/limit`, tetapi UI `JobsFeedView` baru mengekspos filter `normalized_role` dan `saved_only` dengan limit statis `100`. | Tambahkan input pencarian ringan di atas rail `Lowongan`, filter `work_mode` (`Remote`/`Hybrid`/`Onsite`), dan tombol "Muat lebih banyak" bila jumlah lowongan melampaui `100`. |
| **SYS-03** | **Low** | `apps/client/src/views/RoadmapView.vue` | Backend mendukung 3 status langkah (`todo`, `in_progress`, `done`), sedangkan UI baru memakai checkbox toggle `todo <-> done`. | Tambahkan kontrol status tengah `Sedang dipelajari` (`in_progress`) atau klik berputar agar progres belajar jangka panjang lebih akurat. |
| **SYS-04** | **Low** | `apps/client/src-tauri/src/commands/roadmap.rs:63` | Pesan error validasi masih memuat satu karakter em-dash (`—`) pada string bahasa Indonesia. | Ganti dengan koma atau titik dua agar 100% selaras dengan pedoman copywriting aplikasi. |
| **SYS-05** | **Low** | `apps/client/src/views/SettingsView.vue:19-22` | Status Python Scraper Worker baru tampil "Belum dicek / Offline" sampai pengguna menekan tombol `Cek Koneksi` secara manual. | Pertimbangkan auto-ping ringan `checkWorkerHealth()` non-blocking saat `SettingsView` dibuka (timeout 2 detik) agar status online langsung terbaca. |

---

## 3. Audit UI/UX & Kesesuaian `VECTA_DESIGN_SYSTEM.md`

### 3.1 Perbaikan UI yang Langsung Diterapkan Saat Audit Live
Saat menginspeksi 8 tangkapan layar aplikasi Vue 3 yang benar-benar berjalan (`1440x900`, `1024x768`, `390x844`), ditemukan 3 defect halus dan langsung diperbaiki:
1. **Hilangnya Bullet List karena Tailwind v4 Preflight (`GapAnalysisCard.vue`):** Preflight Tailwind menyetel `ul { list-style: none }`, membuat daftar `Kekuatan Utama` dan `Penyesuaian CV` tampil menjorok tanpa titik. **Diperbaiki:** menambahkan `list-style-type: disc;` pada `.v-clean-list`.
2. **Penumpukan Tombol Header pada Mobile `390px` (`PageHeader.vue`):** Tombol `+ Tempel Manual` dan `Refresh Lowongan` sebelumnya tersusun vertikal di sisi kanan judul. **Diperbaiki:** pada `< 1024px`, `PageHeader` menumpuk judul di atas dan merentangkan tombol aksi berdampingan rapi (`flex: 1`).
3. **Ruang Panel Detail & Filter pada Desktop Sempit `1024x768` (`JobsFeedView.vue`):** Rail kiri `400px` terlalu lebar di viewport `1024px` dan filter rail sempat meluber saat disempitkan. **Diperbaiki:** menyetel `.v-jobs-rail` menjadi `360px` di bawah `1280px`, `padding: 12px`, dan `flex-wrap: wrap` pada `.v-jobs-filters` agar seluruh kontrol filter dan panel detail tetap lega tanpa overflow.

### 3.2 Temuan UI/UX & Aksesibilitas untuk Perbaikan Ke Depan

| ID | Prioritas | Lokasi | Temuan | Rekomendasi Perbaikan |
| --- | --- | --- | --- | --- |
| **UI-01** | **Medium** | `apps/client/src/styles/tokens.css:54,59` & `VECTA_DESIGN_SYSTEM.md` Sec 12 | Pengukuran kontras WCAG 2.1 menunjukkan 2 token asli Section 12 berada di bawah ambang AA jika dipakai pada konten aktif: `--text-muted: #68788F` (`3.51:1` di `#1A2333`) dan `--border-default: #2C3A50` (`1.37:1` pada batas form input vs ambang WCAG 1.4.11 `3.0:1`). Saat ini teks penting sudah dimitigasi memakai `--text-secondary: #98A6BA` (`6.38:1`). | Pertimbangkan menaikkan `--text-muted` ke `#8293AB` (`>= 5.04:1`) dan menambah token khusus `--border-input: #5D708E` (`>= 3.13:1`) untuk batas kotak input/checkbox di revisi design system berikutnya. |
| **UI-02** | **Medium** | `apps/client/src/components/ui/VDialog.vue` | Dialog sudah menutup dengan `Escape` dan backdrop click, namun belum menerapkan focus trap penuh (`Tab` masih bisa keluar ke elemen di balik backdrop) dan belum mengembalikan fokus ke tombol pemicu saat ditutup. | Implementasikan focus trap (`inert` pada `#app` sibling atau helper fokus) dan simpan `document.activeElement` sebelum modal dibuka untuk dipulihkan saat `close`. |
| **UI-03** | **Low** | `apps/client/src/views/JobsFeedView.vue` & `CompetencyMapView.vue` | State memuat (`loadingList`, `loadingDetail`, `loading`) masih berupa teks sederhana; belum memakai skeleton terstruktur yang menjaga tinggi layout (CLS rendah). | Tambahkan komponen `VSkeleton` halus (tanpa shimmer mencolok, hormati `prefers-reduced-motion`) untuk daftar lowongan dan tabel kompetensi. |
| **UI-04** | **Low** | `apps/client/src/views/JobsFeedView.vue` | Deskripsi mentah lowongan (`raw_description`) dari hasil scrape/manual belum ditampilkan di panel detail (saat ini menampilkan ringkasan matching, skill, dan analisis AI). | Tambahkan section lipat (`<details>`) "Lihat Deskripsi Lowongan Asli" di bagian bawah panel detail agar pengguna dapat memverifikasi teks asli lowongan langsung di aplikasi. |

---

## 4. Audit Keamanan Siber (Cybersecurity Deep Dive)

### 4.1 Matriks Kontrol Keamanan yang Sudah Terbukti Kuat

| Area Ancaman | Status | Bukti Verifikasi Kode |
| --- | --- | --- |
| **SQL Injection (SQLi)** | **AMAN (0 celah injeksi)** | Seluruh query produksi di `apps/client/src-tauri/src/core/db.rs` memakai parameterized query `rusqlite` (`params![]` / `params_from_iter`). Klausa dinamis di `list_ranked_jobs` (`db.rs:843-878`) hanya menambahkan fragmen literal statis dengan placeholder `?`. Satu-satunya interpolasi nama tabel (`db.rs:1904`) berada di dalam `#[cfg(test)]`. |
| **Cross-Site Scripting (XSS)** | **AMAN di Vue & Cleaner** | Pemindaian seluruh `apps/client/src/` menunjukkan `0` penggunaan `v-html`, `innerHTML`, `outerHTML`, `document.write`, maupun `eval()`. Seluruh teks lowongan dan respons AI dirender lewat auto-escaping template Vue. Di Python (`cleaner.py:19-20`), tag `<script>`, `<style>`, `<noscript>`, `<svg>` dihapus via `BeautifulSoup`. |
| **Kebocoran Gemini API Key** | **AMAN (0 kebocoran)** | Kunci hanya disimpan di OS Credential Manager (`keyring` crate di `core/keychain.rs`), tidak pernah ditulis ke SQLite, `localStorage`, `sessionStorage`, atau `console.*`. Permintaan HTTP ke Gemini memakai header `x-goog-api-key` (bukan query parameter URL; teruji di `http_client_uses_header_not_query_param`). `Debug` trait menyembunyikan secret, UI hanya menerima suffix ter-mask (`AIza...9xQp`), dan Python worker terverifikasi bebas referensi API key (`test_worker_contains_no_gemini_api_key_references`). |
| **URL Scheme Injection (Frontend)** | **DIPERBAIKI PADA SESI INI** | `openExternalUrl` di `apps/client/src/lib/ipc.ts:348-364` kini memvalidasi `URL.protocol` ketat (`https:` / `http:` saja) sebelum memanggil `openUrl` / `window.open` (teruji di `ipc.test.ts`), melengkapi izin Tauri `opener:allow-open-url` (`https://*`) di `capabilities/default.json`. |
| **Memory Safety (Rust)** | **AMAN** | `0` blok `unsafe` di seluruh `apps/client/src-tauri/src/`. Seluruh `.unwrap()` hanya ada di modul uji `#[cfg(test)]`. |

### 4.2 Celah & Risiko Keamanan yang Ditemukan (Untuk Perbaikan Ke Depan)

#### `[HIGH] SEC-01` Content Security Policy (CSP) WebView Dinonaktifkan
- **Lokasi:** `apps/client/src-tauri/tauri.conf.json:20` (`"csp": null`)
- **Analisis Risiko:** Tanpa CSP di Tauri WebView, aplikasi tidak memiliki pertahanan lapis kedua (defense-in-depth). Jika di masa depan terdapat celah XSS pada dependency pihak ketiga atau komponen baru, skrip eksternal dapat dieksekusi bebas dan memanggil IPC Tauri secara langsung.
- **Rekomendasi Perbaikan:** Aktifkan kebijakan CSP ketat di `tauri.conf.json`:
  ```json
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"
  ```
  *(Catatan: pindahkan inline style pada `index.html` ke stylesheet agar nantinya `'unsafe-inline'` pada `style-src` pun dapat diperketat bila diinginkan.)*

#### `[HIGH] SEC-02` Risiko SSRF & HTTP Redirect pada Konfigurasi `worker_url`
- **Lokasi:** `apps/client/src-tauri/src/commands/settings.rs:43-62`, `apps/client/src-tauri/src/core/worker.rs:61-68,78-115`
- **Analisis Risiko:** Validasi `check_worker_health_impl` hanya memeriksa apakah URL diawali `http://` atau `https://`. Pengguna atau proses yang memanggil IPC dapat mengarahkan Rust Core untuk mengirim permintaan `GET /health` dan `POST /api/v1/scrape` ke alamat jaringan internal non-loopback (misal metadata cloud `169.254.169.254` atau layanan admin LAN). Selain itu, `reqwest::Client` secara default mengikuti hingga 10 HTTP redirect (`301/302/307`), sehingga endpoint yang berkompromi dapat mengalihkan request ke target lain.
- **Rekomendasi Perbaikan:**
  1. Parsing URL menggunakan crate `url` dan batasi host default hanya ke loopback (`127.0.0.1`, `localhost`, `::1`). Jika remote worker suatu saat diperlukan, wajibkan opt-in eksplisit dan wajib HTTPS.
  2. Nonaktifkan redirect otomatis pada `HttpWorkerClient` (`Client::builder().redirect(reqwest::redirect::Policy::none())`).

#### `[MEDIUM] SEC-03` Permukaan Prompt Injection dari Konten Lowongan Eksternal
- **Lokasi:** `apps/client/src-tauri/src/core/gemini.rs:345-350` (Kontrak A `extract_job` dan Kontrak B `analyze_gap`)
- **Analisis Risiko:** Deskripsi lowongan hasil scraping dari portal publik (LinkedIn, Glints, JobStreet) adalah input yang tidak tepercaya (*untrusted third-party text*). Saat ini teks tersebut langsung disambungkan ke dalam satu string prompt `user` bersama instruksi sistem. Deskripsi yang mengandung instruksi manipulatif berpotensi meracuni hasil ekstraksi skill, ringkasan peran, atau saran CV.
- **Rekomendasi Perbaikan:**
  1. Pindahkan instruksi peran dan aturan ekstraksi ke field `systemInstruction` resmi milik Gemini API, bukan digabung di `contents[0].parts`.
  2. Bungkus data lowongan dengan tag delimiter eksplisit (misal `<job_description_data>...</job_description_data>`) disertai instruksi tegas untuk mengabaikan perintah apa pun di dalam blok data tersebut.
  3. Tambahkan sanitasi panjang/jumlah item pasca-parsing JSON di Rust (misal batas maksimal jumlah skill per lowongan dan panjang field string).

#### `[MEDIUM] SEC-04` Validasi Skema URL Belum Diterapkan di Layer Backend (Defense-in-Depth)
- **Lokasi:** `apps/client/src-tauri/src/commands/jobs.rs:57-61`, `services/scraper-worker/app/scrapers/linkedin_guest.py:67`
- **Analisis Risiko:** Meski layer UI (`ipc.ts:openExternalUrl`) dan Tauri capability (`capabilities/default.json`) sudah membatasi pembukaan URL, backend Rust (`ingest_manual_job`) dan scraper LinkedIn masih menerima dan menyimpan string `url` apa adanya ke kolom `jobs.url` tanpa memverifikasi skema `https://` / `http://`.
- **Rekomendasi Perbaikan:** Tambahkan fungsi `sanitize_external_url(url: Option<&str>) -> Option<String>` di Rust (`commands/jobs.rs`) dan di Python (`cleaner.py`) yang hanya meloloskan skema `http`/`https` dengan host valid dan membuang URL tak sah atau berisiko.

#### `[MEDIUM] SEC-05` Amplifikasi Request & Eksposur Exception Mentah di Python Scraper Worker
- **Lokasi:** `services/scraper-worker/app/pipeline/schemas.py:15-18`, `services/scraper-worker/app/main.py:83-90`
- **Analisis Risiko:**
  1. `ScrapeRequest.queries` hanya dibatasi `min_length=1` tanpa `max_length` dan tanpa batas panjang per query. Jika worker tidak sengaja dijalankan dengan `--host 0.0.0.0`, satu request POST berisi ratusan query dapat memicu amplifikasi scraping masif (DoS lokal / pemblokiran IP oleh LinkedIn/Glints/JobStreet).
  2. Blok `except Exception as exc` pada `main.py:88` mengembalikan teks exception mentah `{exc}` ke response JSON, yang berpotensi membocorkan path internal atau detail library.
- **Rekomendasi Perbaikan:**
  1. Tambahkan batas tegas pada `ScrapeRequest`: `queries: list[str] = Field(min_length=1, max_length=15)` dengan panjang per query `<= 120` karakter, serta `location: str = Field(default="Indonesia", max_length=80)`.
  2. Tambahkan middleware/konfigurasi agar worker default selalu bind ke `127.0.0.1` dan ganti `{exc}` di respons publik dengan pesan generik sambil mencatat detail exception ke log lokal saja.

#### `[MEDIUM] SEC-06` Detail Error Internal SQLite & OS Keychain Terekspose di `AppError.message`
- **Lokasi:** `apps/client/src-tauri/src/core/db.rs:99-101`, `apps/client/src-tauri/src/core/keychain.rs:22,39-41,51-55`
- **Analisis Risiko:** Fungsi `db_err` meneruskan string `rusqlite::Error` dan `keyring::Error` mentah langsung ke `AppError.message` yang ditampilkan di UI. Pada desktop lokal dampaknya moderat, namun memaparkan detail skema tabel atau error OS internal ke antarmuka pengguna bukan praktik terbaik.
- **Rekomendasi Perbaikan:** Catat error teknis lengkap ke logger internal Rust (tanpa pernah mencatat secret/API key) dan kembalikan pesan ringkas yang aman bagi pengguna di `AppError.message`.

#### `[LOW] SEC-07` Karakter Wildcard SQL `LIKE` (`%` dan `_`) Belum Di-escape
- **Lokasi:** `apps/client/src-tauri/src/core/db.rs:862-867`
- **Analisis Risiko:** Query sudah aman dari SQL Injection karena parameterized, tetapi input `search_query` yang mengandung `%` atau `_` akan diinterpretasikan sebagai wildcard SQLite, yang bisa menghasilkan pencocokan berlebih saat fitur pencarian UI diaktifkan.
- **Rekomendasi Perbaikan:** Escape `\`, `%`, dan `_` pada string pencarian dan tambahkan `ESCAPE '\'` pada klausa `LIKE`.

#### `[LOW] SEC-08` Kerentanan pada `devDependencies` Node.js (`npm audit`)
- **Lokasi:** `apps/client/package.json` (`vitest` -> `tinypool`, `@vitest/mocker`; `vue-tsc` -> `@vue/language-core` -> `minimatch`)
- **Analisis Risiko:** `npm audit` melaporkan 6 temuan (2 critical pada `tinypool`/`vitest`, 4 moderate pada `@vitest/mocker`/`minimatch`). Seluruhnya berada di **`devDependencies`** (alat build/test lokal, **0** paket rentan yang masuk ke production bundle `dist/` maupun binary Tauri). Meski tidak berdampak ke pengguna akhir, tetap berisiko di lingkungan developer/CI.
- **Rekomendasi Perbaikan:** Upgrade `vitest` ke versi `^3.2.x` / `^4.x` terbaru yang sudah mem-patch `tinypool` & `@vitest/mocker`, serta perbarui `vue-tsc` ke versi terbaru lalu verifikasi ulang dengan `npm audit`.

---

## 5. Peta Jalan Perbaikan Prioritas (Hardening Backlog)

| Prioritas | Target Sprint | Daftar Item |
| --- | --- | --- |
| **P0 (Wajib Sebelum Rilis Publik)** | Hardening Keamanan Inti | `SEC-01` (Aktifkan CSP ketat di `tauri.conf.json`), `SEC-02` (Batasi `worker_url` ke loopback + matikan redirect), `SEC-08` (Upgrade `vitest`/`vue-tsc` agar `npm audit` bersih). |
| **P1 (Kualitas & Pertahanan Lapis Kedua)** | Hardening Backend & UX | `SEC-03` (`systemInstruction` + XML delimiter prompt Gemini), `SEC-04` (Validasi skema URL di Rust & Python), `SEC-05` (Batas `max_length` & sanitasi exception FastAPI), `SYS-01` (Persist `worker_url` di SQLite), `UI-02` (Focus trap penuh `VDialog`). |
| **P2 (Polish & Fitur Pendukung)** | Penyempurnaan Harian | `SEC-06` (Pisahkan log error internal vs pesan UI), `SEC-07` (`ESCAPE '\'` untuk SQL `LIKE`), `SYS-02` (Search box & filter `work_mode` di `JobsFeedView`), `SYS-03` (Status `in_progress` Roadmap), `SYS-04` (Hapus em-dash di `roadmap.rs:63`), `UI-01` (Kalibrasi kontras `--text-muted` & `--border-input`), `UI-03` (Skeleton loading), `UI-04` (Drawer deskripsi asli lowongan). |

---

## 6. Log Remediasi & Verifikasi Pasca-Perbaikan (2026-11-25)

Seluruh temuan **P0, P1, dan P2** dari audit di atas telah dieksekusi dan diverifikasi ulang:

| ID | Status | Ringkasan Implementasi |
| --- | --- | --- |
| **SEC-01** | **RESOLVED** | CSP ketat diaktifkan di `apps/client/src-tauri/tauri.conf.json` (`script-src 'self'`, `object-src 'none'`, `frame-ancestors 'none'`, `connect-src 'self' ipc: http://ipc.localhost https://ipc.localhost`). |
| **SEC-02** | **RESOLVED** | `validate_worker_url` di `core/worker.rs` & `commands/settings.rs` membatasi host khusus loopback (`127.0.0.1`, `localhost`, `::1`), menolak kredensial/query/fragment, dan mematikan redirect otomatis (`Policy::none()`) pada `HttpWorkerClient` maupun `HttpGeminiClient`. Teruji oleh 2 unit test anti-SSRF & anti-redirect. |
| **SEC-03** | **RESOLVED** | `HttpGeminiClient::generate_structured` memisahkan instruksi sistem ke field resmi `systemInstruction`, membungkus seluruh data tak tepercaya dengan delimiter XML (`<job_description_data>`, `<user_story_data>`, dll.), serta menerapkan batas jumlah & panjang pasca-parsing JSON. |
| **SEC-04** | **RESOLVED** | `sanitize_external_url` (khusus `http`/`https` dengan host valid) diterapkan di Rust (`commands/jobs.rs` untuk manual & hasil scrape) serta di Python (`pipeline/cleaner.py` + ketiga adapter scraper). Teruji di Rust & Pytest. |
| **SEC-05** | **RESOLVED** | `ScrapeRequest` membatasi `queries` (`max_length=15`, masing-masing `<= 120` karakter), `sources` (`<= 3`), `location` (`<= 80`). Exception tak terduga di `app/main.py` dicatat ke logger lokal dan tidak lagi memaparkan `{exc}` ke response JSON. |
| **SEC-06** | **RESOLVED** | `db_err` di `core/db.rs` dan `OsKeyringStore` di `core/keychain.rs` mencatat detail error OS/SQLite ke `stderr` internal (tanpa secret) dan mengembalikan pesan UI yang ringkas serta aman. |
| **SEC-07** | **RESOLVED** | `escape_like_pattern` + klausa `ESCAPE '\'` ditambahkan pada `list_ranked_jobs` di `core/db.rs` beserta unit test pencarian literal `100%`. |
| **SEC-08** | **RESOLVED** | `vitest` (`^5.1.1`) dan `vue-tsc` (`^3.3.14`) diperbarui. `npm audit` kini melaporkan **`0 vulnerabilities`**. |
| **SYS-01** | **RESOLVED** | Migrasi `shared/migrations/002_app_settings.sql` ditambahkan; `worker_url` kini dipersistensikan di SQLite (`app_settings`) dan dimuat otomatis saat `AppState::production` dijalankan. |
| **SYS-02** | **RESOLVED** | `JobsFeedView.vue`, `stores/jobs.ts`, dan `mockIpc.ts` kini mendukung input pencarian (`search_query` debounced), filter `work_mode` (`Remote`/`Hybrid`/`Onsite`), serta tombol paginasi `Muat Lebih Banyak` (`PAGE_SIZE = 50`). |
| **SYS-03** | **RESOLVED** | `RoadmapView.vue` dan `stores/roadmap.ts` kini mendukung status tengah `in_progress` (`Mulai Belajar` / `Sedang dipelajari` / `Jeda`) di samping toggle selesai cepat. |
| **SYS-04** | **RESOLVED** | Karakter em-dash (`—`) pada `commands/roadmap.rs:63` dihapus. |
| **SYS-05** | **RESOLVED** | `SettingsView.vue` menjalankan pengecekan kesehatan Python Worker secara non-blocking saat halaman dibuka. |
| **UI-01** | **MITIGATED** | Token `--border-input: #5D708E` (kontras >= `3.1:1`) ditambahkan di `tokens.css` untuk kotak input, select, dan checkbox sembari menjaga token asli Section 12 (`--border-default`, `--text-muted`) tetap konsisten dengan `VECTA_DESIGN_SYSTEM.md`. |
| **UI-02** | **RESOLVED** | `VDialog.vue` dilengkapi focus trap penuh (`Tab`/`Shift+Tab`) dan pemulihan fokus otomatis ke elemen pemicu saat modal ditutup. |
| **UI-03** | **RESOLVED** | Komponen `VSkeleton.vue` (menghormati `prefers-reduced-motion`) ditambahkan dan digunakan pada state muat `JobsFeedView.vue` dan `CompetencyMapView.vue`. |
| **UI-04** | **RESOLVED** | Panel lipat `<details>` **"Lihat Deskripsi Lowongan Asli"** ditambahkan di bagian bawah detail `JobsFeedView.vue`. |

### Skor Verifikasi Akhir Pasca-Remediasi
- **Rust Core & Tauri 2.0 (`cargo test && cargo build`):** **PASS (`57/57` test, `0` warning)**
- **Python Scraper Worker (`uv run pytest -v`):** **PASS (`16/16` test)**
- **Frontend Unit & Store (`npm test` / Vitest 5.1.1):** **PASS (`9/9` test)**
- **Frontend Typecheck & Production Build (`vue-tsc 3.3.14 && vite build`):** **PASS (`0` error, `1.36s`)**
- **Dependency Security (`npm audit`):** **PASS (`0` vulnerabilities)**
- **UI Anti-Pattern Detector (`npx impeccable detect`):** **PASS (`0` temuan)**
