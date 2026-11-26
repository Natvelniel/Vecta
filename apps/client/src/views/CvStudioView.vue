<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useRouter } from "vue-router";
import type { AtsExportResult, MasterCvProfile } from "@/lib/bindings";
import { useJobsStore } from "@/stores/jobs";
import { useProfileStore } from "@/stores/profile";
import { useToast } from "@/composables/useToast";
import { ipc, IpcError } from "@/lib/ipc";
import { shareOrDownloadPdf } from "@/lib/pdfShare";
import PageHeader from "@/components/layout/PageHeader.vue";
import CvA4Preview from "@/components/domain/CvA4Preview.vue";
import VBadge from "@/components/ui/VBadge.vue";
import VButton from "@/components/ui/VButton.vue";
import VCard from "@/components/ui/VCard.vue";
import VIcon from "@/components/ui/VIcon.vue";

const router = useRouter();
const profileStore = useProfileStore();
const jobsStore = useJobsStore();
const toast = useToast();

const exportLang = ref<"id" | "en">("id");
const mobileTab = ref<"edit" | "preview">("edit");
const targetJobId = ref<string>("");
const lastExport = ref<AtsExportResult | null>(null);
const certInput = ref("");
const langInput = ref("");

const form = reactive<MasterCvProfile>({
  user_id: "",
  full_name: "",
  headline: "",
  email: "",
  phone: "",
  location: "Indonesia",
  linkedin_url: "",
  portfolio_url: "",
  summary: "",
  experiences: [],
  education: [],
  projects: [],
  certifications: [],
  languages: ["Bahasa Indonesia"],
  updated_at: "",
});

function hydrateForm(cv: MasterCvProfile) {
  form.user_id = cv.user_id;
  form.full_name = cv.full_name;
  form.headline = cv.headline;
  form.email = cv.email;
  form.phone = cv.phone;
  form.location = cv.location;
  form.linkedin_url = cv.linkedin_url;
  form.portfolio_url = cv.portfolio_url;
  form.summary = cv.summary;
  form.experiences = cv.experiences.map((e) => ({ ...e, bullets: [...e.bullets] }));
  form.education = cv.education.map((e) => ({ ...e }));
  form.projects = cv.projects.map((p) => ({ ...p }));
  form.certifications = [...cv.certifications];
  form.languages = [...cv.languages];
  form.updated_at = cv.updated_at;
  certInput.value = cv.certifications.join(", ");
  langInput.value = cv.languages.join(", ");
}

onMounted(async () => {
  if (!profileStore.activeProfile) {
    await profileStore.loadActiveProfile();
  }
  const uid = profileStore.userId;
  if (uid) {
    const [cv] = await Promise.all([profileStore.loadMasterCv(uid), jobsStore.loadJobs(uid)]);
    if (cv) hydrateForm(cv);
    if (jobsStore.jobs.length > 0 && !targetJobId.value) {
      targetJobId.value = jobsStore.selectedJobId ?? jobsStore.jobs[0].job_id;
    }
  }
});

watch(targetJobId, async (jid) => {
  if (jid && profileStore.userId) {
    await jobsStore.selectJob(profileStore.userId, jid);
  }
});

const userSkillNames = computed(
  () => profileStore.activeProfile?.skills.map((s) => s.canonical_name) ?? [],
);

const previewCv = computed<MasterCvProfile>(() => buildPayload());

const atsKeywordDiff = computed(() => {
  const detail = jobsStore.selectedDetail;
  if (!targetJobId.value || !detail || detail.job_id !== targetJobId.value) return null;
  const matched = detail.matched_skills.map((s) => s.canonical_name);
  const missing = detail.missing_skills.map((s) => ({
    skill_id: s.skill_id,
    canonical_name: s.canonical_name,
  }));
  return {
    title: detail.title,
    company: detail.company,
    matched,
    missing,
  };
});

async function handleQuickAddKeywordToCv(skillId: number, canonicalName: string) {
  if (!profileStore.userId) return;
  try {
    await ipc.quickAddUserSkill(profileStore.userId, skillId);
    await Promise.all([
      profileStore.loadActiveProfile(),
      jobsStore.selectJob(profileStore.userId, targetJobId.value),
    ]);
    toast.show(`"${canonicalName}" ditambahkan ke profil & CV ATS.`, { tone: "success" });
  } catch (e) {
    toast.show(e instanceof IpcError ? e.message : "Gagal menambahkan skill.", { tone: "error" });
  }
}

function addProject() {
  form.projects.push({
    name: "",
    period: "",
    description: "",
  });
}

function removeProject(idx: number) {
  form.projects.splice(idx, 1);
}

function addExperience() {
  form.experiences.push({
    role: "",
    company: "",
    location: form.location || "Indonesia",
    period: "",
    bullets: [""],
  });
}

function removeExperience(idx: number) {
  form.experiences.splice(idx, 1);
}

function addEducation() {
  form.education.push({
    institution: "",
    degree: "",
    period: "",
    notes: "",
  });
}

function removeEducation(idx: number) {
  form.education.splice(idx, 1);
}

function buildPayload(): MasterCvProfile {
  return {
    ...form,
    user_id: profileStore.userId ?? form.user_id,
    experiences: form.experiences.map((e) => ({
      ...e,
      bullets: e.bullets.map((b) => b.trim()).filter(Boolean),
    })),
    certifications: certInput.value
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean),
    languages: langInput.value
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean),
  };
}

async function handleSaveCv() {
  if (!profileStore.userId) {
    toast.show("Buat atau impor profil terlebih dahulu di Profil Cerita.", { tone: "warning" });
    router.push("/onboarding");
    return;
  }
  const ok = await profileStore.saveMasterCv(buildPayload());
  if (ok) {
    toast.show("Master CV disimpan ke database SQLite lokal.", { tone: "success" });
  }
}

async function handleShareExportedPdf() {
  if (!lastExport.value) return;
  const result = await shareOrDownloadPdf(lastExport.value.file_name, lastExport.value.pdf_base64);
  toast.show(
    result === "shared"
      ? "Menu bagikan sistem dibuka untuk CV ATS PDF."
      : "CV ATS PDF mulai diunduh ke perangkat.",
    { tone: "success" },
  );
}

async function handleExportMasterPdf() {
  if (!profileStore.userId) return;
  const saved = await profileStore.saveMasterCv(buildPayload());
  if (!saved) return;
  const res = await profileStore.exportAtsPdf(profileStore.userId, null, exportLang.value);
  if (res) {
    lastExport.value = res;
    if (typeof navigator !== "undefined" && /android/i.test(navigator.userAgent)) {
      await handleShareExportedPdf();
    } else {
      toast.show(`CV ATS PDF berhasil dibuat di ${res.file_path}`, {
        tone: "success",
        actionLabel: "Unduh / Bagikan PDF",
        onAction: () => handleShareExportedPdf(),
      });
    }
  } else if (profileStore.error) {
    toast.show(profileStore.error, { tone: "error" });
  }
}
</script>

<template>
  <div class="v-cv-root">
    <PageHeader
      title="Studio CV ATS"
      subtitle="Master CV tunggal -> 1-klik ekspor PDF standar ATS 1 kolom (100% offline)"
    >
      <template #actions>
        <div class="v-lang-group" role="group" aria-label="Bahasa template CV ATS">
          <button
            type="button"
            class="v-lang-btn"
            :aria-pressed="exportLang === 'id'"
            @click="exportLang = 'id'"
          >
            ID
          </button>
          <button
            type="button"
            class="v-lang-btn"
            :aria-pressed="exportLang === 'en'"
            @click="exportLang = 'en'"
          >
            EN
          </button>
        </div>
        <VButton variant="secondary" :loading="profileStore.saving" @click="handleSaveCv">
          Simpan Master CV
        </VButton>
        <VButton
          variant="primary"
          :loading="profileStore.exportingPdf"
          @click="handleExportMasterPdf"
        >
          <span>{{ profileStore.exportingPdf ? "Menyusun PDF..." : "Export CV ATS (PDF)" }}</span>
        </VButton>
      </template>
    </PageHeader>

    <div class="v-page-container">
      <div v-if="lastExport" class="v-export-banner" role="status">
        <div>
          <strong style="color: var(--text-primary)">CV ATS PDF siap unggah!</strong>
          <p class="t-meta" style="margin-top: 2px">
            Tersimpan di: <span class="t-num">{{ lastExport.file_path }}</span> ({{
              Math.round(lastExport.pdf_size_bytes / 1024)
            }} KB · Skor Kata Kunci ATS {{ Math.round(lastExport.ats_keyword_score ?? 100) }}%)
          </p>
        </div>
        <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap">
          <VBadge tone="success">1 Kolom · Terbaca ATS</VBadge>
          <VButton variant="primary" size="compact" @click="handleShareExportedPdf">
            Unduh / Bagikan PDF
          </VButton>
        </div>
      </div>

      <!-- Target Job ATS Keyword Diff -->
      <VCard v-if="jobsStore.jobs.length > 0">
        <div class="v-sec-head" style="flex-wrap: wrap">
          <div>
            <h2 class="t-section">Audit Kata Kunci ATS vs Target Lowongan</h2>
            <p class="t-meta" style="margin-top: 2px">
              Bandingkan skill pada Master CV dengan lowongan target secara instan (0 token).
            </p>
          </div>
          <label for="cv-target-job" class="sr-only">Pilih target lowongan</label>
          <select id="cv-target-job" v-model="targetJobId" class="v-target-select">
            <option v-for="j in jobsStore.jobs.slice(0, 20)" :key="j.job_id" :value="j.job_id">
              {{ j.title }} · {{ j.company }}
            </option>
          </select>
        </div>

        <div v-if="atsKeywordDiff" class="v-diff-grid">
          <div>
            <span class="t-meta" style="color: var(--status-success); font-weight: 700">
              Terpenuhi di CV ({{ atsKeywordDiff.matched.length }})
            </span>
            <div class="v-diff-chips">
              <span v-for="m in atsKeywordDiff.matched" :key="m" class="v-diff-chip v-diff-chip--ok">
                {{ m }}
              </span>
              <span v-if="atsKeywordDiff.matched.length === 0" class="t-meta">Belum ada kecocokan langsung.</span>
            </div>
          </div>

          <div>
            <span class="t-meta" style="color: var(--status-warning); font-weight: 700">
              Belum Ada di CV ({{ atsKeywordDiff.missing.length }})
            </span>
            <div class="v-diff-chips">
              <button
                v-for="miss in atsKeywordDiff.missing"
                :key="miss.skill_id"
                type="button"
                class="v-diff-chip v-diff-chip--miss"
                @click="handleQuickAddKeywordToCv(miss.skill_id, miss.canonical_name)"
              >
                <span>+ {{ miss.canonical_name }} ke CV</span>
              </button>
              <span v-if="atsKeywordDiff.missing.length === 0" class="t-meta">
                Semua skill utama lowongan ini sudah tercakup!
              </span>
            </div>
          </div>
        </div>
      </VCard>

      <!-- Mobile Edit / Live A4 Preview Toggle -->
      <div class="v-cv-mobile-tabs" role="tablist" aria-label="Mode editor CV">
        <button
          type="button"
          role="tab"
          class="v-cv-mtab"
          :class="{ 'v-cv-mtab--active': mobileTab === 'edit' }"
          :aria-selected="mobileTab === 'edit'"
          @click="mobileTab = 'edit'"
        >
          Edit CV
        </button>
        <button
          type="button"
          role="tab"
          class="v-cv-mtab"
          :class="{ 'v-cv-mtab--active': mobileTab === 'preview' }"
          :aria-selected="mobileTab === 'preview'"
          @click="mobileTab = 'preview'"
        >
          Pratinjau A4
        </button>
      </div>

      <div class="v-cv-workspace" :data-mobile-tab="mobileTab">
        <div class="v-cv-editor-col">
          <VCard>
            <h2 class="t-section">Identitas &amp; Ringkasan</h2>
            <p class="t-meta" style="margin-top: 4px">
              Skill utama akan diambil otomatis dari profilmu dan diurutkan ulang mengikuti lowongan
              saat mengekspor dari halaman Lowongan.
            </p>

            <div class="v-form-grid" style="margin-top: var(--space-4)">
              <label class="v-field">
                <span class="v-label">Nama Lengkap</span>
                <input v-model="form.full_name" type="text" class="v-input" placeholder="Nama lengkapmu" />
              </label>
              <label class="v-field">
                <span class="v-label">Headline / Profesi Target</span>
                <input
                  v-model="form.headline"
                  type="text"
                  class="v-input"
                  placeholder="Contoh: Staf Akuntansi & Pajak / Backend Developer"
                />
              </label>
              <label class="v-field">
                <span class="v-label">Email</span>
                <input v-model="form.email" type="email" class="v-input" placeholder="nama@email.com" />
              </label>
              <label class="v-field">
                <span class="v-label">Nomor Telepon / WA</span>
                <input v-model="form.phone" type="text" class="v-input" placeholder="+62 812..." />
              </label>
              <label class="v-field">
                <span class="v-label">Domisili / Kota</span>
                <input v-model="form.location" type="text" class="v-input" placeholder="Bandung, Jawa Barat" />
              </label>
              <label class="v-field">
                <span class="v-label">Tautan LinkedIn / Portofolio</span>
                <input
                  v-model="form.linkedin_url"
                  type="text"
                  class="v-input"
                  placeholder="https://linkedin.com/in/..."
                />
              </label>
            </div>

            <label class="v-field" style="margin-top: var(--space-4)">
              <span class="v-label">Ringkasan Profesional</span>
              <textarea
                v-model="form.summary"
                class="v-textarea"
                placeholder="Ringkasan singkat latar belakang pendidikan, pengalaman inti, dan nilai tambahmu..."
              />
            </label>

            <div class="v-form-grid" style="margin-top: var(--space-4)">
              <label class="v-field">
                <span class="v-label">Sertifikasi (pisahkan dengan koma)</span>
                <input
                  v-model="certInput"
                  type="text"
                  class="v-input"
                  placeholder="Brevet A/B, AK3U Kemnaker, TOEFL..."
                />
              </label>
              <label class="v-field">
                <span class="v-label">Bahasa (pisahkan dengan koma)</span>
                <input
                  v-model="langInput"
                  type="text"
                  class="v-input"
                  placeholder="Bahasa Indonesia, Bahasa Inggris"
                />
              </label>
            </div>
          </VCard>

          <VCard>
            <div class="v-sec-head">
              <h2 class="t-section">Pengalaman Kerja / Magang</h2>
              <VButton variant="secondary" size="compact" @click="addExperience">
                <VIcon name="plus" :size="13" />
                <span>Tambah</span>
              </VButton>
            </div>

            <p v-if="form.experiences.length === 0" class="t-meta" style="margin-top: var(--space-3)">
              Belum ada pengalaman. Klik Tambah untuk mengisi pengalaman kerja, magang, atau organisasi.
            </p>

            <div v-else class="v-item-stack">
              <div v-for="(exp, idx) in form.experiences" :key="idx" class="v-item-box">
                <div class="v-form-grid">
                  <input v-model="exp.role" type="text" class="v-input" placeholder="Posisi / Peran" />
                  <input v-model="exp.company" type="text" class="v-input" placeholder="Perusahaan / Instansi" />
                  <input v-model="exp.period" type="text" class="v-input" placeholder="Periode (mis. 2023 - Sekarang)" />
                  <input v-model="exp.location" type="text" class="v-input" placeholder="Kota" />
                </div>
                <textarea
                  :value="exp.bullets.join('\n')"
                  class="v-textarea v-textarea--sm"
                  placeholder="Satu poin pencapaian per baris..."
                  @input="
                    exp.bullets = ($event.target as HTMLTextAreaElement).value.split('\n')
                  "
                />
                <div style="display: flex; justify-content: flex-end">
                  <VButton variant="tertiary" size="compact" @click="removeExperience(idx)">
                    Hapus Pengalaman
                  </VButton>
                </div>
              </div>
            </div>
          </VCard>

          <VCard>
            <div class="v-sec-head">
              <h2 class="t-section">Proyek &amp; Portofolio</h2>
              <VButton variant="secondary" size="compact" @click="addProject">
                <VIcon name="plus" :size="13" />
                <span>Tambah</span>
              </VButton>
            </div>

            <p v-if="form.projects.length === 0" class="t-meta" style="margin-top: var(--space-3)">
              Tambahkan proyek kuliah, freelance, audit, kampanye, atau portofolio karya terbaikmu.
            </p>

            <div v-else class="v-item-stack">
              <div v-for="(proj, idx) in form.projects" :key="idx" class="v-item-box">
                <div class="v-form-grid">
                  <input v-model="proj.name" type="text" class="v-input" placeholder="Nama Proyek / Karya" />
                  <input v-model="proj.period" type="text" class="v-input" placeholder="Tahun / Periode (mis. 2025)" />
                </div>
                <textarea
                  v-model="proj.description"
                  class="v-textarea v-textarea--sm"
                  placeholder="Ringkas peran, metode/tools yang dipakai, dan hasil terukur..."
                />
                <div style="display: flex; justify-content: flex-end">
                  <VButton variant="tertiary" size="compact" @click="removeProject(idx)">
                    Hapus Proyek
                  </VButton>
                </div>
              </div>
            </div>
          </VCard>

          <VCard>
            <div class="v-sec-head">
              <h2 class="t-section">Pendidikan</h2>
              <VButton variant="secondary" size="compact" @click="addEducation">
                <VIcon name="plus" :size="13" />
                <span>Tambah</span>
              </VButton>
            </div>

            <div v-if="form.education.length > 0" class="v-item-stack">
              <div v-for="(edu, idx) in form.education" :key="idx" class="v-item-box">
                <div class="v-form-grid">
                  <input v-model="edu.institution" type="text" class="v-input" placeholder="Universitas / Politeknik / SMK" />
                  <input v-model="edu.degree" type="text" class="v-input" placeholder="Jurusan & Gelar (mis. S1 Akuntansi)" />
                  <input v-model="edu.period" type="text" class="v-input" placeholder="Tahun (mis. 2019 - 2023)" />
                  <input v-model="edu.notes" type="text" class="v-input" placeholder="IPK / Prestasi (opsional)" />
                </div>
                <div style="display: flex; justify-content: flex-end">
                  <VButton variant="tertiary" size="compact" @click="removeEducation(idx)">
                    Hapus Pendidikan
                  </VButton>
                </div>
              </div>
            </div>
          </VCard>
        </div>

        <aside class="v-cv-preview-col" aria-label="Pratinjau langsung CV A4">
          <div class="v-cv-preview-sticky">
            <CvA4Preview
              :cv="previewCv"
              :skills="userSkillNames"
              :highlighted-keywords="atsKeywordDiff?.matched"
              :lang="exportLang"
            />
          </div>
        </aside>
      </div>
    </div>
  </div>
</template>

<style scoped>
.v-cv-root {
  display: flex;
  flex-direction: column;
  flex: 1;
}
.v-page-container {
  max-width: 1200px;
  width: 100%;
  margin: 0 auto;
  padding: var(--space-8);
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}
.v-export-banner {
  padding: var(--space-4) var(--space-5);
  border-radius: var(--radius-card);
  border: 1px solid var(--brand-border);
  background: var(--brand-soft);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-cv-workspace {
  display: grid;
  grid-template-columns: 1.08fr 0.92fr;
  gap: var(--space-6);
  align-items: start;
}
.v-cv-editor-col,
.v-cv-preview-col {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
  min-width: 0;
}
.v-cv-preview-sticky {
  position: sticky;
  top: var(--space-5);
}
.v-target-select {
  max-width: 320px;
  width: 100%;
  height: 36px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--bg-primary);
  color: var(--text-1);
  font-size: 13px;
}
.v-diff-grid {
  margin-top: var(--space-4);
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-4);
}
.v-diff-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}
.v-diff-chip {
  font-size: 12px;
  font-weight: 600;
  padding: 4px 10px;
  border-radius: var(--radius-sm);
}
.v-diff-chip--ok {
  background: var(--filter-active-tint);
  border: 1px solid var(--filter-active-border);
  color: var(--status-success);
}
.v-diff-chip--miss {
  background: var(--warning-soft);
  border: 1px solid var(--warning-border);
  color: var(--status-warning);
}
.v-diff-chip--miss:hover {
  filter: brightness(1.08);
}
.v-cv-mobile-tabs {
  display: none;
  gap: 6px;
  padding: 4px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
}
.v-cv-mtab {
  flex: 1;
  height: 34px;
  border-radius: var(--radius-sm);
  border: 0;
  background: transparent;
  color: var(--text-2);
  font-size: 13px;
  font-weight: 600;
}
.v-cv-mtab--active {
  background: var(--brand);
  color: var(--brand-fg);
}
.v-sec-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}
.v-form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-3);
}
.v-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.v-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
}
.v-input,
.v-textarea {
  width: 100%;
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
}
.v-input {
  height: 38px;
  padding: 0 var(--space-3);
}
.v-textarea {
  min-height: 96px;
  padding: var(--space-3);
  resize: vertical;
  line-height: 1.55;
}
.v-textarea--sm {
  min-height: 76px;
}
.v-item-stack {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin-top: var(--space-4);
}
.v-item-box {
  padding: var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-divider);
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.v-lang-group {
  display: inline-flex;
  border-radius: var(--radius-control);
  border: 1px solid var(--border-default);
  background: var(--bg-primary);
  overflow: hidden;
}
.v-lang-btn {
  height: 36px;
  padding: 0 12px;
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
@media (max-width: 1023px) {
  .v-page-container {
    padding: var(--space-5) var(--space-4) var(--space-10);
  }
  .v-cv-mobile-tabs {
    display: flex;
  }
  .v-cv-workspace,
  .v-diff-grid,
  .v-form-grid {
    grid-template-columns: 1fr;
  }
  .v-cv-workspace[data-mobile-tab="edit"] .v-cv-preview-col {
    display: none;
  }
  .v-cv-workspace[data-mobile-tab="preview"] .v-cv-editor-col {
    display: none;
  }
}
</style>
