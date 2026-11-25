<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useJobsStore } from "@/stores/jobs";
import { useProfileStore } from "@/stores/profile";
import { useRoadmapStore } from "@/stores/roadmap";
import { formatExperienceLevel, formatHours, formatScore, formatWorkMode } from "@/lib/format";
import PageHeader from "@/components/layout/PageHeader.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";

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
                  </div>
                  <span class="t-num v-row-score">{{ formatScore(job.final_score) }}%</span>
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
@media (max-width: 1023px) {
  .v-page-container {
    padding: var(--space-5) var(--space-4) var(--space-10);
  }
  .v-metrics-strip,
  .v-dash-columns {
    grid-template-columns: 1fr;
    gap: var(--space-6);
  }
}
</style>
