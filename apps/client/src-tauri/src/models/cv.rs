use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::profile::{ExperienceLevel, ExtractedSkillDraft, RecommendedKeywordDraft};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct CvExperienceItem {
    pub role: String,
    pub company: String,
    pub location: String,
    pub period: String,
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct CvEducationItem {
    pub institution: String,
    pub degree: String,
    pub period: String,
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct CvProjectItem {
    pub name: String,
    pub period: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct MasterCvProfile {
    pub user_id: String,
    pub full_name: String,
    pub headline: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub linkedin_url: String,
    pub portfolio_url: String,
    pub summary: String,
    pub experiences: Vec<CvExperienceItem>,
    pub education: Vec<CvEducationItem>,
    pub projects: Vec<CvProjectItem>,
    pub certifications: Vec<String>,
    pub languages: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ImportedCvDraft {
    pub extracted_text: String,
    pub dominant_domain: String,
    pub estimated_experience_level: ExperienceLevel,
    pub professional_summary: String,
    pub extracted_skills: Vec<ExtractedSkillDraft>,
    pub recommended_keywords: Vec<RecommendedKeywordDraft>,
    pub master_cv: MasterCvProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AtsExportResult {
    pub file_name: String,
    pub file_path: String,
    pub ats_keyword_score: f64,
    pub matched_keywords: Vec<String>,
    pub missing_keywords: Vec<String>,
    pub pdf_size_bytes: u32,
    pub pdf_base64: String,
}
