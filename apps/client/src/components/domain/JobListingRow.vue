<script setup lang="ts">
import { computed, ref } from "vue";
import type { NormalizedJobSummary } from "@/lib/ipc";
import {
  formatApplicationStatus,
  formatJobSource,
  formatRelativeDate,
  formatWorkMode,
} from "@/lib/format";
import ScoreRing from "@/components/ui/ScoreRing.vue";
import VIcon from "@/components/ui/VIcon.vue";

const props = defineProps<{
  job: NormalizedJobSummary;
  selected?: boolean;
}>();

const emit = defineEmits<{
  (e: "select", jobId: string): void;
  (e: "quick-save", jobId: string): void;
  (e: "quick-dismiss", jobId: string): void;
}>();

const swipeOffsetX = ref(0);
const swipeActive = ref(false);
let touchStartX = 0;
let touchStartY = 0;
let lockHorizontal = false;

function handleTouchStart(e: TouchEvent) {
  const t = e.touches[0];
  if (!t) return;
  touchStartX = t.clientX;
  touchStartY = t.clientY;
  lockHorizontal = false;
  swipeActive.value = true;
}

function handleTouchMove(e: TouchEvent) {
  if (!swipeActive.value) return;
  const t = e.touches[0];
  if (!t) return;
  const dx = t.clientX - touchStartX;
  const dy = t.clientY - touchStartY;
  if (!lockHorizontal) {
    if (Math.abs(dy) > Math.abs(dx) && Math.abs(dy) > 10) {
      swipeActive.value = false;
      swipeOffsetX.value = 0;
      return;
    }
    if (Math.abs(dx) > Math.abs(dy) * 1.5 && Math.abs(dx) > 12) {
      lockHorizontal = true;
    }
  }
  if (lockHorizontal) {
    swipeOffsetX.value = Math.max(-108, Math.min(108, dx));
  }
}

function handleTouchEnd() {
  if (!swipeActive.value) {
    swipeOffsetX.value = 0;
    return;
  }
  const dx = swipeOffsetX.value;
  swipeActive.value = false;
  swipeOffsetX.value = 0;
  lockHorizontal = false;
  if (dx > 72) {
    emit("quick-save", props.job.job_id);
  } else if (dx < -72) {
    emit("quick-dismiss", props.job.job_id);
  }
}

const salaryBadge = computed(() => {
  const min = props.job.salary_min_idr;
  const max = props.job.salary_max_idr;
  if (min && max && max > min) {
    return `Rp ${(min / 1_000_000).toFixed(min % 1_000_000 === 0 ? 0 : 1)}-${(max / 1_000_000).toFixed(max % 1_000_000 === 0 ? 0 : 1)} jt`;
  }
  if (min) {
    return `Rp ${(min / 1_000_000).toFixed(min % 1_000_000 === 0 ? 0 : 1)} jt+`;
  }
  return props.job.salary_text?.trim() || null;
});

const opportunityLabel = computed(() => {
  switch (props.job.opportunity_type) {
    case "mt_odp":
      return "MT / ODP";
    case "internship":
      return "Magang";
    case "contract":
      return "Kontrak";
    case "freelance":
      return "Freelance";
    default:
      return null;
  }
});

const freshnessTag = computed(() => {
  const ts = Date.parse(props.job.posted_at ?? props.job.scraped_at);
  if (Number.isNaN(ts)) return null;
  const ageHours = (Date.now() - ts) / 3_600_000;
  if (ageHours <= 48) return { label: "Baru", tone: "new" as const };
  if (ageHours >= 24 * 21) return { label: "Lama", tone: "stale" as const };
  return null;
});
</script>

<template>
  <div class="v-job-swipe-wrap">
    <div v-if="swipeOffsetX > 18" class="v-swipe-cue v-swipe-cue--save" aria-hidden="true">
      <VIcon name="bookmarkFill" :size="15" />
      <span>{{ job.is_saved ? "Hapus Simpan" : "Simpan" }}</span>
    </div>
    <div v-else-if="swipeOffsetX < -18" class="v-swipe-cue v-swipe-cue--dismiss" aria-hidden="true">
      <span>Sembunyikan</span>
      <VIcon name="close" :size="15" />
    </div>

    <div
      class="v-job-row"
      role="button"
      tabindex="0"
      :aria-selected="selected ? 'true' : 'false'"
      :style="
        swipeOffsetX !== 0
          ? { transform: `translate3d(${swipeOffsetX}px, 0, 0)`, transition: 'none' }
          : undefined
      "
      @click="emit('select', job.job_id)"
      @keydown.enter.prevent="emit('select', job.job_id)"
      @keydown.space.prevent="emit('select', job.job_id)"
      @touchstart.passive="handleTouchStart"
      @touchmove.passive="handleTouchMove"
      @touchend="handleTouchEnd"
    >
      <div class="v-job-row__top">
        <div class="v-job-row__titles">
          <div class="t-card v-job-row__title">{{ job.title }}</div>
          <div class="t-secondary v-job-row__sub">
            {{ job.company }} · {{ job.location || "Indonesia" }} · {{ formatWorkMode(job.work_mode) }}
          </div>
          <div v-if="salaryBadge || opportunityLabel || freshnessTag" class="v-job-row__badges">
            <span v-if="salaryBadge" class="v-pill-salary t-num">{{ salaryBadge }}</span>
            <span v-if="opportunityLabel" class="v-pill-opp">{{ opportunityLabel }}</span>
            <span
              v-if="freshnessTag"
              class="v-pill-fresh"
              :data-tone="freshnessTag.tone"
            >
              {{ freshnessTag.label }}
            </span>
          </div>
        </div>
        <div class="v-job-row__right">
          <div class="v-row-quick-actions">
            <button
              type="button"
              class="v-row-qbtn"
              :aria-label="job.is_saved ? 'Batal simpan lowongan' : 'Simpan lowongan'"
              :title="job.is_saved ? 'Tersimpan' : 'Simpan'"
              @click.stop="emit('quick-save', job.job_id)"
            >
              <VIcon :name="job.is_saved ? 'bookmarkFill' : 'bookmark'" :size="14" />
            </button>
            <button
              type="button"
              class="v-row-qbtn v-row-qbtn--dim"
              aria-label="Sembunyikan lowongan"
              title="Sembunyikan"
              @click.stop="emit('quick-dismiss', job.job_id)"
            >
              <VIcon name="close" :size="14" />
            </button>
          </div>
          <ScoreRing :score="job.final_score" :size="44" />
        </div>
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
            class="v-trust-tag"
          >
            Terverifikasi · {{ job.also_available_on.length + 1 }} Portal
          </span>
          <span
            v-if="job.red_flags_count > 0"
            class="v-warn-tag"
          >
            Waspada ({{ job.red_flags_count }})
          </span>
        </span>
        <span class="t-meta t-num v-job-row__counts">
          <span style="color: var(--text-1); font-weight: 600">{{ job.matched_skills_count }} cocok</span>
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
            class="v-job-row__saved"
            aria-label="Tersimpan"
          >
            <VIcon name="bookmarkFill" :size="13" />
          </span>
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.v-job-swipe-wrap {
  position: relative;
  overflow: hidden;
  border-radius: var(--radius-card);
}
.v-swipe-cue {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 var(--space-5);
  font-size: 12px;
  font-weight: 700;
  pointer-events: none;
}
.v-swipe-cue--save {
  justify-content: flex-start;
  background: var(--filter-active-tint);
  color: var(--status-success);
}
.v-swipe-cue--dismiss {
  justify-content: flex-end;
  background: var(--warning-soft);
  color: var(--status-warning);
}
.v-job-row {
  position: relative;
  width: 100%;
  display: block;
  text-align: left;
  padding: var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid transparent;
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
  cursor: pointer;
  touch-action: pan-y;
  transition:
    background-color var(--dur-hover) var(--ease-state),
    border-color var(--dur-hover) var(--ease-state),
    box-shadow var(--dur-hover) var(--ease-state),
    transform var(--dur-press) ease-out;
}
@media (hover: hover) {
  .v-job-row:hover {
    box-shadow: var(--shadow-surface-hover);
  }
}
.v-job-row:active {
  transform: scale(0.985);
}
.v-job-row[aria-selected="true"] {
  background: var(--surface-1);
  border-color: var(--brand);
  box-shadow: var(--shadow-surface-hover);
}

.v-job-row__top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
}
.v-job-row__right {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.v-row-quick-actions {
  display: none;
  align-items: center;
  gap: 4px;
}
@media (hover: hover) and (min-width: 1024px) {
  .v-job-row:hover .v-row-quick-actions,
  .v-job-row:focus-within .v-row-quick-actions {
    display: inline-flex;
  }
}
.v-row-qbtn {
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-2);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
@media (hover: hover) {
  .v-row-qbtn:hover {
    border-color: var(--brand-border);
    color: var(--text-1);
  }
  .v-row-qbtn--dim:hover {
    border-color: var(--warning-border);
    color: var(--status-warning);
  }
}
.v-job-row__titles {
  min-width: 0;
  flex: 1;
}
.v-job-row__title {
  font-size: 15px;
  line-height: 1.3;
  color: var(--text-1);
}
.v-job-row__sub {
  margin-top: 4px;
}
.v-job-row__badges {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.v-pill-salary,
.v-pill-opp,
.v-pill-fresh,
.v-mode-tag,
.v-app-tag,
.v-trust-tag,
.v-warn-tag {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 7px;
  border-radius: var(--radius-sm);
  line-height: 1.35;
}

.v-pill-salary {
  background: var(--surface-2);
  color: var(--text-1);
  border: 1px solid var(--border);
}
.v-pill-opp {
  background: var(--surface-2);
  color: var(--text-2);
  border: 1px solid var(--border);
}
.v-pill-fresh[data-tone="new"] {
  background: var(--surface-2);
  color: var(--text-1);
  border: 1px solid var(--border);
}
.v-pill-fresh[data-tone="stale"] {
  background: var(--surface-2);
  color: var(--text-3);
}

.v-job-row__foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  flex-wrap: wrap;
  margin-top: var(--space-3);
}
.v-job-row__meta {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.v-mode-tag {
  background: var(--surface-2);
  color: var(--text-2);
}
.v-mode-tag[data-ai="true"] {
  background: var(--surface-2);
  color: var(--text-1);
  border: 1px solid var(--border);
}
.v-app-tag {
  background: var(--brand-soft);
  color: var(--text-1);
}
.v-trust-tag {
  background: var(--surface-2);
  color: var(--text-1);
  border: 1px solid var(--border);
}
.v-warn-tag {
  background: var(--warning-soft);
  color: var(--status-warning);
}
.v-job-row__counts {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
}
.v-job-row__saved {
  color: var(--text-1);
  display: inline-flex;
}
</style>
