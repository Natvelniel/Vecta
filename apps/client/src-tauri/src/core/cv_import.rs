use vecta_taxonomy::TaxonomyEngine;

use crate::core::local_matcher::infer_experience_level;
use crate::error::AppError;
use crate::models::cv::{CvEducationItem, CvExperienceItem, ImportedCvDraft, MasterCvProfile};
use crate::models::profile::{
    ExperienceLevel, ExtractedSkillDraft, FitType, Proficiency, RecommendedKeywordDraft,
    SkillCategory,
};

const MAX_PDF_BYTES: usize = 10 * 1024 * 1024;
const MAX_EXTRACTED_CHARS: usize = 25_000;

fn extract_email(text: &str) -> String {
    for token in text.split(|c: char| c.is_whitespace() || matches!(c, '|' | ',' | ';' | '(' | ')' | '<' | '>')) {
        let t = token.trim_matches(|c: char| c == '.' || c == ':');
        if t.contains('@') && t.contains('.') && t.len() >= 6 && t.len() <= 100 && !t.starts_with('@') && !t.ends_with('.') {
            return t.to_string();
        }
    }
    String::new()
}

fn extract_phone(text: &str) -> String {
    for line in text.lines().take(25) {
        for segment in line.split(|c: char| matches!(c, '|' | '•' | ',' | ';')) {
            let trimmed = segment.trim();
            let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 9 && digits.len() <= 15 && (digits.starts_with("08") || digits.starts_with("62") || digits.starts_with("02") || trimmed.starts_with('+')) {
                return trimmed.to_string();
            }
        }
    }
    String::new()
}

fn extract_url_containing(text: &str, needle: &str) -> String {
    for token in text.split(|c: char| c.is_whitespace() || matches!(c, '|' | ',' | ';' | '(' | ')')) {
        let lower = token.to_lowercase();
        if lower.contains(needle) {
            let clean = token.trim_matches(|c: char| matches!(c, '.' | ',' | ';'));
            if clean.starts_with("http://") || clean.starts_with("https://") {
                return clean.to_string();
            }
            return format!("https://{clean}");
        }
    }
    String::new()
}

fn infer_candidate_name(text: &str) -> String {
    for line in text.lines().take(8) {
        let trimmed = line.trim();
        if trimmed.len() < 3 || trimmed.len() > 60 {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if lower.contains("curriculum vitae")
            || lower == "resume"
            || lower == "cv"
            || trimmed.contains('@')
            || trimmed.contains("http")
            || trimmed.chars().filter(|c| c.is_ascii_digit()).count() > 2
        {
            continue;
        }
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if (1..=5).contains(&words.len()) {
            return trimmed.to_string();
        }
    }
    String::new()
}

pub(crate) fn infer_city_from_text(text: &str) -> String {
    const CITIES: &[&str] = &[
        "Jakarta", "Bandung", "Surabaya", "Yogyakarta", "Semarang", "Medan",
        "Makassar", "Denpasar", "Bali", "Tangerang", "Bekasi", "Depok", "Bogor",
        "Malang", "Palembang", "Balikpapan", "Batam", "Solo", "Surakarta",
    ];
    let head: String = text.lines().take(20).collect::<Vec<_>>().join(" ");
    let lower = head.to_lowercase();
    for &city in CITIES {
        if lower.contains(&city.to_lowercase()) {
            return city.to_string();
        }
    }
    "Indonesia".into()
}

fn infer_education_lines(text: &str) -> Vec<CvEducationItem> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if (lower.contains("universitas")
            || lower.contains("institut ")
            || lower.contains("politeknik")
            || lower.contains("university")
            || lower.contains("smk ")
            || lower.contains("s1 ")
            || lower.contains("d3 ")
            || lower.contains("sarjana"))
            && trimmed.len() >= 8
            && trimmed.len() <= 140
        {
            out.push(CvEducationItem {
                institution: trimmed.to_string(),
                degree: String::new(),
                period: String::new(),
                notes: String::new(),
            });
            if out.len() >= 2 {
                break;
            }
        }
    }
    out
}

fn default_roles_for_domain(domain: &str) -> (&'static str, Vec<&'static str>) {
    match domain {
        "informatika_data" => (
            "Software & Data Engineer",
            vec!["Software Engineer", "Backend Developer", "Data Analyst"],
        ),
        "ekonomi_akuntansi_pajak" => (
            "Akuntansi & Perpajakan",
            vec!["Accounting Staff", "Tax Staff", "Finance Officer"],
        ),
        "bisnis_marketing_sales" => (
            "Digital Marketing & Sales",
            vec!["Digital Marketing", "Social Media Specialist", "Account Executive"],
        ),
        "hr_psikologi_admin_ops" => (
            "HR, Administrasi & Operasional",
            vec!["HRGA Staff", "Staff Administrasi", "Supply Chain Staff"],
        ),
        "teknik_sipil_arsitektur_gis" => (
            "Teknik Sipil & Konstruksi",
            vec!["Civil Engineer", "Quantity Surveyor", "Site Engineer"],
        ),
        "teknik_mesin_elektro_industri" => (
            "Teknik Mesin, Elektro & HSE",
            vec!["Maintenance Engineer", "HSE Officer", "Quality Control"],
        ),
        "kesehatan_farmasi_keperawatan" => (
            "Kesehatan & Farmasi",
            vec!["Apoteker", "Regulatory Affairs", "Perawat"],
        ),
        "hukum_regulasi_kepatuhan" => (
            "Hukum & Kepatuhan",
            vec!["Legal Officer", "Corporate Legal", "Compliance Staff"],
        ),
        "komunikasi_desain_media" => (
            "Desain Grafis & Komunikasi",
            vec!["Graphic Designer", "Video Editor", "UI/UX Designer"],
        ),
        "pertanian_pangan_lingkungan" => (
            "Pangan, Pertanian & Lingkungan",
            vec!["QC Pangan", "Food Safety", "Environmental Officer"],
        ),
        "pendidikan_pelatihan" => (
            "Pendidikan & Pelatihan",
            vec!["Guru", "Instructional Designer", "Corporate Trainer"],
        ),
        "pariwisata_perhotelan_fnb" => (
            "Perhotelan, F&B & Event",
            vec!["Front Office", "Hotel Operations", "Event Management"],
        ),
        "sains_lab_geologi_energi" => (
            "Sains, Laboratorium & Energi",
            vec!["Analis Laboratorium", "QC Analis Kimia", "Geologist"],
        ),
        _ => (
            "Profesional Umum",
            vec!["Staff Administrasi", "Management Trainee", "Operations Staff"],
        ),
    }
}

fn map_category(cat: &str) -> SkillCategory {
    match cat {
        "core_hard" => SkillCategory::CoreHard,
        "tools_tech" => SkillCategory::ToolsTech,
        "domain" => SkillCategory::Domain,
        "soft_skill" => SkillCategory::SoftSkill,
        _ => SkillCategory::CoreHard,
    }
}

/// Extract text from PDF bytes locally (0 Gemini tokens) and build a structured cross-domain draft.
pub fn import_cv_from_pdf_bytes(pdf_bytes: &[u8]) -> Result<ImportedCvDraft, AppError> {
    if pdf_bytes.is_empty() {
        return Err(AppError::validation_error("File PDF kosong."));
    }
    if pdf_bytes.len() > MAX_PDF_BYTES {
        return Err(AppError::validation_error(
            "Ukuran file CV PDF maksimal 10 MB.",
        ));
    }
    if !pdf_bytes.starts_with(b"%PDF-") {
        return Err(AppError::validation_error(
            "Format file tidak valid. Pastikan file yang diunggah adalah dokumen PDF asli.",
        ));
    }

    let raw_text = pdf_extract::extract_text_from_mem(pdf_bytes).map_err(|e| {
        eprintln!("[vecta::cv_import] pdf_extract error: {e}");
        AppError::validation_error(
            "Teks dari PDF tidak dapat dibaca (kemungkinan hasil scan gambar). Ekspor CV sebagai PDF teks atau tempel ceritamu secara manual.",
        )
    })?;

    build_draft_from_cv_text(&raw_text)
}

/// Deterministic local CV text parser (0 Gemini tokens) across all 14 Indonesian domains.
pub fn build_draft_from_cv_text(raw_text: &str) -> Result<ImportedCvDraft, AppError> {
    let cleaned: String = raw_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(MAX_EXTRACTED_CHARS)
        .collect();

    if cleaned.chars().count() < 30 {
        return Err(AppError::validation_error(
            "Teks yang terbaca dari CV terlalu sedikit (< 30 karakter). Pastikan PDF bukan hasil scan gambar.",
        ));
    }

    let taxonomy = TaxonomyEngine::global();
    let extraction = taxonomy.extract_from_profile_text(&cleaned);

    let full_name = infer_candidate_name(&cleaned);
    let email = extract_email(&cleaned);
    let phone = extract_phone(&cleaned);
    let location = infer_city_from_text(&cleaned);
    let linkedin_url = extract_url_containing(&cleaned, "linkedin.com");
    let portfolio_url = extract_url_containing(&cleaned, "github.com");
    let education = infer_education_lines(&cleaned);

    let estimated_level = match infer_experience_level("", &cleaned) {
        ExperienceLevel::Unknown => ExperienceLevel::Junior,
        other => other,
    };

    let extracted_skills: Vec<ExtractedSkillDraft> = extraction
        .skills
        .iter()
        .take(25)
        .map(|s| ExtractedSkillDraft {
            canonical_name: s.canonical_name.clone(),
            category: map_category(&s.category),
            proficiency: Proficiency::Intermediate,
            evidence: "Terdeteksi otomatis dari dokumen CV lokal (0 token).".into(),
        })
        .collect();

    let (primary_role, default_queries) = default_roles_for_domain(&extraction.dominant_domain);
    let top_skill_names: Vec<String> = extracted_skills
        .iter()
        .take(4)
        .map(|s| s.canonical_name.clone())
        .collect();

    let summary = if top_skill_names.is_empty() {
        format!("Kandidat bidang {primary_role} yang siap berkontribusi secara profesional.")
    } else {
        format!(
            "Kandidat bidang {primary_role} dengan kompetensi utama pada {}.",
            top_skill_names.join(", ")
        )
    };

    let primary_queries: Vec<String> = default_queries.iter().map(|s| (*s).to_string()).collect();
    let recommended_keywords = vec![RecommendedKeywordDraft {
        role_label: primary_role.to_string(),
        fit_type: FitType::PrimaryFit,
        match_reason: format!(
            "Cocok dengan rumpun kompetensi {} yang terdeteksi pada CV.",
            extraction.dominant_domain.replace('_', " ")
        ),
        scrape_queries: primary_queries,
    }];

    let master_cv = MasterCvProfile {
        user_id: String::new(),
        full_name: full_name.clone(),
        headline: primary_role.to_string(),
        email,
        phone,
        location,
        linkedin_url,
        portfolio_url,
        summary: summary.clone(),
        experiences: vec![CvExperienceItem {
            role: primary_role.to_string(),
            company: String::new(),
            location: String::new(),
            period: String::new(),
            bullets: vec![],
        }],
        education,
        projects: vec![],
        certifications: vec![],
        languages: vec!["Bahasa Indonesia".into()],
        updated_at: String::new(),
    };

    Ok(ImportedCvDraft {
        extracted_text: cleaned,
        dominant_domain: extraction.dominant_domain,
        estimated_experience_level: estimated_level,
        professional_summary: summary,
        extracted_skills,
        recommended_keywords,
        master_cv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_non_it_indonesian_accounting_cv_locally_with_zero_tokens() {
        let cv = "Siti Rahmawati\nBandung, Jawa Barat | siti.rahma@example.com | +62 812-3456-7890 | linkedin.com/in/sitirahma\n\nPendidikan\nUniversitas Padjadjaran - S1 Akuntansi (2019 - 2023)\n\nKeahlian\nAkuntansi Keuangan, Laporan Keuangan PSAK, Brevet A/B, e-Faktur, e-Bupot, Accurate Online, Microsoft Excel (Pivot Table, VLOOKUP).";
        let draft = build_draft_from_cv_text(cv).expect("parse draft");
        assert_eq!(draft.dominant_domain, "ekonomi_akuntansi_pajak");
        assert_eq!(draft.master_cv.full_name, "Siti Rahmawati");
        assert_eq!(draft.master_cv.email, "siti.rahma@example.com");
        assert_eq!(draft.master_cv.location, "Bandung");
        assert!(draft
            .extracted_skills
            .iter()
            .any(|s| s.canonical_name == "Brevet Pajak A & B"));
        assert!(draft
            .extracted_skills
            .iter()
            .any(|s| s.canonical_name == "Accurate Accounting"));
        assert!(!draft.recommended_keywords.is_empty());
    }
}
