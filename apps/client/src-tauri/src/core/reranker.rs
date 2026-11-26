use vecta_taxonomy::TaxonomyEngine;

use crate::error::AppError;
use crate::state::AppState;

const DOMAIN_MISMATCH_SCORE_CAP: f64 = 0.48;
const MAX_RERANK_TOP_N: usize = 25;

pub fn rerank_top_jobs_for_user(
    state: &AppState,
    user_id: &str,
    top_n: usize,
) -> Result<u32, AppError> {
    if !state.local_embedder.has_reranker() {
        return Ok(0);
    }

    let profile = match state.db.get_profile_by_id(user_id)? {
        Some(p) => p,
        None => return Ok(0),
    };

    let limit = top_n.clamp(1, MAX_RERANK_TOP_N);
    let candidates = state.db.load_top_jobs_for_rerank(user_id, limit)?;
    if candidates.is_empty() {
        return Ok(0);
    }

    let skill_list: Vec<String> = profile
        .skills
        .iter()
        .take(14)
        .map(|s| s.canonical_name.clone())
        .collect();
    let story_excerpt: String = profile
        .summary
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .or(profile.raw_story_text.as_deref())
        .unwrap_or("")
        .chars()
        .take(360)
        .collect();
    let query = format!(
        "Target Role: {}. Level: {}. Ringkasan: {} Keahlian: {}",
        profile.target_roles.join(", "),
        profile.experience_level.as_str(),
        story_excerpt,
        skill_list.join(", ")
    );

    let passages: Vec<String> = candidates
        .iter()
        .map(|c| {
            let desc_excerpt: String = c
                .role_summary
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(&c.raw_description)
                .chars()
                .take(420)
                .collect();
            format!(
                "Posisi: {} ({}). Keahlian: {}. Deskripsi: {}",
                c.title,
                c.normalized_role,
                c.skill_names.join(", "),
                desc_excerpt
            )
        })
        .collect();

    let probs = state
        .local_embedder
        .rerank_pairs(&query, &passages)
        .map_err(|e| AppError::db_error(format!("Cross-Encoder reranker gagal: {e}")))?;

    let mut updates = Vec::with_capacity(candidates.len());
    for (cand, prob) in candidates.into_iter().zip(probs.into_iter()) {
        let p = (prob as f64).clamp(0.0, 1.0);
        let mut blended = (0.60 * cand.stage1_score + 0.40 * p).clamp(0.0, 1.0);
        if cand.domain_score.is_some_and(|ds| ds <= 0.30) && cand.matched_skills_count == 0 {
            blended = blended.min(DOMAIN_MISMATCH_SCORE_CAP);
        }

        let mut bullets: Vec<String> =
            serde_json::from_str(&cand.match_explanation_json).unwrap_or_default();
        bullets.retain(|b| !b.starts_with("Diverifikasi Reranker Neural"));
        let pct = (p * 100.0).round() as u32;
        bullets.insert(0, format!("Diverifikasi Reranker Neural (relevansi {pct}%)"));
        bullets.truncate(4);
        let expl_json = serde_json::to_string(&bullets).unwrap_or_else(|_| "[]".to_string());

        updates.push((cand.job_id, p, blended, expl_json));
    }

    let updated = state.db.apply_reranker_scores(user_id, &updates)?;
    let _ = TaxonomyEngine::global();
    Ok(updated)
}

pub fn spawn_background_rerank(app: tauri::AppHandle, user_id: String, top_n: usize) {
    use tauri::{Emitter, Manager};
    tauri::async_runtime::spawn(async move {
        let app_bg = app.clone();
        let uid_bg = user_id.clone();
        let res = tokio::task::spawn_blocking(move || {
            let state = app_bg.state::<AppState>();
            rerank_top_jobs_for_user(&state, &uid_bg, top_n)
        })
        .await;
        if let Ok(Ok(count)) = res {
            let _ = app.emit(
                "rerank-complete",
                serde_json::json!({ "user_id": user_id, "reranked_count": count }),
            );
        }
    });
}
