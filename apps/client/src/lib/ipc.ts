import {
  commands,
  type AiMatchRunReport,
  type ApiKeyStatus_Serialize,
  type AppError,
  type ApplicationStatus,
  type ApplyKitResult,
  type AtsExportResult,
  type CompanySnapshot,
  type CompetencyMapResponse_Serialize,
  type CompetencySkillItem_Serialize,
  type ConfirmedProfileInput_Deserialize,
  type ErrorCode,
  type GapAnalysis,
  type ImportedCvDraft,
  type InterviewPrepKit,
  type JobFilterInput_Deserialize,
  type JobMatchDetail_Serialize,
  type JobMatchSummary_Serialize,
  type LocalAiModelStatus,
  type MasterCvProfile,
  type ProdiRoleItem,
  type ProfileSummaryItem,
  type RoadmapDetail_Serialize,
  type RoadmapSummary_Serialize,
  type RoleJobCount,
  type SalaryBenchmarkSummary,
  type SavedWatchlistItem_Serialize,
  type ScrapeKeywordItem_Serialize,
  type ScrapeRunReport_Serialize,
  type SearchPreferences_Deserialize,
  type SearchPreferences_Serialize,
  type SkillImpactItem,
  type StepStatus,
  type StepUpdateResult_Serialize,
  type StoryProfileDraft,
  type UserProfileDetail_Serialize,
  type VectaBackupReport,
  type VectaImportReport,
  type WorkerHealth,
} from "./bindings";
import { safeNumber } from "./format";

export interface ScrapeProgressEvent {
  stage: string;
  message: string;
  current: number;
  total: number;
}

export interface RerankCompleteEvent {
  user_id: string;
  reranked_count: number;
}

export type NormalizedJobSummary = Omit<
  JobMatchSummary_Serialize,
  "vector_score" | "skill_overlap_score" | "level_score" | "final_score" | "domain_score" | "reranker_score"
> & {
  vector_score: number;
  skill_overlap_score: number;
  level_score: number;
  final_score: number;
  domain_score: number | null;
  reranker_score: number | null;
};

export type NormalizedJobDetail = Omit<
  JobMatchDetail_Serialize,
  "vector_score" | "skill_overlap_score" | "level_score" | "final_score" | "domain_score" | "reranker_score"
> & {
  vector_score: number;
  skill_overlap_score: number;
  level_score: number;
  final_score: number;
  domain_score: number | null;
  reranker_score: number | null;
};

export type NormalizedCompetencySkill = Omit<
  CompetencySkillItem_Serialize,
  "demand_percentage"
> & {
  demand_percentage: number;
};

export type NormalizedCompetencyMap = Omit<
  CompetencyMapResponse_Serialize,
  "skills"
> & {
  skills: NormalizedCompetencySkill[];
};

export type NormalizedRoadmapSummary = Omit<
  RoadmapSummary_Serialize,
  "progress_percentage"
> & {
  progress_percentage: number;
};

export class IpcError extends Error {
  readonly code: ErrorCode;
  readonly retryable: boolean;

  constructor(err: AppError) {
    super(err.message || friendlyErrorMessage(err.code));
    this.name = "IpcError";
    this.code = err.code;
    this.retryable = err.retryable;
  }
}

export function friendlyErrorMessage(code: ErrorCode): string {
  switch (code) {
    case "API_KEY_MISSING":
      return "Kunci API Gemini belum tersimpan. Hubungkan dulu melalui Pengaturan.";
    case "API_KEY_INVALID":
      return "Kunci API Gemini tidak valid atau ditolak. Periksa kembali kunci dari Google AI Studio.";
    case "GEMINI_RATE_LIMITED":
      return "Kuota permintaan Gemini sedang penuh. Tunggu sebentar lalu coba lagi.";
    case "GEMINI_PARSE_FAILED":
      return "Respons AI belum bisa diproses. Coba ulangi sekali lagi.";
    case "SCRAPER_UNREACHABLE":
      return "Python Scraper Worker lokal tidak merespons. Jalankan worker lokal atau gunakan Tempel Manual.";
    case "DB_ERROR":
      return "Gagal membaca atau menyimpan data lokal SQLite.";
    case "VALIDATION_ERROR":
      return "Data yang dimasukkan belum lengkap atau tidak sesuai format.";
  }
}

function ratioToPercent(value: number | null | undefined): number {
  const n = safeNumber(value);
  // Rust matching engine outputs normalized 0.0..=1.0 ratios; clamp and scale to 0..100%.
  return Math.max(0, Math.min(100, n * 100));
}

function optionalRatioToPercent(value: number | null | undefined): number | null {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return null;
  }
  return Math.max(0, Math.min(100, value * 100));
}

export function normalizeJobSummary(raw: JobMatchSummary_Serialize): NormalizedJobSummary {
  return {
    ...raw,
    vector_score: ratioToPercent(raw.vector_score),
    skill_overlap_score: ratioToPercent(raw.skill_overlap_score),
    level_score: ratioToPercent(raw.level_score),
    final_score: ratioToPercent(raw.final_score),
    domain_score: optionalRatioToPercent(raw.domain_score),
    reranker_score: optionalRatioToPercent(raw.reranker_score),
    match_explanation: raw.match_explanation ?? [],
  };
}

export function normalizeJobDetail(raw: JobMatchDetail_Serialize): NormalizedJobDetail {
  return {
    ...raw,
    vector_score: ratioToPercent(raw.vector_score),
    skill_overlap_score: ratioToPercent(raw.skill_overlap_score),
    level_score: ratioToPercent(raw.level_score),
    final_score: ratioToPercent(raw.final_score),
    domain_score: optionalRatioToPercent(raw.domain_score),
    reranker_score: optionalRatioToPercent(raw.reranker_score),
    match_explanation: raw.match_explanation ?? [],
  };
}

export function normalizeCompetencyMap(
  raw: CompetencyMapResponse_Serialize,
): NormalizedCompetencyMap {
  return {
    ...raw,
    skills: raw.skills.map((s) => ({
      ...s,
      demand_percentage: safeNumber(s.demand_percentage),
    })),
  };
}

export function normalizeRoadmapSummary(
  raw: RoadmapSummary_Serialize,
): NormalizedRoadmapSummary {
  return {
    ...raw,
    progress_percentage: safeNumber(raw.progress_percentage),
  };
}

type ResultEnvelope<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };

let onApiKeyMissingListener: (() => void) | null = null;

export function registerApiKeyMissingHandler(handler: () => void): void {
  onApiKeyMissingListener = handler;
}

export function unwrapResult<T>(res: ResultEnvelope<T>): T {
  if (res.status === "ok") {
    return res.data;
  }
  if (res.error.code === "API_KEY_MISSING" && onApiKeyMissingListener) {
    onApiKeyMissingListener();
  }
  throw new IpcError(res.error);
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function isMockMode(): boolean {
  return (
    import.meta.env.DEV &&
    import.meta.env.VITE_MOCK_IPC === "1" &&
    !isTauriRuntime()
  );
}

async function getMockBackend() {
  const mod = await import("./mockIpc");
  return mod.mockCommands;
}

export const ipc = {
  async getApiKeyStatus(): Promise<ApiKeyStatus_Serialize> {
    if (isMockMode()) return (await getMockBackend()).getApiKeyStatus();
    return unwrapResult(await commands.getApiKeyStatus());
  },

  async saveAndTestApiKey(apiKey: string): Promise<ApiKeyStatus_Serialize> {
    if (isMockMode()) return (await getMockBackend()).saveAndTestApiKey(apiKey);
    return unwrapResult(await commands.saveAndTestApiKey(apiKey));
  },

  async deleteApiKey(): Promise<null> {
    if (isMockMode()) return (await getMockBackend()).deleteApiKey();
    return unwrapResult(await commands.deleteApiKey());
  },

  async checkWorkerHealth(workerUrl: string | null): Promise<WorkerHealth> {
    if (isMockMode()) return (await getMockBackend()).checkWorkerHealth(workerUrl);
    return unwrapResult(await commands.checkWorkerHealth(workerUrl));
  },

  async getSearchPreferences(): Promise<SearchPreferences_Serialize> {
    if (isMockMode()) return (await getMockBackend()).getSearchPreferences();
    return unwrapResult(await commands.getSearchPreferences());
  },

  async saveSearchPreferences(prefs: SearchPreferences_Deserialize): Promise<SearchPreferences_Serialize> {
    if (isMockMode()) return (await getMockBackend()).saveSearchPreferences(prefs);
    return unwrapResult(await commands.saveSearchPreferences(prefs));
  },

  async exportVectaBackup(): Promise<VectaBackupReport> {
    if (isMockMode()) return (await getMockBackend()).exportVectaBackup();
    return unwrapResult(await commands.exportVectaBackup());
  },

  async importVectaBackup(backupJson: string): Promise<VectaImportReport> {
    if (isMockMode()) return (await getMockBackend()).importVectaBackup(backupJson);
    return unwrapResult(await commands.importVectaBackup(backupJson));
  },

  async parseUserStory(rawStory: string): Promise<StoryProfileDraft> {
    if (isMockMode()) return (await getMockBackend()).parseUserStory(rawStory);
    return unwrapResult(await commands.parseUserStory(rawStory));
  },

  async saveConfirmedProfile(
    input: ConfirmedProfileInput_Deserialize,
  ): Promise<UserProfileDetail_Serialize> {
    if (isMockMode()) return (await getMockBackend()).saveConfirmedProfile(input);
    return unwrapResult(await commands.saveConfirmedProfile(input));
  },

  async getActiveProfile(): Promise<UserProfileDetail_Serialize | null> {
    if (isMockMode()) return (await getMockBackend()).getActiveProfile();
    return unwrapResult(await commands.getActiveProfile());
  },

  async toggleScrapeKeyword(keywordId: number, isActive: boolean): Promise<null> {
    if (isMockMode()) return (await getMockBackend()).toggleScrapeKeyword(keywordId, isActive);
    return unwrapResult(await commands.toggleScrapeKeyword(keywordId, isActive));
  },

  async addCustomKeyword(
    userId: string,
    roleLabel: string,
    queries: string[],
  ): Promise<ScrapeKeywordItem_Serialize> {
    if (isMockMode()) {
      return (await getMockBackend()).addCustomKeyword(userId, roleLabel, queries);
    }
    return unwrapResult(await commands.addCustomKeyword(userId, roleLabel, queries));
  },

  async listProdiRolesCatalog(query: string | null = null): Promise<ProdiRoleItem[]> {
    if (isMockMode()) return (await getMockBackend()).listProdiRolesCatalog(query);
    return unwrapResult(await commands.listProdiRolesCatalog(query));
  },

  async listUserProfiles(): Promise<ProfileSummaryItem[]> {
    if (isMockMode()) return (await getMockBackend()).listUserProfiles();
    return unwrapResult(await commands.listUserProfiles());
  },

  async switchActiveProfile(profileId: string): Promise<UserProfileDetail_Serialize> {
    if (isMockMode()) return (await getMockBackend()).switchActiveProfile(profileId);
    return unwrapResult(await commands.switchActiveProfile(profileId));
  },

  async deleteUserProfile(profileId: string): Promise<UserProfileDetail_Serialize> {
    if (isMockMode()) return (await getMockBackend()).deleteUserProfile(profileId);
    return unwrapResult(await commands.deleteUserProfile(profileId));
  },

  async ingestManualJob(
    userId: string,
    titleHint: string | null,
    url: string | null,
    rawText: string,
  ): Promise<NormalizedJobDetail> {
    if (isMockMode()) {
      return normalizeJobDetail(
        await (await getMockBackend()).ingestManualJob(userId, titleHint, url, rawText),
      );
    }
    return normalizeJobDetail(
      unwrapResult(await commands.ingestManualJob(userId, titleHint, url, rawText)),
    );
  },

  async runScrapeForActiveKeywords(
    userId: string,
    sources: string[],
    limitPerQuery: number,
  ): Promise<ScrapeRunReport_Serialize> {
    if (isMockMode()) {
      return (await getMockBackend()).runScrapeForActiveKeywords(userId, sources, limitPerQuery);
    }
    return unwrapResult(await commands.runScrapeForActiveKeywords(userId, sources, limitPerQuery));
  },

  async scrapeAndStoreJobs(
    userId: string,
    prefs: SearchPreferences_Deserialize,
  ): Promise<ScrapeRunReport_Serialize> {
    if (isMockMode()) {
      return (await getMockBackend()).scrapeAndStoreJobs(userId, prefs);
    }
    return unwrapResult(await commands.scrapeAndStoreJobs(userId, prefs));
  },

  async runAiMatchingQueue(
    userId: string,
    maxJobs: number,
    rpmLimit: number,
  ): Promise<AiMatchRunReport> {
    if (isMockMode()) {
      return (await getMockBackend()).runAiMatchingQueue(userId, maxJobs, rpmLimit);
    }
    return unwrapResult(await commands.runAiMatchingQueue(userId, maxJobs, rpmLimit));
  },

  async enrichJobWithAi(userId: string, jobId: string): Promise<NormalizedJobDetail> {
    const raw = isMockMode()
      ? await (await getMockBackend()).enrichJobWithAi(userId, jobId)
      : unwrapResult(await commands.enrichJobWithAi(userId, jobId));
    return normalizeJobDetail(raw);
  },

  async enrichJobDescriptionFromSource(
    userId: string,
    jobId: string,
  ): Promise<NormalizedJobDetail> {
    const raw = isMockMode()
      ? await (await getMockBackend()).enrichJobDescriptionFromSource(userId, jobId)
      : unwrapResult(await commands.enrichJobDescriptionFromSource(userId, jobId));
    return normalizeJobDetail(raw);
  },

  async updateJobApplication(
    userId: string,
    jobId: string,
    status: ApplicationStatus,
    notes: string | null,
  ): Promise<NormalizedJobDetail> {
    const raw = isMockMode()
      ? await (await getMockBackend()).updateJobApplication(userId, jobId, status, notes)
      : unwrapResult(await commands.updateJobApplication(userId, jobId, status, notes));
    return normalizeJobDetail(raw);
  },

  async generateApplyKit(
    userId: string,
    jobId: string,
    language: "id" | "en",
    forceRefresh: boolean,
  ): Promise<ApplyKitResult> {
    if (isMockMode()) {
      return (await getMockBackend()).generateApplyKit(userId, jobId, language, forceRefresh);
    }
    return unwrapResult(await commands.generateApplyKit(userId, jobId, language, forceRefresh));
  },

  async listRankedJobs(
    userId: string,
    filter: JobFilterInput_Deserialize,
  ): Promise<NormalizedJobSummary[]> {
    const raw = isMockMode()
      ? await (await getMockBackend()).listRankedJobs(userId, filter)
      : unwrapResult(await commands.listRankedJobs(userId, filter));
    return raw.map(normalizeJobSummary);
  },

  async getJobDetail(userId: string, jobId: string): Promise<NormalizedJobDetail> {
    const raw = isMockMode()
      ? await (await getMockBackend()).getJobDetail(userId, jobId)
      : unwrapResult(await commands.getJobDetail(userId, jobId));
    return normalizeJobDetail(raw);
  },

  async toggleSaveJob(userId: string, jobId: string): Promise<boolean> {
    if (isMockMode()) return (await getMockBackend()).toggleSaveJob(userId, jobId);
    return unwrapResult(await commands.toggleSaveJob(userId, jobId));
  },

  async listSavedWatchlists(userId: string): Promise<SavedWatchlistItem_Serialize[]> {
    if (isMockMode()) return (await getMockBackend()).listSavedWatchlists(userId);
    return unwrapResult(await commands.listSavedWatchlists(userId));
  },

  async saveWatchlist(
    userId: string,
    name: string,
    filter: JobFilterInput_Deserialize,
  ): Promise<SavedWatchlistItem_Serialize> {
    if (isMockMode()) return (await getMockBackend()).saveWatchlist(userId, name, filter);
    return unwrapResult(await commands.saveWatchlist(userId, name, filter));
  },

  async deleteWatchlist(id: string): Promise<null> {
    if (isMockMode()) return (await getMockBackend()).deleteWatchlist(id);
    return unwrapResult(await commands.deleteWatchlist(id));
  },

  async generateGapAnalysis(
    userId: string,
    jobId: string,
    forceRefresh: boolean,
  ): Promise<GapAnalysis> {
    if (isMockMode()) {
      return (await getMockBackend()).generateGapAnalysis(userId, jobId, forceRefresh);
    }
    return unwrapResult(await commands.generateGapAnalysis(userId, jobId, forceRefresh));
  },

  async listAvailableRoles(): Promise<RoleJobCount[]> {
    if (isMockMode()) return (await getMockBackend()).listAvailableRoles();
    return unwrapResult(await commands.listAvailableRoles());
  },

  async getCompetencyMap(
    userId: string,
    targetRole: string,
  ): Promise<NormalizedCompetencyMap> {
    const raw = isMockMode()
      ? await (await getMockBackend()).getCompetencyMap(userId, targetRole)
      : unwrapResult(await commands.getCompetencyMap(userId, targetRole));
    return normalizeCompetencyMap(raw);
  },

  async simulateSkillImpact(
    userId: string,
    targetRole: string,
    candidateSkillIds: number[],
  ): Promise<SkillImpactItem[]> {
    if (isMockMode()) {
      return (await getMockBackend()).simulateSkillImpact(userId, targetRole, candidateSkillIds);
    }
    return unwrapResult(await commands.simulateSkillImpact(userId, targetRole, candidateSkillIds));
  },

  async getSalaryBenchmark(
    roleFilter: string | null,
    clusterFilter: string | null,
  ): Promise<SalaryBenchmarkSummary> {
    if (isMockMode()) {
      return (await getMockBackend()).getSalaryBenchmark(roleFilter, clusterFilter);
    }
    return unwrapResult(await commands.getSalaryBenchmark(roleFilter, clusterFilter));
  },

  async getCompanySnapshot(userId: string, companyName: string): Promise<CompanySnapshot> {
    if (isMockMode()) {
      return (await getMockBackend()).getCompanySnapshot(userId, companyName);
    }
    return unwrapResult(await commands.getCompanySnapshot(userId, companyName));
  },

  async generateInterviewPrep(
    userId: string,
    jobId: string,
    language: "id" | "en" = "id",
  ): Promise<InterviewPrepKit> {
    if (isMockMode()) {
      return (await getMockBackend()).generateInterviewPrep(userId, jobId, language);
    }
    return unwrapResult(await commands.generateInterviewPrep(userId, jobId, language));
  },

  async generateLearningRoadmap(
    userId: string,
    targetRole: string,
    sourceJobId: string | null,
    hoursPerWeek: number,
  ): Promise<RoadmapDetail_Serialize> {
    if (isMockMode()) {
      return (await getMockBackend()).generateLearningRoadmap(
        userId,
        targetRole,
        sourceJobId,
        hoursPerWeek,
      );
    }
    return unwrapResult(
      await commands.generateLearningRoadmap(userId, targetRole, sourceJobId, hoursPerWeek),
    );
  },

  async listRoadmaps(userId: string): Promise<NormalizedRoadmapSummary[]> {
    const raw = isMockMode()
      ? await (await getMockBackend()).listRoadmaps(userId)
      : unwrapResult(await commands.listRoadmaps(userId));
    return raw.map(normalizeRoadmapSummary);
  },

  async getRoadmapDetail(roadmapId: string): Promise<RoadmapDetail_Serialize> {
    if (isMockMode()) return (await getMockBackend()).getRoadmapDetail(roadmapId);
    return unwrapResult(await commands.getRoadmapDetail(roadmapId));
  },

  async updateRoadmapStepStatus(
    stepId: string,
    status: StepStatus,
  ): Promise<StepUpdateResult_Serialize> {
    if (isMockMode()) return (await getMockBackend()).updateRoadmapStepStatus(stepId, status);
    return unwrapResult(await commands.updateRoadmapStepStatus(stepId, status));
  },

  async quickAddUserSkill(userId: string, skillId: number): Promise<null> {
    if (isMockMode()) return (await getMockBackend()).quickAddUserSkill(userId, skillId);
    return unwrapResult(await commands.quickAddUserSkill(userId, skillId));
  },

  async getLocalAiStatus(): Promise<LocalAiModelStatus> {
    if (isMockMode()) return (await getMockBackend()).getLocalAiStatus();
    return unwrapResult(await commands.getLocalAiStatus());
  },

  async getScraperMode(): Promise<string> {
    if (isMockMode()) return (await getMockBackend()).getScraperMode();
    return unwrapResult(await commands.getScraperMode());
  },

  async setScraperMode(mode: string): Promise<string> {
    if (isMockMode()) return (await getMockBackend()).setScraperMode(mode);
    return unwrapResult(await commands.setScraperMode(mode));
  },

  async importCvPdf(pdfBase64: string): Promise<ImportedCvDraft> {
    if (isMockMode()) return (await getMockBackend()).importCvPdf(pdfBase64);
    return unwrapResult(await commands.importCvPdf(pdfBase64));
  },

  async parseStoryLocal(rawStory: string): Promise<ImportedCvDraft> {
    if (isMockMode()) return (await getMockBackend()).parseStoryLocal(rawStory);
    return unwrapResult(await commands.parseStoryLocal(rawStory));
  },

  async getMasterCv(userId: string): Promise<MasterCvProfile> {
    if (isMockMode()) return (await getMockBackend()).getMasterCv(userId);
    return unwrapResult(await commands.getMasterCv(userId));
  },

  async saveMasterCv(cv: MasterCvProfile): Promise<MasterCvProfile> {
    if (isMockMode()) return (await getMockBackend()).saveMasterCv(cv);
    return unwrapResult(await commands.saveMasterCv(cv));
  },

  async exportTailoredAtsPdf(
    userId: string,
    jobId: string | null,
    language: "id" | "en",
  ): Promise<AtsExportResult> {
    if (isMockMode()) {
      return (await getMockBackend()).exportTailoredAtsPdf(userId, jobId, language);
    }
    return unwrapResult(await commands.exportTailoredAtsPdf(userId, jobId, language));
  },

  async rerankTopJobs(userId: string, topN: number | null = 25): Promise<number> {
    if (isMockMode()) {
      return (await getMockBackend()).rerankTopJobs(userId, topN);
    }
    return unwrapResult(await commands.rerankTopJobs(userId, topN));
  },
};

export async function openExternalUrl(url: string): Promise<void> {
  if (!url) return;
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return;
  }
  if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
    return;
  }
  if (isTauriRuntime()) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(parsed.toString());
    return;
  }
  window.open(parsed.toString(), "_blank", "noopener,noreferrer");
}

export async function listenScrapeProgress(
  handler: (event: ScrapeProgressEvent) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<ScrapeProgressEvent>("scrape-progress", (evt) => {
    handler(evt.payload);
  });
  return unlisten;
}

export async function listenRerankComplete(
  handler: (event: RerankCompleteEvent) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<RerankCompleteEvent>("rerank-complete", (evt) => {
    handler(evt.payload);
  });
  return unlisten;
}
