use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::profile::{Proficiency, SkillCategory};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RoleJobCount {
    pub normalized_role: String,
    pub job_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct CompetencySkillItem {
    pub skill_id: i64,
    pub canonical_name: String,
    pub category: SkillCategory,
    pub demand_count: u32,
    pub demand_percentage: f64,
    pub is_owned_by_user: bool,
    pub user_proficiency: Option<Proficiency>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct CompetencyMapResponse {
    pub target_role: String,
    pub total_jobs: u32,
    pub is_small_sample: bool,
    pub skills: Vec<CompetencySkillItem>,
}
