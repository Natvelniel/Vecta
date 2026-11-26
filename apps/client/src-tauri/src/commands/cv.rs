use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use tauri::State;

use crate::core::cv_export::{export_ats_pdf, AtsExportContext};
use crate::core::cv_import::import_cv_from_pdf_bytes;
use crate::core::story_profiler::analyze_story_to_imported_draft;
use crate::error::AppError;
use crate::models::cv::{AtsExportResult, ImportedCvDraft, MasterCvProfile};
use crate::state::AppState;

pub fn import_cv_pdf_impl(pdf_base64: String) -> Result<ImportedCvDraft, AppError> {
    let trimmed = pdf_base64.trim();
    let payload = trimmed
        .split_once("base64,")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let bytes = BASE64
        .decode(payload)
        .map_err(|_| AppError::validation_error("Encoding file PDF tidak valid."))?;
    import_cv_from_pdf_bytes(&bytes)
}

pub fn parse_story_local_impl(
    state: Option<&AppState>,
    raw_story: String,
) -> Result<ImportedCvDraft, AppError> {
    let embedder = state.map(|s| &s.local_embedder);
    analyze_story_to_imported_draft(&raw_story, embedder.map(|v| v.as_ref()))
}

pub fn get_master_cv_impl(state: &AppState, user_id: String) -> Result<MasterCvProfile, AppError> {
    state.db.get_master_cv(&user_id)
}

pub fn save_master_cv_impl(
    state: &AppState,
    cv: MasterCvProfile,
) -> Result<MasterCvProfile, AppError> {
    if cv.user_id.trim().is_empty() {
        return Err(AppError::validation_error(
            "User ID wajib diisi untuk menyimpan Master CV.",
        ));
    }
    state.db.save_master_cv(&cv)
}

pub fn export_tailored_ats_pdf_impl(
    state: &AppState,
    user_id: String,
    job_id: Option<String>,
    language: String,
) -> Result<AtsExportResult, AppError> {
    let master_cv = state.db.get_master_cv(&user_id)?;
    let profile = state.db.get_profile_by_id(&user_id)?;
    let user_skills: Vec<String> = profile
        .as_ref()
        .map(|p| p.skills.iter().map(|s| s.canonical_name.clone()).collect())
        .unwrap_or_default();

    let (job_title, job_company, matched_skills, missing_skills, tailored_headline, tailored_bullets) =
        if let Some(ref jid) = job_id {
            let detail = state.db.get_job_detail(&user_id, jid)?;
            let matched = detail
                .matched_skills
                .iter()
                .map(|s| s.canonical_name.clone())
                .collect();
            let missing = detail
                .missing_skills
                .iter()
                .map(|s| s.canonical_name.clone())
                .collect();
            let (hl, bul) = detail
                .ai_apply_kit
                .as_ref()
                .map(|k| (Some(k.tailored_headline.clone()), k.cv_bullets.clone()))
                .unwrap_or((None, vec![]));
            (
                Some(detail.title),
                Some(detail.company),
                matched,
                missing,
                hl,
                bul,
            )
        } else {
            (None, None, user_skills.clone(), vec![], None, vec![])
        };

    let ctx = AtsExportContext {
        master_cv,
        user_skills,
        job_title,
        job_company,
        matched_skills,
        missing_skills,
        tailored_headline,
        tailored_bullets,
        language: if language.eq_ignore_ascii_case("en") {
            "en".into()
        } else {
            "id".into()
        },
    };

    export_ats_pdf(&ctx, &state.font_dirs, &state.export_dir)
}

#[tauri::command]
#[specta::specta]
pub fn import_cv_pdf(pdf_base64: String) -> Result<ImportedCvDraft, AppError> {
    import_cv_pdf_impl(pdf_base64)
}

#[tauri::command]
#[specta::specta]
pub fn parse_story_local(
    state: State<'_, AppState>,
    raw_story: String,
) -> Result<ImportedCvDraft, AppError> {
    parse_story_local_impl(Some(&state), raw_story)
}

#[tauri::command]
#[specta::specta]
pub fn get_master_cv(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<MasterCvProfile, AppError> {
    get_master_cv_impl(&state, user_id)
}

#[tauri::command]
#[specta::specta]
pub fn save_master_cv(
    state: State<'_, AppState>,
    cv: MasterCvProfile,
) -> Result<MasterCvProfile, AppError> {
    save_master_cv_impl(&state, cv)
}

#[tauri::command]
#[specta::specta]
pub fn export_tailored_ats_pdf(
    state: State<'_, AppState>,
    user_id: String,
    job_id: Option<String>,
    language: String,
) -> Result<AtsExportResult, AppError> {
    export_tailored_ats_pdf_impl(&state, user_id, job_id, language)
}
