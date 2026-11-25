use std::collections::HashSet;

use vecta_taxonomy::{RequirementLevel, TaxonomyEngine};

use crate::models::job::{RequirementType, WorkMode};
use crate::models::profile::{ExperienceLevel, SkillCategory};

#[derive(Debug, Clone)]
pub struct LocalSkillEntry {
    pub skill_id: i64,
    pub canonical_name: String,
    pub category: SkillCategory,
    pub aliases: Vec<String>,
}

/// Return all curated baseline skills across all 14 Indonesian academic & professional domains
/// from the `vecta-taxonomy` engine so SQLite seeding covers IT, Finance/Tax, Marketing, HR,
/// Civil/Mechanical/HSE, Healthcare/Pharma, Legal, Design, Food/Agri, Education, Hospitality, & Lab Science.
pub fn taxonomy_seed_skills() -> Vec<(String, SkillCategory, Vec<String>)> {
    TaxonomyEngine::global()
        .all_skills()
        .iter()
        .map(|s| {
            let cat = match s.category.as_str() {
                "core_hard" => SkillCategory::CoreHard,
                "tools_tech" => SkillCategory::ToolsTech,
                "domain" => SkillCategory::Domain,
                "soft_skill" => SkillCategory::SoftSkill,
                _ => SkillCategory::CoreHard,
            };
            (s.canonical_name.clone(), cat, s.aliases.clone())
        })
        .collect()
}

pub struct UniversalLocalExtraction {
    pub dominant_domain: String,
    pub normalized_role: String,
    pub skill_ids: Vec<(i64, RequirementType)>,
}

/// Full 4-stage precision cross-domain extraction combining `vecta-taxonomy` (14 domains,
/// section-aware segmenter, domain disambiguation) with any extra custom user skills in SQLite.
pub fn extract_universal_job_metadata(
    title: &str,
    description: &str,
    query_hint: Option<&str>,
    catalog: &[LocalSkillEntry],
) -> UniversalLocalExtraction {
    let tax = TaxonomyEngine::global().extract_job(title, description, query_hint);

    let mut name_to_id = std::collections::HashMap::with_capacity(catalog.len());
    for entry in catalog {
        name_to_id.insert(entry.canonical_name.to_lowercase(), entry.skill_id);
    }

    let mut seen = HashSet::new();
    let mut skill_ids = Vec::new();

    for s in &tax.skills {
        if let Some(&sid) = name_to_id.get(&s.canonical_name.to_lowercase()) {
            if seen.insert(sid) {
                let req = match s.requirement {
                    RequirementLevel::Mandatory => RequirementType::Mandatory,
                    RequirementLevel::NiceToHave => RequirementType::NiceToHave,
                };
                skill_ids.push((sid, req));
            }
        }
    }

    // Also match any custom user-added skills in SQLite that aren't in the static 14-domain packs
    let title_lower = title.to_lowercase();
    let sections = vecta_taxonomy::segment_job_description(description);
    let body = if !sections.requirements.is_empty() || !sections.responsibilities.is_empty() {
        format!("{title_lower}\n{}\n{}", sections.requirements.to_lowercase(), sections.responsibilities.to_lowercase())
    } else {
        format!("{title_lower}\n{}", description.to_lowercase())
    };
    let tax_names: HashSet<String> = TaxonomyEngine::global()
        .all_skills()
        .iter()
        .map(|s| s.canonical_name.to_lowercase())
        .collect();

    for entry in catalog {
        if seen.contains(&entry.skill_id) || tax_names.contains(&entry.canonical_name.to_lowercase()) {
            continue;
        }
        if contains_term_boundary(&body, &entry.canonical_name)
            || entry.aliases.iter().any(|a| contains_term_boundary(&body, a))
        {
            seen.insert(entry.skill_id);
            let req = if contains_term_boundary(&title_lower, &entry.canonical_name)
                || entry.category == SkillCategory::CoreHard
            {
                RequirementType::Mandatory
            } else {
                RequirementType::NiceToHave
            };
            skill_ids.push((entry.skill_id, req));
        }
    }

    UniversalLocalExtraction {
        dominant_domain: tax.dominant_domain,
        normalized_role: tax.normalized_role,
        skill_ids,
    }
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Case-insensitive boundary-aware term matcher that handles punctuation tokens like `C#`, `C++`, `.NET`, `CI/CD`.
pub fn contains_term_boundary(haystack_lower: &str, needle: &str) -> bool {
    let needle_lower = needle.trim().to_lowercase();
    if needle_lower.is_empty() {
        return false;
    }
    let h_bytes = haystack_lower.as_bytes();
    let n_bytes = needle_lower.as_bytes();
    if n_bytes.len() > h_bytes.len() {
        return false;
    }

    let first_is_ident = is_ident_char(n_bytes[0]);
    let last_is_ident = is_ident_char(n_bytes[n_bytes.len() - 1]);

    let mut start = 0usize;
    while let Some(pos) = haystack_lower[start..].find(&needle_lower) {
        let abs = start + pos;
        let end = abs + n_bytes.len();

        let left_ok = if abs == 0 {
            true
        } else {
            let prev = h_bytes[abs - 1];
            if first_is_ident {
                !is_ident_char(prev)
            } else {
                prev.is_ascii_whitespace() || prev == b'(' || prev == b',' || prev == b'/'
            }
        };

        let right_ok = if end == h_bytes.len() {
            true
        } else {
            let next = h_bytes[end];
            if last_is_ident {
                // Prevent matching `Java` inside `JavaScript` or `C` inside `C#`/`C++`
                !is_ident_char(next) && next != b'+' && next != b'#'
            } else {
                !is_ident_char(next)
            }
        };

        if left_ok && right_ok {
            return true;
        }
        start = abs + 1;
        if start >= h_bytes.len() {
            break;
        }
    }
    false
}

/// Extract skills from a job listing using the local SQLite skill dictionary (0 Gemini tokens).
pub fn extract_local_skills(
    title: &str,
    description: &str,
    catalog: &[LocalSkillEntry],
) -> Vec<(i64, RequirementType)> {
    let title_lower = title.to_lowercase();
    let combined_lower = format!("{title_lower}\n{}", description.to_lowercase());
    let mut seen = HashSet::new();
    let mut found = Vec::new();

    for entry in catalog {
        if !seen.insert(entry.skill_id) {
            continue;
        }
        let mut matched_in_title = contains_term_boundary(&title_lower, &entry.canonical_name);
        let mut matched_any =
            matched_in_title || contains_term_boundary(&combined_lower, &entry.canonical_name);

        if !matched_any {
            for alias in &entry.aliases {
                if contains_term_boundary(&title_lower, alias) {
                    matched_in_title = true;
                    matched_any = true;
                    break;
                }
                if contains_term_boundary(&combined_lower, alias) {
                    matched_any = true;
                    break;
                }
            }
        }

        if matched_any {
            let req_type = if matched_in_title || entry.category == SkillCategory::CoreHard {
                RequirementType::Mandatory
            } else {
                RequirementType::NiceToHave
            };
            found.push((entry.skill_id, req_type));
        }
    }

    found
}

pub fn infer_work_mode(title: &str, location: Option<&str>, description: &str) -> WorkMode {
    let text = format!(
        "{} {} {}",
        title.to_lowercase(),
        location.unwrap_or("").to_lowercase(),
        description.to_lowercase()
    );
    if contains_term_boundary(&text, "hybrid") || contains_term_boundary(&text, "hibrida") {
        WorkMode::Hybrid
    } else if contains_term_boundary(&text, "remote")
        || contains_term_boundary(&text, "wfh")
        || text.contains("work from home")
        || text.contains("work from anywhere")
        || contains_term_boundary(&text, "wfa")
    {
        WorkMode::Remote
    } else if contains_term_boundary(&text, "onsite")
        || text.contains("on-site")
        || text.contains("on site")
        || contains_term_boundary(&text, "wfo")
        || text.contains("work from office")
    {
        WorkMode::Onsite
    } else {
        WorkMode::Unknown
    }
}

pub fn infer_experience_level(title: &str, description: &str) -> ExperienceLevel {
    let title_l = title.to_lowercase();
    let combined = format!("{title_l}\n{}", description.to_lowercase());

    if contains_term_boundary(&title_l, "intern")
        || contains_term_boundary(&title_l, "internship")
        || contains_term_boundary(&title_l, "magang")
    {
        return ExperienceLevel::Intern;
    }
    if contains_term_boundary(&title_l, "senior")
        || contains_term_boundary(&title_l, "sr")
        || contains_term_boundary(&title_l, "lead")
        || contains_term_boundary(&title_l, "principal")
        || contains_term_boundary(&title_l, "staff")
        || contains_term_boundary(&title_l, "manager")
    {
        return ExperienceLevel::Senior;
    }
    if contains_term_boundary(&title_l, "junior")
        || contains_term_boundary(&title_l, "jr")
        || contains_term_boundary(&title_l, "entry")
        || title_l.contains("fresh graduate")
    {
        return ExperienceLevel::Junior;
    }
    if contains_term_boundary(&title_l, "mid")
        || title_l.contains("middle")
        || contains_term_boundary(&title_l, "intermediate")
    {
        return ExperienceLevel::Mid;
    }

    if combined.contains("fresh graduate")
        || combined.contains("0-1 year")
        || combined.contains("0 - 1 tahun")
        || combined.contains("minimal 1 tahun")
    {
        ExperienceLevel::Junior
    } else if combined.contains("5+ year")
        || combined.contains("5 years")
        || combined.contains("minimal 5 tahun")
    {
        ExperienceLevel::Senior
    } else if combined.contains("2-4 year")
        || combined.contains("3+ year")
        || combined.contains("minimal 2 tahun")
        || combined.contains("minimal 3 tahun")
    {
        ExperienceLevel::Mid
    } else {
        ExperienceLevel::Unknown
    }
}

pub fn infer_normalized_role(title: &str, query_hint: Option<&str>) -> String {
    vecta_taxonomy::normalize_role_universal(title, "umum_lintas_bidang", query_hint)
}

fn tokenize_keywords(text: &str) -> HashSet<String> {
    const STOP_WORDS: &[&str] = &[
        "dan", "yang", "untuk", "dengan", "dari", "pada", "dalam", "atau", "sebagai", "saya",
        "the", "and", "for", "with", "from", "that", "this", "are", "you", "will", "our", "job",
    ];
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '#' && c != '+')
        .filter(|w| w.len() >= 2 && !stop.contains(w))
        .map(str::to_string)
        .collect()
}

/// Compute a 0.0..=1.0 local text/keyword relevance score between a user profile and a job
/// when vector embeddings are not yet generated (0 Gemini tokens).
pub fn compute_local_keyword_relevance(
    profile_summary: &str,
    target_roles: &[String],
    user_skill_names: &[String],
    job_title: &str,
    job_description: &str,
    preferred_location: Option<&str>,
    job_location: Option<&str>,
) -> f64 {
    let mut profile_tokens = tokenize_keywords(profile_summary);
    for r in target_roles {
        profile_tokens.extend(tokenize_keywords(r));
    }
    for s in user_skill_names {
        profile_tokens.extend(tokenize_keywords(s));
    }
    if profile_tokens.is_empty() {
        return 0.5;
    }

    let title_tokens = tokenize_keywords(job_title);
    let desc_tokens = tokenize_keywords(job_description);

    let title_hits = title_tokens
        .iter()
        .filter(|t| profile_tokens.contains(*t))
        .count();
    let title_score = if title_tokens.is_empty() {
        0.0
    } else {
        (title_hits as f64 / (title_tokens.len().min(6) as f64)).clamp(0.0, 1.0)
    };

    let desc_hits = profile_tokens
        .iter()
        .filter(|t| desc_tokens.contains(*t))
        .count();
    let desc_score = (desc_hits as f64 / (profile_tokens.len().min(12) as f64)).clamp(0.0, 1.0);

    let loc_bonus = match (preferred_location, job_location) {
        (Some(pref), Some(jloc)) => {
            let p = pref.trim().to_lowercase();
            let j = jloc.trim().to_lowercase();
            if p.is_empty() || p == "indonesia" {
                0.0
            } else if j.contains(&p) || j.contains("remote") {
                0.10
            } else {
                0.0
            }
        }
        _ => 0.0,
    };

    ((0.55 * title_score) + (0.45 * desc_score) + loc_bonus).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_matcher_distinguishes_java_vs_javascript_and_go_vs_good() {
        assert!(contains_term_boundary("we use javascript and vue.js", "JavaScript"));
        assert!(!contains_term_boundary("we use javascript and vue.js", "Java"));
        assert!(!contains_term_boundary("this is a good opportunity", "Go"));
        assert!(contains_term_boundary("experience with go and docker", "Go"));
        assert!(contains_term_boundary("backend c# and .net developer", "C#"));
        assert!(contains_term_boundary("backend c# and .net developer", ".NET"));
    }

    #[test]
    fn extract_local_skills_and_metadata_without_gemini() {
        let catalog = vec![
            LocalSkillEntry {
                skill_id: 1,
                canonical_name: "PHP".into(),
                category: SkillCategory::CoreHard,
                aliases: vec!["PHP 8".into()],
            },
            LocalSkillEntry {
                skill_id: 2,
                canonical_name: "Laravel".into(),
                category: SkillCategory::CoreHard,
                aliases: vec![],
            },
            LocalSkillEntry {
                skill_id: 3,
                canonical_name: "Docker".into(),
                category: SkillCategory::ToolsTech,
                aliases: vec!["Docker Compose".into()],
            },
        ];

        let skills = extract_local_skills(
            "Junior Backend Developer PHP",
            "Build APIs with Laravel and Docker Compose (Remote WFH)",
            &catalog,
        );
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0], (1, RequirementType::Mandatory));
        assert_eq!(skills[1], (2, RequirementType::Mandatory));
        assert_eq!(skills[2], (3, RequirementType::NiceToHave));

        assert_eq!(
            infer_work_mode("Backend", Some("Bandung"), "Kerja remote / WFH penuh"),
            WorkMode::Remote
        );
        assert_eq!(
            infer_experience_level("Senior Backend Engineer", "5+ years"),
            ExperienceLevel::Senior
        );
        assert_eq!(
            infer_normalized_role("Web Programmer PHP Laravel", None),
            "Backend Developer"
        );
    }

    #[test]
    fn local_keyword_relevance_scores_relevant_role_higher_than_unrelated() {
        let high = compute_local_keyword_relevance(
            "Backend Developer PHP Laravel MySQL",
            &["Backend Developer".into()],
            &["PHP".into(), "Laravel".into(), "MySQL".into()],
            "Backend Developer PHP Laravel",
            "Membangun REST API dengan PHP, Laravel, dan database MySQL",
            Some("Bandung"),
            Some("Bandung, Jawa Barat"),
        );
        let low = compute_local_keyword_relevance(
            "Backend Developer PHP Laravel MySQL",
            &["Backend Developer".into()],
            &["PHP".into(), "Laravel".into(), "MySQL".into()],
            "Account Executive Sales",
            "Menawarkan produk properti dan mencapai target penjualan bulanan",
            Some("Bandung"),
            Some("Surabaya"),
        );
        assert!(high > 0.65, "expected high local relevance, got {high}");
        assert!(low < 0.25, "expected low local relevance, got {low}");
    }
}
