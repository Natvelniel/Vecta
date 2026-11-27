import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type {
  AiMatchRunReport,
  ApplicationStatus,
  EnrichmentStatus,
  JobFilterInput_Deserialize,
  JobSource,
  PortalScrapeStatus,
  SavedWatchlistItem_Serialize,
  ScrapeRunReport_Serialize,
  SearchPreferences_Serialize,
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

export type JobSortBy = "score" | "newest" | "salary_desc";

const DEFAULT_PREFS: SearchPreferences_Serialize = {
  location: "Indonesia",
  locations: ["Indonesia"],
  sources: ["jobstreet", "linkedin", "kalibrr", "dealls", "karirhub"],
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

  const searchPreferences = ref<SearchPreferences_Serialize>({ ...DEFAULT_PREFS });
  const roleFilter = ref<string | null>(null);

  // Multi-select filter arrays: empty array === "Semua" active (pure color distinction, zero checkmarks)
  const workModes = ref<WorkMode[]>([]);
  const sources = ref<JobSource[]>([]);
  const locationClusters = ref<string[]>([]);
  const opportunityTypes = ref<string[]>([]);
  const applicationStatuses = ref<ApplicationStatus[]>([]);
  const enrichmentFilter = ref<EnrichmentStatus | null>(null);

  const locationFilter = ref<string>("");
  const searchQuery = ref<string>("");
  const savedOnly = ref<boolean>(false);
  const salaryOnly = ref<boolean>(false);
  const sortBy = ref<JobSortBy>("score");
  const hasMore = ref<boolean>(false);

  const watchlists = ref<SavedWatchlistItem_Serialize[]>([]);
  const activeWatchlistId = ref<string | null>(null);

  const loadingList = ref(false);
  const loadingMore = ref(false);
  const loadingDetail = ref(false);
  const enrichingDescription = ref(false);
  const scraping = ref(false);
  const runningAiQueue = ref(false);
  const enrichingJob = ref(false);
  const analyzingGap = ref(false);
  const generatingApplyKit = ref(false);
  const updatingApplication = ref(false);
  const ingestingManual = ref(false);

  const progress = ref<ScrapeProgressEvent | null>(null);
  const lastScrapeReport = ref<ScrapeRunReport_Serialize | null>(null);
  const portalStatuses = ref<Record<string, PortalScrapeStatus>>({});
  const lastAiReport = ref<AiMatchRunReport | null>(null);
  const error = ref<string | null>(null);

  const pendingLocalCount = computed(
    () => jobs.value.filter((j) => j.enrichment_status === "local_only").length,
  );

  // Multi-select rule:
  // - Empty array = "Semua" active
  // - Toggling an item adds/removes it; when all manual items are removed, "Semua" re-activates automatically
  function toggleMultiValue<T>(list: T[], item: T): T[] {
    const idx = list.indexOf(item);
    return idx >= 0 ? list.filter((v) => v !== item) : [...list, item];
  }

  function toggleWorkMode(mode: WorkMode) {
    activeWatchlistId.value = null;
    workModes.value = toggleMultiValue(workModes.value, mode);
  }
  function clearWorkModes() {
    activeWatchlistId.value = null;
    workModes.value = [];
  }

  function toggleSource(src: JobSource) {
    activeWatchlistId.value = null;
    sources.value = toggleMultiValue(sources.value, src);
  }
  function clearSources() {
    activeWatchlistId.value = null;
    sources.value = [];
  }

  function toggleLocationCluster(cluster: string) {
    activeWatchlistId.value = null;
    locationClusters.value = toggleMultiValue(locationClusters.value, cluster);
  }
  function clearLocationClusters() {
    activeWatchlistId.value = null;
    locationClusters.value = [];
  }

  function toggleOpportunityType(typeSlug: string) {
    activeWatchlistId.value = null;
    opportunityTypes.value = toggleMultiValue(opportunityTypes.value, typeSlug);
  }
  function clearOpportunityTypes() {
    activeWatchlistId.value = null;
    opportunityTypes.value = [];
  }

  function toggleApplicationStatus(status: ApplicationStatus) {
    activeWatchlistId.value = null;
    applicationStatuses.value = toggleMultiValue(applicationStatuses.value, status);
  }
  function clearApplicationStatuses() {
    activeWatchlistId.value = null;
    applicationStatuses.value = [];
  }

  function resetAllFilters() {
    activeWatchlistId.value = null;
    roleFilter.value = null;
    workModes.value = [];
    sources.value = [];
    locationClusters.value = [];
    opportunityTypes.value = [];
    applicationStatuses.value = [];
    enrichmentFilter.value = null;
    locationFilter.value = "";
    searchQuery.value = "";
    savedOnly.value = false;
    salaryOnly.value = false;
    sortBy.value = "score";
  }

  function buildFilter(offset: number): JobFilterInput_Deserialize {
    const trimmedSearch = searchQuery.value.trim();
    const trimmedLoc = locationFilter.value.trim();
    const includeDismissed = applicationStatuses.value.includes("dismissed");
    return {
      normalized_role: roleFilter.value,
      work_mode: null,
      source: null,
      saved_only: savedOnly.value ? true : null,
      min_score: null,
      search_query: trimmedSearch ? trimmedSearch : null,
      location_query: trimmedLoc ? trimmedLoc : null,
      enrichment_status: enrichmentFilter.value,
      application_status: null,
      include_dismissed: includeDismissed ? true : null,
      work_modes: workModes.value.length > 0 ? [...workModes.value] : null,
      sources: sources.value.length > 0 ? [...sources.value] : null,
      location_clusters: locationClusters.value.length > 0 ? [...locationClusters.value] : null,
      opportunity_types: opportunityTypes.value.length > 0 ? [...opportunityTypes.value] : null,
      application_statuses:
        applicationStatuses.value.length > 0 ? [...applicationStatuses.value] : null,
      salary_only: salaryOnly.value ? true : null,
      sort_by: sortBy.value,
      limit: PAGE_SIZE,
      offset,
    };
  }

  async function loadSearchPreferences() {
    try {
      const prefs = await ipc.getSearchPreferences();
      searchPreferences.value = {
        ...prefs,
        locations:
          prefs.locations && prefs.locations.length > 0
            ? prefs.locations
            : [prefs.location || "Indonesia"],
      };
    } catch {
      searchPreferences.value = { ...DEFAULT_PREFS };
    }
  }

  async function saveSearchPreferences(prefs: SearchPreferences_Serialize): Promise<boolean> {
    error.value = null;
    try {
      searchPreferences.value = await ipc.saveSearchPreferences(prefs);
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menyimpan preferensi pencarian.";
      return false;
    }
  }

  async function loadWatchlists(userId: string) {
    try {
      watchlists.value = await ipc.listSavedWatchlists(userId);
    } catch {
      watchlists.value = [];
    }
  }

  async function createWatchlist(userId: string, name: string): Promise<boolean> {
    error.value = null;
    try {
      const item = await ipc.saveWatchlist(userId, name, buildFilter(0));
      watchlists.value = [item, ...watchlists.value];
      activeWatchlistId.value = item.id;
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menyimpan watchlist.";
      return false;
    }
  }

  async function removeWatchlist(id: string): Promise<boolean> {
    try {
      await ipc.deleteWatchlist(id);
      watchlists.value = watchlists.value.filter((w) => w.id !== id);
      if (activeWatchlistId.value === id) activeWatchlistId.value = null;
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menghapus watchlist.";
      return false;
    }
  }

  async function applyWatchlist(userId: string, item: SavedWatchlistItem_Serialize) {
    resetAllFilters();
    activeWatchlistId.value = item.id;
    const f = item.filter;
    roleFilter.value = f.normalized_role ?? null;
    workModes.value = f.work_modes ? [...f.work_modes] : [];
    sources.value = f.sources ? [...f.sources] : [];
    locationClusters.value = f.location_clusters ? [...f.location_clusters] : [];
    opportunityTypes.value = f.opportunity_types ? [...f.opportunity_types] : [];
    applicationStatuses.value = f.application_statuses ? [...f.application_statuses] : [];
    enrichmentFilter.value = f.enrichment_status ?? null;
    locationFilter.value = f.location_query ?? "";
    searchQuery.value = f.search_query ?? "";
    savedOnly.value = Boolean(f.saved_only);
    salaryOnly.value = Boolean(f.salary_only);
    sortBy.value = (f.sort_by as JobSortBy) || "score";
    await loadJobs(userId);
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
    const switching = selectedDetail.value?.job_id !== jobId;
    selectedJobId.value = jobId;
    if (switching) {
      selectedDetail.value = null;
    }
    loadingDetail.value = true;
    try {
      const detail = await ipc.getJobDetail(userId, jobId);
      if (selectedJobId.value === jobId) {
        selectedDetail.value = detail;
      }
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memuat detail lowongan.";
    } finally {
      loadingDetail.value = false;
    }
  }

  async function toggleSave(userId: string, jobId: string): Promise<boolean | null> {
    try {
      const saved = await ipc.toggleSaveJob(userId, jobId);
      const item = jobs.value.find((j) => j.job_id === jobId);
      if (item) {
        item.is_saved = saved;
        if (saved && item.application_status === "new") item.application_status = "saved";
        if (!saved && item.application_status === "saved") item.application_status = "new";
      }
      if (selectedDetail.value && selectedDetail.value.job_id === jobId) {
        selectedDetail.value.is_saved = saved;
        if (saved && selectedDetail.value.application_status === "new") {
          selectedDetail.value.application_status = "saved";
        }
        if (!saved && selectedDetail.value.application_status === "saved") {
          selectedDetail.value.application_status = "new";
        }
      }
      return saved;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memperbarui status simpan.";
      return null;
    }
  }

  async function updateApplication(
    userId: string,
    jobId: string,
    status: ApplicationStatus,
    notes: string | null = null,
  ): Promise<NormalizedJobDetail | null> {
    updatingApplication.value = true;
    error.value = null;
    try {
      const updated = await ipc.updateJobApplication(userId, jobId, status, notes);
      if (selectedDetail.value && selectedDetail.value.job_id === jobId) {
        selectedDetail.value = updated;
      }
      if (status === "dismissed" && !applicationStatuses.value.includes("dismissed")) {
        jobs.value = jobs.value.filter((j) => j.job_id !== jobId);
        if (selectedJobId.value === jobId) {
          selectedJobId.value = jobs.value[0]?.job_id ?? null;
          selectedDetail.value = null;
          if (selectedJobId.value) {
            await selectJob(userId, selectedJobId.value);
          }
        }
      } else {
        const item = jobs.value.find((j) => j.job_id === jobId);
        if (item) {
          item.application_status = updated.application_status;
          item.is_saved = updated.is_saved;
          item.applied_at = updated.applied_at;
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

  async function runScrape(
    userId: string,
    overridePrefs?: SearchPreferences_Serialize,
  ): Promise<ScrapeRunReport_Serialize | null> {
    scraping.value = true;
    error.value = null;
    progress.value = { stage: "starting", message: "Menyiapkan penarikan lowongan...", current: 0, total: 1 };
    try {
      const prefs = overridePrefs ?? searchPreferences.value;
      const report = await ipc.scrapeAndStoreJobs(userId, prefs);
      lastScrapeReport.value = report;
      const nextStatuses: Record<string, PortalScrapeStatus> = { ...portalStatuses.value };
      for (const st of report.portal_statuses ?? []) {
        nextStatuses[st.source] = st;
      }
      portalStatuses.value = nextStatuses;
      await loadJobs(userId);
      return report;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Penarikan lowongan gagal.";
      return null;
    } finally {
      scraping.value = false;
      progress.value = null;
    }
  }

  async function enrichJobDescription(userId: string, jobId: string): Promise<boolean> {
    enrichingDescription.value = true;
    try {
      const detail = await ipc.enrichJobDescriptionFromSource(userId, jobId);
      if (selectedDetail.value && selectedDetail.value.job_id === jobId) {
        selectedDetail.value = detail;
      }
      const item = jobs.value.find((j) => j.job_id === jobId);
      if (item) {
        item.final_score = detail.final_score;
        item.matched_skills_count = detail.matched_skills.length;
        item.missing_skills_count = detail.missing_skills.length;
      }
      return true;
    } catch {
      return false;
    } finally {
      enrichingDescription.value = false;
    }
  }

  async function runAiMatchingQueue(
    userId: string,
    maxJobs?: number,
    rpmLimit?: number,
  ): Promise<AiMatchRunReport | null> {
    runningAiQueue.value = true;
    error.value = null;
    progress.value = { stage: "ai_matching", message: "Menyiapkan antrean AI batching...", current: 0, total: 1 };
    try {
      const report = await ipc.runAiMatchingQueue(
        userId,
        maxJobs ?? searchPreferences.value.ai_enrich_max_jobs,
        rpmLimit ?? searchPreferences.value.ai_rpm_limit,
      );
      lastAiReport.value = report;
      await loadJobs(userId);
      return report;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "AI Matching antrean gagal.";
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
      const detail = await ipc.enrichJobWithAi(userId, jobId);
      if (selectedDetail.value && selectedDetail.value.job_id === jobId) {
        selectedDetail.value = detail;
      }
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
    workModes,
    sources,
    locationClusters,
    opportunityTypes,
    applicationStatuses,
    enrichmentFilter,
    locationFilter,
    searchQuery,
    savedOnly,
    salaryOnly,
    sortBy,
    hasMore,
    watchlists,
    activeWatchlistId,
    pendingLocalCount,
    loadingList,
    loadingMore,
    loadingDetail,
    enrichingDescription,
    scraping,
    runningAiQueue,
    enrichingJob,
    analyzingGap,
    generatingApplyKit,
    updatingApplication,
    ingestingManual,
    progress,
    lastScrapeReport,
    portalStatuses,
    lastAiReport,
    error,
    toggleWorkMode,
    clearWorkModes,
    toggleSource,
    clearSources,
    toggleLocationCluster,
    clearLocationClusters,
    toggleOpportunityType,
    clearOpportunityTypes,
    toggleApplicationStatus,
    clearApplicationStatuses,
    resetAllFilters,
    buildFilter,
    loadSearchPreferences,
    saveSearchPreferences,
    loadWatchlists,
    createWatchlist,
    removeWatchlist,
    applyWatchlist,
    loadJobs,
    loadMoreJobs,
    selectJob,
    toggleSave,
    updateApplication,
    runScrape,
    enrichJobDescription,
    runAiMatchingQueue,
    enrichSingleJob,
    ingestManual,
    runGapAnalysis,
    generateApplyKit,
  };
});
