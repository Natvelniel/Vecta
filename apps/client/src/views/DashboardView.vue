<script setup lang="ts">
import { computed, onActivated, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useJobsStore } from "@/stores/jobs";
import { useProfileStore } from "@/stores/profile";
import { useRoadmapStore } from "@/stores/roadmap";
import { formatExperienceLevel, formatHours, formatScore, formatWorkMode } from "@/lib/format";
import PageHeader from "@/components/layout/PageHeader.vue";
import ScoreRing from "@/components/ui/ScoreRing.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";
import VIcon from "@/components/ui/VIcon.vue";

const router = useRouter();
const profileStore = useProfileStore();
const jobsStore = useJobsStore();
const roadmapStore = useRoadmapStore();

async function syncDashboardData() {
  if (!profileStore.activeProfile) {
    await profileStore.loadActiveProfile();
  }
  const uid = profileStore.userId;
  if (uid) {
    await Promise.all([
      profileStore.loadActiveProfile(),
      jobsStore.loadJobs(uid),
      roadmapStore.loadRoadmaps(uid),
    ]);
  }
}

onMounted(syncDashboardData);
onActivated(syncDashboardData);

const highFitCount = computed(
  () => jobsStore.jobs.filter((j) => formatScore(j.final_score) >= 75).length,
);

const appliedCount = computed(
  () =>
    jobsStore.jobs.filter((j) =>
      ["applied", "interview", "offer"].includes(j.application_status),
    ).length,
);

const interviewCount = computed(
  () => jobsStore.jobs.filter((j) => j.application_status === "interview").length,
);

const actionItems = computed(() => {
  const items: Array<{ jobId: string; title: string; company: string; kind: "interview" | "followup"; hint: string }> = [];
  const now = Date.now();
  for (const job of jobsStore.jobs) {
    if (job.application_status === "interview") {
      items.push({
        jobId: job.job_id,
        title: job.title,
        company: job.company,
        kind: "interview",
        hint: "Siapkan jawaban STAR & tanya balik HR",
      });
    } else if (job.application_status === "applied") {
      const refTs = Date.parse(job.scraped_at);
      if (!Number.isNaN(refTs) && now - refTs >= 7 * 24 * 3600_000) {
        items.push({
          jobId: job.job_id,
          title: job.title,
          company: job.company,
          kind: "followup",
          hint: "Sudah >= 7 hari · kirim pesan follow-up",
        });
      }
    }
  }
  return items.slice(0, 4);
});

const activeRoadmapProgress = computed(() => {
  const first = roadmapStore.roadmaps[0];
  return first ? formatScore(first.progress_percentage) : 0;
});

const activeRoadmapStepRatio = computed(() => {
  const first = roadmapStore.roadmaps[0];
  if (!first) return "Belum ada roadmap aktif";
  return `${first.completed_steps} dari ${first.total_steps} langkah selesai`;
});

const nextSteps = computed(() => {
  if (!roadmapStore.activeDetail) return [];
  return roadmapStore.activeDetail.steps.filter((s) => s.status !== "done").slice(0, 3);
});

const headerSubtitle = computed(() => {
  const p = profileStore.activeProfile;
  if (!p) return "Belum ada profil aktif";
  const role = p.target_roles[0] || "Profil Utama";
  return `${role} (${formatExperienceLevel(p.experience_level)}) · ${profileStore.activeKeywordsCount} kata kunci aktif`;
});

async function openJob(jobId: string) {
  jobsStore.selectedJobId = jobId;
  if (typeof window !== "undefined" && window.innerWidth < 1024) {
    await router.push("/jobs");
    await router.push({ path: "/jobs", query: { detail: jobId } });
    return;
  }
  await router.push({ path: "/jobs", query: { detail: jobId } });
}

async function openPipelineJob(jobId: string) {
  jobsStore.selectedJobId = jobId;
  if (typeof window !== "undefined" && window.innerWidth < 1024) {
    await router.push({ path: "/jobs", query: { tab: "pipeline" } });
    await router.push({ path: "/jobs", query: { tab: "pipeline", detail: jobId } });
    return;
  }
  await router.push({ path: "/jobs", query: { tab: "pipeline", detail: jobId } });
}

function formatSalaryMini(minIdr: number | null, maxIdr: number | null): string | null {
  if (minIdr && maxIdr && maxIdr > minIdr) {
    return `Rp ${(minIdr / 1_000_000).toFixed(0)}-${(maxIdr / 1_000_000).toFixed(0)} jt`;
  }
  if (minIdr) return `Rp ${(minIdr / 1_000_000).toFixed(0)} jt+`;
  return null;
}
</script>

<template>
  <div class="v-dash-root">
    <PageHeader title="Ringkasan Karier" :subtitle="headerSubtitle">
      <template #actions>
        <VButton variant="primary" @click="router.push(profileStore.activeProfile ? '/jobs' : '/onboarding')">
          {{ profileStore.activeProfile ? "Buka Daftar Lowongan" : "Buat Profil Cerita" }}
        </VButton>
      </template>
    </PageHeader>

    <div class="v-page-container">
      <!-- Empty state when user skipped onboarding -->
      <VCard v-if="!profileStore.activeProfile && !profileStore.loading">
        <h2 class="t-section">Mulai dari Cerita Pengalamanmu</h2>
        <p class="t-body" style="margin-top: var(--space-2); max-width: 62ch">
          Belum ada profil aktif di database lokal. Ceritakan proyek dan keahlianmu dalam satu
          paragraf agar Vecta dapat mencocokkan lowongan dan memetakan celah kompetensimu.
        </p>
        <div style="margin-top: var(--space-5)">
          <VButton variant="primary" @click="router.push('/onboarding')">
            Tulis Profil Cerita
          </VButton>
        </div>
      </VCard>

      <template v-else>
        <!-- Quiet Architectural Metrics Strip -->
        <div class="v-metrics-strip">
          <div class="v-metric-cell">
            <div class="t-meta v-metric-label">Cocok &ge;75%</div>
            <div class="t-display t-num v-metric-val">
              {{ highFitCount }}
            </div>
            <div class="t-meta v-metric-sub">
              dari {{ jobsStore.jobs.length }} lowongan
            </div>
          </div>

          <div class="v-metric-cell">
            <div class="t-meta v-metric-label">Lamaran</div>
            <div class="t-display t-num v-metric-val" style="color: var(--status-success)">
              {{ appliedCount }}
            </div>
            <div class="t-meta v-metric-sub">
              {{ interviewCount }} interview · {{ jobsStore.pendingLocalCount }} antrean
            </div>
          </div>

          <div class="v-metric-cell">
            <div class="t-meta v-metric-label">Roadmap</div>
            <div class="t-display t-num v-metric-val" style="color: var(--brand-hover)">
              {{ activeRoadmapProgress }}%
            </div>
            <div class="t-meta v-metric-sub">{{ activeRoadmapStepRatio }}</div>
          </div>
        </div>

        <div class="v-dash-columns">
          <section>
            <div class="v-col-head">
              <h2 class="t-section">Lowongan Paling Cocok</h2>
              <VButton
                variant="tertiary"
                size="compact"
                style="margin-right: -12px"
                @click="router.push('/jobs')"
              >
                Lihat semua
              </VButton>
            </div>

            <div v-if="jobsStore.jobs.length === 0" class="v-col-empty">
              <p class="t-body">Belum ada lowongan tersimpan.</p>
              <VButton
                variant="secondary"
                size="compact"
                style="margin-top: var(--space-3)"
                @click="router.push('/jobs')"
              >
                Cari atau Tempel Lowongan
              </VButton>
            </div>

            <div v-else class="v-col-list">
              <button
                v-for="job in jobsStore.jobs.slice(0, 5)"
                :key="job.job_id"
                type="button"
                class="v-list-row"
                @click="openJob(job.job_id)"
              >
                <div class="v-list-row__inner">
                  <div style="min-width: 0">
                    <div class="t-card" style="font-size: 15px">{{ job.title }}</div>
                    <div class="t-secondary" style="margin-top: 2px">
                      {{ job.company }} · {{ job.location || "Indonesia" }} ·
                      {{ formatWorkMode(job.work_mode) }}
                    </div>
                    <div
                      v-if="formatSalaryMini(job.salary_min_idr, job.salary_max_idr) || job.opportunity_type !== 'full_time'"
                      class="v-dash-pills"
                    >
                      <span
                        v-if="formatSalaryMini(job.salary_min_idr, job.salary_max_idr)"
                        class="v-mini-pill v-mini-pill--salary t-num"
                      >
                        {{ formatSalaryMini(job.salary_min_idr, job.salary_max_idr) }}
                      </span>
                      <span
                        v-if="job.opportunity_type !== 'full_time'"
                        class="v-mini-pill"
                      >
                        {{ job.opportunity_type === 'mt_odp' ? 'MT / ODP' : job.opportunity_type }}
                      </span>
                    </div>
                  </div>
                  <ScoreRing :score="job.final_score" :size="40" />
                </div>
              </button>
            </div>
          </section>

          <section>
            <div class="v-col-head">
              <h2 class="t-section v-col-title">Langkah Roadmap</h2>
              <VButton
                variant="tertiary"
                size="compact"
                class="v-col-action"
                @click="router.push('/roadmap')"
              >
                Buka Roadmap
              </VButton>
            </div>

            <div v-if="nextSteps.length === 0" class="v-col-empty">
              <p class="t-body">Belum ada langkah roadmap aktif.</p>
              <VButton
                variant="secondary"
                size="compact"
                style="margin-top: var(--space-3)"
                @click="router.push('/competency')"
              >
                Cek Peta Kompetensi
              </VButton>
            </div>

            <div v-else class="v-col-list">
              <button
                v-for="step in nextSteps"
                :key="step.id"
                type="button"
                class="v-list-row"
                @click="router.push('/roadmap')"
              >
                <div class="v-step-meta">
                  <span class="t-meta">{{ step.phase_name }}</span>
                  <span class="t-meta t-num">{{ formatHours(step.estimated_hours) }}</span>
                </div>
                <div class="t-card" style="font-size: 15px; margin-top: 4px">
                  {{ step.topic_title }}
                </div>
                <div
                  v-if="step.skill_name"
                  class="t-meta"
                  style="margin-top: 6px; color: var(--status-warning)"
                >
                  Target skill: {{ step.skill_name }}
                </div>
              </button>
            </div>
          </section>
        </div>

        <!-- Tindakan Hari Ini + Mobile Quick Shortcuts -->
        <div class="v-dash-extra">
          <VCard>
            <div class="v-col-head" style="margin-bottom: var(--space-2)">
              <h2 class="t-section v-col-title">Tindakan Hari Ini</h2>
              <VButton
                variant="tertiary"
                size="compact"
                class="v-col-action"
                @click="router.push({ path: '/jobs', query: { tab: 'pipeline' } })"
              >
                Buka Pipeline
              </VButton>
            </div>
            <p v-if="actionItems.length === 0" class="t-secondary">
              Semua lamaranmu terpantau rapi. Tandai lowongan sebagai Dilamar atau Interview untuk pengingat otomatis di sini.
            </p>
            <div v-else class="v-action-list">
              <button
                v-for="item in actionItems"
                :key="item.jobId"
                type="button"
                class="v-action-row"
                @click="openPipelineJob(item.jobId)"
              >
                <div style="min-width: 0; text-align: left">
                  <strong class="v-action-title">{{ item.title }}</strong>
                  <div class="t-meta">{{ item.company }} · {{ item.hint }}</div>
                </div>
                <span class="v-action-badge" :data-kind="item.kind">
                  {{ item.kind === "interview" ? "Interview" : "Follow-up" }}
                </span>
              </button>
            </div>
          </VCard>

          <div class="v-mobile-shortcuts">
            <button type="button" class="v-shortcut-card" @click="router.push('/roadmap')">
              <span class="v-shortcut-icon">
                <VIcon name="roadmap" :size="18" />
              </span>
              <div class="v-shortcut-copy">
                <strong class="v-shortcut-title">Roadmap Belajar</strong>
                <span class="t-meta v-shortcut-sub">0 Token &amp; AI</span>
              </div>
            </button>
            <button type="button" class="v-shortcut-card" @click="router.push('/cv')">
              <span class="v-shortcut-icon">
                <VIcon name="cv" :size="18" />
              </span>
              <div class="v-shortcut-copy">
                <strong class="v-shortcut-title">Studio CV ATS</strong>
                <span class="t-meta v-shortcut-sub">Pratinjau A4</span>
              </div>
            </button>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.v-dash-root {
  display: flex;
  flex-direction: column;
  flex: 1;
}
.v-page-container {
  max-width: 1200px;
  width: 100%;
  margin: 0 auto;
  padding: var(--space-8);
}
.v-metrics-strip {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-8);
  padding-bottom: var(--space-8);
  border-bottom: 1px solid var(--border-divider);
}
.v-metric-cell {
  min-width: 0;
}
.v-metric-val {
  margin-top: var(--space-2);
}
.v-metric-sub {
  margin-top: 6px;
}
.v-dash-columns {
  display: grid;
  grid-template-columns: 1.15fr 1fr;
  gap: var(--space-12);
  margin-top: var(--space-8);
}
.v-col-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  margin-bottom: var(--space-3);
}
.v-col-title {
  min-width: 0;
}
.v-col-action {
  flex-shrink: 0;
  white-space: nowrap;
  margin-right: -8px;
}
.v-col-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
}
.v-list-row {
  width: 100%;
  display: block;
  text-align: left;
  padding: var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid var(--border-default);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
  transition:
    background var(--dur-hover) var(--ease-state),
    border-color var(--dur-hover) var(--ease-state),
    transform var(--dur-press) ease-out;
}
.v-list-row:active {
  transform: scale(0.99);
}
@media (hover: hover) {
  .v-list-row:hover {
    background: var(--bg-elevated);
    border-color: var(--brand-border);
  }
}
.v-list-row__inner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
}
.v-row-score {
  font-size: 15px;
  font-weight: 700;
  color: var(--brand-hover);
  flex-shrink: 0;
}
.v-step-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}
.v-col-empty {
  padding: var(--space-5);
  border-radius: var(--radius-card);
  border: 1px solid var(--border-default);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-surface);
}
.v-dash-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
}
.v-mini-pill {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 7px;
  border-radius: var(--radius-sm);
  background: var(--brand-soft);
  color: var(--text-1);
}
.v-mini-pill--salary {
  background: var(--success-soft);
  color: var(--status-success);
}
.v-dash-extra {
  margin-top: var(--space-8);
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}
.v-action-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: var(--space-2);
}
.v-action-row {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-divider);
  background: var(--bg-primary);
}
@media (hover: hover) {
  .v-action-row:hover {
    border-color: var(--brand-border);
    background: var(--bg-hover);
  }
}
.v-action-title {
  display: block;
  font-size: 13.5px;
  color: var(--text-1);
}
.v-action-badge {
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 700;
  padding: 3px 9px;
  border-radius: var(--radius-full);
  background: var(--warning-soft);
  color: var(--status-warning);
}
.v-action-badge[data-kind="interview"] {
  background: var(--brand-soft);
  color: var(--brand-hover);
}
.v-mobile-shortcuts {
  display: none;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-3);
}
.v-shortcut-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--space-2);
  min-width: 0;
  padding: var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid var(--border-default);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
  color: var(--text-1);
  text-align: left;
  transition: border-color var(--dur-hover) var(--ease-state), transform var(--dur-press) ease-out;
}
.v-shortcut-card:active {
  transform: scale(0.96);
}
.v-shortcut-icon {
  width: 36px;
  height: 36px;
  border-radius: var(--radius-control);
  background: var(--surface-recessed);
  box-shadow: var(--shadow-recessed);
  border: 1px solid var(--border);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-1);
}
.v-shortcut-copy {
  min-width: 0;
  width: 100%;
}
.v-shortcut-title {
  display: block;
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.v-shortcut-sub {
  display: block;
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
@media (hover: hover) {
  .v-shortcut-card:hover {
    border-color: var(--brand-border);
  }
}
@media (max-width: 1023px) {
  .v-page-container {
    padding: var(--space-4) var(--space-4) var(--space-10);
  }
  .v-metrics-strip {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    padding: 12px 10px;
    border-bottom: 0;
    border-radius: var(--radius-card);
    background: var(--surface-recessed);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-recessed);
  }
  .v-metric-cell {
    text-align: left;
    padding: 0 4px;
  }
  .v-metric-cell + .v-metric-cell {
    border-left: 1px solid var(--border-divider);
    padding-left: 10px;
  }
  .v-metric-label {
    font-size: 11px;
    line-height: 1.3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .v-metric-val {
    font-size: 22px;
    margin-top: 2px;
  }
  .v-metric-sub {
    font-size: 10.5px;
    line-height: 1.3;
    margin-top: 2px;
  }
  .v-dash-columns {
    grid-template-columns: 1fr;
    gap: var(--space-6);
    margin-top: var(--space-6);
  }
  .v-mobile-shortcuts {
    display: grid;
  }
}
</style>
