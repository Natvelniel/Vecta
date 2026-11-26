<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import type { SkillCategory, SkillImpactItem } from "@/lib/bindings";
import { useCompetencyStore } from "@/stores/competency";
import { useJobsStore } from "@/stores/jobs";
import { useProfileStore } from "@/stores/profile";
import { useRoadmapStore } from "@/stores/roadmap";
import { useToast } from "@/composables/useToast";
import { formatProficiency, formatScore, formatSkillCategory } from "@/lib/format";
import { ipc, IpcError } from "@/lib/ipc";
import PageHeader from "@/components/layout/PageHeader.vue";
import SalaryBenchmarkCard from "@/components/domain/SalaryBenchmarkCard.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";
import VIcon from "@/components/ui/VIcon.vue";
import VSkeleton from "@/components/ui/VSkeleton.vue";
import VSwitch from "@/components/ui/VSwitch.vue";

const router = useRouter();
const profileStore = useProfileStore();
const competencyStore = useCompetencyStore();
const jobsStore = useJobsStore();
const roadmapStore = useRoadmapStore();
const toast = useToast();

const CLUSTERS: Array<{ id: SkillCategory | "all"; label: string }> = [
  { id: "all", label: "Semua" },
  { id: "core_hard", label: "Inti Teknis" },
  { id: "tools_tech", label: "Alat & Infra" },
  { id: "soft_skill", label: "Komunikasi" },
  { id: "domain", label: "Domain" },
];

const simSkillIds = ref<number[]>([]);
const simResults = ref<SkillImpactItem[]>([]);
const simLoading = ref(false);

async function runImpactSimulation() {
  const uid = profileStore.userId;
  const role = competencyStore.selectedRole ?? "";
  if (!uid || simSkillIds.value.length === 0) {
    simResults.value = [];
    return;
  }
  simLoading.value = true;
  try {
    simResults.value = await ipc.simulateSkillImpact(uid, role, simSkillIds.value);
  } catch {
    simResults.value = [];
  } finally {
    simLoading.value = false;
  }
}

function toggleSimulateSkill(skillId: number) {
  if (simSkillIds.value.includes(skillId)) {
    simSkillIds.value = simSkillIds.value.filter((id) => id !== skillId);
  } else {
    if (simSkillIds.value.length >= 3) {
      toast.show("Maksimal 3 skill untuk simulasi What-If sekaligus.", { tone: "info" });
      return;
    }
    simSkillIds.value = [...simSkillIds.value, skillId];
  }
  void runImpactSimulation();
}

watch(
  () => competencyStore.selectedRole,
  () => {
    simSkillIds.value = [];
    simResults.value = [];
  },
);

onMounted(async () => {
  if (!profileStore.activeProfile) {
    await profileStore.loadActiveProfile();
  }
  if (profileStore.userId) {
    await competencyStore.loadRolesAndMap(profileStore.userId);
  }
});

const subtitle = computed(() => {
  if (!competencyStore.mapData) return "Agregasi permintaan skill pasar lokal";
  return `${competencyStore.mapData.target_role} · ${competencyStore.mapData.total_jobs} lowongan · ${competencyStore.gapCount} gap`;
});

async function handleRoleChange(e: Event) {
  const val = (e.target as HTMLSelectElement).value;
  if (profileStore.userId && val) {
    await competencyStore.selectRole(profileStore.userId, val);
  }
}

async function handleQuickAdd(skillId: number, skillName: string) {
  if (!profileStore.userId) return;
  try {
    await ipc.quickAddUserSkill(profileStore.userId, skillId);
    await Promise.all([
      profileStore.loadActiveProfile(),
      competencyStore.loadRolesAndMap(profileStore.userId),
      jobsStore.loadJobs(profileStore.userId),
    ]);
    toast.show(`${skillName} ditambahkan ke profil. Skor lowongan diperbarui lokal (0 token).`, {
      tone: "success",
    });
  } catch (e) {
    toast.show(e instanceof IpcError ? e.message : "Gagal menambahkan skill.", { tone: "error" });
  }
}

async function handleGenerateRoadmap() {
  if (!profileStore.userId || !competencyStore.selectedRole) return;
  const ok = await roadmapStore.generateRoadmap(
    profileStore.userId,
    competencyStore.selectedRole,
    null,
    8,
  );
  if (ok) {
    toast.show(`Roadmap untuk ${competencyStore.selectedRole} selesai disusun.`, {
      tone: "success",
    });
    router.push("/roadmap");
  } else if (roadmapStore.error) {
    toast.show(roadmapStore.error, { tone: "error" });
  }
}
</script>

<template>
  <div class="v-comp-root">
    <PageHeader title="Peta Kompetensi Pasar" :subtitle="subtitle">
      <template #actions>
        <label v-if="competencyStore.availableRoles.length > 0" for="comp-role-select" class="sr-only">
          Pilih peran target
        </label>
        <select
          v-if="competencyStore.availableRoles.length > 0"
          id="comp-role-select"
          :value="competencyStore.selectedRole ?? ''"
          class="v-role-select"
          @change="handleRoleChange"
        >
          <option
            v-for="r in competencyStore.availableRoles"
            :key="r.normalized_role"
            :value="r.normalized_role"
          >
            {{ r.normalized_role }} ({{ r.job_count }})
          </option>
        </select>
        <VButton
          variant="primary"
          :disabled="competencyStore.gapCount === 0 || !competencyStore.selectedRole"
          :loading="roadmapStore.generating"
          @click="handleGenerateRoadmap"
        >
          {{ roadmapStore.generating ? "Menyusun..." : "Buat Roadmap Gap" }}
        </VButton>
      </template>
    </PageHeader>

    <div class="v-page-container">
      <div v-if="competencyStore.loading && !competencyStore.mapData" class="v-skeleton-stack" aria-busy="true" aria-label="Memuat peta kompetensi">
        <div v-for="n in 6" :key="n" class="v-ledger-row">
          <VSkeleton width="140px" height="16px" />
          <VSkeleton width="100%" height="6px" radius="var(--radius-full)" />
          <VSkeleton width="68px" height="16px" />
          <VSkeleton width="88px" height="32px" radius="var(--radius-control)" />
        </div>
      </div>

      <VCard v-else-if="!competencyStore.loading && !competencyStore.mapData">
        <h2 class="t-section">Belum Ada Data Permintaan Pasar</h2>
        <p class="t-body" style="margin-top: var(--space-2); max-width: 60ch">
          Ambil lowongan lewat halaman Lowongan atau tempel beberapa lowongan manual untuk melihat
          peta frekuensi skill yang paling dicari di pasar.
        </p>
        <div style="margin-top: var(--space-5)">
          <VButton variant="primary" @click="router.push('/jobs')">Buka Lowongan</VButton>
        </div>
      </VCard>

      <template v-else-if="competencyStore.mapData">
        <SalaryBenchmarkCard :role-filter="competencyStore.selectedRole" />

        <div v-if="competencyStore.mapData.is_small_sample" class="v-sample-note" role="status">
          <span class="t-secondary" style="color: var(--status-info)">
            Sampel masih kecil ({{ competencyStore.mapData.total_jobs }} lowongan). Tambah lebih
            banyak lowongan untuk peran ini agar distribusi pasar makin akurat.
          </span>
        </div>

        <div class="v-filter-bar">
          <div class="v-filter-tabs" role="group" aria-label="Filter kategori kompetensi">
            <button
              v-for="c in CLUSTERS"
              :key="c.id"
              type="button"
              class="v-filter-tab"
              :aria-pressed="competencyStore.categoryFilter === c.id"
              @click="competencyStore.categoryFilter = c.id"
            >
              {{ c.label }}
            </button>
          </div>
          <div style="padding-bottom: 8px">
            <VSwitch v-model="competencyStore.gapOnly" label="Hanya Gap" />
          </div>
        </div>

        <!-- What-If Skill Impact Simulator Summary Bar -->
        <div v-if="simSkillIds.length > 0" class="v-sim-bar liquid-glass" role="status">
          <div class="v-sim-bar__info">
            <div class="v-sim-head">
              <VIcon name="sparkle" :size="15" />
              <strong>Simulasi Dampak What-If (0 Token)</strong>
              <span v-if="simLoading" class="t-meta">Menghitung...</span>
            </div>
            <div v-if="!simLoading && simResults.length > 0" class="v-sim-metrics">
              <span v-for="item in simResults" :key="item.skill_id" class="v-sim-chip t-num">
                <strong>{{ item.canonical_name }}:</strong>
                +{{ item.avg_score_delta }}% skor · {{ item.affected_jobs_count }} lowongan
                <span v-if="item.jobs_crossing_75_count > 0">
                  ({{ item.jobs_crossing_75_count }} tembus >= 75%)
                </span>
              </span>
            </div>
          </div>
          <div class="v-sim-bar__actions">
            <VButton
              variant="tertiary"
              size="compact"
              @click="simSkillIds = []; simResults = []"
            >
              Reset
            </VButton>
            <VButton
              variant="primary"
              size="compact"
              :loading="roadmapStore.generating"
              @click="handleGenerateRoadmap"
            >
              Buat Roadmap
            </VButton>
          </div>
        </div>

        <div role="region" aria-label="Tabel permintaan kompetensi">
          <div class="v-ledger-head">
            <div>Kompetensi</div>
            <div>Frekuensi di Lowongan</div>
            <div style="text-align: right">Permintaan</div>
            <div style="text-align: right">Status Profil</div>
          </div>

          <div v-if="competencyStore.filteredSkills.length === 0" class="v-empty-rows">
            <p class="t-card">Tidak ada celah skill pada kategori ini.</p>
            <p class="t-meta" style="margin-top: var(--space-2)">
              Kamu sudah menguasai seluruh kompetensi teratas pada filter yang dipilih.
            </p>
            <VButton
              variant="secondary"
              size="compact"
              style="margin-top: var(--space-4)"
              @click="
                competencyStore.categoryFilter = 'all';
                competencyStore.gapOnly = false;
              "
            >
              Tampilkan Semua
            </VButton>
          </div>

          <div
            v-for="row in competencyStore.filteredSkills"
            :key="row.skill_id"
            class="v-ledger-row"
          >
            <div>
              <div style="font-weight: 600; font-size: 14px; color: var(--text-primary)">
                {{ row.canonical_name }}
              </div>
              <div class="t-meta" style="margin-top: 2px">
                {{ formatSkillCategory(row.category) }}
              </div>
            </div>

            <div class="v-ledger-bar-col">
              <div
                class="v-bar-track"
                role="progressbar"
                :aria-label="`Permintaan ${row.canonical_name}`"
                :aria-valuenow="formatScore(row.demand_percentage)"
                aria-valuemin="0"
                aria-valuemax="100"
              >
                <div
                  class="v-bar-fill"
                  :style="{
                    width: `${formatScore(row.demand_percentage)}%`,
                    background: row.is_owned_by_user
                      ? 'var(--status-success)'
                      : 'var(--status-warning)',
                  }"
                />
              </div>
            </div>

            <div class="t-num" style="text-align: right">
              <span style="font-size: 14px; font-weight: 600; color: var(--text-primary)">
                {{ formatScore(row.demand_percentage) }}%
              </span>
              <span class="t-meta" style="margin-left: 8px">
                {{ row.demand_count }}/{{ competencyStore.mapData.total_jobs }}
              </span>
            </div>

            <div style="display: flex; align-items: center; justify-content: flex-end; gap: 6px; flex-wrap: wrap">
              <span
                v-if="row.is_owned_by_user"
                style="display: inline-flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 500; color: var(--status-success)"
              >
                <VIcon name="check" :size="13" />
                <span>{{ formatProficiency(row.user_proficiency ?? "beginner") }}</span>
              </span>
              <template v-else>
                <button
                  type="button"
                  class="v-sim-pill"
                  :class="{ 'v-sim-pill--active': simSkillIds.includes(row.skill_id) }"
                  @click="toggleSimulateSkill(row.skill_id)"
                >
                  {{ simSkillIds.includes(row.skill_id) ? "Disimulasikan" : "Simulasi" }}
                </button>
                <VButton
                  variant="secondary"
                  size="compact"
                  :title="`Tambahkan ${row.canonical_name} ke profil (0 token)`"
                  @click="handleQuickAdd(row.skill_id, row.canonical_name)"
                >
                  <VIcon name="plus" :size="13" />
                  <span>Tambah</span>
                </VButton>
              </template>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.v-comp-root {
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
.v-role-select {
  height: 40px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-default);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: 13px;
}
.v-sample-note {
  margin-bottom: var(--space-5);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-control);
  background: var(--info-soft);
  border: 1px solid rgba(85, 166, 232, 0.25);
}
.v-filter-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  flex-wrap: wrap;
  border-bottom: 1px solid var(--border-divider);
  margin-bottom: var(--space-4);
}
.v-filter-tabs {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex-wrap: wrap;
}
.v-filter-tab {
  position: relative;
  height: 40px;
  padding: 0 var(--space-3);
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 500;
  border-radius: var(--radius-control);
}
.v-filter-tab:hover {
  color: var(--text-primary);
  background: var(--bg-elevated);
}
.v-filter-tab[aria-pressed="true"] {
  color: var(--text-primary);
  font-weight: 600;
}
.v-filter-tab[aria-pressed="true"]::after {
  content: "";
  position: absolute;
  left: 12px;
  right: 12px;
  bottom: -1px;
  height: 2px;
  border-radius: 2px;
  background: var(--brand-primary);
}
.v-ledger-head,
.v-ledger-row {
  display: grid;
  grid-template-columns: minmax(190px, 1.3fr) minmax(200px, 2.2fr) 116px 144px;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4) var(--space-3);
  border-bottom: 1px solid var(--border-divider);
}
.v-ledger-head {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  padding-bottom: var(--space-3);
  border-bottom-color: var(--border-default);
}
.v-ledger-row {
  border-radius: var(--radius-control);
  transition: background var(--dur-hover) var(--ease-state);
}
.v-ledger-row:hover {
  background: var(--bg-elevated);
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
  border-radius: var(--radius-full);
  transition: width 220ms var(--ease-out);
}
.v-empty-rows {
  padding: var(--space-12) var(--space-4);
  text-align: center;
  border-bottom: 1px solid var(--border-divider);
}
.v-sim-pill {
  height: 30px;
  padding: 0 10px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 12px;
  font-weight: 600;
  transition: background-color 140ms ease, border-color 140ms ease, color 140ms ease;
}
.v-sim-pill--active {
  background: var(--filter-active-bg);
  border-color: var(--filter-active-border);
  color: var(--filter-active-fg);
}
.v-sim-bar {
  margin-top: var(--space-5);
  padding: var(--space-4);
  border-radius: var(--radius-card);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  flex-wrap: wrap;
}
.v-sim-bar__info {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
  flex: 1;
}
.v-sim-head {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13.5px;
  color: var(--text-1);
}
.v-sim-metrics {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.v-sim-chip {
  font-size: 12px;
  padding: 4px 10px;
  border-radius: var(--radius-control);
  background: var(--success-soft);
  color: var(--status-success);
}
.v-sim-bar__actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
@media (max-width: 1023px) {
  .v-page-container {
    padding: var(--space-5) var(--space-4) var(--space-10);
  }
  .v-ledger-head {
    display: none;
  }
  .v-ledger-row {
    grid-template-columns: 1fr auto;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-1);
  }
  .v-ledger-bar-col {
    grid-column: 1 / -1;
  }
}
</style>
