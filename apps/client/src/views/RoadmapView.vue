<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import type { StepStatus } from "@/lib/bindings";
import { useCompetencyStore } from "@/stores/competency";
import { useJobsStore } from "@/stores/jobs";
import { useProfileStore } from "@/stores/profile";
import { useRoadmapStore } from "@/stores/roadmap";
import { useToast } from "@/composables/useToast";
import { formatHours } from "@/lib/format";
import { ipc, IpcError } from "@/lib/ipc";
import PageHeader from "@/components/layout/PageHeader.vue";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";
import VIcon from "@/components/ui/VIcon.vue";

const router = useRouter();
const profileStore = useProfileStore();
const roadmapStore = useRoadmapStore();
const jobsStore = useJobsStore();
const competencyStore = useCompetencyStore();
const toast = useToast();

onMounted(async () => {
  if (!profileStore.activeProfile) {
    await profileStore.loadActiveProfile();
  }
  if (profileStore.userId) {
    await roadmapStore.loadRoadmaps(profileStore.userId);
  }
});

const doneCount = computed(
  () => roadmapStore.activeDetail?.steps.filter((s) => s.status === "done").length ?? 0,
);
const totalSteps = computed(() => roadmapStore.activeDetail?.steps.length ?? 0);
const progressPct = computed(() =>
  totalSteps.value > 0 ? Math.round((doneCount.value / totalSteps.value) * 100) : 0,
);

async function handleToggleStep(stepId: string, currentStatus: StepStatus) {
  if (!profileStore.userId) return;
  const result = await roadmapStore.toggleStepDone(profileStore.userId, stepId, currentStatus);
  handleStepResult(result);
}

async function handleSetInProgress(stepId: string, currentStatus: StepStatus) {
  if (!profileStore.userId) return;
  const next: StepStatus = currentStatus === "in_progress" ? "todo" : "in_progress";
  await roadmapStore.setStepStatus(profileStore.userId, stepId, next);
}

function handleStepResult(result: Awaited<ReturnType<typeof roadmapStore.toggleStepDone>>) {
  if (result?.suggest_add_skill) {
    const sk = result.suggest_add_skill;
    toast.show(`Selesai belajar ${sk.canonical_name}. Tambahkan ke profilmu sekarang?`, {
      tone: "success",
      actionLabel: "Tambahkan ke Profil",
      onAction: async () => {
        if (!profileStore.userId) return;
        try {
          await ipc.quickAddUserSkill(profileStore.userId, sk.skill_id);
          await Promise.all([
            profileStore.loadActiveProfile(),
            jobsStore.loadJobs(profileStore.userId),
            competencyStore.loadRolesAndMap(profileStore.userId),
          ]);
          toast.show(`${sk.canonical_name} ditambahkan. Skor lowongan dihitung ulang lokal.`, {
            tone: "success",
          });
        } catch (e) {
          toast.show(e instanceof IpcError ? e.message : "Gagal menambahkan skill.", {
            tone: "error",
          });
        }
      },
    });
  }
}

async function copyKeyword(keyword: string) {
  try {
    await navigator.clipboard.writeText(keyword);
    toast.show(`Kata kunci "${keyword}" disalin ke clipboard.`, { tone: "info" });
  } catch {
    toast.show(`Kata kunci belajar: ${keyword}`, { tone: "info" });
  }
}

async function handleSelectRoadmap(e: Event) {
  const id = (e.target as HTMLSelectElement).value;
  if (id) await roadmapStore.selectRoadmap(id);
}
</script>

<template>
  <div class="v-rm-root">
    <PageHeader :title="roadmapStore.activeDetail?.title ?? 'Roadmap Belajar'">
      <template #title-extra>
        <VBadge v-if="roadmapStore.activeDetail" tone="ai">
          <VIcon name="aiNode" :size="13" />
          <span>AI-Generated</span>
        </VBadge>
      </template>
      <template #actions>
        <label v-if="roadmapStore.roadmaps.length > 1" for="rm-picker" class="sr-only">
          Pilih roadmap
        </label>
        <select
          v-if="roadmapStore.roadmaps.length > 1"
          id="rm-picker"
          :value="roadmapStore.activeRoadmapId ?? ''"
          class="v-rm-select"
          @change="handleSelectRoadmap"
        >
          <option v-for="rm in roadmapStore.roadmaps" :key="rm.id" :value="rm.id">
            {{ rm.title }} ({{ Math.round(rm.progress_percentage) }}%)
          </option>
        </select>

        <span v-if="roadmapStore.activeDetail" class="t-meta t-num">
          <strong style="color: var(--brand-hover); font-size: 15px">{{ progressPct }}%</strong>
          · {{ doneCount }}/{{ totalSteps }} langkah selesai
        </span>
      </template>
    </PageHeader>

    <div class="v-narrow-container">
      <VCard v-if="!roadmapStore.loading && !roadmapStore.activeDetail">
        <h2 class="t-section">Belum Ada Roadmap Belajar</h2>
        <p class="t-body" style="margin-top: var(--space-2); max-width: 58ch">
          Susun roadmap 3 fase (Fundamental, Praktik, Proyek Mini) dari celah skill pada halaman
          Lowongan atau Peta Kompetensi Pasar.
        </p>
        <div style="margin-top: var(--space-5); display: flex; gap: var(--space-3); flex-wrap: wrap">
          <VButton variant="primary" @click="router.push('/competency')">
            Buka Peta Kompetensi
          </VButton>
          <VButton variant="secondary" @click="router.push('/jobs')">Buka Lowongan</VButton>
        </div>
      </VCard>

      <template v-else-if="roadmapStore.activeDetail">
        <p class="t-body" style="margin-bottom: var(--space-3)">
          Estimasi
          <strong class="t-num" style="color: var(--text-primary)">
            {{ roadmapStore.activeDetail.total_estimated_weeks }} minggu
          </strong>
          · Centang langkah selesai untuk memperbarui skill profilmu secara otomatis.
        </p>

        <div
          class="v-bar-track"
          role="progressbar"
          aria-label="Progres roadmap"
          :aria-valuenow="progressPct"
          aria-valuemin="0"
          aria-valuemax="100"
        >
          <div class="v-bar-fill" :style="{ width: `${progressPct}%` }" />
        </div>

        <div class="v-timeline">
          <div class="v-timeline-rail" aria-hidden="true" />
          <article
            v-for="step in roadmapStore.activeDetail.steps"
            :key="step.id"
            class="v-timeline-step"
          >
            <button
              type="button"
              class="v-step-check"
              :aria-pressed="step.status === 'done'"
              :aria-label="`${step.status === 'done' ? 'Tandai belum selesai' : 'Tandai selesai'}: ${step.topic_title}`"
              @click="handleToggleStep(step.id, step.status)"
            >
              <span class="v-step-box">
                <VIcon v-if="step.status === 'done'" name="check" :size="13" />
              </span>
            </button>

            <div class="v-step-head">
              <div>
                <div class="v-step-tags">
                  <span class="t-meta">{{ step.phase_name }}</span>
                  <VBadge v-if="step.skill_name">{{ step.skill_name }}</VBadge>
                  <VBadge v-if="step.status === 'in_progress'" tone="info">
                    Sedang dipelajari
                  </VBadge>
                </div>
                <h2
                  class="t-card"
                  :style="
                    step.status === 'done'
                      ? 'text-decoration: line-through; color: var(--text-secondary);'
                      : undefined
                  "
                >
                  {{ step.topic_title }}
                </h2>
              </div>
              <div class="v-step-meta">
                <button
                  v-if="step.status !== 'done'"
                  type="button"
                  class="v-progress-toggle"
                  :aria-pressed="step.status === 'in_progress'"
                  @click="handleSetInProgress(step.id, step.status)"
                >
                  {{ step.status === "in_progress" ? "Jeda" : "Mulai Belajar" }}
                </button>
                <span class="t-meta t-num">{{ formatHours(step.estimated_hours) }}</span>
              </div>
            </div>

            <p v-if="step.mini_project_idea" class="t-body" style="margin-top: var(--space-2)">
              <strong style="color: var(--text-primary); font-weight: 500">Proyek Mini:</strong>
              {{ step.mini_project_idea }}
            </p>

            <div v-if="step.learning_keywords.length > 0" class="v-kw-pills">
              <button
                v-for="kw in step.learning_keywords"
                :key="kw"
                type="button"
                class="v-kw-pill"
                title="Klik untuk menyalin kata kunci belajar"
                @click="copyKeyword(kw)"
              >
                {{ kw }}
              </button>
            </div>
          </article>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.v-rm-root {
  display: flex;
  flex-direction: column;
  flex: 1;
}
.v-narrow-container {
  max-width: 820px;
  width: 100%;
  margin: 0 auto;
  padding: var(--space-8);
}
.v-rm-select {
  height: 36px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-default);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: 13px;
}
.v-bar-track {
  height: 5px;
  width: 100%;
  background: var(--bg-hover);
  border-radius: var(--radius-full);
  overflow: hidden;
}
.v-bar-fill {
  height: 100%;
  background: var(--brand-primary);
  border-radius: var(--radius-full);
  transition: width 240ms var(--ease-out);
}
.v-timeline {
  position: relative;
  padding-left: 44px;
  margin-top: var(--space-6);
}
.v-timeline-rail {
  position: absolute;
  left: 19px;
  top: 24px;
  bottom: 24px;
  width: 2px;
  background: var(--border-divider);
}
.v-timeline-step {
  position: relative;
  padding: var(--space-5) 0 var(--space-6);
  border-bottom: 1px solid var(--border-divider);
}
.v-step-check {
  position: absolute;
  left: -44px;
  top: 12px;
  width: 40px;
  height: 40px;
  border: 0;
  background: transparent;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v-step-box {
  width: 22px;
  height: 22px;
  border-radius: var(--radius-sm);
  border: 1.5px solid var(--border-input);
  background: var(--bg-primary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #ffffff;
  transition: background var(--dur-hover) var(--ease-state), border-color var(--dur-hover) var(--ease-state);
}
.v-step-check[aria-pressed="true"] .v-step-box {
  background: var(--brand-primary);
  border-color: var(--brand-primary);
}
.v-step-meta {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
}
.v-progress-toggle {
  height: 28px;
  padding: 0 10px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-default);
  background: var(--bg-elevated);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  transition: background var(--dur-hover) var(--ease-state), color var(--dur-hover) var(--ease-state);
}
.v-progress-toggle:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.v-progress-toggle[aria-pressed="true"] {
  background: var(--info-soft);
  border-color: rgba(85, 166, 232, 0.35);
  color: var(--status-info);
}
.v-step-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-step-tags {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-1);
}
.v-kw-pills {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-top: var(--space-3);
}
.v-kw-pill {
  padding: 4px 10px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-default);
  background: var(--bg-elevated);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  transition: background var(--dur-hover) var(--ease-state), color var(--dur-hover) var(--ease-state);
}
.v-kw-pill:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
@media (max-width: 1023px) {
  .v-narrow-container {
    padding: var(--space-5) var(--space-4) var(--space-10);
  }
}
</style>
