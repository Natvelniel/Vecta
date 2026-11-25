<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useSettingsStore } from "@/stores/settings";
import { useToast } from "@/composables/useToast";
import { openExternalUrl } from "@/lib/ipc";
import PageHeader from "@/components/layout/PageHeader.vue";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";
import VIcon from "@/components/ui/VIcon.vue";

const settingsStore = useSettingsStore();
const toast = useToast();

const newApiKey = ref("");
const confirmDelete = ref(false);
const workerUrlInput = ref(settingsStore.workerUrl);

onMounted(async () => {
  await Promise.all([
    settingsStore.loadApiKeyStatus(),
    settingsStore.loadOfflineEngineStatus(),
  ]);
  workerUrlInput.value = settingsStore.workerUrl;
  if (!settingsStore.workerHealth && !settingsStore.checkingWorker) {
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
</script>

<template>
  <div class="v-settings-root">
    <PageHeader title="Pengaturan" subtitle="Konfigurasi lokal · Zero secrets di SQLite" />

    <div class="v-narrow-container">
      <!-- Offline Multilingual AI Engine Status -->
      <VCard>
        <div class="v-sec-head">
          <div>
            <h2 class="t-section">Mesin AI Semantik Offline (Tanpa API Key)</h2>
            <p class="t-secondary" style="margin-top: 4px">
              Model multilingual 101 bahasa (termasuk Bahasa Indonesia) + taksonomi 14 rumpun prodi
              berjalan 100% lokal di perangkatmu.
            </p>
          </div>
          <VBadge :tone="settingsStore.localAiStatus.is_ready ? 'success' : 'neutral'" class="t-num">
            <span>
              {{
                settingsStore.localAiStatus.is_ready
                  ? `Aktif · ${settingsStore.localAiStatus.dimensions} dim · ${settingsStore.localAiStatus.size_mb} MB`
                  : "Mode Taksonomi Lokal"
              }}
            </span>
          </VBadge>
        </div>

        <div class="v-sec-foot" style="margin-top: var(--space-4)">
          <span class="t-meta">Mode Pengambil Lowongan:</span>
          <div style="display: flex; gap: 8px; flex-wrap: wrap">
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

      <!-- Python Scraper Worker -->
      <VCard>
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
  min-width: 220px;
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
@media (max-width: 1023px) {
  .v-narrow-container {
    padding: var(--space-5) var(--space-4) var(--space-10);
  }
}
</style>
