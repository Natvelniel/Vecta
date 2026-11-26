<script setup lang="ts">
import type { GapAnalysis } from "@/lib/bindings";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";
import VIcon from "@/components/ui/VIcon.vue";

defineProps<{
  analysis: GapAnalysis | null;
  loading?: boolean;
  generatingRoadmap?: boolean;
}>();

const emit = defineEmits<{
  (e: "analyze", forceRefresh: boolean): void;
  (e: "create-roadmap"): void;
}>();
</script>

<template>
  <VCard compact style="margin-top: var(--space-6)">
    <div v-if="!analysis" class="v-gap-empty">
      <div style="flex: 1; min-width: 0; width: 100%">
        <div class="v-gap-title-row">
          <h3 class="t-section">Analisis Gap &amp; Saran CV</h3>
          <VBadge tone="ai">
            <VIcon name="aiNode" :size="13" />
            <span>AI · Gemini</span>
          </VBadge>
        </div>
        <p class="t-body" style="margin-top: var(--space-2)">
          Belum pernah dianalisis. Hasilnya disimpan di cache SQLite lokal agar tidak boros kuota.
        </p>
      </div>
      <VButton variant="ai" :loading="loading" @click="emit('analyze', false)">
        <VIcon v-if="!loading" name="aiNode" :size="14" />
        <span>{{ loading ? "Menganalisis..." : "Jalankan Analisis AI" }}</span>
      </VButton>
    </div>

    <div v-else>
      <div class="v-gap-head">
        <div class="v-gap-title-row">
          <h3 class="t-section">Analisis Gap &amp; Saran CV</h3>
          <VBadge tone="ai">
            <VIcon name="aiNode" :size="13" />
            <span>AI · Cache lokal</span>
          </VBadge>
        </div>
        <VButton variant="tertiary" size="compact" :loading="loading" @click="emit('analyze', true)">
          Analisis Ulang
        </VButton>
      </div>

      <p class="v-gap-verdict">{{ analysis.verdict_headline }}</p>

      <div class="v-gap-grid">
        <div>
          <div style="font-size: 13px; font-weight: 600; color: var(--status-success)">
            Kekuatan Utama
          </div>
          <ul class="v-clean-list">
            <li v-for="(item, idx) in analysis.strength_highlights" :key="idx">{{ item }}</li>
          </ul>
        </div>

        <div>
          <div style="font-size: 13px; font-weight: 600; color: var(--brand-hover)">
            Penyesuaian CV
          </div>
          <ul class="v-clean-list">
            <li v-for="(tip, idx) in analysis.cv_tailoring_tips" :key="idx">{{ tip }}</li>
          </ul>
        </div>
      </div>

      <div v-if="analysis.prioritized_gaps.length > 0" class="v-gap-priority">
        <div style="font-size: 13px; font-weight: 600; color: var(--status-warning); margin-bottom: var(--space-2)">
          Prioritas Celah Skill
        </div>
        <div class="v-gap-priority-list">
          <div
            v-for="gap in analysis.prioritized_gaps"
            :key="gap.skill"
            class="v-gap-priority-item"
          >
            <VBadge
              :tone="
                gap.severity === 'high'
                  ? 'error'
                  : gap.severity === 'medium'
                    ? 'warning'
                    : 'neutral'
              "
            >
              {{ gap.skill }}
            </VBadge>
            <span class="t-secondary">{{ gap.reason }}</span>
          </div>
        </div>
      </div>

      <div class="v-gap-foot">
        <span class="t-meta">Rekomendasi AI bersifat pendamping, bukan jaminan hasil seleksi.</span>
        <VButton variant="ai" :loading="generatingRoadmap" @click="emit('create-roadmap')">
          <VIcon v-if="!generatingRoadmap" name="aiNode" :size="14" />
          <span>{{ generatingRoadmap ? "Menyusun..." : "Buat Roadmap Belajar" }}</span>
        </VButton>
      </div>
    </div>
  </VCard>
</template>

<style scoped>
.v-gap-empty,
.v-gap-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  flex-wrap: wrap;
}
.v-gap-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.v-gap-verdict {
  margin: var(--space-4) 0 var(--space-6);
  font-size: 15px;
  font-weight: 500;
  line-height: 1.6;
  color: var(--text-primary);
}
.v-gap-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-8);
}
.v-clean-list {
  margin: var(--space-2) 0 0;
  padding-left: 18px;
  list-style-type: disc;
  color: var(--text-secondary);
}
.v-clean-list li {
  margin-bottom: var(--space-2);
  line-height: 1.55;
}
.v-gap-priority {
  margin-top: var(--space-5);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-divider);
}
.v-gap-priority-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.v-gap-priority-item {
  display: flex;
  align-items: baseline;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-gap-foot {
  margin-top: var(--space-5);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-divider);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  flex-wrap: wrap;
}
@media (max-width: 1023px) {
  .v-gap-grid {
    grid-template-columns: 1fr;
    gap: var(--space-5);
  }
}
</style>
