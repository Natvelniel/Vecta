import { describe, expect, it, vi } from "vitest";
import {
  IpcError,
  normalizeCompetencyMap,
  normalizeJobSummary,
  normalizeRoadmapSummary,
  openExternalUrl,
  registerApiKeyMissingHandler,
  unwrapResult,
} from "./ipc";
import { formatPercent, formatProficiency, nextProficiency, safeNumber } from "./format";

describe("Vecta IPC adapter & numeric guards", () => {
  it("normalizes null Rust f64 exports into guaranteed finite numbers", () => {
    const summary = normalizeJobSummary({
      job_id: "j-1",
      source: "linkedin",
      url: null,
      title: "Backend Engineer",
      normalized_role: "Backend Engineer",
      company: "PT Contoh",
      location: "Jakarta",
      work_mode: "hybrid",
      experience_level: "junior",
      vector_score: null,
      skill_overlap_score: 0.804,
      level_score: 1.0,
      final_score: 0.872,
      matched_skills_count: 4,
      missing_skills_count: 1,
      has_gap_analysis: false,
      is_saved: false,
      enrichment_status: "ai_enriched",
      posted_at: null,
      salary_text: "Rp 10 - 15 Juta",
      domain_slug: "informatika_data",
      quality_score: 0.98,
      red_flags_count: 0,
      also_available_on: [],
      application_status: "new",
      applied_at: null,
      scraped_at: "2026-11-24T10:00:00Z",
    });

    expect(summary.vector_score).toBe(0);
    expect(summary.skill_overlap_score).toBeCloseTo(80.4, 2);
    expect(summary.level_score).toBe(100);
    expect(summary.final_score).toBeCloseTo(87.2, 2);

    const comp = normalizeCompetencyMap({
      target_role: "Backend Engineer",
      total_jobs: 10,
      is_small_sample: false,
      skills: [
        {
          skill_id: 1,
          canonical_name: "Rust",
          category: "core_hard",
          demand_count: 8,
          demand_percentage: null,
          is_owned_by_user: true,
          user_proficiency: "intermediate",
        },
      ],
    });
    expect(comp.skills[0].demand_percentage).toBe(0);

    const rm = normalizeRoadmapSummary({
      id: "rm-1",
      user_id: "u-1",
      target_role: "Backend Engineer",
      source_job_id: null,
      title: "Roadmap",
      total_estimated_weeks: 4,
      total_steps: 4,
      completed_steps: 1,
      progress_percentage: null,
      created_at: "2026-11-24T10:00:00Z",
    });
    expect(rm.progress_percentage).toBe(0);
  });

  it("unwraps ok envelopes and throws IpcError while triggering API_KEY_MISSING hook", () => {
    expect(unwrapResult({ status: "ok", data: 42 })).toBe(42);

    const spy = vi.fn();
    registerApiKeyMissingHandler(spy);

    expect(() =>
      unwrapResult({
        status: "error",
        error: { code: "API_KEY_MISSING", message: "Kunci belum diatur", retryable: false },
      }),
    ).toThrow(IpcError);
    expect(spy).toHaveBeenCalledOnce();
  });

  it("formats numeric percentages and proficiency cycles consistently", () => {
    expect(safeNumber(undefined)).toBe(0);
    expect(safeNumber(Number.NaN, 5)).toBe(5);
    expect(formatPercent(87.6)).toBe("88%");
    expect(formatProficiency("intermediate")).toBe("Menengah");
    expect(nextProficiency("beginner")).toBe("intermediate");
    expect(nextProficiency("intermediate")).toBe("advanced");
    expect(nextProficiency("advanced")).toBe("beginner");
  });

  it("rejects non-http(s) URL schemes in openExternalUrl", async () => {
    const openSpy = vi.fn();
    vi.stubGlobal("window", { open: openSpy });

    await openExternalUrl("javascript:alert(1)");
    await openExternalUrl("file:///C:/Windows/win.ini");
    await openExternalUrl("not-a-valid-url");
    expect(openSpy).not.toHaveBeenCalled();

    await openExternalUrl("https://aistudio.google.com/apikey");
    expect(openSpy).toHaveBeenCalledWith(
      "https://aistudio.google.com/apikey",
      "_blank",
      "noopener,noreferrer",
    );
    vi.unstubAllGlobals();
  });
});
