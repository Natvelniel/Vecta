import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type {
  AiMatchRunReport,
  ApplicationStatus,
  EnrichmentStatus,
  JobSource,
  ScrapeRunReport,
  SearchPreferences,
  WorkMode,
} from "@/lib/bindings";
import {
  ipc,
  IpcError,
  type NormalizedJobDetail,
  type NormalizedJobSummary,
  type ScrapeProgressEvent,
} from "@/lib/ipc";

const PAGE_SIZE = 50;

const DEFAULT_PREFS: SearchPreferences = {
  location: "Indonesia",
  sources: ["jobstreet", "linkedin", "indeed"],
  limit_per_query: 25,
  max_pages: 2,
  posted_within_days: 14,
  work_mode: "any",
  custom_queries: [],
  ai_enrich_enabled: false,
  ai_enrich_max_jobs: 15,
  ai_rpm_limit: 10,
};

export const useJobsStore = defineStore("jobs", () => {
  const jobs = ref<NormalizedJobSummary[]>([]);
  const selectedJobId = ref<string | null>(null);
  const selectedDetail = ref<NormalizedJobDetail | null>(null);

  const searchPreferences = ref<SearchPreferences>({ ...DEFAULT_PREFS });
  const roleFilter = ref<string | null>(null);
  const workModeFilter = ref<WorkMode | null>(null);
  const sourceFilter = ref<JobSource | null>(null);
  const enrichmentFilter = ref<EnrichmentStatus | null>(null);
  const applicationStatusFilter = ref<ApplicationStatus | null>(null);
  const locationFilter = ref<string>("");
  const searchQuery = ref<string>("");
  const savedOnly = ref<boolean>(false);
  const hasMore = ref<boolean>(false);

  const loadingList = ref(false);
  const loadingMore = ref(false);
  const loadingDetail = ref(false);
  const scraping = ref(false);
  const runningAiQueue = ref(false);
  const enrichingJob = ref(false);
  const analyzingGap = ref(false);
  const generatingApplyKit = ref(false);
  const updatingApplication = ref(false);
  const ingestingManual = ref(false);

  const progress = ref<ScrapeProgressEvent | null>(null);
  const lastScrapeReport = ref<ScrapeRunReport | null>(null);
  const lastAiReport = ref<AiMatchRunReport | null>(null);
  const error = ref<string | null>(null);

  const pendingLocalCount = computed(
    () => jobs.value.filter((j) => j.enrichment_status === "local_only").length,
  );

  function buildFilter(offset: number) {
    const trimmedSearch = searchQuery.value.trim();
    const trimmedLoc = locationFilter.value.trim();
    return {
      normalized_role: roleFilter.value,
      work_mode: workModeFilter.value,
      source: sourceFilter.value,
      saved_only: savedOnly.value ? true : null,
      min_score: null,
      search_query: trimmedSearch ? trimmedSearch : null,
      location_query: trimmedLoc ? trimmedLoc : null,
      enrichment_status: enrichmentFilter.value,
      application_status: applicationStatusFilter.value,
      include_dismissed: applicationStatusFilter.value === "dismissed" ? true : null,
      limit: PAGE_SIZE,
      offset,
    };
  }

  async function loadSearchPreferences() {
    try {
      searchPreferences.value = await ipc.getSearchPreferences();
    } catch {
      searchPreferences.value = { ...DEFAULT_PREFS };
    }
  }

  async function saveSearchPreferences(prefs: SearchPreferences): Promise<boolean> {
    error.value = null;
    try {
      searchPreferences.value = await ipc.saveSearchPreferences(prefs);
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menyimpan preferensi pencarian.";
      return false;
    }
  }

  async function loadJobs(userId: string) {
    loadingList.value = true;
    error.value = null;
    try {
      const list = await ipc.listRankedJobs(userId, buildFilter(0));
      jobs.value = list;
      hasMore.value = list.length === PAGE_SIZE;
      if (list.length === 0) {
        selectedJobId.value = null;
        selectedDetail.value = null;
      } else {
        const stillExists = list.some((j) => j.job_id === selectedJobId.value);
        const targetId = stillExists && selectedJobId.value ? selectedJobId.value : list[0].job_id;
        await selectJob(userId, targetId);
      }
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memuat daftar lowongan.";
    } finally {
      loadingList.value = false;
    }
  }

  async function loadMoreJobs(userId: string) {
    if (loadingMore.value || !hasMore.value) return;
    loadingMore.value = true;
    error.value = null;
    try {
      const nextBatch = await ipc.listRankedJobs(userId, buildFilter(jobs.value.length));
      if (nextBatch.length > 0) {
        const existingIds = new Set(jobs.value.map((j) => j.job_id));
        const uniqueNext = nextBatch.filter((j) => !existingIds.has(j.job_id));
        jobs.value = [...jobs.value, ...uniqueNext];
      }
      hasMore.value = nextBatch.length === PAGE_SIZE;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memuat lowongan tambahan.";
    } finally {
      loadingMore.value = false;
    }
  }

  async function selectJob(userId: string, jobId: string) {
    selectedJobId.value = jobId;
    loadingDetail.value = true;
    try {
      selectedDetail.value = await ipc.getJobDetail(userId, jobId);
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memuat detail lowongan.";
    } finally {
      loadingDetail.value = false;
    }
  }

  async function toggleSave(userId: string, jobId: string): Promise<boolean | null> {
    try {
      const nextSaved = await ipc.toggleSaveJob(userId, jobId);
      const item = jobs.value.find((j) => j.job_id === jobId);
      if (item) {
        item.is_saved = nextSaved;
        if (nextSaved && item.application_status === "new") item.application_status = "saved";
        if (!nextSaved && item.application_status === "saved") item.application_status = "new";
      }
      if (selectedDetail.value && selectedDetail.value.job_id === jobId) {
        selectedDetail.value.is_saved = nextSaved;
        if (nextSaved && selectedDetail.value.application_status === "new") {
          selectedDetail.value.application_status = "saved";
        }
        if (!nextSaved && selectedDetail.value.application_status === "saved") {
          selectedDetail.value.application_status = "new";
        }
      }
      if (savedOnly.value && !nextSaved) {
        await loadJobs(userId);
      }
      return nextSaved;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memperbarui status simpan.";
      return null;
    }
  }

  async function updateApplication(
    userId: string,
    jobId: string,
    status: ApplicationStatus,
    notes: string | null,
  ): Promise<NormalizedJobDetail | null> {
    updatingApplication.value = true;
    error.value = null;
    try {
      const updated = await ipc.updateJobApplication(userId, jobId, status, notes);
      if (selectedDetail.value && selectedDetail.value.job_id === jobId) {
        selectedDetail.value = updated;
      }
      if (status === "dismissed" && applicationStatusFilter.value !== "dismissed") {
        await loadJobs(userId);
      } else {
        const row = jobs.value.find((j) => j.job_id === jobId);
        if (row) {
          row.application_status = updated.application_status;
          row.applied_at = updated.applied_at;
          row.is_saved = updated.is_saved;
        }
      }
      return updated;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memperbarui status lamaran.";
      return null;
    } finally {
      updatingApplication.value = false;
    }
  }

  /**
   * Decoupled Stage 1: Scrape multi-page & store cleanly to SQLite with 0 Gemini tokens.
   * If `searchPreferences.ai_enrich_enabled` is ON, automatically chains Stage 2 right after.
   */
  async function runScrape(userId: string): Promise<ScrapeRunReport | null> {
    scraping.value = true;
    error.value = null;
    progress.value = {
      stage: "scraping",
      message: `Menarik lowongan dari ${searchPreferences.value.sources.join(", ")} (${searchPreferences.value.location})...`,
      current: 0,
      total: 0,
    };
    try {
      const report = await ipc.scrapeAndStoreJobs(userId, searchPreferences.value);
      lastScrapeReport.value = report;
      await loadJobs(userId);
      if (searchPreferences.value.ai_enrich_enabled && report.pending_local_count > 0) {
        await runAiMatchingQueue(userId);
      }
      return report;
    } catch (e) {
      error.value =
        e instanceof IpcError ? e.message : "Gagal mengambil lowongan dari worker lokal.";
      return null;
    } finally {
      scraping.value = false;
      if (!runningAiQueue.value) {
        progress.value = null;
      }
    }
  }

  /**
   * Decoupled Stage 2: Resumable batched AI enrichment + RPM limiter on pending `local_only` jobs.
   */
  async function runAiMatchingQueue(
    userId: string,
    customMaxJobs?: number,
  ): Promise<AiMatchRunReport | null> {
    runningAiQueue.value = true;
    error.value = null;
    const maxJobs = customMaxJobs ?? searchPreferences.value.ai_enrich_max_jobs;
    const rpm = searchPreferences.value.ai_rpm_limit;
    progress.value = {
      stage: "ai_batch_extract",
      message: `Menjalankan AI Matching batch (maks ${maxJobs} lowongan, batas ${rpm} RPM)...`,
      current: 0,
      total: maxJobs,
    };
    try {
      const report = await ipc.runAiMatchingQueue(userId, maxJobs, rpm);
      lastAiReport.value = report;
      await loadJobs(userId);
      return report;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menjalankan AI Matching batch.";
      return null;
    } finally {
      runningAiQueue.value = false;
      progress.value = null;
    }
  }

  async function enrichSingleJob(userId: string, jobId: string): Promise<boolean> {
    enrichingJob.value = true;
    error.value = null;
    try {
      const updated = await ipc.enrichJobWithAi(userId, jobId);
      selectedDetail.value = updated;
      await loadJobs(userId);
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memperkaya lowongan dengan AI.";
      return false;
    } finally {
      enrichingJob.value = false;
    }
  }

  async function ingestManual(
    userId: string,
    titleHint: string | null,
    url: string | null,
    rawText: string,
  ): Promise<NormalizedJobDetail | null> {
    ingestingManual.value = true;
    error.value = null;
    try {
      const detail = await ipc.ingestManualJob(userId, titleHint, url, rawText);
      selectedJobId.value = detail.job_id;
      selectedDetail.value = detail;
      await loadJobs(userId);
      return detail;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memproses lowongan manual.";
      return null;
    } finally {
      ingestingManual.value = false;
    }
  }

  async function runGapAnalysis(
    userId: string,
    jobId: string,
    forceRefresh = false,
  ): Promise<boolean> {
    analyzingGap.value = true;
    error.value = null;
    try {
      const analysis = await ipc.generateGapAnalysis(userId, jobId, forceRefresh);
      if (selectedDetail.value && selectedDetail.value.job_id === jobId) {
        selectedDetail.value.ai_gap_analysis = analysis;
      }
      const item = jobs.value.find((j) => j.job_id === jobId);
      if (item) item.has_gap_analysis = true;
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menjalankan analisis gap AI.";
      return false;
    } finally {
      analyzingGap.value = false;
    }
  }

  async function generateApplyKit(
    userId: string,
    jobId: string,
    language: "id" | "en" = "id",
    forceRefresh = false,
  ): Promise<boolean> {
    generatingApplyKit.value = true;
    error.value = null;
    try {
      const kit = await ipc.generateApplyKit(userId, jobId, language, forceRefresh);
      if (selectedDetail.value && selectedDetail.value.job_id === jobId) {
        selectedDetail.value.ai_apply_kit = kit;
      }
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menyusun bahan lamaran AI.";
      return false;
    } finally {
      generatingApplyKit.value = false;
    }
  }

  return {
    jobs,
    selectedJobId,
    selectedDetail,
    searchPreferences,
    roleFilter,
    workModeFilter,
    sourceFilter,
    enrichmentFilter,
    applicationStatusFilter,
    locationFilter,
    searchQuery,
    savedOnly,
    hasMore,
    pendingLocalCount,
    loadingList,
    loadingMore,
    loadingDetail,
    scraping,
    runningAiQueue,
    enrichingJob,
    analyzingGap,
    generatingApplyKit,
    updatingApplication,
    ingestingManual,
    progress,
    lastScrapeReport,
    lastAiReport,
    error,
    loadSearchPreferences,
    saveSearchPreferences,
    loadJobs,
    loadMoreJobs,
    selectJob,
    toggleSave,
    updateApplication,
    runScrape,
    runAiMatchingQueue,
    enrichSingleJob,
    ingestManual,
    runGapAnalysis,
    generateApplyKit,
  };
});
