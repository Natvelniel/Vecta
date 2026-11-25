<script setup lang="ts">
import { computed, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useProfileStore } from "@/stores/profile";
import { useSettingsStore } from "@/stores/settings";
import { useToast } from "@/composables/useToast";
import {
  formatExperienceLevel,
  formatFitType,
  formatProficiency,
  nextProficiency,
} from "@/lib/format";
import { openExternalUrl } from "@/lib/ipc";
import PageHeader from "@/components/layout/PageHeader.vue";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";
import VIcon from "@/components/ui/VIcon.vue";

const PRESET_STORIES: Record<string, string> = {
  "Akuntansi & Pajak":
    "Saya lulusan S1 Akuntansi yang berpengalaman menyusun laporan keuangan sesuai PSAK, rekonsiliasi bank, dan pelaporan PPh 21, PPh 23, serta PPN menggunakan e-Faktur, e-Bupot, dan Accurate Online. Memiliki sertifikat Brevet A/B serta mahir Microsoft Excel Pivot Table.",
  "Digital Marketing":
    "Saya berpengalaman mengelola kampanye Meta Ads, TikTok Ads, dan Google Ads untuk brand retail. Terbiasa menyusun strategi SEO, kalender konten Social Media Management, serta Copywriting untuk meningkatkan konversi penjualan.",
  "Teknik Sipil":
    "Saya lulusan S1 Teknik Sipil dengan pengalaman Site Engineering dan Quantity Surveyor. Menguasai AutoCAD, ETABS, Revit BIM, penyusunan RAB dan BoQ, serta pemantauan Kurva S di proyek konstruksi gedung.",
  "Software Dev":
    "Saya lulusan Informatika yang membangun backend REST API memakai Python FastAPI dan aplikasi desktop lokal dengan Rust serta SQLite. Terbiasa merancang skema database relasional, menulis unit test otomatis, dan ingin berkembang sebagai Backend Engineer.",
};

const route = useRoute();
const router = useRouter();
const settingsStore = useSettingsStore();
const profileStore = useProfileStore();
const toast = useToast();

const isFirstRun = computed(() => route.query.firstRun === "1");
const firstRunStep = ref<1 | 2>(settingsStore.apiKeyStatus.has_key ? 2 : 1);

const apiKeyInput = ref("");
const addingSkill = ref(false);
const newSkillName = ref("");
const customKeywordInput = ref("");
const pdfInputRef = ref<HTMLInputElement | null>(null);

async function handleImportPdfFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  if (!file.name.toLowerCase().endsWith(".pdf")) {
    toast.show("Pilih file dokumen berformat .pdf.", { tone: "warning" });
    return;
  }
  const reader = new FileReader();
  reader.onload = async () => {
    const dataUrl = String(reader.result ?? "");
    const draft = await profileStore.importCvPdf(dataUrl);
    if (draft) {
      toast.show(
        `CV PDF terbaca lokal (0 token): ${draft.extracted_skills.length} skill terdeteksi. Tinjau lalu klik Simpan.`,
        { tone: "success" },
      );
    } else if (profileStore.error) {
      toast.show(profileStore.error, { tone: "error" });
    }
  };
  reader.readAsDataURL(file);
}

async function handleParseLocal() {
  const ok = await profileStore.parseStoryLocal();
  if (ok) {
    toast.show(
      "Profil dipetakan secara lokal lintas 14 rumpun prodi (0 token Gemini). Tinjau lalu simpan.",
      { tone: "success" },
    );
  }
}

async function handleSaveApiKey() {
  const ok = await settingsStore.saveAndTestApiKey(apiKeyInput.value);
  if (ok) {
    apiKeyInput.value = "";
    firstRunStep.value = 2;
    toast.show("Kunci API Gemini terverifikasi dan tersimpan aman di OS Keychain.", {
      tone: "success",
    });
  }
}

function skipFirstRun() {
  sessionStorage.setItem("vecta:skipped_onboarding", "1");
  router.push("/dashboard");
}

function applyPreset(label: string) {
  profileStore.rawStoryDraft = PRESET_STORIES[label] ?? "";
}

async function handleParseStory() {
  const ok = await profileStore.parseStory();
  if (ok) {
    toast.show("AI selesai memetakan profil. Tinjau skill dan kata kunci sebelum menyimpan.", {
      tone: "success",
    });
  }
}

function cycleSkill(index: number) {
  const item = profileStore.skillsDraft[index];
  if (!item) return;
  item.proficiency = nextProficiency(item.proficiency);
}

function removeSkill(index: number) {
  const removed = profileStore.skillsDraft.splice(index, 1)[0];
  if (!removed) return;
  toast.show(`Skill ${removed.canonical_name} dihapus dari draf profil.`, {
    tone: "info",
    actionLabel: "Urungkan",
    onAction: () => {
      profileStore.skillsDraft.splice(index, 0, removed);
    },
  });
}

function confirmAddSkill() {
  const clean = newSkillName.value.trim();
  if (!clean) return;
  const exists = profileStore.skillsDraft.some(
    (s) => s.canonical_name.toLowerCase() === clean.toLowerCase(),
  );
  if (!exists) {
    profileStore.skillsDraft.push({
      canonical_name: clean,
      category: "core_hard",
      proficiency: "beginner",
      evidence: "Ditambahkan manual oleh pengguna.",
    });
  }
  newSkillName.value = "";
  addingSkill.value = false;
}

async function handleToggleKeyword(index: number) {
  const ok = await profileStore.toggleKeyword(index);
  const kw = profileStore.keywordsDraft[index];
  if (ok && kw) {
    toast.show(
      `Kata kunci ${kw.role_label} ${kw.is_active ? "diaktifkan" : "dinonaktifkan"} (0 token Gemini).`,
      { tone: "success" },
    );
  }
}

async function handleAddCustomKeyword() {
  const clean = customKeywordInput.value.trim();
  if (!clean) return;
  const ok = await profileStore.addCustomKeyword(clean);
  if (ok) {
    customKeywordInput.value = "";
    toast.show(`Peran kustom ${clean} ditambahkan.`, { tone: "success" });
  }
}

async function handleSaveProfile() {
  const ok = await profileStore.saveProfile();
  if (ok) {
    sessionStorage.removeItem("vecta:skipped_onboarding");
    toast.show("Profil tersimpan dan skor kecocokan lowongan diperbarui secara lokal.", {
      tone: "success",
    });
    router.push("/jobs");
  }
}
</script>

<template>
  <!-- Guided First-Run Step 1: BYOK Gemini Key -->
  <div v-if="isFirstRun && firstRunStep === 1" class="v-narrow-wrap">
    <div class="v-fr-top">
      <div class="v-fr-step">
        <VBadge tone="brand" class="t-num">Langkah 1 dari 2</VBadge>
        <span class="t-meta">Koneksi Kunci AI Lokal</span>
      </div>
      <VButton variant="tertiary" size="compact" @click="skipFirstRun">
        Lewati ke Dashboard
      </VButton>
    </div>

    <h1 class="t-page">Hubungkan Kunci API Gemini</h1>
    <p class="t-body" style="margin-top: var(--space-3); max-width: 60ch">
      Vecta menggunakan model Bring Your Own Key. Kunci disimpan melalui OS Credential Manager
      perangkatmu, tidak pernah dikirim ke server pihak ketiga atau disimpan ke SQLite.
    </p>

    <VCard style="margin-top: var(--space-8)">
      <form @submit.prevent="handleSaveApiKey">
        <label for="fr-api-key" class="v-field-label">Kunci API Google Gemini</label>
        <div class="v-inline-field">
          <input
            id="fr-api-key"
            v-model="apiKeyInput"
            type="password"
            class="v-input t-num"
            placeholder="Tempel kunci dari Google AI Studio (AIza...)"
            autocomplete="off"
          />
          <VButton type="submit" variant="primary" size="tall" :loading="settingsStore.loadingKey">
            {{ settingsStore.loadingKey ? "Memverifikasi..." : "Tes & Simpan Kunci" }}
          </VButton>
        </div>
        <p v-if="settingsStore.error" class="v-error-text" role="alert">
          {{ settingsStore.error }}
        </p>
      </form>

      <div class="v-fr-card-foot">
        <span class="t-meta">Belum punya kunci? Dapatkan gratis di Google AI Studio.</span>
        <VButton
          variant="secondary"
          size="compact"
          @click="openExternalUrl('https://aistudio.google.com/apikey')"
        >
          <span>Buka Google AI Studio</span>
          <VIcon name="external" :size="13" />
        </VButton>
      </div>
    </VCard>
  </div>

  <!-- Story-to-Chips Studio (First-Run Step 2 OR Regular Workspace Editor) -->
  <div v-else class="v-story-root">
    <PageHeader
      v-if="!isFirstRun"
      title="Profil & CV"
      subtitle="Impor CV PDF instan (0 token) atau tulis cerita lintas 14 rumpun prodi Indonesia"
    >
      <template #actions>
        <input
          ref="pdfInputRef"
          type="file"
          accept="application/pdf,.pdf"
          class="sr-only"
          @change="handleImportPdfFile"
        />
        <VButton
          variant="secondary"
          :loading="profileStore.parsing"
          @click="pdfInputRef?.click()"
        >
          <VIcon name="plus" :size="14" />
          <span>Impor CV PDF (0 Token)</span>
        </VButton>
        <VButton
          variant="primary"
          :disabled="!profileStore.hasStructuredDraft || profileStore.skillsDraft.length === 0"
          :loading="profileStore.saving"
          @click="handleSaveProfile"
        >
          {{ profileStore.saving ? "Menyimpan..." : "Simpan & Lihat Lowongan" }}
        </VButton>
      </template>
    </PageHeader>

    <div class="v-page-container">
      <div v-if="isFirstRun" style="margin-bottom: var(--space-8)">
        <div class="v-fr-top" style="margin-bottom: var(--space-6)">
          <div class="v-fr-step">
            <VBadge tone="brand" class="t-num">Langkah 2 dari 2</VBadge>
            <span class="t-meta">Ceritakan Profilmu</span>
          </div>
          <VButton variant="tertiary" size="compact" @click="skipFirstRun">
            Lewati ke Dashboard
          </VButton>
        </div>
        <h1 class="t-page">Ceritakan Pengalaman &amp; Proyekmu</h1>
        <p class="t-body" style="margin-top: var(--space-2); max-width: 64ch">
          Tulis dengan bahasa sehari-hari. AI menyusunnya menjadi profil terstruktur yang bisa kamu
          koreksi sebelum disimpan.
        </p>
      </div>

      <div class="v-studio-grid">
        <!-- Left Column: User Natural-Language Story & Editable Skill Chips -->
        <div>
          <VCard>
            <div class="v-field-head">
              <label for="story-input" class="v-field-label" style="margin-bottom: 0">
                Cerita pengalaman, skill, dan proyekmu
              </label>
              <span class="t-meta t-num">{{ profileStore.rawStoryDraft.trim().length }} karakter</span>
            </div>

            <textarea
              id="story-input"
              v-model="profileStore.rawStoryDraft"
              class="v-textarea"
              placeholder="Ceritakan proyek yang pernah kamu kerjakan, tools yang dikuasai, dan peran yang ingin kamu tuju (minimal 30 karakter)..."
            />

            <p v-if="profileStore.error" class="v-error-text" role="alert">
              {{ profileStore.error }}
            </p>

            <div class="v-story-bar">
              <div class="v-presets">
                <span class="t-meta" style="margin-right: 4px">Contoh:</span>
                <VButton
                  v-for="label in Object.keys(PRESET_STORIES)"
                  :key="label"
                  variant="tertiary"
                  size="compact"
                  style="padding: 0 8px"
                  @click="applyPreset(label)"
                >
                  {{ label }}
                </VButton>
              </div>
              <div style="display: flex; gap: 8px; flex-wrap: wrap">
                <VButton
                  variant="secondary"
                  :loading="profileStore.parsing"
                  @click="handleParseLocal"
                >
                  <span>Petakan Lokal (0 Token)</span>
                </VButton>
                <VButton variant="ai" :loading="profileStore.parsing" @click="handleParseStory">
                  <VIcon v-if="!profileStore.parsing" name="aiNode" :size="14" />
                  <span>{{ profileStore.parsing ? "Memetakan..." : "Petakan · AI" }}</span>
                </VButton>
              </div>
            </div>
          </VCard>

          <section v-if="profileStore.hasStructuredDraft" style="margin-top: var(--space-8)">
            <div class="v-field-head" style="margin-bottom: var(--space-3)">
              <h2 class="t-section">
                Skill Terdeteksi
                <span class="t-meta t-num" style="margin-left: 6px">
                  {{ profileStore.skillsDraft.length }}
                </span>
              </h2>
              <span class="t-meta">Klik skill untuk ubah level (Dasar / Menengah / Mahir)</span>
            </div>

            <div class="v-chip-wrap">
              <span
                v-for="(sk, idx) in profileStore.skillsDraft"
                :key="sk.canonical_name"
                class="v-skill-edit"
              >
                <button
                  type="button"
                  class="v-skill-edit__main"
                  :title="sk.evidence || 'Klik untuk ubah tingkat kemahiran'"
                  @click="cycleSkill(idx)"
                >
                  <span>{{ sk.canonical_name }}</span>
                  <span class="t-meta">{{ formatProficiency(sk.proficiency) }}</span>
                </button>
                <button
                  type="button"
                  class="v-skill-edit__del"
                  :aria-label="`Hapus skill ${sk.canonical_name}`"
                  @click="removeSkill(idx)"
                >
                  <VIcon name="close" :size="13" />
                </button>
              </span>

              <VButton
                v-if="!addingSkill"
                variant="secondary"
                size="compact"
                style="height: 38px"
                @click="addingSkill = true"
              >
                <VIcon name="plus" :size="14" />
                <span>Tambah Skill</span>
              </VButton>

              <form v-else class="v-add-skill-form" @submit.prevent="confirmAddSkill">
                <label for="new-skill-field" class="sr-only">Nama skill baru</label>
                <input
                  id="new-skill-field"
                  v-model="newSkillName"
                  type="text"
                  class="v-input-compact"
                  placeholder="Nama skill..."
                />
                <VButton type="submit" variant="primary" size="compact" style="height: 38px">
                  Simpan
                </VButton>
                <VButton
                  variant="tertiary"
                  size="compact"
                  style="height: 38px"
                  @click="addingSkill = false"
                >
                  Batal
                </VButton>
              </form>
            </div>
          </section>
        </div>

        <!-- Right Column: AI-Structured Summary & Search Keywords -->
        <aside v-if="profileStore.hasStructuredDraft" aria-label="Ringkasan profil dan kata kunci">
          <VCard>
            <div class="v-dossier-head">
              <div class="v-dossier-title">
                <h2 class="t-section">Ringkasan Profil</h2>
                <VBadge tone="ai">
                  <VIcon name="aiNode" :size="13" />
                  <span>AI-Structured</span>
                </VBadge>
              </div>
              <VBadge tone="brand">
                {{ formatExperienceLevel(profileStore.experienceLevelDraft) }}
              </VBadge>
            </div>

            <p class="t-body" style="margin-top: var(--space-3)">
              {{ profileStore.summaryDraft || "Ringkasan profil akan tampil setelah pemetaan AI." }}
            </p>

            <div class="v-kw-section">
              <div class="v-field-head" style="margin-bottom: var(--space-3)">
                <h3 class="t-card">Kata Kunci Lowongan</h3>
                <span class="t-meta">0 token Gemini</span>
              </div>

              <div class="v-kw-list">
                <button
                  v-for="(kw, idx) in profileStore.keywordsDraft"
                  :key="kw.id ?? kw.role_label"
                  type="button"
                  class="v-kw-btn"
                  :aria-pressed="kw.is_active"
                  @click="handleToggleKeyword(idx)"
                >
                  <div class="v-kw-btn__top">
                    <span style="font-weight: 600; font-size: 14px; color: var(--text-primary)">
                      {{ kw.role_label }}
                    </span>
                    <span
                      class="t-meta"
                      :style="{
                        color: kw.is_active ? 'var(--status-success)' : 'var(--text-secondary)',
                      }"
                    >
                      {{ kw.is_active ? "Aktif" : "Nonaktif" }}
                    </span>
                  </div>
                  <div class="t-meta" style="margin-top: 4px">
                    {{ formatFitType(kw.fit_type) }} · {{ kw.scrape_queries.join(", ") }}
                  </div>
                </button>
              </div>

              <form class="v-kw-add" @submit.prevent="handleAddCustomKeyword">
                <label for="onb-custom-kw" class="sr-only">Tambah peran kustom</label>
                <input
                  id="onb-custom-kw"
                  v-model="customKeywordInput"
                  type="text"
                  class="v-input"
                  style="height: 40px; font-size: 13px"
                  placeholder="Tambah peran kustom..."
                />
                <VButton type="submit" variant="secondary">Tambah</VButton>
              </form>
            </div>

            <div v-if="isFirstRun" class="v-dossier-foot">
              <VButton
                variant="primary"
                style="width: 100%"
                :loading="profileStore.saving"
                @click="handleSaveProfile"
              >
                {{ profileStore.saving ? "Menyimpan..." : "Simpan Profil & Lihat Lowongan" }}
              </VButton>
            </div>
          </VCard>
        </aside>

        <aside v-else>
          <VCard>
            <h2 class="t-section">Cara Kerja Pemetaan Profil</h2>
            <p class="t-body" style="margin-top: var(--space-3)">
              Setelah kamu menulis pengalamanmu dan menekan <strong>Petakan Profil · AI</strong>,
              Vecta mengekstrak daftar skill baku, tingkat pengalaman, dan rekomendasi kata kunci
              pencarian lowongan.
            </p>
            <p class="t-body" style="margin-top: var(--space-3)">
              Seluruh hasil ekstraksi AI bisa kamu koreksi sebelum disimpan ke database SQLite lokal.
            </p>
          </VCard>
        </aside>
      </div>
    </div>
  </div>
</template>

<style scoped>
.v-story-root {
  display: flex;
  flex-direction: column;
  flex: 1;
}
.v-narrow-wrap {
  max-width: 800px;
  margin: 0 auto;
  padding: var(--space-16) var(--space-8);
  width: 100%;
}
.v-page-container {
  max-width: 1200px;
  width: 100%;
  margin: 0 auto;
  padding: var(--space-8);
}
.v-fr-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  margin-bottom: var(--space-8);
  flex-wrap: wrap;
}
.v-fr-step {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
.v-inline-field {
  display: flex;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-fr-card-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
  margin-top: var(--space-5);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-divider);
}
.v-studio-grid {
  display: grid;
  grid-template-columns: 1.15fr 0.85fr;
  gap: var(--space-8);
  align-items: start;
}
.v-field-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-3);
  margin-bottom: var(--space-2);
  flex-wrap: wrap;
}
.v-field-label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}
.v-input,
.v-textarea,
.v-input-compact {
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-control);
  font-size: 14px;
}
.v-input {
  flex: 1;
  min-width: 220px;
  height: 44px;
  padding: 0 var(--space-4);
}
.v-input-compact {
  width: 168px;
  height: 38px;
  padding: 0 var(--space-3);
  font-size: 13px;
}
.v-textarea {
  width: 100%;
  min-height: 172px;
  padding: var(--space-4);
  resize: vertical;
  line-height: 1.65;
}
.v-input:focus,
.v-textarea:focus,
.v-input-compact:focus {
  border-color: var(--brand-hover);
}
.v-error-text {
  margin: var(--space-3) 0 0;
  font-size: 13px;
  color: var(--status-error);
}
.v-story-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
  margin-top: var(--space-4);
}
.v-presets {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex-wrap: wrap;
}
.v-chip-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  align-items: center;
}
.v-skill-edit {
  display: inline-flex;
  align-items: stretch;
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.v-skill-edit__main {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  height: 36px;
  padding: 0 var(--space-3);
  border: 0;
  background: transparent;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}
.v-skill-edit__main:hover {
  background: var(--bg-hover);
}
.v-skill-edit__del {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  border: 0;
  border-left: 1px solid var(--border-divider);
  background: transparent;
  color: var(--text-secondary);
}
.v-skill-edit__del:hover {
  background: var(--error-soft);
  color: var(--status-error);
}
.v-add-skill-form {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}
.v-dossier-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-dossier-title {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}
.v-kw-section {
  margin-top: var(--space-6);
  padding-top: var(--space-6);
  border-top: 1px solid var(--border-divider);
}
.v-kw-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.v-kw-btn {
  width: 100%;
  text-align: left;
  padding: var(--space-4);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-divider);
  background: var(--bg-primary);
  transition: background var(--dur-hover) var(--ease-state), border-color var(--dur-hover) var(--ease-state);
}
.v-kw-btn:hover {
  background: var(--bg-hover);
}
.v-kw-btn[aria-pressed="true"] {
  background: rgba(36, 59, 117, 0.42);
  border-color: var(--brand-border);
}
.v-kw-btn__top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}
.v-kw-add {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-3);
}
.v-dossier-foot {
  margin-top: var(--space-6);
  padding-top: var(--space-5);
  border-top: 1px solid var(--border-divider);
}

@media (max-width: 1199px) {
  .v-studio-grid {
    grid-template-columns: 1fr;
  }
}
@media (max-width: 1023px) {
  .v-page-container,
  .v-narrow-wrap {
    padding: var(--space-5) var(--space-4) var(--space-10);
  }
}
</style>
