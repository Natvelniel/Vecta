<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import type {
  ApplicationStatus,
  JobSource,
  SavedWatchlistItem_Serialize,
  SearchPreferences_Serialize,
  WorkMode,
} from "@/lib/bindings";
import type { JobSortBy } from "@/stores/jobs";
import { useCompetencyStore } from "@/stores/competency";
import { useJobsStore } from "@/stores/jobs";
import { useProfileStore } from "@/stores/profile";
import { useRoadmapStore } from "@/stores/roadmap";
import { useSettingsStore } from "@/stores/settings";
import { useToast } from "@/composables/useToast";
import {
  formatApplicationStatus,
  formatDomainSlug,
  formatExperienceLevel,
  formatJobSource,
  formatRelativeDate,
  formatWorkMode,
} from "@/lib/format";
import { formatLocationClusterLabel } from "@/lib/locationClusters";
import { ipc, IpcError, listenRerankComplete, listenScrapeProgress, openExternalUrl } from "@/lib/ipc";
import { shareOrDownloadPdf } from "@/lib/pdfShare";
import PageHeader from "@/components/layout/PageHeader.vue";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VIcon from "@/components/ui/VIcon.vue";
import VSkeleton from "@/components/ui/VSkeleton.vue";
import LocationClusterSelect from "@/components/ui/LocationClusterSelect.vue";
import JobListingRow from "@/components/domain/JobListingRow.vue";
import MatchScorePanel from "@/components/domain/MatchScorePanel.vue";
import GapAnalysisCard from "@/components/domain/GapAnalysisCard.vue";
import ApplyCopilotCard from "@/components/domain/ApplyCopilotCard.vue";
import ApplicationPipelineBoard from "@/components/domain/ApplicationPipelineBoard.vue";
import CompanySnapshotModal from "@/components/domain/CompanySnapshotModal.vue";
import InterviewPrepCard from "@/components/domain/InterviewPrepCard.vue";
import ManualJobModal from "@/components/domain/ManualJobModal.vue";
import SearchPreferencesDrawer from "@/components/domain/SearchPreferencesDrawer.vue";
import ScrapeKeywordsPopover from "@/components/domain/ScrapeKeywordsPopover.vue";

const route = useRoute();
const router = useRouter();
const profileStore = useProfileStore();
const jobsStore = useJobsStore();
const competencyStore = useCompetencyStore();
const roadmapStore = useRoadmapStore();
const settingsStore = useSettingsStore();
const toast = useToast();

const isWelcomeFlow = computed(() => route.query.welcome === "1");
const activeTab = ref<"explore" | "pipeline">(
  route.query.tab === "pipeline" ? "pipeline" : "explore",
);
const companyModalOpen = ref(false);
const marketMedianIdr = ref<number | null>(null);

const pipelineCount = computed(
  () =>
    jobsStore.jobs.filter(
      (j) =>
        j.is_saved ||
        ["saved", "applied", "interview", "offer", "rejected"].includes(j.application_status),
    ).length,
);

const manualModalOpen = ref(false);
const preferencesOpen = ref(false);
const savingPrefs = ref(false);
const manualModalRef = ref<InstanceType<typeof ManualJobModal> | null>(null);
const mobileDetailOpen = ref(false);
const filtersExpanded = ref(false);
const mobileMenuOpen = ref(false);
const mobileMenuRef = ref<HTMLElement | null>(null);
const watchlistPopoverOpen = ref(false);
const watchlistName = ref("");
const aiBarDismissed = ref(false);
const noteDraft = ref("");
const jobsListRef = ref<HTMLElement | null>(null);
const ptrDistance = ref(0);
const ptrReady = computed(() => ptrDistance.value >= 56);
let ptrStartX = 0;
let ptrStartY = 0;
let ptrTracking = false;
const attemptedDetailEnrich = new Set<string>();
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;
let unlistenProgress: (() => void) | null = null;
let unlistenRerank: (() => void) | null = null;

function isListAtScrollTop(): boolean {
  const el = jobsListRef.value;
  if (!el) return false;
  if (el.scrollTop > 2) return false;
  const mainEl = el.closest(".v-main") as HTMLElement | null;
  return (mainEl?.scrollTop ?? 0) <= 2;
}

function onListTouchStart(e: TouchEvent) {
  if (jobsStore.scraping || e.touches.length !== 1) return;
  if (!isListAtScrollTop()) return;
  const touch = e.touches[0];
  ptrStartX = touch.clientX;
  ptrStartY = touch.clientY;
  ptrTracking = true;
  ptrDistance.value = 0;
}

function onListTouchMove(e: TouchEvent) {
  if (!ptrTracking || e.touches.length !== 1) return;
  const touch = e.touches[0];
  const dx = touch.clientX - ptrStartX;
  const dy = touch.clientY - ptrStartY;
  if (ptrDistance.value === 0 && (Math.abs(dx) > Math.abs(dy) * 1.15 || dy < -6)) {
    ptrTracking = false;
    ptrDistance.value = 0;
    return;
  }
  if (dy > 0 && (isListAtScrollTop() || ptrDistance.value > 0)) {
    ptrDistance.value = Math.min(82, Math.round(dy * 0.45));
  }
}

function onListTouchEnd() {
  if (!ptrTracking) return;
  ptrTracking = false;
  const shouldRefresh = ptrDistance.value >= 56 && !jobsStore.scraping;
  ptrDistance.value = 0;
  if (shouldRefresh) {
    void handleRefreshScrape();
  }
}

const WORK_MODE_OPTIONS: Array<{ id: WorkMode; label: string }> = [
  { id: "remote", label: "Remote" },
  { id: "hybrid", label: "Hybrid" },
  { id: "onsite", label: "Onsite" },
];

const SOURCE_OPTIONS: Array<{ id: JobSource; label: string }> = [
  { id: "jobstreet", label: "JobStreet" },
  { id: "linkedin", label: "LinkedIn" },
  { id: "kalibrr", label: "Kalibrr" },
  { id: "dealls", label: "Dealls" },
  { id: "karirhub", label: "Karirhub" },
  { id: "remotive", label: "Remotive" },
  { id: "glints", label: "Glints" },
  { id: "indeed", label: "Indeed" },
  { id: "manual", label: "Manual" },
];

const OPPORTUNITY_TYPE_OPTIONS: Array<{ id: string; label: string }> = [
  { id: "full_time", label: "Full-Time" },
  { id: "mt_odp", label: "MT / ODP" },
  { id: "internship", label: "Magang" },
  { id: "contract", label: "Kontrak" },
  { id: "freelance", label: "Freelance" },
];

const APPLICATION_STATUS_OPTIONS: Array<{ id: ApplicationStatus; label: string }> = [
  { id: "new", label: "Baru" },
  { id: "saved", label: "Disimpan" },
  { id: "applied", label: "Dilamar" },
  { id: "interview", label: "Interview" },
  { id: "offer", label: "Offer" },
  { id: "rejected", label: "Ditolak" },
  { id: "dismissed", label: "Disembunyikan" },
];

const SORT_OPTIONS: Array<{ id: JobSortBy; label: string }> = [
  { id: "score", label: "Skor Cocok" },
  { id: "newest", label: "Terbaru" },
  { id: "salary_desc", label: "Gaji Tertinggi" },
];

const ENRICHABLE_DESC_SOURCES = new Set<JobSource>(["jobstreet", "linkedin", "karirhub"]);

const canEnrichDetailFromSource = computed(() => {
  const detail = jobsStore.selectedDetail;
  return (
    !!detail &&
    ENRICHABLE_DESC_SOURCES.has(detail.source) &&
    !!detail.url
  );
});

type DescBlock = { kind: "list"; items: string[] } | { kind: "p"; text: string };
type DescSection = { title: string | null; blocks: DescBlock[] };

const KNOWN_DESC_HEADING = /^(ringkasan|deskripsi|kualifikasi|persyaratan|tanggung jawab|benefit|fasilitas|kriteria|keahlian|skills?|about(?: the (?:role|job|company))?|responsibilities|requirements|qualifications|what you(?:'ll| will) do|who we(?:'re| are) looking for|benefits?|perks?|industri|pendidikan|gaji|pengalaman|lokasi|kategori|detail tambahan)\b/i;

const parsedDescriptionSections = computed<DescSection[]>(() => {
  const raw = jobsStore.selectedDetail?.raw_description ?? "";
  const cleaned = raw.replace(/\r\n?/g, "\n").trim();
  if (!cleaned) return [];

  const sections: DescSection[] = [];
  let current: DescSection = { title: null, blocks: [] };

  const pushParagraph = (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    current.blocks.push({ kind: "p", text: trimmed });
  };
  const pushBullet = (item: string) => {
    const trimmed = item.trim();
    if (!trimmed) return;
    const last = current.blocks[current.blocks.length - 1];
    if (last && last.kind === "list") {
      last.items.push(trimmed);
    } else {
      current.blocks.push({ kind: "list", items: [trimmed] });
    }
  };
  const startSection = (title: string) => {
    if (current.title !== null || current.blocks.length > 0) {
      sections.push(current);
    }
    current = { title: title.replace(/[:\s]+$/, ""), blocks: [] };
  };

  let paraBuf: string[] = [];
  const flushPara = () => {
    if (paraBuf.length > 0) {
      pushParagraph(paraBuf.join(" "));
      paraBuf = [];
    }
  };

  for (const rawLine of cleaned.split("\n")) {
    const line = rawLine.trim();
    if (!line) {
      flushPara();
      continue;
    }

    const bulletMatch = line.match(/^(?:[-*•▪●]|(?:\d{1,2}[.)]))\s+(.+)$/);
    if (bulletMatch) {
      flushPara();
      pushBullet(bulletMatch[1]);
      continue;
    }

    const headingInline = line.match(/^([A-Za-zÀ-ÿ0-9 /&(),'-]{2,48}):\s*(.*)$/);
    if (headingInline) {
      const label = headingInline[1].trim();
      const rest = headingInline[2].trim();
      if (!rest || KNOWN_DESC_HEADING.test(label) || label.length <= 26) {
        flushPara();
        startSection(label);
        if (rest) {
          if (rest.includes(" | ")) {
            rest.split(" | ").forEach((item) => pushBullet(item));
          } else {
            paraBuf.push(rest);
          }
        }
        continue;
      }
    }

    if (KNOWN_DESC_HEADING.test(line) && line.length <= 44 && !/[.!?]$/.test(line)) {
      flushPara();
      startSection(line);
      continue;
    }

    paraBuf.push(line);
  }
  flushPara();
  if (current.title !== null || current.blocks.length > 0) {
    sections.push(current);
  }
  return sections;
});

async function handleEnrichDescriptionFromSource() {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  const ok = await jobsStore.enrichJobDescription(
    profileStore.userId,
    jobsStore.selectedDetail.job_id,
  );
  if (ok) {
    toast.show("Deskripsi lengkap berhasil dimuat.", { tone: "success" });
  } else {
    toast.show(
      friendlyActionMessage(
        jobsStore.error,
        "Deskripsi lengkap belum tersedia dari portal sumber.",
      ),
      { tone: "info" },
    );
  }
}

watch(
  () => jobsStore.selectedDetail,
  (detail) => {
    noteDraft.value = detail?.user_notes ?? "";
    if (
      detail &&
      profileStore.userId &&
      detail.raw_description.trim().length < 400 &&
      ENRICHABLE_DESC_SOURCES.has(detail.source) &&
      !!detail.url &&
      !attemptedDetailEnrich.has(detail.job_id) &&
      !jobsStore.enrichingDescription
    ) {
      attemptedDetailEnrich.add(detail.job_id);
      void jobsStore.enrichJobDescription(profileStore.userId, detail.job_id);
    }
    if (detail && (detail.salary_min_idr || detail.salary_max_idr)) {
      ipc
        .getSalaryBenchmark(detail.normalized_role, null)
        .then((res) => {
          marketMedianIdr.value = !res.is_small_sample ? (res.median_idr ?? null) : null;
        })
        .catch(() => {
          marketMedianIdr.value = null;
        });
    } else {
      marketMedianIdr.value = null;
    }
  },
  { immediate: true },
);

const salaryComparisonLabel = computed(() => {
  const detail = jobsStore.selectedDetail;
  const median = marketMedianIdr.value;
  if (!detail || !median) return null;
  const min = detail.salary_min_idr;
  const max = detail.salary_max_idr;
  const mid = min && max ? (min + max) / 2 : (min ?? max);
  if (!mid) return null;
  if (mid >= median * 1.1) return "Di atas median pasar";
  if (mid <= median * 0.9) return "Di bawah median pasar";
  return "Sesuai median pasar";
});

async function handlePipelineUpdate(payload: {
  jobId: string;
  status: ApplicationStatus;
  notes: string | null;
}) {
  if (!profileStore.userId) return;
  await jobsStore.updateApplication(profileStore.userId, payload.jobId, payload.status, payload.notes);
}

async function handleOpenPipelineDetail(jobId: string) {
  if (!profileStore.userId) return;
  await jobsStore.selectJob(profileStore.userId, jobId);
  activeTab.value = "explore";
  mobileDetailOpen.value = true;
}

function handleMobileMenuOutsideClick(e: MouseEvent) {
  if (!mobileMenuOpen.value) return;
  const target = e.target as Node | null;
  if (mobileMenuRef.value && target && !mobileMenuRef.value.contains(target)) {
    mobileMenuOpen.value = false;
  }
}

function handleMobileMenuKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && mobileMenuOpen.value) {
    mobileMenuOpen.value = false;
  }
}

onBeforeUnmount(() => {
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  if (unlistenProgress) unlistenProgress();
  if (unlistenRerank) unlistenRerank();
  window.removeEventListener("click", handleMobileMenuOutsideClick);
  window.removeEventListener("keydown", handleMobileMenuKeydown);
});

function handleSearchInput(value: string) {
  jobsStore.searchQuery = value;
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  searchDebounceTimer = setTimeout(() => {
    void refreshFilters();
  }, 220);
}

function handleLocationClustersChange(nextClusters: string[]) {
  jobsStore.activeWatchlistId = null;
  jobsStore.locationClusters = nextClusters;
  void refreshFilters();
}

async function handleLoadMore() {
  if (!profileStore.userId) return;
  await jobsStore.loadMoreJobs(profileStore.userId);
}

onMounted(async () => {
  window.addEventListener("click", handleMobileMenuOutsideClick);
  window.addEventListener("keydown", handleMobileMenuKeydown);
  unlistenProgress = await listenScrapeProgress((evt) => {
    jobsStore.progress = evt;
  });
  unlistenRerank = await listenRerankComplete((evt) => {
    if (evt.reranked_count > 0 && profileStore.userId === evt.user_id) {
      void jobsStore.loadJobs(evt.user_id);
    }
  });
  await jobsStore.loadSearchPreferences();
  if (!profileStore.activeProfile) {
    await profileStore.loadActiveProfile();
  }
  const uid = profileStore.userId;
  if (uid) {
    await Promise.all([
      jobsStore.loadJobs(uid),
      jobsStore.loadWatchlists(uid),
      competencyStore.loadRolesAndMap(uid),
    ]);
  }
});

const activeFilterCount = computed(() => {
  let count = 0;
  if (jobsStore.roleFilter) count += 1;
  count += jobsStore.workModes.length;
  count += jobsStore.locationClusters.length;
  count += jobsStore.sources.length;
  count += jobsStore.opportunityTypes.length;
  count += jobsStore.applicationStatuses.length;
  if (jobsStore.savedOnly) count += 1;
  if (jobsStore.salaryOnly) count += 1;
  if (jobsStore.sortBy !== "score") count += 1;
  return count;
});

const activeFilterChips = computed(() => {
  const chips: Array<{ key: string; kind: string; value?: string; label: string }> = [];
  if (jobsStore.roleFilter) {
    chips.push({ key: "role", kind: "role", label: jobsStore.roleFilter });
  }
  for (const m of jobsStore.workModes) {
    chips.push({ key: `mode:${m}`, kind: "mode", value: m, label: formatWorkMode(m) });
  }
  for (const cl of jobsStore.locationClusters) {
    chips.push({ key: `loc:${cl}`, kind: "loc", value: cl, label: formatLocationClusterLabel(cl) });
  }
  for (const src of jobsStore.sources) {
    chips.push({ key: `src:${src}`, kind: "src", value: src, label: formatJobSource(src) });
  }
  for (const ot of jobsStore.opportunityTypes) {
    const lbl = OPPORTUNITY_TYPE_OPTIONS.find((o) => o.id === ot)?.label ?? ot;
    chips.push({ key: `opp:${ot}`, kind: "opp", value: ot, label: lbl });
  }
  for (const st of jobsStore.applicationStatuses) {
    chips.push({
      key: `status:${st}`,
      kind: "status",
      value: st,
      label: formatApplicationStatus(st),
    });
  }
  if (jobsStore.savedOnly) chips.push({ key: "saved", kind: "saved", label: "Hanya Disimpan" });
  if (jobsStore.salaryOnly) chips.push({ key: "salary", kind: "salary", label: "Hanya Bergaji" });
  return chips;
});

async function clearFilterChip(chip: { kind: string; value?: string }) {
  if (chip.kind === "role") jobsStore.roleFilter = null;
  if (chip.kind === "mode" && chip.value) jobsStore.toggleWorkMode(chip.value as WorkMode);
  if (chip.kind === "loc" && chip.value) jobsStore.toggleLocationCluster(chip.value);
  if (chip.kind === "src" && chip.value) jobsStore.toggleSource(chip.value as JobSource);
  if (chip.kind === "opp" && chip.value) jobsStore.toggleOpportunityType(chip.value);
  if (chip.kind === "status" && chip.value) {
    jobsStore.toggleApplicationStatus(chip.value as ApplicationStatus);
  }
  if (chip.kind === "saved") jobsStore.savedOnly = false;
  if (chip.kind === "salary") jobsStore.salaryOnly = false;
  await refreshFilters();
}

async function handleSaveWatchlist() {
  if (!profileStore.userId || !watchlistName.value.trim()) return;
  const ok = await jobsStore.createWatchlist(profileStore.userId, watchlistName.value.trim());
  if (ok) {
    toast.show(`Watchlist "${watchlistName.value.trim()}" tersimpan.`, { tone: "success" });
    watchlistName.value = "";
    watchlistPopoverOpen.value = false;
  }
}

async function handleApplyWatchlist(item: SavedWatchlistItem_Serialize) {
  if (!profileStore.userId) return;
  await jobsStore.applyWatchlist(profileStore.userId, item);
}

async function handleRemoveWatchlist(id: string) {
  await jobsStore.removeWatchlist(id);
}

const showContextualAiBar = computed(
  () =>
    settingsStore.apiKeyStatus.has_key &&
    !aiBarDismissed.value &&
    !jobsStore.scraping &&
    !jobsStore.runningAiQueue &&
    jobsStore.pendingLocalCount > 0,
);

const hasActiveFilters = computed(
  () =>
    activeFilterCount.value > 0 ||
    Boolean(jobsStore.searchQuery.trim()),
);

const progressPercent = computed(() => {
  const p = jobsStore.progress;
  if (!p || p.total <= 0) return 0;
  return Math.min(100, Math.round((p.current / p.total) * 100));
});

function friendlyActionMessage(raw: string | null | undefined, fallback: string): string {
  if (!raw) return fallback;
  if (/https?:\/\/|error sending request|status \d{3}|rustls|dns/i.test(raw)) {
    return fallback;
  }
  return raw;
}

const roleOptions = computed(() => {
  const fromRoles = competencyStore.availableRoles.map((r) => r.normalized_role);
  const fromJobs = jobsStore.jobs.map((j) => j.normalized_role);
  return Array.from(new Set([...fromRoles, ...fromJobs])).sort();
});

async function refreshFilters() {
  if (!profileStore.userId) return;
  await jobsStore.loadJobs(profileStore.userId);
}

async function resetFilters() {
  jobsStore.resetAllFilters();
  await refreshFilters();
}

async function handleSelectJob(jobId: string) {
  if (!profileStore.userId) return;
  await jobsStore.selectJob(profileStore.userId, jobId);
  mobileDetailOpen.value = true;
}

async function handleToggleSavedFilter() {
  jobsStore.savedOnly = !jobsStore.savedOnly;
  await refreshFilters();
}

async function handleToggleSalaryFilter() {
  jobsStore.salaryOnly = !jobsStore.salaryOnly;
  await refreshFilters();
}

async function handleToggleBookmark() {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  await jobsStore.toggleSave(profileStore.userId, jobsStore.selectedDetail.job_id);
}

async function handleSavePreferences(prefs: SearchPreferences_Serialize, runImmediately: boolean) {
  savingPrefs.value = true;
  const ok = await jobsStore.saveSearchPreferences(prefs);
  savingPrefs.value = false;
  if (!ok) {
    toast.show(
      friendlyActionMessage(jobsStore.error, "Preferensi belum dapat disimpan. Coba ulangi."),
      { tone: "info" },
    );
    return;
  }
  preferencesOpen.value = false;
  if (runImmediately) {
    await handleRefreshScrape();
  } else {
    toast.show(
      `Preferensi pencarian disimpan (${prefs.location}, ${prefs.sources.length} portal).`,
      { tone: "success" },
    );
  }
}

async function handleRefreshScrape() {
  if (!profileStore.userId) {
    toast.show("Buat Profil Cerita terlebih dahulu sebelum mengambil lowongan.", {
      tone: "warning",
      actionLabel: "Buat Profil",
      onAction: () => router.push("/onboarding"),
    });
    return;
  }
  const report = await jobsStore.runScrape(profileStore.userId);
  if (report) {
    if (report.warnings.length > 0) {
      console.debug("[vecta::scraper] partial warnings suppressed from UI:", report.warnings);
    }
    const aiNote = jobsStore.searchPreferences.ai_enrich_enabled
      ? " + AI Matching selesai."
      : " (0 token Gemini).";
    toast.show(
      `Selesai: ${report.newly_ingested} lowongan baru tersimpan, ${report.skipped_duplicates} duplikat dilewati${aiNote}`,
      { tone: "success" },
    );
  } else {
    if (jobsStore.error) {
      console.debug("[vecta::scraper] network error suppressed from UI:", jobsStore.error);
    }
    toast.show("Koneksi portal sedang lambat. Coba lagi sebentar atau tempel lowongan manual.", {
      tone: "info",
      actionLabel: "Tempel Manual",
      onAction: () => {
        manualModalOpen.value = true;
      },
    });
  }
}

async function handleRunAiQueue() {
  if (!profileStore.userId) return;
  const report = await jobsStore.runAiMatchingQueue(profileStore.userId);
  if (report) {
    toast.show(
      `AI Matching selesai: ${report.enriched_jobs} lowongan diperkaya (${report.api_calls_used} req Gemini), ${report.remaining_local_jobs} tersisa lokal.`,
      { tone: report.failed_jobs > 0 ? "info" : "success" },
    );
  } else if (jobsStore.error) {
    toast.show(
      friendlyActionMessage(jobsStore.error, "Pemrosesan AI tertunda. Coba lagi sebentar."),
      { tone: "info" },
    );
  }
}

async function handleEnrichSingleJob() {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  const ok = await jobsStore.enrichSingleJob(profileStore.userId, jobsStore.selectedDetail.job_id);
  if (ok) {
    toast.show("Lowongan diperkaya dengan AI dan skor kecocokan diperbarui.", {
      tone: "success",
    });
  } else if (jobsStore.error) {
    toast.show(
      friendlyActionMessage(jobsStore.error, "Pengayaan AI sedang tidak tersedia."),
      { tone: "info" },
    );
  }
}

async function handleStatusChange(nextStatus: ApplicationStatus) {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  const updated = await jobsStore.updateApplication(
    profileStore.userId,
    jobsStore.selectedDetail.job_id,
    nextStatus,
    noteDraft.value.trim() || null,
  );
  if (updated) {
    toast.show(
      nextStatus === "dismissed"
        ? "Lowongan disembunyikan dan akan dilewati oleh antrean AI."
        : `Status lamaran diperbarui menjadi ${formatApplicationStatus(nextStatus)}.`,
      { tone: "success" },
    );
  }
}

async function handleSaveNote() {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  const updated = await jobsStore.updateApplication(
    profileStore.userId,
    jobsStore.selectedDetail.job_id,
    jobsStore.selectedDetail.application_status,
    noteDraft.value.trim() || null,
  );
  if (updated) {
    toast.show("Catatan lamaran disimpan secara lokal.", { tone: "success" });
  }
}

async function handleGenerateApplyKit(language: "id" | "en", forceRefresh: boolean) {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  const ok = await jobsStore.generateApplyKit(
    profileStore.userId,
    jobsStore.selectedDetail.job_id,
    language,
    forceRefresh,
  );
  if (ok) {
    toast.show("Bahan lamaran AI siap disalin dan tersimpan di cache SQLite lokal.", {
      tone: "success",
    });
  } else if (jobsStore.error) {
    toast.show(friendlyActionMessage(jobsStore.error, "Bahan lamaran belum dapat dibuat. Coba lagi."), { tone: "info" });
  }
}

async function handleExportTailoredPdf() {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  const res = await profileStore.exportAtsPdf(
    profileStore.userId,
    jobsStore.selectedDetail.job_id,
    "id",
  );
  if (res) {
    if (typeof navigator !== "undefined" && /android/i.test(navigator.userAgent)) {
      const outcome = await shareOrDownloadPdf(res.file_name, res.pdf_base64);
      toast.show(
        outcome === "shared"
          ? `CV ATS PDF siap dibagikan (Skor ATS ${Math.round(res.ats_keyword_score ?? 100)}%).`
          : `CV ATS PDF mulai diunduh (Skor ATS ${Math.round(res.ats_keyword_score ?? 100)}%).`,
        { tone: "success" },
      );
    } else {
      toast.show(
        `CV ATS PDF ter-tailor disimpan di ${res.file_path} (Skor ATS ${Math.round(res.ats_keyword_score ?? 100)}%).`,
        {
          tone: "success",
          actionLabel: "Bagikan / Unduh PDF",
          onAction: () => shareOrDownloadPdf(res.file_name, res.pdf_base64),
        },
      );
    }
  } else if (profileStore.error) {
    toast.show(friendlyActionMessage(profileStore.error, "Ekspor PDF belum berhasil. Coba lagi."), { tone: "info" });
  }
}

const isFollowUpDue = computed(() => {
  const detail = jobsStore.selectedDetail;
  if (!detail || detail.application_status !== "applied" || !detail.applied_at) return false;
  const appliedMs = Date.parse(detail.applied_at);
  if (Number.isNaN(appliedMs)) return false;
  return Date.now() - appliedMs >= 7 * 24 * 3600_000;
});

async function handleManualSubmit(payload: {
  titleHint: string | null;
  url: string | null;
  rawText: string;
}) {
  if (!profileStore.userId) {
    toast.show("Buat Profil Cerita terlebih dahulu sebelum menyimpan lowongan manual.", {
      tone: "warning",
    });
    return;
  }
  const detail = await jobsStore.ingestManual(
    profileStore.userId,
    payload.titleHint,
    payload.url,
    payload.rawText,
  );
  if (detail) {
    manualModalRef.value?.resetFields();
    manualModalOpen.value = false;
    mobileDetailOpen.value = true;
    toast.show(`Lowongan ${detail.title} berhasil dianalisis dan di-ranking.`, {
      tone: "success",
    });
  }
}

async function handleQuickAddSkill(skillId: number, skillName: string) {
  if (!profileStore.userId) return;
  try {
    await ipc.quickAddUserSkill(profileStore.userId, skillId);
    await Promise.all([
      profileStore.loadActiveProfile(),
      jobsStore.loadJobs(profileStore.userId),
      competencyStore.loadRolesAndMap(profileStore.userId),
    ]);
    toast.show(
      `${skillName} ditambahkan ke profil. Skor kecocokan dihitung ulang lokal (0 token Gemini).`,
      { tone: "success" },
    );
  } catch (e) {
    toast.show(friendlyActionMessage(e instanceof IpcError ? e.message : null, "Gagal menambahkan skill."), { tone: "info" });
  }
}

async function handleGapAnalysis(forceRefresh: boolean) {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  const ok = await jobsStore.runGapAnalysis(
    profileStore.userId,
    jobsStore.selectedDetail.job_id,
    forceRefresh,
  );
  if (ok) {
    toast.show("Analisis gap AI selesai dan tersimpan di cache SQLite lokal.", {
      tone: "success",
    });
  }
}

async function handleQuickSave(jobId: string) {
  if (!profileStore.userId) return;
  const saved = await jobsStore.toggleSave(profileStore.userId, jobId);
  if (saved !== null) {
    toast.show(saved ? "Lowongan disimpan." : "Lowongan dihapus dari simpanan.", {
      tone: "info",
    });
  }
}

async function handleQuickDismiss(jobId: string) {
  if (!profileStore.userId) return;
  const target = jobsStore.jobs.find((j) => j.job_id === jobId);
  const prevStatus: ApplicationStatus = target?.application_status ?? "new";
  const updated = await jobsStore.updateApplication(profileStore.userId, jobId, "dismissed", null);
  if (updated) {
    toast.show("Lowongan disembunyikan.", {
      tone: "info",
      actionLabel: "Urungkan",
      onAction: async () => {
        if (profileStore.userId) {
          await jobsStore.updateApplication(profileStore.userId, jobId, prevStatus, null);
        }
      },
    });
  }
}

async function handleCreateRoadmapFromJob() {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  const ok = await roadmapStore.generateRoadmap(
    profileStore.userId,
    jobsStore.selectedDetail.normalized_role,
    jobsStore.selectedDetail.job_id,
    8,
  );
  if (ok) {
    toast.show("Roadmap belajar selesai disusun dari celah skill lowongan ini.", {
      tone: "success",
    });
    router.push("/roadmap");
  } else if (roadmapStore.error) {
    toast.show(friendlyActionMessage(roadmapStore.error, "Roadmap belum dapat disusun. Coba lagi."), { tone: "info" });
  }
}
</script>

<template>
  <div class="v-jobs-shell">
    <PageHeader
      class="v-jobs-desktop-header"
      title="Lowongan"
      compact-mobile
      :subtitle="`${jobsStore.jobs.length} lowongan · ${jobsStore.searchPreferences.location} · ${jobsStore.searchPreferences.sources.length} portal`"
    >
      <template #actions>
        <VButton variant="secondary" size="compact" @click="preferencesOpen = true">
          <VIcon name="sliders" :size="14" />
          <span class="v-hide-xs">Preferensi</span>
        </VButton>
        <VButton variant="secondary" size="compact" @click="manualModalOpen = true">
          <VIcon name="plus" :size="14" />
          <span class="v-hide-xs">Tempel</span>
        </VButton>
        <VButton variant="primary" size="compact" :loading="jobsStore.scraping" @click="handleRefreshScrape">
          <VIcon v-if="!jobsStore.scraping" name="refresh" :size="14" />
          <span>{{ jobsStore.scraping ? "Mengambil..." : "Cari" }}</span>
        </VButton>
      </template>
    </PageHeader>

    <div v-if="jobsStore.progress" class="v-progress-bar" role="status" aria-live="polite">
      <div class="v-progress-bar__row">
        <span class="t-secondary" style="color: var(--text-primary)">
          {{ jobsStore.progress.message }}
        </span>
        <span v-if="jobsStore.progress.total > 0" class="t-meta t-num">
          {{ jobsStore.progress.current }} / {{ jobsStore.progress.total }} ({{ progressPercent }}%)
        </span>
      </div>
      <div class="v-progress-track">
        <div
          class="v-progress-fill"
          :class="{ 'v-progress-fill--indeterminate': jobsStore.progress.total <= 0 }"
          :style="jobsStore.progress.total > 0 ? { width: `${progressPercent}%` } : undefined"
        />
      </div>
    </div>

    <div
      v-if="showContextualAiBar"
      class="v-ai-queue-bar"
      role="status"
    >
      <div class="v-ai-queue-bar__text">
        <VBadge tone="ai">
          <VIcon name="aiNode" :size="12" />
          <span>AI</span>
        </VBadge>
        <span class="t-secondary v-ai-queue-bar__copy">
          <strong class="t-num" style="color: var(--text-primary)">
            {{ jobsStore.pendingLocalCount }}
          </strong>
          siap AI
        </span>
      </div>
      <div class="v-ai-queue-bar__actions">
        <VButton
          variant="ai"
          size="compact"
          :loading="jobsStore.runningAiQueue"
          @click="handleRunAiQueue"
        >
          <VIcon v-if="!jobsStore.runningAiQueue" name="aiNode" :size="13" />
          <span>Jalankan ({{ jobsStore.searchPreferences.ai_enrich_max_jobs }})</span>
        </VButton>
        <button
          type="button"
          class="v-icon-dismiss"
          aria-label="Sembunyikan banner antrean AI"
          @click="aiBarDismissed = true"
        >
          <VIcon name="close" :size="14" />
        </button>
      </div>
    </div>

    <!-- Segmented Switch: Eksplorasi vs Lamaran Saya (+ Compact Mobile Primary Actions) -->
    <div class="v-jobs-mode-bar">
      <div class="v-jobs-tabs" role="tablist" aria-label="Mode tampilan lowongan">
        <button
          type="button"
          role="tab"
          class="v-jobs-tab"
          :class="{ 'v-jobs-tab--active': activeTab === 'explore' }"
          :aria-selected="activeTab === 'explore'"
          @click="activeTab = 'explore'"
        >
          <span class="v-jobs-tab-label">Eksplorasi</span>
          <span class="v-jobs-tab-count v-jobs-tab-count--mobile t-num">{{ jobsStore.jobs.length }}</span>
        </button>
        <button
          type="button"
          role="tab"
          class="v-jobs-tab"
          :class="{ 'v-jobs-tab--active': activeTab === 'pipeline' }"
          :aria-selected="activeTab === 'pipeline'"
          @click="activeTab = 'pipeline'"
        >
          <span class="v-jobs-tab-label">Lamaran</span>
          <span class="v-jobs-tab-count t-num">{{ pipelineCount }}</span>
        </button>
      </div>

      <div class="v-mobile-mode-actions">
        <VButton
          variant="primary"
          size="compact"
          class="v-mobile-scrape-btn"
          :loading="jobsStore.scraping"
          @click="handleRefreshScrape"
        >
          <VIcon v-if="!jobsStore.scraping" name="refresh" :size="14" />
          <span>{{ jobsStore.scraping ? "Muat" : "Cari" }}</span>
        </VButton>
        <div ref="mobileMenuRef" class="v-mobile-menu-wrap">
          <button
            type="button"
            class="v-overflow-btn"
            :aria-expanded="mobileMenuOpen"
            aria-label="Menu aksi lowongan"
            @click="mobileMenuOpen = !mobileMenuOpen"
          >
            <VIcon name="moreVertical" :size="16" />
          </button>
          <Transition name="v-dialog-pop">
            <div v-if="mobileMenuOpen" class="v-mobile-menu-pop" role="menu">
              <button
                type="button"
                class="v-mobile-menu-item"
                role="menuitem"
                @click="mobileMenuOpen = false; preferencesOpen = true"
              >
                <VIcon name="sliders" :size="14" />
                <span>Preferensi Pencarian</span>
              </button>
              <button
                type="button"
                class="v-mobile-menu-item"
                role="menuitem"
                @click="mobileMenuOpen = false; manualModalOpen = true"
              >
                <VIcon name="plus" :size="14" />
                <span>Tempel Lowongan Manual</span>
              </button>
              <button
                type="button"
                class="v-mobile-menu-item"
                role="menuitem"
                @click="mobileMenuOpen = false; watchlistPopoverOpen = true"
              >
                <VIcon name="bookmark" :size="14" />
                <span>Simpan Filter Saat Ini</span>
              </button>
            </div>
          </Transition>
        </div>
      </div>
    </div>

    <ApplicationPipelineBoard
      v-if="activeTab === 'pipeline'"
      :jobs="jobsStore.jobs"
      :selected-job-id="jobsStore.selectedJobId"
      :user-name="profileStore.activeProfile?.profile_name ?? 'Kandidat'"
      @select="(jid) => { if (profileStore.userId) jobsStore.selectJob(profileStore.userId, jid); }"
      @update-status="handlePipelineUpdate"
      @open-detail="handleOpenPipelineDetail"
    />

    <div v-else class="v-jobs-split" :data-mobile-detail="String(mobileDetailOpen)">
      <!-- Left 400px Scannable Job Rail -->
      <section class="v-jobs-rail" aria-label="Daftar lowongan terurut">
        <div class="v-jobs-filters">
          <div class="v-search-bar-row">
            <div class="v-search-field">
              <VIcon name="search" :size="15" class="v-search-icon" />
              <label for="jobs-search-input" class="sr-only">Cari judul atau perusahaan</label>
              <input
                id="jobs-search-input"
                type="search"
                :value="jobsStore.searchQuery"
                class="v-search-input"
                placeholder="Cari judul, perusahaan, skill..."
                @input="handleSearchInput(($event.target as HTMLInputElement).value)"
              />
              <button
                v-if="jobsStore.searchQuery.trim()"
                type="button"
                class="v-search-clear"
                aria-label="Hapus kata pencarian"
                @click="handleSearchInput('')"
              >
                <VIcon name="close" :size="13" />
              </button>
            </div>

            <button
              type="button"
              class="v-filter-toggle"
              :class="{ 'v-filter-toggle--active': filtersExpanded || activeFilterCount > 0 }"
              :aria-expanded="filtersExpanded"
              @click="filtersExpanded = !filtersExpanded"
            >
              <VIcon name="sliders" :size="14" />
              <span>Filter</span>
              <span v-if="activeFilterCount > 0" class="v-filter-badge t-num">
                {{ activeFilterCount }}
              </span>
            </button>
          </div>

          <!-- Opportunity Watchlists Strip (hidden on mobile when empty to declutter top feed) -->
          <div
            class="v-watchlist-strip"
            :class="{ 'v-watchlist-strip--empty': jobsStore.watchlists.length === 0 }"
            aria-label="Watchlist pencarian tersimpan"
          >
            <div
              v-for="wl in jobsStore.watchlists"
              :key="wl.id"
              class="v-wl-chip"
              :class="{ 'v-wl-chip--active': jobsStore.activeWatchlistId === wl.id }"
            >
              <button type="button" class="v-wl-btn" @click="handleApplyWatchlist(wl)">
                <span>{{ wl.name }}</span>
              </button>
              <button
                type="button"
                class="v-wl-del"
                :aria-label="`Hapus watchlist ${wl.name}`"
                @click.stop="handleRemoveWatchlist(wl.id)"
              >
                <VIcon name="close" :size="10" />
              </button>
            </div>

            <button
              type="button"
              class="v-wl-add v-hide-xs"
              @click="watchlistPopoverOpen = !watchlistPopoverOpen"
            >
              <VIcon name="plus" :size="12" />
              <span>Simpan Filter</span>
            </button>
          </div>

          <form
            v-if="watchlistPopoverOpen"
            class="v-wl-form"
            @submit.prevent="handleSaveWatchlist"
          >
            <input
              v-model="watchlistName"
              type="text"
              class="v-wl-input"
              placeholder="Nama watchlist (mis. Remote Jakarta Bergaji)..."
            />
            <VButton type="submit" variant="primary" size="compact">Simpan</VButton>
          </form>

          <div v-if="activeFilterChips.length > 0" class="v-active-chips">
            <button
              v-for="chip in activeFilterChips"
              :key="chip.key"
              type="button"
              class="v-active-chip"
              @click="clearFilterChip(chip)"
            >
              <span>{{ chip.label }}</span>
              <VIcon name="close" :size="11" />
            </button>
            <button type="button" class="v-chip-reset" @click="resetFilters">Reset</button>
          </div>

          <!-- Mobile Dimmed Backdrop for Bottom Sheet Filter -->
          <Transition name="v-backdrop-fade">
            <div
              v-if="filtersExpanded"
              class="v-filter-backdrop"
              aria-hidden="true"
              @click="filtersExpanded = false"
            />
          </Transition>

          <!-- Rich Multi-Select Filter Drawer (Solid Architectural Surface + Searchable Location Dropdown) -->
          <Transition name="v-sheet-slide">
            <div v-if="filtersExpanded" class="v-filter-drawer" role="dialog" aria-label="Filter lowongan">
              <div class="v-sheet-grab" aria-hidden="true" />
              <div class="v-mf-group">
                <span class="v-mf-label">MODE KERJA</span>
                <div class="v-mf-pills">
                  <button
                    type="button"
                    class="v-mf-pill"
                    :class="{ 'v-mf-pill--all': jobsStore.workModes.length === 0 }"
                    @click="jobsStore.clearWorkModes(); refreshFilters()"
                  >
                    Semua
                  </button>
                  <button
                    v-for="opt in WORK_MODE_OPTIONS"
                    :key="opt.id"
                    type="button"
                    class="v-mf-pill"
                    :class="{ 'v-mf-pill--manual': jobsStore.workModes.includes(opt.id) }"
                    @click="jobsStore.toggleWorkMode(opt.id); refreshFilters()"
                  >
                    {{ opt.label }}
                  </button>
                </div>
              </div>

              <div class="v-mf-group">
                <span class="v-mf-label">LOKASI KLASTER (38 PROVINSI, REMOTE &amp; GLOBAL)</span>
                <LocationClusterSelect
                  :model-value="jobsStore.locationClusters"
                  @update:model-value="handleLocationClustersChange"
                />
              </div>

            <div class="v-mf-group">
              <span class="v-mf-label">PORTAL SUMBER</span>
              <div class="v-mf-pills">
                <button
                  type="button"
                  class="v-mf-pill"
                  :class="{ 'v-mf-pill--all': jobsStore.sources.length === 0 }"
                  @click="jobsStore.clearSources(); refreshFilters()"
                >
                  Semua
                </button>
                <button
                  v-for="src in SOURCE_OPTIONS"
                  :key="src.id"
                  type="button"
                  class="v-mf-pill"
                  :class="{ 'v-mf-pill--manual': jobsStore.sources.includes(src.id) }"
                  @click="jobsStore.toggleSource(src.id); refreshFilters()"
                >
                  {{ src.label }}
                </button>
              </div>
            </div>

            <div class="v-mf-group">
              <span class="v-mf-label">TIPE PELUANG</span>
              <div class="v-mf-pills">
                <button
                  type="button"
                  class="v-mf-pill"
                  :class="{ 'v-mf-pill--all': jobsStore.opportunityTypes.length === 0 }"
                  @click="jobsStore.clearOpportunityTypes(); refreshFilters()"
                >
                  Semua
                </button>
                <button
                  v-for="ot in OPPORTUNITY_TYPE_OPTIONS"
                  :key="ot.id"
                  type="button"
                  class="v-mf-pill"
                  :class="{ 'v-mf-pill--manual': jobsStore.opportunityTypes.includes(ot.id) }"
                  @click="jobsStore.toggleOpportunityType(ot.id); refreshFilters()"
                >
                  {{ ot.label }}
                </button>
              </div>
            </div>

            <div class="v-mf-group">
              <span class="v-mf-label">STATUS LAMARAN</span>
              <div class="v-mf-pills">
                <button
                  type="button"
                  class="v-mf-pill"
                  :class="{ 'v-mf-pill--all': jobsStore.applicationStatuses.length === 0 }"
                  @click="jobsStore.clearApplicationStatuses(); refreshFilters()"
                >
                  Semua
                </button>
                <button
                  v-for="st in APPLICATION_STATUS_OPTIONS"
                  :key="st.id"
                  type="button"
                  class="v-mf-pill"
                  :class="{ 'v-mf-pill--manual': jobsStore.applicationStatuses.includes(st.id) }"
                  @click="jobsStore.toggleApplicationStatus(st.id); refreshFilters()"
                >
                  {{ st.label }}
                </button>
              </div>
            </div>

            <div class="v-mf-group">
              <span class="v-mf-label">URUTKAN &amp; FILTER CEPAT</span>
              <div class="v-mf-pills">
                <button
                  v-for="srt in SORT_OPTIONS"
                  :key="srt.id"
                  type="button"
                  class="v-mf-pill"
                  :class="{ 'v-mf-pill--all': jobsStore.sortBy === srt.id }"
                  @click="jobsStore.sortBy = srt.id; refreshFilters()"
                >
                  {{ srt.label }}
                </button>
                <button
                  type="button"
                  class="v-mf-pill"
                  :class="{ 'v-mf-pill--manual': jobsStore.savedOnly }"
                  @click="handleToggleSavedFilter"
                >
                  Hanya Disimpan
                </button>
                <button
                  type="button"
                  class="v-mf-pill"
                  :class="{ 'v-mf-pill--manual': jobsStore.salaryOnly }"
                  @click="handleToggleSalaryFilter"
                >
                  Hanya Bergaji
                </button>
              </div>
            </div>

            <div class="v-filter-row">
              <label for="jobs-role-select" class="sr-only">Filter peran</label>
              <select
                id="jobs-role-select"
                :value="jobsStore.roleFilter ?? ''"
                class="v-select-compact"
                @change="
                  jobsStore.roleFilter = ($event.target as HTMLSelectElement).value || null;
                  refreshFilters();
                "
              >
                <option value="">Semua Peran</option>
                <option v-for="role in roleOptions" :key="role" :value="role">
                  {{ role }}
                </option>
              </select>
              <ScrapeKeywordsPopover />
            </div>

            <div class="v-filter-drawer__foot">
              <VButton
                v-if="hasActiveFilters"
                variant="tertiary"
                size="compact"
                @click="resetFilters"
              >
                Reset
              </VButton>
              <VButton
                variant="secondary"
                size="compact"
                @click="watchlistPopoverOpen = true; filtersExpanded = false"
              >
                <VIcon name="plus" :size="12" />
                <span>Simpan</span>
              </VButton>
              <VButton
                variant="primary"
                size="compact"
                style="margin-left: auto"
                @click="filtersExpanded = false"
              >
                Terapkan ({{ jobsStore.jobs.length }} hasil)
              </VButton>
            </div>
            </div>
          </Transition>
        </div>

        <div
          ref="jobsListRef"
          class="v-jobs-list"
          @touchstart.passive="onListTouchStart"
          @touchmove.passive="onListTouchMove"
          @touchend="onListTouchEnd"
          @touchcancel="onListTouchEnd"
        >
          <div
            v-if="ptrDistance > 6"
            class="v-ptr-indicator"
            :class="{ 'v-ptr-indicator--ready': ptrReady }"
            :style="{ height: `${Math.min(52, ptrDistance)}px` }"
            aria-live="polite"
          >
            <VIcon name="refresh" :size="14" :class="{ 'v-ptr-spin': ptrReady }" />
            <span>{{ ptrReady ? "Lepas untuk muat ulang" : "Tarik ke bawah untuk perbarui" }}</span>
          </div>

          <div
            v-if="jobsStore.loadingList || (jobsStore.scraping && jobsStore.jobs.length === 0)"
            class="v-list-skeleton"
            aria-busy="true"
            aria-label="Memuat daftar lowongan"
          >
            <div v-for="n in 5" :key="n" class="v-skel-row">
              <div class="v-skel-row__top">
                <VSkeleton width="68%" height="16px" />
                <VSkeleton width="40px" height="18px" />
              </div>
              <VSkeleton width="54%" height="12px" />
              <div class="v-skel-row__chips">
                <VSkeleton width="64px" height="18px" />
                <VSkeleton width="76px" height="18px" />
              </div>
            </div>
          </div>

          <div v-else-if="jobsStore.jobs.length === 0" class="v-list-empty">
            <p class="t-card">
              {{
                isWelcomeFlow && !hasActiveFilters
                  ? "Profilmu sudah siap! Ambil lowongan pertamamu sekarang."
                  : "Belum ada lowongan yang tampil."
              }}
            </p>
            <p class="t-meta" style="margin-top: var(--space-2)">
              {{
                hasActiveFilters
                  ? "Ubah kata kunci pencarian atau reset filter untuk melihat seluruh daftar."
                  : "Klik Cari Lowongan Sekarang untuk mengambil data resmi dari JobStreet, LinkedIn, Kalibrr & Dealls (0 token Gemini)."
              }}
            </p>
            <div style="margin-top: var(--space-4); display: flex; gap: 8px; justify-content: center; flex-wrap: wrap">
              <VButton
                v-if="hasActiveFilters"
                variant="secondary"
                size="compact"
                @click="resetFilters"
              >
                Reset Filter
              </VButton>
              <template v-else>
                <VButton
                  variant="primary"
                  size="compact"
                  :loading="jobsStore.scraping"
                  @click="handleRefreshScrape"
                >
                  Cari Lowongan Sekarang
                </VButton>
                <VButton variant="secondary" size="compact" @click="manualModalOpen = true">
                  Tempel Lowongan Manual
                </VButton>
              </template>
            </div>
          </div>

          <div class="v-jobs-list-inner">
            <JobListingRow
              v-for="job in jobsStore.jobs"
              :key="job.job_id"
              :job="job"
              :selected="jobsStore.selectedJobId === job.job_id"
              @select="handleSelectJob"
              @quick-save="handleQuickSave"
              @quick-dismiss="handleQuickDismiss"
            />
          </div>

          <div v-if="jobsStore.hasMore && !jobsStore.loadingList" class="v-load-more">
            <VButton
              variant="secondary"
              size="compact"
              :loading="jobsStore.loadingMore"
              style="width: 100%"
              @click="handleLoadMore"
            >
              {{ jobsStore.loadingMore ? "Memuat..." : "Muat Lebih Banyak" }}
            </VButton>
          </div>
        </div>
      </section>

      <!-- Right Detail Pane -->
      <section class="v-jobs-detail" aria-label="Detail lowongan terpilih">
        <div v-if="jobsStore.loadingDetail && !jobsStore.selectedDetail" class="v-detail-skeleton" aria-busy="true" aria-label="Memuat detail lowongan">
          <VSkeleton width="58%" height="28px" />
          <VSkeleton width="36%" height="14px" />
          <VSkeleton width="100%" height="96px" radius="var(--radius-card)" />
          <VSkeleton width="100%" height="180px" radius="var(--radius-card)" />
        </div>

        <div v-else-if="!jobsStore.selectedDetail" class="v-detail-empty">
          <p class="t-section">Pilih satu lowongan di sebelah kiri.</p>
          <p class="t-body" style="margin-top: var(--space-2)">
            Skor kecocokan hibrida, status lamaran, dan Copilot Lamaran akan tampil di sini.
          </p>
        </div>

        <div v-else :key="jobsStore.selectedDetail.job_id" class="v-detail-inner">
          <div v-if="mobileDetailOpen" class="v-mobile-back">
            <VButton
              variant="tertiary"
              size="compact"
              style="padding-left: 0"
              @click="mobileDetailOpen = false"
            >
              <VIcon name="back" :size="15" />
              <span>Kembali ke Daftar</span>
            </VButton>
          </div>

          <!-- Section 9.1: Title Most Prominent -->
          <div class="v-detail-head">
            <div style="flex: 1 1 320px; min-width: 0; width: 100%">
              <div class="v-detail-title-row">
                <h2 class="t-page">{{ jobsStore.selectedDetail.title }}</h2>
                <VBadge
                  :tone="
                    jobsStore.selectedDetail.enrichment_status === 'ai_enriched' ? 'ai' : 'neutral'
                  "
                >
                  <VIcon
                    v-if="jobsStore.selectedDetail.enrichment_status === 'ai_enriched'"
                    name="aiNode"
                    :size="11"
                  />
                  <span>
                    {{
                      jobsStore.selectedDetail.enrichment_status === "ai_enriched"
                        ? "AI · Hibrida"
                        : "Lokal · 0 token"
                    }}
                  </span>
                </VBadge>
              </div>
              <p class="t-body" style="margin-top: var(--space-2)">
                <button
                  type="button"
                  class="v-company-link"
                  title="Lihat profil perusahaan lokal"
                  @click="companyModalOpen = true"
                >
                  {{ jobsStore.selectedDetail.company }}
                </button>
                · {{ jobsStore.selectedDetail.location || "Indonesia" }} ·
                {{ formatWorkMode(jobsStore.selectedDetail.work_mode) }} ·
                {{ formatExperienceLevel(jobsStore.selectedDetail.experience_level) }} ·
                {{ formatJobSource(jobsStore.selectedDetail.source) }} ·
                {{ formatDomainSlug(jobsStore.selectedDetail.domain_slug) }} ·
                {{ formatRelativeDate(jobsStore.selectedDetail.scraped_at) }}
              </p>
              <p
                v-if="jobsStore.selectedDetail.also_available_on.length > 0"
                class="t-meta"
                style="margin-top: 4px"
              >
                Juga tayang di:
                {{
                  jobsStore.selectedDetail.also_available_on
                    .map((p) => p.source)
                    .join(", ")
                }}
              </p>
              <p
                v-if="jobsStore.selectedDetail.salary_text"
                class="t-secondary"
                style="color: var(--status-success); font-weight: 500; margin-top: 4px; display: flex; align-items: center; gap: 8px; flex-wrap: wrap"
              >
                <span>Estimasi gaji: {{ jobsStore.selectedDetail.salary_text }}</span>
                <span v-if="salaryComparisonLabel" class="v-salary-market-chip">
                  {{ salaryComparisonLabel }}
                </span>
              </p>
            </div>
            <div class="v-detail-actions">
              <VButton
                variant="secondary"
                size="compact"
                :loading="profileStore.exportingPdf"
                @click="handleExportTailoredPdf"
              >
                <span>Export CV ATS</span>
              </VButton>
              <VButton
                v-if="jobsStore.selectedDetail.enrichment_status === 'local_only'"
                variant="ai"
                size="compact"
                :loading="jobsStore.enrichingJob"
                @click="handleEnrichSingleJob"
              >
                <VIcon v-if="!jobsStore.enrichingJob" name="aiNode" :size="13" />
                <span>Perkaya AI</span>
              </VButton>
              <VButton
                variant="secondary"
                size="compact"
                :pressed="jobsStore.selectedDetail.is_saved"
                @click="handleToggleBookmark"
              >
                <VIcon
                  :name="jobsStore.selectedDetail.is_saved ? 'bookmarkFill' : 'bookmark'"
                  :size="14"
                />
                <span>{{ jobsStore.selectedDetail.is_saved ? "Tersimpan" : "Simpan" }}</span>
              </VButton>
              <VButton
                v-if="jobsStore.selectedDetail.url"
                variant="secondary"
                size="compact"
                @click="openExternalUrl(jobsStore.selectedDetail.url!)"
              >
                <span>Lowongan Asli</span>
                <VIcon name="external" :size="13" />
              </VButton>
            </div>
          </div>

          <!-- Red-Flag / Scam Protection Alert -->
          <div
            v-if="jobsStore.selectedDetail.red_flags.length > 0"
            class="v-redflag-box"
            role="alert"
          >
            <strong style="color: var(--status-warning); font-size: 13px">
              Peringatan Keamanan Lowongan (Skor Transparansi {{ Math.round((jobsStore.selectedDetail.quality_score ?? 1) * 100) }}%)
            </strong>
            <ul class="v-redflag-list">
              <li
                v-for="rf in jobsStore.selectedDetail.red_flags"
                :key="rf.code"
                class="t-secondary"
              >
                {{ rf.message }}
              </li>
            </ul>
          </div>

          <!-- Follow-Up & Interview Prep Prompt -->
          <div
            v-if="isFollowUpDue || jobsStore.selectedDetail.application_status === 'interview'"
            class="v-prep-box"
          >
            <strong style="color: var(--brand-hover); font-size: 13px">
              {{
                jobsStore.selectedDetail.application_status === "interview"
                  ? "Mode Persiapan Interview"
                  : "Waktunya Follow-Up Lamaran (>= 7 Hari)"
              }}
            </strong>
            <p class="t-secondary" style="margin-top: 4px">
              {{
                jobsStore.selectedDetail.application_status === "interview"
                  ? `Fokuskan jawaban STAR pada ${jobsStore.selectedDetail.matched_skills.slice(0, 3).map((s) => s.canonical_name).join(", ") || "pengalaman inti"} dan siapkan penjelasan untuk skill gap utama.`
                  : `Sudah lebih dari 7 hari sejak kamu melamar ke ${jobsStore.selectedDetail.company}. Kirim pesan follow-up singkat dan sopan untuk menanyakan progres seleksi.`
              }}
            </p>
          </div>

          <!-- Application Pipeline Tracker Bar -->
          <div class="v-pipeline-bar" aria-label="Pelacak status lamaran">
            <div class="v-pipeline-status">
              <label for="detail-app-status" class="t-meta">Status Lamaran</label>
              <select
                id="detail-app-status"
                :value="jobsStore.selectedDetail.application_status"
                :disabled="jobsStore.updatingApplication"
                class="v-pipeline-select"
                @change="
                  handleStatusChange(($event.target as HTMLSelectElement).value as ApplicationStatus)
                "
              >
                <option value="new">Baru</option>
                <option value="saved">Disimpan</option>
                <option value="applied">Dilamar</option>
                <option value="interview">Interview</option>
                <option value="offer">Offer</option>
                <option value="rejected">Ditolak</option>
                <option value="dismissed">Disembunyikan</option>
              </select>
              <VButton
                v-if="jobsStore.selectedDetail.application_status !== 'dismissed'"
                variant="tertiary"
                size="compact"
                :disabled="jobsStore.updatingApplication"
                @click="handleStatusChange('dismissed')"
              >
                Sembunyikan
              </VButton>
              <span v-if="jobsStore.selectedDetail.applied_at" class="t-meta">
                Dilamar {{ formatRelativeDate(jobsStore.selectedDetail.applied_at) }}
              </span>
            </div>

            <form class="v-pipeline-note" @submit.prevent="handleSaveNote">
              <label for="detail-app-note" class="sr-only">Catatan pribadi lamaran</label>
              <input
                id="detail-app-note"
                v-model="noteDraft"
                type="text"
                class="v-note-input"
                placeholder="Tambah catatan pribadi (mis. kontak HR, jadwal interview, link CV)..."
              />
              <VButton
                variant="secondary"
                size="compact"
                type="submit"
                :loading="jobsStore.updatingApplication"
              >
                Simpan Catatan
              </VButton>
            </form>
          </div>

          <!-- Section 9.2: Local Match Score Breakdown -->
          <MatchScorePanel
            :final-score="jobsStore.selectedDetail.final_score"
            :vector-score="jobsStore.selectedDetail.vector_score"
            :skill-score="jobsStore.selectedDetail.skill_overlap_score"
            :level-score="jobsStore.selectedDetail.level_score"
            :domain-score="jobsStore.selectedDetail.domain_score"
            :reranker-score="jobsStore.selectedDetail.reranker_score"
            :match-explanation="jobsStore.selectedDetail.match_explanation"
          />

          <!-- Matched Skills -->
          <div style="margin-top: var(--space-6)">
            <h3 class="t-card" style="margin-bottom: 10px">
              Skill Cocok
              <span class="t-meta t-num" style="margin-left: 6px">
                {{ jobsStore.selectedDetail.matched_skills.length }}
              </span>
            </h3>
            <p v-if="jobsStore.selectedDetail.matched_skills.length === 0" class="t-body">
              Belum ada skill yang cocok terdeteksi pada lowongan ini.
            </p>
            <div v-else class="v-skill-wrap">
              <span
                v-for="s in jobsStore.selectedDetail.matched_skills"
                :key="s.skill_id"
                class="v-skill-chip"
              >
                <span style="color: var(--status-success); display: inline-flex">
                  <VIcon name="check" :size="13" />
                </span>
                <span>{{ s.canonical_name }}</span>
              </span>
            </div>
          </div>

          <!-- Missing Skills -->
          <div style="margin-top: var(--space-5)">
            <div class="v-missing-head">
              <h3 class="t-card">
                Skill Belum Dimiliki
                <span class="t-meta t-num" style="margin-left: 6px">
                  {{ jobsStore.selectedDetail.missing_skills.length }}
                </span>
              </h3>
              <span v-if="jobsStore.selectedDetail.missing_skills.length > 0" class="t-meta">
                Klik Tambah jika sudah menguasainya (0 token)
              </span>
            </div>

            <p
              v-if="jobsStore.selectedDetail.missing_skills.length === 0"
              class="t-body"
              style="color: var(--status-success); font-weight: 500"
            >
              Seluruh skill terdeteksi pada lowongan ini sudah ada di profilmu.
            </p>

            <div v-else class="v-skill-wrap">
              <span
                v-for="m in jobsStore.selectedDetail.missing_skills"
                :key="m.skill_id"
                class="v-gap-chip"
              >
                <span class="v-gap-chip__label">
                  <span>{{ m.canonical_name }}</span>
                  <span class="t-meta" style="color: var(--status-warning)">
                    {{ m.requirement_type === "mandatory" ? "Wajib" : "Plus" }}
                  </span>
                </span>
                <button
                  type="button"
                  class="v-gap-chip__add"
                  :aria-label="`Tambahkan ${m.canonical_name} ke profil`"
                  @click="handleQuickAddSkill(m.skill_id, m.canonical_name)"
                >
                  <VIcon name="plus" :size="13" />
                  <span>Tambah</span>
                </button>
              </span>
            </div>
          </div>

          <!-- Section 9.3: AI Gap Analysis Card -->
          <GapAnalysisCard
            :analysis="jobsStore.selectedDetail.ai_gap_analysis"
            :loading="jobsStore.analyzingGap"
            :generating-roadmap="roadmapStore.generating"
            @analyze="handleGapAnalysis"
            @create-roadmap="handleCreateRoadmapFromJob"
          />

          <!-- Phase 4: On-Demand Apply Copilot -->
          <ApplyCopilotCard
            :kit="jobsStore.selectedDetail.ai_apply_kit"
            :loading="jobsStore.generatingApplyKit"
            @generate="handleGenerateApplyKit"
          />

          <!-- Wave 3: Local 0-Token STAR Interview Prep -->
          <InterviewPrepCard
            v-if="profileStore.userId"
            :user-id="profileStore.userId"
            :job-id="jobsStore.selectedDetail.job_id"
          />

          <section class="v-desc-reader" aria-label="Deskripsi Lowongan">
            <div class="v-desc-reader__head">
              <div class="v-desc-reader__title-group">
                <h3 class="t-card">Deskripsi Lowongan</h3>
                <span class="t-meta v-desc-reader__meta">
                  {{
                    jobsStore.selectedDetail.raw_description.trim().length >= 400
                      ? "Teks lengkap lokal"
                      : "Ringkasan lokal"
                  }}
                </span>
              </div>
              <VButton
                v-if="canEnrichDetailFromSource"
                variant="tertiary"
                size="compact"
                :loading="jobsStore.enrichingDescription"
                @click="handleEnrichDescriptionFromSource"
              >
                Muat Info Lengkap
              </VButton>
            </div>

            <div
              v-if="jobsStore.enrichingDescription && !jobsStore.selectedDetail.raw_description.trim()"
              class="v-desc-reader__body"
              aria-busy="true"
            >
              <VSkeleton width="94%" height="14px" />
              <VSkeleton width="88%" height="14px" />
              <VSkeleton width="72%" height="14px" />
            </div>

            <div v-else-if="parsedDescriptionSections.length > 0" class="v-desc-reader__body">
              <div
                v-for="(sec, idx) in parsedDescriptionSections"
                :key="idx"
                class="v-desc-reader__section"
              >
                <h4 v-if="sec.title" class="v-desc-reader__heading">{{ sec.title }}</h4>
                <template v-for="(blk, bIdx) in sec.blocks" :key="bIdx">
                  <ul v-if="blk.kind === 'list'" class="v-desc-reader__list">
                    <li v-for="(item, iIdx) in blk.items" :key="iIdx">{{ item }}</li>
                  </ul>
                  <p v-else class="v-desc-reader__para">{{ blk.text }}</p>
                </template>
              </div>
            </div>

            <div v-else class="v-desc-reader__body">
              <p class="v-desc-reader__para t-meta">
                Deskripsi belum tersedia dari portal sumber.
              </p>
            </div>
          </section>
        </div>
      </section>
    </div>

    <ManualJobModal
      ref="manualModalRef"
      :open="manualModalOpen"
      :submitting="jobsStore.ingestingManual"
      @close="manualModalOpen = false"
      @submit="handleManualSubmit"
    />

    <SearchPreferencesDrawer
      :open="preferencesOpen"
      :preferences="jobsStore.searchPreferences"
      :portal-statuses="jobsStore.portalStatuses"
      :saving="savingPrefs"
      @close="preferencesOpen = false"
      @save="handleSavePreferences"
    />

    <CompanySnapshotModal
      v-if="profileStore.userId && jobsStore.selectedDetail"
      :open="companyModalOpen"
      :user-id="profileStore.userId"
      :company-name="jobsStore.selectedDetail.company"
      @close="companyModalOpen = false"
    />
  </div>
</template>

<style scoped>
.v-jobs-shell {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.v-jobs-mode-bar {
  flex-shrink: 0;
  padding: var(--space-2) var(--space-8) var(--space-1);
  background: var(--bg-primary);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}
.v-jobs-tabs {
  display: inline-flex;
  gap: 3px;
  padding: 3px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--surface-recessed);
  box-shadow: var(--shadow-recessed);
  min-width: 0;
}
.v-jobs-tab {
  height: 30px;
  padding: 0 12px;
  border-radius: 7px;
  border: 0;
  background: transparent;
  color: var(--text-2);
  font-size: 12.5px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
  transition: color 140ms ease-out, background-color 140ms ease-out, transform var(--dur-press) ease-out;
}
.v-jobs-tab:not(:disabled):active {
  transform: scale(0.96);
}
.v-jobs-tab--active {
  background: var(--surface-1);
  color: var(--text-1);
  box-shadow: var(--shadow-control);
}
.v-jobs-tab-count {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: var(--radius-full);
  background: var(--surface-2);
  color: var(--text-2);
}
.v-jobs-tab-count--mobile {
  display: none;
}
.v-mobile-mode-actions {
  display: none;
}
.v-mobile-menu-wrap {
  position: relative;
}
.v-overflow-btn {
  width: 36px;
  height: 36px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-1);
  color: var(--text-1);
  box-shadow: var(--shadow-control);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background-color 140ms ease-out, transform var(--dur-press) ease-out;
}
.v-overflow-btn:not(:disabled):active {
  transform: scale(0.96);
}
.v-mobile-menu-pop {
  position: absolute;
  right: 0;
  top: calc(100% + 6px);
  min-width: 210px;
  padding: 6px;
  border-radius: var(--radius-card);
  border: 1px solid var(--border);
  background: var(--surface-1);
  box-shadow: var(--shadow-overlay), inset 0 1px 0 rgba(255, 255, 255, 0.08);
  display: flex;
  flex-direction: column;
  gap: 2px;
  z-index: 45;
}
.v-mobile-menu-item {
  width: 100%;
  min-height: 38px;
  padding: 0 10px;
  border-radius: var(--radius-control);
  border: 0;
  background: transparent;
  color: var(--text-1);
  font-size: 13px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 8px;
  text-align: left;
  transition: background-color 140ms ease-out;
}
@media (hover: hover) {
  .v-mobile-menu-item:hover {
    background: var(--surface-hover);
  }
}
.v-company-link {
  border: 0;
  padding: 0;
  background: transparent;
  color: var(--text-primary);
  font-weight: 600;
  text-decoration: underline;
  text-decoration-color: var(--border-strong);
  text-underline-offset: 3px;
  cursor: pointer;
}
@media (hover: hover) {
  .v-company-link:hover {
    color: var(--brand-hover);
    text-decoration-color: var(--brand-hover);
  }
}
.v-salary-market-chip {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: var(--brand-soft);
  color: var(--text-1);
}
.v-jobs-banner {
  flex-shrink: 0;
  padding: var(--space-3) var(--space-8);
  background: var(--warning-soft);
  border-bottom: 1px solid var(--warning-border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-progress-bar {
  flex-shrink: 0;
  padding: var(--space-3) var(--space-8);
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-divider);
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.v-progress-bar__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}
.v-progress-track {
  height: 4px;
  border-radius: 999px;
  background: var(--bg-hover);
  overflow: hidden;
}
.v-progress-fill {
  height: 100%;
  border-radius: 999px;
  background: var(--brand-primary);
  transition: width var(--dur-normal) var(--ease-out);
}
.v-progress-fill--indeterminate {
  width: 36%;
  animation: vectaIndeterminate 1.4s ease-in-out infinite;
}
@keyframes vectaIndeterminate {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(300%);
  }
}
.v-ai-queue-bar {
  flex-shrink: 0;
  padding: 8px var(--space-8);
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-divider);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: nowrap;
}
.v-ai-queue-bar__text {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}
.v-ai-queue-bar__copy {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.v-ai-queue-bar__actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-shrink: 0;
}
.v-icon-dismiss {
  width: 30px;
  height: 30px;
  border-radius: var(--radius-control);
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
@media (hover: hover) {
  .v-icon-dismiss:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
}
.v-detail-title-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-pipeline-bar {
  margin-top: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid var(--border-default);
  background: var(--bg-secondary);
  box-shadow: var(--shadow-surface);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  min-width: 0;
  max-width: 100%;
}
.v-redflag-box {
  margin-top: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid var(--warning-border);
  background: var(--warning-soft);
}
.v-redflag-list {
  margin: 6px 0 0;
  padding-left: 18px;
  list-style-type: disc;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.v-prep-box {
  margin-top: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid var(--brand-border);
  background: var(--brand-soft);
}
.v-pipeline-status {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-pipeline-select {
  height: 34px;
  padding: 0 28px 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: 13px;
  font-weight: 500;
}
.v-pipeline-note {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
  min-width: 0;
  max-width: 100%;
}
.v-note-input {
  flex: 1 1 160px;
  min-width: 0;
  max-width: 100%;
  height: 36px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--surface-recessed);
  box-shadow: var(--shadow-recessed);
  color: var(--text-primary);
  font-size: 13px;
}
.v-note-input:focus {
  border-color: var(--brand-hover);
}
.v-jobs-split {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}
.v-jobs-rail {
  width: 400px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border-divider);
  background: var(--bg-primary);
  min-height: 0;
}
.v-jobs-filters {
  flex-shrink: 0;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border-divider);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.v-search-bar-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  min-width: 0;
}
.v-search-field {
  flex: 1;
  min-width: 0;
  height: 40px;
  padding: 0 var(--space-2) 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-recessed);
  box-shadow: var(--shadow-recessed);
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.v-search-field:focus-within {
  border-color: var(--brand-hover);
}
.v-search-icon {
  color: var(--text-secondary);
  flex-shrink: 0;
}
.v-search-input {
  flex: 1;
  min-width: 0;
  width: 100%;
  height: 100%;
  border: 0;
  background: transparent;
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
}
.v-search-clear {
  width: 26px;
  height: 26px;
  border-radius: var(--radius-sm);
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.v-filter-toggle {
  height: 40px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-default);
  background: var(--bg-elevated);
  color: var(--text-primary);
  box-shadow: var(--shadow-control);
  font-size: 13px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  transition: background-color 140ms ease-out, border-color 140ms ease-out, transform var(--dur-press) ease-out;
}
.v-filter-toggle:not(:disabled):active {
  transform: scale(0.96);
}
.v-filter-toggle--active {
  background: var(--brand-soft);
  border-color: var(--brand-border);
}
.v-filter-badge {
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: var(--radius-full);
  background: var(--brand-primary);
  color: var(--brand-fg);
  font-size: 11px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v-active-chips {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.v-active-chip {
  height: 26px;
  padding: 0 8px;
  border-radius: var(--radius-full);
  border: 1px solid var(--brand-border);
  background: var(--brand-soft);
  color: var(--text-primary);
  font-size: 11.5px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.v-chip-reset {
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  font-size: 11.5px;
  font-weight: 500;
  padding: 2px 6px;
}
.v-filter-drawer {
  padding-top: var(--space-3);
  border-top: 1px solid var(--border-divider);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  max-height: 56vh;
  overflow-y: auto;
}
.v-watchlist-strip {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow-x: auto;
  padding-bottom: 2px;
  scrollbar-width: none;
}
.v-watchlist-strip::-webkit-scrollbar {
  display: none;
}
.v-wl-chip {
  display: inline-flex;
  align-items: center;
  flex-shrink: 0;
  height: 28px;
  border-radius: var(--radius-full);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-2);
  overflow: hidden;
  transition: background-color 140ms ease, border-color 140ms ease, color 140ms ease;
}
.v-wl-chip--active {
  background: var(--filter-active-tint);
  border-color: var(--filter-active-border);
  color: var(--status-success);
}
.v-wl-btn {
  border: 0;
  background: transparent;
  color: inherit;
  font-size: 11.5px;
  font-weight: 600;
  padding: 0 8px 0 10px;
  height: 100%;
  white-space: nowrap;
}
.v-wl-del {
  border: 0;
  background: transparent;
  color: var(--text-3);
  padding: 0 7px 0 2px;
  height: 100%;
  display: inline-flex;
  align-items: center;
}
@media (hover: hover) {
  .v-wl-del:hover {
    color: var(--status-error);
  }
  .v-wl-add:hover {
    border-color: var(--brand);
    color: var(--text-1);
  }
}
.v-wl-add {
  flex-shrink: 0;
  height: 28px;
  padding: 0 10px;
  border-radius: var(--radius-full);
  border: 1px dashed var(--border-strong);
  background: transparent;
  color: var(--text-2);
  font-size: 11.5px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.v-wl-form {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}
.v-wl-input {
  flex: 1;
  min-width: 0;
  height: 34px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-strong);
  background: var(--surface-1);
  color: var(--text-1);
  font-size: 12.5px;
}
.v-mf-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.v-mf-label {
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.06em;
  color: var(--text-3);
}
.v-mf-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.v-mf-pill {
  height: 30px;
  padding: 0 11px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 12px;
  font-weight: 600;
  transition: transform var(--dur-press) ease-out;
}
.v-mf-pill:not(:disabled):active {
  transform: scale(0.96);
}
/* Active "Semua" pill = Quiet selected monochrome (never competes with primary CTA) */
.v-mf-pill--all {
  background: var(--pill-all-bg);
  border-color: var(--pill-all-border);
  color: var(--pill-all-fg);
}
/* Active Manual Multi-Select Pill = Calibrated Pine Emerald WCAG AA (ZERO checkmarks) */
.v-mf-pill--manual {
  background: var(--filter-active-bg);
  border-color: var(--filter-active-border);
  color: var(--filter-active-fg);
}
@media (hover: hover) {
  .v-mf-pill:hover {
    background: var(--surface-hover);
    color: var(--text-1);
  }
  .v-mf-pill--all:hover {
    background: var(--pill-all-bg);
    color: var(--pill-all-fg);
  }
  .v-mf-pill--manual:hover {
    background: var(--filter-active-hover);
    color: var(--filter-active-fg);
  }
}
.v-jobs-list-inner {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.v-filter-drawer__foot {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.v-filter-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-2);
}
.v-filter-row :deep(.v-btn),
.v-filter-row :deep(.v-kw-wrap) {
  width: 100%;
}
.v-select-compact,
.v-select-mode {
  width: 100%;
  min-width: 0;
  height: 36px;
  padding: 0 24px 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: 13px;
}
.v-list-skeleton {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.v-skel-row {
  padding: var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid var(--border-divider);
  background: var(--bg-elevated);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.v-detail-skeleton {
  max-width: 780px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.v-load-more {
  padding-top: var(--space-2);
}
.v-skel-row__top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.v-skel-row__chips {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 2px;
}
.v-ptr-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
  border-radius: var(--radius-control);
  background: var(--bg-secondary);
  border: 1px solid var(--border-divider);
  overflow: hidden;
  transition: color 120ms var(--ease-standard), border-color 120ms var(--ease-standard);
}
.v-ptr-indicator--ready {
  color: var(--text-primary);
  border-color: var(--border-card);
}
.v-ptr-spin {
  transform: rotate(180deg);
  transition: transform 160ms var(--ease-out-expo);
}
.v-desc-reader {
  margin-top: var(--space-6);
  border: 1px solid var(--border-divider);
  border-radius: var(--radius-card);
  background: var(--bg-secondary);
  box-shadow: var(--shadow-surface);
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
}
.v-desc-reader__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: var(--space-2) var(--space-3);
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--border-divider);
}
.v-desc-reader__title-group {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: var(--space-2);
  min-width: 0;
}
.v-desc-reader__meta {
  font-size: 12px;
}
.v-desc-reader__body {
  padding: var(--space-4) var(--space-5) var(--space-5);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  font-family: var(--font-primary);
  font-size: 13.5px;
  line-height: 1.65;
  color: var(--text-secondary);
  word-break: break-word;
  overflow-wrap: anywhere;
}
.v-desc-reader__section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.v-desc-reader__heading {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: -0.01em;
}
.v-desc-reader__para {
  margin: 0;
}
.v-desc-reader__list {
  margin: 0;
  padding-left: 20px;
  list-style: disc;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.v-jobs-list {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.v-list-empty {
  padding: var(--space-12) var(--space-5);
  text-align: center;
}
.v-jobs-detail {
  flex: 1;
  min-width: 0;
  max-width: 100%;
  overflow-y: auto;
  overflow-x: clip;
  touch-action: pan-y;
  padding: var(--space-6) var(--space-10) var(--space-12);
}
.v-detail-inner {
  min-width: 0;
  width: 100%;
  max-width: 780px;
  margin: 0 auto;
}
.v-detail-empty {
  padding: var(--space-16) 0;
  text-align: center;
}
.v-mobile-back {
  display: none;
  margin-bottom: var(--space-4);
}
.v-detail-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-5);
  flex-wrap: wrap;
  min-width: 0;
}
.v-detail-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
  flex-shrink: 1;
  max-width: 100%;
}
.v-missing-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-3);
  margin-bottom: 10px;
  flex-wrap: wrap;
}
.v-skill-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.v-skill-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  height: 32px;
  padding: 0 var(--space-3);
  background: var(--bg-hover);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}
.v-gap-chip {
  display: inline-flex;
  align-items: stretch;
  background: var(--warning-soft);
  border: 1px solid var(--warning-border);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.v-gap-chip__label {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  height: 34px;
  padding: 0 var(--space-3);
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}
.v-gap-chip__add {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0 var(--space-3);
  border: 0;
  border-left: 1px solid var(--warning-border);
  background: transparent;
  color: var(--status-warning);
  font-size: 12px;
  font-weight: 500;
}
@media (hover: hover) {
  .v-gap-chip__add:hover {
    background: var(--warning-soft);
  }
}

@media (max-width: 1279px) {
  .v-jobs-rail {
    width: 360px;
  }
  .v-jobs-filters {
    padding: var(--space-3);
  }
}
@media (max-width: 1199px) {
  .v-jobs-detail {
    padding: var(--space-6);
  }
}
.v-filter-backdrop,
.v-sheet-grab {
  display: none;
}

@media (max-width: 1023px) {
  .v-jobs-desktop-header {
    display: none;
  }
  .v-jobs-mode-bar {
    padding: 8px 12px 6px;
    gap: 8px;
    flex-wrap: nowrap;
    border-bottom: 1px solid var(--border-divider);
  }
  .v-jobs-tabs {
    flex: 1 1 0;
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    overflow: hidden;
  }
  .v-jobs-tab {
    height: 32px;
    min-width: 0;
    padding: 0 6px;
    justify-content: center;
    gap: 4px;
    font-size: 12px;
    overflow: hidden;
  }
  .v-jobs-tab-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .v-jobs-tab-count {
    font-size: 10.5px;
    padding: 1px 5px;
    flex-shrink: 0;
  }
  .v-jobs-tab-count--mobile {
    display: inline-flex;
  }
  .v-mobile-mode-actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .v-mobile-scrape-btn {
    min-width: 72px;
    padding: 0 10px;
  }
  .v-watchlist-strip--empty {
    display: none;
  }
  .v-jobs-filters {
    padding: var(--space-2) var(--space-3);
    border-bottom: 0;
  }
  .v-hide-xs {
    display: none;
  }
  .v-filter-backdrop {
    display: block;
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgba(0, 0, 0, 0.56);
  }
  .v-sheet-grab {
    display: block;
    width: 40px;
    height: 4px;
    border-radius: var(--radius-full);
    background: var(--border-strong);
    margin: 0 auto 6px;
    flex-shrink: 0;
  }
  .v-filter-drawer {
    position: fixed;
    left: 10px;
    right: 10px;
    bottom: calc(76px + env(safe-area-inset-bottom, 0px));
    z-index: 61;
    max-height: calc(100dvh - 152px);
    padding: 14px var(--space-4) 12px;
    border-radius: 20px;
    border: 1px solid var(--border);
    box-shadow: var(--shadow-overlay), inset 0 1px 0 rgba(255, 255, 255, 0.08);
    background: var(--surface-1);
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .v-filter-drawer__foot {
    position: sticky;
    bottom: -12px;
    padding: 10px 0 2px;
    background: var(--surface-1);
    border-top: 1px solid var(--border-divider);
  }
  .v-jobs-shell {
    height: auto;
    min-height: 100%;
    max-width: 100%;
    overflow-x: clip;
  }
  .v-jobs-split {
    overflow-x: clip;
    max-width: 100%;
    flex: none;
  }
  .v-jobs-banner,
  .v-progress-bar,
  .v-ai-queue-bar {
    padding: 6px var(--space-3);
  }
  .v-jobs-rail {
    width: 100%;
    max-width: 100%;
    min-width: 0;
    border-right: 0;
    overflow-x: clip;
  }
  .v-jobs-list {
    overflow: visible;
  }
  .v-jobs-detail {
    padding: var(--space-4) var(--space-3) var(--space-10);
  }
  .v-detail-actions {
    width: 100%;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2);
  }
  .v-detail-actions :deep(.v-btn) {
    width: 100%;
    min-width: 0;
  }
  .v-mobile-back {
    display: block;
  }
  .v-jobs-split[data-mobile-detail="false"] .v-jobs-detail {
    display: none;
  }
  .v-jobs-split[data-mobile-detail="true"] .v-jobs-rail {
    display: none;
  }
}
</style>
