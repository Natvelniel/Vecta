use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceLevel {
    Intern,
    Junior,
    Mid,
    Senior,
    Unknown,
}

impl ExperienceLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Intern => "intern",
            Self::Junior => "junior",
            Self::Mid => "mid",
            Self::Senior => "senior",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "intern" => Self::Intern,
            "junior" => Self::Junior,
            "mid" => Self::Mid,
            "senior" => Self::Senior,
            _ => Self::Unknown,
        }
    }

    pub fn ordinal(&self) -> Option<i32> {
        match self {
            Self::Intern => Some(0),
            Self::Junior => Some(1),
            Self::Mid => Some(2),
            Self::Senior => Some(3),
            Self::Unknown => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    CoreHard,
    ToolsTech,
    SoftSkill,
    Domain,
}

impl SkillCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CoreHard => "core_hard",
            Self::ToolsTech => "tools_tech",
            Self::SoftSkill => "soft_skill",
            Self::Domain => "domain",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "core_hard" => Self::CoreHard,
            "tools_tech" => Self::ToolsTech,
            "soft_skill" => Self::SoftSkill,
            "domain" => Self::Domain,
            _ => Self::CoreHard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Proficiency {
    Beginner,
    Intermediate,
    Advanced,
}

impl Proficiency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Beginner => "beginner",
            Self::Intermediate => "intermediate",
            Self::Advanced => "advanced",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "beginner" => Self::Beginner,
            "intermediate" => Self::Intermediate,
            "advanced" => Self::Advanced,
            _ => Self::Intermediate,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum FitType {
    PrimaryFit,
    AdjacentFit,
    Custom,
}

impl FitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PrimaryFit => "primary_fit",
            Self::AdjacentFit => "adjacent_fit",
            Self::Custom => "custom",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "primary_fit" => Self::PrimaryFit,
            "adjacent_fit" => Self::AdjacentFit,
            _ => Self::Custom,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ApiKeyStatus {
    pub has_key: bool,
    pub masked_suffix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct WorkerHealth {
    pub status: String,
    pub version: String,
    pub available_sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExtractedSkillDraft {
    pub canonical_name: String,
    pub category: SkillCategory,
    pub proficiency: Proficiency,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RecommendedKeywordDraft {
    pub role_label: String,
    pub fit_type: FitType,
    pub match_reason: String,
    pub scrape_queries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct StoryProfileDraft {
    pub professional_summary: String,
    pub estimated_experience_level: ExperienceLevel,
    pub extracted_skills: Vec<ExtractedSkillDraft>,
    pub recommended_keywords: Vec<RecommendedKeywordDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ConfirmedSkillInput {
    pub canonical_name: String,
    pub category: SkillCategory,
    pub proficiency: Proficiency,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ConfirmedKeywordInput {
    pub role_label: String,
    pub fit_type: FitType,
    pub scrape_queries: Vec<String>,
    pub match_reason: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ConfirmedProfileInput {
    pub id: Option<String>,
    pub profile_name: String,
    pub target_roles: Vec<String>,
    pub raw_story_text: String,
    pub summary: String,
    pub experience_level: ExperienceLevel,
    pub skills: Vec<ConfirmedSkillInput>,
    pub keywords: Vec<ConfirmedKeywordInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct UserSkillItem {
    pub skill_id: i64,
    pub canonical_name: String,
    pub category: SkillCategory,
    pub proficiency: Proficiency,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ScrapeKeywordItem {
    pub id: i64,
    pub user_id: String,
    pub role_label: String,
    pub fit_type: FitType,
    pub scrape_queries: Vec<String>,
    pub match_reason: Option<String>,
    pub is_active: bool,
    pub last_scraped_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ProfileSummaryItem {
    pub id: String,
    pub profile_name: String,
    pub target_roles: Vec<String>,
    pub experience_level: ExperienceLevel,
    pub skill_count: i64,
    pub is_active: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct UserProfileDetail {
    pub id: String,
    pub profile_name: String,
    pub target_roles: Vec<String>,
    pub raw_story_text: Option<String>,
    pub summary: Option<String>,
    pub experience_level: ExperienceLevel,
    pub has_embedding: bool,
    pub is_active: bool,
    pub skills: Vec<UserSkillItem>,
    pub keywords: Vec<ScrapeKeywordItem>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ProdiRoleItem {
    pub id: String,
    pub prodi_label: String,
    pub domain: String,
    pub role_title: String,
    pub mandatory_skills: Vec<String>,
    pub preferred_skills: Vec<String>,
    pub certifications: Vec<String>,
    pub scrape_queries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VectaBackupReport {
    pub schema_version: u32,
    pub exported_at: String,
    pub file_name: String,
    pub payload_json: String,
    pub profiles_count: u32,
    pub watchlists_count: u32,
    pub tracked_jobs_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VectaImportReport {
    pub imported_profiles: u32,
    pub imported_watchlists: u32,
    pub imported_preferences: bool,
    pub warnings: Vec<String>,
}
