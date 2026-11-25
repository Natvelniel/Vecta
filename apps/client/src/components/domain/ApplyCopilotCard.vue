<script setup lang="ts">
import { ref } from "vue";
import type { ApplyKitResult } from "@/lib/bindings";
import { useToast } from "@/composables/useToast";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";
import VIcon from "@/components/ui/VIcon.vue";

const props = defineProps<{
  kit: ApplyKitResult | null;
  loading: boolean;
}>();

const emit = defineEmits<{
  (e: "generate", language: "id" | "en", forceRefresh: boolean): void;
}>();

const toast = useToast();
const selectedLang = ref<"id" | "en">("id");

async function copyText(label: string, text: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast.show(`${label} disalin ke clipboard.`, { tone: "success" });
  } catch {
    toast.show(`Gagal menyalin otomatis. Blok teks ${label} secara manual.`, { tone: "warning" });
  }
}

function handleGenerate(forceRefresh: boolean) {
  emit("generate", selectedLang.value, forceRefresh);
}
</script>

<template>
  <VCard style="margin-top: var(--space-6)">
    <div class="v-cop-head">
      <div class="v-cop-title">
        <h3 class="t-card" style="font-size: 16px">Copilot Lamaran (CV &amp; Cover Note)</h3>
        <VBadge tone="ai">
          <VIcon name="aiNode" :size="12" />
          <span>{{ kit ? `AI · Cache (${kit.language.toUpperCase()})` : "AI · On-Demand" }}</span>
        </VBadge>
      </div>

      <div class="v-cop-controls">
        <div class="v-lang-toggle" role="group" aria-label="Bahasa bahan lamaran">
          <button
            type="button"
            class="v-lang-btn"
            :aria-pressed="selectedLang === 'id'"
            @click="selectedLang = 'id'"
          >
            ID
          </button>
          <button
            type="button"
            class="v-lang-btn"
            :aria-pressed="selectedLang === 'en'"
            @click="selectedLang = 'en'"
          >
            EN
          </button>
        </div>

        <VButton
          v-if="kit"
          variant="tertiary"
          size="compact"
          :loading="loading"
          @click="handleGenerate(true)"
        >
          Buat Ulang
        </VButton>
      </div>
    </div>

    <div v-if="!kit" class="v-cop-empty">
      <p class="t-secondary" style="max-width: 56ch">
        Susun headline CV, 3-5 poin pencapaian yang cocok dengan lowongan ini, serta draf singkat
        pesan lamaran siap salin. Hasil disimpan di cache SQLite lokal agar tidak boros kuota.
      </p>
      <VButton
        variant="ai"
        size="compact"
        :loading="loading"
        style="margin-top: var(--space-4)"
        @click="handleGenerate(false)"
      >
        <VIcon v-if="!loading" name="aiNode" :size="14" />
        <span>{{ loading ? "Menyusun bahan lamaran..." : "Buat Bahan Lamaran · AI" }}</span>
      </VButton>
    </div>

    <div v-else class="v-cop-body">
      <!-- Tailored Headline -->
      <div class="v-cop-sec">
        <div class="v-cop-sec-head">
          <span class="t-meta" style="color: var(--brand-hover); font-weight: 600">
            Headline Profil Ter-tailor
          </span>
          <button
            type="button"
            class="v-copy-link"
            @click="copyText('Headline profil', kit.tailored_headline)"
          >
            Salin
          </button>
        </div>
        <p class="t-body" style="color: var(--text-primary); font-weight: 500; margin-top: 4px">
          {{ kit.tailored_headline }}
        </p>
      </div>

      <!-- Tailored CV Bullets -->
      <div v-if="kit.cv_bullets.length > 0" class="v-cop-sec">
        <div class="v-cop-sec-head">
          <span class="t-meta" style="color: var(--status-success); font-weight: 600">
            Poin CV Siap Pakai
          </span>
          <button
            type="button"
            class="v-copy-link"
            @click="copyText('Poin CV', kit.cv_bullets.map((b) => `- ${b}`).join('\n'))"
          >
            Salin Semua
          </button>
        </div>
        <ul class="v-cop-list">
          <li v-for="(bullet, i) in kit.cv_bullets" :key="i" class="t-secondary">
            {{ bullet }}
          </li>
        </ul>
      </div>

      <!-- Cover Note -->
      <div class="v-cop-sec">
        <div class="v-cop-sec-head">
          <span class="t-meta" style="color: var(--text-primary); font-weight: 600">
            Draf Pesan Lamaran / Cover Note
          </span>
          <button
            type="button"
            class="v-copy-link"
            @click="copyText('Draf pesan lamaran', kit.cover_note)"
          >
            Salin Pesan
          </button>
        </div>
        <div class="v-cover-box t-secondary">
          {{ kit.cover_note }}
        </div>
      </div>

      <!-- Interview Angles -->
      <div v-if="kit.interview_angles.length > 0" class="v-cop-sec">
        <span class="t-meta" style="color: var(--status-warning); font-weight: 600">
          Prediksi Topik Wawancara
        </span>
        <ul class="v-cop-list">
          <li v-for="(angle, i) in kit.interview_angles" :key="i" class="t-secondary">
            {{ angle }}
          </li>
        </ul>
      </div>
    </div>
  </VCard>
</template>

<style scoped>
.v-cop-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-cop-title {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-cop-controls {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.v-lang-toggle {
  display: inline-flex;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-default);
  background: var(--bg-primary);
  overflow: hidden;
}
.v-lang-btn {
  height: 28px;
  padding: 0 10px;
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
}
.v-lang-btn[aria-pressed="true"] {
  background: var(--brand-soft);
  color: var(--text-primary);
}
.v-cop-empty {
  margin-top: var(--space-4);
}
.v-cop-body {
  margin-top: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.v-cop-sec {
  padding-top: var(--space-3);
  border-top: 1px solid var(--border-divider);
}
.v-cop-sec:first-child {
  padding-top: 0;
  border-top: 0;
}
.v-cop-sec-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}
.v-copy-link {
  border: 0;
  background: transparent;
  color: var(--brand-hover);
  font-size: 12px;
  font-weight: 500;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}
.v-copy-link:hover {
  background: var(--bg-hover);
}
.v-cop-list {
  margin: var(--space-2) 0 0;
  padding-left: 18px;
  list-style-type: disc;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.v-cover-box {
  margin-top: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-control);
  background: var(--bg-primary);
  border: 1px solid var(--border-divider);
  white-space: pre-wrap;
  line-height: 1.65;
  color: var(--text-primary);
}
</style>
