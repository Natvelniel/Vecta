# Panduan Build Mobile Vecta V3 (Tauri 2.0 Android & iOS)

Seluruh inti Rust Vecta V3 dirancang **100% pure-Rust** agar siap dikompilasi ke Android (`aarch64-linux-android`) dan iOS (`aarch64-apple-ios`) tanpa ketergantungan Python Worker maupun C++ ONNX Runtime:

- **Taksonomi 14 Rumpun Prodi (`vecta-taxonomy`):** Pure Rust + `aho-corasick` (< 400 KB).
- **Scraper Bawaan (`native_scraper`):** Pure Rust (`reqwest` dengan `rustls-tls` + `scraper`).
- **Offline Multilingual AI (`local_embedder`):** Pure Rust (`tokenizers` dengan fitur `fancy-regex` tanpa `onig` C library + bobot INT8 `embeddings_i8.vemb` ~124 MB).
- **Import & Export CV ATS PDF (`cv_import`, `cv_export`):** Pure Rust (`pdf-extract` + `typst-as-lib` + `typst-pdf` + font open-source tertanam).
- **OS Keychain Gate:** `keyring` otomatis di-gate khusus desktop (`#[cfg(not(any(target_os = "android", target_os = "ios")))]`).
- **Mobile Safe-Area CSS:** `AppShell.vue` sudah mendukung `env(safe-area-inset-top)` dan `env(safe-area-inset-bottom)` serta navigasi bottom-bar responsif.

---

## Prasyarat Android (Windows / macOS / Linux)

1. Instal **Android Studio** (API 34+ SDK, Android NDK r26+, dan Build-Tools).
2. Atur environment variable:
   - `ANDROID_HOME` (mis. `C:\Users\<User>\AppData\Local\Android\Sdk`)
   - `NDK_HOME` (mis. `%ANDROID_HOME%\ndk\<versi>`)
   - `JAVA_HOME` (JDK 17 dari Android Studio JBR)
3. Tambahkan target Rust Android:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi
   ```

## Langkah Inisialisasi & Build Android APK / AAB

```bash
cd apps/client
npm.cmd run tauri android init
npm.cmd run tauri android build -- --apk --target aarch64
```

Perkiraan ukuran hasil build Android (`arm64-v8a`):
- **Click-to-Install Release APK (`arm64-v8a`, via GitHub Actions `release.yml`):** ~25 - 35 MB (ringan untuk unduhan jaringan seluler, tanpa perlu ekstrak ZIP; mencakup taksonomi 14 domain, scraper Rust 5 portal, red-flag scam detector, import CV PDF, dan ekspor PDF ATS Typst dengan font tertanam).
- **Full Offline APK dengan model 101-bahasa (build lokal opsional):** ~155 - 170 MB jika menyertakan `resources/models/potion-m128-i8/`.

## Automated Signed APK via GitHub Actions

Workflow `.github/workflows/release.yml` otomatis membangun dan menandatangani file `Vecta_<tag>_android_arm64.apk` di runner `ubuntu-latest` setiap kali tag `v*` di-push.

Untuk memastikan update versi berikutnya dapat di-install menimpa versi lama (in-place upgrade) tanpa uninstall terlebih dahulu, tambahkan 4 GitHub Repository Secrets berikut di **Settings -> Secrets and variables -> Actions**:
- `ANDROID_KEYSTORE_BASE64` (keystore `.jks` dalam format Base64)
- `ANDROID_KEYSTORE_PASSWORD`
- `ANDROID_KEY_ALIAS`
- `ANDROID_KEY_PASSWORD`

Jika 4 secrets tersebut belum diisi, CI tetap menghasilkan APK signed yang valid agar pengguna Android bisa langsung klik dan install.
