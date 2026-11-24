# Product Requirements Document (PRD): Vecta

> **Vecta** — *Local-first AI career intelligence app that turns raw skill stories into real-time competency maps, hybrid job matching, and actionable learning roadmaps.*

| Metadata | Detail | 
| ----- | ----- | 
| **Versi Dokumen** | 2.0 (Agentic-Ready Product & Engineering Blueprint) | 
| **Status** | Siap Eksekusi (Single Source of Truth untuk Antigravity Agent) | 
| **Target Platform** | Desktop (Windows / macOS) & Mobile (Android / iOS) via Tauri 2.0 | 
| **Model AI** | Bring Your Own Key (BYOK) — Google Gemini API | 
| **Tujuan Proyek** | Personal Career Intelligence Tool & Flagship Portfolio Showcase | 

## 0. Agent Execution Directives (Aturan Main Wajib untuk AI Agent)

Bagian ini adalah kontrak operasional bagi *coding agent* di Antigravity (termasuk saat menggunakan *Superpowers* / *Impeccable*). Patuhi seluruh aturan di bawah ini tanpa pengecualian:

1. **Single Source of Truth:** Dokumen `PRD.md` ini adalah acuan tertinggi untuk skema database, nama *command* IPC, struktur folder, kontrak JSON Gemini, formula *matching*, dan token desain. Jangan mengubah nama tabel/kolom atau *endpoint* secara sepihak tanpa memperbarui dokumen ini.

2. **Strict Layered Architecture:**

   * Frontend Vue 3 **dilarang** memanggil Gemini API atau Python Scraper Worker secara langsung. Seluruh orkestrasi wajib melewati **Tauri IPC Commands (Rust Core)**.

   * Frontend Vue **dilarang** mengakses SQLite secara langsung; semua operasi baca/tulis melalui `src-tauri/src/core/db.rs`.

   * Python Scraper Worker **dilarang** menyimpan atau membaca Gemini API Key. Tugasnya murni pengumpulan, pembersihan, dan deduplikasi teks lowongan.

3. **Type Safety Lintas Bahasa:**

   * Seluruh *struct* input/output Tauri IPC di Rust wajib menurunkan `serde::Serialize`, `serde::Deserialize`, dan `specta::Type` agar `tauri-specta` menghasilkan `apps/client/src/lib/bindings.ts` secara otomatis. Jangan menulis tipe IPC manual di TypeScript.

   * Seluruh *endpoint* Python Worker wajib menggunakan Pydantic v2 schema.

4. **Zero-Secrets Leakage:** Gemini API Key **harus** disimpan lewat OS Keychain (`keyring` crate di Desktop / *secure storage* di Mobile). Dilarang keras menuliskannya ke `localStorage`, SQLite, file `.env` yang ter-commit, atau mencetaknya ke `println!`/`tracing`.

5. **Anti-Slop UI Enforcement:** Dilarang menggunakan *neon glow*, gradasi warna generik, atau *hardcode* warna Tailwind (seperti `bg-slate-900` atau `text-blue-500`) pada komponen utama. Seluruh warna, radius, dan bayangan wajib menggunakan token CSS semantik dari `src/styles/themes.css` agar ketiga tema (`paper-ink`, `swiss-mono`, `warm-stone`) bekerja 100% konsisten.

6. **Verify Before Done (Disiplin TDD):** Sebelum menandai sub-tugas atau *milestone* selesai, agen wajib menjalankan unit test terkait (`cargo test`, `pnpm typecheck`, `pytest`) dan memastikan tidak ada *compiler warning* fatal atau kegagalan tes.

## 1. Ringkasan Eksekutif & Visi Produk

### 1.1 Latar Belakang Masalah

Platform pencari kerja konvensional masih bergantung pada pencarian kata kunci kaku dan formulir profil yang panjang, menimbulkan tiga friksi utama:

1. **Form Fatigue:** Pencari kerja memahami proyek yang pernah dikerjakan, tetapi kesulitan menerjemahkannya ke istilah formal HR atau bingung menentukan posisi apa saja yang relevan untuk dibidik.

2. **Buta Peta Kompetensi Pasar:** Sulit mengetahui secara objektif *skill* apa yang benar-benar paling sering diminta industri untuk suatu profesi tertentu berdasarkan data lowongan aktual.

3. **Matching Dangkal & Tanpa Arah Aksi:** Pencocokan kata kunci gagal menangkap konteks pengalaman, tidak menjelaskan *skill gap* prioritas, dan tidak menawarkan langkah belajar konkret untuk menutup celah tersebut.

### 1.2 Solusi yang Ditawarkan

**Vecta** adalah aplikasi *desktop* dan *mobile* berarsitektur *local-first* yang mengubah cerita kemampuan bebas (*freeform storytelling*) menjadi profil kompetensi terstruktur, peta kebutuhan pasar nyata, pemeringkatan lowongan hibrida, analisis *gap on-demand*, dan *roadmap* belajar bertahap—dibungkus dalam antarmuka native modern dengan tiga pilihan tema kurasi.

### 1.3 Lima Fitur Utama Vecta (Core MVP)

1. **AI Story-to-Chips (*AI Help on Formatting* & Keyword Recommender):** Mengubah cerita santai pengguna menjadi ringkasan profesional, daftar *skill* baku beserta level kemahiran, dan rekomendasi *keyword scraping* (**Primary Fit** & **Adjacent Fit**) yang dikonfirmasi lewat *chip* interaktif.

2. **Multi-Source Job Collector (*3-Layer Scraping* + Input Manual):** Python Worker mengambil lowongan dari portal besar (LinkedIn Guest + Glints/JobStreet) berdasarkan *keyword* aktif, dilengkapi jalur *paste* teks/URL manual. Tiap lowongan diekstrak satu kali (*Extract-Once*) oleh Gemini.

3. **Dynamic Competency Mapping (Peta Kompetensi Pasar):** Mengagregasi seluruh lowongan per profesi ke dalam 4 klaster (`core_hard`, `tools_tech`, `soft_skill`, `domain`) lengkap dengan persentase permintaan pasar dan indikator kepemilikan *skill* pengguna.

4. **Hybrid Smart Matching & On-Demand Gap Analysis:** Mesin lokal di Rust mengurutkan lowongan secara instan tanpa biaya token menggunakan formula hibrida (Vektor Semantik + Irisan Skill + Level), disertai analisis *gap* mendalam dan tips penyesuaian CV secara *on-demand*.

5. **AI Career Roadmap (Tracker Penutup Gap):** Membangkitkan rencana belajar bertahap (Fundamental $\rightarrow$ Praktik $\rightarrow$ Proyek Mini) dari *skill gap* profesi target maupun 1 lowongan spesifik lengkap dengan *checklist* progres yang terhubung ke tabel `skills`.

## 2. Arsitektur Sistem & Tech Stack Eksplisit

Vecta menggunakan arsitektur **Hybrid Monorepo** yang memisahkan *native client & core engine* ringan (Tauri 2.0 + Rust) dari mesin *scraping* (Python 3.13).

### 2.1 Diagram Aliran Data Lintas Lapisan

```
+-----------------------------------------------------------------------+
|  APPS/CLIENT — TAURI 2.0 (Desktop Windows/macOS & Mobile Android/iOS) |
|                                                                       |
|  [Vue 3 + TS + shadcn-vue + motion-v]  <-- tauri-specta (Typed IPC)   |
|                    |                                                  |
|                    v                                                  |
|  [Rust Core Engine: commands/ + core/]                                |
|    |-- keychain.rs  --> OS Credential Manager (BYOK Gemini Key)       |
|    |-- db.rs        --> SQLite Lokal (WAL Mode + Vec<f32> BLOB)       |
|    |-- matching.rs  --> Cosine Similarity + Hybrid Score (<1ms)       |
|    |-- gemini.rs    --> HTTPS Structured JSON & Embeddings            |
+----------|--------------------------------------------|---------------+
           | HTTP POST 127.0.0.1:8000/api/v1/scrape     | HTTPS (BYOK)
           v                                            v
+------------------------------------+    +-----------------------------+
|  SERVICES/SCRAPER-WORKER (Python)  |    |   GOOGLE GEMINI API         |
|  FastAPI + JobSpy + httpx + bs4    |    |   - gemini-2.5-flash (JSON) |
|  - LinkedIn Guest (no login)       |    |   - gemini-embedding-001    |
|  - Glints / JobStreet JSON API     |    |     (768-dim float32)       |
|  - HTML Cleaner & Deduplication    |    +-----------------------------+
+------------------------------------+

```

### 2.2 Manifest Dependensi Esensial

| Komponen | Paket / Crate Utama | Catatan Konfigurasi | 
| ----- | ----- | ----- | 
| **Frontend (`apps/client/package.json`)** | `vue` (^3.5), `vue-router` (^4), `pinia` (^2), `tailwindcss` (^4), `shadcn-vue`, `lucide-vue-next`, `motion-v`, `@formkit/auto-animate`, `@tauri-apps/api` (^2) | Font: `@fontsource/plus-jakarta-sans` & `@fontsource/jetbrains-mono` (bundling lokal agar *offline-ready*). | 
| **Rust Core (`apps/client/src-tauri/Cargo.toml`)** | `tauri` (^2), `tauri-specta` (^2), `specta` (^2), `specta-typescript`, `serde` (`derive`), `serde_json`, `rusqlite` (`bundled`), `reqwest` (`json`, `rustls-tls`), `tokio` (`full`), `keyring`, `uuid` (`v4`), `chrono`, `thiserror`, `sha2` | Gunakan `rusqlite` fitur `bundled` agar tidak perlu DLL SQLite eksternal di Windows/Android; gunakan `rustls-tls` agar kompilasi lintas platform mulus. | 
| **Python Worker (`services/scraper-worker/pyproject.toml`)** | `fastapi`, `uvicorn[standard]`, `pydantic` (^2), `python-jobspy`, `httpx`, `beautifulsoup4`, `pytest`, `pytest-asyncio` | Dikelola dengan `uv` (`Python >= 3.11`, target `3.13`). | 

## 3. Kebutuhan Fungsional & Acceptance Criteria (Given/When/Then)

Setiap fitur MVP di bawah ini memiliki kriteria penerimaan objektif yang menjadi patokan pengujian agen.

### FR-01: Pengaturan BYOK & Validasi Koneksi Gemini (MVP)

* **Deskripsi:** Pengguna memasukkan Gemini API Key di Pengaturan, sistem memvalidasi kunci secara langsung dan menyimpannya terenkripsi di OS Keychain.

* **Acceptance Criteria:**

  * *Given* pengguna belum mengatur API Key, *When* pengguna mencoba menjalankan fitur AI, *Then* aplikasi menampilkan banner/modal arahan ke `SettingsView` tanpa *crash*.

  * *Given* pengguna memasukkan API Key valid dan menekan "Tes & Simpan", *When* Rust mengirim permintaan uji ringan ke Gemini API, *Then* kunci tersimpan di OS Keychain, UI menampilkan status terhubung (hanya 4 karakter terakhir yang ditampilkan, misal `AIza...9xQp`), dan kunci tidak pernah muncul di SQLite maupun `localStorage`.

  * *Given* pengguna memasukkan API Key tidak valid, *When* tes koneksi gagal (`HTTP 400/401/403`), *Then* kunci tidak disimpan dan UI menampilkan pesan kesalahan bahasa Indonesia yang jelas.

### FR-02: Story-to-Chips Profile Builder (MVP)

* **Deskripsi:** *Textbox* cerita bebas yang dirapikan AI menjadi profil terstruktur, daftar *skill* baku, dan level kemahiran yang bisa disunting lewat *chip* interaktif.

* **Acceptance Criteria:**

  * *Given* pengguna menulis minimal 30 karakter cerita di `OnboardingStoryView` dan menekan "Rapikan & Petakan Profil", *When* Kontrak D berhasil dipanggil, *Then* UI menampilkan ringkasan profesional, daftar *skill chips* (lengkap dengan level dan *tooltip* bukti cerita), serta daftar *recommended keyword chips*.

  * *Given* daftar *skill chips* tampil di layar konfirmasi, *When* pengguna mengklik sebuah *chip*, *Then* level kemahirannya berputar (`beginner` $\rightarrow$ `intermediate` $\rightarrow$ `advanced`); mengklik ikon `x` menghapus *skill* tersebut dengan animasi `v-auto-animate`; dan pengguna dapat menambahkan *skill* baru manual.

  * *Given* pengguna menekan "Simpan Profil & Cari Lowongan", *When* Rust menyimpan profil, *Then* data tersimpan secara atomik (transaksi SQLite) ke `user_profiles`, `skills`, `user_skills`, `user_scrape_keywords`, sekaligus membangkitkan dan menyimpan *embedding* 768-dimensi ke `user_profiles.embedding`.

### FR-03: Keyword Recommender & Custom Roles (MVP)

* **Deskripsi:** Rekomendasi kata kunci *scraping* terbagi dalam `primary_fit` dan `adjacent_fit` ditambah dukungan kata kunci kustom.

* **Acceptance Criteria:**

  * *Given* hasil Kontrak D memiliki rekomendasi peran, *When* ditampilkan di layar konfirmasi maupun pengaturan pencarian, *Then* setiap peran menampilkan label `Primary Fit` atau `Adjacent Fit`, alasan singkat kecocokan, daftar `scrape_queries`, dan status centang aktif/nonaktif (`is_active`).

  * *Given* pengguna mengetik peran kustom (misal `"Cloud Security Engineer"`), *When* ditekan Enter, *Then* peran masuk ke `user_scrape_keywords` dengan `fit_type = 'custom'` dan `is_active = 1`.

### FR-04: Multi-Source Job Collector & Input Manual (MVP)

* **Deskripsi:** Mengambil lowongan otomatis dari Python Worker (minimal 2 sumber portal) serta menerima tempel teks/URL manual.

* **Acceptance Criteria:**

  * *Given* pengguna menempelkan teks deskripsi lowongan pada modal "Tambah Lowongan Manual", *When* dikirim, *Then* Rust memanggil Kontrak A, menyimpan ke `jobs` (`source = 'manual'`), membangkitkan *embedding*, menghitung skor *matching*, dan langsung menampilkan kartu lowongan tersebut di `JobsFeedView`.

  * *Given* pengguna menekan "Refresh Lowongan", *When* Rust memanggil `POST /api/v1/scrape` pada Python Worker, *Then* lowongan yang sudah ada (`UNIQUE(source, external_id)`) dilewati tanpa memanggil Gemini API, sedangkan lowongan baru diekstrak via Kontrak A + Embedding lalu langsung di-ranking.

  * *Given* Python Worker belum menyala atau satu sumber portal memblokir *request*, *When* scraping dijalankan, *Then* aplikasi tidak *crash*: jika *worker* mati tampilkan pesan panduan menyalakan *worker*; jika 1 portal gagal namun portal lain sukses, simpan hasil yang sukses dan tampilkan peringatan parsial.

### FR-05: Dynamic Competency Mapping (MVP)

* **Deskripsi:** Peta kebutuhan kompetensi per profesi dari agregasi lowongan aktual, dikelompokkan dalam 4 klaster.

* **Acceptance Criteria:**

  * *Given* terdapat minimal 1 lowongan pada `normalized_role` tertentu, *When* pengguna memilih peran tersebut di `CompetencyMapView`, *Then* aplikasi menampilkan daftar *skill* terurut berdasarkan persentase kemunculan (`demand_percentage`), dapat difilter berdasarkan 4 kategori (`core_hard`, `tools_tech`, `soft_skill`, `domain`) dan *toggle* "Hanya Tampilkan Gap".

  * *Given* sebuah *skill* pada peta sudah dimiliki pengguna di `user_skills`, *When* baris *skill* dirender, *Then* tampil indikator status cocok (`--status-match`); jika belum dimiliki tampil indikator gap (`--status-gap`) beserta tombol aksi "Buat Roadmap dari Gap Peran Ini".

### FR-06: Hybrid Smart Matching & On-Demand Gap Analysis (MVP)

* **Deskripsi:** Pemeringkatan instan 100% lokal di Rust serta analisis *gap* mendalam dan saran CV berbasis Gemini yang di-*cache*.

* **Acceptance Criteria:**

  * *Given* profil pengguna diperbarui atau lowongan baru masuk, *When* mesin `matching.rs` dijalankan, *Then* seluruh baris `job_matches` dihitung ulang dalam waktu $< 100\text{ms}$ untuk 1.000 lowongan tanpa satu pun panggilan jaringan ke Gemini.

  * *Given* pengguna membuka detail satu lowongan di `JobsFeedView`, *When* `ai_gap_analysis_json` masih `NULL`, *Then* tampil skor hibrida + daftar *matched/missing skills* instan beserta tombol "Analisis Gap Mendalam".

  * *Given* pengguna menekan "Analisis Gap Mendalam", *When* Kontrak B selesai, *Then* hasilnya disimpan ke `job_matches.ai_gap_analysis_json`; saat lowongan yang sama dibuka kembali di lain waktu, data langsung dimuat dari *cache* SQLite tanpa memanggil Gemini lagi (kecuali pengguna menekan tombol "Analisis Ulang").

### FR-07: AI Career Roadmap & Auto-Sync Profil (MVP)

* **Deskripsi:** Generator rencana belajar bertahap yang bisa dicentang progresnya dan memperbarui profil *skill* pengguna.

* **Acceptance Criteria:**

  * *Given* pengguna memicu pembuatan *roadmap* dari Peta Kompetensi atau Detail Lowongan, *When* Kontrak C selesai, *Then* roadmap tersimpan ke `learning_roadmaps` dan `roadmap_steps`, lalu tampil sebagai *timeline* bertahap (`Fundamental`, `Praktik`, `Proyek Mini`) lengkap dengan kata kunci belajar, ide proyek mini, dan estimasi jam.

  * *Given* pengguna mencentang sebuah langkah menjadi `done`, *When* langkah tersebut terhubung ke `skill_id` yang belum ada di `user_skills`, *Then* aplikasi memunculkan *toast* interaktif: *"Selesai belajar \[Nama Skill\]! Tambahkan ke profilmu?"* yang jika diklik akan menambah *skill* ke `user_skills` (level `beginner`) dan otomatis menghitung ulang skor *matching*.

### FR-08: Multi-Theme Engine (MVP)

* **Deskripsi:** 3 tema kurasi (`paper-ink`, `swiss-mono`, `warm-stone`) yang dapat diganti instan tanpa FOUC.

* **Acceptance Criteria:**

  * *Given* pengguna memilih salah satu tema pada `ThemeSwitcher`, *When* diklik, *Then* `document.documentElement.dataset.theme` langsung berubah, tersimpan ke `localStorage['vecta:theme']`, dan seluruh halaman berubah seketika tanpa *reload*.

  * *Given* pengguna menutup lalu membuka kembali aplikasi saat tema `swiss-mono` aktif, *When* jendela Tauri terbuka, *Then* tidak ada kedipan layar putih (*Zero-FOUC*) karena skrip *inline* di `index.html` sudah menetapkan `data-theme` sebelum render pertama.

### Fitur Fase 2 (Out-of-Scope MVP — Jangan Dikerjakan di Milestone 1–4)

* **FR-09:** Sinkronisasi otomatis Desktop $\leftrightarrow$ Mobile menggunakan Turso (LibSQL).

* **FR-10:** Chrome Extension Companion (*1-Click Job Capture* Manifest V3).

* **FR-11:** Visualisasi *Radar Chart* (`vue-echarts`), komparasi 2 profesi berdampingan, dan *CV Tailoring Editor* interaktif.

## 4. Sistem Desain, Multi-Theme Engine & Blueprint Layar

### 4.1 Prinsip Anti-Slop UI

1. **Zero Neon Glow & Zero Gradients:** Dilarang menggunakan *box-shadow* berpendar, latar gradien ungu/biru generik, atau batas berwarna mencolok.

2. **Satu Fokus per Layar (*Calm Layout*):** Maksimal 2 kolom utama pada Desktop (*sidebar* + kanvas utama, atau *list* + *detail split-pane*). Hindari dashboard 4-kotak berdesakan.

3. **Neutral Chips with Dot Indicators:** *Skill chip* memakai latar netral `--bg-elevated` dengan teks `--text-primary` dan titik indikator $6\text{px}$ (`--status-match` / `--status-gap` / `--status-critical`) untuk statusnya.

4. **Tipografi Presisi:**

   * `--font-sans`: `'Plus Jakarta Sans', system-ui, -apple-system, sans-serif` (Heading utama memakai `letter-spacing: -0.02em`).

   * `--font-mono`: `'JetBrains Mono', ui-monospace, monospace` dengan `font-variant-numeric: tabular-nums` pada semua angka skor, persentase, dan jam belajar.

### 4.2 Implementasi Token CSS Lengkap (`apps/client/src/styles/themes.css`)

Agen **wajib** menggunakan definisi token berikut sebagai dasar `themes.css` dan memetakannya ke utilitas Tailwind v4 / variabel `shadcn-vue`:

```
:root {
  --font-sans: 'Plus Jakarta Sans', system-ui, -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', ui-monospace, SFMono-Regular, monospace;
  --ease-vecta: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
}

/* 1. EDITORIAL PAPER & INK (Default — Light, Calm, Read.cv / Notion vibe) */
:root,
[data-theme="paper-ink"] {
  color-scheme: light;
  --bg-canvas: #fbfbf9;
  --bg-surface: #ffffff;
  --bg-elevated: #f2f1ec;
  --bg-hover: #eae8e1;
  --border-subtle: #e6e5e0;
  --border-strong: #d4d2c9;

  --text-primary: #141413;
  --text-secondary: #3f3e3a;
  --text-muted: #5c5a54;

  --accent-primary: #2563eb;
  --accent-primary-fg: #ffffff;
  --accent-secondary: #1d4ed8;
  --accent-soft: rgba(37, 99, 235, 0.10);

  --status-match: #16a34a;
  --status-gap: #d97706;
  --status-critical: #dc2626;
  --status-adjacent: #6366f1;

  --radius-sm: 6px;
  --radius-card: 10px;
  --radius-pill: 9999px;
  --shadow-card: 0 1px 2px rgba(20, 20, 19, 0.04);
  --shadow-popover: 0 8px 24px rgba(20, 20, 19, 0.08);
}

/* 2. SWISS MONOCHROME (Ultra-Minimal Dark — Linear / Vercel vibe) */
[data-theme="swiss-mono"] {
  color-scheme: dark;
  --bg-canvas: #09090b;
  --bg-surface: #121214;
  --bg-elevated: #1c1c1f;
  --bg-hover: #26262a;
  --border-subtle: #27272a;
  --border-strong: #3f3f46;

  --text-primary: #fafafa;
  --text-secondary: #d4d4d8;
  --text-muted: #a1a1aa;

  --accent-primary: #f97316;
  --accent-primary-fg: #09090b;
  --accent-secondary: #e4e4e7;
  --accent-soft: rgba(249, 115, 22, 0.14);

  --status-match: #22c55e;
  --status-gap: #f97316;
  --status-critical: #ef4444;
  --status-adjacent: #a78bfa;

  --radius-sm: 6px;
  --radius-card: 8px;
  --radius-pill: 9999px;
  --shadow-card: none;
  --shadow-popover: 0 12px 32px rgba(0, 0, 0, 0.45);
}

/* 3. ORGANIC WARM STONE & FOREST (Natural — Claude / Things 3 / Bear vibe) */
[data-theme="warm-stone"] {
  color-scheme: light;
  --bg-canvas: #f6f4ef;
  --bg-surface: #ffffff;
  --bg-elevated: #ebe6dc;
  --bg-hover: #e2dcd0;
  --border-subtle: #e2ddd3;
  --border-strong: #cec7b9;

  --text-primary: #1c1917;
  --text-secondary: #44403c;
  --text-muted: #57534e;

  --accent-primary: #1e4d3b;
  --accent-primary-fg: #ffffff;
  --accent-secondary: #c25934;
  --accent-soft: rgba(30, 77, 59, 0.10);

  --status-match: #1e4d3b;
  --status-gap: #c25934;
  --status-critical: #b91c1c;
  --status-adjacent: #7c5e3c;

  --radius-sm: 8px;
  --radius-card: 16px;
  --radius-pill: 9999px;
  --shadow-card: 0 6px 24px rgba(28, 25, 23, 0.06);
  --shadow-popover: 0 12px 32px rgba(28, 25, 23, 0.10);
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}

```

### 4.3 Skrip Anti-FOUC (`apps/client/index.html`)

Wajib disisipkan di dalam `<head>` sebelum bundel JS dimuat:

```
<script>
  (function () {
    const allowed = ['paper-ink', 'swiss-mono', 'warm-stone'];
    const saved = localStorage.getItem('vecta:theme');
    const theme = allowed.includes(saved) ? saved : 'paper-ink';
    document.documentElement.dataset.theme = theme;
  })();
</script>

```

### 4.4 Blueprint Layar & State Matrix

* **Navigasi Responsif:** Desktop ($\ge 1024\text{px}$) memakai *sidebar* kiri selebar $240\text{px}$ berisi 5 menu (`Dashboard`, `Profil Cerita`, `Lowongan`, `Peta Kompetensi`, `Roadmap`) + *quick swatch* `ThemeSwitcher` dan tombol `Pengaturan` di bawah. Mobile ($< 1024\text{px}$) berubah otomatis menjadi *bottom navigation bar* 5 ikon bersih.

| Layar | Struktur Layout Desktop | Empty State | Loading State | Error / Degraded State | 
| ----- | ----- | ----- | ----- | ----- | 
| **`OnboardingStoryView`** | Kanvas tulis tunggal di tengah (`max-w-3xl`), diikuti panel konfirmasi *chip* di bawahnya setelah AI selesai. | *Placeholder* santai + 3 tombol contoh cerita sekali klik (*Software Dev*, *Data Analyst*, *Hybrid Tech*). | Indikator proses halus pada CTA + *skeleton chips* di area hasil. | Pesan inline dengan tombol "Coba Lagi" (cerita yang diketik user **tidak boleh hilang**). | 
| **`DashboardView`** | Baris 3 KPI ringkas (Lowongan Cocok $\ge 75\%$, Progres Roadmap, Total Skill) + 2 kolom bawah: *Top 5 Matches* & *Langkah Belajar Berikutnya*. | Ajakan 1-klik "Mulai Ceritakan Profilmu" atau "Ambil Lowongan Pertama". | *Skeleton* angka & kartu tipis tanpa kedipan keras. | Banner peringatan tipis jika API Key belum diatur. | 
| **`JobsFeedView`** | *Master-Detail Split-Pane* (kiri $380\text{px}$ daftar kartu terurut `final_score`, kanan panel detail + `MatchRing` + tombol *Analisis Gap* & *Buat Roadmap*). Header berisi filter peran, tombol "Refresh Lowongan", dan "+ Tempel Manual". | Ilustrasi minimalis + tombol "Jalankan Scraper" & "Tempel Teks Lowongan". | Progress bar tipis di atas daftar saat scraping berjalan (daftar lama tetap bisa dibaca). | Toast peringatan parsial jika salah satu portal gagal / *worker* offline. | 
| **`CompetencyMapView`** | Header pemilih peran (`normalized_role`) + filter 4 klaster + toggle "Hanya Tampilkan Gap" + daftar *horizontal demand bar* terurut. | Pesan "Belum ada data lowongan untuk peran ini" + tombol picu scraping peran tersebut. | *Skeleton* baris bar horizontal. | Pesan jika sampel lowongan $< 5$ (*"Sampel kecil: akurasi peta akan meningkat setelah lebih banyak lowongan masuk"*). | 
| **`RoadmapView`** | Selector *roadmap* di kiri/atas + *vertical timeline* 3 fase (`Fundamental`, `Praktik`, `Proyek Mini`) berisi *checklist*, kata kunci pencarian, ide proyek mini, dan estimasi jam. | Tombol "Buat Roadmap dari Gap Terbesar". | *Skeleton timeline* 3 fase. | Pesan gagal generate + tombol ulang tanpa menghapus *roadmap* lama. | 
| **`SettingsView`** | Form satu kolom (`max-w-2xl`): kartu **Gemini API Key (BYOK)**, kartu **Pilihan 3 Tema** (pratinjau visual mini), konfigurasi URL Python Worker (`http://127.0.0.1:8000`), dan aksi ekspor/reset database lokal. | Input API Key kosong dengan panduan tautan resmi Google AI Studio. | Spinner pada tombol "Tes & Simpan". | Pesan error spesifik (kunci tidak valid, kuota habis `429`, atau jaringan terputus). | 

### 4.5 Koreografi Motion (`motion-v` & `@formkit/auto-animate`)

* **Story-to-Chips:** Setelah respons Kontrak D tiba, *chips* muncul berurutan (*stagger* $30\text{ms}$, `opacity: 0 -> 1`, `scale: 0.96 -> 1`). Kontainer *chip* menggunakan `v-auto-animate` agar penghapusan/penambahan *chip* mengalir halus.

* **Match Ring:** Saat memilih lowongan pada *split-pane*, *SVG stroke-dashoffset* mengisi lingkaran skor selama $380\text{ms}$ dengan `--ease-vecta` bersamaan dengan *count-up* angka `JetBrains Mono`.

* **Competency Bar & Filter:** Mengaktifkan toggle "Hanya Tampilkan Gap" memanfaatkan `v-auto-animate` (*FLIP transition*) sehingga baris *skill* yang sudah dikuasai meluncur keluar dan sisanya naik mulus.

* **Roadmap Check:** Centang langkah memicu animasi ceklis mikro (*spring*) dan pengisian garis vertikal *timeline* dengan `--accent-primary`.

## 5. Spesifikasi Python Scraper Worker (`services/scraper-worker`)

### 5.1 Strategi Pengambilan Data 3-Lapis

1. **Lapis 1 — Internal API & Public Guest Endpoint (MVP):**

   * `linkedin_guest.py`: Memanfaatkan `python-jobspy` / *public guest search* LinkedIn tanpa login dengan jeda acak (*jitter*) $3\text{--}8$ detik antar *request*. **Larangan Keras:** Jangan pernah meminta atau menggunakan kredensial akun LinkedIn pribadi.

   * `glints_api.py` / `jobstreet_api.py`: Mengakses *endpoint* JSON/GraphQL publik mereka menggunakan `httpx.AsyncClient` dengan header wajar dan *timeout* $15$ detik.

2. **Lapis 2 — Worker Terpisah:**

   * MVP: FastAPI lokal berjalan di `http://127.0.0.1:8000`.

   * Fase 2: Dijalankan terjadwal lewat GitHub Actions Cron dan menulis ke Turso (LibSQL).

3. **Lapis 3 — Jalur Manual & 1-Click Capture:**

   * MVP: Jalur *Paste Teks/URL* langsung lewat Rust Core (0% risiko *anti-bot*).

   * Fase 2: Chrome Extension Manifest V3.

### 5.2 Kontrak HTTP FastAPI Worker

#### `GET /health`

* **Response `200 OK`:** `{ "status": "ok", "version": "0.1.0", "available_sources": ["linkedin", "glints", "jobstreet"] }`

#### `POST /api/v1/scrape`

* **Request Body (Pydantic `ScrapeRequest`):**

```
{
  "queries": ["Software Developer", "Data Analyst"],
  "sources": ["linkedin", "glints"],
  "location": "Indonesia",
  "limit_per_query": 15
}

```

* **Response Body `200 OK` (Pydantic `ScrapeResponse` — Mendukung Partial Success):**

```
{
  "jobs": [
    {
      "source": "linkedin",
      "external_id": "li-4123456789",
      "url": "https://www.linkedin.com/jobs/view/4123456789",
      "title": "Junior Backend Developer",
      "company": "PT Contoh Teknologi",
      "location": "Jakarta, Indonesia",
      "raw_description": "Teks deskripsi lowongan bersih tanpa tag HTML..."
    }
  ],
  "stats": {
    "total_fetched": 24,
    "deduplicated_in_batch": 3,
    "duration_ms": 6420
  },
  "warnings": [
    {
      "source": "glints",
      "code": "RATE_LIMITED",
      "message": "Glints mengembalikan HTTP 429 pada query kedua; hasil parsial tetap dikembalikan."
    }
  ]
}

```

### 5.3 Aturan Pembersihan & Deduplikasi (`pipeline/cleaner.py`)

1. Hapus seluruh tag HTML (`script`, `style`, markup) menggunakan BeautifulSoup, konversi `<li>` menjadi baris bullet bersih (`- ...`), dan normalisasi spasi ganda menjadi satu baris baru.

2. Potong `raw_description` maksimal $12.000$ karakter (membuang *boilerplate* footer portal) agar hemat token Gemini.

3. Jika sebuah portal tidak memberikan `external_id` stabil, buat ID deterministik: `sha256(f"{source}|{company.lower().strip()}|{title.lower().strip()}|{location.lower().strip()}")[:16]`.

4. Setiap adapter wajib dibungkus `try/except` independen agar kegagalan 1 sumber tidak menggagalkan sumber lain.

## 6. Spesifikasi Algoritma Hybrid Smart Matching (`core/matching.rs`)

### 6.1 Normalisasi Input Embedding

* **Vektor Profil (**$\mathbf{u}$**):** Dibangkitkan dari teks terstruktur hasil rapian AI (bukan curhatan mentah):
  `"Role Targets: {roles}\nExperience Level: {level}\nSummary: {summary}\nSkills: {skill_1 (proficiency), skill_2 (proficiency), ...}"`

* **Vektor Lowongan (**$\mathbf{j}$**):** Dibangkitkan dari hasil ekstraksi Kontrak A:
  `"Role: {normalized_role} ({experience_level})\nTitle: {title}\nSummary: {role_summary}\nMandatory Skills: {...}\nNice to Have: {...}"`

* **Format Penyimpanan SQLite `BLOB`:** Array `Vec<f32>` berukuran $768$ diserialisasi ke bytes *little-endian* (`f32::to_le_bytes`, total $3.072$ bytes per vektor) agar lintas arsitektur x86_64 dan ARM64 (Android/Apple Silicon).

### 6.2 Formula Skor Gabungan

$$
\text{Final Score} = (0.40 \times S_{\text{vector}}) + (0.45 \times S_{\text{skill}}) + (0.15 \times S_{\text{level}})
$$

1. **Skor Vektor Semantik (**$S_{\text{vector}}$ **— Bobot** $40\%$**):**
   

   $$
   S_{\text{vector}} = \text{clamp}\left(\frac{\mathbf{u} \cdot \mathbf{j}}{\|\mathbf{u}\| \|\mathbf{j}\|}, 0.0, 1.0\right)
   $$

2. **Skor Irisan Kompetensi Berbobot (**$S_{\text{skill}}$ **— Bobot** $45\%$**):**
   Dengan bobot $w_i = 1.0$ untuk `mandatory` dan $w_i = 0.5$ untuk `nice_to_have`:
   

   $$
   S_{\text{skill}} = \frac{\sum_{i \in (\text{User} \cap \text{Job})} w_i}{\sum_{k \in \text{Job}} w_k}
   $$

3. **Skor Kesesuaian Level (**$S_{\text{level}}$ **— Bobot** $15\%$**):**
   Skala ordinal: `intern = 0`, `junior = 1`, `mid = 2`, `senior = 3`. Dengan $\Delta = |L_{\text{user}} - L_{\text{job}}|$:

   * $\Delta = 0 \Rightarrow S_{\text{level}} = 1.00$

   * $\Delta = 1 \Rightarrow S_{\text{level}} = 0.65$

   * $\Delta = 2 \Rightarrow S_{\text{level}} = 0.25$

   * $\Delta \ge 3 \Rightarrow S_{\text{level}} = 0.00$

### 6.3 Penanganan Edge Cases Wajib (Target Unit Test `matching.rs`)

| Kasus Batas (*Edge Case*) | Perilaku Wajib Sistem | 
| ----- | ----- | 
| Lowongan memiliki $0$ *skill* terekstrak ($\sum w_k = 0$) | Tetapkan $S_{\text{skill}} = S_{\text{vector}}$ agar lowongan tidak otomatis mendapat skor $0$ atau `NaN`. | 
| Pengguna belum memiliki *skill* di `user_skills` | Tetapkan $S_{\text{skill}} = 0.0$ dan `matched_skills = []`, seluruh *skill* lowongan masuk `missing_skills`. | 
| Salah satu *embedding* `NULL` atau panjang dimensi tidak sama atau $\|\mathbf{u}\| =$ | Tetapkan $S_{\text{vector}} = 0.0$ (jangan *panic* atau menghasilkan `NaN`), dan distribusikan ulang bobot ke $(0.75 \times S_{\text{skill}}) + (0.25 \times S_{\text{level}})$. | 
| Level pengguna atau lowongan bernilai `unknown` | Tetapkan $S_{\text{level}} = 0.75$ (netral cenderung positif). | 
| Pencocokan nama *skill* lintas kapitalisasi/alias | Cocokkan berdasarkan `skill_id` relasional hasil normalisasi saat ingest, bukan perbandingan string mentah saat *matching*. | 

## 7. Skema Database SQLite & Query Analitik

### 7.1 Konfigurasi Koneksi (`core/db.rs`)

Setiap koneksi SQLite yang dibuka oleh Rust wajib langsung mengeksekusi pragma berikut:

```
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;

```

### 7.2 DDL Migrasi Awal (`shared/migrations/001_initial_schema.sql`)

```
PRAGMA foreign_keys = ON;

-- 0. Pelacak versi migrasi skema
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 1. Profil Pengguna (mendukung lebih dari 1 profil karier)
CREATE TABLE IF NOT EXISTS user_profiles (
    id TEXT PRIMARY KEY,
    profile_name TEXT NOT NULL,
    target_roles_json TEXT NOT NULL DEFAULT '[]',
    raw_story_text TEXT,
    summary TEXT,
    experience_level TEXT NOT NULL CHECK (experience_level IN (
        'intern', 'junior', 'mid', 'senior', 'unknown'
    )) DEFAULT 'junior',
    embedding BLOB,                        -- Vec<f32> little-endian (768 * 4 bytes)
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. Kamus Kompetensi Baku (pusat normalisasi & agregasi Peta Kompetensi)
CREATE TABLE IF NOT EXISTS skills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    canonical_name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    category TEXT NOT NULL CHECK (category IN (
        'core_hard', 'tools_tech', 'soft_skill', 'domain'
    )),
    aliases_json TEXT NOT NULL DEFAULT '[]',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 3. Relasi Skill Milik Pengguna (M:N)
CREATE TABLE IF NOT EXISTS user_skills (
    user_id TEXT NOT NULL,
    skill_id INTEGER NOT NULL,
    proficiency TEXT NOT NULL CHECK (proficiency IN (
        'beginner', 'intermediate', 'advanced'
    )) DEFAULT 'intermediate',
    evidence TEXT,
    PRIMARY KEY (user_id, skill_id),
    FOREIGN KEY (user_id) REFERENCES user_profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

-- 4. Rekomendasi & Daftar Aktif Keyword Scraping milik Pengguna
CREATE TABLE IF NOT EXISTS user_scrape_keywords (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    role_label TEXT NOT NULL,
    fit_type TEXT NOT NULL CHECK (fit_type IN (
        'primary_fit', 'adjacent_fit', 'custom'
    )) DEFAULT 'custom',
    scrape_queries_json TEXT NOT NULL DEFAULT '[]',
    match_reason TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    last_scraped_at DATETIME,
    FOREIGN KEY (user_id) REFERENCES user_profiles(id) ON DELETE CASCADE
);

-- 5. Data Lowongan Hasil Scraping atau Input Manual
CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL CHECK (source IN (
        'linkedin', 'glints', 'jobstreet', 'manual'
    )),
    external_id TEXT NOT NULL,
    url TEXT,
    title TEXT NOT NULL,
    normalized_role TEXT NOT NULL,
    company TEXT NOT NULL,
    location TEXT,
    work_mode TEXT NOT NULL CHECK (work_mode IN (
        'onsite', 'hybrid', 'remote', 'unknown'
    )) DEFAULT 'unknown',
    experience_level TEXT NOT NULL CHECK (experience_level IN (
        'intern', 'junior', 'mid', 'senior', 'unknown'
    )) DEFAULT 'unknown',
    raw_description TEXT NOT NULL,
    extracted_summary_json TEXT,
    embedding BLOB,
    scraped_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source, external_id)
);

-- 6. Relasi Kompetensi yang Diminta Setiap Lowongan (M:N)
CREATE TABLE IF NOT EXISTS job_skills (
    job_id TEXT NOT NULL,
    skill_id INTEGER NOT NULL,
    requirement_type TEXT NOT NULL CHECK (requirement_type IN (
        'mandatory', 'nice_to_have'
    )),
    weight REAL NOT NULL DEFAULT 1.0,
    PRIMARY KEY (job_id, skill_id),
    FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE,
    FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

-- 7. Hasil Hybrid Smart Matching & Cache Analisis Gap
CREATE TABLE IF NOT EXISTS job_matches (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    vector_score REAL NOT NULL,
    skill_overlap_score REAL NOT NULL,
    level_score REAL NOT NULL,
    final_score REAL NOT NULL,
    matched_skills_json TEXT NOT NULL DEFAULT '[]',
    missing_skills_json TEXT NOT NULL DEFAULT '[]',
    ai_gap_analysis_json TEXT,
    is_saved INTEGER NOT NULL DEFAULT 0,
    computed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, job_id),
    FOREIGN KEY (user_id) REFERENCES user_profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
);

-- 8. Header Roadmap Belajar
CREATE TABLE IF NOT EXISTS learning_roadmaps (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    target_role TEXT NOT NULL,
    source_job_id TEXT,
    title TEXT NOT NULL,
    total_estimated_weeks INTEGER NOT NULL DEFAULT 4,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES user_profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (source_job_id) REFERENCES jobs(id) ON DELETE SET NULL
);

-- 9. Item Checklist Tahapan Belajar
CREATE TABLE IF NOT EXISTS roadmap_steps (
    id TEXT PRIMARY KEY,
    roadmap_id TEXT NOT NULL,
    skill_id INTEGER,
    phase_order INTEGER NOT NULL,
    phase_name TEXT NOT NULL,
    topic_title TEXT NOT NULL,
    learning_keywords_json TEXT NOT NULL DEFAULT '[]',
    mini_project_idea TEXT,
    estimated_hours INTEGER NOT NULL DEFAULT 4,
    status TEXT NOT NULL CHECK (status IN (
        'todo', 'in_progress', 'done'
    )) DEFAULT 'todo',
    completed_at DATETIME,
    FOREIGN KEY (roadmap_id) REFERENCES learning_roadmaps(id) ON DELETE CASCADE,
    FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE SET NULL
);

-- Indeks Performa Query
CREATE INDEX IF NOT EXISTS idx_jobs_normalized_role ON jobs(normalized_role);
CREATE INDEX IF NOT EXISTS idx_job_skills_skill_id ON job_skills(skill_id);
CREATE INDEX IF NOT EXISTS idx_job_matches_ranking ON job_matches(user_id, final_score DESC);
CREATE INDEX IF NOT EXISTS idx_roadmap_steps_order ON roadmap_steps(roadmap_id, phase_order);
CREATE INDEX IF NOT EXISTS idx_user_scrape_keywords_active ON user_scrape_keywords(user_id, is_active);

```

### 7.3 Query Analitik Utama

**A. Peta Kompetensi per Profesi:**

```
SELECT
    s.id AS skill_id,
    s.canonical_name,
    s.category,
    COUNT(js.job_id) AS demand_count,
    ROUND(100.0 * COUNT(js.job_id) / (
        SELECT COUNT(*) FROM jobs WHERE normalized_role = :target_role
    ), 1) AS demand_percentage,
    CASE WHEN us.skill_id IS NOT NULL THEN 1 ELSE 0 END AS is_owned_by_user,
    us.proficiency AS user_proficiency
FROM job_skills js
JOIN jobs j ON j.id = js.job_id
JOIN skills s ON s.id = js.skill_id
LEFT JOIN user_skills us
    ON us.skill_id = s.id AND us.user_id = :user_id
WHERE j.normalized_role = :target_role
GROUP BY s.id
ORDER BY demand_count DESC, s.canonical_name ASC
LIMIT 30;

```

**B. Top Missing Skills untuk Input Generator Roadmap Berbasis Peran:**

```
SELECT s.id, s.canonical_name, s.category, COUNT(js.job_id) AS demand_count
FROM job_skills js
JOIN jobs j ON j.id = js.job_id
JOIN skills s ON s.id = js.skill_id
LEFT JOIN user_skills us ON us.skill_id = s.id AND us.user_id = :user_id
WHERE j.normalized_role = :target_role
  AND us.skill_id IS NULL
GROUP BY s.id
ORDER BY demand_count DESC
LIMIT 8;

```

## 8. Integrasi Google Gemini API & Kontrak Structured JSON

### 8.1 Konfigurasi Endpoint & Ketahanan Jaringan (`core/gemini.rs`)

Simpan nama model sebagai konstanta terpusat agar mudah diperbarui tanpa menyentuh logika bisnis:

* **Base URL:** `https://generativelanguage.googleapis.com/v1beta`

* **Generation Model (Default):** `gemini-2.5-flash`

  * Endpoint: `POST /models/{model}:generateContent`

  * Header autentikasi: `x-goog-api-key: {API_KEY}` (hindari menaruh kunci di URL query string agar tidak bocor di log HTTP).

  * Parameter wajib: `generationConfig.responseMimeType = "application/json"`, `generationConfig.responseSchema = <schema>`, `generationConfig.temperature = 0.1` (untuk Kontrak A & D) atau `0.3` (untuk Kontrak B & C).

* **Embedding Model (Default):** `gemini-embedding-001`

  * Endpoint: `POST /models/{model}:embedContent`

  * Payload wajib menyertakan `"outputDimensionality": 768` dan `"taskType"` (`"RETRIEVAL_DOCUMENT"` untuk lowongan, `"RETRIEVAL_QUERY"` untuk profil pengguna).

* **Retry & Rate-Limit Policy:** Jika menerima `HTTP 429` atau `HTTP 503`, lakukan *exponential backoff* maksimal $3$ percobaan ($1\text{s}, 2\text{s}, 4\text{s}$ + jitter). Untuk pemrosesan batch hasil scraping, beri jeda minimal $350\text{ms}$ antar panggilan ekstraksi atau batasi konkurensi maksimal $2$ *request* paralel agar aman pada *free tier* BYOK.

### 8.2 Kontrak A: Ekstraksi Kompetensi Lowongan (`shared/schemas/job_extraction.schema.json`)

* **Konteks Wajib di Prompt:** Daftar maksimal $50$ `known_canonical_skills` paling populer dari tabel `skills` untuk mencegah fragmentasi sinonim.

* **JSON Schema (`responseSchema`):**

```
{
  "type": "OBJECT",
  "required": ["normalized_role", "experience_level", "work_mode", "role_summary", "skills"],
  "properties": {
    "normalized_role": { "type": "STRING" },
    "experience_level": { "type": "STRING", "enum": ["intern", "junior", "mid", "senior", "unknown"] },
    "work_mode": { "type": "STRING", "enum": ["onsite", "hybrid", "remote", "unknown"] },
    "role_summary": { "type": "STRING" },
    "skills": {
      "type": "ARRAY",
      "items": {
        "type": "OBJECT",
        "required": ["canonical_name", "category", "requirement_type"],
        "properties": {
          "canonical_name": { "type": "STRING" },
          "category": { "type": "STRING", "enum": ["core_hard", "tools_tech", "soft_skill", "domain"] },
          "requirement_type": { "type": "STRING", "enum": ["mandatory", "nice_to_have"] },
          "matched_aliases": { "type": "ARRAY", "items": { "type": "STRING" } }
        }
      }
    }
  }
}

```

### 8.3 Kontrak B: On-Demand Skill Gap & CV Tailoring (`shared/schemas/gap_analysis.schema.json`)

* **Bahasa Output:** Bahasa Indonesia objektif dan langsung pada poin tindakan.

* **JSON Schema (`responseSchema`):**

```
{
  "type": "OBJECT",
  "required": ["verdict_headline", "strength_highlights", "prioritized_gaps", "cv_tailoring_tips"],
  "properties": {
    "verdict_headline": { "type": "STRING" },
    "strength_highlights": { "type": "ARRAY", "items": { "type": "STRING" } },
    "prioritized_gaps": {
      "type": "ARRAY",
      "items": {
        "type": "OBJECT",
        "required": ["skill", "severity", "reason"],
        "properties": {
          "skill": { "type": "STRING" },
          "severity": { "type": "STRING", "enum": ["high", "medium", "low"] },
          "reason": { "type": "STRING" }
        }
      }
    },
    "cv_tailoring_tips": { "type": "ARRAY", "items": { "type": "STRING" } }
  }
}

```

### 8.4 Kontrak C: Generator Roadmap Belajar (`shared/schemas/roadmap.schema.json`)

* **Aturan Prompt:** Urutkan fase 1 (`Fundamental`) $\rightarrow$ 2 (`Praktik`) $\rightarrow$ 3 (`Proyek Mini`). Jangan sertakan tautan URL mentah yang rawan mati; gunakan `learning_keywords` yang abadi.

* **JSON Schema (`responseSchema`):**

```
{
  "type": "OBJECT",
  "required": ["title", "target_role", "total_estimated_weeks", "steps"],
  "properties": {
    "title": { "type": "STRING" },
    "target_role": { "type": "STRING" },
    "total_estimated_weeks": { "type": "INTEGER" },
    "steps": {
      "type": "ARRAY",
      "items": {
        "type": "OBJECT",
        "required": ["phase_order", "phase_name", "target_skill", "topic_title", "learning_keywords", "mini_project_idea", "estimated_hours"],
        "properties": {
          "phase_order": { "type": "INTEGER" },
          "phase_name": { "type": "STRING" },
          "target_skill": { "type": "STRING" },
          "topic_title": { "type": "STRING" },
          "learning_keywords": { "type": "ARRAY", "items": { "type": "STRING" } },
          "mini_project_idea": { "type": "STRING" },
          "estimated_hours": { "type": "INTEGER" }
        }
      }
    }
  }
}

```

### 8.5 Kontrak D: Story-to-Profile & Keyword Recommender (`shared/schemas/story_profile.schema.json`)

* **Aturan Prompt:** Ekstrak *skill* eksplisit maupun implisit ke nama baku bahasa Inggris (cocokkan terhadap `known_canonical_skills`), taksir `proficiency` dari bukti cerita, dan hasilkan $3\text{--}5$ peran target terbagi `primary_fit` dan `adjacent_fit`.

* **JSON Schema (`responseSchema`):**

```
{
  "type": "OBJECT",
  "required": ["professional_summary", "estimated_experience_level", "extracted_skills", "recommended_keywords"],
  "properties": {
    "professional_summary": { "type": "STRING" },
    "estimated_experience_level": { "type": "STRING", "enum": ["intern", "junior", "mid", "senior", "unknown"] },
    "extracted_skills": {
      "type": "ARRAY",
      "items": {
        "type": "OBJECT",
        "required": ["canonical_name", "category", "proficiency", "evidence"],
        "properties": {
          "canonical_name": { "type": "STRING" },
          "category": { "type": "STRING", "enum": ["core_hard", "tools_tech", "soft_skill", "domain"] },
          "proficiency": { "type": "STRING", "enum": ["beginner", "intermediate", "advanced"] },
          "evidence": { "type": "STRING" }
        }
      }
    },
    "recommended_keywords": {
      "type": "ARRAY",
      "items": {
        "type": "OBJECT",
        "required": ["role_label", "fit_type", "match_reason", "scrape_queries"],
        "properties": {
          "role_label": { "type": "STRING" },
          "fit_type": { "type": "STRING", "enum": ["primary_fit", "adjacent_fit"] },
          "match_reason": { "type": "STRING" },
          "scrape_queries": { "type": "ARRAY", "items": { "type": "STRING" } }
        }
      }
    }
  }
}

```

## 9. Struktur Monorepo, Kontrak IPC & Taksonomi Error

### 9.1 Struktur Direktori Monorepo

```
vecta/
├── PRD.md                                 # Dokumen spesifikasi ini
├── README.md                              # Dokumentasi arsitektur, showcase 3 tema, setup BYOK
├── shared/
│   ├── migrations/
│   │   └── 001_initial_schema.sql
│   └── schemas/
│       ├── job_extraction.schema.json
│       ├── gap_analysis.schema.json
│       ├── roadmap.schema.json
│       └── story_profile.schema.json
├── apps/
│   └── client/                            # Tauri 2.0 (Desktop & Mobile)
│       ├── index.html                     # Termasuk inline script anti-FOUC data-theme
│       ├── package.json
│       ├── vite.config.ts
│       ├── src/
│       │   ├── main.ts
│       │   ├── App.vue
│       │   ├── styles/
│       │   │   └── themes.css             # Token CSS 3 tema (paper-ink, swiss-mono, warm-stone)
│       │   ├── composables/
│       │   │   └── useTheme.ts
│       │   ├── router/
│       │   │   └── index.ts
│       │   ├── stores/
│       │   │   ├── settings.ts
│       │   │   ├── profile.ts
│       │   │   ├── jobs.ts
│       │   │   ├── competency.ts
│       │   │   └── roadmap.ts
│       │   ├── views/
│       │   │   ├── OnboardingStoryView.vue
│       │   │   ├── DashboardView.vue
│       │   │   ├── JobsFeedView.vue
│       │   │   ├── CompetencyMapView.vue
│       │   │   ├── RoadmapView.vue
│       │   │   └── SettingsView.vue
│       │   ├── components/
│       │   │   ├── layout/                # AppSidebar.vue, MobileBottomNav.vue, ThemeSwitcher.vue
│       │   │   ├── profile/               # SkillChip.vue, KeywordChipGroup.vue
│       │   │   ├── jobs/                  # JobCard.vue, MatchRing.vue, GapAnalysisPanel.vue, ManualJobModal.vue
│       │   │   ├── competency/            # SkillDemandRow.vue
│       │   │   └── roadmap/               # RoadmapTimeline.vue
│       │   └── lib/
│       │       └── bindings.ts            # Auto-generated oleh tauri-specta (JANGAN EDIT MANUAL)
│       └── src-tauri/
│           ├── Cargo.toml
│           ├── tauri.conf.json
│           └── src/
│               ├── main.rs
│               ├── lib.rs                 # Registrasi state, Tauri commands & specta export
│               ├── error.rs               # Standardized AppError enum
│               ├── commands/
│               │   ├── mod.rs
│               │   ├── settings.rs
│               │   ├── profile.rs
│               │   ├── jobs.rs
│               │   ├── competency.rs
│               │   └── roadmap.rs
│               ├── core/
│               │   ├── mod.rs
│               │   ├── config.rs          # Model constants & worker URL
│               │   ├── db.rs              # SQLite pool/conn & migration runner
│               │   ├── keychain.rs        # OS Keychain BYOK wrapper
│               │   ├── matching.rs        # Cosine similarity & hybrid scoring
│               │   └── gemini.rs          # HTTP client Gemini + retry/backoff
│               └── models/
│                   ├── mod.rs
│                   ├── profile.rs
│                   ├── job.rs
│                   ├── competency.rs
│                   └── roadmap.rs
└── services/
    └── scraper-worker/                    # Python 3.13 Worker (uv)
        ├── pyproject.toml
        ├── app/
        │   ├── main.py
        │   ├── config.py
        │   ├── scrapers/
        │   │   ├── base.py
        │   │   ├── linkedin_guest.py
        │   │   ├── glints_api.py
        │   │   └── jobstreet_api.py
        │   └── pipeline/
        │       ├── cleaner.py
        │       └── schemas.py
        └── tests/
            ├── test_cleaner.py
            └── test_api.py

```

### 9.2 Taksonomi Error Terstandar (`src-tauri/src/error.rs`)

Semua Tauri command wajib mengembalikan `Result<T, AppError>` yang ter-serialisasi rapi ke frontend (`{ code: string, message: string, retryable: boolean }`):

| Error Code | Penyebab | `retryable` | Penanganan UI | 
| ----- | ----- | ----- | ----- | 
| `API_KEY_MISSING` | Pengguna belum mengatur Gemini API Key. | `false` | Arahkan ke `SettingsView`. | 
| `API_KEY_INVALID` | Gemini mengembalikan HTTP 400/401/403. | `false` | Minta pengguna memeriksa kunci di Google AI Studio. | 
| `GEMINI_RATE_LIMITED` | HTTP 429 setelah 3x retry habis. | `true` | Tampilkan jeda tunggu & tombol coba lagi. | 
| `GEMINI_PARSE_FAILED` | Respons JSON tidak memenuhi skema / korup. | `true` | Catat detail di log internal, tampilkan tombol ulangi. | 
| `SCRAPER_UNREACHABLE` | Python Worker di `127.0.0.1:8000` tidak merespons. | `true` | Tampilkan panduan menjalankan `uv run uvicorn app.main:app`. | 
| `DB_ERROR` | Kegagalan transaksi/constraint SQLite. | `false` | Tampilkan pesan ramah & catat ke *stderr* log. | 
| `VALIDATION_ERROR` | Input kosong atau terlalu pendek (misal cerita $< 30$ karakter). | `false` | Sorot input terkait di form. | 

### 9.3 Daftar Lengkap Tauri IPC Commands (`tauri-specta`)

| Command Name | Input Parameters | Return Type (`Result<T, AppError>`) | Efek Samping Utama | 
| ----- | ----- | ----- | ----- | 
| `get_api_key_status` | `()` | `ApiKeyStatus` (`has_key`, `masked_suffix`) | Baca status dari OS Keychain. | 
| `save_and_test_api_key` | `api_key: String` | `ApiKeyStatus` | Uji ke Gemini, simpan ke Keychain jika valid. | 
| `delete_api_key` | `()` | `()` | Hapus kredensial dari Keychain. | 
| `check_worker_health` | `worker_url: Option<String>` | `WorkerHealth` | `GET /health` ke Python Worker. | 
| `parse_user_story` | `raw_story: String` | `StoryProfileDraft` | Panggil Kontrak D (belum menulis ke DB). | 
| `save_confirmed_profile` | `input: ConfirmedProfileInput` | `UserProfileDetail` | Transaksi SQLite + Embedding + Recalculate Matches. | 
| `get_active_profile` | `()` | `Option<UserProfileDetail>` | Muat profil aktif, *skills*, dan *keywords*. | 
| `toggle_scrape_keyword` | `keyword_id: i64, is_active: bool` | `()` | Perbarui status centang keyword. | 
| `add_custom_keyword` | `user_id: String, role_label: String, queries: Vec<String>` | `ScrapeKeywordItem` | Tambah keyword `fit_type = 'custom'`. | 
| `ingest_manual_job` | `user_id: String, title_hint: Option<String>, url: Option<String>, raw_text: String` | `JobMatchDetail` | Kontrak A + Embedding + Simpan + Match. | 
| `run_scrape_for_active_keywords` | `user_id: String, sources: Vec<String>, limit_per_query: u32` | `ScrapeRunReport` | Panggil Python Worker $\rightarrow$ Deduplikasi $\rightarrow$ Kontrak A + Embedding $\rightarrow$ Match. | 
| `list_ranked_jobs` | `user_id: String, filter: JobFilterInput` | `Vec<JobMatchSummary>` | Query terurut `final_score DESC`. | 
| `get_job_detail` | `user_id: String, job_id: String` | `JobMatchDetail` | Ambil detail lowongan + *cache* gap jika ada. | 
| `toggle_save_job` | `user_id: String, job_id: String` | `bool` | Toggle `job_matches.is_saved`. | 
| `generate_gap_analysis` | `user_id: String, job_id: String, force_refresh: bool` | `GapAnalysis` | Panggil Kontrak B (jika belum ter-*cache* atau *force*). | 
| `list_available_roles` | `()` | `Vec<RoleJobCount>` | Daftar `normalized_role` & jumlah lowongan. | 
| `get_competency_map` | `user_id: String, target_role: String` | `CompetencyMapResponse` | Eksekusi query agregasi Peta Kompetensi. | 
| `generate_learning_roadmap` | `user_id: String, target_role: String, source_job_id: Option<String>, hours_per_week: u32` | `RoadmapDetail` | Panggil Kontrak C + Simpan ke 2 tabel roadmap. | 
| `list_roadmaps` | `user_id: String` | `Vec<RoadmapSummary>` | Daftar roadmap + persentase progres. | 
| `get_roadmap_detail` | `roadmap_id: String` | `RoadmapDetail` | Muat langkah-langkah terurut `phase_order`. | 
| `update_roadmap_step_status` | `step_id: String, status: StepStatus` | `StepUpdateResult` | Perbarui status + kembalikan `suggest_add_skill: Option<SkillRef>`. | 
| `quick_add_user_skill` | `user_id: String, skill_id: i64` | `()` | Tambah *skill* baru (`beginner`) + Recalculate Matches. | 

## 10. Strategi Pengujian & Verifikasi (TDD Matrix)

Agen wajib menulis dan menjalankan pengujian otomatis pada lapisan kritis berikut:

1. **Rust Unit Tests (`cargo test` di `apps/client/src-tauri`):**

   * `matching::tests`: Verifikasi *cosine similarity*, serialisasi/deserialisasi `Vec<f32>` *little-endian*, formula bobot $0.40/0.45/0.15$, dan seluruh 5 *edge cases* pada Section 6.3.

   * `db::tests`: Uji migrasi pada SQLite *in-memory* (`Connection::open_in_memory()`), *cascade delete*, deduplikasi `UNIQUE(source, external_id)`, dan akurasi query agregasi Peta Kompetensi.

   * `gemini::tests`: Uji *deserialization* fixture JSON untuk Kontrak A, B, C, dan D (termasuk penanganan field opsional kosong dan error JSON tidak valid).

2. **Python Unit Tests (`pytest` di `services/scraper-worker`):**

   * `test_cleaner.py`: Uji pembersihan HTML kotor, pemotongan panjang teks, dan determinisme hash `external_id`.

   * `test_api.py`: Uji `GET /health` dan `POST /api/v1/scrape` dengan *mock* adapter untuk memastikan *partial failure* mengembalikan `200 OK` + array `warnings` tanpa *crash*.

3. **Frontend Static & Build Checks (`apps/client`):**

   * `pnpm typecheck` (`vue-tsc --noEmit`) wajib lulus $0$ error setelah setiap perubahan `bindings.ts` atau komponen Vue.

   * Pemeriksaan visual manual pada ketiga tema (`paper-ink`, `swiss-mono`, `warm-stone`) pada ukuran layar Desktop ($1440\times 900$) dan Mobile ($390\times 844$).

## 11. Rencana Eksekusi Milestone & Definition of Done (DoD)

Agen harus mengeksekusi proyek secara berurutan per *milestone*. Jangan melompat ke *milestone* berikutnya sebelum seluruh perintah verifikasi pada *milestone* aktif lulus.

### Milestone 1 — Fondasi Monorepo, Multi-Theme Engine, SQLite & Gemini Client

* **Daftar Tugas:**

  1. Inisialisasi struktur folder monorepo `vecta/` sesuai Section 9.1.

  2. Setup Tauri 2.0 + Vue 3 + TypeScript + Tailwind v4 + `shadcn-vue` + `motion-v` + `@formkit/auto-animate` + `tauri-specta`.

  3. Implementasi `src/styles/themes.css`, skrip anti-FOUC `index.html`, composable `useTheme.ts`, `AppSidebar.vue`, `MobileBottomNav.vue`, dan `ThemeSwitcher.vue`.

  4. Implementasi `shared/migrations/001_initial_schema.sql`, `core/db.rs`, `error.rs`, `core/keychain.rs`, dan `core/gemini.rs` (termasuk tes koneksi & fungsi *embedding*).

  5. Bangun `SettingsView.vue` lengkap (input/tes BYOK + *switcher* 3 tema + cek status *worker*).

* **Definition of Done & Verifikasi:**

  * `cargo test` lulus (migrasi DB + deserialization *fixture* Gemini).

  * `pnpm typecheck` lulus $0$ error dan `src/lib/bindings.ts` ter-generate otomatis.

  * `pnpm tauri dev` berjalan di Desktop: ganti tema 3x berlangsung mulus tanpa FOUC saat *reload*, dan API Key valid tersimpan ke OS Keychain.

### Milestone 2 — Alur Story-to-Chips, Hybrid Matching & Manual Job Ingest

* **Daftar Tugas:**

  1. Implementasi Kontrak D (`parse_user_story`) & `save_confirmed_profile` (lengkap dengan normalisasi *skill* dan *embedding* 768-dim).

  2. Bangun `OnboardingStoryView.vue` dengan kanvas tulis tenang, 3 contoh cepat, animasi *stagger* kemunculan *chip*, edit level 1-klik, dan seleksi *keyword scraping*.

  3. Implementasi `core/matching.rs` lengkap dengan seluruh *edge cases* dan unit test.

  4. Implementasi Kontrak A (`ingest_manual_job`) dan modal `ManualJobModal.vue` agar alur **Cerita** $\rightarrow$ **Profil** $\rightarrow$ **Tambah Lowongan** $\rightarrow$ **Skor Matching** dapat diuji penuh secara *end-to-end*.

* **Definition of Done & Verifikasi:**

  * `cargo test` lulus 100% untuk `matching::tests` (termasuk seluruh *edge cases* Section 6.3).

  * Cerita bebas 1 paragraf berhasil berubah menjadi *chip* interaktif, tersimpan ke SQLite, dan 3 lowongan yang ditempel manual langsung terurut sesuai skor hibrida.

### Milestone 3 — Python Scraper Worker, Jobs Feed & Dynamic Competency Map

* **Daftar Tugas:**

  1. Inisialisasi `services/scraper-worker` dengan `uv`, FastAPI, `pipeline/cleaner.py`, `scrapers/linkedin_guest.py`, dan minimal 1 portal lokal (`glints_api.py` atau `jobstreet_api.py`).

  2. Implementasi command `run_scrape_for_active_keywords` di Rust (dengan pengecekan deduplikasi sebelum memanggil Kontrak A).

  3. Bangun `JobsFeedView.vue` (*master-detail split-pane*, `MatchRing.vue` animatif, filter peran, dan *empty/loading/degraded states*).

  4. Bangun `CompetencyMapView.vue` lengkap dengan filter 4 kategori, *toggle* "Hanya Tampilkan Gap" (`v-auto-animate`), dan `DashboardView.vue`.

* **Definition of Done & Verifikasi:**

  * `pytest` di `services/scraper-worker` lulus 100% (termasuk simulasi *partial failure*).

  * Klik "Refresh Lowongan" dari Tauri mengambil lowongan nyata, melewati duplikat pada klik kedua (0 panggilan Gemini untuk duplikat), dan `CompetencyMapView` menampilkan persentase permintaan *skill* akurat.

### Milestone 4 — On-Demand Gap Analysis, AI Career Roadmap & Mobile Build Prep

* **Daftar Tugas:**

  1. Implementasi Kontrak B (`generate_gap_analysis`) dengan mekanisme *caching* SQLite + komponen `GapAnalysisPanel.vue`.

  2. Implementasi Kontrak C (`generate_learning_roadmap`) dan `RoadmapView.vue` (*timeline* interaktif + *toast* auto-tambah *skill* saat tahap selesai dicentang).

  3. Audit visual menyeluruh di ketiga tema (`paper-ink`, `swiss-mono`, `warm-stone`) pada *viewport* Desktop dan Mobile.

  4. Verifikasi kompilasi rilis Desktop (`pnpm tauri build`) dan inisialisasi target Android (`pnpm tauri android init`).

* **Definition of Done & Verifikasi:**

  * Seluruh tes (`cargo test`, `pytest`, `pnpm typecheck`) hijau.

  * Membuka analisis gap lowongan kedua kalinya memuat dari *cache* ($0$ panggilan Gemini); mencentang langkah roadmap memunculkan tawaran tambah *skill* yang otomatis memperbarui skor *matching*.

### Milestone 5 — Fase 2: Portfolio Showcase Polish (Pasca-MVP)

* Integrasi replikasi **Turso (LibSQL)** untuk sinkronisasi Desktop $\leftrightarrow$ Mobile.

* GitHub Actions Cron terjadwal untuk *scraping* otomatis setiap 12 jam.

* Penambahan *Radar Chart* (`vue-echarts`), komparasi dua profesi, dan Chrome Extension *1-Click Capture* (Manifest V3).

* Penyusunan `README.md` portofolio dengan diagram arsitektur, *showcase* ketiga tema, GIF demo transisi, dan panduan setup BYOK.