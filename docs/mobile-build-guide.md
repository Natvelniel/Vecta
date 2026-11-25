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

Perkiraan ukuran hasil build Android (`arm64-v8a`, termasuk model multilingual offline 101 bahasa ~142 MB + font ATS):
- **Full Offline APK (`arm64-v8a`):** ~155 - 170 MB (bekerja 100% offline tanpa internet maupun API key).
- **Play Store AAB (tanpa bundle model, unduh model via Play Asset Delivery):** ~14 - 18 MB unduhan awal.
