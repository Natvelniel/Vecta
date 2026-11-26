use crate::core::db::{JobMatchRecord, JobMatchUpsert, ProfileMatchRecord};
use crate::error::AppError;
use crate::models::job::JobMatchSkillRef;
use crate::models::profile::{ExperienceLevel, Proficiency};

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedMatch {
    pub vector_score: f64,
    pub skill_overlap_score: f64,
    pub level_score: f64,
    pub final_score: f64,
    pub domain_score: Option<f64>,
    pub reranker_score: Option<f64>,
    pub match_explanation: Vec<String>,
    pub matched_skills: Vec<JobMatchSkillRef>,
    pub missing_skills: Vec<JobMatchSkillRef>,
}

pub fn quantize_i8_vec(vec: &[f32]) -> (f32, Vec<i8>) {
    let max_abs = vec.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
    let scale = if max_abs > 1e-8 { max_abs / 127.0 } else { 1.0 };
    let inv = 1.0 / scale;
    let q = vec
        .iter()
        .map(|&v| (v * inv).round().clamp(-127.0, 127.0) as i8)
        .collect();
    (scale, q)
}

pub fn encode_embedding_i8(vec: &[f32]) -> Vec<u8> {
    let (scale, q) = quantize_i8_vec(vec);
    let mut out = Vec::with_capacity(4 + q.len());
    out.extend_from_slice(&scale.to_le_bytes());
    for v in q {
        out.push(v as u8);
    }
    out
}

pub fn cosine_i8_bytes(a: &[u8], b: &[u8]) -> Option<f64> {
    if a.len() < 5 || a.len() != b.len() {
        return None;
    }
    let sa = f32::from_le_bytes([a[0], a[1], a[2], a[3]]) as f64;
    let sb = f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64;
    if !sa.is_finite() || !sb.is_finite() {
        return None;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for (&ba, &bb) in a[4..].iter().zip(b[4..].iter()) {
        let xa = ((ba as i8) as f64) * sa;
        let yb = ((bb as i8) as f64) * sb;
        dot += xa * yb;
        na += xa * xa;
        nb += yb * yb;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 || !denom.is_finite() {
        return None;
    }
    Some((dot / denom).clamp(0.0, 1.0))
}

/// Calibrate raw multilingual-e5-base cosine similarity ([0.72..0.93]) into an intuitive [0.0..1.0] scale.
pub fn calibrate_e5_cosine(raw: f64) -> f64 {
    let c = raw.clamp(0.0, 1.0);
    if c <= 0.72 {
        (c / 0.72) * 0.15
    } else if c <= 0.82 {
        0.15 + ((c - 0.72) / 0.10) * 0.45
    } else if c <= 0.92 {
        0.60 + ((c - 0.82) / 0.10) * 0.35
    } else {
        (0.95 + ((c - 0.92) / 0.08) * 0.05).clamp(0.0, 1.0)
    }
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

pub fn asymmetric_level_compatibility(user: ExperienceLevel, job: ExperienceLevel) -> f64 {
    match (user.ordinal(), job.ordinal()) {
        (Some(u), Some(j)) => {
            if u == j {
                1.00
            } else if u > j {
                match u - j {
                    1 => 0.85,
                    2 => 0.60,
                    _ => 0.35,
                }
            } else {
                match j - u {
                    1 => 0.55,
                    2 => 0.20,
                    _ => 0.05,
                }
            }
        }
        _ => 0.78,
    }
}

pub fn compute_neural_domain_score(user_domain: Option<&str>, job_domain: &str) -> Option<f64> {
    let u = user_domain?.trim();
    let j = job_domain.trim();
    if u.is_empty() || j.is_empty() {
        return None;
    }
    if u.eq_ignore_ascii_case(j) {
        return Some(1.0);
    }
    if u.eq_ignore_ascii_case("umum_lintas_bidang")
        || j.eq_ignore_ascii_case("umum_lintas_bidang")
        || u.eq_ignore_ascii_case("soft_skills_bahasa_umum")
        || j.eq_ignore_ascii_case("soft_skills_bahasa_umum")
    {
        return Some(0.65);
    }
    Some(0.25)
}

fn proficiency_weight(prof: Proficiency) -> f64 {
    match prof {
        Proficiency::Advanced => 1.0,
        Proficiency::Intermediate => 0.88,
        Proficiency::Beginner => 0.68,
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
    let parent_credits = if user.skill_names_lower.is_empty() {
        None
    } else {
        Some(
            vecta_taxonomy::TaxonomyEngine::global()
                .parent_names_lower_for_owned(&user.skill_names_lower),
        )
    };
    compute_hybrid_match_with_precomputed(
        user,
        precomputed.as_ref(),
        parent_credits.as_ref(),
        job,
        None,
    )
}

/// Compute a 0-token local match score using a deterministic keyword/location relevance signal
/// when vector embeddings are not yet populated (`local_only` jobs).
pub fn compute_local_hybrid_match(
    user: &ProfileMatchRecord,
    job: &JobMatchRecord,
    local_keyword_score: f64,
) -> ComputedMatch {
    let precomputed = user
        .embedding
        .as_deref()
        .and_then(PrecomputedUserEmbedding::from_bytes);
    let parent_credits = if user.skill_names_lower.is_empty() {
        None
    } else {
        Some(
            vecta_taxonomy::TaxonomyEngine::global()
                .parent_names_lower_for_owned(&user.skill_names_lower),
        )
    };
    compute_hybrid_match_with_precomputed(
        user,
        precomputed.as_ref(),
        parent_credits.as_ref(),
        job,
        Some(local_keyword_score.clamp(0.0, 1.0)),
    )
}

pub fn compute_hybrid_matches_batch(
    user: &ProfileMatchRecord,
    jobs: &[JobMatchRecord],
) -> Vec<ComputedMatch> {
    let precomputed = user
        .embedding
        .as_deref()
        .and_then(PrecomputedUserEmbedding::from_bytes);
    let parent_credits = if user.skill_names_lower.is_empty() {
        None
    } else {
        Some(
            vecta_taxonomy::TaxonomyEngine::global()
                .parent_names_lower_for_owned(&user.skill_names_lower),
        )
    };
    jobs.iter()
        .map(|job| {
            compute_hybrid_match_with_precomputed(
                user,
                precomputed.as_ref(),
                parent_credits.as_ref(),
                job,
                None,
            )
        })
        .collect()
}

fn compute_hybrid_match_with_precomputed(
    user: &ProfileMatchRecord,
    user_emb: Option<&PrecomputedUserEmbedding>,
    parent_credits: Option<&std::collections::HashSet<String>>,
    job: &JobMatchRecord,
    local_context_fallback: Option<f64>,
) -> ComputedMatch {
    let has_multifacet = (user.emb_role.is_some() && job.emb_role.is_some())
        || (user.emb_story.is_some() && job.emb_context.is_some())
        || (user.emb_skills.is_some() && job.emb_skills.is_some());

    let mut multifacet_weighted_sum = 0.0f64;
    let mut multifacet_weight_total = 0.0f64;
    if has_multifacet {
        if let (Some(ur), Some(jr)) = (user.emb_role.as_deref(), job.emb_role.as_deref()) {
            if let Some(raw) = cosine_i8_bytes(ur, jr) {
                multifacet_weighted_sum += 0.40 * calibrate_e5_cosine(raw);
                multifacet_weight_total += 0.40;
            }
        }
        if let (Some(us), Some(jc)) = (user.emb_story.as_deref(), job.emb_context.as_deref()) {
            if let Some(raw) = cosine_i8_bytes(us, jc) {
                multifacet_weighted_sum += 0.35 * calibrate_e5_cosine(raw);
                multifacet_weight_total += 0.35;
            }
        }
        if let (Some(uk), Some(jk)) = (user.emb_skills.as_deref(), job.emb_skills.as_deref()) {
            if let Some(raw) = cosine_i8_bytes(uk, jk) {
                multifacet_weighted_sum += 0.25 * calibrate_e5_cosine(raw);
                multifacet_weight_total += 0.25;
            }
        }
    }

    let valid_vector = if multifacet_weight_total > 0.0 {
        Some((multifacet_weighted_sum / multifacet_weight_total).clamp(0.0, 1.0))
    } else {
        match (user_emb, job.embedding.as_deref()) {
            (Some(u_emb), Some(j_bytes)) => u_emb.cosine_with_bytes(j_bytes),
            _ => None,
        }
    };

    let (has_valid_vector, vector_score) = match valid_vector {
        Some(v) => (true, v),
        None => match local_context_fallback {
            Some(local_ctx) => (true, local_ctx),
            None => (false, 0.0),
        },
    };

    let use_neural_v2 = multifacet_weight_total > 0.0 || user.dominant_domain.is_some();

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
            let p_mult = if use_neural_v2 {
                user.skill_proficiencies
                    .get(&row.skill_id)
                    .copied()
                    .map(proficiency_weight)
                    .unwrap_or(1.0)
            } else {
                1.0
            };
            matched_weight += w * p_mult;
            matched_skills.push(skill_ref);
        } else {
            // Parent-Child Partial Credit (0.45x): if the job asks for a parent skill (e.g. Perpajakan or PHP)
            // and the candidate owns a direct child specialization (e.g. Brevet Pajak A & B or Laravel),
            // grant fair partial weight while still showing the parent skill in missing_skills for awareness.
            if let Some(parents) = parent_credits {
                if !parents.is_empty() && parents.contains(&row.canonical_name.to_lowercase()) {
                    matched_weight += w * 0.45;
                }
            }
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

    let level_score = if use_neural_v2 {
        asymmetric_level_compatibility(user.experience_level, job.experience_level)
    } else {
        level_compatibility(user.experience_level, job.experience_level)
    };

    let domain_score = compute_neural_domain_score(
        user.dominant_domain.as_deref(),
        &job.domain_slug,
    );

    let mut final_score = if use_neural_v2 {
        let ds = domain_score.unwrap_or(0.65);
        if has_valid_vector {
            (0.36 * vector_score)
                + (0.36 * skill_overlap_score)
                + (0.14 * level_score)
                + (0.14 * ds)
        } else {
            (0.58 * skill_overlap_score) + (0.22 * level_score) + (0.20 * ds)
        }
    } else if has_valid_vector {
        (0.40 * vector_score) + (0.45 * skill_overlap_score) + (0.15 * level_score)
    } else {
        (0.75 * skill_overlap_score) + (0.25 * level_score)
    }
    .clamp(0.0, 1.0);

    let mut match_explanation = Vec::new();
    if use_neural_v2 {
        if let Some(ds) = domain_score {
            if ds <= 0.30 && matched_skills.is_empty() {
                final_score = final_score.min(0.48);
                match_explanation.push(
                    "Lintas bidang tanpa irisan skill inti (skor dibatasi maksimal 48%).".into(),
                );
            } else if ds >= 0.99 {
                match_explanation.push("Bidang utama selaras dengan target profil Anda.".into());
            }
        }
        if !matched_skills.is_empty() {
            let names: Vec<&str> = matched_skills
                .iter()
                .take(3)
                .map(|s| s.canonical_name.as_str())
                .collect();
            match_explanation.push(format!("Skill inti cocok: {}", names.join(", ")));
        }
        if multifacet_weight_total > 0.0 && vector_score >= 0.65 {
            match_explanation.push(format!(
                "Relevansi semantik multi-faset kuat ({}%).",
                (vector_score * 100.0).round() as u32
            ));
        }
    }

    ComputedMatch {
        vector_score,
        skill_overlap_score,
        level_score,
        final_score,
        domain_score,
        reranker_score: None,
        match_explanation,
        matched_skills,
        missing_skills,
    }
}

pub fn to_match_upsert(
    user_id: &str,
    job_id: &str,
    computed: &ComputedMatch,
) -> Result<JobMatchUpsert, AppError> {
    let matched_skills_json = if computed.matched_skills.is_empty() {
        "[]".to_string()
    } else {
        serde_json::to_string(&computed.matched_skills)
            .map_err(|e| AppError::db_error(format!("Gagal serialisasi matched_skills: {e}")))?
    };
    let missing_skills_json = if computed.missing_skills.is_empty() {
        "[]".to_string()
    } else {
        serde_json::to_string(&computed.missing_skills)
            .map_err(|e| AppError::db_error(format!("Gagal serialisasi missing_skills: {e}")))?
    };

    let match_explanation_json = if computed.match_explanation.is_empty() {
        "[]".to_string()
    } else {
        serde_json::to_string(&computed.match_explanation)
            .unwrap_or_else(|_| "[]".to_string())
    };

    Ok(JobMatchUpsert {
        user_id: user_id.to_string(),
        job_id: job_id.to_string(),
        vector_score: computed.vector_score,
        skill_overlap_score: computed.skill_overlap_score,
        level_score: computed.level_score,
        final_score: computed.final_score,
        domain_score: computed.domain_score,
        reranker_score: computed.reranker_score,
        match_explanation_json,
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
            emb_role: None,
            emb_skills: None,
            emb_story: None,
            dominant_domain: None,
            skill_proficiencies: Default::default(),
            skill_ids: HashSet::from([1, 3]),
            skill_names_lower: HashSet::new(),
        };
        let job = JobMatchRecord {
            job_id: "j1".into(),
            experience_level: ExperienceLevel::Mid,
            embedding: Some(job_vec),
            emb_role: None,
            emb_skills: None,
            emb_context: None,
            domain_slug: String::new(),
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
            emb_role: None,
            emb_skills: None,
            emb_story: None,
            dominant_domain: None,
            skill_proficiencies: Default::default(),
            skill_ids: HashSet::from([1]),
            skill_names_lower: HashSet::new(),
        };
        let job = JobMatchRecord {
            job_id: "j1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(encode_embedding(&[0.6, 0.8])),
            emb_role: None,
            emb_skills: None,
            emb_context: None,
            domain_slug: String::new(),
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
            emb_role: None,
            emb_skills: None,
            emb_story: None,
            dominant_domain: None,
            skill_proficiencies: Default::default(),
            skill_ids: HashSet::new(),
            skill_names_lower: HashSet::new(),
        };
        let job = JobMatchRecord {
            job_id: "j1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(encode_embedding(&[1.0, 0.0])),
            emb_role: None,
            emb_skills: None,
            emb_context: None,
            domain_slug: String::new(),
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
                emb_role: None,
                emb_skills: None,
                emb_story: None,
                dominant_domain: None,
                skill_proficiencies: Default::default(),
                skill_ids: HashSet::from([1]),
                skill_names_lower: HashSet::new(),
            };
            let job = JobMatchRecord {
                job_id: "j1".into(),
                experience_level: ExperienceLevel::Junior,
                embedding: j_emb,
                emb_role: None,
                emb_skills: None,
                emb_context: None,
                domain_slug: String::new(),
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
            emb_role: None,
            emb_skills: None,
            emb_story: None,
            dominant_domain: None,
            skill_proficiencies: Default::default(),
            skill_ids: HashSet::from([42]),
            skill_names_lower: HashSet::new(),
        };
        let job = JobMatchRecord {
            job_id: "j1".into(),
            experience_level: ExperienceLevel::Junior,
            embedding: Some(encode_embedding(&[1.0, 0.0])),
            emb_role: None,
            emb_skills: None,
            emb_context: None,
            domain_slug: String::new(),
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
            emb_role: None,
            emb_skills: None,
            emb_story: None,
            dominant_domain: None,
            skill_proficiencies: Default::default(),
            skill_ids: (1..=10).collect(),
            skill_names_lower: HashSet::new(),
        };
        let jobs: Vec<JobMatchRecord> = (0..1000)
            .map(|idx| JobMatchRecord {
                job_id: format!("job-{idx}"),
                experience_level: ExperienceLevel::Junior,
                embedding: Some(encode_embedding(&vec_j)),
                emb_role: None,
                emb_skills: None,
                emb_context: None,
                domain_slug: String::new(),
                skills: (5..=15)
                    .map(|sid| make_skill_row(sid, "Skill", RequirementType::Mandatory))
                    .collect(),
            })
            .collect();

        let started = Instant::now();
        let scores: Vec<f64> = compute_hybrid_matches_batch(&user, &jobs)
            .into_iter()
            .map(|m| m.final_score)
            .collect();
        let elapsed = started.elapsed();

        assert_eq!(scores.len(), 1000);
        assert!(
            elapsed.as_millis() < 100,
            "1,000 matches took {}ms (must be < 100ms)",
            elapsed.as_millis()
        );
    }

    #[test]
    fn neural_v2_asymmetric_level_multifacet_and_domain_gate() {
        assert!(
            asymmetric_level_compatibility(ExperienceLevel::Senior, ExperienceLevel::Mid)
                > asymmetric_level_compatibility(ExperienceLevel::Mid, ExperienceLevel::Senior)
        );

        let same_vec = encode_embedding_i8(&vec![0.5f32; 768]);
        let mut profs = std::collections::HashMap::new();
        profs.insert(1i64, Proficiency::Advanced);

        let user = ProfileMatchRecord {
            user_id: "u1".into(),
            experience_level: ExperienceLevel::Mid,
            embedding: None,
            emb_role: Some(same_vec.clone()),
            emb_skills: Some(same_vec.clone()),
            emb_story: Some(same_vec.clone()),
            dominant_domain: Some("ekonomi_akuntansi_pajak".into()),
            skill_proficiencies: profs,
            skill_ids: HashSet::from([1]),
            skill_names_lower: HashSet::new(),
        };

        let mismatched_job = JobMatchRecord {
            job_id: "j-bad".into(),
            experience_level: ExperienceLevel::Mid,
            embedding: None,
            emb_role: Some(same_vec.clone()),
            emb_skills: Some(same_vec.clone()),
            emb_context: Some(same_vec.clone()),
            domain_slug: "informatika_data".into(),
            skills: vec![make_skill_row(99, "Kubernetes", RequirementType::Mandatory)],
        };
        let bad_match = compute_hybrid_match(&user, &mismatched_job);
        assert!(
            bad_match.final_score <= 0.48,
            "cross-domain zero-skill match must be capped <= 0.48, got {}",
            bad_match.final_score
        );
        assert!(!bad_match.match_explanation.is_empty());

        let matched_job = JobMatchRecord {
            job_id: "j-good".into(),
            experience_level: ExperienceLevel::Mid,
            embedding: None,
            emb_role: Some(same_vec.clone()),
            emb_skills: Some(same_vec.clone()),
            emb_context: Some(same_vec),
            domain_slug: "ekonomi_akuntansi_pajak".into(),
            skills: vec![make_skill_row(1, "Akuntansi", RequirementType::Mandatory)],
        };
        let good_match = compute_hybrid_match(&user, &matched_job);
        assert!(good_match.final_score > 0.90);
        assert_eq!(good_match.domain_score, Some(1.0));
    }
}
