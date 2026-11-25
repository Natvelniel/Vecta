import { defineStore } from "pinia";
import { ref } from "vue";
import type { ApiKeyStatus_Serialize, LocalAiModelStatus, WorkerHealth } from "@/lib/bindings";
import { ipc, IpcError } from "@/lib/ipc";

const DEFAULT_WORKER_URL = "http://127.0.0.1:8000";

export const useSettingsStore = defineStore("settings", () => {
  const apiKeyStatus = ref<ApiKeyStatus_Serialize>({
    has_key: false,
    masked_suffix: null,
  });
  const localAiStatus = ref<LocalAiModelStatus>({
    model_id: "local:potion-m128-i8",
    is_ready: false,
    dimensions: 0,
    vocab_size: 0,
    size_mb: 0,
    source_path: null,
  });
  const scraperMode = ref<string>("native_rust");
  const workerUrl = ref<string>(DEFAULT_WORKER_URL);
  const workerHealth = ref<WorkerHealth | null>(null);
  const workerError = ref<string | null>(null);
  const loadingKey = ref(false);
  const checkingWorker = ref(false);
  const error = ref<string | null>(null);

  async function loadApiKeyStatus() {
    error.value = null;
    try {
      apiKeyStatus.value = await ipc.getApiKeyStatus();
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memeriksa status kunci API.";
    }
  }

  async function loadOfflineEngineStatus() {
    try {
      const [ai, mode] = await Promise.all([ipc.getLocalAiStatus(), ipc.getScraperMode()]);
      localAiStatus.value = ai;
      scraperMode.value = mode;
    } catch {
      // Keep safe defaults
    }
  }

  async function updateScraperMode(mode: "native_rust" | "python_worker"): Promise<boolean> {
    try {
      scraperMode.value = await ipc.setScraperMode(mode);
      return true;
    } catch {
      return false;
    }
  }

  async function saveAndTestApiKey(rawKey: string): Promise<boolean> {
    loadingKey.value = true;
    error.value = null;
    try {
      apiKeyStatus.value = await ipc.saveAndTestApiKey(rawKey);
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memverifikasi kunci API Gemini.";
      return false;
    } finally {
      loadingKey.value = false;
    }
  }

  async function deleteApiKey(): Promise<boolean> {
    loadingKey.value = true;
    error.value = null;
    try {
      await ipc.deleteApiKey();
      apiKeyStatus.value = { has_key: false, masked_suffix: null };
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menghapus kunci API.";
      return false;
    } finally {
      loadingKey.value = false;
    }
  }

  async function checkWorkerHealth(customUrl?: string): Promise<boolean> {
    checkingWorker.value = true;
    workerError.value = null;
    const target = customUrl !== undefined ? customUrl.trim() : workerUrl.value.trim();
    try {
      const health = await ipc.checkWorkerHealth(target || null);
      workerHealth.value = health;
      if (target) workerUrl.value = target;
      return true;
    } catch (e) {
      workerHealth.value = null;
      workerError.value =
        e instanceof IpcError
          ? e.message
          : "Python Scraper Worker tidak merespons di alamat lokal.";
      return false;
    } finally {
      checkingWorker.value = false;
    }
  }

  return {
    apiKeyStatus,
    localAiStatus,
    scraperMode,
    workerUrl,
    workerHealth,
    workerError,
    loadingKey,
    checkingWorker,
    error,
    loadApiKeyStatus,
    loadOfflineEngineStatus,
    updateScraperMode,
    saveAndTestApiKey,
    deleteApiKey,
    checkWorkerHealth,
  };
});
