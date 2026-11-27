<script setup lang="ts">
import { computed } from "vue";
import { formatScore } from "@/lib/format";

const props = defineProps<{
  finalScore: number;
  vectorScore: number;
  skillScore: number;
  levelScore: number;
  domainScore?: number | null;
  rerankerScore?: number | null;
  matchExplanation?: string[];
}>();

const finalPct = computed(() => formatScore(props.finalScore));
const vecPct = computed(() => formatScore(props.vectorScore));
const skillPct = computed(() => formatScore(props.skillScore));
const levelPct = computed(() => formatScore(props.levelScore));
const domainPct = computed(() =>
  props.domainScore !== null && props.domainScore !== undefined ? formatScore(props.domainScore) : null,
);
const rerankerPct = computed(() =>
  props.rerankerScore !== null && props.rerankerScore !== undefined
    ? formatScore(props.rerankerScore)
    : null,
);
const explanationBullets = computed(() => (props.matchExplanation ?? []).slice(0, 3));
</script>

<template>
  <div class="v-match-panel">
    <div style="flex-shrink: 0">
      <div class="t-meta">Skor Kecocokan</div>
      <div class="t-display t-num" style="margin-top: 4px; color: var(--brand-hover)">
        {{ finalPct }}%
      </div>
      <div v-if="rerankerPct !== null" class="v-rerank-pill t-meta t-num">
        Neural Reranker {{ rerankerPct }}%
      </div>
    </div>

    <div class="v-match-panel__axes">
      <div class="v-match-panel__head">
        <span style="font-size: 13px; font-weight: 500; color: var(--text-primary)">
          Komposisi Matching Lokal
        </span>
        <span class="t-meta t-num">Hitung lokal &lt;100ms · 0 token</span>
      </div>

      <div
        class="v-meter-track"
        role="progressbar"
        aria-label="Skor kecocokan hibrida"
        :aria-valuenow="finalPct"
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <div
          class="v-meter-seg"
          :style="{ width: `${((domainPct !== null ? 0.36 : 0.4) * vecPct).toFixed(1)}%`, background: 'var(--brand-primary)' }"
        />
        <div
          class="v-meter-seg"
          :style="{ width: `${((domainPct !== null ? 0.36 : 0.45) * skillPct).toFixed(1)}%`, background: 'var(--status-success)' }"
        />
        <div
          class="v-meter-seg"
          :style="{ width: `${((domainPct !== null ? 0.14 : 0.15) * levelPct).toFixed(1)}%`, background: 'var(--status-info)' }"
        />
        <div
          v-if="domainPct !== null"
          class="v-meter-seg"
          :style="{ width: `${(0.14 * domainPct).toFixed(1)}%`, background: 'var(--status-warning)' }"
        />
      </div>

      <div class="t-meta t-num v-meter-legend">
        <span><span class="v-swatch" style="background: var(--brand-primary)" />Konteks {{ vecPct }}%</span>
        <span><span class="v-swatch" style="background: var(--status-success)" />Skill {{ skillPct }}%</span>
        <span><span class="v-swatch" style="background: var(--status-info)" />Level {{ levelPct }}%</span>
        <span v-if="domainPct !== null">
          <span class="v-swatch" style="background: var(--status-warning)" />Domain {{ domainPct }}%
        </span>
      </div>

      <ul v-if="explanationBullets.length > 0" class="v-match-expl">
        <li v-for="(item, idx) in explanationBullets" :key="idx" class="t-meta">
          {{ item }}
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.v-match-panel {
  margin-top: var(--space-5);
  padding: var(--space-5) var(--space-6);
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-card);
  display: flex;
  align-items: center;
  gap: var(--space-8);
}
.v-match-panel__axes {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.v-match-panel__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-meter-track {
  height: 6px;
  width: 100%;
  background: var(--bg-primary);
  border-radius: var(--radius-full);
  display: flex;
  gap: 3px;
  overflow: hidden;
}
.v-meter-seg {
  height: 100%;
  border-radius: var(--radius-full);
  transition: width var(--dur-meter) var(--ease-luxury);
}
.v-meter-legend {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-5);
}
.v-swatch {
  display: inline-block;
  width: 10px;
  height: 3px;
  border-radius: 2px;
  vertical-align: middle;
  margin-right: 6px;
}
@media (max-width: 1023px) {
  .v-match-panel {
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-5);
  }
}
.v-rerank-pill {
  display: inline-block;
  margin-top: 6px;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  border: 1px solid var(--border-default);
  color: var(--text-primary);
}
.v-match-expl {
  margin: 4px 0 0;
  padding-left: 16px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  color: var(--text-secondary);
}
</style>
