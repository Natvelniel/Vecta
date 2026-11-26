<script setup lang="ts">
import { ref, watch } from "vue";
import type { CompanySnapshot } from "@/lib/bindings";
import { ipc } from "@/lib/ipc";
import VButton from "@/components/ui/VButton.vue";
import VIcon from "@/components/ui/VIcon.vue";

const props = defineProps<{
  open: boolean;
  userId: string;
  companyName: string;
}>();

const emit = defineEmits<{
  (e: "close"): void;
}>();

const loading = ref(false);
const snapshot = ref<CompanySnapshot | null>(null);

watch(
  () => [props.open, props.companyName] as const,
  async ([isOpen, name]) => {
    if (!isOpen || !props.userId || !name.trim()) return;
    loading.value = true;
    try {
      snapshot.value = await ipc.getCompanySnapshot(props.userId, name);
    } catch {
      snapshot.value = null;
    } finally {
      loading.value = false;
    }
  },
  { immediate: true },
);

function formatSalaryMillion(val: number | null | undefined): string {
  if (!val || val <= 0) return "-";
  const jt = val / 1_000_000;
  return `Rp ${jt % 1 === 0 ? jt.toFixed(0) : jt.toFixed(1).replace(".", ",")} jt`;
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="v-cs-backdrop" @click.self="emit('close')">
      <div class="v-cs-modal liquid-glass" role="dialog" aria-modal="true" :aria-label="`Profil perusahaan ${companyName}`">
        <div class="v-cs-head">
          <div>
            <span class="t-meta">Ringkasan Perusahaan (Lokal)</span>
            <h2 class="t-section">{{ companyName }}</h2>
          </div>
          <button type="button" class="v-cs-close" aria-label="Tutup modal perusahaan" @click="emit('close')">
            <VIcon name="close" :size="16" />
          </button>
        </div>

        <p v-if="loading" class="t-meta" style="padding: var(--space-6) 0">
          Mengumpulkan data lowongan perusahaan...
        </p>

        <div v-else-if="snapshot" class="v-cs-body">
          <div class="v-cs-stats">
            <div class="v-cs-stat">
              <span class="t-meta">Lowongan</span>
              <strong class="t-num">{{ snapshot.total_jobs }}</strong>
            </div>
            <div class="v-cs-stat">
              <span class="t-meta">Rata Kecocokan</span>
              <strong class="t-num" style="color: var(--status-success)">{{ snapshot.avg_match_score }}%</strong>
            </div>
            <div class="v-cs-stat">
              <span class="t-meta">Waspada Red-Flag</span>
              <strong
                class="t-num"
                :style="{ color: snapshot.high_risk_count > 0 ? 'var(--status-warning)' : 'var(--status-success)' }"
              >
                {{ snapshot.high_risk_count }}
              </strong>
            </div>
          </div>

          <div class="v-cs-row t-meta">
            <span>Remote: <strong class="t-num">{{ snapshot.remote_count }}</strong></span>
            <span>Hybrid: <strong class="t-num">{{ snapshot.hybrid_count }}</strong></span>
            <span>Onsite: <strong class="t-num">{{ snapshot.onsite_count }}</strong></span>
            <span v-if="snapshot.salary_min_idr || snapshot.salary_max_idr">
              Gaji:
              <strong class="t-num" style="color: var(--status-success)">
                {{ formatSalaryMillion(snapshot.salary_min_idr) }} - {{ formatSalaryMillion(snapshot.salary_max_idr) }}
              </strong>
            </span>
          </div>

          <div v-if="snapshot.top_skills.length > 0">
            <h3 class="t-card" style="margin-bottom: 8px">Skill yang Paling Sering Dicari</h3>
            <div class="v-cs-skills">
              <span
                v-for="sk in snapshot.top_skills"
                :key="sk.skill_name"
                class="v-cs-skill"
                :class="{ 'v-cs-skill--owned': sk.user_has }"
              >
                {{ sk.skill_name }} ({{ sk.count }})
              </span>
            </div>
          </div>

          <div v-if="snapshot.recent_titles.length > 0">
            <h3 class="t-card" style="margin-bottom: 6px">Posisi Terkini</h3>
            <ul class="v-cs-titles">
              <li v-for="title in snapshot.recent_titles" :key="title" class="t-secondary">{{ title }}</li>
            </ul>
          </div>
        </div>

        <div class="v-cs-foot">
          <VButton variant="primary" size="compact" @click="emit('close')">Tutup</VButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.v-cs-backdrop {
  position: fixed;
  inset: 0;
  z-index: 80;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(5px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-4);
}
.v-cs-modal {
  width: 100%;
  max-width: 520px;
  max-height: 84vh;
  overflow-y: auto;
  border-radius: var(--radius-dialog);
  background: var(--surface-1);
  border: 1px solid var(--glass-border);
  box-shadow: var(--shadow-overlay);
  padding: var(--space-5);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.v-cs-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
}
.v-cs-close {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-1);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v-cs-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.v-cs-stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-3);
}
.v-cs-stat {
  padding: var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.v-cs-stat strong {
  font-size: 18px;
}
.v-cs-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}
.v-cs-skills {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.v-cs-skill {
  font-size: 12px;
  font-weight: 600;
  padding: 4px 9px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-2);
}
.v-cs-skill--owned {
  background: var(--filter-active-tint);
  border-color: var(--filter-active-border);
  color: var(--status-success);
}
.v-cs-titles {
  margin: 0;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.v-cs-foot {
  display: flex;
  justify-content: flex-end;
}
</style>
