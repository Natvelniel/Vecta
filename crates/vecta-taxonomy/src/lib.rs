pub mod models;
pub mod role_normalizer;
pub mod segmenter;

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

pub use models::{
    DocumentSections, ExtractedSkill, ExtractionResult, RequirementLevel, TaxonomySkill,
};
pub use role_normalizer::normalize_role_universal;
pub use segmenter::segment_job_description;

const DOMAIN_PACKS: &[&str] = &[
    include_str!("../../../shared/taxonomy/01_informatika_data.json"),
    include_str!("../../../shared/taxonomy/02_ekonomi_akuntansi_pajak.json"),
    include_str!("../../../shared/taxonomy/03_bisnis_marketing_sales.json"),
    include_str!("../../../shared/taxonomy/04_hr_psikologi_admin_ops.json"),
    include_str!("../../../shared/taxonomy/05_teknik_sipil_arsitektur_gis.json"),
    include_str!("../../../shared/taxonomy/06_teknik_mesin_elektro_industri.json"),
    include_str!("../../../shared/taxonomy/07_kesehatan_farmasi_keperawatan.json"),
    include_str!("../../../shared/taxonomy/08_hukum_regulasi_kepatuhan.json"),
    include_str!("../../../shared/taxonomy/09_komunikasi_desain_media.json"),
    include_str!("../../../shared/taxonomy/10_pertanian_pangan_lingkungan.json"),
    include_str!("../../../shared/taxonomy/11_pendidikan_pelatihan.json"),
    include_str!("../../../shared/taxonomy/12_pariwisata_perhotelan_fnb.json"),
    include_str!("../../../shared/taxonomy/13_sains_lab_geologi_energi.json"),
    include_str!("../../../shared/taxonomy/14_soft_skills_bahasa_umum.json"),
];

#[derive(Debug, Clone)]
struct PatternRef {
    skill_idx: usize,
    pattern_len: usize,
    first_is_ident: bool,
    last_is_ident: bool,
}

pub struct TaxonomyEngine {
    skills: Vec<TaxonomySkill>,
    id_to_idx: HashMap<String, usize>,
    canonical_lower_to_idx: HashMap<String, usize>,
    automaton: AhoCorasick,
    patterns: Vec<PatternRef>,
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_valid_boundary(
    haystack_bytes: &[u8],
    start: usize,
    end: usize,
    first_is_ident: bool,
    last_is_ident: bool,
) -> bool {
    let left_ok = if start == 0 {
        true
    } else {
        let prev = haystack_bytes[start - 1];
        if first_is_ident {
            !is_ident_byte(prev)
        } else {
            prev.is_ascii_whitespace()
                || prev == b'('
                || prev == b','
                || prev == b'/'
                || prev == b'-'
        }
    };

    let right_ok = if end >= haystack_bytes.len() {
        true
    } else {
        let next = haystack_bytes[end];
        if last_is_ident {
            !is_ident_byte(next) && next != b'+' && next != b'#'
        } else {
            !is_ident_byte(next)
        }
    };

    left_ok && right_ok
}

static GLOBAL_ENGINE: OnceLock<TaxonomyEngine> = OnceLock::new();

impl TaxonomyEngine {
    pub fn global() -> &'static Self {
        GLOBAL_ENGINE.get_or_init(Self::build_default)
    }

    fn build_default() -> Self {
        let mut skills = Vec::new();
        for pack in DOMAIN_PACKS {
            let parsed: Vec<TaxonomySkill> =
                serde_json::from_str(pack).expect("embedded taxonomy pack must be valid JSON");
            skills.extend(parsed);
        }

        let mut id_to_idx = HashMap::with_capacity(skills.len());
        let mut canonical_lower_to_idx = HashMap::with_capacity(skills.len());
        for (idx, s) in skills.iter().enumerate() {
            id_to_idx.insert(s.id.clone(), idx);
            canonical_lower_to_idx.insert(s.canonical_name.to_lowercase(), idx);
        }

        let mut raw_patterns: Vec<String> = Vec::new();
        let mut patterns: Vec<PatternRef> = Vec::new();
        let mut seen_pat: HashSet<(usize, String)> = HashSet::new();

        for (idx, skill) in skills.iter().enumerate() {
            let mut terms = Vec::with_capacity(1 + skill.aliases.len());
            terms.push(skill.canonical_name.as_str());
            for alias in &skill.aliases {
                terms.push(alias.as_str());
            }
            for term in terms {
                let clean = term.trim();
                if clean.is_empty() {
                    continue;
                }
                let lower = clean.to_lowercase();
                if !seen_pat.insert((idx, lower.clone())) {
                    continue;
                }
                let (pattern_len, first_is_ident, last_is_ident) = {
                    let bytes = lower.as_bytes();
                    (
                        bytes.len(),
                        is_ident_byte(bytes[0]),
                        is_ident_byte(bytes[bytes.len() - 1]),
                    )
                };
                raw_patterns.push(lower);
                patterns.push(PatternRef {
                    skill_idx: idx,
                    pattern_len,
                    first_is_ident,
                    last_is_ident,
                });
            }
        }

        let automaton = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::Standard)
            .build(&raw_patterns)
            .expect("failed to build taxonomy Aho-Corasick automaton");

        Self {
            skills,
            id_to_idx,
            canonical_lower_to_idx,
            automaton,
            patterns,
        }
    }

    pub fn all_skills(&self) -> &[TaxonomySkill] {
        &self.skills
    }

    pub fn parent_canonical_name(&self, skill_canonical_name: &str) -> Option<&str> {
        let idx = *self
            .canonical_lower_to_idx
            .get(&skill_canonical_name.trim().to_lowercase())?;
        let parent_id = self.skills[idx].parent_id.as_deref()?;
        let parent_idx = *self.id_to_idx.get(parent_id)?;
        Some(self.skills[parent_idx].canonical_name.as_str())
    }

    /// Precompute the set of parent canonical names (lowercase) implied by the user's owned skills
    /// so batch matching over 1,000+ jobs does zero allocations per job skill.
    pub fn parent_names_lower_for_owned(&self, owned_lower: &HashSet<String>) -> HashSet<String> {
        let mut out = HashSet::new();
        for name in owned_lower {
            if let Some(&idx) = self.canonical_lower_to_idx.get(name) {
                if let Some(pid) = self.skills[idx].parent_id.as_deref() {
                    if let Some(&pidx) = self.id_to_idx.get(pid) {
                        out.insert(self.skills[pidx].canonical_name.to_lowercase());
                    }
                }
            }
        }
        out
    }

    fn scan_text(&self, text: &str) -> HashSet<usize> {
        let mut hits = HashSet::new();
        if text.trim().is_empty() {
            return hits;
        }
        let bytes = text.as_bytes();
        for mat in self.automaton.find_overlapping_iter(text) {
            let pref = &self.patterns[mat.pattern().as_usize()];
            if mat.end() - mat.start() == pref.pattern_len
                && is_valid_boundary(
                    bytes,
                    mat.start(),
                    mat.end(),
                    pref.first_is_ident,
                    pref.last_is_ident,
                )
            {
                hits.insert(pref.skill_idx);
            }
        }
        hits
    }

    /// Extract structured skills, dominant domain, and normalized role from a job posting
    /// with 4-stage anti-false-positive protection (0 Gemini tokens).
    pub fn extract_job(
        &self,
        title: &str,
        raw_description: &str,
        query_hint: Option<&str>,
    ) -> ExtractionResult {
        let sections = segment_job_description(raw_description);

        let title_hits = self.scan_text(title);
        let req_hits = self.scan_text(&sections.requirements);
        let resp_hits = self.scan_text(&sections.responsibilities);
        let nice_hits = self.scan_text(&sections.nice_to_have);
        let gen_hits = self.scan_text(&sections.general);

        // Ignore About Company buzzwords whenever substantive job sections exist.
        let has_substantive = !sections.requirements.trim().is_empty()
            || !sections.responsibilities.trim().is_empty()
            || !sections.general.trim().is_empty();
        let about_hits = if has_substantive {
            HashSet::new()
        } else {
            self.scan_text(&sections.about_company)
        };

        let mut candidate_indices: HashSet<usize> = HashSet::new();
        candidate_indices.extend(&title_hits);
        candidate_indices.extend(&req_hits);
        candidate_indices.extend(&resp_hits);
        candidate_indices.extend(&nice_hits);
        candidate_indices.extend(&gen_hits);
        candidate_indices.extend(&about_hits);

        // Stage 2: Compute domain evidence strictly from unambiguous matches
        let mut domain_scores: HashMap<&str, usize> = HashMap::new();
        for &idx in &candidate_indices {
            let s = &self.skills[idx];
            if s.requires_domain_context || s.domain == "umum_lintas_bidang" {
                continue;
            }
            let weight = if title_hits.contains(&idx) { 3 } else { 1 };
            *domain_scores.entry(s.domain.as_str()).or_insert(0) += weight;
        }

        let dominant_domain = domain_scores
            .iter()
            .max_by_key(|(_, score)| **score)
            .map(|(dom, _)| (*dom).to_string())
            .unwrap_or_else(|| "umum_lintas_bidang".into());

        // Stage 3: Gate ambiguous skills requiring domain context
        let mut extracted = Vec::new();
        let mut sorted_indices: Vec<usize> = candidate_indices.into_iter().collect();
        sorted_indices.sort_unstable();

        for idx in sorted_indices {
            let s = &self.skills[idx];
            let in_title = title_hits.contains(&idx);
            if s.requires_domain_context && !in_title {
                let dom_ev = domain_scores.get(s.domain.as_str()).copied().unwrap_or(0);
                if dom_ev == 0 {
                    continue;
                }
            }

            let in_req = req_hits.contains(&idx);
            let in_resp = resp_hits.contains(&idx);
            let in_nice_only =
                nice_hits.contains(&idx) && !in_title && !in_req && !in_resp && !gen_hits.contains(&idx);

            let requirement = if in_nice_only {
                RequirementLevel::NiceToHave
            } else if in_title || in_req || s.category == "core_hard" {
                RequirementLevel::Mandatory
            } else {
                RequirementLevel::NiceToHave
            };

            let parent_canonical_name = s
                .parent_id
                .as_deref()
                .and_then(|pid| self.id_to_idx.get(pid))
                .map(|&pidx| self.skills[pidx].canonical_name.clone());

            extracted.push(ExtractedSkill {
                skill_id: s.id.clone(),
                canonical_name: s.canonical_name.clone(),
                domain: s.domain.clone(),
                category: s.category.clone(),
                parent_canonical_name,
                requirement,
                matched_in_title: in_title,
            });
        }

        let normalized_role = normalize_role_universal(title, &dominant_domain, query_hint);

        ExtractionResult {
            dominant_domain,
            normalized_role,
            skills: extracted,
        }
    }

    /// Extract skills from a user CV or story text (0 Gemini tokens).
    pub fn extract_from_profile_text(&self, text: &str) -> ExtractionResult {
        self.extract_job("", text, None)
    }
}
