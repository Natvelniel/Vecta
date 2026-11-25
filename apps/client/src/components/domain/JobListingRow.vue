<script setup lang="ts">
import type { NormalizedJobSummary } from "@/lib/ipc";
import {
  formatApplicationStatus,
  formatJobSource,
  formatRelativeDate,
  formatScore,
  formatWorkMode,
} from "@/lib/format";
import VIcon from "@/components/ui/VIcon.vue";

defineProps<{
  job: NormalizedJobSummary;
  selected?: boolean;
}>();

const emit = defineEmits<{
  (e: "select", jobId: string): void;
}>();
</script>

<template>
  <button
    type="button"
    class="v-job-row"
    :aria-selected="selected ? 'true' : 'false'"
    @click="emit('select', job.job_id)"
  >
    <div class="v-job-row__top">
      <div class="v-job-row__titles">
        <div class="t-card" style="font-size: 15px">{{ job.title }}</div>
        <div class="t-secondary" style="margin-top: 4px">
          {{ job.company }} · {{ job.location || "Indonesia" }} · {{ formatWorkMode(job.work_mode) }}
        </div>
      </div>
      <span
        class="t-num v-job-row__score"
        :style="{
          color:
            formatScore(job.final_score) >= 75
              ? 'var(--brand-hover)'
              : 'var(--text-secondary)',
        }"
      >
        {{ formatScore(job.final_score) }}%
      </span>
    </div>

    <div class="v-job-row__foot">
      <span class="t-meta v-job-row__meta">
        <span>{{ formatJobSource(job.source) }}</span>
        <span>·</span>
        <span>{{ formatRelativeDate(job.scraped_at) }}</span>
        <span
          class="v-mode-tag"
          :data-ai="job.enrichment_status === 'ai_enriched'"
        >
          {{ job.enrichment_status === "ai_enriched" ? "AI" : "Lokal" }}
        </span>
        <span
          v-if="job.application_status !== 'new' && job.application_status !== 'saved'"
          class="v-app-tag"
        >
          {{ formatApplicationStatus(job.application_status) }}
        </span>
        <span
          v-if="job.also_available_on.length > 0"
          class="v-mode-tag"
        >
          +{{ job.also_available_on.length }} portal
        </span>
        <span
          v-if="job.red_flags_count > 0"
          class="v-warn-tag"
        >
          Waspada ({{ job.red_flags_count }})
        </span>
      </span>
      <span class="t-meta t-num v-job-row__counts">
        <span style="color: var(--status-success)">{{ job.matched_skills_count }} cocok</span>
        <span
          :style="{
            color:
              job.missing_skills_count > 0
                ? 'var(--status-warning)'
                : 'var(--text-secondary)',
          }"
        >
          {{ job.missing_skills_count }} gap
        </span>
        <span
          v-if="job.is_saved"
          style="color: var(--brand-hover); display: inline-flex"
          aria-label="Tersimpan"
        >
          <VIcon name="bookmarkFill" :size="13" />
        </span>
      </span>
    </div>
  </button>
</template>

<style scoped>
.v-job-row {
  width: 100%;
  display: block;
  text-align: left;
  padding: var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid transparent;
  background: transparent;
  transition:
    background var(--dur-hover) var(--ease-state),
    border-color var(--dur-hover) var(--ease-state);
}
.v-job-row:hover {
  background: var(--bg-elevated);
}
.v-job-row[aria-selected="true"] {
  background: var(--bg-elevated);
  border-color: var(--brand-border);
}
.v-job-row__top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
}
.v-job-row__titles {
  min-width: 0;
}
.v-job-row__score {
  font-size: 14px;
  font-weight: 700;
  flex-shrink: 0;
}
.v-job-row__foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  margin-top: var(--space-3);
}
.v-job-row__counts {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}
.v-job-row__meta {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.v-mode-tag {
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 500;
  background: var(--bg-hover);
  color: var(--text-secondary);
}
.v-mode-tag[data-ai="true"] {
  background: var(--ai-soft);
  color: var(--ai-accent);
}
.v-app-tag {
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  background: var(--brand-soft);
  color: var(--text-primary);
}
.v-warn-tag {
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  background: var(--warning-soft);
  color: var(--status-warning);
}
</style>
