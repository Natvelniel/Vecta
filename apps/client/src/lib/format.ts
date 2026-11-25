import type {
  ApplicationStatus,
  ExperienceLevel,
  FitType,
  JobSource,
  Proficiency,
  SkillCategory,
  WorkMode,
} from "./bindings";

export function safeNumber(value: number | null | undefined, fallback = 0): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

export function formatPercent(value: number | null | undefined): string {
  return `${Math.round(safeNumber(value))}%`;
}

export function formatScore(value: number | null | undefined): number {
  return Math.max(0, Math.min(100, Math.round(safeNumber(value))));
}

export function formatHours(hours: number | null | undefined): string {
  return `${Math.max(0, Math.round(safeNumber(hours)))} jam`;
}

export function formatProficiency(level: Proficiency): string {
  switch (level) {
    case "beginner":
      return "Dasar";
    case "intermediate":
      return "Menengah";
    case "advanced":
      return "Mahir";
  }
}

export function nextProficiency(level: Proficiency): Proficiency {
  switch (level) {
    case "beginner":
      return "intermediate";
    case "intermediate":
      return "advanced";
    case "advanced":
      return "beginner";
  }
}

export function formatExperienceLevel(level: ExperienceLevel): string {
  switch (level) {
    case "intern":
      return "Magang";
    case "junior":
      return "Junior";
    case "mid":
      return "Menengah";
    case "senior":
      return "Senior";
    case "unknown":
      return "Umum";
  }
}

export function formatWorkMode(mode: WorkMode): string {
  switch (mode) {
    case "remote":
      return "Remote";
    case "hybrid":
      return "Hybrid";
    case "onsite":
      return "Onsite";
    case "unknown":
      return "Fleksibel";
  }
}

export function formatJobSource(source: JobSource): string {
  switch (source) {
    case "linkedin":
      return "LinkedIn";
    case "glints":
      return "Glints";
    case "jobstreet":
      return "JobStreet";
    case "indeed":
      return "Indeed";
    case "kalibrr":
      return "Kalibrr";
    case "dealls":
      return "Dealls";
    case "manual":
      return "Manual";
  }
}

export function formatDomainSlug(slug: string | null | undefined): string {
  switch (slug) {
    case "informatika_data":
      return "Informatika & Data";
    case "ekonomi_akuntansi_pajak":
      return "Akuntansi & Keuangan";
    case "bisnis_marketing_sales":
      return "Marketing & Sales";
    case "hr_psikologi_admin_ops":
      return "HR, Admin & Ops";
    case "teknik_sipil_arsitektur_gis":
      return "Teknik Sipil & Arsitektur";
    case "teknik_mesin_elektro_industri":
      return "Mesin, Elektro & K3";
    case "kesehatan_farmasi_keperawatan":
      return "Kesehatan & Farmasi";
    case "hukum_regulasi_kepatuhan":
      return "Hukum & Kepatuhan";
    case "komunikasi_desain_media":
      return "Desain & Media";
    case "pertanian_pangan_lingkungan":
      return "Pangan & Lingkungan";
    case "pendidikan_pelatihan":
      return "Pendidikan & Pelatihan";
    case "pariwisata_perhotelan_fnb":
      return "Perhotelan & F&B";
    case "sains_lab_geologi_energi":
      return "Sains & Laboratorium";
    default:
      return "Lintas Bidang";
  }
}

export function formatApplicationStatus(status: ApplicationStatus): string {
  switch (status) {
    case "new":
      return "Baru";
    case "saved":
      return "Disimpan";
    case "applied":
      return "Sudah Dilamar";
    case "interview":
      return "Interview";
    case "offer":
      return "Offer";
    case "rejected":
      return "Ditolak";
    case "dismissed":
      return "Disembunyikan";
  }
}

export function formatSkillCategory(cat: SkillCategory): string {
  switch (cat) {
    case "core_hard":
      return "Inti Teknis";
    case "tools_tech":
      return "Alat & Infra";
    case "soft_skill":
      return "Komunikasi";
    case "domain":
      return "Domain";
  }
}

export function formatFitType(fit: FitType): string {
  switch (fit) {
    case "primary_fit":
      return "Cocok Utama";
    case "adjacent_fit":
      return "Peran Terkait";
    case "custom":
      return "Kustom";
  }
}

export function formatRelativeDate(iso: string | null | undefined): string {
  if (!iso) return "Baru saja";
  const parsed = Date.parse(iso);
  if (Number.isNaN(parsed)) return iso;
  const diffMs = Date.now() - parsed;
  const diffMin = Math.floor(diffMs / 60_000);
  if (diffMin < 1) return "Baru saja";
  if (diffMin < 60) return `${diffMin} menit lalu`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr} jam lalu`;
  const diffDays = Math.floor(diffHr / 24);
  if (diffDays < 30) return `${diffDays} hari lalu`;
  return new Date(parsed).toLocaleDateString("id-ID", {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}
