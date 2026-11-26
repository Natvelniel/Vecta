use std::collections::{HashMap, HashSet};

use crate::models::competency::{
    ClusterSalaryStat, CompanySkillFrequency, CompanySnapshot, SalaryBenchmarkSummary,
};
use crate::models::job::WorkMode;

#[derive(Debug, Clone)]
pub struct SalarySampleRow {
    pub location_cluster: String,
    pub salary_min_idr: Option<i64>,
    pub salary_max_idr: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct CompanyJobRow {
    pub title: String,
    pub work_mode: WorkMode,
    pub salary_min_idr: Option<i64>,
    pub salary_max_idr: Option<i64>,
    pub quality_score: f64,
    pub final_score: f64,
    pub required_skills: Vec<String>,
}

fn percentile_sorted(sorted: &[i64], num: usize, den: usize) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = ((sorted.len() - 1) * num + den / 2) / den;
    sorted[idx.min(sorted.len() - 1)]
}

fn extract_midpoint(min_idr: Option<i64>, max_idr: Option<i64>) -> Option<i64> {
    match (min_idr, max_idr) {
        (Some(a), Some(b)) if a > 0 && b > 0 => Some((a + b) / 2),
        (Some(a), _) if a > 0 => Some(a),
        (_, Some(b)) if b > 0 => Some(b),
        _ => None,
    }
}

pub fn compute_salary_benchmark(
    role_filter: Option<&str>,
    cluster_filter: Option<&str>,
    rows: &[SalarySampleRow],
) -> SalaryBenchmarkSummary {
    let mut values: Vec<i64> = Vec::new();
    let mut by_cluster_map: HashMap<String, Vec<i64>> = HashMap::new();

    for row in rows {
        if let Some(mid) = extract_midpoint(row.salary_min_idr, row.salary_max_idr) {
            values.push(mid);
            let cluster = if row.location_cluster.trim().is_empty() {
                "indonesia".to_string()
            } else {
                row.location_cluster.clone()
            };
            by_cluster_map.entry(cluster).or_default().push(mid);
        }
    }

    values.sort_unstable();
    let sample_count = values.len() as i64;
    let is_small_sample = sample_count < 3;

    let (min_idr, p25_idr, median_idr, p75_idr, max_idr) = if values.is_empty() {
        (None, None, None, None, None)
    } else {
        (
            Some(values[0]),
            Some(percentile_sorted(&values, 1, 4)),
            Some(percentile_sorted(&values, 1, 2)),
            Some(percentile_sorted(&values, 3, 4)),
            Some(values[values.len() - 1]),
        )
    };

    let mut by_cluster: Vec<ClusterSalaryStat> = by_cluster_map
        .into_iter()
        .map(|(cluster, mut vals)| {
            vals.sort_unstable();
            ClusterSalaryStat {
                location_cluster: cluster,
                sample_count: vals.len() as i64,
                median_idr: percentile_sorted(&vals, 1, 2),
            }
        })
        .collect();
    by_cluster.sort_by(|a, b| {
        b.sample_count
            .cmp(&a.sample_count)
            .then_with(|| b.median_idr.cmp(&a.median_idr))
    });

    SalaryBenchmarkSummary {
        role_filter: role_filter.map(str::to_string),
        cluster_filter: cluster_filter.map(str::to_string),
        sample_count,
        is_small_sample,
        min_idr,
        p25_idr,
        median_idr,
        p75_idr,
        max_idr,
        by_cluster,
    }
}

pub fn build_company_snapshot(
    company_name: &str,
    rows: &[CompanyJobRow],
    user_skills: &[String],
) -> CompanySnapshot {
    let owned_lower: HashSet<String> = user_skills.iter().map(|s| s.to_lowercase()).collect();
    let total_jobs = rows.len() as i64;

    let mut score_sum = 0.0;
    let mut remote_count = 0i64;
    let mut hybrid_count = 0i64;
    let mut onsite_count = 0i64;
    let mut high_risk_count = 0i64;
    let mut min_salary: Option<i64> = None;
    let mut max_salary: Option<i64> = None;
    let mut skill_freq: HashMap<String, i64> = HashMap::new();
    let mut recent_titles: Vec<String> = Vec::new();

    for row in rows {
        score_sum += row.final_score;
        match row.work_mode {
            WorkMode::Remote => remote_count += 1,
            WorkMode::Hybrid => hybrid_count += 1,
            WorkMode::Onsite => onsite_count += 1,
            WorkMode::Unknown => {}
        }
        if row.quality_score < 0.6 {
            high_risk_count += 1;
        }
        if let Some(smin) = row.salary_min_idr.filter(|&v| v > 0) {
            min_salary = Some(min_salary.map_or(smin, |cur| cur.min(smin)));
        }
        if let Some(smax) = row.salary_max_idr.filter(|&v| v > 0) {
            max_salary = Some(max_salary.map_or(smax, |cur| cur.max(smax)));
        }
        if recent_titles.len() < 5 && !recent_titles.iter().any(|t| t == &row.title) {
            recent_titles.push(row.title.clone());
        }
        for sk in &row.required_skills {
            *skill_freq.entry(sk.clone()).or_insert(0) += 1;
        }
    }

    let avg_match_score = if total_jobs > 0 {
        ((score_sum / total_jobs as f64) * 1000.0).round() / 10.0
    } else {
        0.0
    };

    let mut top_skills: Vec<CompanySkillFrequency> = skill_freq
        .into_iter()
        .map(|(skill_name, count)| {
            let user_has = owned_lower.contains(&skill_name.to_lowercase());
            CompanySkillFrequency {
                skill_name,
                count,
                user_has,
            }
        })
        .collect();
    top_skills.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.skill_name.cmp(&b.skill_name))
    });
    top_skills.truncate(12);

    CompanySnapshot {
        company_name: company_name.to_string(),
        total_jobs,
        avg_match_score,
        remote_count,
        hybrid_count,
        onsite_count,
        salary_min_idr: min_salary,
        salary_max_idr: max_salary,
        high_risk_count,
        top_skills,
        recent_titles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_percentiles_and_small_sample_flag() {
        let small = compute_salary_benchmark(
            None,
            None,
            &[SalarySampleRow {
                location_cluster: "jabodetabek".into(),
                salary_min_idr: Some(8_000_000),
                salary_max_idr: Some(12_000_000),
            }],
        );
        assert_eq!(small.sample_count, 1);
        assert!(small.is_small_sample);
        assert_eq!(small.median_idr, Some(10_000_000));

        let enough = compute_salary_benchmark(
            Some("Backend Engineer"),
            None,
            &[
                SalarySampleRow {
                    location_cluster: "bandung_jabar".into(),
                    salary_min_idr: Some(6_000_000),
                    salary_max_idr: None,
                },
                SalarySampleRow {
                    location_cluster: "jabodetabek".into(),
                    salary_min_idr: Some(10_000_000),
                    salary_max_idr: None,
                },
                SalarySampleRow {
                    location_cluster: "jabodetabek".into(),
                    salary_min_idr: Some(14_000_000),
                    salary_max_idr: None,
                },
                SalarySampleRow {
                    location_cluster: "jabodetabek".into(),
                    salary_min_idr: Some(18_000_000),
                    salary_max_idr: None,
                },
            ],
        );
        assert!(!enough.is_small_sample);
        assert_eq!(enough.sample_count, 4);
        assert_eq!(enough.min_idr, Some(6_000_000));
        assert_eq!(enough.max_idr, Some(18_000_000));
        assert_eq!(enough.by_cluster[0].location_cluster, "jabodetabek");
    }

    #[test]
    fn builds_company_snapshot_with_owned_skills() {
        let snap = build_company_snapshot(
            "PT Nusantara",
            &[
                CompanyJobRow {
                    title: "Backend Engineer".into(),
                    work_mode: WorkMode::Hybrid,
                    salary_min_idr: Some(9_000_000),
                    salary_max_idr: Some(14_000_000),
                    quality_score: 0.95,
                    final_score: 0.82,
                    required_skills: vec!["Rust".into(), "Docker".into()],
                },
                CompanyJobRow {
                    title: "Platform Engineer".into(),
                    work_mode: WorkMode::Remote,
                    salary_min_idr: Some(12_000_000),
                    salary_max_idr: Some(16_000_000),
                    quality_score: 0.5,
                    final_score: 0.74,
                    required_skills: vec!["Docker".into(), "Kubernetes".into()],
                },
            ],
            &["Rust".into()],
        );
        assert_eq!(snap.total_jobs, 2);
        assert_eq!(snap.remote_count, 1);
        assert_eq!(snap.hybrid_count, 1);
        assert_eq!(snap.high_risk_count, 1);
        assert_eq!(snap.salary_min_idr, Some(9_000_000));
        assert_eq!(snap.salary_max_idr, Some(16_000_000));
        assert_eq!(snap.top_skills[0].skill_name, "Docker");
        assert_eq!(snap.top_skills[0].count, 2);
        assert!(snap.top_skills.iter().any(|s| s.skill_name == "Rust" && s.user_has));
    }
}
