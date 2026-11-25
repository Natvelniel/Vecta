import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type {
  AtsExportResult,
  ConfirmedKeywordInput_Deserialize,
  ConfirmedSkillInput_Deserialize,
  ExperienceLevel,
  ImportedCvDraft,
  MasterCvProfile,
  Proficiency,
  SkillCategory,
  UserProfileDetail_Serialize,
} from "@/lib/bindings";
import { ipc, IpcError } from "@/lib/ipc";

export interface DraftSkillItem {
  canonical_name: string;
  category: SkillCategory;
  proficiency: Proficiency;
  evidence: string | null;
}

export interface DraftKeywordItem {
  id?: number;
  role_label: string;
  fit_type: "primary_fit" | "adjacent_fit" | "custom";
  scrape_queries: string[];
  match_reason: string | null;
  is_active: boolean;
}

export const useProfileStore = defineStore("profile", () => {
  const activeProfile = ref<UserProfileDetail_Serialize | null>(null);
  const rawStoryDraft = ref<string>("");
  const summaryDraft = ref<string>("");
  const experienceLevelDraft = ref<ExperienceLevel>("junior");
  const skillsDraft = ref<DraftSkillItem[]>([]);
  const keywordsDraft = ref<DraftKeywordItem[]>([]);
  const hasStructuredDraft = ref(false);
  const masterCv = ref<MasterCvProfile | null>(null);
  const pendingImportedCv = ref<MasterCvProfile | null>(null);
  const exportingPdf = ref(false);

  const loading = ref(false);
  const parsing = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);

  const userId = computed(() => activeProfile.value?.id ?? null);
  const activeKeywordsCount = computed(() =>
    (activeProfile.value?.keywords ?? keywordsDraft.value).filter((k) => k.is_active).length,
  );

  function hydrateDraftFromProfile(profile: UserProfileDetail_Serialize) {
    rawStoryDraft.value = profile.raw_story_text ?? "";
    summaryDraft.value = profile.summary ?? "";
    experienceLevelDraft.value = profile.experience_level;
    skillsDraft.value = profile.skills.map((s) => ({
      canonical_name: s.canonical_name,
      category: s.category,
      proficiency: s.proficiency,
      evidence: s.evidence,
    }));
    keywordsDraft.value = profile.keywords.map((k) => ({
      id: k.id,
      role_label: k.role_label,
      fit_type: k.fit_type,
      scrape_queries: [...k.scrape_queries],
      match_reason: k.match_reason,
      is_active: k.is_active,
    }));
    hasStructuredDraft.value = true;
  }

  async function loadActiveProfile() {
    loading.value = true;
    error.value = null;
    try {
      const profile = await ipc.getActiveProfile();
      activeProfile.value = profile;
      if (profile) {
        hydrateDraftFromProfile(profile);
      }
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memuat profil aktif.";
    } finally {
      loading.value = false;
    }
  }

  async function parseStory(): Promise<boolean> {
    if (rawStoryDraft.value.trim().length < 30) {
      error.value = "Cerita pengalaman minimal 30 karakter agar AI dapat memetakan skill secara akurat.";
      return false;
    }
    parsing.value = true;
    error.value = null;
    try {
      const draft = await ipc.parseUserStory(rawStoryDraft.value.trim());
      summaryDraft.value = draft.professional_summary;
      experienceLevelDraft.value = draft.estimated_experience_level;
      skillsDraft.value = draft.extracted_skills.map((s) => ({
        canonical_name: s.canonical_name,
        category: s.category,
        proficiency: s.proficiency,
        evidence: s.evidence,
      }));
      // Preserve existing custom user keywords across re-parsing
      const existingCustom = keywordsDraft.value.filter((k) => k.fit_type === "custom");
      const fresh = draft.recommended_keywords.map((k) => ({
        role_label: k.role_label,
        fit_type: k.fit_type,
        scrape_queries: [...k.scrape_queries],
        match_reason: k.match_reason,
        is_active: k.fit_type === "primary_fit",
      }));
      keywordsDraft.value = [...fresh, ...existingCustom];
      hasStructuredDraft.value = true;
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memetakan cerita profil.";
      return false;
    } finally {
      parsing.value = false;
    }
  }

  async function saveProfile(): Promise<boolean> {
    if (skillsDraft.value.length === 0) {
      error.value = "Tambahkan minimal satu skill sebelum menyimpan profil.";
      return false;
    }
    saving.value = true;
    error.value = null;
    try {
      const targetRoles = keywordsDraft.value
        .filter((k) => k.is_active)
        .map((k) => k.role_label);
      const skills: ConfirmedSkillInput_Deserialize[] = skillsDraft.value.map((s) => ({
        canonical_name: s.canonical_name,
        category: s.category,
        proficiency: s.proficiency,
        evidence: s.evidence,
      }));
      const keywords: ConfirmedKeywordInput_Deserialize[] = keywordsDraft.value.map((k) => ({
        role_label: k.role_label,
        fit_type: k.fit_type,
        scrape_queries: k.scrape_queries,
        match_reason: k.match_reason,
        is_active: k.is_active,
      }));
      const saved = await ipc.saveConfirmedProfile({
        id: activeProfile.value?.id ?? null,
        profile_name: pendingImportedCv.value?.full_name || activeProfile.value?.profile_name || "Profil Utama",
        target_roles: targetRoles.length > 0 ? targetRoles : ["Profesional Umum"],
        raw_story_text: rawStoryDraft.value.trim(),
        summary: summaryDraft.value.trim(),
        experience_level: experienceLevelDraft.value,
        skills,
        keywords,
      });
      activeProfile.value = saved;
      hydrateDraftFromProfile(saved);
      if (pendingImportedCv.value) {
        const cvToSave = { ...pendingImportedCv.value, user_id: saved.id };
        masterCv.value = await ipc.saveMasterCv(cvToSave);
        pendingImportedCv.value = null;
      }
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menyimpan profil.";
      return false;
    } finally {
      saving.value = false;
    }
  }

  async function toggleKeyword(index: number): Promise<boolean> {
    const item = keywordsDraft.value[index];
    if (!item) return false;
    const nextActive = !item.is_active;
    item.is_active = nextActive;

    if (activeProfile.value && typeof item.id === "number") {
      try {
        await ipc.toggleScrapeKeyword(item.id, nextActive);
        const liveKw = activeProfile.value.keywords.find((k) => k.id === item.id);
        if (liveKw) liveKw.is_active = nextActive;
        return true;
      } catch (e) {
        item.is_active = !nextActive;
        error.value = e instanceof IpcError ? e.message : "Gagal memperbarui kata kunci.";
        return false;
      }
    }
    return true;
  }

  async function addCustomKeyword(roleLabel: string): Promise<boolean> {
    const clean = roleLabel.trim();
    if (!clean) return false;
    if (activeProfile.value) {
      try {
        const created = await ipc.addCustomKeyword(activeProfile.value.id, clean, [clean]);
        activeProfile.value.keywords.push(created);
        keywordsDraft.value.push({
          id: created.id,
          role_label: created.role_label,
          fit_type: created.fit_type,
          scrape_queries: [...created.scrape_queries],
          match_reason: created.match_reason,
          is_active: created.is_active,
        });
        return true;
      } catch (e) {
        error.value = e instanceof IpcError ? e.message : "Gagal menambahkan peran kustom.";
        return false;
      }
    }
    keywordsDraft.value.push({
      role_label: clean,
      fit_type: "custom",
      scrape_queries: [clean],
      match_reason: "Ditambahkan manual oleh pengguna.",
      is_active: true,
    });
    return true;
  }

  function hydrateFromImportedDraft(draft: ImportedCvDraft) {
    rawStoryDraft.value = draft.extracted_text;
    summaryDraft.value = draft.professional_summary;
    experienceLevelDraft.value = draft.estimated_experience_level;
    skillsDraft.value = draft.extracted_skills.map((s) => ({
      canonical_name: s.canonical_name,
      category: s.category,
      proficiency: s.proficiency,
      evidence: s.evidence,
    }));
    const existingCustom = keywordsDraft.value.filter((k) => k.fit_type === "custom");
    const fresh = draft.recommended_keywords.map((k) => ({
      role_label: k.role_label,
      fit_type: k.fit_type,
      scrape_queries: [...k.scrape_queries],
      match_reason: k.match_reason,
      is_active: k.fit_type === "primary_fit",
    }));
    keywordsDraft.value = [...fresh, ...existingCustom];
    pendingImportedCv.value = draft.master_cv;
    hasStructuredDraft.value = true;
  }

  async function importCvPdf(pdfBase64: string): Promise<ImportedCvDraft | null> {
    parsing.value = true;
    error.value = null;
    try {
      const draft = await ipc.importCvPdf(pdfBase64);
      hydrateFromImportedDraft(draft);
      return draft;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal membaca file CV PDF.";
      return null;
    } finally {
      parsing.value = false;
    }
  }

  async function parseStoryLocal(): Promise<boolean> {
    if (rawStoryDraft.value.trim().length < 30) {
      error.value = "Cerita pengalaman minimal 30 karakter agar Vecta dapat memetakan skill.";
      return false;
    }
    parsing.value = true;
    error.value = null;
    try {
      const draft = await ipc.parseStoryLocal(rawStoryDraft.value.trim());
      hydrateFromImportedDraft(draft);
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memetakan cerita secara lokal.";
      return false;
    } finally {
      parsing.value = false;
    }
  }

  async function loadMasterCv(uid: string): Promise<MasterCvProfile | null> {
    try {
      masterCv.value = await ipc.getMasterCv(uid);
      return masterCv.value;
    } catch {
      return null;
    }
  }

  async function saveMasterCv(cv: MasterCvProfile): Promise<boolean> {
    saving.value = true;
    error.value = null;
    try {
      masterCv.value = await ipc.saveMasterCv(cv);
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menyimpan Master CV.";
      return false;
    } finally {
      saving.value = false;
    }
  }

  async function exportAtsPdf(
    uid: string,
    jobId: string | null,
    language: "id" | "en" = "id",
  ): Promise<AtsExportResult | null> {
    exportingPdf.value = true;
    error.value = null;
    try {
      return await ipc.exportTailoredAtsPdf(uid, jobId, language);
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal mengekspor CV ATS PDF.";
      return null;
    } finally {
      exportingPdf.value = false;
    }
  }

  return {
    activeProfile,
    userId,
    activeKeywordsCount,
    rawStoryDraft,
    summaryDraft,
    experienceLevelDraft,
    skillsDraft,
    keywordsDraft,
    hasStructuredDraft,
    masterCv,
    exportingPdf,
    loading,
    parsing,
    saving,
    error,
    loadActiveProfile,
    parseStory,
    parseStoryLocal,
    importCvPdf,
    saveProfile,
    toggleKeyword,
    addCustomKeyword,
    loadMasterCv,
    saveMasterCv,
    exportAtsPdf,
  };
});
