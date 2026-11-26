<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import type { SearchPreferences_Serialize } from "@/lib/bindings";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VDialog from "@/components/ui/VDialog.vue";
import VIcon from "@/components/ui/VIcon.vue";
import VSwitch from "@/components/ui/VSwitch.vue";

const props = defineProps<{
  open: boolean;
  preferences: SearchPreferences_Serialize;
  saving?: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "save", prefs: SearchPreferences_Serialize, runImmediately: boolean): void;
}>();

const QUICK_CITIES = [
  "Indonesia",
  "Jakarta",
  "Bandung",
  "Surabaya",
  "Yogyakarta",
  "Bali",
  "Medan",
  "Makassar",
  "Remote",
];
const PORTALS = [
  { id: "jobstreet", label: "JobStreet", hint: "Multi-halaman" },
  { id: "linkedin", label: "LinkedIn", hint: "Guest" },
  { id: "kalibrr", label: "Kalibrr", hint: "Lintas Jurusan" },
  { id: "dealls", label: "Dealls", hint: "Indonesia" },
  { id: "indeed", label: "Indeed", hint: "RSS / Worker" },
  { id: "glints", label: "Glints", hint: "Worker Opsional" },
];

const draft = reactive<SearchPreferences_Serialize>({
  location: "Indonesia",
  locations: ["Indonesia"],
  sources: ["jobstreet", "linkedin", "kalibrr", "dealls"],
  limit_per_query: 25,
  max_pages: 2,
  posted_within_days: 14,
  work_mode: "any",
  custom_queries: [],
  ai_enrich_enabled: false,
  ai_enrich_max_jobs: 15,
  ai_rpm_limit: 10,
});

const customCityInput = ref("");
const customQueriesText = reactive({ value: "" });

watch(
  () => [props.open, props.preferences] as const,
  ([isOpen, prefs]) => {
    if (isOpen && prefs) {
      const locs =
        prefs.locations && prefs.locations.length > 0
          ? [...prefs.locations]
          : [prefs.location || "Indonesia"];
      draft.location = locs[0];
      draft.locations = locs;
      draft.sources = [...prefs.sources];
      draft.limit_per_query = prefs.limit_per_query;
      draft.max_pages = prefs.max_pages;
      draft.posted_within_days = prefs.posted_within_days;
      draft.work_mode = prefs.work_mode;
      draft.custom_queries = [...prefs.custom_queries];
      draft.ai_enrich_enabled = prefs.ai_enrich_enabled;
      draft.ai_enrich_max_jobs = prefs.ai_enrich_max_jobs;
      draft.ai_rpm_limit = prefs.ai_rpm_limit;
      customQueriesText.value = prefs.custom_queries.join(", ");
    }
  },
  { immediate: true },
);

function toggleCity(city: string) {
  const exists = draft.locations.some((c) => c.toLowerCase() === city.toLowerCase());
  if (exists) {
    if (draft.locations.length === 1) return;
    draft.locations = draft.locations.filter((c) => c.toLowerCase() !== city.toLowerCase());
  } else {
    if (draft.locations.length >= 5) return;
    draft.locations = [...draft.locations, city];
  }
  draft.location = draft.locations[0] ?? "Indonesia";
}

function addCustomCity() {
  const clean = customCityInput.value.trim();
  if (!clean || draft.locations.length >= 5) return;
  if (!draft.locations.some((c) => c.toLowerCase() === clean.toLowerCase())) {
    draft.locations = [...draft.locations, clean];
    draft.location = draft.locations[0];
  }
  customCityInput.value = "";
}

function togglePortal(id: string) {
  if (draft.sources.includes(id)) {
    if (draft.sources.length === 1) return;
    draft.sources = draft.sources.filter((s) => s !== id);
  } else {
    draft.sources = [...draft.sources, id];
  }
}

const estimatedApiCalls = computed(() => {
  if (!draft.ai_enrich_enabled) return 0;
  const batches = Math.ceil(draft.ai_enrich_max_jobs / 5);
  return batches * 2;
});

function buildPayload(): SearchPreferences_Serialize {
  const parsedQueries = customQueriesText.value
    .split(/[,\n]/)
    .map((q) => q.trim())
    .filter((q) => q.length > 0)
    .slice(0, 15);
  const locs = draft.locations.length > 0 ? draft.locations : ["Indonesia"];
  return {
    location: locs[0],
    locations: [...locs],
    sources: [...draft.sources],
    limit_per_query: draft.limit_per_query,
    max_pages: draft.max_pages,
    posted_within_days: draft.posted_within_days,
    work_mode: draft.work_mode,
    custom_queries: parsedQueries,
    ai_enrich_enabled: draft.ai_enrich_enabled,
    ai_enrich_max_jobs: draft.ai_enrich_max_jobs,
    ai_rpm_limit: draft.ai_rpm_limit,
  };
}
</script>

<template>
  <VDialog :open="open" title="Atur Pencarian Multi-Kota & Hemat Token" @close="emit('close')">
    <form class="v-pref-form" @submit.prevent="emit('save', buildPayload(), true)">
      <!-- Multi-City Location Picker -->
      <div class="v-pref-group">
        <label for="pref-location" class="v-pref-label">
          Kota / Lokasi Target (pilih hingga 5 lokasi untuk scraping serentak)
        </label>
        <div class="v-city-chips">
          <button
            v-for="city in QUICK_CITIES"
            :key="city"
            type="button"
            class="v-chip-btn"
            :aria-pressed="draft.locations.some((c) => c.toLowerCase() === city.toLowerCase())"
            @click="toggleCity(city)"
          >
            {{ city }}
          </button>
        </div>
        <div class="v-city-add">
          <input
            id="pref-location"
            v-model="customCityInput"
            type="text"
            class="v-pref-input"
            placeholder="Tambah kota lain (mis. Semarang, Balikpapan, Batam)..."
            @keydown.enter.prevent="addCustomCity"
          />
          <VButton type="button" variant="secondary" size="compact" @click="addCustomCity">
            Tambah
          </VButton>
        </div>
        <div class="t-meta">
          Aktif ({{ draft.locations.length }}/5): {{ draft.locations.join(" · ") }}
        </div>
      </div>

      <!-- Custom keywords -->
      <div class="v-pref-group">
        <label for="pref-queries" class="v-pref-label">
          Kata Kunci Pencarian (opsional, pisahkan dengan koma)
        </label>
        <input
          id="pref-queries"
          v-model="customQueriesText.value"
          type="text"
          class="v-pref-input"
          placeholder="Kosongkan untuk memakai kata kunci aktif dari Profil Cerita"
        />
      </div>

      <!-- Portals -->
      <div class="v-pref-group">
        <div class="v-pref-label">Portal Sumber Lowongan</div>
        <div class="v-portal-grid">
          <button
            v-for="p in PORTALS"
            :key="p.id"
            type="button"
            class="v-portal-btn"
            :aria-pressed="draft.sources.includes(p.id)"
            @click="togglePortal(p.id)"
          >
            <span class="v-portal-btn__name">{{ p.label }}</span>
            <span class="t-meta">{{ p.hint }}</span>
          </button>
        </div>
      </div>

      <!-- Volume & Recency -->
      <div class="v-pref-grid">
        <div class="v-pref-group">
          <label for="pref-limit" class="v-pref-label">Lowongan / Query</label>
          <select id="pref-limit" v-model.number="draft.limit_per_query" class="v-pref-select">
            <option :value="15">15 lowongan</option>
            <option :value="25">25 lowongan</option>
            <option :value="40">40 lowongan</option>
            <option :value="50">50 lowongan</option>
          </select>
        </div>

        <div class="v-pref-group">
          <label for="pref-pages" class="v-pref-label">Jumlah Halaman</label>
          <select id="pref-pages" v-model.number="draft.max_pages" class="v-pref-select">
            <option :value="1">1 halaman</option>
            <option :value="2">2 halaman</option>
            <option :value="3">3 halaman</option>
          </select>
        </div>

        <div class="v-pref-group">
          <label for="pref-days" class="v-pref-label">Rentang Posting</label>
          <select id="pref-days" v-model="draft.posted_within_days" class="v-pref-select">
            <option :value="null">Kapan saja</option>
            <option :value="3">3 hari terakhir</option>
            <option :value="7">7 hari terakhir</option>
            <option :value="14">14 hari terakhir</option>
            <option :value="30">30 hari terakhir</option>
          </select>
        </div>

        <div class="v-pref-group">
          <label for="pref-mode" class="v-pref-label">Mode Kerja</label>
          <select id="pref-mode" v-model="draft.work_mode" class="v-pref-select">
            <option value="any">Semua Mode</option>
            <option value="remote">Remote</option>
            <option value="hybrid">Hybrid</option>
            <option value="onsite">Onsite</option>
          </select>
        </div>
      </div>

      <!-- Decoupled AI Matching Toggle & RPM Guard -->
      <div class="v-ai-box">
        <div class="v-ai-box__head">
          <VSwitch
            v-model="draft.ai_enrich_enabled"
            label="Otomatis jalankan AI Matching setelah scrape"
          />
          <VBadge :tone="draft.ai_enrich_enabled ? 'ai' : 'success'" class="t-num">
            <VIcon v-if="draft.ai_enrich_enabled" name="aiNode" :size="12" />
            <span>
              {{
                draft.ai_enrich_enabled
                  ? `~${estimatedApiCalls} req Gemini`
                  : "0 token Gemini"
              }}
            </span>
          </VBadge>
        </div>

        <p class="t-meta" style="margin-top: var(--space-2)">
          {{
            draft.ai_enrich_enabled
              ? "Tahap 1 menyimpan semua lowongan secara lokal dulu, lalu Tahap 2 memperkaya Top-N lowongan secara batch (5 lowongan/call) dengan batas aman RPM."
              : "Mode Cepat Lokal: semua lowongan langsung tersimpan rapi dan dinilai memakai kamus skill + kata kunci lokal (0 token). Kamu bisa jalankan AI Matching kapan saja nanti."
          }}
        </p>

        <div v-if="draft.ai_enrich_enabled" class="v-pref-grid" style="margin-top: var(--space-3)">
          <div class="v-pref-group">
            <label for="pref-ai-max" class="v-pref-label">Batas AI / Sesi</label>
            <select id="pref-ai-max" v-model.number="draft.ai_enrich_max_jobs" class="v-pref-select">
              <option :value="10">Top 10 lowongan</option>
              <option :value="15">Top 15 lowongan</option>
              <option :value="25">Top 25 lowongan</option>
              <option :value="50">Top 50 lowongan</option>
            </select>
          </div>

          <div class="v-pref-group">
            <label for="pref-rpm" class="v-pref-label">Batas Kecepatan (RPM)</label>
            <select id="pref-rpm" v-model.number="draft.ai_rpm_limit" class="v-pref-select">
              <option :value="5">5 RPM (Ekstra aman)</option>
              <option :value="10">10 RPM (Aman Free Tier)</option>
              <option :value="15">15 RPM (Standar)</option>
              <option :value="30">30 RPM (Cepat / Paid)</option>
            </select>
          </div>
        </div>
      </div>

      <div class="v-pref-actions">
        <VButton variant="tertiary" type="button" @click="emit('close')">Batal</VButton>
        <div class="v-pref-actions__right">
          <VButton
            variant="secondary"
            type="button"
            :disabled="saving"
            @click="emit('save', buildPayload(), false)"
          >
            Simpan Saja
          </VButton>
          <VButton variant="primary" type="submit" :loading="saving">
            Simpan &amp; Cari Lowongan
          </VButton>
        </div>
      </div>
    </form>
  </VDialog>
</template>

<style scoped>
.v-pref-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  max-height: 76vh;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding-right: 2px;
}
.v-pref-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.v-pref-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
}
.v-pref-input,
.v-pref-select {
  width: 100%;
  height: 40px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
}
.v-pref-input:focus,
.v-pref-select:focus {
  border-color: var(--brand-hover);
}
.v-city-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 2px;
}
.v-city-add {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}
.v-chip-btn {
  height: 30px;
  padding: 0 11px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-default);
  background: var(--bg-primary);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  transition: background-color 140ms ease, color 140ms ease, border-color 140ms ease;
}
.v-chip-btn[aria-pressed="true"] {
  background: var(--filter-active-bg);
  border-color: var(--filter-active-border);
  color: var(--filter-active-fg);
  font-weight: 600;
}
.v-portal-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-2);
}
.v-portal-btn {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-default);
  background: var(--bg-primary);
  color: var(--text-secondary);
  text-align: left;
}
.v-portal-btn[aria-pressed="true"] {
  background: var(--brand-soft);
  border-color: var(--brand-border);
  color: var(--text-primary);
}
.v-portal-btn__name {
  font-size: 13px;
  font-weight: 600;
}
.v-pref-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-3);
}
.v-ai-box {
  padding: var(--space-4);
  border-radius: var(--radius-card);
  background: var(--bg-primary);
  border: 1px solid var(--border-default);
}
.v-ai-box__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-pref-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding-top: var(--space-3);
  border-top: 1px solid var(--border-divider);
  flex-wrap: wrap;
}
.v-pref-actions__right {
  display: flex;
  gap: var(--space-2);
  flex-wrap: wrap;
}
</style>
