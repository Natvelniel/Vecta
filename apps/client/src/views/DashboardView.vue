<script setup lang="ts">
import { computed, onMounted } from "vue";
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

onMounted(async () => {
  if (!profileStore.activeProfile) {
    await profileStore.loadActiveProfile();
  }
  const uid = profileStore.userId;
  if (uid) {
    await Promise.all([jobsStore.loadJobs(uid), roadmapStore.loadRoadmaps(uid)]);
  }
});

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

function openJob(jobId: string) {
  jobsStore.selectedJobId = jobId;
  router.push("/jobs");
}

function openPipelineJob(jobId: string) {
  jobsStore.selectedJobId = jobId;
  router.push({ path: "/jobs", query: { tab: "pipeline", jobId } });
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
        <!-- Quiet Unboxed Metrics Strip (Section 2.3: no redundant stat cards) -->
        <div class="v-metrics-strip">
          <div>
            <div class="t-meta">Lowongan Cocok (&ge; 75%)</div>
            <div class="t-display t-num" style="margin-top: var(--space-2)">
              {{ highFitCount }}
            </div>
            <div class="t-meta" style="margin-top: 6px">
              dari {{ jobsStore.jobs.length }} lowongan tersimpan
            </div>
          </div>

          <div>
            <div class="t-meta">Lamaran Aktif</div>
            <div class="t-display t-num" style="margin-top: var(--space-2); color: var(--status-success)">
              {{ appliedCount }}
            </div>
            <div class="t-meta" style="margin-top: 6px">
              {{ interviewCount }} tahap interview · {{ jobsStore.pendingLocalCount }} antrean AI
            </div>
          </div>

          <div>
            <div class="t-meta">Progres Roadmap Aktif</div>
            <div class="t-display t-num" style="margin-top: var(--space-2); color: var(--brand-hover)">
              {{ activeRoadmapProgress }}%
            </div>
            <div class="t-meta" style="margin-top: 6px">{{ activeRoadmapStepRatio }}</div>
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
              <h2 class="t-section">Langkah Belajar Berikutnya</h2>
              <VButton
                variant="tertiary"
                size="compact"
                style="margin-right: -12px"
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

        <!-- Perlu Tindakan Hari Ini + Mobile Quick Shortcuts -->
        <div class="v-dash-extra">
          <VCard>
            <div class="v-col-head" style="margin-bottom: var(--space-2)">
              <h2 class="t-section">Perlu Tindakan Hari Ini</h2>
              <VButton
                variant="tertiary"
                size="compact"
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
              <VIcon name="roadmap" :size="18" />
              <div style="text-align: left">
                <strong style="display: block; font-size: 13.5px; color: var(--text-1)">Roadmap Belajar</strong>
                <span class="t-meta">Lokal 0 Token &amp; AI</span>
              </div>
            </button>
            <button type="button" class="v-shortcut-card" @click="router.push('/cv')">
              <VIcon name="cv" :size="18" />
              <div style="text-align: left">
                <strong style="display: block; font-size: 13.5px; color: var(--text-1)">Studio CV ATS</strong>
                <span class="t-meta">Live Preview A4</span>
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
.v-dash-columns {
  display: grid;
  grid-template-columns: 1.15fr 1fr;
  gap: var(--space-12);
  margin-top: var(--space-8);
}
.v-col-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: var(--space-3);
}
.v-col-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0 calc(var(--space-4) * -1);
}
.v-list-row {
  width: 100%;
  display: block;
  text-align: left;
  padding: var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid transparent;
  background: transparent;
  transition: background var(--dur-hover) var(--ease-state);
}
.v-list-row:hover {
  background: var(--bg-elevated);
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
  padding: var(--space-6);
  border-radius: var(--radius-card);
  border: 1px solid var(--border-divider);
  background: var(--bg-elevated);
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
.v-action-row:hover {
  border-color: var(--brand-border);
  background: var(--bg-hover);
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
  grid-template-columns: 1fr 1fr;
  gap: var(--space-3);
}
.v-shortcut-card {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid var(--border);
  background: var(--surface-1);
  color: var(--text-1);
}
.v-shortcut-card:hover {
  border-color: var(--brand-border);
}
@media (max-width: 1023px) {
  .v-page-container {
    padding: var(--space-5) var(--space-4) var(--space-10);
  }
  .v-metrics-strip,
  .v-dash-columns {
    grid-template-columns: 1fr;
    gap: var(--space-6);
  }
  .v-mobile-shortcuts {
    display: grid;
  }
}
</style>
