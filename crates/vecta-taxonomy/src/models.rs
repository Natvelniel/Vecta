use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaxonomySkill {
    pub id: String,
    pub canonical_name: String,
    pub domain: String,
    pub category: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub requires_domain_context: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementLevel {
    Mandatory,
    NiceToHave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedSkill {
    pub skill_id: String,
    pub canonical_name: String,
    pub domain: String,
    pub category: String,
    pub parent_canonical_name: Option<String>,
    pub requirement: RequirementLevel,
    pub matched_in_title: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentSections {
    pub requirements: String,
    pub responsibilities: String,
    pub nice_to_have: String,
    pub about_company: String,
    pub general: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub dominant_domain: String,
    pub normalized_role: String,
    pub skills: Vec<ExtractedSkill>,
}
