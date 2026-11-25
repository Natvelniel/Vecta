use serde::{Deserialize, Serialize};
use specta::Type;

use crate::core::native_scraper::quality::{JobRedFlag, PortalLinkRef};
use crate::models::profile::{ExperienceLevel, SkillCategory};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum JobSource {
    Linkedin,
    Glints,
    Jobstreet,
    Indeed,
    Kalibrr,
    Dealls,
    Manual,
}

impl JobSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Linkedin => "linkedin",
            Self::Glints => "glints",
            Self::Jobstreet => "jobstreet",
            Self::Indeed => "indeed",
            Self::Kalibrr => "kalibrr",
            Self::Dealls => "dealls",
            Self::Manual => "manual",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "linkedin" => Self::Linkedin,
            "glints" => Self::Glints,
            "jobstreet" => Self::Jobstreet,
            "indeed" => Self::Indeed,
            "kalibrr" => Self::Kalibrr,
            "dealls" => Self::Dealls,
            _ => Self::Manual,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum EnrichmentStatus {
    LocalOnly,
    AiEnriched,
}

impl EnrichmentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalOnly => "local_only",
            Self::AiEnriched => "ai_enriched",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "local_only" => Self::LocalOnly,
            _ => Self::AiEnriched,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationStatus {
    New,
    Saved,
    Applied,
    Interview,
    Offer,
    Rejected,
    Dismissed,
}

impl ApplicationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Saved => "saved",
            Self::Applied => "applied",
            Self::Interview => "interview",
            Self::Offer => "offer",
            Self::Rejected => "rejected",
            Self::Dismissed => "dismissed",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "saved" => Self::Saved,
            "applied" => Self::Applied,
            "interview" => Self::Interview,
            "offer" => Self::Offer,
            "rejected" => Self::Rejected,
            "dismissed" => Self::Dismissed,
            _ => Self::New,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkMode {
    Onsite,
    Hybrid,
    Remote,
    Unknown,
}

impl WorkMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Onsite => "onsite",
            Self::Hybrid => "hybrid",
            Self::Remote => "remote",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "onsite" => Self::Onsite,
            "hybrid" => Self::Hybrid,
            "remote" => Self::Remote,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RequirementType {
    Mandatory,
    NiceToHave,
}

impl RequirementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mandatory => "mandatory",
            Self::NiceToHave => "nice_to_have",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "nice_to_have" => Self::NiceToHave,
            _ => Self::Mandatory,
        }
    }

    pub fn default_weight(&self) -> f64 {
        match self {
            Self::Mandatory => 1.0,
            Self::NiceToHave => 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExtractedJobSkill {
    pub canonical_name: String,
    pub category: SkillCategory,
    pub requirement_type: RequirementType,
    #[serde(default)]
    pub matched_aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct JobExtractionResult {
    pub normalized_role: String,
    pub experience_level: ExperienceLevel,
    pub work_mode: WorkMode,
    pub role_summary: String,
    pub skills: Vec<ExtractedJobSkill>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum GapSeverity {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PrioritizedGap {
    pub skill: String,
    pub severity: GapSeverity,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GapAnalysis {
    pub verdict_headline: String,
    pub strength_highlights: Vec<String>,
    pub prioritized_gaps: Vec<PrioritizedGap>,
    pub cv_tailoring_tips: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ApplyKitResult {
    pub language: String,
    pub tailored_headline: String,
    pub cv_bullets: Vec<String>,
    pub cover_note: String,
    pub interview_angles: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
pub struct JobFilterInput {
    pub normalized_role: Option<String>,
    pub work_mode: Option<WorkMode>,
    pub source: Option<JobSource>,
    pub saved_only: Option<bool>,
    pub min_score: Option<f64>,
    pub search_query: Option<String>,
    #[serde(default)]
    pub location_query: Option<String>,
    #[serde(default)]
    pub enrichment_status: Option<EnrichmentStatus>,
    #[serde(default)]
    pub application_status: Option<ApplicationStatus>,
    #[serde(default)]
    pub include_dismissed: Option<bool>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct JobMatchSkillRef {
    pub skill_id: i64,
    pub canonical_name: String,
    pub category: SkillCategory,
    pub requirement_type: RequirementType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct JobMatchSummary {
    pub job_id: String,
    pub source: JobSource,
    pub url: Option<String>,
    pub title: String,
    pub normalized_role: String,
    pub company: String,
    pub location: Option<String>,
    pub work_mode: WorkMode,
    pub experience_level: ExperienceLevel,
    pub enrichment_status: EnrichmentStatus,
    pub posted_at: Option<String>,
    pub salary_text: Option<String>,
    pub domain_slug: String,
    pub quality_score: f64,
    pub red_flags_count: u32,
    pub also_available_on: Vec<PortalLinkRef>,
    pub vector_score: f64,
    pub skill_overlap_score: f64,
    pub level_score: f64,
    pub final_score: f64,
    pub matched_skills_count: u32,
    pub missing_skills_count: u32,
    pub has_gap_analysis: bool,
    pub is_saved: bool,
    pub application_status: ApplicationStatus,
    pub applied_at: Option<String>,
    pub scraped_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct JobMatchDetail {
    pub job_id: String,
    pub source: JobSource,
    pub external_id: String,
    pub url: Option<String>,
    pub title: String,
    pub normalized_role: String,
    pub company: String,
    pub location: Option<String>,
    pub work_mode: WorkMode,
    pub experience_level: ExperienceLevel,
    pub enrichment_status: EnrichmentStatus,
    pub posted_at: Option<String>,
    pub salary_text: Option<String>,
    pub domain_slug: String,
    pub quality_score: f64,
    pub red_flags: Vec<JobRedFlag>,
    pub also_available_on: Vec<PortalLinkRef>,
    pub raw_description: String,
    pub role_summary: Option<String>,
    pub vector_score: f64,
    pub skill_overlap_score: f64,
    pub level_score: f64,
    pub final_score: f64,
    pub matched_skills: Vec<JobMatchSkillRef>,
    pub missing_skills: Vec<JobMatchSkillRef>,
    pub ai_gap_analysis: Option<GapAnalysis>,
    pub ai_apply_kit: Option<ApplyKitResult>,
    pub is_saved: bool,
    pub application_status: ApplicationStatus,
    pub applied_at: Option<String>,
    pub user_notes: Option<String>,
    pub scraped_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ScrapeWarningItem {
    pub source: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ScrapeRunReport {
    pub total_fetched: u32,
    pub newly_ingested: u32,
    pub skipped_duplicates: u32,
    pub failed_extractions: u32,
    pub pending_local_count: u32,
    pub warnings: Vec<ScrapeWarningItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct AiMatchRunReport {
    pub processed_jobs: u32,
    pub enriched_jobs: u32,
    pub failed_jobs: u32,
    pub remaining_local_jobs: u32,
    pub api_calls_used: u32,
    pub warnings: Vec<ScrapeWarningItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SearchPreferences {
    pub location: String,
    pub sources: Vec<String>,
    pub limit_per_query: u32,
    pub max_pages: u32,
    pub posted_within_days: Option<u32>,
    pub work_mode: String,
    pub custom_queries: Vec<String>,
    pub ai_enrich_enabled: bool,
    pub ai_enrich_max_jobs: u32,
    pub ai_rpm_limit: u32,
}

impl Default for SearchPreferences {
    fn default() -> Self {
        Self {
            location: "Indonesia".into(),
            sources: vec!["jobstreet".into(), "linkedin".into(), "indeed".into()],
            limit_per_query: 25,
            max_pages: 2,
            posted_within_days: Some(14),
            work_mode: "any".into(),
            custom_queries: vec![],
            ai_enrich_enabled: false,
            ai_enrich_max_jobs: 15,
            ai_rpm_limit: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ScrapeProgressEvent {
    pub stage: String,
    pub message: String,
    pub current: u32,
    pub total: u32,
}
