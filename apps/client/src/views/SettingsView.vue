<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useJobsStore } from "@/stores/jobs";
import { useProfileStore } from "@/stores/profile";
import { useSettingsStore } from "@/stores/settings";
import { useTheme } from "@/composables/useTheme";
import { useToast } from "@/composables/useToast";
import { ipc, IpcError, openExternalUrl } from "@/lib/ipc";
import PageHeader from "@/components/layout/PageHeader.vue";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";
import VIcon from "@/components/ui/VIcon.vue";

const settingsStore = useSettingsStore();
const profileStore = useProfileStore();
const jobsStore = useJobsStore();
const { theme, setTheme } = useTheme();
const toast = useToast();
const isAndroid = typeof navigator !== "undefined" && /android/i.test(navigator.userAgent);

const newApiKey = ref("");
const confirmDelete = ref(false);
const workerUrlInput = ref(settingsStore.workerUrl);
const backupInputRef = ref<HTMLInputElement | null>(null);
const exportingBackup = ref(false);
const importingBackup = ref(false);

onMounted(async () => {
  await Promise.all([
    settingsStore.loadApiKeyStatus(),
    settingsStore.loadOfflineEngineStatus(),
  ]);
  workerUrlInput.value = settingsStore.workerUrl;
  if (!isAndroid && !settingsStore.workerHealth && !settingsStore.checkingWorker) {
    void settingsStore.checkWorkerHealth(workerUrlInput.value);
  }
});

async function handleSwitchScraperMode(mode: "native_rust" | "python_worker") {
  const ok = await settingsStore.updateScraperMode(mode);
  if (ok) {
    toast.show(
      mode === "native_rust"
        ? "Scraper Bawaan (Native Rust, tanpa Python) diaktifkan."
        : "Python Scraper Worker opsional diaktifkan.",
      { tone: "success" },
    );
    void settingsStore.checkWorkerHealth(workerUrlInput.value);
  }
}

async function handleSaveKey() {
  const ok = await settingsStore.saveAndTestApiKey(newApiKey.value);
  if (ok) {
    newApiKey.value = "";
    confirmDelete.value = false;
    toast.show("Kunci API Gemini valid dan tersimpan aman di OS Credential Manager.", {
      tone: "success",
    });
  }
}

async function handleDeleteKey() {
  const ok = await settingsStore.deleteApiKey();
  if (ok) {
    confirmDelete.value = false;
    toast.show("Kunci API Gemini dihapus dari OS Keychain.", { tone: "info" });
  }
}

async function handleCheckWorker() {
  const ok = await settingsStore.checkWorkerHealth(workerUrlInput.value);
  if (ok && settingsStore.workerHealth) {
    toast.show(
      `Worker lokal merespons sehat (${settingsStore.workerHealth.available_sources.join(", ")}).`,
      { tone: "success" },
    );
  } else if (settingsStore.workerError) {
    toast.show(settingsStore.workerError, { tone: "warning" });
  }
}

async function handleExportBackup() {
  exportingBackup.value = true;
  try {
    const rep = await ipc.exportVectaBackup();
    const blob = new Blob([rep.payload_json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = rep.file_name || "vecta-backup.vecta";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    toast.show(
      `Backup siap diunduh (${rep.profiles_count} profil · ${rep.watchlists_count} watchlist · tanpa API key).`,
      { tone: "success" },
    );
  } catch (e) {
    toast.show(e instanceof IpcError ? e.message : "Gagal mengekspor backup.", { tone: "error" });
  } finally {
    exportingBackup.value = false;
  }
}

async function handleImportBackupFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  importingBackup.value = true;
  try {
    const text = await file.text();
    const rep = await ipc.importVectaBackup(text);
    await Promise.all([profileStore.loadActiveProfile(), jobsStore.loadSearchPreferences()]);
    toast.show(
      `Backup berhasil dipulihkan: ${rep.imported_profiles} profil, ${rep.imported_watchlists} watchlist.`,
      { tone: "success" },
    );
  } catch (e) {
    toast.show(e instanceof IpcError ? e.message : "Format file .vecta tidak valid.", {
      tone: "error",
    });
  } finally {
    importingBackup.value = false;
  }
}
</script>

<template>
  <div class="v-settings-root">
    <PageHeader title="Pengaturan" subtitle="Konfigurasi lokal · Zero secrets di SQLite" />

    <div class="v-narrow-container">
      <!-- Dual Theme Selector -->
      <VCard>
        <div class="v-sec-head">
          <div>
            <h2 class="t-section">Tema Tampilan Vecta</h2>
            <p class="t-secondary" style="margin-top: 4px">
              Pilih antara Obsidian Hitam fokus tinggi atau Putih Swiss kontras editorial.
            </p>
          </div>
          <VBadge tone="neutral">{{ theme === "dark" ? "Obsidian Hitam" : "Putih Swiss" }}</VBadge>
        </div>

        <div class="v-theme-grid" style="margin-top: var(--space-4)">
          <button
            type="button"
            class="v-theme-card"
            :class="{ 'v-theme-card--active': theme === 'dark' }"
            @click="setTheme('dark', { x: $event.clientX, y: $event.clientY })"
          >
            <span class="v-theme-swatch v-theme-swatch--dark">
              <span />
              <i />
            </span>
            <span>
              <strong style="display: block; color: var(--text-1)">Hitam · Obsidian Studio</strong>
              <span class="t-meta">Fokus nyaman · Aksen indigo &amp; emerald</span>
            </span>
          </button>

          <button
            type="button"
            class="v-theme-card"
            :class="{ 'v-theme-card--active': theme === 'light' }"
            @click="setTheme('light', { x: $event.clientX, y: $event.clientY })"
          >
            <span class="v-theme-swatch v-theme-swatch--light">
              <span />
              <i />
            </span>
            <span>
              <strong style="display: block; color: var(--text-1)">Putih · Swiss Alabaster</strong>
              <span class="t-meta">Bersih &amp; cerah · Hitam pekat &amp; teal</span>
            </span>
          </button>
        </div>
      </VCard>

      <!-- Portable Backup & Migration (.vecta) -->
      <VCard>
        <div class="v-sec-head">
          <div>
            <h2 class="t-section">Backup &amp; Migrasi Data (.vecta)</h2>
            <p class="t-secondary" style="margin-top: 4px">
              Ekspor profil, skill, CV master, preferensi multi-kota, dan watchlist. Kunci API Gemini
              tidak pernah disertakan.
            </p>
          </div>
          <VBadge tone="success">Portabel 0-Secret</VBadge>
        </div>

        <div class="v-sec-foot" style="margin-top: var(--space-4)">
          <input
            ref="backupInputRef"
            type="file"
            accept=".vecta,.json,application/json"
            style="display: none"
            @change="handleImportBackupFile"
          />
          <VButton variant="primary" :loading="exportingBackup" @click="handleExportBackup">
            <VIcon name="download" :size="14" />
            <span>Ekspor Backup (.vecta)</span>
          </VButton>
          <VButton
            variant="secondary"
            :loading="importingBackup"
            @click="backupInputRef?.click()"
          >
            <VIcon name="upload" :size="14" />
            <span>Impor Backup (.vecta)</span>
          </VButton>
        </div>
      </VCard>

      <!-- Offline Multilingual AI Engine Status -->
      <VCard>
        <div class="v-sec-head">
          <div>
            <h2 class="t-section">Mesin AI Semantik Offline (Tanpa API Key)</h2>
            <p class="t-secondary" style="margin-top: 4px">
              <template v-if="settingsStore.localAiStatus.engine_tier === 'neural_v2'">
                Vecta Neural Engine v2: Bi-Encoder XLM-R {{ settingsStore.localAiStatus.dimensions }} dim,
                {{ settingsStore.localAiStatus.reranker_ready ? "Cross-Encoder Reranker aktif" : "Reranker nonaktif" }},
                dan {{ settingsStore.localAiStatus.semantic_index_ready ? "Semantic Index 917 skill / 238 peran siap" : "Taksonomi lokal" }}.
              </template>
              <template v-else>
                Model multilingual 101 bahasa (termasuk Bahasa Indonesia) + taksonomi 23 rumpun prodi
                (917 skill) berjalan 100% lokal di perangkatmu.
              </template>
            </p>
          </div>
          <VBadge :tone="settingsStore.localAiStatus.is_ready ? 'success' : 'neutral'" class="t-num">
            <span>
              {{
                settingsStore.localAiStatus.is_ready
                  ? `${settingsStore.localAiStatus.engine_tier === "neural_v2" ? "Neural v2" : "Aktif"} · ${settingsStore.localAiStatus.dimensions} dim · ${settingsStore.localAiStatus.size_mb} MB`
                  : "Mode Taksonomi Lokal"
              }}
            </span>
          </VBadge>
        </div>

        <div class="v-sec-foot" style="margin-top: var(--space-4)">
          <span class="t-meta">Mode Pengambil Lowongan:</span>
          <div style="display: flex; gap: 8px; flex-wrap: wrap">
            <VBadge v-if="isAndroid" tone="success">Native Rust Scraper Aktif</VBadge>
            <template v-else>
              <VButton
                :variant="settingsStore.scraperMode === 'native_rust' ? 'primary' : 'secondary'"
                size="compact"
                @click="handleSwitchScraperMode('native_rust')"
              >
                Native Rust (Bawaan · Tanpa Python)
              </VButton>
              <VButton
                :variant="settingsStore.scraperMode === 'python_worker' ? 'primary' : 'secondary'"
                size="compact"
                @click="handleSwitchScraperMode('python_worker')"
              >
                Python Worker (Opsional)
              </VButton>
            </template>
          </div>
        </div>
      </VCard>

      <!-- Section 9.4: BYOK Gemini API Key -->
      <VCard>
        <div class="v-sec-head">
          <div>
            <h2 class="t-section">Kunci API Google Gemini</h2>
            <p class="t-secondary" style="margin-top: 4px">
              Disimpan melalui OS Credential Manager perangkatmu dan tidak pernah ditampilkan penuh.
            </p>
          </div>
          <VBadge :tone="settingsStore.apiKeyStatus.has_key ? 'success' : 'warning'" class="t-num">
            <span
              class="v-dot"
              :style="{
                background: settingsStore.apiKeyStatus.has_key
                  ? 'var(--status-success)'
                  : 'var(--status-warning)',
              }"
            />
            <span>
              {{
                settingsStore.apiKeyStatus.has_key
                  ? settingsStore.apiKeyStatus.masked_suffix || "Tersimpan"
                  : "Belum terhubung"
              }}
            </span>
          </VBadge>
        </div>

        <form style="margin-top: var(--space-5)" @submit.prevent="handleSaveKey">
          <label for="settings-api-key" class="v-field-label">
            Ganti atau simpan kunci API baru
          </label>
          <div class="v-inline-field">
            <input
              id="settings-api-key"
              v-model="newApiKey"
              type="password"
              class="v-input t-num"
              placeholder="Kosongkan jika tetap memakai kunci tersimpan"
              autocomplete="off"
            />
            <VButton
              type="submit"
              variant="primary"
              size="tall"
              :loading="settingsStore.loadingKey"
            >
              {{ settingsStore.loadingKey ? "Memverifikasi..." : "Tes & Simpan" }}
            </VButton>
          </div>
          <p v-if="settingsStore.error" class="v-err-text" role="alert">
            {{ settingsStore.error }}
          </p>
        </form>

        <div class="v-sec-foot">
          <VButton
            variant="tertiary"
            size="compact"
            style="padding-left: 0"
            @click="openExternalUrl('https://aistudio.google.com/apikey')"
          >
            <span>Dapatkan kunci di Google AI Studio</span>
            <VIcon name="external" :size="13" />
          </VButton>

          <template v-if="settingsStore.apiKeyStatus.has_key">
            <VButton
              v-if="!confirmDelete"
              variant="destructive"
              size="compact"
              @click="confirmDelete = true"
            >
              Hapus Kunci
            </VButton>
            <div v-else class="v-confirm-row">
              <span class="t-meta" style="color: var(--status-error)">
                Yakin hapus dari Keychain?
              </span>
              <VButton
                variant="destructive"
                size="compact"
                :loading="settingsStore.loadingKey"
                @click="handleDeleteKey"
              >
                Ya, Hapus
              </VButton>
              <VButton variant="secondary" size="compact" @click="confirmDelete = false">
                Batal
              </VButton>
            </div>
          </template>
        </div>
      </VCard>

      <!-- Python Scraper Worker (Desktop only) -->
      <VCard v-if="!isAndroid">
        <div class="v-sec-head">
          <div>
            <h2 class="t-section">Python Scraper Worker</h2>
            <p class="t-secondary" style="margin-top: 4px">
              Worker lokal untuk mengambil lowongan dari LinkedIn Guest, Glints, dan JobStreet.
            </p>
          </div>
          <VBadge :tone="settingsStore.workerHealth ? 'success' : 'warning'" class="t-num">
            <span
              class="v-dot"
              :style="{
                background: settingsStore.workerHealth
                  ? 'var(--status-success)'
                  : 'var(--status-warning)',
              }"
            />
            <span>
              {{
                settingsStore.workerHealth
                  ? `Online v${settingsStore.workerHealth.version}`
                  : "Belum dicek / Offline"
              }}
            </span>
          </VBadge>
        </div>

        <form style="margin-top: var(--space-5)" @submit.prevent="handleCheckWorker">
          <label for="settings-worker-url" class="v-field-label">URL Worker Lokal</label>
          <div class="v-inline-field">
            <input
              id="settings-worker-url"
              v-model="workerUrlInput"
              type="url"
              class="v-input t-num"
              placeholder="http://127.0.0.1:8000"
            />
            <VButton
              type="submit"
              variant="secondary"
              size="tall"
              :loading="settingsStore.checkingWorker"
            >
              {{ settingsStore.checkingWorker ? "Memeriksa..." : "Cek Koneksi" }}
            </VButton>
          </div>
          <p v-if="settingsStore.workerError" class="v-warn-text" role="status">
            {{ settingsStore.workerError }}
          </p>
        </form>
      </VCard>
    </div>
  </div>
</template>

<style scoped>
.v-settings-root {
  display: flex;
  flex-direction: column;
  flex: 1;
}
.v-narrow-container {
  max-width: 800px;
  width: 100%;
  margin: 0 auto;
  padding: var(--space-8);
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
}
.v-sec-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-dot {
  width: 6px;
  height: 6px;
  border-radius: var(--radius-full);
}
.v-field-label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}
.v-inline-field {
  display: flex;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-input {
  flex: 1;
  min-width: 0;
  width: 100%;
  height: 44px;
  padding: 0 var(--space-4);
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 1px solid var(--border-input);
  border-radius: var(--radius-control);
  font-size: 14px;
}
.v-input:focus {
  border-color: var(--brand-hover);
}
.v-err-text {
  margin: var(--space-3) 0 0;
  font-size: 13px;
  color: var(--status-error);
}
.v-warn-text {
  margin: var(--space-3) 0 0;
  font-size: 13px;
  color: var(--status-warning);
}
.v-sec-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
  margin-top: var(--space-4);
}
.v-confirm-row {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}
.v-theme-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-3);
}
.v-theme-card {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3);
  border-radius: var(--radius-card);
  border: 1px solid var(--border);
  background: var(--surface-2);
  text-align: left;
  transition: border-color 150ms ease, background-color 150ms ease, transform 110ms ease;
}
.v-theme-card:active {
  transform: scale(0.985);
}
.v-theme-card--active {
  border-color: var(--brand);
  background: var(--brand-soft);
}
.v-theme-swatch {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  padding: 5px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  flex-shrink: 0;
  border: 1px solid var(--border-strong);
}
.v-theme-swatch--dark {
  background: #09090b;
}
.v-theme-swatch--dark span {
  display: block;
  width: 100%;
  height: 7px;
  border-radius: 4px;
  background: #5e6ad2;
}
.v-theme-swatch--dark i {
  display: block;
  width: 65%;
  height: 5px;
  border-radius: 3px;
  background: #10b981;
}
.v-theme-swatch--light {
  background: #f5f6f8;
}
.v-theme-swatch--light span {
  display: block;
  width: 100%;
  height: 7px;
  border-radius: 4px;
  background: #0d0e12;
}
.v-theme-swatch--light i {
  display: block;
  width: 65%;
  height: 5px;
  border-radius: 3px;
  background: #0d9488;
}
@media (max-width: 640px) {
  .v-theme-grid {
    grid-template-columns: 1fr;
  }
}
@media (max-width: 1023px) {
  .v-narrow-container {
    padding: var(--space-5) var(--space-4) var(--space-10);
  }
}
</style>
