use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::profile::SkillCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Todo,
    InProgress,
    Done,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "done" => Self::Done,
            _ => Self::Todo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GeneratedRoadmapStep {
    pub phase_order: i32,
    pub phase_name: String,
    pub target_skill: String,
    pub topic_title: String,
    pub learning_keywords: Vec<String>,
    pub mini_project_idea: String,
    pub estimated_hours: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct GeneratedRoadmap {
    pub title: String,
    pub target_role: String,
    pub total_estimated_weeks: i32,
    pub steps: Vec<GeneratedRoadmapStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SkillRef {
    pub skill_id: i64,
    pub canonical_name: String,
    pub category: SkillCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RoadmapStepItem {
    pub id: String,
    pub roadmap_id: String,
    pub skill_id: Option<i64>,
    pub skill_name: Option<String>,
    pub phase_order: i32,
    pub phase_name: String,
    pub topic_title: String,
    pub learning_keywords: Vec<String>,
    pub mini_project_idea: Option<String>,
    pub estimated_hours: i32,
    pub status: StepStatus,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct RoadmapSummary {
    pub id: String,
    pub user_id: String,
    pub target_role: String,
    pub source_job_id: Option<String>,
    pub title: String,
    pub total_estimated_weeks: i32,
    pub total_steps: u32,
    pub completed_steps: u32,
    pub progress_percentage: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RoadmapDetail {
    pub id: String,
    pub user_id: String,
    pub target_role: String,
    pub source_job_id: Option<String>,
    pub title: String,
    pub total_estimated_weeks: i32,
    pub steps: Vec<RoadmapStepItem>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct StepUpdateResult {
    pub step_id: String,
    pub status: StepStatus,
    pub completed_at: Option<String>,
    pub suggest_add_skill: Option<SkillRef>,
}
