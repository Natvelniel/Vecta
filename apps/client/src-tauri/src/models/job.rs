use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::profile::{ExperienceLevel, SkillCategory};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum JobSource {
    Linkedin,
    Glints,
    Jobstreet,
    Manual,
}

impl JobSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Linkedin => "linkedin",
            Self::Glints => "glints",
            Self::Jobstreet => "jobstreet",
            Self::Manual => "manual",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "linkedin" => Self::Linkedin,
            "glints" => Self::Glints,
            "jobstreet" => Self::Jobstreet,
            _ => Self::Manual,
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
pub struct JobFilterInput {
    pub normalized_role: Option<String>,
    pub work_mode: Option<WorkMode>,
    pub source: Option<JobSource>,
    pub saved_only: Option<bool>,
    pub min_score: Option<f64>,
    pub search_query: Option<String>,
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
    pub vector_score: f64,
    pub skill_overlap_score: f64,
    pub level_score: f64,
    pub final_score: f64,
    pub matched_skills_count: u32,
    pub missing_skills_count: u32,
    pub has_gap_analysis: bool,
    pub is_saved: bool,
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
    pub raw_description: String,
    pub role_summary: Option<String>,
    pub vector_score: f64,
    pub skill_overlap_score: f64,
    pub level_score: f64,
    pub final_score: f64,
    pub matched_skills: Vec<JobMatchSkillRef>,
    pub missing_skills: Vec<JobMatchSkillRef>,
    pub ai_gap_analysis: Option<GapAnalysis>,
    pub is_saved: bool,
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
    pub warnings: Vec<ScrapeWarningItem>,
}
