use crate::core::db::{JobMatchRecord, JobMatchUpsert, ProfileMatchRecord};
use crate::error::AppError;
use crate::models::job::JobMatchSkillRef;
use crate::models::profile::ExperienceLevel;

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedMatch {
    pub vector_score: f64,
    pub skill_overlap_score: f64,
    pub level_score: f64,
    pub final_score: f64,
    pub matched_skills: Vec<JobMatchSkillRef>,
    pub missing_skills: Vec<JobMatchSkillRef>,
}

pub fn encode_embedding(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for &value in vec {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub fn decode_embedding(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if !value.is_finite() {
            return None;
        }
        out.push(value);
    }
    Some(out)
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.is_empty() || a.len() != b.len() {
        return None;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (&x, &y) in a.iter().zip(b.iter()) {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let xf = x as f64;
        let yf = y as f64;
        dot += xf * yf;
        norm_a += xf * xf;
        norm_b += yf * yf;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 || !denom.is_finite() {
        return None;
    }
    let raw = (dot / denom).clamp(0.0, 1.0);
    Some(raw as f32)
}

pub fn level_compatibility(user: ExperienceLevel, job: ExperienceLevel) -> f64 {
    match (user.ordinal(), job.ordinal()) {
        (Some(u), Some(j)) => match (u - j).abs() {
            0 => 1.00,
            1 => 0.65,
            2 => 0.25,
            _ => 0.00,
        },
        _ => 0.75,
    }
}

pub fn format_profile_embedding_text(
    roles: &[String],
    level: ExperienceLevel,
    summary: &str,
    skills: &[(String, String)],
) -> String {
    let roles_str = roles.join(", ");
    let skills_str = skills
        .iter()
        .map(|(name, prof)| format!("{name} ({prof})"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Role Targets: {roles_str}\nExperience Level: {}\nSummary: {summary}\nSkills: {skills_str}",
        level.as_str()
    )
}

pub fn format_job_embedding_text(
    normalized_role: &str,
    level: ExperienceLevel,
    title: &str,
    role_summary: &str,
    mandatory_skills: &[String],
    nice_to_have_skills: &[String],
) -> String {
    format!(
        "Role: {normalized_role} ({})\nTitle: {title}\nSummary: {role_summary}\nMandatory Skills: {}\nNice to Have: {}",
        level.as_str(),
        mandatory_skills.join(", "),
        nice_to_have_skills.join(", ")
    )
}

pub struct PrecomputedUserEmbedding {
    values: Vec<f32>,
    norm: f64,
}

impl PrecomputedUserEmbedding {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let values = decode_embedding(bytes)?;
        let mut norm_sq = 0.0f64;
        for &x in &values {
            let xf = x as f64;
            norm_sq += xf * xf;
        }
        let norm = norm_sq.sqrt();
        if norm == 0.0 || !norm.is_finite() {
            return None;
        }
        Some(Self { values, norm })
    }

    pub fn cosine_with_bytes(&self, job_bytes: &[u8]) -> Option<f64> {
        if job_bytes.len() != self.values.len() * 4 {
            return None;
        }
        let mut dot = 0.0f64;
        let mut norm_b_sq = 0.0f64;
        for (&ux, chunk) in self.values.iter().zip(job_bytes.chunks_exact(4)) {
            let jy = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            if !jy.is_finite() {
                return None;
            }
            let xf = ux as f64;
            let yf = jy as f64;
            dot += xf * yf;
            norm_b_sq += yf * yf;
        }
        let denom = self.norm * norm_b_sq.sqrt();
        if denom == 0.0 || !denom.is_finite() {
            return None;
        }
        Some((dot / denom).clamp(0.0, 1.0))
    }
}

pub fn compute_hybrid_match(user: &ProfileMatchRecord, job: &JobMatchRecord) -> ComputedMatch {
    let precomputed = user
        .embedding
        .as_deref()
        .and_then(PrecomputedUserEmbedding::from_bytes);
    compute_hybrid_match_with_precomputed(user, precomputed.as_ref(), job)
}

pub fn compute_hybrid_matches_batch(
    user: &ProfileMatchRecord,
    jobs: &[JobMatchRecord],
) -> Vec<ComputedMatch> {
    let precomputed = user
        .embedding
        .as_deref()
        .and_then(PrecomputedUserEmbedding::from_bytes);
    jobs.iter()
        .map(|job| compute_hybrid_match_with_precomputed(user, precomputed.as_ref(), job))
        .collect()
}

fn compute_hybrid_match_with_precomputed(
    user: &ProfileMatchRecord,
    user_emb: Option<&PrecomputedUserEmbedding>,
    job: &JobMatchRecord,
) -> ComputedMatch {
    let valid_vector = match (user_emb, job.embedding.as_deref()) {
        (Some(u_emb), Some(j_bytes)) => u_emb.cosine_with_bytes(j_bytes),
        _ => None,
    };

    let (has_valid_vector, vector_score) = match valid_vector {
        Some(v) => (true, v),
        None => (false, 0.0),
    };

    let mut matched_skills = Vec::new();
    let mut missing_skills = Vec::new();
    let mut matched_weight = 0.0f64;
    let mut total_weight = 0.0f64;

    for row in &job.skills {
        let w = if row.weight > 0.0 {
            row.weight
        } else {
            row.requirement_type.default_weight()
        };
        total_weight += w;

        let skill_ref = JobMatchSkillRef {
            skill_id: row.skill_id,
            canonical_name: row.canonical_name.clone(),
            category: row.category,
            requirement_type: row.requirement_type,
        };

        if user.skill_ids.contains(&row.skill_id) {
            matched_weight += w;
            matched_skills.push(skill_ref);
        } else {
            missing_skills.push(skill_ref);
        }
    }

    let skill_overlap_score = if total_weight == 0.0 {
        vector_score
    } else if user.skill_ids.is_empty() {
        0.0
    } else {
        (matched_weight / total_weight).clamp(0.0, 1.0)
    };

    let level_score = level_compatibility(user.experience_level, job.experience_level);

    let final_score = if has_valid_vector {
        (0.40 * vector_score) + (0.45 * skill_overlap_score) + (0.15 * level_score)
    } else {
        (0.75 * skill_overlap_score) + (0.25 * level_score)
    }
    .clamp(0.0, 1.0);

    ComputedMatch {
        vector_score,
        skill_overlap_score,
        level_score,
        final_score,
        matched_skills,
        missing_skills,
    }
}

pub fn to_match_upsert(
    user_id: &str,
    job_id: &str,
    computed: &ComputedMatch,
) -> Result<JobMatchUpsert, AppError> {
    let matched_skills_json = serde_json::to_string(&computed.matched_skills)
        .map_err(|e| AppError::db_error(format!("Gagal serialisasi matched_skills: {e}")))?;
    let missing_skills_json = serde_json::to_string(&computed.missing_skills)
        .map_err(|e| AppError::db_error(format!("Gagal serialisasi missing_skills: {e}")))?;

    Ok(JobMatchUpsert {
        user_id: user_id.to_string(),
        job_id: job_id.to_string(),
        vector_score: computed.vector_score,
        skill_overlap_score: computed.skill_overlap_score,
        level_score: computed.level_score,
        final_score: computed.final_score,
        matched_skills_json,
        missing_skills_json,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Instant;

    use super::*;
    use crate::core::db::JobSkillRow;
    use crate::models::job::RequirementType;
    use crate::models::profile::SkillCategory;

    fn make_skill_row(id: i64, name: &str, req: RequirementType) -> JobSkillRow {
        JobSkillRow {
            skill_id: id,
            canonical_name: name.into(),
            category: SkillCategory::CoreHard,
            requirement_type: req,
            weight: req.default_weight(),
        }
    }

    #[test]
    fn embedding_le_blob_roundtrip_768_dims() {
        let original: Vec<f32> = (0..768).map(|i| (i as f32) * 0.001 - 0.35).collect();
        let bytes = encode_embedding(&original);
        assert_eq!(bytes.len(), 768 * 4);

        let decoded = decode_embedding(&bytes).expect("valid blob decodes");
        assert_eq!(decoded, original);

        assert!(decode_embedding(&[]).is_none());
        assert!(decode_embedding(&[1, 2, 3, 4, 5]).is_none());

        let nan_bytes = f32::NAN.to_le_bytes();
        assert!(decode_embedding(&nan_bytes).is_none());
    }

    #[test]
    fn hybrid_formula_standard_case() {
        let user_vec = encode_embedding(&[1.0, 0.0]);
        let job_vec = encode_embedding(&[0.8, 0.6]);

        let user = ProfileMatchRecord {
            user_id: "u1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(user_vec),
            skill_ids: HashSet::from([1, 3]),
        };
        let job = JobMatchRecord {
            job_id: "j1".into(),
            experience_level: ExperienceLevel::Mid,
            embedding: Some(job_vec),
            skills: vec![
                make_skill_row(1, "Rust", RequirementType::Mandatory),
                make_skill_row(2, "Docker", RequirementType::Mandatory),
                make_skill_row(3, "Redis", RequirementType::NiceToHave),
            ],
        };

        let m = compute_hybrid_match(&user, &job);
        assert!((m.vector_score - 0.8).abs() < 1e-5);
        assert!((m.skill_overlap_score - 0.6).abs() < 1e-5);
        assert!((m.level_score - 0.65).abs() < 1e-5);
        assert!((m.final_score - 0.6875).abs() < 1e-5);
        assert_eq!(m.matched_skills.len(), 2);
        assert_eq!(m.missing_skills.len(), 1);
    }

    #[test]
    fn edge_case_zero_job_skills_falls_back_to_vector_score() {
        let user = ProfileMatchRecord {
            user_id: "u1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(encode_embedding(&[1.0, 0.0])),
            skill_ids: HashSet::from([1]),
        };
        let job = JobMatchRecord {
            job_id: "j1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(encode_embedding(&[0.6, 0.8])),
            skills: vec![],
        };

        let m = compute_hybrid_match(&user, &job);
        assert!(!m.final_score.is_nan());
        assert!((m.skill_overlap_score - m.vector_score).abs() < 1e-5);
        assert!((m.final_score - 0.66).abs() < 1e-5);
    }

    #[test]
    fn edge_case_user_has_no_skills() {
        let user = ProfileMatchRecord {
            user_id: "u1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(encode_embedding(&[1.0, 0.0])),
            skill_ids: HashSet::new(),
        };
        let job = JobMatchRecord {
            job_id: "j1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(encode_embedding(&[1.0, 0.0])),
            skills: vec![
                make_skill_row(1, "Rust", RequirementType::Mandatory),
                make_skill_row(2, "SQL", RequirementType::NiceToHave),
            ],
        };

        let m = compute_hybrid_match(&user, &job);
        assert_eq!(m.skill_overlap_score, 0.0);
        assert!(m.matched_skills.is_empty());
        assert_eq!(m.missing_skills.len(), 2);
    }

    #[test]
    fn edge_case_missing_or_mismatched_or_zero_norm_embedding_redistributes_weights() {
        let base_job_skills = vec![
            make_skill_row(1, "Rust", RequirementType::Mandatory),
            make_skill_row(2, "Docker", RequirementType::Mandatory),
        ];
        let cases: Vec<(Option<Vec<u8>>, Option<Vec<u8>>)> = vec![
            (None, Some(encode_embedding(&[1.0, 0.0]))),
            (
                Some(encode_embedding(&[1.0, 0.0])),
                Some(encode_embedding(&[1.0, 0.0, 0.0])),
            ),
            (
                Some(encode_embedding(&[0.0, 0.0])),
                Some(encode_embedding(&[1.0, 0.0])),
            ),
            (Some(vec![1, 2, 3]), Some(encode_embedding(&[1.0, 0.0]))),
        ];

        for (u_emb, j_emb) in cases {
            let user = ProfileMatchRecord {
                user_id: "u1".into(),
                experience_level: ExperienceLevel::Junior,
                embedding: u_emb,
                skill_ids: HashSet::from([1]),
            };
            let job = JobMatchRecord {
                job_id: "j1".into(),
                experience_level: ExperienceLevel::Junior,
                embedding: j_emb,
                skills: base_job_skills.clone(),
            };

            let m = compute_hybrid_match(&user, &job);
            assert!(!m.final_score.is_nan());
            assert_eq!(m.vector_score, 0.0);
            assert!((m.final_score - 0.625).abs() < 1e-5);
        }
    }

    #[test]
    fn edge_case_unknown_level_scores_075() {
        assert!(
            (level_compatibility(ExperienceLevel::Unknown, ExperienceLevel::Senior) - 0.75).abs()
                < 1e-6
        );
        assert!(
            (level_compatibility(ExperienceLevel::Junior, ExperienceLevel::Unknown) - 0.75).abs()
                < 1e-6
        );
        assert!(
            (level_compatibility(ExperienceLevel::Intern, ExperienceLevel::Senior) - 0.0).abs()
                < 1e-6
        );
        assert!(
            (level_compatibility(ExperienceLevel::Intern, ExperienceLevel::Mid) - 0.25).abs()
                < 1e-6
        );
    }

    #[test]
    fn matches_by_skill_id_not_raw_string() {
        let user = ProfileMatchRecord {
            user_id: "u1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(encode_embedding(&[1.0, 0.0])),
            skill_ids: HashSet::from([42]),
        };
        let job = JobMatchRecord {
            job_id: "j1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(encode_embedding(&[1.0, 0.0])),
            skills: vec![make_skill_row(42, "TypeScript", RequirementType::Mandatory)],
        };

        let m = compute_hybrid_match(&user, &job);
        assert!((m.skill_overlap_score - 1.0).abs() < 1e-6);
        assert_eq!(m.matched_skills[0].skill_id, 42);
    }

    #[test]
    fn performance_1000_jobs_under_100ms() {
        let vec_u: Vec<f32> = (0..768).map(|i| ((i % 13) as f32 + 1.0) * 0.1).collect();
        let vec_j: Vec<f32> = (0..768).map(|i| ((i % 11) as f32 + 1.0) * 0.1).collect();
        let user = ProfileMatchRecord {
            user_id: "u1".into(),
            experience_level: ExperienceLevel::Mid,
            embedding: Some(encode_embedding(&vec_u)),
            skill_ids: (1..=10).collect(),
        };
        let jobs: Vec<JobMatchRecord> = (0..1000)
            .map(|idx| JobMatchRecord {
                job_id: format!("job-{idx}"),
                experience_level: ExperienceLevel::Junior,
                embedding: Some(encode_embedding(&vec_j)),
                skills: (5..=15)
                    .map(|sid| make_skill_row(sid, "Skill", RequirementType::Mandatory))
                    .collect(),
            })
            .collect();

        let started = Instant::now();
        let scores: Vec<f64> = jobs
            .iter()
            .map(|j| compute_hybrid_match(&user, j).final_score)
            .collect();
        let elapsed = started.elapsed();

        assert_eq!(scores.len(), 1000);
        assert!(
            elapsed.as_millis() < 100,
            "1,000 matches took {}ms (must be < 100ms)",
            elapsed.as_millis()
        );
    }
}
