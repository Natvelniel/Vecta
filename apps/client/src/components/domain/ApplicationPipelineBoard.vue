<script setup lang="ts">
import { computed, ref } from "vue";
import type { ApplicationStatus } from "@/lib/bindings";
import type { NormalizedJobSummary } from "@/lib/ipc";
import { useToast } from "@/composables/useToast";
import { formatRelativeDate, formatWorkMode } from "@/lib/format";
import ScoreRing from "@/components/ui/ScoreRing.vue";
import VButton from "@/components/ui/VButton.vue";
import VIcon from "@/components/ui/VIcon.vue";
import VSelect from "@/components/ui/VSelect.vue";

const props = defineProps<{
  jobs: NormalizedJobSummary[];
  selectedJobId: string | null;
  userName: string;
}>();

const emit = defineEmits<{
  (e: "select", jobId: string): void;
  (e: "update-status", payload: { jobId: string; status: ApplicationStatus; notes: string | null }): void;
  (e: "open-detail", jobId: string): void;
}>();

const toast = useToast();
const followUpLang = ref<"id" | "en">("id");
const activeStage = ref<ApplicationStatus>("applied");
const editingNoteId = ref<string | null>(null);
const noteText = ref("");

const STAGES: Array<{ id: ApplicationStatus; label: string }> = [
  { id: "saved", label: "Disimpan" },
  { id: "applied", label: "Dilamar" },
  { id: "interview", label: "Interview" },
  { id: "offer", label: "Offer" },
  { id: "rejected", label: "Ditolak" },
];

const STATUS_OPTIONS = STAGES.map((s) => ({ value: s.id, label: s.label }));

const jobsByStage = computed(() => {
  const map: Record<string, NormalizedJobSummary[]> = {
    saved: [],
    applied: [],
    interview: [],
    offer: [],
    rejected: [],
  };
  for (const job of props.jobs) {
    if (job.application_status === "new" && job.is_saved) {
      map.saved.push(job);
    } else if (map[job.application_status]) {
      map[job.application_status].push(job);
    }
  }
  return map;
});

const activeStageJobs = computed(() => jobsByStage.value[activeStage.value] ?? []);

function isFollowUpOverdue(job: NormalizedJobSummary): boolean {
  if (job.application_status !== "applied") return false;
  const ts = Date.parse(job.scraped_at);
  if (Number.isNaN(ts)) return false;
  return Date.now() - ts >= 7 * 24 * 3600_000;
}

function buildFollowUpText(job: NormalizedJobSummary): string {
  const name = props.userName.trim() || "Kandidat";
  if (followUpLang.value === "en") {
    return `Dear Hiring Team at ${job.company},\n\nI hope you are having a great week. I am writing to kindly follow up on my application for the ${job.title} position. I remain very enthusiastic about the opportunity to contribute my skills to ${job.company} and would be grateful for any update regarding the selection timeline.\n\nThank you for your time and consideration.\n\nBest regards,\n${name}`;
  }
  return `Yth. Tim Rekrutmen ${job.company},\n\nSemoga Bapak/Ibu dalam keadaan baik. Perkenalkan, saya ${name} yang telah mengajukan lamaran untuk posisi ${job.title}. Melalui pesan ini, saya bermaksud menanyakan perkembangan proses seleksi untuk posisi tersebut. Saya sangat antusias untuk dapat berkontribusi di ${job.company}.\n\nTerima kasih atas waktu dan perhatian Bapak/Ibu.\n\nHormat saya,\n${name}`;
}

async function copyFollowUp(job: NormalizedJobSummary) {
  const text = buildFollowUpText(job);
  try {
    await navigator.clipboard.writeText(text);
    toast.show(
      `Template follow-up (${followUpLang.value.toUpperCase()}) untuk ${job.company} disalin.`,
      { tone: "success" },
    );
  } catch {
    toast.show(`Salin manual teks follow-up untuk ${job.company}.`, { tone: "info" });
  }
}

function openNoteEditor(job: NormalizedJobSummary) {
  if (editingNoteId.value === job.job_id) {
    editingNoteId.value = null;
    return;
  }
  editingNoteId.value = job.job_id;
  noteText.value = "";
}

function saveInlineNote(job: NormalizedJobSummary) {
  emit("update-status", {
    jobId: job.job_id,
    status: job.application_status,
    notes: noteText.value.trim() || null,
  });
  editingNoteId.value = null;
}
</script>

<template>
  <div class="v-pipe-root">
    <div class="v-pipe-topbar">
      <div class="v-pipe-stages" role="tablist" aria-label="Tahapan lamaran">
        <button
          v-for="stage in STAGES"
          :key="stage.id"
          type="button"
          role="tab"
          class="v-pipe-tab"
          :class="{ 'v-pipe-tab--active': activeStage === stage.id }"
          :aria-selected="activeStage === stage.id"
          @click="activeStage = stage.id"
        >
          <span>{{ stage.label }}</span>
          <span class="v-pipe-count t-num">{{ jobsByStage[stage.id]?.length ?? 0 }}</span>
        </button>
      </div>

      <div class="v-pipe-lang" role="group" aria-label="Bahasa template follow-up">
        <span
          class="v-pipe-lang__thumb"
          :style="{ transform: followUpLang === 'en' ? 'translateX(100%)' : 'translateX(0)' }"
          aria-hidden="true"
        />
        <button
          type="button"
          class="v-lang-btn"
          :aria-pressed="followUpLang === 'id'"
          @click="followUpLang = 'id'"
        >
          Follow-up ID
        </button>
        <button
          type="button"
          class="v-lang-btn"
          :aria-pressed="followUpLang === 'en'"
          @click="followUpLang = 'en'"
        >
          Follow-up EN
        </button>
      </div>
    </div>

    <div v-if="activeStageJobs.length === 0" class="v-pipe-empty">
      <p class="t-card">Belum ada lowongan pada tahap {{ STAGES.find((s) => s.id === activeStage)?.label }}.</p>
      <p class="t-meta" style="margin-top: 6px">
        Ubah status lamaran dari tab Eksplorasi atau simpan lowongan terlebih dahulu untuk melacak progresmu.
      </p>
    </div>

    <div v-else class="v-pipe-grid">
      <article
        v-for="job in activeStageJobs"
        :key="job.job_id"
        class="v-pipe-card"
        :class="{ 'v-pipe-card--selected': selectedJobId === job.job_id }"
      >
        <div class="v-pipe-card__head" @click="emit('select', job.job_id)">
          <div style="min-width: 0; flex: 1">
            <h3 class="t-card v-pipe-title">{{ job.title }}</h3>
            <p class="t-secondary" style="margin-top: 2px">
              {{ job.company }} · {{ job.location || "Indonesia" }} · {{ formatWorkMode(job.work_mode) }}
            </p>
            <div class="v-pipe-meta">
              <span class="t-meta">{{ formatRelativeDate(job.scraped_at) }}</span>
              <span v-if="isFollowUpOverdue(job)" class="v-pipe-overdue">
                Perlu Follow-up (&ge; 7 hari)
              </span>
            </div>
          </div>
          <ScoreRing :score="job.final_score" :size="40" />
        </div>

        <div class="v-pipe-card__controls">
          <label :for="`pipe-st-${job.job_id}`" class="sr-only">Status lamaran</label>
          <VSelect
            :id="`pipe-st-${job.job_id}`"
            :model-value="job.application_status === 'new' && job.is_saved ? 'saved' : job.application_status"
            :options="STATUS_OPTIONS"
            compact
            :full-width="false"
            aria-label="Status lamaran"
            @update:model-value="
              emit('update-status', {
                jobId: job.job_id,
                status: ($event as ApplicationStatus),
                notes: null,
              })
            "
          />

          <VButton variant="secondary" size="compact" @click="copyFollowUp(job)">
            Salin Follow-up ({{ followUpLang.toUpperCase() }})
          </VButton>
          <VButton variant="tertiary" size="compact" @click="openNoteEditor(job)">
            Catatan
          </VButton>
          <VButton variant="tertiary" size="compact" @click="emit('open-detail', job.job_id)">
            <span>Detail</span>
            <VIcon name="chevronRight" :size="13" />
          </VButton>
        </div>

        <form
          v-if="editingNoteId === job.job_id"
          class="v-pipe-note-form"
          @submit.prevent="saveInlineNote(job)"
        >
          <input
            v-model="noteText"
            type="text"
            class="v-pipe-note-input"
            placeholder="Catat nama HR, tanggal interview, atau link submit..."
          />
          <VButton type="submit" variant="primary" size="compact">Simpan</VButton>
        </form>
      </article>
    </div>
  </div>
</template>

<style scoped>
.v-pipe-root {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-4) var(--space-6) var(--space-8);
  flex: 1;
  overflow-y: auto;
  overscroll-behavior: contain;
}
.v-pipe-topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-pipe-stages {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow-x: auto;
  scrollbar-width: none;
}
.v-pipe-stages::-webkit-scrollbar {
  display: none;
}
.v-pipe-tab {
  height: 36px;
  padding: 0 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 13px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  white-space: nowrap;
}
.v-pipe-tab--active {
  background: var(--brand);
  border-color: var(--brand);
  color: var(--brand-fg);
}
.v-pipe-count {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: var(--radius-full);
  background: rgba(255, 255, 255, 0.16);
}
.v-pipe-lang {
  position: relative;
  display: inline-grid;
  grid-template-columns: 1fr 1fr;
  padding: 3px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background: var(--surface-recessed);
  box-shadow: var(--shadow-recessed);
  overflow: hidden;
  flex-shrink: 0;
}
.v-pipe-lang__thumb {
  position: absolute;
  top: 3px;
  bottom: 3px;
  left: 3px;
  width: calc(50% - 3px);
  border-radius: calc(var(--radius-control) - 2px);
  background: var(--surface-1);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-control);
  transition: transform 360ms var(--ease-glide);
  pointer-events: none;
}
.v-lang-btn {
  position: relative;
  z-index: 1;
  height: 28px;
  padding: 0 10px;
  border: 0;
  background: transparent;
  color: var(--text-2);
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
  transition: color 220ms ease;
}
.v-lang-btn[aria-pressed="true"] {
  color: var(--text-1);
}
.v-pipe-empty {
  padding: var(--space-10) var(--space-5);
  border-radius: var(--radius-card);
  border: 1px solid var(--border-divider);
  background: var(--surface-1);
  text-align: center;
}
.v-pipe-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(310px, 1fr));
  gap: var(--space-4);
}
.v-pipe-card {
  padding: var(--space-4);
  border-radius: var(--radius-card);
  border: 1px solid var(--border);
  background: var(--surface-1);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.v-pipe-card--selected {
  border-color: var(--brand-border);
}
.v-pipe-card__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  cursor: pointer;
}
.v-pipe-title {
  font-size: 15px;
  color: var(--text-1);
}
.v-pipe-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 6px;
}
.v-pipe-overdue {
  font-size: 11px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: var(--warning-soft);
  color: var(--status-warning);
}
.v-pipe-card__controls {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  padding-top: var(--space-2);
  border-top: 1px solid var(--border-divider);
}
.v-pipe-select {
  height: 32px;
  padding: 0 22px 0 8px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--bg-primary);
  color: var(--text-1);
  font-size: 12px;
  font-weight: 600;
}
.v-pipe-note-form {
  display: flex;
  gap: 6px;
}
.v-pipe-note-input {
  flex: 1;
  min-width: 0;
  height: 34px;
  padding: 0 10px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--bg-primary);
  color: var(--text-1);
  font-size: 12.5px;
}
@media (max-width: 1023px) {
  .v-pipe-root {
    padding: var(--space-3) var(--space-3) var(--space-8);
  }
  .v-pipe-grid {
    grid-template-columns: 1fr;
  }
}
</style>
