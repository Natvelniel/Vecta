import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockCommands } from "@/lib/mockIpc";
import {
  ipc,
  normalizeCompetencyMap,
  normalizeJobDetail,
  normalizeJobSummary,
  normalizeRoadmapSummary,
} from "@/lib/ipc";
import { useProfileStore } from "./profile";
import { useJobsStore } from "./jobs";
import { useCompetencyStore } from "./competency";
import { useRoadmapStore } from "./roadmap";
import { useSettingsStore } from "./settings";

describe("Vecta end-to-end store logic & cross-store loop", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.restoreAllMocks();

    vi.spyOn(ipc, "getApiKeyStatus").mockImplementation(() => mockCommands.getApiKeyStatus());
    vi.spyOn(ipc, "saveAndTestApiKey").mockImplementation((k) => mockCommands.saveAndTestApiKey(k));
    vi.spyOn(ipc, "deleteApiKey").mockImplementation(() => mockCommands.deleteApiKey());
    vi.spyOn(ipc, "checkWorkerHealth").mockImplementation((u) => mockCommands.checkWorkerHealth(u));
    vi.spyOn(ipc, "getLocalAiStatus").mockImplementation(() => mockCommands.getLocalAiStatus());
    vi.spyOn(ipc, "getScraperMode").mockImplementation(() => mockCommands.getScraperMode());
    vi.spyOn(ipc, "setScraperMode").mockImplementation((m) => mockCommands.setScraperMode(m));
    vi.spyOn(ipc, "importCvPdf").mockImplementation((b) => mockCommands.importCvPdf(b));
    vi.spyOn(ipc, "parseStoryLocal").mockImplementation((s) => mockCommands.parseStoryLocal(s));
    vi.spyOn(ipc, "getMasterCv").mockImplementation((u) => mockCommands.getMasterCv(u));
    vi.spyOn(ipc, "saveMasterCv").mockImplementation((c) => mockCommands.saveMasterCv(c));
    vi.spyOn(ipc, "exportTailoredAtsPdf").mockImplementation((u, j, l) =>
      mockCommands.exportTailoredAtsPdf(u, j, l),
    );
    vi.spyOn(ipc, "getSearchPreferences").mockImplementation(() =>
      mockCommands.getSearchPreferences(),
    );
    vi.spyOn(ipc, "saveSearchPreferences").mockImplementation((p) =>
      mockCommands.saveSearchPreferences(p),
    );
    vi.spyOn(ipc, "getActiveProfile").mockImplementation(() => mockCommands.getActiveProfile());
    vi.spyOn(ipc, "parseUserStory").mockImplementation((s) => mockCommands.parseUserStory(s));
    vi.spyOn(ipc, "saveConfirmedProfile").mockImplementation((i) =>
      mockCommands.saveConfirmedProfile(i),
    );
    vi.spyOn(ipc, "toggleScrapeKeyword").mockImplementation((id, a) =>
      mockCommands.toggleScrapeKeyword(id, a),
    );
    vi.spyOn(ipc, "addCustomKeyword").mockImplementation((u, r, q) =>
      mockCommands.addCustomKeyword(u, r, q),
    );
    vi.spyOn(ipc, "listRankedJobs").mockImplementation(async (u, f) =>
      (await mockCommands.listRankedJobs(u, f)).map(normalizeJobSummary),
    );
    vi.spyOn(ipc, "getJobDetail").mockImplementation(async (u, j) =>
      normalizeJobDetail(await mockCommands.getJobDetail(u, j)),
    );
    vi.spyOn(ipc, "toggleSaveJob").mockImplementation((u, j) => mockCommands.toggleSaveJob(u, j));
    vi.spyOn(ipc, "scrapeAndStoreJobs").mockImplementation((u, p) =>
      mockCommands.scrapeAndStoreJobs(u, p),
    );
    vi.spyOn(ipc, "runAiMatchingQueue").mockImplementation((u, m, r) =>
      mockCommands.runAiMatchingQueue(u, m, r),
    );
    vi.spyOn(ipc, "enrichJobWithAi").mockImplementation(async (u, j) =>
      normalizeJobDetail(await mockCommands.enrichJobWithAi(u, j)),
    );
    vi.spyOn(ipc, "updateJobApplication").mockImplementation(async (u, j, s, n) =>
      normalizeJobDetail(await mockCommands.updateJobApplication(u, j, s, n)),
    );
    vi.spyOn(ipc, "generateApplyKit").mockImplementation((u, j, l, f) =>
      mockCommands.generateApplyKit(u, j, l, f),
    );
    vi.spyOn(ipc, "ingestManualJob").mockImplementation(async (u, t, url, txt) =>
      normalizeJobDetail(await mockCommands.ingestManualJob(u, t, url, txt)),
    );
    vi.spyOn(ipc, "generateGapAnalysis").mockImplementation((u, j, f) =>
      mockCommands.generateGapAnalysis(u, j, f),
    );
    vi.spyOn(ipc, "listSavedWatchlists").mockImplementation((u) =>
      mockCommands.listSavedWatchlists(u),
    );
    vi.spyOn(ipc, "saveWatchlist").mockImplementation((u, n, f) =>
      mockCommands.saveWatchlist(u, n, f),
    );
    vi.spyOn(ipc, "deleteWatchlist").mockImplementation((id) =>
      mockCommands.deleteWatchlist(id),
    );
    vi.spyOn(ipc, "listProdiRolesCatalog").mockImplementation((q = null) =>
      mockCommands.listProdiRolesCatalog(q ?? null),
    );
    vi.spyOn(ipc, "listUserProfiles").mockImplementation(() => mockCommands.listUserProfiles());
    vi.spyOn(ipc, "switchActiveProfile").mockImplementation((id) =>
      mockCommands.switchActiveProfile(id),
    );
    vi.spyOn(ipc, "deleteUserProfile").mockImplementation((id) =>
      mockCommands.deleteUserProfile(id),
    );
    vi.spyOn(ipc, "getSalaryBenchmark").mockImplementation((r, c) =>
      mockCommands.getSalaryBenchmark(r, c),
    );
    vi.spyOn(ipc, "getCompanySnapshot").mockImplementation((u, c) =>
      mockCommands.getCompanySnapshot(u, c),
    );
    vi.spyOn(ipc, "generateInterviewPrep").mockImplementation((u, j, l) =>
      mockCommands.generateInterviewPrep(u, j, l),
    );
    vi.spyOn(ipc, "simulateSkillImpact").mockImplementation((u, r, ids) =>
      mockCommands.simulateSkillImpact(u, r, ids),
    );
    vi.spyOn(ipc, "exportVectaBackup").mockImplementation(() => mockCommands.exportVectaBackup());
    vi.spyOn(ipc, "importVectaBackup").mockImplementation((j) => mockCommands.importVectaBackup(j));
    vi.spyOn(ipc, "listAvailableRoles").mockImplementation(() => mockCommands.listAvailableRoles());
    vi.spyOn(ipc, "getCompetencyMap").mockImplementation(async (u, r) =>
      normalizeCompetencyMap(await mockCommands.getCompetencyMap(u, r)),
    );
    vi.spyOn(ipc, "listRoadmaps").mockImplementation(async (u) =>
      (await mockCommands.listRoadmaps(u)).map(normalizeRoadmapSummary),
    );
    vi.spyOn(ipc, "getRoadmapDetail").mockImplementation((id) => mockCommands.getRoadmapDetail(id));
    vi.spyOn(ipc, "updateRoadmapStepStatus").mockImplementation((id, s) =>
      mockCommands.updateRoadmapStepStatus(id, s),
    );
    vi.spyOn(ipc, "quickAddUserSkill").mockImplementation((u, s) =>
      mockCommands.quickAddUserSkill(u, s),
    );
  });

  it("validates minimum story length and preserves custom keywords across re-parsing", async () => {
    const profile = useProfileStore();
    await profile.loadActiveProfile();
    expect(profile.activeProfile?.id).toBe("user-demo-1");

    profile.rawStoryDraft = "terlalu pendek";
    const badOk = await profile.parseStory();
    expect(badOk).toBe(false);
    expect(profile.error).toContain("30 karakter");

    await profile.addCustomKeyword("Security Engineer");
    expect(profile.keywordsDraft.some((k) => k.role_label === "Security Engineer")).toBe(true);

    profile.rawStoryDraft =
      "Saya membangun aplikasi desktop Rust, SQLite, dan backend Python FastAPI dengan pengujian otomatis.";
    const goodOk = await profile.parseStory();
    expect(goodOk).toBe(true);
    expect(profile.keywordsDraft.some((k) => k.role_label === "Security Engineer")).toBe(true);
  });

  it("ranks jobs, toggles bookmark, ingests manual job, and caches gap analysis", async () => {
    const profile = useProfileStore();
    const jobs = useJobsStore();
    await profile.loadActiveProfile();
    const uid = profile.userId!;

    await jobs.loadJobs(uid);
    expect(jobs.jobs.length).toBeGreaterThanOrEqual(3);
    expect(jobs.selectedDetail?.job_id).toBe(jobs.jobs[0].job_id);

    const beforeSaved = jobs.selectedDetail!.is_saved;
    const afterSaved = await jobs.toggleSave(uid, jobs.selectedDetail!.job_id);
    expect(afterSaved).toBe(!beforeSaved);

    const manual = await jobs.ingestManual(
      uid,
      "Rust Desktop Engineer",
      "https://example.com/job/rust",
      "Dicari Rust Desktop Engineer yang berpengalaman membangun aplikasi Tauri, SQLite, dan Docker.",
    );
    expect(manual).not.toBeNull();
    expect(manual?.title).toBe("Rust Desktop Engineer");

    const gapOk = await jobs.runGapAnalysis(uid, manual!.job_id, false);
    expect(gapOk).toBe(true);
  });

  it("closes the loop: roadmap step completion suggests skill -> quickAdd recalculates job match & competency map", async () => {
    const profile = useProfileStore();
    const jobs = useJobsStore();
    const comp = useCompetencyStore();
    const roadmap = useRoadmapStore();
    await profile.loadActiveProfile();
    const uid = profile.userId!;

    await jobs.loadJobs(uid);
    const job1Before = await ipc.getJobDetail(uid, "job-1");
    expect(job1Before.missing_skills.some((s) => s.canonical_name === "Docker")).toBe(true);
    const scoreBefore = job1Before.final_score;

    await roadmap.loadRoadmaps(uid);
    const dockerStep = roadmap.activeDetail!.steps.find((s) => s.skill_name === "Docker")!;
    const stepRes = await roadmap.toggleStepDone(uid, dockerStep.id, dockerStep.status);
    expect(stepRes?.suggest_add_skill?.canonical_name).toBe("Docker");

    await ipc.quickAddUserSkill(uid, stepRes!.suggest_add_skill!.skill_id);
    const job1After = await ipc.getJobDetail(uid, "job-1");
    expect(job1After.missing_skills.some((s) => s.canonical_name === "Docker")).toBe(false);
    expect(job1After.final_score).toBeGreaterThan(scoreBefore);

    await comp.loadRolesAndMap(uid);
    comp.gapOnly = true;
    expect(comp.filteredSkills.some((s) => s.canonical_name === "Docker")).toBe(false);
  });

  it("handles BYOK settings key validation, masking, deletion, and worker health check", async () => {
    const settings = useSettingsStore();
    await settings.loadApiKeyStatus();
    expect(settings.apiKeyStatus.has_key).toBe(true);
    expect(settings.apiKeyStatus.masked_suffix).toBe("AIza...9xQp");

    const badSave = await settings.saveAndTestApiKey("short");
    expect(badSave).toBe(false);
    expect(settings.error).toBeTruthy();

    const goodSave = await settings.saveAndTestApiKey("AIzaSyTESTNEWKEY9999");
    expect(goodSave).toBe(true);
    expect(settings.apiKeyStatus.masked_suffix).toBe("AIza...9999");

    const delOk = await settings.deleteApiKey();
    expect(delOk).toBe(true);
    expect(settings.apiKeyStatus.has_key).toBe(false);

    const healthy = await settings.checkWorkerHealth("http://127.0.0.1:8000");
    expect(healthy).toBe(true);
    expect(settings.workerHealth?.available_sources).toContain("linkedin");
  });

  it("supports search_query, work_mode filtering, and roadmap in_progress step status", async () => {
    const profile = useProfileStore();
    const jobs = useJobsStore();
    const roadmap = useRoadmapStore();
    await profile.loadActiveProfile();
    const uid = profile.userId!;

    jobs.searchQuery = "Systems";
    await jobs.loadJobs(uid);
    expect(jobs.jobs.length).toBe(1);
    expect(jobs.jobs[0].title).toContain("Systems");

    jobs.searchQuery = "";
    jobs.toggleWorkMode("remote");
    await jobs.loadJobs(uid);
    expect(jobs.jobs.length).toBeGreaterThanOrEqual(1);
    expect(jobs.jobs.every((j) => j.work_mode === "remote")).toBe(true);

    await roadmap.loadRoadmaps(uid);
    const secondStep = roadmap.activeDetail!.steps[1];
    const inProgRes = await roadmap.setStepStatus(uid, secondStep.id, "in_progress");
    expect(inProgRes?.status).toBe("in_progress");
    expect(roadmap.activeDetail!.steps[1].status).toBe("in_progress");
  });

  it("executes decoupled V2 workflow: preferences -> Stage 1 local scrape -> Stage 2 AI queue -> application tracker -> Apply Copilot", async () => {
    const profile = useProfileStore();
    const jobs = useJobsStore();
    await profile.loadActiveProfile();
    const uid = profile.userId!;

    await jobs.loadSearchPreferences();
    const savedOk = await jobs.saveSearchPreferences({
      ...jobs.searchPreferences,
      location: "Bandung",
      locations: ["Bandung"],
      sources: ["jobstreet", "linkedin", "kalibrr", "dealls"],
      ai_enrich_enabled: false,
      ai_enrich_max_jobs: 10,
      ai_rpm_limit: 10,
    });
    expect(savedOk).toBe(true);
    expect(jobs.searchPreferences.location).toBe("Bandung");

    const scrapeReport = await jobs.runScrape(uid);
    expect(scrapeReport).not.toBeNull();
    expect(jobs.pendingLocalCount).toBeGreaterThanOrEqual(1);

    const aiReport = await jobs.runAiMatchingQueue(uid);
    expect(aiReport).not.toBeNull();
    expect(aiReport!.enriched_jobs).toBeGreaterThanOrEqual(1);
    expect(jobs.pendingLocalCount).toBe(0);

    const targetJobId = jobs.jobs[0].job_id;
    const updated = await jobs.updateApplication(uid, targetJobId, "applied", "Sudah kirim CV");
    expect(updated?.application_status).toBe("applied");
    expect(updated?.user_notes).toBe("Sudah kirim CV");
    expect(updated?.applied_at).not.toBeNull();

    const kitOk = await jobs.generateApplyKit(uid, targetJobId, "id", false);
    expect(kitOk).toBe(true);
    expect(jobs.selectedDetail?.ai_apply_kit?.cv_bullets.length).toBeGreaterThanOrEqual(3);
  });

  it("supports V3 zero-API-key workflow: offline AI status, native scraper toggle, local CV import, Master CV & ATS PDF export", async () => {
    const settings = useSettingsStore();
    const profile = useProfileStore();
    await settings.loadOfflineEngineStatus();
    expect(settings.localAiStatus.is_ready).toBe(true);
    expect(settings.localAiStatus.dimensions).toBe(256);
    expect(settings.scraperMode).toBe("native_rust");

    const toggled = await settings.updateScraperMode("python_worker");
    expect(toggled).toBe(true);
    expect(settings.scraperMode).toBe("python_worker");

    const draft = await profile.importCvPdf("JVBERi0xLjQ=");
    expect(draft).not.toBeNull();
    expect(profile.skillsDraft.length).toBeGreaterThan(0);

    await profile.loadActiveProfile();
    const uid = profile.userId!;
    const cv = await profile.loadMasterCv(uid);
    expect(cv?.full_name).toBeTruthy();

    const saved = await profile.saveMasterCv({ ...cv!, headline: "Akuntan & Analis Pajak" });
    expect(saved).toBe(true);
    expect(profile.masterCv?.headline).toBe("Akuntan & Analis Pajak");

    const pdf = await profile.exportAtsPdf(uid, "job-1", "id");
    expect(pdf?.ats_keyword_score).toBeGreaterThan(0);
    expect(pdf?.file_path).toContain(".pdf");
  });

  it("enforces multi-filter Semua/manual toggle rules, watchlists, prodi catalog & portable backup", async () => {
    const profile = useProfileStore();
    const jobs = useJobsStore();
    await profile.loadActiveProfile();
    const uid = profile.userId!;

    // Default state = "Semua" (empty arrays)
    expect(jobs.workModes).toEqual([]);
    expect(jobs.locationClusters).toEqual([]);
    expect(jobs.searchPreferences.locations).toContain("Indonesia");

    // Selecting manual items disables "Semua"; removing the last re-activates "Semua"
    jobs.toggleWorkMode("remote");
    jobs.toggleWorkMode("hybrid");
    expect(jobs.workModes).toEqual(["remote", "hybrid"]);
    jobs.toggleWorkMode("remote");
    jobs.toggleWorkMode("hybrid");
    expect(jobs.workModes).toEqual([]);

    jobs.toggleLocationCluster("jabodetabek");
    expect(jobs.locationClusters).toEqual(["jabodetabek"]);
    jobs.clearLocationClusters();
    expect(jobs.locationClusters).toEqual([]);

    // Watchlist create & apply
    await jobs.loadWatchlists(uid);
    const beforeCount = jobs.watchlists.length;
    jobs.toggleWorkMode("remote");
    jobs.salaryOnly = true;
    const ok = await jobs.createWatchlist(uid, "Remote Bergaji");
    expect(ok).toBe(true);
    expect(jobs.watchlists.length).toBe(beforeCount + 1);

    // Prodi catalog & impact simulator & zero-secret backup
    const prodiRoles = await ipc.listProdiRolesCatalog("Akuntansi");
    expect(prodiRoles.length).toBeGreaterThanOrEqual(1);
    const sim = await ipc.simulateSkillImpact(uid, "Backend Engineer", [7]);
    expect(sim.length).toBe(1);
    expect(sim[0].avg_score_delta).toBeGreaterThan(0);
    const backup = await ipc.exportVectaBackup();
    expect(backup.file_name).toBe("vecta-backup.vecta");
    expect(backup.payload_json).not.toContain("AIza");

    // Wave 2 & 3: Salary Benchmark, Company Snapshot, Interview STAR & Multi-Profile CRUD
    const bench = await ipc.getSalaryBenchmark("Backend Engineer", null);
    expect(bench.sample_count).toBeGreaterThanOrEqual(1);
    const snap = await ipc.getCompanySnapshot(uid, "PT Teknologi Nusantara");
    expect(snap.total_jobs).toBeGreaterThanOrEqual(1);
    await jobs.loadJobs(uid);
    const prep = await ipc.generateInterviewPrep(uid, jobs.jobs[0].job_id, "id");
    expect(prep.questions.length).toBeGreaterThanOrEqual(2);

    const beforeProfiles = await profile.loadProfiles();
    profile.startNewProfileDraft();
    profile.rawStoryDraft = "Saya lulusan Akuntansi yang menguasai PSAK dan pelaporan pajak e-Faktur.";
    profile.skillsDraft = [
      { canonical_name: "PSAK", category: "core_hard", proficiency: "intermediate", evidence: null },
    ];
    profile.keywordsDraft = [
      {
        role_label: "Akuntan Pajak",
        fit_type: "primary_fit",
        scrape_queries: ["Akuntan Pajak"],
        match_reason: "Jalur baru",
        is_active: true,
      },
    ];
    const createdSecond = await profile.saveProfile();
    expect(createdSecond).toBe(true);
    expect(profile.profiles.length).toBe(beforeProfiles.length + 1);
    const firstId = beforeProfiles[0].id;
    const switchedBack = await profile.switchProfile(firstId);
    expect(switchedBack).toBe(true);
    expect(profile.userId).toBe(firstId);
  });
});
