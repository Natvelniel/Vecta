<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import type {
  ApplicationStatus,
  JobSource,
  SearchPreferences,
  WorkMode,
} from "@/lib/bindings";
import { useCompetencyStore } from "@/stores/competency";
import { useJobsStore } from "@/stores/jobs";
import { useProfileStore } from "@/stores/profile";
import { useRoadmapStore } from "@/stores/roadmap";
import { useToast } from "@/composables/useToast";
import {
  formatApplicationStatus,
  formatDomainSlug,
  formatExperienceLevel,
  formatJobSource,
  formatRelativeDate,
  formatWorkMode,
} from "@/lib/format";
import { ipc, IpcError, listenScrapeProgress, openExternalUrl } from "@/lib/ipc";
import PageHeader from "@/components/layout/PageHeader.vue";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VIcon from "@/components/ui/VIcon.vue";
import VSkeleton from "@/components/ui/VSkeleton.vue";
import JobListingRow from "@/components/domain/JobListingRow.vue";
import MatchScorePanel from "@/components/domain/MatchScorePanel.vue";
import GapAnalysisCard from "@/components/domain/GapAnalysisCard.vue";
import ApplyCopilotCard from "@/components/domain/ApplyCopilotCard.vue";
import ManualJobModal from "@/components/domain/ManualJobModal.vue";
import SearchPreferencesDrawer from "@/components/domain/SearchPreferencesDrawer.vue";
import ScrapeKeywordsPopover from "@/components/domain/ScrapeKeywordsPopover.vue";

const router = useRouter();
const profileStore = useProfileStore();
const jobsStore = useJobsStore();
const competencyStore = useCompetencyStore();
const roadmapStore = useRoadmapStore();
const toast = useToast();

const manualModalOpen = ref(false);
const preferencesOpen = ref(false);
const savingPrefs = ref(false);
const manualModalRef = ref<InstanceType<typeof ManualJobModal> | null>(null);
const mobileDetailOpen = ref(false);
const noteDraft = ref("");
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;
let unlistenProgress: (() => void) | null = null;

watch(
  () => jobsStore.selectedDetail,
  (detail) => {
    noteDraft.value = detail?.user_notes ?? "";
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  if (unlistenProgress) unlistenProgress();
});

function handleSearchInput(value: string) {
  jobsStore.searchQuery = value;
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  searchDebounceTimer = setTimeout(() => {
    void refreshFilters();
  }, 220);
}

async function handleLoadMore() {
  if (!profileStore.userId) return;
  await jobsStore.loadMoreJobs(profileStore.userId);
}

onMounted(async () => {
  unlistenProgress = await listenScrapeProgress((evt) => {
    jobsStore.progress = evt;
  });
  await jobsStore.loadSearchPreferences();
  if (!profileStore.activeProfile) {
    await profileStore.loadActiveProfile();
  }
  const uid = profileStore.userId;
  if (uid) {
    await Promise.all([jobsStore.loadJobs(uid), competencyStore.loadRolesAndMap(uid)]);
  }
});

const uniqueScrapeWarnings = computed(() => {
  const warnings = jobsStore.lastScrapeReport?.warnings ?? [];
  const bySource = new Map<string, { source: string; count: number; sample: string }>();
  for (const w of warnings) {
    const existing = bySource.get(w.source);
    if (existing) {
      existing.count += 1;
    } else {
      bySource.set(w.source, { source: w.source, count: 1, sample: w.message });
    }
  }
  return Array.from(bySource.values());
});

const hasActiveFilters = computed(
  () =>
    jobsStore.savedOnly ||
    Boolean(jobsStore.roleFilter) ||
    Boolean(jobsStore.workModeFilter) ||
    Boolean(jobsStore.sourceFilter) ||
    Boolean(jobsStore.applicationStatusFilter) ||
    Boolean(jobsStore.searchQuery.trim()),
);

const progressPercent = computed(() => {
  const p = jobsStore.progress;
  if (!p || p.total <= 0) return 0;
  return Math.min(100, Math.round((p.current / p.total) * 100));
});

function dismissScrapeWarnings() {
  jobsStore.lastScrapeReport = null;
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
  jobsStore.roleFilter = null;
  jobsStore.workModeFilter = null;
  jobsStore.sourceFilter = null;
  jobsStore.applicationStatusFilter = null;
  jobsStore.searchQuery = "";
  jobsStore.savedOnly = false;
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

async function handleToggleBookmark() {
  if (!profileStore.userId || !jobsStore.selectedDetail) return;
  await jobsStore.toggleSave(profileStore.userId, jobsStore.selectedDetail.job_id);
}

async function handleSavePreferences(prefs: SearchPreferences, runImmediately: boolean) {
  savingPrefs.value = true;
  const ok = await jobsStore.saveSearchPreferences(prefs);
  savingPrefs.value = false;
  if (!ok) {
    toast.show(jobsStore.error ?? "Gagal menyimpan preferensi pencarian.", { tone: "error" });
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
    const aiNote = jobsStore.searchPreferences.ai_enrich_enabled
      ? " + AI Matching selesai."
      : " (0 token Gemini).";
    toast.show(
      `Selesai: ${report.newly_ingested} lowongan baru tersimpan lokal, ${report.skipped_duplicates} duplikat dilewati${aiNote}`,
      { tone: report.warnings.length > 0 ? "warning" : "success" },
    );
  }
}

async function handleRunAiQueue() {
  if (!profileStore.userId) return;
  const report = await jobsStore.runAiMatchingQueue(profileStore.userId);
  if (report) {
    toast.show(
      `AI Matching selesai: ${report.enriched_jobs} lowongan diperkaya (${report.api_calls_used} req Gemini), ${report.remaining_local_jobs} tersisa lokal.`,
      { tone: report.failed_jobs > 0 ? "warning" : "success" },
    );
  } else if (jobsStore.error) {
    toast.show(jobsStore.error, { tone: "error" });
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
    toast.show(jobsStore.error, { tone: "error" });
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
    toast.show(jobsStore.error, { tone: "error" });
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
    toast.show(
      `CV ATS PDF ter-tailor disimpan di ${res.file_path} (Skor ATS ${Math.round(res.ats_keyword_score ?? 100)}%).`,
      { tone: "success" },
    );
  } else if (profileStore.error) {
    toast.show(profileStore.error, { tone: "error" });
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
    toast.show(e instanceof IpcError ? e.message : "Gagal menambahkan skill.", { tone: "error" });
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
    toast.show(roadmapStore.error, { tone: "error" });
  }
}
</script>

<template>
  <div class="v-jobs-shell">
    <PageHeader
      title="Lowongan"
      :subtitle="`${jobsStore.jobs.length} lowongan · ${jobsStore.searchPreferences.location} · ${jobsStore.searchPreferences.sources.length} portal`"
    >
      <template #actions>
        <VButton variant="secondary" @click="preferencesOpen = true">
          <VIcon name="sliders" :size="14" />
          <span>Atur Pencarian</span>
        </VButton>
        <VButton variant="secondary" @click="manualModalOpen = true">
          <VIcon name="plus" :size="14" />
          <span>Tempel Manual</span>
        </VButton>
        <VButton variant="primary" :loading="jobsStore.scraping" @click="handleRefreshScrape">
          <VIcon v-if="!jobsStore.scraping" name="refresh" :size="14" />
          <span>{{ jobsStore.scraping ? "Mengambil..." : "Cari Lowongan" }}</span>
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
      v-if="!jobsStore.scraping && !jobsStore.runningAiQueue && jobsStore.pendingLocalCount > 0"
      class="v-ai-queue-bar"
      role="status"
    >
      <div class="v-ai-queue-bar__text">
        <VBadge tone="ai">
          <VIcon name="aiNode" :size="12" />
          <span>Antrean AI</span>
        </VBadge>
        <span class="t-secondary">
          <strong class="t-num" style="color: var(--text-primary)">
            {{ jobsStore.pendingLocalCount }}
          </strong>
          lowongan tersimpan lokal (0 token). Jalankan AI Matching batch untuk skor semantik.
        </span>
      </div>
      <VButton
        variant="ai"
        size="compact"
        :loading="jobsStore.runningAiQueue"
        @click="handleRunAiQueue"
      >
        <VIcon name="aiNode" :size="13" />
        <span>
          Jalankan AI Matching (Top {{ jobsStore.searchPreferences.ai_enrich_max_jobs }})
        </span>
      </VButton>
    </div>

    <div v-if="jobsStore.error" class="v-jobs-banner" role="alert">
      <span class="t-secondary" style="color: var(--status-warning)">
        {{ jobsStore.error }}
      </span>
      <VButton variant="secondary" size="compact" @click="manualModalOpen = true">
        Gunakan Tempel Manual
      </VButton>
    </div>

    <div
      v-if="uniqueScrapeWarnings.length > 0"
      class="v-jobs-banner"
      role="status"
    >
      <span class="t-secondary" style="color: var(--status-warning)">
        Peringatan parsial scraper:
        {{
          uniqueScrapeWarnings
            .map((w) =>
              w.count > 1
                ? `${w.source} (${w.count}x): ${w.sample}`
                : `${w.source}: ${w.sample}`,
            )
            .join(" · ")
        }}
      </span>
      <VButton variant="tertiary" size="compact" @click="dismissScrapeWarnings">
        Tutup
      </VButton>
    </div>

    <div class="v-jobs-split" :data-mobile-detail="String(mobileDetailOpen)">
      <!-- Left 400px Scannable Job Rail -->
      <section class="v-jobs-rail" aria-label="Daftar lowongan terurut">
        <div class="v-jobs-filters">
          <label for="jobs-search-input" class="sr-only">Cari judul atau perusahaan</label>
          <input
            id="jobs-search-input"
            type="search"
            :value="jobsStore.searchQuery"
            class="v-search-input"
            placeholder="Cari judul atau perusahaan..."
            @input="handleSearchInput(($event.target as HTMLInputElement).value)"
          />

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

            <label for="jobs-mode-select" class="sr-only">Filter mode kerja</label>
            <select
              id="jobs-mode-select"
              :value="jobsStore.workModeFilter ?? ''"
              class="v-select-mode"
              @change="
                jobsStore.workModeFilter =
                  (($event.target as HTMLSelectElement).value as WorkMode) || null;
                refreshFilters();
              "
            >
              <option value="">Semua Mode</option>
              <option value="remote">Remote</option>
              <option value="hybrid">Hybrid</option>
              <option value="onsite">Onsite</option>
            </select>

            <label for="jobs-status-select" class="sr-only">Filter status lamaran</label>
            <select
              id="jobs-status-select"
              :value="jobsStore.applicationStatusFilter ?? ''"
              class="v-select-compact"
              @change="
                jobsStore.applicationStatusFilter =
                  (($event.target as HTMLSelectElement).value as ApplicationStatus) || null;
                refreshFilters();
              "
            >
              <option value="">Semua Status</option>
              <option value="new">Baru</option>
              <option value="saved">Disimpan</option>
              <option value="applied">Dilamar</option>
              <option value="interview">Interview</option>
              <option value="offer">Offer</option>
              <option value="rejected">Ditolak</option>
              <option value="dismissed">Disembunyikan</option>
            </select>

            <label for="jobs-source-select" class="sr-only">Filter portal sumber</label>
            <select
              id="jobs-source-select"
              :value="jobsStore.sourceFilter ?? ''"
              class="v-select-mode"
              @change="
                jobsStore.sourceFilter =
                  (($event.target as HTMLSelectElement).value as JobSource) || null;
                refreshFilters();
              "
            >
              <option value="">Semua Portal</option>
              <option value="jobstreet">JobStreet</option>
              <option value="linkedin">LinkedIn</option>
              <option value="kalibrr">Kalibrr</option>
              <option value="dealls">Dealls</option>
              <option value="indeed">Indeed</option>
              <option value="glints">Glints</option>
              <option value="manual">Manual</option>
            </select>

            <VButton
              variant="secondary"
              size="compact"
              style="height: 36px"
              :pressed="jobsStore.savedOnly"
              @click="handleToggleSavedFilter"
            >
              <VIcon :name="jobsStore.savedOnly ? 'bookmarkFill' : 'bookmark'" :size="14" />
              <span>Simpan</span>
            </VButton>

            <ScrapeKeywordsPopover />
          </div>
        </div>

        <div class="v-jobs-list">
          <div v-if="jobsStore.loadingList" class="v-list-skeleton" aria-busy="true" aria-label="Memuat daftar lowongan">
            <div v-for="n in 4" :key="n" class="v-skel-row">
              <div style="display: flex; justify-content: space-between; gap: 12px">
                <VSkeleton width="68%" height="16px" />
                <VSkeleton width="36px" height="16px" />
              </div>
              <VSkeleton width="52%" height="12px" />
              <VSkeleton width="40%" height="12px" />
            </div>
          </div>

          <div v-else-if="jobsStore.jobs.length === 0" class="v-list-empty">
            <p class="t-card">Belum ada lowongan yang tampil.</p>
            <p class="t-meta" style="margin-top: var(--space-2)">
              {{
                hasActiveFilters
                  ? "Ubah kata kunci pencarian atau reset filter untuk melihat seluruh daftar."
                  : "Klik Cari Lowongan, atur kota & portal di Atur Pencarian, atau tempel deskripsi manual."
              }}
            </p>
            <div style="margin-top: var(--space-4); display: flex; gap: 8px; justify-content: center">
              <VButton
                v-if="hasActiveFilters"
                variant="secondary"
                size="compact"
                @click="resetFilters"
              >
                Reset Filter
              </VButton>
              <VButton v-else variant="secondary" size="compact" @click="manualModalOpen = true">
                Tempel Manual
              </VButton>
            </div>
          </div>

          <JobListingRow
            v-for="job in jobsStore.jobs"
            :key="job.job_id"
            :job="job"
            :selected="jobsStore.selectedJobId === job.job_id"
            @select="handleSelectJob"
          />

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

        <div v-else class="v-detail-inner">
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
            <div style="flex: 1; min-width: 260px">
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
                <strong style="color: var(--text-primary); font-weight: 500">
                  {{ jobsStore.selectedDetail.company }}
                </strong>
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
                style="color: var(--status-success); font-weight: 500; margin-top: 4px"
              >
                Estimasi gaji: {{ jobsStore.selectedDetail.salary_text }}
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

          <details
            v-if="jobsStore.selectedDetail.raw_description"
            class="v-raw-desc"
          >
            <summary class="v-raw-desc__summary">
              <span>Lihat Deskripsi Lowongan Asli</span>
              <span class="t-meta">Teks bersih lokal</span>
            </summary>
            <pre class="v-raw-desc__body">{{ jobsStore.selectedDetail.raw_description }}</pre>
          </details>
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
      :saving="savingPrefs"
      @close="preferencesOpen = false"
      @save="handleSavePreferences"
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
  padding: 10px var(--space-8);
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-divider);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-ai-queue-bar__text {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
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
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
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
}
.v-note-input {
  flex: 1;
  min-width: 200px;
  height: 34px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--bg-primary);
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
.v-search-input {
  width: 100%;
  height: 36px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: 13px;
}
.v-search-input:focus {
  border-color: var(--brand-hover);
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
.v-raw-desc {
  margin-top: var(--space-6);
  border: 1px solid var(--border-divider);
  border-radius: var(--radius-card);
  background: var(--bg-secondary);
  overflow: hidden;
}
.v-raw-desc__summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  cursor: pointer;
  list-style: none;
}
.v-raw-desc__summary::-webkit-details-marker {
  display: none;
}
.v-raw-desc__summary:hover {
  background: var(--bg-elevated);
}
.v-raw-desc__body {
  margin: 0;
  padding: 0 var(--space-5) var(--space-5);
  border-top: 1px solid var(--border-divider);
  padding-top: var(--space-4);
  font-family: var(--font-primary);
  font-size: 13px;
  line-height: 1.65;
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
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
  overflow-y: auto;
  padding: var(--space-6) var(--space-10) var(--space-12);
}
.v-detail-inner {
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
}
.v-detail-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-shrink: 0;
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
.v-gap-chip__add:hover {
  background: rgba(240, 184, 90, 0.18);
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
@media (max-width: 1023px) {
  .v-jobs-banner,
  .v-progress-bar,
  .v-ai-queue-bar {
    padding: var(--space-3) var(--space-4);
  }
  .v-jobs-rail {
    width: 100%;
    border-right: 0;
  }
  .v-jobs-filters {
    flex-wrap: wrap;
  }
  .v-jobs-detail {
    padding: var(--space-5) var(--space-4) var(--space-10);
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
