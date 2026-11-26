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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct SkillImpactItem {
    pub skill_id: i64,
    pub canonical_name: String,
    pub affected_jobs_count: u32,
    pub avg_score_delta: f64,
    pub jobs_crossing_75_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ClusterSalaryStat {
    pub location_cluster: String,
    pub sample_count: i64,
    pub median_idr: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SalaryBenchmarkSummary {
    pub role_filter: Option<String>,
    pub cluster_filter: Option<String>,
    pub sample_count: i64,
    pub is_small_sample: bool,
    pub min_idr: Option<i64>,
    pub p25_idr: Option<i64>,
    pub median_idr: Option<i64>,
    pub p75_idr: Option<i64>,
    pub max_idr: Option<i64>,
    pub by_cluster: Vec<ClusterSalaryStat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CompanySkillFrequency {
    pub skill_name: String,
    pub count: i64,
    pub user_has: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct CompanySnapshot {
    pub company_name: String,
    pub total_jobs: i64,
    pub avg_match_score: f64,
    pub remote_count: i64,
    pub hybrid_count: i64,
    pub onsite_count: i64,
    pub salary_min_idr: Option<i64>,
    pub salary_max_idr: Option<i64>,
    pub high_risk_count: i64,
    pub top_skills: Vec<CompanySkillFrequency>,
    pub recent_titles: Vec<String>,
}
