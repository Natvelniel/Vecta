use std::collections::{HashMap, HashSet};

use vecta_taxonomy::TaxonomyEngine;

use crate::core::cv_import::infer_city_from_text;
use crate::core::local_embedder::LocalEmbedder;
use crate::core::local_matcher::infer_experience_level;
use crate::core::matching::calibrate_e5_cosine;
use crate::error::AppError;
use crate::models::cv::{CvExperienceItem, ImportedCvDraft, MasterCvProfile};
use crate::models::profile::{
    ExperienceLevel, ExtractedSkillDraft, FitType, Proficiency, RecommendedKeywordDraft,
    SkillCategory, StoryProfileDraft,
};

const NEGATION_TRIGGERS: &[&str] = &[
    "belum pernah",
    "tidak pernah",
    "tak pernah",
    "gak pernah",
    "ga pernah",
    "nggak pernah",
    "belum bisa",
    "tidak bisa",
    "belum menguasai",
    "tidak menguasai",
    "tidak minat",
    "tidak tertarik",
    "kurang berminat",
    "kurang suka",
    "tidak suka",
    "hindari",
    "bukan bidang",
    "bukan ",
    "kecuali ",
    "no experience",
    "never used",
    "never worked",
    "not interested",
    "dislike",
    "avoid ",
];

const LEARNING_TRIGGERS: &[&str] = &[
    "sedang belajar",
    "lagi belajar",
    "baru belajar",
    "mulai belajar",
    "masih belajar",
    "masih pemula",
    "dasar-dasar",
    "dasar ",
    "pernah coba",
    "sedikit paham",
    "kenal dasar",
    "currently learning",
    "learning ",
    "beginner",
    "basic ",
    "familiar with",
];

const ADVANCED_TRIGGERS: &[&str] = &[
    "mahir",
    "ahli",
    "menguasai",
    "berpengalaman",
    "tiap hari",
    "setiap hari",
    "harian",
    "rutin",
    "bertahun-tahun",
    "memimpin",
    "menangani penuh",
    "production",
    "end-to-end",
    "expert",
    "proficient",
    "advanced",
    "senior",
];

const CONTRAST_MARKERS: &[&str] = &[
    " walaupun ",
    " meskipun ",
    " sedangkan ",
    " melainkan ",
    " tetapi ",
    " namun ",
    " namun,",
    " tapi ",
    " tapi,",
    " though ",
    " although ",
    " whereas ",
    " however ",
    " but ",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClauseStance {
    Negated,
    Learning,
    Standard,
    Advanced,
}

#[derive(Debug, Clone)]
struct AnalyzedClause {
    text: String,
    stance: ClauseStance,
}

fn contains_any(lower: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| lower.contains(p))
}

fn split_on_contrast_markers(segment: &str) -> Vec<String> {
    let mut parts = vec![segment.trim().to_string()];
    for &marker in CONTRAST_MARKERS {
        let mut next_parts = Vec::new();
        for p in parts {
            let lower = p.to_lowercase();
            let mut start = 0usize;
            let mut found = false;
            while let Some(rel_idx) = lower[start..].find(marker) {
                let abs_idx = start + rel_idx;
                let left = p[start..abs_idx].trim();
                if !left.is_empty() {
                    next_parts.push(left.to_string());
                }
                start = abs_idx + marker.len();
                found = true;
            }
            if found {
                let tail = p[start..].trim();
                if !tail.is_empty() {
                    next_parts.push(tail.to_string());
                }
            } else if !p.is_empty() {
                next_parts.push(p);
            }
        }
        parts = next_parts;
    }
    parts
}

fn split_on_stance_shifts(segment: &str) -> Vec<String> {
    const JOINERS: &[&str] = &[" dan ", " serta ", " and "];
    const SHIFT_PREFIXES: &[&str] = &[
        "sedang ",
        "lagi ",
        "baru ",
        "mulai ",
        "masih ",
        "belum ",
        "tidak ",
        "tak ",
        "gak ",
        "ga ",
        "nggak ",
        "bukan ",
        "kurang ",
        "currently ",
        "learning ",
        "never ",
        "not ",
    ];
    let mut parts = vec![segment.trim().to_string()];
    for &joiner in JOINERS {
        let mut next = Vec::new();
        for p in parts {
            let lower = p.to_lowercase();
            let mut start = 0usize;
            let mut last_cut = 0usize;
            let mut did_cut = false;
            while let Some(rel) = lower[start..].find(joiner) {
                let abs = start + rel;
                let after = lower[abs + joiner.len()..].trim_start();
                if SHIFT_PREFIXES.iter().any(|sp| after.starts_with(sp)) {
                    let left = p[last_cut..abs].trim();
                    if !left.is_empty() {
                        next.push(left.to_string());
                    }
                    last_cut = abs + joiner.len();
                    did_cut = true;
                }
                start = abs + joiner.len();
            }
            if did_cut {
                let tail = p[last_cut..].trim();
                if !tail.is_empty() {
                    next.push(tail.to_string());
                }
            } else if !p.is_empty() {
                next.push(p);
            }
        }
        parts = next;
    }
    parts
}

fn split_clauses_with_stance(text: &str) -> Vec<AnalyzedClause> {
    let mut raw_segments = Vec::new();
    for line in text.lines() {
        for sentence in line.split(['.', ';', '!', '?']) {
            let trimmed = sentence.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Also split on comma when the right side starts with negation or contrast
            for chunk in split_on_contrast_markers(trimmed) {
                for comma_part in chunk.split(',') {
                    for piece in split_on_stance_shifts(comma_part) {
                        let cp = piece.trim();
                        if !cp.is_empty() {
                            raw_segments.push(cp.to_string());
                        }
                    }
                }
            }
        }
    }

    // Re-group adjacent non-negated comma parts if they are tiny skill enumerations unless negated
    let mut clauses = Vec::new();
    for seg in raw_segments {
        let lower = format!(" {} ", seg.to_lowercase());
        let stance = if contains_any(&lower, NEGATION_TRIGGERS) {
            ClauseStance::Negated
        } else if contains_any(&lower, LEARNING_TRIGGERS) {
            ClauseStance::Learning
        } else if contains_any(&lower, ADVANCED_TRIGGERS) {
            ClauseStance::Advanced
        } else {
            ClauseStance::Standard
        };
        clauses.push(AnalyzedClause { text: seg, stance });
    }
    clauses
}

fn truncate_snippet(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let prefix: String = trimmed.chars().take(max_chars.saturating_sub(3)).collect();
    format!("{prefix}...")
}

fn infer_story_experience_level(raw_story: &str) -> ExperienceLevel {
    let lower = raw_story.to_lowercase();

    // Detect explicit academic context so "kuliah 4 tahun" isn't counted as 4 YOE senior
    let is_fresh_or_intern = lower.contains("fresh graduate")
        || lower.contains("lulusan baru")
        || lower.contains("baru lulus")
        || lower.contains("mahasiswa")
        || lower.contains("magang")
        || lower.contains("kampus merdeka")
        || lower.contains("msib")
        || lower.contains("praktik kerja lapangan")
        || lower.contains("pkl");

    // Parse "<num> tahun" / "<num> year"
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '.' && c != '+')
        .filter(|t| !t.is_empty())
        .collect();
    let mut max_work_years = 0.0f32;
    for win in tokens.windows(2) {
        let num_str = win[0].trim_end_matches('+');
        let unit = win[1];
        if matches!(unit, "tahun" | "thn" | "year" | "years" | "yrs" | "yr") {
            if let Ok(val) = num_str.replace(',', ".").parse::<f32>() {
                // Ignore 4-year degree phrasing nearby
                let context_is_edu = lower.contains(&format!("kuliah {num_str}"))
                    || lower.contains(&format!("studi {num_str}"))
                    || lower.contains(&format!("s1 {num_str}"));
                if !context_is_edu && val > max_work_years && val <= 40.0 {
                    max_work_years = val;
                }
            }
        }
    }

    if max_work_years >= 4.0
        || lower.contains("senior ")
        || lower.contains("head of ")
        || lower.contains("team lead")
        || lower.contains("supervisor")
        || lower.contains("manager")
    {
        if !is_fresh_or_intern || max_work_years >= 4.0 {
            return ExperienceLevel::Senior;
        }
    }
    if max_work_years >= 2.0 || lower.contains("mid-level") || lower.contains("menengah") {
        return ExperienceLevel::Mid;
    }
    if is_fresh_or_intern {
        if lower.contains("magang") || lower.contains("intern") || lower.contains("pkl") {
            return ExperienceLevel::Intern;
        }
        return ExperienceLevel::Junior;
    }
    if max_work_years > 0.0 || lower.contains("bulan") || lower.contains("months") {
        return ExperienceLevel::Junior;
    }

    match infer_experience_level("", raw_story) {
        ExperienceLevel::Unknown => ExperienceLevel::Junior,
        other => other,
    }
}

fn map_category(cat: &str) -> SkillCategory {
    match cat {
        "core_hard" => SkillCategory::CoreHard,
        "tools_tech" => SkillCategory::ToolsTech,
        "soft_skill" => SkillCategory::SoftSkill,
        "domain" => SkillCategory::Domain,
        _ => SkillCategory::CoreHard,
    }
}

fn stance_to_proficiency(stance: ClauseStance) -> Proficiency {
    match stance {
        ClauseStance::Advanced => Proficiency::Advanced,
        ClauseStance::Learning => Proficiency::Beginner,
        ClauseStance::Standard | ClauseStance::Negated => Proficiency::Intermediate,
    }
}

pub fn analyze_story(
    raw_story: &str,
    embedder: Option<&LocalEmbedder>,
) -> Result<(StoryProfileDraft, String, Vec<String>), AppError> {
    let cleaned = raw_story.trim();
    if cleaned.chars().count() < 30 {
        return Err(AppError::validation_error(
            "Cerita kemampuan minimal 30 karakter agar analisis profil berjalan akurat.",
        ));
    }

    let tax = TaxonomyEngine::global();
    let clauses = split_clauses_with_stance(cleaned);

    // Collect excluded/negated skills first
    let mut negated_skills_lower: HashSet<String> = HashSet::new();
    let mut affirmative_clauses: Vec<&AnalyzedClause> = Vec::new();
    for cl in &clauses {
        if cl.stance == ClauseStance::Negated {
            let neg_ext = tax.extract_from_profile_text(&cl.text);
            for s in neg_ext.skills {
                negated_skills_lower.insert(s.canonical_name.to_lowercase());
            }
        } else {
            affirmative_clauses.push(cl);
        }
    }

    let affirmed_full_text: String = if affirmative_clauses.is_empty() {
        cleaned.to_string()
    } else {
        affirmative_clauses
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(". ")
    };

    // Map lowercase canonical -> Draft (keep highest proficiency & clearest evidence)
    let mut skill_map: HashMap<String, (ExtractedSkillDraft, u8, String)> = HashMap::new();

    // Path A: Clause-scoped Aho-Corasick taxonomy extraction
    for cl in &affirmative_clauses {
        let ext = tax.extract_from_profile_text(&cl.text);
        let prof = stance_to_proficiency(cl.stance);
        let prof_rank = match prof {
            Proficiency::Beginner => 1,
            Proficiency::Intermediate => 2,
            Proficiency::Advanced => 3,
        };
        let evidence = format!(
            "Terdeteksi ({}): \"{}\"",
            match prof {
                Proficiency::Beginner => "sedang dipelajari",
                Proficiency::Intermediate => "diterapkan",
                Proficiency::Advanced => "dikuasai",
            },
            truncate_snippet(&cl.text, 78)
        );
        for sk in ext.skills {
            let key = sk.canonical_name.to_lowercase();
            if negated_skills_lower.contains(&key) {
                continue;
            }
            let replace = skill_map
                .get(&key)
                .map(|(_, r, _)| prof_rank > *r)
                .unwrap_or(true);
            if replace {
                skill_map.insert(
                    key,
                    (
                        ExtractedSkillDraft {
                            canonical_name: sk.canonical_name,
                            category: map_category(&sk.category),
                            proficiency: prof,
                            evidence: evidence.clone(),
                        },
                        prof_rank,
                        sk.domain,
                    ),
                );
            }
        }
    }

    // Full-affirmed-text fallback catch for multi-clause phrasing missed by comma splits
    let full_ext = tax.extract_from_profile_text(&affirmed_full_text);
    for sk in full_ext.skills {
        let key = sk.canonical_name.to_lowercase();
        if negated_skills_lower.contains(&key) || skill_map.contains_key(&key) {
            continue;
        }
        skill_map.insert(
            key,
            (
                ExtractedSkillDraft {
                    canonical_name: sk.canonical_name,
                    category: map_category(&sk.category),
                    proficiency: Proficiency::Intermediate,
                    evidence: "Terdeteksi dari konteks cerita utama.".into(),
                },
                2,
                sk.domain,
            ),
        );
    }

    // Determine dominant domains from Path A
    let mut domain_counts: HashMap<String, f32> = HashMap::new();
    for (_, _, dom) in skill_map.values() {
        *domain_counts.entry(dom.clone()).or_insert(0.0) += 1.0;
    }

    // Path B: Neural Bi-Encoder (e5-base + semantic_index.vidx) implicit skill discovery & role scoring
    let mut full_story_vec: Option<Vec<f32>> = None;
    let mut top_domain_scores: Vec<(String, f32)> = Vec::new();
    if let Some(emb) = embedder {
        if emb.is_neural_v2() {
            if let Some(index) = emb.semantic_index() {
                if let Some(qvec) = emb.embed_query(&affirmed_full_text) {
                    top_domain_scores = index.classify_domains(&qvec);
                    full_story_vec = Some(qvec);
                }
                let allowed_domains: HashSet<String> = top_domain_scores
                    .iter()
                    .take(4)
                    .map(|(d, _)| d.clone())
                    .collect();

                // Embed top 5 richest affirmative clauses for implicit skill discovery
                let mut rich_clauses: Vec<&&AnalyzedClause> = affirmative_clauses
                    .iter()
                    .filter(|c| c.text.chars().count() >= 18)
                    .collect();
                rich_clauses.sort_by_key(|c| std::cmp::Reverse(c.text.len()));
                rich_clauses.truncate(5);

                for cl in rich_clauses {
                    let Some(cvec) = emb.embed_query(&cl.text) else {
                        continue;
                    };
                    let (hits, median) = index.search_skills(&cvec, 10);
                    let prof = if cl.stance == ClauseStance::Learning {
                        Proficiency::Beginner
                    } else {
                        Proficiency::Intermediate
                    };
                    let mut added_from_clause = 0usize;
                    for hit in hits {
                        if added_from_clause >= 3 {
                            break;
                        }
                        let key = hit.canonical_name.to_lowercase();
                        if negated_skills_lower.contains(&key) || skill_map.contains_key(&key) {
                            continue;
                        }
                        let margin = hit.score - median;
                        let strong = hit.score >= 0.875 && margin >= 0.085;
                        let domain_coherent = hit.score >= 0.848
                            && margin >= 0.080
                            && (allowed_domains.is_empty()
                                || allowed_domains.contains(&hit.domain_slug));
                        if strong || domain_coherent {
                            let pct = (calibrate_e5_cosine(hit.score as f64) * 100.0).round() as u32;
                            skill_map.insert(
                                key,
                                (
                                    ExtractedSkillDraft {
                                        canonical_name: hit.canonical_name.clone(),
                                        category: map_category(&hit.category),
                                        proficiency: prof,
                                        evidence: format!(
                                            "Makna implisit ({pct}%): \"{}\"",
                                            truncate_snippet(&cl.text, 68)
                                        ),
                                    },
                                    if prof == Proficiency::Beginner { 1 } else { 2 },
                                    hit.domain_slug.clone(),
                                ),
                            );
                            *domain_counts.entry(hit.domain_slug).or_insert(0.0) += 0.6;
                            added_from_clause += 1;
                        }
                    }
                }
            } else if let Some(qvec) = emb.embed_query(&affirmed_full_text) {
                full_story_vec = Some(qvec);
            }
        }
    }

    let dominant_domain = domain_counts
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(d, _)| d.clone())
        .or_else(|| top_domain_scores.first().map(|(d, _)| d.clone()))
        .unwrap_or_else(|| full_ext.dominant_domain.clone());

    let mut extracted_pairs: Vec<(ExtractedSkillDraft, u8)> = skill_map
        .into_values()
        .map(|(draft, rank, _)| (draft, rank))
        .collect();
    extracted_pairs.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.canonical_name.cmp(&b.0.canonical_name))
    });
    let extracted_skills: Vec<ExtractedSkillDraft> =
        extracted_pairs.into_iter().take(24).map(|(d, _)| d).collect();

    // Score 238 roles from catalog
    let user_skill_lower: HashSet<String> = extracted_skills
        .iter()
        .map(|s| s.canonical_name.to_lowercase())
        .collect();
    let user_parent_lower = tax.parent_names_lower_for_owned(&user_skill_lower);
    let neural_role_scores: HashMap<String, f32> = embedder
        .and_then(|e| e.semantic_index())
        .zip(full_story_vec.as_ref())
        .map(|(idx, qv)| {
            idx.search_roles(qv, 238)
                .into_iter()
                .map(|r| (r.role_id, r.score))
                .collect()
        })
        .unwrap_or_default();

    let story_lower = affirmed_full_text.to_lowercase();
    struct ScoredRole<'a> {
        entry: &'a vecta_taxonomy::ProdiRoleCatalogEntry,
        score: f64,
        matched: Vec<String>,
    }
    let mut scored_roles: Vec<ScoredRole<'_>> = tax
        .roles_catalog()
        .iter()
        .map(|entry| {
            let mut matched = Vec::new();
            let total_req = (entry.mandatory_skills.len() * 2 + entry.preferred_skills.len()).max(1);
            let mut points = 0.0f64;
            for req in &entry.mandatory_skills {
                let rl = req.to_lowercase();
                if user_skill_lower.contains(&rl) {
                    points += 2.0;
                    matched.push(req.clone());
                } else if user_parent_lower.contains(&rl) {
                    points += 1.0;
                }
            }
            for pref in &entry.preferred_skills {
                let pl = pref.to_lowercase();
                if user_skill_lower.contains(&pl) {
                    points += 1.0;
                    matched.push(pref.clone());
                } else if user_parent_lower.contains(&pl) {
                    points += 0.5;
                }
            }
            let skill_cov = (points / (total_req as f64)).clamp(0.0, 1.0);
            let title_hit = if story_lower.contains(&entry.role_title.to_lowercase()) {
                0.25
            } else {
                0.0
            };
            let dom_bonus: f64 = if entry.domain == dominant_domain {
                0.10
            } else {
                0.0
            };
            let score = if let Some(&ns) = neural_role_scores.get(&entry.id) {
                let cal = calibrate_e5_cosine(ns as f64);
                (0.50 * cal) + (0.40 * skill_cov) + (0.10 * dom_bonus.min(1.0)) + title_hit * 0.4
            } else {
                (0.70 * skill_cov) + title_hit + dom_bonus
            };
            ScoredRole {
                entry,
                score,
                matched,
            }
        })
        .collect();

    scored_roles.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut recommended_keywords: Vec<RecommendedKeywordDraft> = Vec::new();
    let mut seen_titles: HashSet<String> = HashSet::new();

    for sr in &scored_roles {
        if recommended_keywords.len() >= 3 {
            break;
        }
        let tl = sr.entry.role_title.to_lowercase();
        if !seen_titles.insert(tl) {
            continue;
        }
        let reason = if sr.matched.is_empty() {
            format!(
                "Selaras secara semantik dengan bidang {} ({}).",
                sr.entry.domain.replace('_', " "),
                sr.entry.prodi_label
            )
        } else {
            format!(
                "Kecocokan utama pada {} ({}).",
                sr.matched.iter().take(4).cloned().collect::<Vec<_>>().join(", "),
                sr.entry.prodi_label
            )
        };
        let queries = if sr.entry.scrape_queries.is_empty() {
            vec![sr.entry.role_title.clone()]
        } else {
            sr.entry.scrape_queries.clone()
        };
        recommended_keywords.push(RecommendedKeywordDraft {
            role_label: sr.entry.role_title.clone(),
            fit_type: FitType::PrimaryFit,
            match_reason: reason,
            scrape_queries: queries,
        });
    }

    // Add Top-2 AdjacentFit (prefer cross-role / adjacent diversity)
    let mut adjacent_added = 0usize;
    for sr in &scored_roles {
        if adjacent_added >= 2 {
            break;
        }
        let tl = sr.entry.role_title.to_lowercase();
        if !seen_titles.insert(tl) {
            continue;
        }
        let queries = if sr.entry.scrape_queries.is_empty() {
            vec![sr.entry.role_title.clone()]
        } else {
            sr.entry.scrape_queries.clone()
        };
        recommended_keywords.push(RecommendedKeywordDraft {
            role_label: sr.entry.role_title.clone(),
            fit_type: FitType::AdjacentFit,
            match_reason: format!(
                "Opsi karier berdekatan dari rumpun {} dengan skill yang dapat ditransfer.",
                sr.entry.domain.replace('_', " ")
            ),
            scrape_queries: queries,
        });
        adjacent_added += 1;
    }

    let primary_role = recommended_keywords
        .first()
        .map(|k| k.role_label.clone())
        .unwrap_or_else(|| "Kandidat Profesional".to_string());
    let exp_level = infer_story_experience_level(cleaned);

    let top_skills_preview: Vec<String> = extracted_skills
        .iter()
        .take(5)
        .map(|s| s.canonical_name.clone())
        .collect();
    let professional_summary = if top_skills_preview.is_empty() {
        format!(
            "Kandidat jalur {primary_role} tingkat {} yang siap berkontribusi secara terukur.",
            exp_level.as_str()
        )
    } else {
        format!(
            "Kandidat {} ({}) dengan kekuatan utama di {}.",
            primary_role,
            exp_level.as_str(),
            top_skills_preview.join(", ")
        )
    };

    let draft = StoryProfileDraft {
        professional_summary,
        estimated_experience_level: exp_level,
        extracted_skills,
        recommended_keywords,
    };
    let excluded_list: Vec<String> = negated_skills_lower.into_iter().collect();
    Ok((draft, dominant_domain, excluded_list))
}

pub fn analyze_story_to_imported_draft(
    raw_story: &str,
    embedder: Option<&LocalEmbedder>,
) -> Result<ImportedCvDraft, AppError> {
    let (draft, dominant_domain, _) = analyze_story(raw_story, embedder)?;
    let primary_role = draft
        .recommended_keywords
        .first()
        .map(|r| r.role_label.clone())
        .unwrap_or_else(|| "Kandidat Profesional".into());
    let location = infer_city_from_text(raw_story);
    let cleaned: String = raw_story
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let master_cv = MasterCvProfile {
        user_id: String::new(),
        full_name: String::new(),
        headline: primary_role.clone(),
        email: String::new(),
        phone: String::new(),
        location,
        linkedin_url: String::new(),
        portfolio_url: String::new(),
        summary: draft.professional_summary.clone(),
        experiences: vec![CvExperienceItem {
            role: primary_role,
            company: String::new(),
            location: String::new(),
            period: String::new(),
            bullets: vec![],
        }],
        education: vec![],
        projects: vec![],
        certifications: vec![],
        languages: vec!["Indonesia".into()],
        updated_at: String::new(),
    };

    Ok(ImportedCvDraft {
        extracted_text: cleaned,
        dominant_domain,
        estimated_experience_level: draft.estimated_experience_level,
        professional_summary: draft.professional_summary,
        extracted_skills: draft.extracted_skills,
        recommended_keywords: draft.recommended_keywords,
        master_cv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_negated_skills_and_downgrades_learning_stance() {
        let story = "Saya berpengalaman 3 tahun mengerjakan laporan keuangan pakai Excel dan SQL setiap hari. Namun saya belum pernah memakai SAP FICO dan sedang belajar Python untuk analisis data.";
        let (draft, _dom, excluded) = analyze_story(story, None).expect("story parse succeeds");

        assert_eq!(draft.estimated_experience_level, ExperienceLevel::Mid);
        assert!(excluded.iter().any(|s| s.contains("sap")));
        assert!(
            !draft
                .extracted_skills
                .iter()
                .any(|s| s.canonical_name.eq_ignore_ascii_case("SAP FICO")),
            "negated SAP FICO must not appear in extracted skills"
        );
        let python = draft
            .extracted_skills
            .iter()
            .find(|s| s.canonical_name.eq_ignore_ascii_case("Python"))
            .expect("Python should still be extracted as Beginner");
        assert_eq!(python.proficiency, Proficiency::Beginner);

        let sql = draft
            .extracted_skills
            .iter()
            .find(|s| s.canonical_name.eq_ignore_ascii_case("SQL"));
        if let Some(s) = sql {
            assert_eq!(s.proficiency, Proficiency::Advanced);
        }
    }

    #[test]
    fn honors_four_year_degree_without_false_senior_escalation() {
        let story = "Saya lulusan baru S1 Teknik Informatika setelah kuliah 4 tahun dan baru saja menyelesaikan program magang MSIB sebagai frontend developer menggunakan TypeScript dan React.";
        let (draft, _dom, _) = analyze_story(story, None).expect("story parse succeeds");
        assert_ne!(draft.estimated_experience_level, ExperienceLevel::Senior);
    }

    #[test]
    fn recommends_diverse_primary_and_adjacent_roles_from_catalog() {
        let story = "Berpengalaman membangun antarmuka web modern menggunakan TypeScript, React, Tailwind CSS, serta mengintegrasikan REST API bersama tim produk.";
        let (draft, _dom, _) = analyze_story(story, None).expect("story parse succeeds");
        let primary_count = draft
            .recommended_keywords
            .iter()
            .filter(|r| r.fit_type == FitType::PrimaryFit)
            .count();
        let adjacent_count = draft
            .recommended_keywords
            .iter()
            .filter(|r| r.fit_type == FitType::AdjacentFit)
            .count();
        assert_eq!(primary_count, 3);
        assert_eq!(adjacent_count, 2);
    }

    #[test]
    fn real_neural_v2_golden_benchmark_profile_and_reranker() {
        let v2_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("models")
            .join("neural-v2");
        if !v2_dir.join("e5-base.vtxr").exists() || !v2_dir.join("semantic_index.vidx").exists() {
            return;
        }
        let embedder = LocalEmbedder::discover_and_load(&[v2_dir]);
        assert!(embedder.is_neural_v2());

        let story = "Saya berpengalaman 3 tahun sering mengurus laporan kas harian, rekonsiliasi mutasi bank, dan membantu hitung pajak bulanan perusahaan. Namun saya belum pernah memakai SAP FICO dan sedang belajar Python untuk analisis data.";
        let (draft, dom, excluded) =
            analyze_story(story, Some(&embedder)).expect("real neural story analysis succeeds");

        println!(
            "[BENCHMARK story_profiler] level={:?} dom={} skills={:?} top_role={:?} excluded={:?}",
            draft.estimated_experience_level,
            dom,
            draft
                .extracted_skills
                .iter()
                .map(|s| format!("{}:{:?}", s.canonical_name, s.proficiency))
                .collect::<Vec<_>>(),
            draft
                .recommended_keywords
                .iter()
                .map(|r| r.role_label.as_str())
                .collect::<Vec<_>>(),
            excluded
        );

        assert_eq!(draft.estimated_experience_level, ExperienceLevel::Mid);
        assert!(!draft
            .extracted_skills
            .iter()
            .any(|s| s.canonical_name.eq_ignore_ascii_case("SAP FICO")));
        assert!(!draft.extracted_skills.is_empty());

        let python = draft
            .extracted_skills
            .iter()
            .find(|s| s.canonical_name.eq_ignore_ascii_case("Python"))
            .expect("Python should be detected as Beginner");
        assert_eq!(python.proficiency, Proficiency::Beginner);

        let query = "Staf Akuntansi & Pajak berpengalaman laporan keuangan, rekonsiliasi bank, PPh PPN";
        let passages = vec![
            "Accounting & Tax Officer: menyusun laporan keuangan bulanan, rekonsiliasi bank, pelaporan PPh 21/23 dan PPN e-Faktur.".to_string(),
            "Barista Coffee Shop: membuat minuman espresso, latte art, membersihkan mesin kopi, melayani kasir cafe.".to_string(),
        ];
        let scores = embedder
            .rerank_pairs(query, &passages)
            .expect("cross-encoder reranker scores");
        println!(
            "[BENCHMARK reranker] accounting={:.4} barista={:.4}",
            scores[0], scores[1]
        );
        assert!(
            scores[0].is_finite() && scores[1].is_finite() && scores[0] > scores[1] + 0.10,
            "expected accounting reranker prob ({:.4}) >> barista ({:.4})",
            scores[0],
            scores[1]
        );
    }
}
