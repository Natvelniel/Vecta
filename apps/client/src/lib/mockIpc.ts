import type {
  AiMatchRunReport,
  ApiKeyStatus_Serialize,
  ApplicationStatus,
  ApplyKitResult,
  AtsExportResult,
  CompanySnapshot,
  CompetencyMapResponse_Serialize,
  ConfirmedProfileInput_Deserialize,
  EnrichmentStatus,
  GapAnalysis,
  ImportedCvDraft,
  InterviewPrepKit,
  JobFilterInput_Deserialize,
  JobMatchDetail_Serialize,
  JobMatchSummary_Serialize,
  LocalAiModelStatus,
  MasterCvProfile,
  ProdiRoleItem,
  ProfileSummaryItem,
  RoadmapDetail_Serialize,
  RoadmapSummary_Serialize,
  RoleJobCount,
  SalaryBenchmarkSummary,
  SavedWatchlistItem_Serialize,
  ScrapeKeywordItem_Serialize,
  ScrapeRunReport,
  SearchPreferences_Deserialize,
  SearchPreferences_Serialize,
  SkillImpactItem,
  StepStatus,
  StepUpdateResult_Serialize,
  StoryProfileDraft,
  UserProfileDetail_Serialize,
  VectaBackupReport,
  VectaImportReport,
  WorkerHealth,
} from "./bindings";
import { IpcError } from "./ipc";

// Dev-only in-memory preview harness (active strictly when VITE_MOCK_IPC=1 outside Tauri)
const nowIso = new Date().toISOString();
const twoHoursAgo = new Date(Date.now() - 2 * 3600_000).toISOString();
const fiveHoursAgo = new Date(Date.now() - 5 * 3600_000).toISOString();
const oneDayAgo = new Date(Date.now() - 26 * 3600_000).toISOString();

let hasKey = true;
let maskedKey: string | null = "AIza...9xQp";
let workerUrlState = "http://127.0.0.1:8000";
let scraperModeState = "native_rust";
let masterCvState: MasterCvProfile = {
  user_id: "user-demo-1",
  full_name: "Budi Santoso",
  headline: "Backend Engineer & Local-First Developer",
  email: "budi.santoso@example.com",
  phone: "+62 812-3456-7890",
  location: "Bandung, Jawa Barat",
  linkedin_url: "https://linkedin.com/in/budisantoso",
  portfolio_url: "https://github.com/budisantoso",
  summary:
    "Junior Backend Developer berpengalaman membangun layanan API berbasis Python FastAPI serta aplikasi lokal berkinerja tinggi dengan Rust dan SQLite.",
  experiences: [
    {
      role: "Backend Developer Intern",
      company: "PT Karya Digital Nusantara",
      location: "Bandung",
      period: "2024 - 2025",
      bullets: [
        "Membangun endpoint REST API menggunakan Python FastAPI dan SQLite dengan cakupan unit test otomatis.",
        "Mengoptimalkan query relasional untuk mempercepat waktu respons pencarian data di bawah 80ms.",
      ],
    },
  ],
  education: [
    {
      institution: "Institut Teknologi Bandung",
      degree: "S1 Teknik Informatika",
      period: "2021 - 2025",
      notes: "IPK 3.74",
    },
  ],
  projects: [],
  certifications: ["AWS Cloud Practitioner"],
  languages: ["Bahasa Indonesia", "Bahasa Inggris"],
  updated_at: nowIso,
};
let searchPrefsState: SearchPreferences_Serialize = {
  location: "Indonesia",
  locations: ["Indonesia", "Jakarta", "Bandung"],
  sources: ["jobstreet", "linkedin", "kalibrr", "dealls"],
  limit_per_query: 25,
  max_pages: 2,
  posted_within_days: 14,
  work_mode: "any",
  custom_queries: [],
  ai_enrich_enabled: false,
  ai_enrich_max_jobs: 15,
  ai_rpm_limit: 10,
};

const skillCatalog = new Map<number, { canonical_name: string; category: "core_hard" | "tools_tech" | "soft_skill" | "domain" }>([
  [1, { canonical_name: "Rust", category: "core_hard" }],
  [2, { canonical_name: "Python", category: "core_hard" }],
  [3, { canonical_name: "SQLite", category: "core_hard" }],
  [4, { canonical_name: "FastAPI", category: "core_hard" }],
  [5, { canonical_name: "REST API", category: "core_hard" }],
  [6, { canonical_name: "Git", category: "tools_tech" }],
  [7, { canonical_name: "Docker", category: "tools_tech" }],
  [8, { canonical_name: "PostgreSQL", category: "core_hard" }],
  [9, { canonical_name: "Redis", category: "tools_tech" }],
  [10, { canonical_name: "CI/CD", category: "tools_tech" }],
  [11, { canonical_name: "gRPC", category: "core_hard" }],
  [12, { canonical_name: "System Design", category: "domain" }],
  [13, { canonical_name: "Technical Writing", category: "soft_skill" }],
]);

let activeProfile: UserProfileDetail_Serialize = {
  id: "user-demo-1",
  profile_name: "Profil Utama",
  target_roles: ["Backend Engineer", "Systems Developer"],
  raw_story_text:
    "Saya lulusan Informatika yang membangun backend REST API memakai Python FastAPI dan aplikasi desktop lokal dengan Rust serta SQLite. Terbiasa merancang skema database relasional, menulis unit test otomatis, dan ingin berkembang sebagai Backend Engineer.",
  summary:
    "Junior Backend Developer berpengalaman membangun layanan API berbasis Python FastAPI serta aplikasi lokal berkinerja tinggi dengan Rust dan SQLite.",
  experience_level: "junior",
  has_embedding: true,
  is_active: true,
  skills: [
    { skill_id: 1, canonical_name: "Rust", category: "core_hard", proficiency: "intermediate", evidence: "Membangun aplikasi desktop lokal dengan Rust." },
    { skill_id: 2, canonical_name: "Python", category: "core_hard", proficiency: "advanced", evidence: "Pengembangan backend REST API." },
    { skill_id: 3, canonical_name: "SQLite", category: "core_hard", proficiency: "intermediate", evidence: "Merancang skema database relasional lokal." },
    { skill_id: 4, canonical_name: "FastAPI", category: "core_hard", proficiency: "intermediate", evidence: "Membangun layanan API Python FastAPI." },
    { skill_id: 5, canonical_name: "REST API", category: "core_hard", proficiency: "intermediate", evidence: "Desain endpoint dan pengujian otomatis." },
    { skill_id: 6, canonical_name: "Git", category: "tools_tech", proficiency: "advanced", evidence: "Alur kolaborasi dan branching harian." },
  ],
  keywords: [
    {
      id: 1,
      user_id: "user-demo-1",
      role_label: "Backend Engineer",
      fit_type: "primary_fit",
      scrape_queries: ["Backend Engineer", "Rust Developer"],
      match_reason: "Selaras dengan pengalaman Rust, SQLite, dan API Python.",
      is_active: true,
      last_scraped_at: twoHoursAgo,
    },
    {
      id: 2,
      user_id: "user-demo-1",
      role_label: "Systems Developer",
      fit_type: "adjacent_fit",
      scrape_queries: ["Systems Developer", "Desktop Engineer"],
      match_reason: "Kuat pada performa lokal dan manajemen memori.",
      is_active: true,
      last_scraped_at: fiveHoursAgo,
    },
    {
      id: 3,
      user_id: "user-demo-1",
      role_label: "Python API Developer",
      fit_type: "custom",
      scrape_queries: ["FastAPI Developer"],
      match_reason: "Ditambahkan manual oleh pengguna.",
      is_active: false,
      last_scraped_at: null,
    },
  ],
  created_at: oneDayAgo,
  updated_at: nowIso,
};

const mockProfilesList: UserProfileDetail_Serialize[] = [activeProfile];

interface MockJobRecord {
  job_id: string;
  source: "linkedin" | "glints" | "jobstreet" | "indeed" | "kalibrr" | "dealls" | "manual";
  external_id: string;
  url: string | null;
  title: string;
  normalized_role: string;
  company: string;
  location: string | null;
  location_cluster: string;
  work_mode: "onsite" | "hybrid" | "remote" | "unknown";
  experience_level: "intern" | "junior" | "mid" | "senior" | "unknown";
  enrichment_status: EnrichmentStatus;
  posted_at: string | null;
  salary_text: string | null;
  salary_min_idr: number | null;
  salary_max_idr: number | null;
  opportunity_type: string;
  raw_description: string;
  role_summary: string | null;
  vector_score: number;
  level_score: number;
  skill_refs: Array<{ skill_id: number; requirement_type: "mandatory" | "nice_to_have" }>;
  ai_gap_analysis: GapAnalysis | null;
  ai_apply_kit: ApplyKitResult | null;
  is_saved: boolean;
  application_status: ApplicationStatus;
  applied_at: string | null;
  user_notes: string | null;
  scraped_at: string;
}

const mockJobs: MockJobRecord[] = [
  {
    job_id: "job-1",
    source: "linkedin",
    external_id: "li-1001",
    url: "https://linkedin.com",
    title: "Backend Engineer (Rust & SQLite)",
    normalized_role: "Backend Engineer",
    company: "PT Nusantara Teknologi",
    location: "Jakarta Selatan",
    location_cluster: "jabodetabek",
    work_mode: "hybrid",
    experience_level: "junior",
    enrichment_status: "ai_enriched",
    posted_at: twoHoursAgo,
    salary_text: "Rp 11jt - 16jt",
    salary_min_idr: 11_000_000,
    salary_max_idr: 16_000_000,
    opportunity_type: "full_time",
    raw_description: "Membangun layanan backend lokal dan API berkinerja tinggi memakai Rust, SQLite, Python, REST API, Git, dan Docker.",
    role_summary: "Mengembangkan engine backend lokal dan REST API dengan standar reliability tinggi.",
    vector_score: 0.91,
    level_score: 1.0,
    skill_refs: [
      { skill_id: 1, requirement_type: "mandatory" },
      { skill_id: 3, requirement_type: "mandatory" },
      { skill_id: 5, requirement_type: "mandatory" },
      { skill_id: 2, requirement_type: "nice_to_have" },
      { skill_id: 6, requirement_type: "nice_to_have" },
      { skill_id: 7, requirement_type: "mandatory" },
    ],
    ai_gap_analysis: {
      verdict_headline: "Kandidat sangat siap untuk peran ini. Menutup celah Docker untuk deployment container akan menaikkan kecocokan mendekati sempurna.",
      strength_highlights: [
        "Pengalaman membangun engine Rust dan SQLite lokal cocok langsung dengan arsitektur tim.",
        "Tingkat Junior dan kebiasaan unit test mempercepat proses onboarding.",
      ],
      prioritized_gaps: [
        { skill: "Docker", severity: "high", reason: "Dibutuhkan untuk membungkus layanan backend ke container produksi." },
      ],
      cv_tailoring_tips: [
        "Cantumkan benchmark performa matching lokal (<100ms) pada poin proyek utama CV.",
        "Tambahkan contoh Dockerfile dan docker-compose di repositori portofolio.",
      ],
    },
    ai_apply_kit: null,
    is_saved: true,
    application_status: "saved",
    applied_at: null,
    user_notes: null,
    scraped_at: twoHoursAgo,
  },
  {
    job_id: "job-2",
    source: "indeed",
    external_id: "in-2002",
    url: "https://id.indeed.com",
    title: "Systems Software Engineer",
    normalized_role: "Systems Developer",
    company: "PT Awan Lokal Indonesia",
    location: "Bandung (Remote)",
    location_cluster: "remote_indonesia",
    work_mode: "remote",
    experience_level: "junior",
    enrichment_status: "local_only",
    posted_at: fiveHoursAgo,
    salary_text: "Rp 10jt - 14jt",
    salary_min_idr: 10_000_000,
    salary_max_idr: 14_000_000,
    opportunity_type: "full_time",
    raw_description: "Pengembangan runtime desktop & CLI memakai Rust, SQLite, Git, Docker, dan gRPC.",
    role_summary: null,
    vector_score: 0.84,
    level_score: 1.0,
    skill_refs: [
      { skill_id: 1, requirement_type: "mandatory" },
      { skill_id: 3, requirement_type: "mandatory" },
      { skill_id: 6, requirement_type: "nice_to_have" },
      { skill_id: 7, requirement_type: "mandatory" },
      { skill_id: 11, requirement_type: "nice_to_have" },
    ],
    ai_gap_analysis: null,
    ai_apply_kit: null,
    is_saved: false,
    application_status: "new",
    applied_at: null,
    user_notes: null,
    scraped_at: fiveHoursAgo,
  },
  {
    job_id: "job-3",
    source: "jobstreet",
    external_id: "js-3003",
    url: "https://id.jobstreet.com",
    title: "Backend Python & API Developer",
    normalized_role: "Backend Engineer",
    company: "Studio Karya Digital",
    location: "Jakarta Selatan",
    location_cluster: "jabodetabek",
    work_mode: "hybrid",
    experience_level: "mid",
    enrichment_status: "ai_enriched",
    posted_at: oneDayAgo,
    salary_text: null,
    salary_min_idr: null,
    salary_max_idr: null,
    opportunity_type: "full_time",
    raw_description: "Membangun layanan FastAPI, PostgreSQL, Redis, dan Docker untuk produk SaaS.",
    role_summary: "Memelihara dan menskalakan layanan Python API dan database PostgreSQL.",
    vector_score: 0.78,
    level_score: 0.65,
    skill_refs: [
      { skill_id: 2, requirement_type: "mandatory" },
      { skill_id: 4, requirement_type: "mandatory" },
      { skill_id: 5, requirement_type: "mandatory" },
      { skill_id: 8, requirement_type: "mandatory" },
      { skill_id: 7, requirement_type: "mandatory" },
      { skill_id: 9, requirement_type: "nice_to_have" },
    ],
    ai_gap_analysis: null,
    ai_apply_kit: null,
    is_saved: false,
    application_status: "applied",
    applied_at: fiveHoursAgo,
    user_notes: "Sudah kirim CV lewat JobStreet.",
    scraped_at: oneDayAgo,
  },
  {
    job_id: "job-4",
    source: "kalibrr",
    external_id: "kl-4004",
    url: "https://kalibrr.com",
    title: "ODP Technology & Data Track 2026",
    normalized_role: "Backend Engineer",
    company: "Bank Digital Nusantara",
    location: "Surabaya",
    location_cluster: "surabaya_jatim",
    work_mode: "onsite",
    experience_level: "junior",
    enrichment_status: "local_only",
    posted_at: twoHoursAgo,
    salary_text: "Rp 9jt - 12jt",
    salary_min_idr: 9_000_000,
    salary_max_idr: 12_000_000,
    opportunity_type: "mt_odp",
    raw_description: "Officer Development Program bidang perbankan digital: SQL, Python, REST API, Git, PostgreSQL, dan pemahaman transaksi QRIS/BI-FAST.",
    role_summary: "Program akselerasi talenta teknologi perbankan dengan rotasi divisi inti.",
    vector_score: 0.86,
    level_score: 1.0,
    skill_refs: [
      { skill_id: 2, requirement_type: "mandatory" },
      { skill_id: 4, requirement_type: "mandatory" },
      { skill_id: 5, requirement_type: "mandatory" },
      { skill_id: 8, requirement_type: "nice_to_have" },
    ],
    ai_gap_analysis: null,
    ai_apply_kit: null,
    is_saved: false,
    application_status: "new",
    applied_at: null,
    user_notes: null,
    scraped_at: twoHoursAgo,
  },
];

let mockWatchlists: SavedWatchlistItem_Serialize[] = [
  {
    id: "wl-demo-1",
    user_id: "user-demo-1",
    name: "Remote & Jabodetabek Bergaji",
    filter: {
      normalized_role: null,
      work_mode: null,
      source: null,
      saved_only: null,
      min_score: null,
      search_query: null,
      location_query: null,
      enrichment_status: null,
      application_status: null,
      include_dismissed: null,
      work_modes: ["remote", "hybrid"],
      sources: null,
      location_clusters: ["jabodetabek", "remote_indonesia"],
      opportunity_types: null,
      application_statuses: null,
      salary_only: true,
      sort_by: "score",
      limit: 50,
      offset: 0,
    },
    created_at: oneDayAgo,
  },
];

const mockProdiCatalog: ProdiRoleItem[] = [
  {
    id: "role_001",
    prodi_label: "S1 Teknik Informatika / Ilmu Komputer",
    domain: "informatika_data",
    role_title: "Backend Developer",
    mandatory_skills: ["Rust", "Python", "REST API", "SQLite", "Git"],
    preferred_skills: ["Docker", "PostgreSQL", "Redis"],
    certifications: [],
    scrape_queries: ["Backend Developer", "Backend Engineer"],
  },
  {
    id: "role_020",
    prodi_label: "S1 Akuntansi / Perpajakan",
    domain: "ekonomi_akuntansi_pajak",
    role_title: "Tax & Accounting Staff",
    mandatory_skills: ["Akuntansi", "Brevet Pajak A & B", "Laporan Keuangan", "PSAK / IFRS", "Microsoft Excel"],
    preferred_skills: ["Accurate Accounting", "SAP ERP"],
    certifications: ["Brevet A/B"],
    scrape_queries: ["Tax Staff", "Accounting Staff"],
  },
  {
    id: "role_042",
    prodi_label: "S1 Manajemen / Bisnis Digital",
    domain: "bisnis_marketing_sales",
    role_title: "Digital Marketing & Growth Specialist",
    mandatory_skills: ["Digital Marketing", "Meta Ads & TikTok Ads", "SEO", "Copywriting", "Google Analytics"],
    preferred_skills: ["CRM", "SEM & Google Ads"],
    certifications: [],
    scrape_queries: ["Digital Marketing", "Performance Marketing"],
  },
  {
    id: "role_055",
    prodi_label: "S1 Psikologi / Manajemen SDM",
    domain: "hr_psikologi_admin_ops",
    role_title: "HRGA & People Operations",
    mandatory_skills: ["Rekrutmen & Seleksi", "Payroll & Kompensasi", "BPJS & Ketenagakerjaan", "HRIS"],
    preferred_skills: ["Asesmen Psikologi & Psikotes"],
    certifications: [],
    scrape_queries: ["HR Staff", "Talent Acquisition"],
  },
  {
    id: "role_071",
    prodi_label: "S1 Teknik Sipil",
    domain: "teknik_sipil_arsitektur_gis",
    role_title: "Site Engineer / Quantity Surveyor",
    mandatory_skills: ["AutoCAD", "ETABS", "RAB & Quantity Surveying", "Manajemen Konstruksi"],
    preferred_skills: ["Revit & BIM"],
    certifications: ["SKA Muda"],
    scrape_queries: ["Site Engineer", "Quantity Surveyor"],
  },
  {
    id: "role_112",
    prodi_label: "S1 Teknik Pertambangan / Geologi",
    domain: "pertambangan_migas_energi",
    role_title: "Mine Plan & Drilling Engineer",
    mandatory_skills: ["Perencanaan Tambang (Surpac / Minescape / Vulcan)", "Operasi Peledakan & Pemboran (Drilling & Blasting)", "K3 / HSE"],
    preferred_skills: ["Sertifikasi Pengawas Operasional (POP / POM / POU Minerba)"],
    certifications: ["POP Minerba"],
    scrape_queries: ["Mine Plan Engineer", "Mining Engineer"],
  },
  {
    id: "role_140",
    prodi_label: "Profesi Apoteker / S1 Farmasi",
    domain: "kesehatan_farmasi_keperawatan",
    role_title: "Apoteker QA & Regulatory Affairs",
    mandatory_skills: ["Farmasi Klinis & Apoteker", "CPOB / CPOTB / CDOB", "Registrasi BPOM & Regulatory Affairs"],
    preferred_skills: ["Farmakovigilans"],
    certifications: ["STRA"],
    scrape_queries: ["Apoteker", "Regulatory Affairs"],
  },
  {
    id: "role_185",
    prodi_label: "S1 Perbankan / Aktuaria / Keuangan",
    domain: "perbankan_aktuaria_asuransi",
    role_title: "ODP / Corporate Credit Analyst",
    mandatory_skills: ["Analisis Kredit Korporasi & UMKM", "Treasury, Pasar Uang & Devisa", "Financial Modeling"],
    preferred_skills: ["Ilmu Aktuaria & Valuasi Liabilitas"],
    certifications: ["BSMR"],
    scrape_queries: ["ODP Bank", "Credit Analyst"],
  },
];

let mockRoadmaps: RoadmapDetail_Serialize[] = [
  {
    id: "rm-1",
    user_id: "user-demo-1",
    target_role: "Backend Engineer",
    source_job_id: "job-1",
    title: "Roadmap Backend Engineer: Container & Produksi",
    total_estimated_weeks: 4,
    created_at: oneDayAgo,
    steps: [
      {
        id: "st-1",
        roadmap_id: "rm-1",
        skill_id: 7,
        skill_name: "Docker",
        phase_order: 1,
        phase_name: "Fase 1 · Fundamental",
        topic_title: "Dockerfile Multi-Stage & Image Ringan",
        learning_keywords: ["docker multi-stage build", "container healthcheck"],
        mini_project_idea: "Bungkus layanan Python Scraper Worker menjadi container terisolasi dengan endpoint healthcheck.",
        estimated_hours: 4,
        status: "todo",
        completed_at: null,
      },
      {
        id: "st-2",
        roadmap_id: "rm-1",
        skill_id: 8,
        skill_name: "PostgreSQL",
        phase_order: 2,
        phase_name: "Fase 2 · Praktik",
        topic_title: "Indeks Komposit, CTE & EXPLAIN ANALYZE",
        learning_keywords: ["postgresql composite index", "explain analyze tuning"],
        mini_project_idea: "Migrasi query agregasi kompetensi ke Postgres dan ukur waktu eksekusi di bawah 20ms.",
        estimated_hours: 6,
        status: "todo",
        completed_at: null,
      },
      {
        id: "st-3",
        roadmap_id: "rm-1",
        skill_id: 9,
        skill_name: "Redis",
        phase_order: 2,
        phase_name: "Fase 2 · Praktik",
        topic_title: "Cache TTL & Pembatasan Laju Request",
        learning_keywords: ["redis ttl caching fastapi", "rate limit token bucket"],
        mini_project_idea: "Tambahkan lapisan cache hasil scrape 15 menit pada worker lokal.",
        estimated_hours: 5,
        status: "todo",
        completed_at: null,
      },
      {
        id: "st-4",
        roadmap_id: "rm-1",
        skill_id: 10,
        skill_name: "CI/CD",
        phase_order: 3,
        phase_name: "Fase 3 · Proyek Mini",
        topic_title: "Alur Pengujian Otomatis Rust & Python",
        learning_keywords: ["github actions cargo test pytest", "build caching matrix"],
        mini_project_idea: "Buat workflow CI yang menjalankan cargo test dan pytest pada setiap pull request.",
        estimated_hours: 5,
        status: "todo",
        completed_at: null,
      },
    ],
  },
];

function computeJobDetail(job: MockJobRecord): JobMatchDetail_Serialize {
  const owned = new Set(activeProfile.skills.map((s) => s.canonical_name.toLowerCase()));
  let matchedW = 0;
  let totalW = 0;
  const matched_skills: JobMatchDetail_Serialize["matched_skills"] = [];
  const missing_skills: JobMatchDetail_Serialize["missing_skills"] = [];

  for (const ref of job.skill_refs) {
    const meta = skillCatalog.get(ref.skill_id) ?? { canonical_name: `Skill #${ref.skill_id}`, category: "core_hard" as const };
    const w = ref.requirement_type === "mandatory" ? 1.0 : 0.5;
    totalW += w;
    const item = {
      skill_id: ref.skill_id,
      canonical_name: meta.canonical_name,
      category: meta.category,
      requirement_type: ref.requirement_type,
    };
    if (owned.has(meta.canonical_name.toLowerCase())) {
      matchedW += w;
      matched_skills.push(item);
    } else {
      missing_skills.push(item);
    }
  }

  const skill_overlap_score = totalW === 0 ? job.vector_score : matchedW / totalW;
  const final_score = 0.4 * job.vector_score + 0.45 * skill_overlap_score + 0.15 * job.level_score;

  return {
    job_id: job.job_id,
    source: job.source,
    external_id: job.external_id,
    url: job.url,
    title: job.title,
    normalized_role: job.normalized_role,
    company: job.company,
    location: job.location,
    location_cluster: job.location_cluster,
    work_mode: job.work_mode,
    experience_level: job.experience_level,
    enrichment_status: job.enrichment_status,
    posted_at: job.posted_at,
    salary_text: job.salary_text,
    salary_min_idr: job.salary_min_idr,
    salary_max_idr: job.salary_max_idr,
    opportunity_type: job.opportunity_type,
    domain_slug: "informatika_data",
    quality_score: 0.98,
    red_flags: [],
    also_available_on:
      job.job_id === "job-1"
        ? [{ source: "jobstreet", url: "https://id.jobstreet.com" }]
        : [],
    raw_description: job.raw_description,
    role_summary: job.role_summary,
    vector_score: job.vector_score,
    skill_overlap_score,
    level_score: job.level_score,
    final_score,
    domain_score: 0.92,
    reranker_score: job.job_id === "job-1" ? 0.91 : null,
    match_explanation: [
      "Bidang utama selaras dengan target profil Anda.",
      ...(matched_skills.length > 0
        ? [`Skill inti cocok: ${matched_skills.slice(0, 3).map((s) => s.canonical_name).join(", ")}`]
        : []),
    ],
    matched_skills,
    missing_skills,
    ai_gap_analysis: job.ai_gap_analysis,
    ai_apply_kit: job.ai_apply_kit,
    is_saved: job.is_saved,
    application_status: job.application_status,
    applied_at: job.applied_at,
    user_notes: job.user_notes,
    scraped_at: job.scraped_at,
  };
}

export const mockCommands = {
  async getApiKeyStatus(): Promise<ApiKeyStatus_Serialize> {
    return { has_key: hasKey, masked_suffix: maskedKey };
  },
  async saveAndTestApiKey(apiKey: string): Promise<ApiKeyStatus_Serialize> {
    if (apiKey.trim().length < 8) {
      throw new IpcError({ code: "API_KEY_INVALID", message: "Format kunci API Gemini tidak valid.", retryable: false });
    }
    hasKey = true;
    maskedKey = `${apiKey.trim().slice(0, 4)}...${apiKey.trim().slice(-4)}`;
    return { has_key: hasKey, masked_suffix: maskedKey };
  },
  async deleteApiKey(): Promise<null> {
    hasKey = false;
    maskedKey = null;
    return null;
  },
  async checkWorkerHealth(workerUrl: string | null): Promise<WorkerHealth> {
    if (workerUrl) workerUrlState = workerUrl;
    return { status: `ok (${workerUrlState})`, version: "0.1.0", available_sources: ["linkedin", "jobstreet", "indeed", "glints"] };
  },
  async getSearchPreferences(): Promise<SearchPreferences_Serialize> {
    return {
      ...searchPrefsState,
      locations: [...searchPrefsState.locations],
      sources: [...searchPrefsState.sources],
      custom_queries: [...searchPrefsState.custom_queries],
    };
  },
  async saveSearchPreferences(prefs: SearchPreferences_Deserialize): Promise<SearchPreferences_Serialize> {
    const locs = prefs.locations && prefs.locations.length > 0 ? prefs.locations : [prefs.location || "Indonesia"];
    searchPrefsState = {
      ...prefs,
      location: locs[0],
      locations: [...locs],
      posted_within_days: prefs.posted_within_days ?? null,
      sources: [...prefs.sources],
      custom_queries: [...prefs.custom_queries],
    };
    return this.getSearchPreferences();
  },
  async listProdiRolesCatalog(query: string | null): Promise<ProdiRoleItem[]> {
    const q = (query ?? "").trim().toLowerCase();
    if (!q) return mockProdiCatalog;
    return mockProdiCatalog.filter(
      (r) =>
        r.prodi_label.toLowerCase().includes(q) ||
        r.role_title.toLowerCase().includes(q) ||
        r.domain.toLowerCase().includes(q),
    );
  },
  async exportVectaBackup(): Promise<VectaBackupReport> {
    const payload = {
      schema_version: 1,
      exported_at: new Date().toISOString(),
      active_profile: activeProfile,
      master_cv: masterCvState,
      search_preferences_json: JSON.stringify(searchPrefsState),
      watchlists: mockWatchlists,
    };
    return {
      schema_version: 1,
      exported_at: payload.exported_at,
      file_name: "vecta-backup.vecta",
      payload_json: JSON.stringify(payload, null, 2),
      profiles_count: 1,
      watchlists_count: mockWatchlists.length,
      tracked_jobs_count: mockJobs.filter((j) => j.application_status !== "new").length,
    };
  },
  async importVectaBackup(backupJson: string): Promise<VectaImportReport> {
    const parsed = JSON.parse(backupJson) as {
      active_profile?: UserProfileDetail_Serialize;
      watchlists?: SavedWatchlistItem_Serialize[];
      search_preferences_json?: string;
    };
    if (parsed.active_profile) activeProfile = parsed.active_profile;
    if (parsed.watchlists) mockWatchlists = parsed.watchlists;
    return {
      imported_profiles: parsed.active_profile ? 1 : 0,
      imported_watchlists: parsed.watchlists?.length ?? 0,
      imported_preferences: Boolean(parsed.search_preferences_json),
      warnings: [],
    };
  },
  async parseUserStory(rawStory: string): Promise<StoryProfileDraft> {
    if (rawStory.trim().length < 30) {
      throw new IpcError({ code: "VALIDATION_ERROR", message: "Cerita pengalaman minimal 30 karakter.", retryable: false });
    }
    return {
      professional_summary: activeProfile.summary ?? "",
      estimated_experience_level: "junior",
      extracted_skills: activeProfile.skills.map((s) => ({
        canonical_name: s.canonical_name,
        category: s.category,
        proficiency: s.proficiency,
        evidence: s.evidence ?? "Diekstrak dari cerita pengalaman pengguna.",
      })),
      recommended_keywords: activeProfile.keywords.map((k) => ({
        role_label: k.role_label,
        fit_type: k.fit_type,
        match_reason: k.match_reason ?? "",
        scrape_queries: k.scrape_queries,
      })),
    };
  },
  async saveConfirmedProfile(input: ConfirmedProfileInput_Deserialize): Promise<UserProfileDetail_Serialize> {
    const newId = input.id || `user-demo-${Date.now()}`;
    const saved: UserProfileDetail_Serialize = {
      ...activeProfile,
      id: newId,
      profile_name: input.profile_name,
      target_roles: input.target_roles,
      raw_story_text: input.raw_story_text,
      summary: input.summary,
      experience_level: input.experience_level,
      is_active: true,
      skills: input.skills.map((s, idx) => ({
        skill_id: idx + 1,
        canonical_name: s.canonical_name,
        category: s.category,
        proficiency: s.proficiency,
        evidence: s.evidence ?? null,
      })),
      keywords: input.keywords.map((k, idx) => ({
        id: idx + 1,
        user_id: newId,
        role_label: k.role_label,
        fit_type: k.fit_type,
        scrape_queries: k.scrape_queries,
        match_reason: k.match_reason ?? null,
        is_active: k.is_active,
        last_scraped_at: null,
      })),
      updated_at: new Date().toISOString(),
    };
    for (const p of mockProfilesList) p.is_active = false;
    const existingIdx = mockProfilesList.findIndex((p) => p.id === newId);
    if (existingIdx >= 0) {
      mockProfilesList[existingIdx] = saved;
    } else {
      mockProfilesList.unshift(saved);
    }
    activeProfile = saved;
    return activeProfile;
  },
  async getActiveProfile(): Promise<UserProfileDetail_Serialize | null> {
    return activeProfile;
  },
  async listUserProfiles(): Promise<ProfileSummaryItem[]> {
    return mockProfilesList.map((p) => ({
      id: p.id,
      profile_name: p.profile_name,
      target_roles: [...p.target_roles],
      experience_level: p.experience_level,
      skill_count: p.skills.length,
      is_active: p.id === activeProfile.id,
      updated_at: p.updated_at,
    }));
  },
  async switchActiveProfile(profileId: string): Promise<UserProfileDetail_Serialize> {
    const found = mockProfilesList.find((p) => p.id === profileId);
    if (!found) {
      throw new IpcError({ code: "VALIDATION_ERROR", message: "Profil tidak ditemukan.", retryable: false });
    }
    for (const p of mockProfilesList) p.is_active = p.id === profileId;
    activeProfile = found;
    return activeProfile;
  },
  async deleteUserProfile(profileId: string): Promise<UserProfileDetail_Serialize> {
    if (mockProfilesList.length <= 1) {
      throw new IpcError({ code: "VALIDATION_ERROR", message: "Minimal harus tersisa satu profil aktif.", retryable: false });
    }
    const idx = mockProfilesList.findIndex((p) => p.id === profileId);
    if (idx >= 0) mockProfilesList.splice(idx, 1);
    mockProfilesList[0].is_active = true;
    activeProfile = mockProfilesList[0];
    return activeProfile;
  },
  async toggleScrapeKeyword(keywordId: number, isActive: boolean): Promise<null> {
    const kw = activeProfile.keywords.find((k) => k.id === keywordId);
    if (kw) kw.is_active = isActive;
    return null;
  },
  async addCustomKeyword(userId: string, roleLabel: string, queries: string[]): Promise<ScrapeKeywordItem_Serialize> {
    const item: ScrapeKeywordItem_Serialize = {
      id: Date.now(),
      user_id: userId,
      role_label: roleLabel,
      fit_type: "custom",
      scrape_queries: queries,
      match_reason: "Ditambahkan manual oleh pengguna.",
      is_active: true,
      last_scraped_at: null,
    };
    activeProfile.keywords.push(item);
    return item;
  },
  async ingestManualJob(_userId: string, titleHint: string | null, url: string | null, rawText: string): Promise<JobMatchDetail_Serialize> {
    if (rawText.trim().length < 30) {
      throw new IpcError({ code: "VALIDATION_ERROR", message: "Deskripsi lowongan minimal 30 karakter.", retryable: false });
    }
    const rec: MockJobRecord = {
      job_id: `job-manual-${Date.now()}`,
      source: "manual",
      external_id: `m-${Date.now()}`,
      url,
      title: titleHint?.trim() || "Backend Engineer (Manual)",
      normalized_role: "Backend Engineer",
      company: "Input Manual",
      location: "Indonesia",
      location_cluster: "remote_indonesia",
      work_mode: "remote",
      experience_level: "junior",
      enrichment_status: "ai_enriched",
      posted_at: new Date().toISOString(),
      salary_text: null,
      salary_min_idr: null,
      salary_max_idr: null,
      opportunity_type: "full_time",
      raw_description: rawText,
      role_summary: "Diekstrak dari deskripsi lowongan manual.",
      vector_score: 0.86,
      level_score: 1.0,
      skill_refs: [
        { skill_id: 1, requirement_type: "mandatory" },
        { skill_id: 3, requirement_type: "mandatory" },
        { skill_id: 7, requirement_type: "nice_to_have" },
      ],
      ai_gap_analysis: null,
      ai_apply_kit: null,
      is_saved: false,
      application_status: "new",
      applied_at: null,
      user_notes: null,
      scraped_at: new Date().toISOString(),
    };
    mockJobs.unshift(rec);
    return computeJobDetail(rec);
  },
  async runScrapeForActiveKeywords(_userId: string, _sources: string[], _limitPerQuery: number): Promise<ScrapeRunReport> {
    const pending = mockJobs.filter((j) => j.enrichment_status === "local_only" && j.application_status !== "dismissed").length;
    return { total_fetched: 12, newly_ingested: 0, skipped_duplicates: 12, failed_extractions: 0, pending_local_count: pending, warnings: [] };
  },
  async scrapeAndStoreJobs(_userId: string, _prefs: SearchPreferences_Deserialize): Promise<ScrapeRunReport> {
    const pending = mockJobs.filter((j) => j.enrichment_status === "local_only" && j.application_status !== "dismissed").length;
    return { total_fetched: 18, newly_ingested: 0, skipped_duplicates: 18, failed_extractions: 0, pending_local_count: pending, warnings: [] };
  },
  async runAiMatchingQueue(_userId: string, maxJobs: number, _rpmLimit: number): Promise<AiMatchRunReport> {
    const targets = mockJobs
      .filter((j) => j.enrichment_status === "local_only" && j.application_status !== "dismissed")
      .slice(0, maxJobs);
    for (const j of targets) {
      j.enrichment_status = "ai_enriched";
      j.role_summary = "Diperkaya dengan AI Matching batch lokal.";
    }
    const remaining = mockJobs.filter((j) => j.enrichment_status === "local_only" && j.application_status !== "dismissed").length;
    return {
      processed_jobs: targets.length,
      enriched_jobs: targets.length,
      failed_jobs: 0,
      remaining_local_jobs: remaining,
      api_calls_used: targets.length > 0 ? 2 : 0,
      warnings: [],
    };
  },
  async enrichJobWithAi(_userId: string, jobId: string): Promise<JobMatchDetail_Serialize> {
    const found = mockJobs.find((j) => j.job_id === jobId) ?? mockJobs[0];
    found.enrichment_status = "ai_enriched";
    found.role_summary = found.role_summary || "Diperkaya on-demand dengan AI Matching.";
    return computeJobDetail(found);
  },
  async updateJobApplication(
    _userId: string,
    jobId: string,
    status: ApplicationStatus,
    notes: string | null,
  ): Promise<JobMatchDetail_Serialize> {
    const found = mockJobs.find((j) => j.job_id === jobId) ?? mockJobs[0];
    found.application_status = status;
    found.user_notes = notes?.trim() || null;
    found.is_saved = status !== "new" && status !== "dismissed";
    if (status === "applied" || status === "interview" || status === "offer") {
      found.applied_at = found.applied_at ?? new Date().toISOString();
    } else if (status === "new" || status === "saved" || status === "dismissed") {
      found.applied_at = null;
    }
    return computeJobDetail(found);
  },
  async generateApplyKit(
    _userId: string,
    jobId: string,
    language: "id" | "en",
    _forceRefresh: boolean,
  ): Promise<ApplyKitResult> {
    const found = mockJobs.find((j) => j.job_id === jobId);
    const kit: ApplyKitResult = {
      language,
      tailored_headline:
        language === "en"
          ? "Backend Engineer focused on reliable local-first APIs and database design"
          : "Backend Engineer berfokus pada API handal, skema database terstruktur, dan pengujian otomatis",
      cv_bullets: [
        "Merancang layanan backend REST API dan skema database relasional terukur dengan cakupan unit test otomatis.",
        "Mengoptimalkan query database dan proses integrasi data lokal berlatensi rendah (<100ms).",
        "Mengelola alur deployment modular dan dokumentasi API untuk mempermudah kolaborasi tim lintas fungsi.",
      ],
      cover_note:
        language === "en"
          ? `Hello ${found?.company ?? "Hiring Team"}, I am excited to apply for the ${found?.title ?? "Backend Engineer"} role.`
          : `Halo Tim Rekrutmen ${found?.company ?? ""}, saya tertarik melamar posisi ${found?.title ?? "Backend Engineer"} karena pengalaman langsung saya selaras dengan kebutuhan tim.`,
      interview_angles: [
        "Desain skema database relasional dan strategi optimasi query",
        "Penanganan error dan pengujian otomatis pada layanan backend",
        "Arsitektur integrasi layanan dan trade-off performa di produksi",
      ],
    };
    if (found) found.ai_apply_kit = kit;
    return kit;
  },
  async listRankedJobs(_userId: string, filter: JobFilterInput_Deserialize): Promise<JobMatchSummary_Serialize[]> {
    const query = filter.search_query?.trim().toLowerCase() ?? "";
    const locQuery = filter.location_query?.trim().toLowerCase() ?? "";
    const filtered = mockJobs
      .map(computeJobDetail)
      .filter((j) => {
        if (filter.application_statuses && filter.application_statuses.length > 0) {
          if (!filter.application_statuses.includes(j.application_status)) return false;
        } else if (filter.application_status) {
          if (j.application_status !== filter.application_status) return false;
        } else if (!filter.include_dismissed && j.application_status === "dismissed") {
          return false;
        }
        if (filter.normalized_role && j.normalized_role !== filter.normalized_role) return false;
        if (filter.saved_only && !j.is_saved) return false;
        if (filter.salary_only && j.salary_min_idr == null && !j.salary_text) return false;
        if (filter.work_modes && filter.work_modes.length > 0) {
          if (!filter.work_modes.includes(j.work_mode)) return false;
        } else if (filter.work_mode && j.work_mode !== filter.work_mode) {
          return false;
        }
        if (filter.sources && filter.sources.length > 0) {
          if (!filter.sources.includes(j.source)) return false;
        } else if (filter.source && j.source !== filter.source) {
          return false;
        }
        if (filter.location_clusters && filter.location_clusters.length > 0) {
          if (!filter.location_clusters.includes(j.location_cluster)) return false;
        }
        if (filter.opportunity_types && filter.opportunity_types.length > 0) {
          if (!filter.opportunity_types.includes(j.opportunity_type)) return false;
        }
        if (filter.enrichment_status && j.enrichment_status !== filter.enrichment_status) return false;
        if (locQuery && !(j.location ?? "").toLowerCase().includes(locQuery)) return false;
        if (
          query &&
          !j.title.toLowerCase().includes(query) &&
          !j.company.toLowerCase().includes(query)
        ) {
          return false;
        }
        return true;
      })
      .sort((a, b) => {
        if (filter.sort_by === "newest") {
          return (b.posted_at ?? b.scraped_at).localeCompare(a.posted_at ?? a.scraped_at);
        }
        if (filter.sort_by === "salary_desc") {
          return (b.salary_max_idr ?? b.salary_min_idr ?? 0) - (a.salary_max_idr ?? a.salary_min_idr ?? 0);
        }
        return (b.final_score ?? 0) - (a.final_score ?? 0);
      });

    const offset = filter.offset ?? 0;
    const sliced = filter.limit != null ? filtered.slice(offset, offset + filter.limit) : filtered.slice(offset);

    return sliced
      .map((d) => ({
        job_id: d.job_id,
        source: d.source,
        url: d.url,
        title: d.title,
        normalized_role: d.normalized_role,
        company: d.company,
        location: d.location,
        location_cluster: d.location_cluster,
        work_mode: d.work_mode,
        experience_level: d.experience_level,
        enrichment_status: d.enrichment_status,
        posted_at: d.posted_at,
        salary_text: d.salary_text,
        salary_min_idr: d.salary_min_idr,
        salary_max_idr: d.salary_max_idr,
        opportunity_type: d.opportunity_type,
        domain_slug: d.domain_slug,
        quality_score: d.quality_score,
        red_flags_count: d.red_flags.length,
        also_available_on: d.also_available_on,
        vector_score: d.vector_score,
        skill_overlap_score: d.skill_overlap_score,
        level_score: d.level_score,
        final_score: d.final_score,
        domain_score: d.domain_score,
        reranker_score: d.reranker_score,
        match_explanation: d.match_explanation,
        matched_skills_count: d.matched_skills.length,
        missing_skills_count: d.missing_skills.length,
        has_gap_analysis: d.ai_gap_analysis !== null,
        is_saved: d.is_saved,
        application_status: d.application_status,
        applied_at: d.applied_at,
        scraped_at: d.scraped_at,
      }));
  },
  async getJobDetail(_userId: string, jobId: string): Promise<JobMatchDetail_Serialize> {
    const found = mockJobs.find((j) => j.job_id === jobId) ?? mockJobs[0];
    return computeJobDetail(found);
  },
  async toggleSaveJob(_userId: string, jobId: string): Promise<boolean> {
    const found = mockJobs.find((j) => j.job_id === jobId);
    if (!found) return false;
    found.is_saved = !found.is_saved;
    return found.is_saved;
  },
  async listSavedWatchlists(_userId: string): Promise<SavedWatchlistItem_Serialize[]> {
    return [...mockWatchlists];
  },
  async saveWatchlist(
    userId: string,
    name: string,
    filter: JobFilterInput_Deserialize,
  ): Promise<SavedWatchlistItem_Serialize> {
    const item: SavedWatchlistItem_Serialize = {
      id: `wl-${Date.now()}`,
      user_id: userId,
      name,
      filter: { ...filter } as unknown as SavedWatchlistItem_Serialize["filter"],
      created_at: new Date().toISOString(),
    };
    mockWatchlists = [item, ...mockWatchlists];
    return item;
  },
  async deleteWatchlist(id: string): Promise<null> {
    mockWatchlists = mockWatchlists.filter((w) => w.id !== id);
    return null;
  },
  async simulateSkillImpact(
    _userId: string,
    _targetRole: string,
    candidateSkillIds: number[],
  ): Promise<SkillImpactItem[]> {
    return candidateSkillIds.map((id, i) => {
      const meta = skillCatalog.get(id);
      return {
        skill_id: id,
        canonical_name: meta?.canonical_name ?? `Skill #${id}`,
        affected_jobs_count: Math.max(1, 3 - i),
        avg_score_delta: Number((14.2 - i * 3.1).toFixed(1)),
        jobs_crossing_75_count: i === 0 ? 2 : 1,
      };
    });
  },
  async generateGapAnalysis(_userId: string, jobId: string, _forceRefresh: boolean): Promise<GapAnalysis> {
    const found = mockJobs.find((j) => j.job_id === jobId);
    const analysis: GapAnalysis = {
      verdict_headline: "Profil teknis sangat selaras. Melengkapi skill gap utama akan langsung meningkatkan kesiapan melamar.",
      strength_highlights: ["Fondasi backend dan pengujian lokal sudah terbukti pada proyek nyata."],
      prioritized_gaps: [{ skill: "Docker", severity: "high", reason: "Dibutuhkan pada sebagian besar lowongan backend modern." }],
      cv_tailoring_tips: ["Tonjolkan metrik teknis terukur pada proyek utama di bagian atas CV."],
    };
    if (found) found.ai_gap_analysis = analysis;
    return analysis;
  },
  async listAvailableRoles(): Promise<RoleJobCount[]> {
    return [
      { normalized_role: "Backend Engineer", job_count: 24 },
      { normalized_role: "Systems Developer", job_count: 9 },
    ];
  },
  async getCompetencyMap(_userId: string, targetRole: string): Promise<CompetencyMapResponse_Serialize> {
    const ownedMap = new Map(activeProfile.skills.map((s) => [s.canonical_name.toLowerCase(), s.proficiency]));
    const rows: Array<[number, number, number]> = [
      [1, 21, 88],
      [8, 19, 79],
      [7, 18, 75],
      [5, 17, 71],
      [6, 16, 67],
      [9, 13, 54],
      [10, 10, 42],
      [12, 9, 38],
      [13, 8, 33],
    ];
    return {
      target_role: targetRole,
      total_jobs: targetRole === "Backend Engineer" ? 24 : 9,
      is_small_sample: targetRole !== "Backend Engineer",
      skills: rows.map(([id, count, pct]) => {
        const meta = skillCatalog.get(id)!;
        const prof = ownedMap.get(meta.canonical_name.toLowerCase()) ?? null;
        return {
          skill_id: id,
          canonical_name: meta.canonical_name,
          category: meta.category,
          demand_count: count,
          demand_percentage: pct,
          is_owned_by_user: prof !== null,
          user_proficiency: prof,
        };
      }),
    };
  },
  async generateLearningRoadmap(_userId: string, targetRole: string, sourceJobId: string | null, _hoursPerWeek: number): Promise<RoadmapDetail_Serialize> {
    const created: RoadmapDetail_Serialize = {
      ...mockRoadmaps[0],
      id: `rm-${Date.now()}`,
      target_role: targetRole,
      source_job_id: sourceJobId,
      title: `Roadmap ${targetRole}`,
      created_at: new Date().toISOString(),
    };
    mockRoadmaps.unshift(created);
    return created;
  },
  async listRoadmaps(_userId: string): Promise<RoadmapSummary_Serialize[]> {
    return mockRoadmaps.map((rm) => {
      const done = rm.steps.filter((s) => s.status === "done").length;
      return {
        id: rm.id,
        user_id: rm.user_id,
        target_role: rm.target_role,
        source_job_id: rm.source_job_id,
        title: rm.title,
        total_estimated_weeks: rm.total_estimated_weeks,
        total_steps: rm.steps.length,
        completed_steps: done,
        progress_percentage: rm.steps.length ? Math.round((done / rm.steps.length) * 100) : 0,
        created_at: rm.created_at,
      };
    });
  },
  async getRoadmapDetail(roadmapId: string): Promise<RoadmapDetail_Serialize> {
    return mockRoadmaps.find((r) => r.id === roadmapId) ?? mockRoadmaps[0];
  },
  async updateRoadmapStepStatus(stepId: string, status: StepStatus): Promise<StepUpdateResult_Serialize> {
    for (const rm of mockRoadmaps) {
      const step = rm.steps.find((s) => s.id === stepId);
      if (step) {
        step.status = status;
        step.completed_at = status === "done" ? new Date().toISOString() : null;
        const owned = activeProfile.skills.some((s) => s.skill_id === step.skill_id);
        return {
          step_id: step.id,
          status: step.status,
          completed_at: step.completed_at,
          suggest_add_skill:
            status === "done" && step.skill_id && step.skill_name && !owned
              ? { skill_id: step.skill_id, canonical_name: step.skill_name, category: "tools_tech" }
              : null,
        };
      }
    }
    return { step_id: stepId, status, completed_at: null, suggest_add_skill: null };
  },
  async quickAddUserSkill(_userId: string, skillId: number): Promise<null> {
    const meta = skillCatalog.get(skillId);
    if (meta && !activeProfile.skills.some((s) => s.skill_id === skillId)) {
      activeProfile.skills.push({
        skill_id: skillId,
        canonical_name: meta.canonical_name,
        category: meta.category,
        proficiency: "beginner",
        evidence: "Ditambahkan cepat dari Vecta.",
      });
    }
    return null;
  },
  async getLocalAiStatus(): Promise<LocalAiModelStatus> {
    return {
      model_id: "local:vecta-neural-v2",
      engine_tier: "neural_v2",
      is_ready: true,
      reranker_ready: true,
      semantic_index_ready: true,
      dimensions: 768,
      vocab_size: 250002,
      size_mb: 424.2,
      source_path: "resources/models/neural-v2",
    };
  },
  async rerankTopJobs(_userId: string, topN: number | null): Promise<number> {
    return Math.min(mockJobs.length, topN ?? 25);
  },
  async getScraperMode(): Promise<string> {
    return scraperModeState;
  },
  async setScraperMode(mode: string): Promise<string> {
    scraperModeState = mode === "python_worker" ? "python_worker" : "native_rust";
    return scraperModeState;
  },
  async importCvPdf(_pdfBase64: string): Promise<ImportedCvDraft> {
    return {
      extracted_text: activeProfile.raw_story_text ?? "",
      dominant_domain: "informatika_data",
      estimated_experience_level: "junior",
      professional_summary: activeProfile.summary ?? "",
      extracted_skills: activeProfile.skills.map((s) => ({
        canonical_name: s.canonical_name,
        category: s.category,
        proficiency: s.proficiency,
        evidence: "Diekstrak lokal dari dokumen CV PDF (0 token).",
      })),
      recommended_keywords: activeProfile.keywords.map((k) => ({
        role_label: k.role_label,
        fit_type: k.fit_type,
        match_reason: k.match_reason ?? "",
        scrape_queries: k.scrape_queries,
      })),
      master_cv: { ...masterCvState },
    };
  },
  async parseStoryLocal(rawStory: string): Promise<ImportedCvDraft> {
    if (rawStory.trim().length < 30) {
      throw new IpcError({ code: "VALIDATION_ERROR", message: "Cerita minimal 30 karakter.", retryable: false });
    }
    return this.importCvPdf("");
  },
  async getMasterCv(_userId: string): Promise<MasterCvProfile> {
    return { ...masterCvState };
  },
  async saveMasterCv(cv: MasterCvProfile): Promise<MasterCvProfile> {
    masterCvState = { ...cv, updated_at: new Date().toISOString() };
    return { ...masterCvState };
  },
  async exportTailoredAtsPdf(
    _userId: string,
    _jobId: string | null,
    _language: "id" | "en",
  ): Promise<AtsExportResult> {
    return {
      file_name: "CV_Budi_Santoso_ATS.pdf",
      file_path: "Documents/Vecta_CVs/CV_Budi_Santoso_ATS.pdf",
      ats_keyword_score: 86,
      matched_keywords: ["Rust", "SQLite", "REST API", "Python"],
      missing_keywords: ["Docker"],
      pdf_size_bytes: 28450,
      pdf_base64: "JVBERi0xLjQKJSBWRVNUQSBNT0NLICBQREYK",
    };
  },
  async getSalaryBenchmark(
    roleFilter: string | null,
    clusterFilter: string | null,
  ): Promise<SalaryBenchmarkSummary> {
    const filtered = mockJobs.filter((j) => {
      if (roleFilter && j.normalized_role.toLowerCase() !== roleFilter.toLowerCase()) return false;
      if (clusterFilter && j.location_cluster !== clusterFilter) return false;
      return j.salary_min_idr !== null || j.salary_max_idr !== null;
    });
    const mids = filtered
      .map((j) => {
        if (j.salary_min_idr && j.salary_max_idr) return Math.round((j.salary_min_idr + j.salary_max_idr) / 2);
        return j.salary_min_idr ?? j.salary_max_idr ?? 0;
      })
      .filter((v) => v > 0)
      .sort((a, b) => a - b);

    const med = mids.length ? mids[Math.floor(mids.length / 2)] : null;
    return {
      role_filter: roleFilter,
      cluster_filter: clusterFilter,
      sample_count: mids.length,
      is_small_sample: mids.length < 3,
      min_idr: mids[0] ?? null,
      p25_idr: mids.length ? mids[Math.floor((mids.length - 1) * 0.25)] : null,
      median_idr: med,
      p75_idr: mids.length ? mids[Math.floor((mids.length - 1) * 0.75)] : null,
      max_idr: mids[mids.length - 1] ?? null,
      by_cluster: [
        { location_cluster: "jabodetabek", sample_count: Math.max(1, mids.length), median_idr: med ?? 12_000_000 },
        { location_cluster: "bandung_jabar", sample_count: 1, median_idr: 9_500_000 },
      ],
    };
  },
  async getCompanySnapshot(_userId: string, companyName: string): Promise<CompanySnapshot> {
    const owned = new Set(activeProfile.skills.map((s) => s.canonical_name.toLowerCase()));
    const rawMatches = mockJobs.filter((j) => j.company.toLowerCase() === companyName.toLowerCase());
    const list = (rawMatches.length > 0 ? rawMatches : [mockJobs[0]]).map(computeJobDetail);
    const freq = new Map<string, number>();
    let remoteCount = 0;
    let hybridCount = 0;
    let onsiteCount = 0;
    for (const j of list) {
      if (j.work_mode === "remote") remoteCount += 1;
      else if (j.work_mode === "hybrid") hybridCount += 1;
      else if (j.work_mode === "onsite") onsiteCount += 1;
      for (const s of [...j.matched_skills, ...j.missing_skills]) {
        freq.set(s.canonical_name, (freq.get(s.canonical_name) ?? 0) + 1);
      }
    }
    return {
      company_name: companyName,
      total_jobs: list.length,
      avg_match_score: Math.round(
        (list.reduce((acc, j) => acc + (j.final_score ?? 0), 0) / list.length) * 1000,
      ) / 10,
      remote_count: remoteCount,
      hybrid_count: hybridCount,
      onsite_count: onsiteCount,
      salary_min_idr: list[0]?.salary_min_idr ?? null,
      salary_max_idr: list[0]?.salary_max_idr ?? null,
      high_risk_count: list.filter((j) => j.red_flags.length > 0).length,
      top_skills: Array.from(freq.entries()).map(([skill_name, count]) => ({
        skill_name,
        count,
        user_has: owned.has(skill_name.toLowerCase()),
      })),
      recent_titles: list.map((j) => j.title).slice(0, 5),
    };
  },
  async generateInterviewPrep(
    _userId: string,
    jobId: string,
    language: "id" | "en" = "id",
  ): Promise<InterviewPrepKit> {
    const job = computeJobDetail(mockJobs.find((j) => j.job_id === jobId) ?? mockJobs[0]);
    const primary = job.matched_skills[0]?.canonical_name ?? "Rust";
    const gap = job.missing_skills[0]?.canonical_name ?? "Docker";
    if (language === "en") {
      return {
        job_id: job.job_id,
        language: "en",
        pitch_30s: `I am a candidate for ${job.title} at ${job.company} with hands-on strength in ${primary}.`,
        questions: [
          {
            category: "Behavioral STAR",
            question: `Tell me about a challenging project where you applied ${primary}.`,
            star_hint: "Situation, Task, Action with concrete metrics.",
            focus_skill: primary,
          },
          {
            category: "Gap Mitigation",
            question: `How do you plan to ramp up on ${gap} during your first 30 days?`,
            star_hint: "Connect prior work to fast 2-week sandbox validation.",
            focus_skill: gap,
          },
        ],
        closing_questions: [`What defines 90-day success for ${job.title}?`],
      };
    }
    return {
      job_id: job.job_id,
      language: "id",
      pitch_30s: `Saya kandidat untuk posisi ${job.title} di ${job.company} dengan pengalaman langsung pada ${primary}.`,
      questions: [
        {
          category: "Behavioral STAR",
          question: `Ceritakan masalah tersulit yang Anda selesaikan memakai ${primary} beserta dampaknya.`,
          star_hint: "Jelaskan Situation, Task, Action, dan Result terukur.",
          focus_skill: primary,
        },
        {
          category: "Mitigasi Gap",
          question: `Bagaimana rencana Anda menguasai ${gap} dalam 30 hari pertama di ${job.company}?`,
          star_hint: "Hubungkan pengalaman serupa dan contoh hasil kerja cepat.",
          focus_skill: gap,
        },
      ],
      closing_questions: [`Apa tolok ukur keberhasilan 90 hari pertama untuk ${job.title}?`],
    };
  },
};
