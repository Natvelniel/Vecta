use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::Serialize;
use typst::foundations::{Dict, IntoValue};
use typst_as_lib::TypstEngine;
use typst_pdf::PdfOptions;

use crate::error::AppError;
use crate::models::cv::{
    AtsExportResult, CvEducationItem, CvExperienceItem, CvProjectItem, MasterCvProfile,
};

const ATS_CLASSIC_TEMPLATE: &str =
    include_str!("../../../../../shared/templates/ats_classic.typ");
const EMBEDDED_FONT_REGULAR: &[u8] =
    include_bytes!("../../resources/fonts/LiberationSans-Regular.ttf");
const EMBEDDED_FONT_BOLD: &[u8] =
    include_bytes!("../../resources/fonts/LiberationSans-Bold.ttf");

#[derive(Serialize)]
struct AtsTemplateLabels {
    summary: String,
    skills: String,
    experience: String,
    projects: String,
    education: String,
    additional: String,
    certifications: String,
    languages: String,
}

#[derive(Serialize)]
struct AtsTemplatePayload {
    full_name: String,
    headline: String,
    email: String,
    phone: String,
    location: String,
    linkedin_url: String,
    portfolio_url: String,
    summary: String,
    skills: Vec<String>,
    experiences: Vec<CvExperienceItem>,
    projects: Vec<CvProjectItem>,
    education: Vec<CvEducationItem>,
    certifications: Vec<String>,
    languages: Vec<String>,
    labels: AtsTemplateLabels,
}

fn load_bundled_fonts(font_dirs: &[PathBuf]) -> Vec<Vec<u8>> {
    let mut loaded = Vec::new();
    for dir in font_dirs {
        for name in [
            "LiberationSans-Regular.ttf",
            "LiberationSans-Bold.ttf",
            "Inter-Regular.ttf",
        ] {
            let p = dir.join(name);
            if let Ok(bytes) = std::fs::read(&p) {
                if bytes.len() > 10_000 {
                    loaded.push(bytes);
                }
            }
        }
        if !loaded.is_empty() {
            break;
        }
    }

    // Embedded open-source ATS fonts fallback (guarantees 100% offline PDF export on Android/iOS/desktop)
    if loaded.is_empty() {
        loaded.push(EMBEDDED_FONT_REGULAR.to_vec());
        loaded.push(EMBEDDED_FONT_BOLD.to_vec());
    }
    loaded
}

fn sanitize_filename_slug(raw: &str) -> String {
    let slug: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '_'
            }
        })
        .collect();
    let collapsed: Vec<&str> = slug.split('_').filter(|s| !s.is_empty()).collect();
    collapsed.join("_").chars().take(42).collect()
}

pub struct AtsExportContext {
    pub master_cv: MasterCvProfile,
    pub user_skills: Vec<String>,
    pub job_title: Option<String>,
    pub job_company: Option<String>,
    pub matched_skills: Vec<String>,
    pub missing_skills: Vec<String>,
    pub tailored_headline: Option<String>,
    pub tailored_bullets: Vec<String>,
    pub language: String,
}

/// Compile a single-column, ATS-verified PDF resume entirely offline using pure-Rust embedded Typst.
pub fn export_ats_pdf(
    ctx: &AtsExportContext,
    font_dirs: &[PathBuf],
    output_dir: &Path,
) -> Result<AtsExportResult, AppError> {
    let fonts = load_bundled_fonts(font_dirs);
    if fonts.is_empty() {
        return Err(AppError::validation_error(
            "Font ATS untuk ekspor PDF tidak ditemukan di resources/fonts.",
        ));
    }

    // Prioritize skills that match the target job first for maximum ATS visibility
    let mut ordered_skills: Vec<String> = Vec::new();
    for s in &ctx.matched_skills {
        if !ordered_skills.iter().any(|x| x.eq_ignore_ascii_case(s)) {
            ordered_skills.push(s.clone());
        }
    }
    for s in &ctx.user_skills {
        if !ordered_skills.iter().any(|x| x.eq_ignore_ascii_case(s)) {
            ordered_skills.push(s.clone());
        }
    }

    let mut experiences = ctx.master_cv.experiences.clone();
    if !ctx.tailored_bullets.is_empty() {
        if let Some(first) = experiences.first_mut() {
            if first.bullets.is_empty() {
                first.bullets = ctx.tailored_bullets.clone();
            }
        }
    }

    let is_en = ctx.language.eq_ignore_ascii_case("en");
    let labels = if is_en {
        AtsTemplateLabels {
            summary: "Professional Summary".into(),
            skills: "Core Skills & Competencies".into(),
            experience: "Experience".into(),
            projects: "Projects & Achievements".into(),
            education: "Education".into(),
            additional: "Certifications & Languages".into(),
            certifications: "Certifications".into(),
            languages: "Languages".into(),
        }
    } else {
        AtsTemplateLabels {
            summary: "Ringkasan Profesional".into(),
            skills: "Keahlian & Kompetensi Utama".into(),
            experience: "Pengalaman Kerja".into(),
            projects: "Proyek & Pencapaian".into(),
            education: "Pendidikan".into(),
            additional: "Sertifikasi & Bahasa".into(),
            certifications: "Sertifikasi".into(),
            languages: "Bahasa".into(),
        }
    };

    let full_name = if ctx.master_cv.full_name.trim().is_empty() {
        "Kandidat Vecta".into()
    } else {
        ctx.master_cv.full_name.trim().to_string()
    };
    let headline = ctx
        .tailored_headline
        .clone()
        .filter(|h| !h.trim().is_empty())
        .unwrap_or_else(|| ctx.master_cv.headline.clone());

    let payload = AtsTemplatePayload {
        full_name: full_name.clone(),
        headline,
        email: ctx.master_cv.email.clone(),
        phone: ctx.master_cv.phone.clone(),
        location: ctx.master_cv.location.clone(),
        linkedin_url: ctx.master_cv.linkedin_url.clone(),
        portfolio_url: ctx.master_cv.portfolio_url.clone(),
        summary: ctx.master_cv.summary.clone(),
        skills: ordered_skills,
        experiences: experiences
            .into_iter()
            .filter(|e| !e.role.trim().is_empty() || !e.company.trim().is_empty())
            .collect(),
        projects: ctx
            .master_cv
            .projects
            .iter()
            .filter(|p| !p.name.trim().is_empty())
            .cloned()
            .collect(),
        education: ctx
            .master_cv
            .education
            .iter()
            .filter(|e| !e.institution.trim().is_empty())
            .cloned()
            .collect(),
        certifications: ctx.master_cv.certifications.clone(),
        languages: ctx.master_cv.languages.clone(),
        labels,
    };

    let cv_json = serde_json::to_string(&payload)
        .map_err(|e| AppError::db_error(format!("Gagal serialisasi data CV: {e}")))?;

    let mut inputs = Dict::new();
    inputs.insert("cv_json".into(), cv_json.into_value());

    let engine = TypstEngine::builder()
        .main_file(ATS_CLASSIC_TEMPLATE)
        .fonts(fonts)
        .build();

    let compiled = engine.compile_with_input(inputs);
    let document = compiled.output.map_err(|e| {
        eprintln!("[vecta::cv_export] Typst compile error: {e:?}");
        AppError::db_error("Gagal mengompilasi template CV ATS menjadi PDF.")
    })?;

    let pdf_bytes = typst_pdf::pdf(&document, &PdfOptions::default()).map_err(|e| {
        eprintln!("[vecta::cv_export] Typst PDF export error: {e:?}");
        AppError::db_error("Gagal mengekspor dokumen PDF ATS.")
    })?;

    std::fs::create_dir_all(output_dir).map_err(|e| {
        AppError::db_error(format!("Gagal membuat folder ekspor CV: {e}"))
    })?;

    let name_slug = sanitize_filename_slug(&full_name);
    let suffix = match (&ctx.job_company, &ctx.job_title) {
        (Some(comp), Some(title)) => format!(
            "_{}_{}",
            sanitize_filename_slug(comp),
            sanitize_filename_slug(title)
        ),
        _ => "_Master_ATS".into(),
    };
    let filename = format!("CV_{name_slug}{suffix}.pdf");
    let out_path = output_dir.join(&filename);
    std::fs::write(&out_path, &pdf_bytes).map_err(|e| {
        AppError::db_error(format!("Gagal menulis file PDF ATS ke disk: {e}"))
    })?;

    let total_req = ctx.matched_skills.len() + ctx.missing_skills.len();
    let ats_keyword_score = if total_req == 0 {
        100.0
    } else {
        ((ctx.matched_skills.len() as f64 / total_req as f64) * 100.0).round()
    };

    Ok(AtsExportResult {
        file_name: filename,
        file_path: out_path.display().to_string(),
        ats_keyword_score,
        matched_keywords: ctx.matched_skills.clone(),
        missing_keywords: ctx.missing_skills.clone(),
        pdf_size_bytes: pdf_bytes.len() as u32,
        pdf_base64: BASE64.encode(&pdf_bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_real_ats_extractable_pdf_with_embedded_typst() {
        let tmp = std::env::temp_dir().join("vecta_ats_pdf_test");
        let font_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("fonts");
        let ctx = AtsExportContext {
            master_cv: MasterCvProfile {
                user_id: "u1".into(),
                full_name: "Budi Santoso".into(),
                headline: "Tax & Accounting Specialist".into(),
                email: "budi@example.com".into(),
                phone: "+62 812-0000-1111".into(),
                location: "Bandung".into(),
                linkedin_url: "https://linkedin.com/in/budisantoso".into(),
                portfolio_url: String::new(),
                summary: "Akuntan berpengalaman menyusun laporan keuangan sesuai PSAK dan kepatuhan pajak.".into(),
                experiences: vec![CvExperienceItem {
                    role: "Accounting Staff".into(),
                    company: "PT Nusantara Sejahtera".into(),
                    location: "Bandung".into(),
                    period: "2023 - Sekarang".into(),
                    bullets: vec![
                        "Menyusun laporan keuangan bulanan dan rekonsiliasi bank dengan Accurate Online.".into(),
                        "Mengelola pelaporan PPh 21, PPh 23, dan PPN melalui e-Faktur dan e-Bupot.".into(),
                    ],
                }],
                education: vec![CvEducationItem {
                    institution: "Universitas Padjadjaran".into(),
                    degree: "S1 Akuntansi".into(),
                    period: "2019 - 2023".into(),
                    notes: "IPK 3.78".into(),
                }],
                projects: vec![],
                certifications: vec!["Brevet Pajak A & B".into()],
                languages: vec!["Bahasa Indonesia".into(), "Bahasa Inggris".into()],
                updated_at: String::new(),
            },
            user_skills: vec!["Akuntansi".into(), "Brevet Pajak A & B".into(), "Accurate Accounting".into()],
            job_title: Some("Tax Staff".into()),
            job_company: Some("PT Maju Bersama".into()),
            matched_skills: vec!["Brevet Pajak A & B".into(), "Akuntansi".into()],
            missing_skills: vec!["SAP FICO".into()],
            tailored_headline: None,
            tailored_bullets: vec![],
            language: "id".into(),
        };

        let res = export_ats_pdf(&ctx, &[font_dir], &tmp).expect("Typst ATS PDF must compile");
        assert!(res.pdf_size_bytes > 5_000);
        assert_eq!(res.ats_keyword_score, 67.0);

        // Verify the exported PDF is truly ATS-readable (extractable text roundtrip!)
        let bytes = std::fs::read(&res.file_path).expect("read generated pdf");
        let extracted = pdf_extract::extract_text_from_mem(&bytes).expect("extract text from ATS pdf");
        assert!(extracted.contains("Budi Santoso"));
        assert!(extracted.contains("Brevet Pajak A & B"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
