<script setup lang="ts">
import { ref, watch } from "vue";
import type { InterviewPrepKit } from "@/lib/bindings";
import { useToast } from "@/composables/useToast";
import { ipc } from "@/lib/ipc";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";

const props = defineProps<{
  userId: string;
  jobId: string;
}>();

const toast = useToast();
const expanded = ref(false);
const lang = ref<"id" | "en">("id");
const loading = ref(false);
const prep = ref<InterviewPrepKit | null>(null);
const openHints = ref<Record<number, boolean>>({});

async function loadPrep() {
  if (!props.userId || !props.jobId) return;
  loading.value = true;
  try {
    prep.value = await ipc.generateInterviewPrep(props.userId, props.jobId, lang.value);
    openHints.value = { 0: true };
  } catch {
    prep.value = null;
  } finally {
    loading.value = false;
  }
}

watch(
  () => props.jobId,
  () => {
    prep.value = null;
    if (expanded.value) void loadPrep();
  },
);

function toggleOpen() {
  expanded.value = !expanded.value;
  if (expanded.value && !prep.value) {
    void loadPrep();
  }
}

function switchLang(next: "id" | "en") {
  if (lang.value === next) return;
  lang.value = next;
  if (expanded.value) void loadPrep();
}

async function copyPitch() {
  if (!prep.value) return;
  try {
    await navigator.clipboard.writeText(prep.value.pitch_30s);
    toast.show("Elevator pitch 30 detik disalin.", { tone: "success" });
  } catch {
    toast.show("Gagal menyalin teks.", { tone: "warning" });
  }
}
</script>

<template>
  <VCard style="margin-top: var(--space-5)">
    <div class="v-ip-head">
      <div>
        <h3 class="t-card">Persiapan Interview STAR (0 Token)</h3>
        <p class="t-meta" style="margin-top: 2px">
          6 pertanyaan terarah + panduan kerangka STAR lokal tanpa kuota AI.
        </p>
      </div>
      <div class="v-ip-actions">
        <div class="v-lang-group" role="group" aria-label="Bahasa simulasi interview">
          <button
            type="button"
            class="v-lang-btn"
            :aria-pressed="lang === 'id'"
            @click="switchLang('id')"
          >
            ID
          </button>
          <button
            type="button"
            class="v-lang-btn"
            :aria-pressed="lang === 'en'"
            @click="switchLang('en')"
          >
            EN
          </button>
        </div>
        <VButton variant="secondary" size="compact" :loading="loading" @click="toggleOpen">
          {{ expanded ? "Sembunyikan" : "Latih Sekarang" }}
        </VButton>
      </div>
    </div>

    <div v-if="expanded && prep" class="v-ip-body">
      <div class="v-ip-pitch">
        <div class="v-ip-pitch-head">
          <strong style="font-size: 12.5px; color: var(--text-1)">Elevator Pitch 30 Detik</strong>
          <VButton variant="tertiary" size="compact" @click="copyPitch">Salin Pitch</VButton>
        </div>
        <p class="t-secondary" style="margin-top: 4px">{{ prep.pitch_30s }}</p>
      </div>

      <div class="v-ip-q-list">
        <div v-for="(q, idx) in prep.questions" :key="idx" class="v-ip-q-item">
          <button
            type="button"
            class="v-ip-q-btn"
            :aria-expanded="Boolean(openHints[idx])"
            @click="openHints[idx] = !openHints[idx]"
          >
            <div style="text-align: left">
              <span class="v-ip-cat">{{ q.category }}</span>
              <span v-if="q.focus_skill" class="v-ip-skill">{{ q.focus_skill }}</span>
              <div class="v-ip-q-text">{{ idx + 1 }}. {{ q.question }}</div>
            </div>
          </button>
          <p v-if="openHints[idx]" class="v-ip-hint">{{ q.star_hint }}</p>
        </div>
      </div>

      <div v-if="prep.closing_questions.length > 0" class="v-ip-closing">
        <strong style="font-size: 12.5px; color: var(--text-1)">Pertanyaan Cerdas untuk Pewawancara</strong>
        <ul class="v-ip-ul">
          <li v-for="cq in prep.closing_questions" :key="cq">{{ cq }}</li>
        </ul>
      </div>
    </div>
  </VCard>
</template>

<style scoped>
.v-ip-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-ip-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.v-lang-group {
  display: inline-flex;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  overflow: hidden;
  background: var(--bg-primary);
}
.v-lang-btn {
  height: 32px;
  padding: 0 10px;
  border: 0;
  background: transparent;
  color: var(--text-2);
  font-size: 12px;
  font-weight: 600;
}
.v-lang-btn[aria-pressed="true"] {
  background: var(--brand-soft);
  color: var(--text-1);
}
.v-ip-body {
  margin-top: var(--space-4);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-divider);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.v-ip-pitch {
  padding: var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--brand-border);
  background: var(--brand-soft);
}
.v-ip-pitch-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.v-ip-q-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.v-ip-q-item {
  border-radius: var(--radius-control);
  border: 1px solid var(--border-divider);
  background: var(--bg-primary);
  overflow: hidden;
}
.v-ip-q-btn {
  width: 100%;
  padding: var(--space-3);
  border: 0;
  background: transparent;
  color: var(--text-1);
}
.v-ip-cat {
  font-size: 11px;
  font-weight: 700;
  color: var(--brand-hover);
  margin-right: 6px;
}
.v-ip-skill {
  font-size: 11px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  background: var(--success-soft);
  color: var(--status-success);
}
.v-ip-q-text {
  margin-top: 4px;
  font-size: 13.5px;
  font-weight: 600;
  line-height: 1.45;
}
.v-ip-hint {
  margin: 0;
  padding: 0 var(--space-3) var(--space-3);
  font-size: 12.5px;
  line-height: 1.55;
  color: var(--text-2);
}
.v-ip-ul {
  margin: 6px 0 0;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12.5px;
  color: var(--text-2);
}
</style>
