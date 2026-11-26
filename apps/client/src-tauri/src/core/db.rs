use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, params_from_iter, Connection, OptionalExtension, ToSql, Transaction};
use uuid::Uuid;

use crate::core::analytics::{
    build_company_snapshot, compute_salary_benchmark, CompanyJobRow, SalarySampleRow,
};
use crate::core::local_matcher::{taxonomy_seed_skills, LocalSkillEntry};
use crate::core::matching::compute_hybrid_match;
use crate::core::native_scraper::quality::{JobRedFlag, PortalLinkRef};
use crate::core::opportunity::extract_opportunity_metadata;
use crate::error::AppError;
use crate::models::competency::{
    CompanySnapshot, CompetencyMapResponse, CompetencySkillItem, RoleJobCount,
    SalaryBenchmarkSummary, SkillImpactItem,
};
use crate::models::cv::{
    CvEducationItem, CvExperienceItem, CvProjectItem, MasterCvProfile,
};
use crate::models::job::{
    ApplicationStatus, ApplyKitResult, EnrichmentStatus, ExtractedJobSkill, GapAnalysis,
    JobExtractionResult, JobFilterInput, JobMatchDetail, JobMatchSkillRef, JobMatchSummary,
    JobSource, RequirementType, SavedWatchlistItem, WorkMode,
};
use crate::models::profile::{
    ConfirmedKeywordInput, ConfirmedProfileInput, ConfirmedSkillInput, ExperienceLevel, FitType,
    Proficiency, ProfileSummaryItem, ScrapeKeywordItem, SkillCategory, UserProfileDetail,
    UserSkillItem, VectaBackupReport, VectaImportReport,
};
use crate::models::roadmap::{
    GeneratedRoadmap, RoadmapDetail, RoadmapStepItem, RoadmapSummary, SkillRef, StepStatus,
    StepUpdateResult,
};

const INITIAL_SCHEMA_SQL: &str =
    include_str!("../../../../../shared/migrations/001_initial_schema.sql");
const APP_SETTINGS_SCHEMA_SQL: &str =
    include_str!("../../../../../shared/migrations/002_app_settings.sql");
const WORKFLOW_MIGRATION_SQL: &str =
    include_str!("../../../../../shared/migrations/003_job_workflow_and_preferences.sql");
const V3_MIGRATION_SQL: &str =
    include_str!("../../../../../shared/migrations/004_v3_taxonomy_offline_ai_and_cv.sql");
const V5_MIGRATION_SQL: &str =
    include_str!("../../../../../shared/migrations/005_opportunity_intelligence.sql");

const PRAGMAS_SQL: &str = "
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
";

#[derive(Debug, Clone)]
pub struct ProfileMatchRecord {
    pub user_id: String,
    pub experience_level: ExperienceLevel,
    pub embedding: Option<Vec<u8>>,
    pub skill_ids: HashSet<i64>,
    pub skill_names_lower: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct JobSkillRow {
    pub skill_id: i64,
    pub canonical_name: String,
    pub category: SkillCategory,
    pub requirement_type: RequirementType,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct JobMatchRecord {
    pub job_id: String,
    pub experience_level: ExperienceLevel,
    pub embedding: Option<Vec<u8>>,
    pub skills: Vec<JobSkillRow>,
}

#[derive(Debug, Clone)]
pub struct NewJobRecord {
    pub id: String,
    pub source: JobSource,
    pub external_id: String,
    pub url: Option<String>,
    pub title: String,
    pub normalized_role: String,
    pub company: String,
    pub location: Option<String>,
    pub work_mode: WorkMode,
    pub experience_level: ExperienceLevel,
    pub raw_description: String,
    pub extracted_summary_json: Option<String>,
    pub embedding: Option<Vec<u8>>,
    pub enrichment_status: EnrichmentStatus,
    pub posted_at: Option<String>,
    pub salary_text: Option<String>,
    pub domain_slug: String,
    pub quality_score: f64,
    pub red_flags_json: String,
    pub also_available_on_json: String,
    pub embedding_model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PendingAiJobRow {
    pub job_id: String,
    pub source: String,
    pub title: String,
    pub company: String,
    pub raw_description: String,
}

#[derive(Debug, Clone)]
pub struct JobMatchUpsert {
    pub user_id: String,
    pub job_id: String,
    pub vector_score: f64,
    pub skill_overlap_score: f64,
    pub level_score: f64,
    pub final_score: f64,
    pub matched_skills_json: String,
    pub missing_skills_json: String,
}

#[derive(Debug, Clone)]
pub struct MissingSkillDemand {
    pub skill_id: i64,
    pub canonical_name: String,
    pub category: SkillCategory,
    pub demand_count: u32,
}

pub struct Database {
    conn: Mutex<Connection>,
}

fn db_err(context: &str, err: rusqlite::Error) -> AppError {
    eprintln!("[vecta::db] {context}: {err}");
    AppError::db_error(format!("{context}. Periksa penyimpanan lokal lalu coba lagi."))
}

fn escape_like_pattern(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '%' | '_' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn configure_connection(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(PRAGMAS_SQL)
        .map_err(|e| db_err("Gagal menerapkan pragma SQLite", e))
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::db_error(format!("Gagal membuat direktori database: {e}"))
            })?;
        }
        let conn = Connection::open(path)
            .map_err(|e| db_err("Gagal membuka koneksi database SQLite", e))?;
        configure_connection(&conn)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| db_err("Gagal membuka database SQLite in-memory", e))?;
        configure_connection(&conn)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn run_migrations(&self) -> Result<(), AppError> {
        self.with_tx(|tx| {
            tx.execute_batch(INITIAL_SCHEMA_SQL)
                .map_err(|e| db_err("Gagal menjalankan migrasi 001_initial_schema", e))?;
            tx.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (?1, ?2)",
                params![1i64, "001_initial_schema"],
            )
            .map_err(|e| db_err("Gagal mencatat versi migrasi", e))?;
            tx.execute_batch(APP_SETTINGS_SCHEMA_SQL)
                .map_err(|e| db_err("Gagal menjalankan migrasi 002_app_settings", e))?;
            tx.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (?1, ?2)",
                params![2i64, "002_app_settings"],
            )
            .map_err(|e| db_err("Gagal mencatat versi migrasi", e))?;
            Ok(())
        })?;

        let has_v3: bool = self.with_conn(|conn| {
            conn.query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 3",
                [],
                |_| Ok(true),
            )
            .optional()
            .map(|o| o.unwrap_or(false))
            .map_err(|e| db_err("Gagal memeriksa versi migrasi 3", e))
        })?;

        if !has_v3 {
            let mut guard = self
                .conn
                .lock()
                .map_err(|_| AppError::db_error("Database mutex poisoned"))?;
            guard
                .execute_batch("PRAGMA foreign_keys = OFF;")
                .map_err(|e| db_err("Gagal menonaktifkan FK sementara untuk migrasi 003", e))?;

            let mig_result = (|| -> Result<(), AppError> {
                let tx = guard
                    .transaction()
                    .map_err(|e| db_err("Gagal memulai transaksi migrasi 003", e))?;
                tx.execute_batch(WORKFLOW_MIGRATION_SQL).map_err(|e| {
                    db_err("Gagal menjalankan migrasi 003_job_workflow_and_preferences", e)
                })?;
                tx.execute(
                    "INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (?1, ?2)",
                    params![3i64, "003_job_workflow_and_preferences"],
                )
                .map_err(|e| db_err("Gagal mencatat versi migrasi 3", e))?;
                tx.commit()
                    .map_err(|e| db_err("Gagal commit migrasi 003", e))?;
                Ok(())
            })();

            let fk_restore = guard
                .execute_batch("PRAGMA foreign_keys = ON;")
                .map_err(|e| db_err("Gagal mengaktifkan kembali FK pasca migrasi 003", e));

            mig_result?;
            fk_restore?;
        }

        let has_v4: bool = self.with_conn(|conn| {
            conn.query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 4",
                [],
                |_| Ok(true),
            )
            .optional()
            .map(|o| o.unwrap_or(false))
            .map_err(|e| db_err("Gagal memeriksa versi migrasi 4", e))
        })?;

        if !has_v4 {
            let mut guard = self
                .conn
                .lock()
                .map_err(|_| AppError::db_error("Database mutex poisoned"))?;
            guard
                .execute_batch("PRAGMA foreign_keys = OFF;")
                .map_err(|e| db_err("Gagal menonaktifkan FK sementara untuk migrasi 004", e))?;

            let mig_result = (|| -> Result<(), AppError> {
                let tx = guard
                    .transaction()
                    .map_err(|e| db_err("Gagal memulai transaksi migrasi 004", e))?;
                tx.execute_batch(V3_MIGRATION_SQL).map_err(|e| {
                    db_err("Gagal menjalankan migrasi 004_v3_taxonomy_offline_ai_and_cv", e)
                })?;
                tx.execute(
                    "INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (?1, ?2)",
                    params![4i64, "004_v3_taxonomy_offline_ai_and_cv"],
                )
                .map_err(|e| db_err("Gagal mencatat versi migrasi 4", e))?;
                tx.commit()
                    .map_err(|e| db_err("Gagal commit migrasi 004", e))?;
                Ok(())
            })();

            let fk_restore = guard
                .execute_batch("PRAGMA foreign_keys = ON;")
                .map_err(|e| db_err("Gagal mengaktifkan kembali FK pasca migrasi 004", e));

            mig_result?;
            fk_restore?;
        }

        let has_v5: bool = self.with_conn(|conn| {
            conn.query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 5",
                [],
                |_| Ok(true),
            )
            .optional()
            .map(|o| o.unwrap_or(false))
            .map_err(|e| db_err("Gagal memeriksa versi migrasi 5", e))
        })?;

        if !has_v5 {
            let mut guard = self
                .conn
                .lock()
                .map_err(|_| AppError::db_error("Database mutex poisoned"))?;
            guard
                .execute_batch("PRAGMA foreign_keys = OFF;")
                .map_err(|e| db_err("Gagal menonaktifkan FK sementara untuk migrasi 005", e))?;

            let mig_result = (|| -> Result<(), AppError> {
                let tx = guard
                    .transaction()
                    .map_err(|e| db_err("Gagal memulai transaksi migrasi 005", e))?;
                tx.execute_batch(V5_MIGRATION_SQL).map_err(|e| {
                    db_err("Gagal menjalankan migrasi 005_opportunity_intelligence", e)
                })?;

                // Backfill location_cluster, salary range, and opportunity_type on existing jobs
                let existing: Vec<(String, String, Option<String>, String, Option<String>, String)> = {
                    let mut stmt = tx
                        .prepare("SELECT id, title, location, work_mode, salary_text, raw_description FROM jobs")
                        .map_err(|e| db_err("Gagal membaca jobs untuk backfill v5", e))?;
                    let rows = stmt
                        .query_map([], |r| {
                            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
                        })
                        .map_err(|e| db_err("Gagal iterasi backfill v5", e))?;
                    let mut buf = Vec::new();
                    for row in rows {
                        buf.push(row.map_err(|e| db_err("Gagal parse baris backfill v5", e))?);
                    }
                    buf
                };

                for (id, title, loc, wm_str, sal, desc) in existing {
                    let meta = extract_opportunity_metadata(
                        &title,
                        loc.as_deref(),
                        WorkMode::from_db_str(&wm_str),
                        sal.as_deref(),
                        &desc,
                    );
                    tx.execute(
                        "UPDATE jobs SET location_cluster = ?1, salary_min_idr = ?2, salary_max_idr = ?3, opportunity_type = ?4 WHERE id = ?5",
                        params![
                            meta.location_cluster,
                            meta.salary_min_idr,
                            meta.salary_max_idr,
                            meta.opportunity_type,
                            id
                        ],
                    )
                    .map_err(|e| db_err("Gagal update backfill v5", e))?;
                }

                tx.execute(
                    "INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (?1, ?2)",
                    params![5i64, "005_opportunity_intelligence"],
                )
                .map_err(|e| db_err("Gagal mencatat versi migrasi 5", e))?;
                tx.commit()
                    .map_err(|e| db_err("Gagal commit migrasi 005", e))?;
                Ok(())
            })();

            let fk_restore = guard
                .execute_batch("PRAGMA foreign_keys = ON;")
                .map_err(|e| db_err("Gagal mengaktifkan kembali FK pasca migrasi 005", e));

            mig_result?;
            fk_restore?;
        }
        Ok(())
    }

    pub fn ensure_seed_skills(&self) -> Result<(), AppError> {
        let seeds = taxonomy_seed_skills();
        self.with_tx(|tx| {
            for (name, category, aliases) in &seeds {
                let _ = Self::upsert_skill(tx, name, *category, aliases)?;
            }
            Ok(())
        })
    }

    pub fn list_all_skills_for_local_matching(&self) -> Result<Vec<LocalSkillEntry>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, canonical_name, category, aliases_json FROM skills")
                .map_err(|e| db_err("Gagal menyiapkan query kamus skill lokal", e))?;
            let rows = stmt
                .query_map([], |r| {
                    let cat_str: String = r.get(2)?;
                    let aliases_json: String = r.get(3)?;
                    let aliases: Vec<String> =
                        serde_json::from_str(&aliases_json).unwrap_or_default();
                    Ok(LocalSkillEntry {
                        skill_id: r.get(0)?,
                        canonical_name: r.get(1)?,
                        category: SkillCategory::from_db_str(&cat_str),
                        aliases,
                    })
                })
                .map_err(|e| db_err("Gagal membaca kamus skill lokal", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal parsing kamus skill lokal", e))?);
            }
            Ok(out)
        })
    }

    pub fn get_app_setting(&self, key: &str) -> Result<Option<String>, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| db_err("Gagal membaca pengaturan lokal", e))
        })
    }

    pub fn set_app_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO app_settings (key, value, updated_at)
                 VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET
                    value = excluded.value,
                    updated_at = datetime('now')",
                params![key, value],
            )
            .map_err(|e| db_err("Gagal menyimpan pengaturan lokal", e))?;
            Ok(())
        })
    }

    pub fn with_conn<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> Result<T, AppError>,
    {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::db_error("Database mutex poisoned"))?;
        f(&guard)
    }

    pub fn with_tx<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Transaction<'_>) -> Result<T, AppError>,
    {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppError::db_error("Database mutex poisoned"))?;
        let tx = guard
            .transaction()
            .map_err(|e| db_err("Gagal memulai transaksi SQLite", e))?;
        let result = f(&tx)?;
        tx.commit()
            .map_err(|e| db_err("Gagal commit transaksi SQLite", e))?;
        Ok(result)
    }

    pub fn upsert_skill(
        tx: &Transaction<'_>,
        canonical_name: &str,
        category: SkillCategory,
        aliases: &[String],
    ) -> Result<i64, AppError> {
        let trimmed = canonical_name.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation_error("Nama skill tidak boleh kosong."));
        }

        // 1. Exact case-insensitive match on canonical_name
        let existing: Option<(i64, String)> = tx
            .query_row(
                "SELECT id, aliases_json FROM skills WHERE canonical_name = ?1 COLLATE NOCASE",
                [trimmed],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| db_err("Gagal mencari skill baku", e))?;

        if let Some((skill_id, existing_aliases_json)) = existing {
            merge_skill_aliases(tx, skill_id, &existing_aliases_json, aliases)?;
            return Ok(skill_id);
        }

        // 2. Fallback scan against canonical_name & aliases_json in both directions (case-insensitive)
        let mut incoming_needles = HashSet::new();
        incoming_needles.insert(trimmed.to_lowercase());
        for alias in aliases {
            let a = alias.trim();
            if !a.is_empty() {
                incoming_needles.insert(a.to_lowercase());
            }
        }

        let mut stmt = tx
            .prepare("SELECT id, canonical_name, aliases_json FROM skills")
            .map_err(|e| db_err("Gagal menyiapkan scan alias skill", e))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| db_err("Gagal membaca daftar alias skill", e))?;

        let mut alias_match: Option<(i64, String, String)> = None;
        for row in rows {
            let (sid, existing_canonical, aliases_json) =
                row.map_err(|e| db_err("Gagal membaca baris skill", e))?;
            let canonical_lower = existing_canonical.trim().to_lowercase();
            let parsed: Vec<String> = serde_json::from_str(&aliases_json).unwrap_or_default();
            if incoming_needles.contains(&canonical_lower)
                || parsed
                    .iter()
                    .any(|a| incoming_needles.contains(&a.trim().to_lowercase()))
            {
                alias_match = Some((sid, existing_canonical, aliases_json));
                break;
            }
        }
        drop(stmt);

        if let Some((skill_id, existing_canonical, existing_aliases_json)) = alias_match {
            let mut to_merge: Vec<String> = Vec::with_capacity(aliases.len() + 1);
            if !trimmed.eq_ignore_ascii_case(existing_canonical.trim()) {
                to_merge.push(trimmed.to_string());
            }
            for alias in aliases {
                let a = alias.trim();
                if !a.is_empty() && !a.eq_ignore_ascii_case(existing_canonical.trim()) {
                    to_merge.push(a.to_string());
                }
            }
            merge_skill_aliases(tx, skill_id, &existing_aliases_json, &to_merge)?;
            return Ok(skill_id);
        }

        // 3. Insert new canonical skill
        let deduped_aliases = normalize_alias_list(aliases);
        let aliases_json = serde_json::to_string(&deduped_aliases)
            .map_err(|e| AppError::db_error(format!("Gagal serialisasi aliases: {e}")))?;

        tx.execute(
            "INSERT INTO skills (canonical_name, category, aliases_json) VALUES (?1, ?2, ?3)",
            params![trimmed, category.as_str(), aliases_json],
        )
        .map_err(|e| db_err("Gagal menyimpan skill baru", e))?;

        Ok(tx.last_insert_rowid())
    }

    pub fn get_top_canonical_skills(&self, limit: usize) -> Result<Vec<String>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT s.canonical_name
                     FROM skills s
                     LEFT JOIN job_skills js ON js.skill_id = s.id
                     LEFT JOIN user_skills us ON us.skill_id = s.id
                     GROUP BY s.id
                     ORDER BY (COUNT(DISTINCT js.job_id) + COUNT(DISTINCT us.user_id)) DESC, s.canonical_name ASC
                     LIMIT ?1",
                )
                .map_err(|e| db_err("Gagal menyiapkan query top skills", e))?;

            let rows = stmt
                .query_map([limit as i64], |r| r.get::<_, String>(0))
                .map_err(|e| db_err("Gagal menjalankan query top skills", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal membaca top skill", e))?);
            }
            Ok(out)
        })
    }

    pub fn save_profile_atomic(
        &self,
        input: &ConfirmedProfileInput,
        embedding: Option<&[u8]>,
    ) -> Result<UserProfileDetail, AppError> {
        let profile_name = input.profile_name.trim();
        if profile_name.is_empty() {
            return Err(AppError::validation_error("Nama profil tidak boleh kosong."));
        }

        let user_id = input
            .id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let target_roles_json = serde_json::to_string(&input.target_roles)
            .map_err(|e| AppError::db_error(format!("Gagal serialisasi target_roles: {e}")))?;

        self.with_tx(|tx| {
            tx.execute("UPDATE user_profiles SET is_active = 0 WHERE id != ?1", [&user_id])
                .map_err(|e| db_err("Gagal menonaktifkan profil lama", e))?;

            tx.execute(
                "INSERT INTO user_profiles (
                    id, profile_name, target_roles_json, raw_story_text, summary,
                    experience_level, embedding, is_active, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT(id) DO UPDATE SET
                    profile_name = excluded.profile_name,
                    target_roles_json = excluded.target_roles_json,
                    raw_story_text = excluded.raw_story_text,
                    summary = excluded.summary,
                    experience_level = excluded.experience_level,
                    embedding = excluded.embedding,
                    is_active = 1,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    user_id,
                    profile_name,
                    target_roles_json,
                    input.raw_story_text,
                    input.summary,
                    input.experience_level.as_str(),
                    embedding,
                ],
            )
            .map_err(|e| db_err("Gagal menyimpan profil pengguna", e))?;

            tx.execute("DELETE FROM user_skills WHERE user_id = ?1", [&user_id])
                .map_err(|e| db_err("Gagal mereset user_skills", e))?;

            for skill in &input.skills {
                let skill_id = Self::upsert_skill(tx, &skill.canonical_name, skill.category, &[])?;
                tx.execute(
                    "INSERT INTO user_skills (user_id, skill_id, proficiency, evidence)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(user_id, skill_id) DO UPDATE SET
                        proficiency = excluded.proficiency,
                        evidence = excluded.evidence",
                    params![
                        user_id,
                        skill_id,
                        skill.proficiency.as_str(),
                        skill.evidence
                    ],
                )
                .map_err(|e| db_err("Gagal menyimpan user_skill", e))?;
            }

            tx.execute(
                "DELETE FROM user_scrape_keywords WHERE user_id = ?1",
                [&user_id],
            )
            .map_err(|e| db_err("Gagal mereset user_scrape_keywords", e))?;

            for kw in &input.keywords {
                let role_label = kw.role_label.trim();
                if role_label.is_empty() {
                    continue;
                }
                let queries = if kw.scrape_queries.is_empty() {
                    vec![role_label.to_string()]
                } else {
                    kw.scrape_queries.clone()
                };
                let queries_json = serde_json::to_string(&queries)
                    .map_err(|e| AppError::db_error(format!("Gagal serialisasi queries: {e}")))?;
                tx.execute(
                    "INSERT INTO user_scrape_keywords (
                        user_id, role_label, fit_type, scrape_queries_json, match_reason, is_active
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        user_id,
                        role_label,
                        kw.fit_type.as_str(),
                        queries_json,
                        kw.match_reason,
                        if kw.is_active { 1i64 } else { 0i64 },
                    ],
                )
                .map_err(|e| db_err("Gagal menyimpan keyword scraping", e))?;
            }

            load_profile_by_id_conn(tx, &user_id)?
                .ok_or_else(|| AppError::db_error("Profil tidak ditemukan setelah disimpan"))
        })
    }

    pub fn get_active_profile(&self) -> Result<Option<UserProfileDetail>, AppError> {
        self.with_conn(|conn| {
            let active_id: Option<String> = conn
                .query_row(
                    "SELECT id FROM user_profiles WHERE is_active = 1 ORDER BY updated_at DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| db_err("Gagal membaca profil aktif", e))?;

            match active_id {
                Some(id) => load_profile_by_id_conn(conn, &id),
                None => Ok(None),
            }
        })
    }

    pub fn get_profile_by_id(&self, user_id: &str) -> Result<Option<UserProfileDetail>, AppError> {
        self.with_conn(|conn| load_profile_by_id_conn(conn, user_id))
    }

    pub fn get_profile_for_matching(
        &self,
        user_id: &str,
    ) -> Result<Option<ProfileMatchRecord>, AppError> {
        self.with_conn(|conn| {
            let row: Option<(String, Option<Vec<u8>>)> = conn
                .query_row(
                    "SELECT experience_level, embedding FROM user_profiles WHERE id = ?1",
                    [user_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|e| db_err("Gagal memuat profil untuk matching", e))?;

            let Some((level_str, embedding)) = row else {
                return Ok(None);
            };

            let mut stmt = conn
                .prepare(
                    "SELECT us.skill_id, s.canonical_name
                     FROM user_skills us
                     JOIN skills s ON s.id = us.skill_id
                     WHERE us.user_id = ?1",
                )
                .map_err(|e| db_err("Gagal memuat skill pengguna", e))?;
            let skill_rows = stmt
                .query_map([user_id], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(|e| db_err("Gagal membaca skill pengguna", e))?;

            let mut skill_ids = HashSet::new();
            let mut skill_names_lower = HashSet::new();
            for row in skill_rows {
                let (sid, cname) = row.map_err(|e| db_err("Gagal membaca skill_id", e))?;
                skill_ids.insert(sid);
                skill_names_lower.insert(cname.trim().to_lowercase());
            }

            Ok(Some(ProfileMatchRecord {
                user_id: user_id.to_string(),
                experience_level: ExperienceLevel::from_db_str(&level_str),
                embedding,
                skill_ids,
                skill_names_lower,
            }))
        })
    }

    pub fn toggle_scrape_keyword(&self, keyword_id: i64, is_active: bool) -> Result<(), AppError> {
        self.with_conn(|conn| {
            let changed = conn
                .execute(
                    "UPDATE user_scrape_keywords SET is_active = ?1 WHERE id = ?2",
                    params![if is_active { 1i64 } else { 0i64 }, keyword_id],
                )
                .map_err(|e| db_err("Gagal mengubah status keyword", e))?;

            if changed == 0 {
                return Err(AppError::validation_error(format!(
                    "Keyword dengan ID {keyword_id} tidak ditemukan."
                )));
            }
            Ok(())
        })
    }

    pub fn add_custom_keyword(
        &self,
        user_id: &str,
        role_label: &str,
        queries: &[String],
    ) -> Result<ScrapeKeywordItem, AppError> {
        let trimmed_label = role_label.trim();
        if trimmed_label.is_empty() {
            return Err(AppError::validation_error(
                "Label peran kustom tidak boleh kosong.",
            ));
        }

        let cleaned_queries: Vec<String> = queries
            .iter()
            .map(|q| q.trim().to_string())
            .filter(|q| !q.is_empty())
            .collect();
        let final_queries = if cleaned_queries.is_empty() {
            vec![trimmed_label.to_string()]
        } else {
            cleaned_queries
        };
        let queries_json = serde_json::to_string(&final_queries)
            .map_err(|e| AppError::db_error(format!("Gagal serialisasi queries: {e}")))?;

        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO user_scrape_keywords (
                    user_id, role_label, fit_type, scrape_queries_json, match_reason, is_active
                 ) VALUES (?1, ?2, 'custom', ?3, NULL, 1)",
                params![user_id, trimmed_label, queries_json],
            )
            .map_err(|e| db_err("Gagal menambah keyword kustom", e))?;

            let id = conn.last_insert_rowid();
            Ok(ScrapeKeywordItem {
                id,
                user_id: user_id.to_string(),
                role_label: trimmed_label.to_string(),
                fit_type: FitType::Custom,
                scrape_queries: final_queries,
                match_reason: None,
                is_active: true,
                last_scraped_at: None,
            })
        })
    }

    pub fn list_active_scrape_queries(&self, user_id: &str) -> Result<Vec<String>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT scrape_queries_json FROM user_scrape_keywords
                     WHERE user_id = ?1 AND is_active = 1
                     ORDER BY id ASC",
                )
                .map_err(|e| db_err("Gagal membaca keyword scraping aktif", e))?;

            let rows = stmt
                .query_map([user_id], |r| r.get::<_, String>(0))
                .map_err(|e| db_err("Gagal iterasi keyword scraping aktif", e))?;

            let mut seen = HashSet::new();
            let mut out = Vec::new();
            for row in rows {
                let json_str = row.map_err(|e| db_err("Gagal baca queries_json", e))?;
                let parsed: Vec<String> = serde_json::from_str(&json_str).unwrap_or_default();
                for q in parsed {
                    let t = q.trim();
                    if !t.is_empty() && seen.insert(t.to_lowercase()) {
                        out.push(t.to_string());
                    }
                }
            }
            Ok(out)
        })
    }

    pub fn touch_active_keywords_scraped(&self, user_id: &str) -> Result<(), AppError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE user_scrape_keywords SET last_scraped_at = CURRENT_TIMESTAMP
                 WHERE user_id = ?1 AND is_active = 1",
                [user_id],
            )
            .map_err(|e| db_err("Gagal memperbarui last_scraped_at", e))?;
            Ok(())
        })
    }

    pub fn job_exists_by_source_external_id(
        &self,
        source: JobSource,
        external_id: &str,
    ) -> Result<Option<String>, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id FROM jobs WHERE source = ?1 AND external_id = ?2",
                params![source.as_str(), external_id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| db_err("Gagal memeriksa duplikat lowongan", e))
        })
    }

    pub fn insert_job_with_skills(
        &self,
        job: &NewJobRecord,
        extracted_skills: &[ExtractedJobSkill],
    ) -> Result<JobMatchRecord, AppError> {
        let meta = extract_opportunity_metadata(
            &job.title,
            job.location.as_deref(),
            job.work_mode,
            job.salary_text.as_deref(),
            &job.raw_description,
        );
        self.with_tx(|tx| {
            tx.execute(
                "INSERT INTO jobs (
                    id, source, external_id, url, title, normalized_role, company,
                    location, work_mode, experience_level, raw_description,
                    extracted_summary_json, embedding, enrichment_status, posted_at, salary_text,
                    domain_slug, quality_score, red_flags_json, also_available_on_json, embedding_model,
                    location_cluster, salary_min_idr, salary_max_idr, opportunity_type
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
                params![
                    job.id,
                    job.source.as_str(),
                    job.external_id,
                    job.url,
                    job.title,
                    job.normalized_role,
                    job.company,
                    job.location,
                    job.work_mode.as_str(),
                    job.experience_level.as_str(),
                    job.raw_description,
                    job.extracted_summary_json,
                    job.embedding.as_deref(),
                    job.enrichment_status.as_str(),
                    job.posted_at,
                    job.salary_text,
                    if job.domain_slug.is_empty() { "umum_lintas_bidang" } else { &job.domain_slug },
                    job.quality_score,
                    if job.red_flags_json.is_empty() { "[]" } else { &job.red_flags_json },
                    if job.also_available_on_json.is_empty() { "[]" } else { &job.also_available_on_json },
                    job.embedding_model,
                    meta.location_cluster,
                    meta.salary_min_idr,
                    meta.salary_max_idr,
                    meta.opportunity_type,
                ],
            )
            .map_err(|e| db_err("Gagal menyimpan lowongan", e))?;

            for skill in extracted_skills {
                let trimmed = skill.canonical_name.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let skill_id = Self::upsert_skill(
                    tx,
                    trimmed,
                    skill.category,
                    &skill.matched_aliases,
                )?;
                let weight = skill.requirement_type.default_weight();

                tx.execute(
                    "INSERT INTO job_skills (job_id, skill_id, requirement_type, weight)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(job_id, skill_id) DO UPDATE SET
                        requirement_type = CASE
                            WHEN job_skills.requirement_type = 'mandatory' OR excluded.requirement_type = 'mandatory'
                            THEN 'mandatory'
                            ELSE 'nice_to_have'
                        END,
                        weight = MAX(job_skills.weight, excluded.weight)",
                    params![job.id, skill_id, skill.requirement_type.as_str(), weight],
                )
                .map_err(|e| db_err("Gagal menyimpan job_skill", e))?;
            }

            let skill_rows = load_job_skills_conn(tx, &job.id)?;

            Ok(JobMatchRecord {
                job_id: job.id.clone(),
                experience_level: job.experience_level,
                embedding: job.embedding.clone(),
                skills: skill_rows,
            })
        })
    }

    pub fn insert_job_with_skill_ids(
        &self,
        job: &NewJobRecord,
        skill_pairs: &[(i64, RequirementType)],
    ) -> Result<JobMatchRecord, AppError> {
        let meta = extract_opportunity_metadata(
            &job.title,
            job.location.as_deref(),
            job.work_mode,
            job.salary_text.as_deref(),
            &job.raw_description,
        );
        self.with_tx(|tx| {
            tx.execute(
                "INSERT INTO jobs (
                    id, source, external_id, url, title, normalized_role, company,
                    location, work_mode, experience_level, raw_description,
                    extracted_summary_json, embedding, enrichment_status, posted_at, salary_text,
                    domain_slug, quality_score, red_flags_json, also_available_on_json, embedding_model,
                    location_cluster, salary_min_idr, salary_max_idr, opportunity_type
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
                params![
                    job.id,
                    job.source.as_str(),
                    job.external_id,
                    job.url,
                    job.title,
                    job.normalized_role,
                    job.company,
                    job.location,
                    job.work_mode.as_str(),
                    job.experience_level.as_str(),
                    job.raw_description,
                    job.extracted_summary_json,
                    job.embedding.as_deref(),
                    job.enrichment_status.as_str(),
                    job.posted_at,
                    job.salary_text,
                    if job.domain_slug.is_empty() { "umum_lintas_bidang" } else { &job.domain_slug },
                    job.quality_score,
                    if job.red_flags_json.is_empty() { "[]" } else { &job.red_flags_json },
                    if job.also_available_on_json.is_empty() { "[]" } else { &job.also_available_on_json },
                    job.embedding_model,
                    meta.location_cluster,
                    meta.salary_min_idr,
                    meta.salary_max_idr,
                    meta.opportunity_type,
                ],
            )
            .map_err(|e| db_err("Gagal menyimpan lowongan lokal", e))?;

            for &(skill_id, req_type) in skill_pairs {
                let weight = req_type.default_weight();
                tx.execute(
                    "INSERT INTO job_skills (job_id, skill_id, requirement_type, weight)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(job_id, skill_id) DO UPDATE SET
                        requirement_type = CASE
                            WHEN job_skills.requirement_type = 'mandatory' OR excluded.requirement_type = 'mandatory'
                            THEN 'mandatory'
                            ELSE 'nice_to_have'
                        END,
                        weight = MAX(job_skills.weight, excluded.weight)",
                    params![job.id, skill_id, req_type.as_str(), weight],
                )
                .map_err(|e| db_err("Gagal menyimpan skill lokal lowongan", e))?;
            }

            let skill_rows = load_job_skills_conn(tx, &job.id)?;
            Ok(JobMatchRecord {
                job_id: job.id.clone(),
                experience_level: job.experience_level,
                embedding: job.embedding.clone(),
                skills: skill_rows,
            })
        })
    }

    pub fn apply_ai_enrichment_to_job(
        &self,
        job_id: &str,
        extracted: &JobExtractionResult,
        extracted_json: &str,
        embedding: Option<&[u8]>,
    ) -> Result<JobMatchRecord, AppError> {
        self.with_tx(|tx| {
            tx.execute(
                "UPDATE jobs
                 SET normalized_role = ?1,
                     work_mode = ?2,
                     experience_level = ?3,
                     extracted_summary_json = ?4,
                     embedding = COALESCE(?5, embedding),
                     enrichment_status = 'ai_enriched'
                 WHERE id = ?6",
                params![
                    extracted.normalized_role,
                    extracted.work_mode.as_str(),
                    extracted.experience_level.as_str(),
                    extracted_json,
                    embedding,
                    job_id,
                ],
            )
            .map_err(|e| db_err("Gagal memperbarui hasil pengayaan AI lowongan", e))?;

            tx.execute("DELETE FROM job_skills WHERE job_id = ?1", params![job_id])
                .map_err(|e| db_err("Gagal memperbarui daftar skill lowongan", e))?;

            for skill in &extracted.skills {
                let trimmed = skill.canonical_name.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let skill_id =
                    Self::upsert_skill(tx, trimmed, skill.category, &skill.matched_aliases)?;
                let weight = skill.requirement_type.default_weight();
                tx.execute(
                    "INSERT INTO job_skills (job_id, skill_id, requirement_type, weight)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(job_id, skill_id) DO UPDATE SET
                        requirement_type = CASE
                            WHEN job_skills.requirement_type = 'mandatory' OR excluded.requirement_type = 'mandatory'
                            THEN 'mandatory'
                            ELSE 'nice_to_have'
                        END,
                        weight = MAX(job_skills.weight, excluded.weight)",
                    params![job_id, skill_id, skill.requirement_type.as_str(), weight],
                )
                .map_err(|e| db_err("Gagal menyimpan skill AI lowongan", e))?;
            }

            let skill_rows = load_job_skills_conn(tx, job_id)?;
            let stored_emb: Option<Vec<u8>> = tx
                .query_row("SELECT embedding FROM jobs WHERE id = ?1", [job_id], |r| {
                    r.get(0)
                })
                .map_err(|e| db_err("Gagal membaca embedding lowongan", e))?;

            Ok(JobMatchRecord {
                job_id: job_id.to_string(),
                experience_level: extracted.experience_level,
                embedding: stored_emb,
                skills: skill_rows,
            })
        })
    }

    pub fn list_pending_local_jobs_for_ai(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<PendingAiJobRow>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT j.id, j.source, j.title, j.company, j.raw_description
                     FROM jobs j
                     JOIN job_matches jm ON jm.job_id = j.id AND jm.user_id = ?1
                     WHERE j.enrichment_status = 'local_only'
                       AND jm.application_status != 'dismissed'
                     ORDER BY jm.final_score DESC, j.scraped_at DESC
                     LIMIT ?2",
                )
                .map_err(|e| db_err("Gagal menyiapkan query antrean AI lowongan", e))?;

            let rows = stmt
                .query_map(params![user_id, limit as i64], |r| {
                    Ok(PendingAiJobRow {
                        job_id: r.get(0)?,
                        source: r.get(1)?,
                        title: r.get(2)?,
                        company: r.get(3)?,
                        raw_description: r.get(4)?,
                    })
                })
                .map_err(|e| db_err("Gagal membaca antrean AI lowongan", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal parsing antrean AI lowongan", e))?);
            }
            Ok(out)
        })
    }

    pub fn count_pending_local_jobs(&self, user_id: &str) -> Result<u32, AppError> {
        self.with_conn(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*)
                     FROM jobs j
                     JOIN job_matches jm ON jm.job_id = j.id AND jm.user_id = ?1
                     WHERE j.enrichment_status = 'local_only'
                       AND jm.application_status != 'dismissed'",
                    [user_id],
                    |r| r.get(0),
                )
                .map_err(|e| db_err("Gagal menghitung antrean AI lowongan", e))?;
            Ok(count.max(0) as u32)
        })
    }

    pub fn load_job_for_ai_enrichment(&self, job_id: &str) -> Result<Option<PendingAiJobRow>, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, source, title, company, raw_description FROM jobs WHERE id = ?1",
                [job_id],
                |r| {
                    Ok(PendingAiJobRow {
                        job_id: r.get(0)?,
                        source: r.get(1)?,
                        title: r.get(2)?,
                        company: r.get(3)?,
                        raw_description: r.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|e| db_err("Gagal memuat lowongan untuk pengayaan AI", e))
        })
    }

    pub fn upsert_job_match_tx(tx: &Transaction<'_>, row: &JobMatchUpsert) -> Result<(), AppError> {
        let match_id = Uuid::new_v4().to_string();
        let mut stmt = tx
            .prepare_cached(
                "INSERT INTO job_matches (
                    id, user_id, job_id, vector_score, skill_overlap_score, level_score,
                    final_score, matched_skills_json, missing_skills_json, computed_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP)
                 ON CONFLICT(user_id, job_id) DO UPDATE SET
                    vector_score = excluded.vector_score,
                    skill_overlap_score = excluded.skill_overlap_score,
                    level_score = excluded.level_score,
                    final_score = excluded.final_score,
                    matched_skills_json = excluded.matched_skills_json,
                    missing_skills_json = excluded.missing_skills_json,
                    computed_at = CURRENT_TIMESTAMP",
            )
            .map_err(|e| db_err("Gagal menyiapkan upsert job_match", e))?;
        stmt.execute(params![
            match_id,
            row.user_id,
            row.job_id,
            row.vector_score,
            row.skill_overlap_score,
            row.level_score,
            row.final_score,
            row.matched_skills_json,
            row.missing_skills_json,
        ])
        .map_err(|e| db_err("Gagal menyimpan skor matching lowongan", e))?;
        Ok(())
    }

    pub fn upsert_job_matches(&self, rows: &[JobMatchUpsert]) -> Result<(), AppError> {
        self.with_tx(|tx| {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO job_matches (
                        id, user_id, job_id, vector_score, skill_overlap_score, level_score,
                        final_score, matched_skills_json, missing_skills_json, computed_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP)
                     ON CONFLICT(user_id, job_id) DO UPDATE SET
                        vector_score = excluded.vector_score,
                        skill_overlap_score = excluded.skill_overlap_score,
                        level_score = excluded.level_score,
                        final_score = excluded.final_score,
                        matched_skills_json = excluded.matched_skills_json,
                        missing_skills_json = excluded.missing_skills_json,
                        computed_at = CURRENT_TIMESTAMP",
                )
                .map_err(|e| db_err("Gagal menyiapkan batch upsert job_matches", e))?;
            for row in rows {
                let match_id = Uuid::new_v4().to_string();
                stmt.execute(params![
                    match_id,
                    row.user_id,
                    row.job_id,
                    row.vector_score,
                    row.skill_overlap_score,
                    row.level_score,
                    row.final_score,
                    row.matched_skills_json,
                    row.missing_skills_json,
                ])
                .map_err(|e| db_err("Gagal menyimpan skor matching lowongan", e))?;
            }
            Ok(())
        })
    }

    pub fn load_all_jobs_for_matching(&self) -> Result<Vec<JobMatchRecord>, AppError> {
        self.with_conn(|conn| {
            let mut skills_by_job: std::collections::HashMap<String, Vec<JobSkillRow>> =
                std::collections::HashMap::new();
            {
                let mut skill_stmt = conn
                    .prepare_cached(
                        "SELECT js.job_id, js.skill_id, s.canonical_name, s.category, js.requirement_type, js.weight
                         FROM job_skills js
                         JOIN skills s ON s.id = js.skill_id",
                    )
                    .map_err(|e| db_err("Gagal menyiapkan batch query job_skills", e))?;
                let skill_rows = skill_stmt
                    .query_map([], |r| {
                        let job_id: String = r.get(0)?;
                        let cat_str: String = r.get(3)?;
                        let req_str: String = r.get(4)?;
                        Ok((
                            job_id,
                            JobSkillRow {
                                skill_id: r.get(1)?,
                                canonical_name: r.get(2)?,
                                category: SkillCategory::from_db_str(&cat_str),
                                requirement_type: RequirementType::from_db_str(&req_str),
                                weight: r.get(5)?,
                            },
                        ))
                    })
                    .map_err(|e| db_err("Gagal membaca batch job_skills", e))?;

                for row in skill_rows {
                    let (job_id, skill) =
                        row.map_err(|e| db_err("Gagal parsing batch job_skill", e))?;
                    skills_by_job.entry(job_id).or_default().push(skill);
                }
            }

            let mut job_stmt = conn
                .prepare_cached("SELECT id, experience_level, embedding FROM jobs")
                .map_err(|e| db_err("Gagal memuat daftar lowongan untuk matching", e))?;
            let job_rows = job_stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<Vec<u8>>>(2)?,
                    ))
                })
                .map_err(|e| db_err("Gagal iterasi lowongan untuk matching", e))?;

            let mut jobs = Vec::new();
            for row in job_rows {
                let (job_id, level_str, embedding) =
                    row.map_err(|e| db_err("Gagal membaca baris lowongan", e))?;
                let skills = skills_by_job.remove(&job_id).unwrap_or_default();
                jobs.push(JobMatchRecord {
                    job_id,
                    experience_level: ExperienceLevel::from_db_str(&level_str),
                    embedding,
                    skills,
                });
            }
            Ok(jobs)
        })
    }

    pub fn load_job_for_matching(&self, job_id: &str) -> Result<Option<JobMatchRecord>, AppError> {
        self.with_conn(|conn| {
            let row: Option<(String, Option<Vec<u8>>)> = conn
                .query_row(
                    "SELECT experience_level, embedding FROM jobs WHERE id = ?1",
                    [job_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|e| db_err("Gagal memuat lowongan untuk matching", e))?;

            let Some((level_str, embedding)) = row else {
                return Ok(None);
            };
            let skills = load_job_skills_conn(conn, job_id)?;
            Ok(Some(JobMatchRecord {
                job_id: job_id.to_string(),
                experience_level: ExperienceLevel::from_db_str(&level_str),
                embedding,
                skills,
            }))
        })
    }

    pub fn list_ranked_jobs(
        &self,
        user_id: &str,
        filter: &JobFilterInput,
    ) -> Result<Vec<JobMatchSummary>, AppError> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT
                    j.id, j.source, j.url, j.title, j.normalized_role, j.company,
                    j.location, j.work_mode, j.experience_level,
                    jm.vector_score, jm.skill_overlap_score, jm.level_score, jm.final_score,
                    jm.matched_skills_json, jm.missing_skills_json,
                    jm.ai_gap_analysis_json, jm.is_saved, j.scraped_at,
                    j.enrichment_status, j.posted_at, j.salary_text,
                    jm.application_status, jm.applied_at,
                    j.domain_slug, j.quality_score, j.red_flags_json, j.also_available_on_json,
                    j.location_cluster, j.salary_min_idr, j.salary_max_idr, j.opportunity_type
                 FROM job_matches jm
                 JOIN jobs j ON j.id = jm.job_id
                 WHERE jm.user_id = ?",
            );
            let mut bind: Vec<Box<dyn ToSql>> = vec![Box::new(user_id.to_string())];

            if let Some(statuses) = filter.application_statuses.as_ref().filter(|v| !v.is_empty()) {
                let placeholders = vec!["?"; statuses.len()].join(", ");
                sql.push_str(&format!(" AND jm.application_status IN ({placeholders})"));
                for st in statuses {
                    bind.push(Box::new(st.as_str().to_string()));
                }
            } else if let Some(app_status) = filter.application_status {
                sql.push_str(" AND jm.application_status = ?");
                bind.push(Box::new(app_status.as_str().to_string()));
            } else if filter.include_dismissed != Some(true) {
                sql.push_str(" AND jm.application_status != 'dismissed'");
            }

            if let Some(role) = filter.normalized_role.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                sql.push_str(" AND j.normalized_role = ?");
                bind.push(Box::new(role.to_string()));
            }
            if let Some(modes) = filter.work_modes.as_ref().filter(|v| !v.is_empty()) {
                let placeholders = vec!["?"; modes.len()].join(", ");
                sql.push_str(&format!(" AND j.work_mode IN ({placeholders})"));
                for wm in modes {
                    bind.push(Box::new(wm.as_str().to_string()));
                }
            } else if let Some(work_mode) = filter.work_mode {
                sql.push_str(" AND j.work_mode = ?");
                bind.push(Box::new(work_mode.as_str().to_string()));
            }
            if let Some(sources) = filter.sources.as_ref().filter(|v| !v.is_empty()) {
                let placeholders = vec!["?"; sources.len()].join(", ");
                sql.push_str(&format!(" AND j.source IN ({placeholders})"));
                for src in sources {
                    bind.push(Box::new(src.as_str().to_string()));
                }
            } else if let Some(source) = filter.source {
                sql.push_str(" AND j.source = ?");
                bind.push(Box::new(source.as_str().to_string()));
            }
            if let Some(clusters) = filter.location_clusters.as_ref().filter(|v| !v.is_empty()) {
                let placeholders = vec!["?"; clusters.len()].join(", ");
                sql.push_str(&format!(" AND j.location_cluster IN ({placeholders})"));
                for cl in clusters {
                    bind.push(Box::new(cl.trim().to_lowercase()));
                }
            }
            if let Some(types) = filter.opportunity_types.as_ref().filter(|v| !v.is_empty()) {
                let placeholders = vec!["?"; types.len()].join(", ");
                sql.push_str(&format!(" AND j.opportunity_type IN ({placeholders})"));
                for ot in types {
                    bind.push(Box::new(ot.trim().to_lowercase()));
                }
            }
            if filter.salary_only == Some(true) {
                sql.push_str(" AND (j.salary_min_idr IS NOT NULL OR TRIM(COALESCE(j.salary_text, '')) != '')");
            }
            if let Some(enrich) = filter.enrichment_status {
                sql.push_str(" AND j.enrichment_status = ?");
                bind.push(Box::new(enrich.as_str().to_string()));
            }
            if filter.saved_only == Some(true) {
                sql.push_str(" AND (jm.is_saved = 1 OR jm.application_status = 'saved')");
            }
            if let Some(min_score) = filter.min_score {
                sql.push_str(" AND jm.final_score >= ?");
                bind.push(Box::new(min_score));
            }
            if let Some(loc_q) = filter.location_query.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                sql.push_str(" AND LOWER(COALESCE(j.location, '')) LIKE ? ESCAPE '\\'");
                let pattern = format!("%{}%", escape_like_pattern(&loc_q.to_lowercase()));
                bind.push(Box::new(pattern));
            }
            if let Some(q) = filter.search_query.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                sql.push_str(
                    " AND (LOWER(j.title) LIKE ? ESCAPE '\\' OR LOWER(j.company) LIKE ? ESCAPE '\\')",
                );
                let pattern = format!("%{}%", escape_like_pattern(&q.to_lowercase()));
                bind.push(Box::new(pattern.clone()));
                bind.push(Box::new(pattern));
            }

            match filter.sort_by.as_deref() {
                Some("newest") => {
                    sql.push_str(" ORDER BY COALESCE(j.posted_at, j.scraped_at) DESC, jm.final_score DESC");
                }
                Some("salary_desc") => {
                    sql.push_str(" ORDER BY COALESCE(j.salary_max_idr, j.salary_min_idr, 0) DESC, jm.final_score DESC");
                }
                _ => {
                    sql.push_str(" ORDER BY jm.final_score DESC, j.scraped_at DESC");
                }
            }

            if let Some(limit) = filter.limit {
                sql.push_str(" LIMIT ?");
                bind.push(Box::new(limit as i64));
                if let Some(offset) = filter.offset {
                    sql.push_str(" OFFSET ?");
                    bind.push(Box::new(offset as i64));
                }
            }

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| db_err("Gagal menyiapkan query daftar lowongan", e))?;
            let rows = stmt
                .query_map(params_from_iter(bind.iter().map(|b| b.as_ref())), |r| {
                    let source_str: String = r.get(1)?;
                    let work_mode_str: String = r.get(7)?;
                    let level_str: String = r.get(8)?;
                    let matched_json: String = r.get(13)?;
                    let missing_json: String = r.get(14)?;
                    let gap_json: Option<String> = r.get(15)?;
                    let is_saved: i64 = r.get(16)?;
                    let enrich_str: String = r.get(18)?;
                    let app_status_str: String = r.get(21)?;
                    let domain_slug: String = r.get(23)?;
                    let quality_score: f64 = r.get(24)?;
                    let red_flags_json: String = r.get(25)?;
                    let also_json: String = r.get(26)?;
                    let location_cluster: String = r.get(27)?;
                    let salary_min_idr: Option<i64> = r.get(28)?;
                    let salary_max_idr: Option<i64> = r.get(29)?;
                    let opportunity_type: String = r.get(30)?;

                    let matched: Vec<JobMatchSkillRef> =
                        serde_json::from_str(&matched_json).unwrap_or_default();
                    let missing: Vec<JobMatchSkillRef> =
                        serde_json::from_str(&missing_json).unwrap_or_default();
                    let red_flags: Vec<JobRedFlag> =
                        serde_json::from_str(&red_flags_json).unwrap_or_default();
                    let also_available_on: Vec<PortalLinkRef> =
                        serde_json::from_str(&also_json).unwrap_or_default();

                    Ok(JobMatchSummary {
                        job_id: r.get(0)?,
                        source: JobSource::from_db_str(&source_str),
                        url: r.get(2)?,
                        title: r.get(3)?,
                        normalized_role: r.get(4)?,
                        company: r.get(5)?,
                        location: r.get(6)?,
                        location_cluster,
                        work_mode: WorkMode::from_db_str(&work_mode_str),
                        experience_level: ExperienceLevel::from_db_str(&level_str),
                        enrichment_status: EnrichmentStatus::from_db_str(&enrich_str),
                        posted_at: r.get(19)?,
                        salary_text: r.get(20)?,
                        salary_min_idr,
                        salary_max_idr,
                        opportunity_type,
                        domain_slug,
                        quality_score,
                        red_flags_count: red_flags.len() as u32,
                        also_available_on,
                        vector_score: r.get(9)?,
                        skill_overlap_score: r.get(10)?,
                        level_score: r.get(11)?,
                        final_score: r.get(12)?,
                        matched_skills_count: matched.len() as u32,
                        missing_skills_count: missing.len() as u32,
                        has_gap_analysis: gap_json.is_some(),
                        is_saved: is_saved != 0,
                        application_status: ApplicationStatus::from_db_str(&app_status_str),
                        applied_at: r.get(22)?,
                        scraped_at: r.get(17)?,
                    })
                })
                .map_err(|e| db_err("Gagal membaca daftar lowongan", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal parsing baris daftar lowongan", e))?);
            }
            Ok(out)
        })
    }

    pub fn get_job_detail(
        &self,
        user_id: &str,
        job_id: &str,
    ) -> Result<JobMatchDetail, AppError> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    "SELECT
                        j.id, j.source, j.external_id, j.url, j.title, j.normalized_role,
                        j.company, j.location, j.work_mode, j.experience_level,
                        j.raw_description, j.extracted_summary_json,
                        jm.vector_score, jm.skill_overlap_score, jm.level_score, jm.final_score,
                        jm.matched_skills_json, jm.missing_skills_json,
                        jm.ai_gap_analysis_json, jm.is_saved, j.scraped_at,
                        j.enrichment_status, j.posted_at, j.salary_text,
                        jm.ai_apply_kit_json, jm.application_status, jm.applied_at, jm.user_notes,
                        j.domain_slug, j.quality_score, j.red_flags_json, j.also_available_on_json,
                        j.location_cluster, j.salary_min_idr, j.salary_max_idr, j.opportunity_type
                     FROM jobs j
                     JOIN job_matches jm ON jm.job_id = j.id AND jm.user_id = ?1
                     WHERE j.id = ?2",
                    params![user_id, job_id],
                    |r| {
                        let source_str: String = r.get(1)?;
                        let work_mode_str: String = r.get(8)?;
                        let level_str: String = r.get(9)?;
                        let extracted_json: Option<String> = r.get(11)?;
                        let matched_json: String = r.get(16)?;
                        let missing_json: String = r.get(17)?;
                        let gap_json: Option<String> = r.get(18)?;
                        let is_saved: i64 = r.get(19)?;
                        let enrich_str: String = r.get(21)?;
                        let apply_kit_json: Option<String> = r.get(24)?;
                        let app_status_str: String = r.get(25)?;
                        let domain_slug: String = r.get(28)?;
                        let quality_score: f64 = r.get(29)?;
                        let red_flags_json: String = r.get(30)?;
                        let also_json: String = r.get(31)?;
                        let location_cluster: String = r.get(32)?;
                        let salary_min_idr: Option<i64> = r.get(33)?;
                        let salary_max_idr: Option<i64> = r.get(34)?;
                        let opportunity_type: String = r.get(35)?;

                        let role_summary = extracted_json.as_deref().and_then(|s| {
                            serde_json::from_str::<serde_json::Value>(s)
                                .ok()
                                .and_then(|v| {
                                    v.get("role_summary")
                                        .and_then(|rs| rs.as_str())
                                        .map(str::to_string)
                                })
                        });
                        let matched_skills: Vec<JobMatchSkillRef> =
                            serde_json::from_str(&matched_json).unwrap_or_default();
                        let missing_skills: Vec<JobMatchSkillRef> =
                            serde_json::from_str(&missing_json).unwrap_or_default();
                        let ai_gap_analysis: Option<GapAnalysis> = gap_json
                            .as_deref()
                            .and_then(|s| serde_json::from_str(s).ok());
                        let ai_apply_kit: Option<ApplyKitResult> = apply_kit_json
                            .as_deref()
                            .and_then(|s| serde_json::from_str(s).ok());
                        let red_flags: Vec<JobRedFlag> =
                            serde_json::from_str(&red_flags_json).unwrap_or_default();
                        let also_available_on: Vec<PortalLinkRef> =
                            serde_json::from_str(&also_json).unwrap_or_default();

                        Ok(JobMatchDetail {
                            job_id: r.get(0)?,
                            source: JobSource::from_db_str(&source_str),
                            external_id: r.get(2)?,
                            url: r.get(3)?,
                            title: r.get(4)?,
                            normalized_role: r.get(5)?,
                            company: r.get(6)?,
                            location: r.get(7)?,
                            location_cluster,
                            work_mode: WorkMode::from_db_str(&work_mode_str),
                            experience_level: ExperienceLevel::from_db_str(&level_str),
                            enrichment_status: EnrichmentStatus::from_db_str(&enrich_str),
                            posted_at: r.get(22)?,
                            salary_text: r.get(23)?,
                            salary_min_idr,
                            salary_max_idr,
                            opportunity_type,
                            domain_slug,
                            quality_score,
                            red_flags,
                            also_available_on,
                            raw_description: r.get(10)?,
                            role_summary,
                            vector_score: r.get(12)?,
                            skill_overlap_score: r.get(13)?,
                            level_score: r.get(14)?,
                            final_score: r.get(15)?,
                            matched_skills,
                            missing_skills,
                            ai_gap_analysis,
                            ai_apply_kit,
                            is_saved: is_saved != 0,
                            application_status: ApplicationStatus::from_db_str(&app_status_str),
                            applied_at: r.get(26)?,
                            user_notes: r.get(27)?,
                            scraped_at: r.get(20)?,
                        })
                    },
                )
                .optional()
                .map_err(|e| db_err("Gagal memuat detail lowongan", e))?;

            row.ok_or_else(|| {
                AppError::validation_error(format!(
                    "Detail lowongan '{job_id}' untuk pengguna '{user_id}' tidak ditemukan."
                ))
            })
        })
    }

    pub fn toggle_save_job(&self, user_id: &str, job_id: &str) -> Result<bool, AppError> {
        self.with_tx(|tx| {
            let updated = tx
                .execute(
                    "UPDATE job_matches
                     SET is_saved = 1 - is_saved,
                         application_status = CASE
                             WHEN is_saved = 0 AND application_status = 'new' THEN 'saved'
                             WHEN is_saved = 1 AND application_status = 'saved' THEN 'new'
                             ELSE application_status
                         END
                     WHERE user_id = ?1 AND job_id = ?2",
                    params![user_id, job_id],
                )
                .map_err(|e| db_err("Gagal toggle simpan lowongan", e))?;
            if updated == 0 {
                return Err(AppError::validation_error(format!(
                    "Lowongan '{job_id}' untuk pengguna '{user_id}' tidak ditemukan."
                )));
            }
            let saved: i64 = tx
                .query_row(
                    "SELECT is_saved FROM job_matches WHERE user_id = ?1 AND job_id = ?2",
                    params![user_id, job_id],
                    |r| r.get(0),
                )
                .map_err(|e| db_err("Gagal membaca status simpan lowongan", e))?;
            Ok(saved != 0)
        })
    }

    pub fn update_job_application(
        &self,
        user_id: &str,
        job_id: &str,
        status: ApplicationStatus,
        notes: Option<&str>,
    ) -> Result<JobMatchDetail, AppError> {
        let clean_notes = notes
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.chars().take(2000).collect::<String>());
        let set_saved: i64 = match status {
            ApplicationStatus::Saved
            | ApplicationStatus::Applied
            | ApplicationStatus::Interview
            | ApplicationStatus::Offer => 1,
            ApplicationStatus::Dismissed | ApplicationStatus::New => 0,
            ApplicationStatus::Rejected => 1,
        };
        let marks_applied = matches!(
            status,
            ApplicationStatus::Applied
                | ApplicationStatus::Interview
                | ApplicationStatus::Offer
        );

        self.with_tx(|tx| {
            let changed = tx
                .execute(
                    "UPDATE job_matches
                     SET application_status = ?1,
                         is_saved = ?2,
                         applied_at = CASE
                             WHEN ?3 = 1 THEN COALESCE(applied_at, CURRENT_TIMESTAMP)
                             WHEN ?1 IN ('new', 'saved', 'dismissed') THEN NULL
                             ELSE applied_at
                         END,
                         user_notes = ?4
                     WHERE user_id = ?5 AND job_id = ?6",
                    params![
                        status.as_str(),
                        set_saved,
                        if marks_applied { 1i64 } else { 0i64 },
                        clean_notes,
                        user_id,
                        job_id,
                    ],
                )
                .map_err(|e| db_err("Gagal memperbarui status lamaran lowongan", e))?;
            if changed == 0 {
                return Err(AppError::validation_error(format!(
                    "Lowongan '{job_id}' untuk pengguna '{user_id}' tidak ditemukan."
                )));
            }
            Ok(())
        })?;

        self.get_job_detail(user_id, job_id)
    }

    pub fn get_cached_apply_kit(
        &self,
        user_id: &str,
        job_id: &str,
    ) -> Result<Option<ApplyKitResult>, AppError> {
        self.with_conn(|conn| {
            let raw: Option<Option<String>> = conn
                .query_row(
                    "SELECT ai_apply_kit_json FROM job_matches WHERE user_id = ?1 AND job_id = ?2",
                    params![user_id, job_id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| db_err("Gagal membaca cache copilot lamaran", e))?;

            let Some(maybe_json) = raw else {
                return Err(AppError::validation_error(format!(
                    "Data matching untuk lowongan '{job_id}' tidak ditemukan."
                )));
            };

            match maybe_json {
                Some(json_str) => Ok(serde_json::from_str::<ApplyKitResult>(&json_str).ok()),
                None => Ok(None),
            }
        })
    }

    pub fn save_apply_kit(
        &self,
        user_id: &str,
        job_id: &str,
        kit: &ApplyKitResult,
    ) -> Result<(), AppError> {
        let json = serde_json::to_string(kit)
            .map_err(|_| AppError::db_error("Gagal serialisasi copilot lamaran."))?;
        self.with_conn(|conn| {
            let changed = conn
                .execute(
                    "UPDATE job_matches SET ai_apply_kit_json = ?1 WHERE user_id = ?2 AND job_id = ?3",
                    params![json, user_id, job_id],
                )
                .map_err(|e| db_err("Gagal menyimpan cache copilot lamaran", e))?;
            if changed == 0 {
                return Err(AppError::validation_error(format!(
                    "Data matching untuk lowongan '{job_id}' tidak ditemukan."
                )));
            }
            Ok(())
        })
    }

    pub fn get_master_cv(&self, user_id: &str) -> Result<MasterCvProfile, AppError> {
        self.with_conn(|conn| {
            let found = conn
                .query_row(
                    "SELECT user_id, full_name, headline, email, phone, location,
                            linkedin_url, portfolio_url, summary,
                            experiences_json, education_json, projects_json,
                            certifications_json, languages_json, updated_at
                     FROM user_cv_profiles
                     WHERE user_id = ?1",
                    [user_id],
                    |r| {
                        let exp_j: String = r.get(9)?;
                        let edu_j: String = r.get(10)?;
                        let proj_j: String = r.get(11)?;
                        let cert_j: String = r.get(12)?;
                        let lang_j: String = r.get(13)?;
                        Ok(MasterCvProfile {
                            user_id: r.get(0)?,
                            full_name: r.get(1)?,
                            headline: r.get(2)?,
                            email: r.get(3)?,
                            phone: r.get(4)?,
                            location: r.get(5)?,
                            linkedin_url: r.get(6)?,
                            portfolio_url: r.get(7)?,
                            summary: r.get(8)?,
                            experiences: serde_json::from_str::<Vec<CvExperienceItem>>(&exp_j)
                                .unwrap_or_default(),
                            education: serde_json::from_str::<Vec<CvEducationItem>>(&edu_j)
                                .unwrap_or_default(),
                            projects: serde_json::from_str::<Vec<CvProjectItem>>(&proj_j)
                                .unwrap_or_default(),
                            certifications: serde_json::from_str::<Vec<String>>(&cert_j)
                                .unwrap_or_default(),
                            languages: serde_json::from_str::<Vec<String>>(&lang_j)
                                .unwrap_or_default(),
                            updated_at: r.get(14)?,
                        })
                    },
                )
                .optional()
                .map_err(|e| db_err("Gagal membaca Master CV lokal", e))?;

            if let Some(cv) = found {
                return Ok(cv);
            }

            // Seed default empty Master CV from active user_profile if not yet saved
            let fallback: Option<(String, Option<String>)> = conn
                .query_row(
                    "SELECT profile_name, summary FROM user_profiles WHERE id = ?1",
                    [user_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|e| db_err("Gagal memuat ringkasan profil untuk Master CV", e))?;

            let (name, summary) = fallback.unwrap_or_else(|| ("Kandidat Vecta".into(), None));
            Ok(MasterCvProfile {
                user_id: user_id.to_string(),
                full_name: name,
                summary: summary.unwrap_or_default(),
                languages: vec!["Bahasa Indonesia".into()],
                ..Default::default()
            })
        })
    }

    pub fn save_master_cv(&self, cv: &MasterCvProfile) -> Result<MasterCvProfile, AppError> {
        let exp_j = serde_json::to_string(&cv.experiences)
            .map_err(|_| AppError::db_error("Gagal serialisasi experiences CV."))?;
        let edu_j = serde_json::to_string(&cv.education)
            .map_err(|_| AppError::db_error("Gagal serialisasi education CV."))?;
        let proj_j = serde_json::to_string(&cv.projects)
            .map_err(|_| AppError::db_error("Gagal serialisasi projects CV."))?;
        let cert_j = serde_json::to_string(&cv.certifications)
            .map_err(|_| AppError::db_error("Gagal serialisasi certifications CV."))?;
        let lang_j = serde_json::to_string(&cv.languages)
            .map_err(|_| AppError::db_error("Gagal serialisasi languages CV."))?;

        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO user_cv_profiles (
                    user_id, full_name, headline, email, phone, location,
                    linkedin_url, portfolio_url, summary,
                    experiences_json, education_json, projects_json,
                    certifications_json, languages_json, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, datetime('now'))
                 ON CONFLICT(user_id) DO UPDATE SET
                    full_name = excluded.full_name,
                    headline = excluded.headline,
                    email = excluded.email,
                    phone = excluded.phone,
                    location = excluded.location,
                    linkedin_url = excluded.linkedin_url,
                    portfolio_url = excluded.portfolio_url,
                    summary = excluded.summary,
                    experiences_json = excluded.experiences_json,
                    education_json = excluded.education_json,
                    projects_json = excluded.projects_json,
                    certifications_json = excluded.certifications_json,
                    languages_json = excluded.languages_json,
                    updated_at = datetime('now')",
                params![
                    cv.user_id,
                    cv.full_name.trim(),
                    cv.headline.trim(),
                    cv.email.trim(),
                    cv.phone.trim(),
                    cv.location.trim(),
                    cv.linkedin_url.trim(),
                    cv.portfolio_url.trim(),
                    cv.summary.trim(),
                    exp_j,
                    edu_j,
                    proj_j,
                    cert_j,
                    lang_j,
                ],
            )
            .map_err(|e| db_err("Gagal menyimpan Master CV lokal", e))?;
            Ok(())
        })?;

        self.get_master_cv(&cv.user_id)
    }

    pub fn get_cached_gap_analysis(
        &self,
        user_id: &str,
        job_id: &str,
    ) -> Result<Option<GapAnalysis>, AppError> {
        self.with_conn(|conn| {
            let raw: Option<Option<String>> = conn
                .query_row(
                    "SELECT ai_gap_analysis_json FROM job_matches WHERE user_id = ?1 AND job_id = ?2",
                    params![user_id, job_id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| db_err("Gagal membaca cache gap analysis", e))?;

            let Some(maybe_json) = raw else {
                return Err(AppError::validation_error(format!(
                    "Data matching untuk lowongan '{job_id}' tidak ditemukan."
                )));
            };

            match maybe_json {
                Some(json_str) => {
                    let parsed: GapAnalysis = serde_json::from_str(&json_str).map_err(|e| {
                        AppError::db_error(format!("Cache gap analysis korup: {e}"))
                    })?;
                    Ok(Some(parsed))
                }
                None => Ok(None),
            }
        })
    }

    pub fn save_gap_analysis(
        &self,
        user_id: &str,
        job_id: &str,
        gap: &GapAnalysis,
    ) -> Result<(), AppError> {
        let json = serde_json::to_string(gap)
            .map_err(|e| AppError::db_error(format!("Gagal serialisasi gap analysis: {e}")))?;
        self.with_conn(|conn| {
            let changed = conn
                .execute(
                    "UPDATE job_matches SET ai_gap_analysis_json = ?1 WHERE user_id = ?2 AND job_id = ?3",
                    params![json, user_id, job_id],
                )
                .map_err(|e| db_err("Gagal menyimpan cache gap analysis", e))?;
            if changed == 0 {
                return Err(AppError::validation_error(format!(
                    "Data matching untuk lowongan '{job_id}' tidak ditemukan."
                )));
            }
            Ok(())
        })
    }

    pub fn list_available_roles(&self) -> Result<Vec<RoleJobCount>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT normalized_role, COUNT(*) AS job_count
                     FROM jobs
                     GROUP BY normalized_role
                     ORDER BY job_count DESC, normalized_role ASC",
                )
                .map_err(|e| db_err("Gagal memuat daftar peran tersedia", e))?;

            let rows = stmt
                .query_map([], |r| {
                    Ok(RoleJobCount {
                        normalized_role: r.get(0)?,
                        job_count: r.get::<_, i64>(1)? as u32,
                    })
                })
                .map_err(|e| db_err("Gagal membaca daftar peran", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal parsing peran", e))?);
            }
            Ok(out)
        })
    }

    pub fn get_competency_map(
        &self,
        user_id: &str,
        target_role: &str,
    ) -> Result<CompetencyMapResponse, AppError> {
        let role = target_role.trim();
        self.with_conn(|conn| {
            let total_jobs: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM jobs WHERE normalized_role = ?1",
                    [role],
                    |r| r.get(0),
                )
                .map_err(|e| db_err("Gagal menghitung jumlah lowongan untuk peran", e))?;

            if total_jobs == 0 {
                return Ok(CompetencyMapResponse {
                    target_role: role.to_string(),
                    total_jobs: 0,
                    is_small_sample: true,
                    skills: Vec::new(),
                });
            }

            // PRD Section 7.3 Query A
            let mut stmt = conn
                .prepare(
                    "SELECT
                        s.id AS skill_id,
                        s.canonical_name,
                        s.category,
                        COUNT(js.job_id) AS demand_count,
                        ROUND(100.0 * COUNT(js.job_id) / (
                            SELECT COUNT(*) FROM jobs WHERE normalized_role = ?1
                        ), 1) AS demand_percentage,
                        CASE WHEN us.skill_id IS NOT NULL THEN 1 ELSE 0 END AS is_owned_by_user,
                        us.proficiency AS user_proficiency
                     FROM job_skills js
                     JOIN jobs j ON j.id = js.job_id
                     JOIN skills s ON s.id = js.skill_id
                     LEFT JOIN user_skills us
                         ON us.skill_id = s.id AND us.user_id = ?2
                     WHERE j.normalized_role = ?1
                     GROUP BY s.id
                     ORDER BY demand_count DESC, s.canonical_name ASC
                     LIMIT 30",
                )
                .map_err(|e| db_err("Gagal menyiapkan query peta kompetensi", e))?;

            let rows = stmt
                .query_map(params![role, user_id], |r| {
                    let cat_str: String = r.get(2)?;
                    let is_owned: i64 = r.get(5)?;
                    let prof_str: Option<String> = r.get(6)?;
                    Ok(CompetencySkillItem {
                        skill_id: r.get(0)?,
                        canonical_name: r.get(1)?,
                        category: SkillCategory::from_db_str(&cat_str),
                        demand_count: r.get::<_, i64>(3)? as u32,
                        demand_percentage: r.get(4)?,
                        is_owned_by_user: is_owned != 0,
                        user_proficiency: prof_str.as_deref().map(Proficiency::from_db_str),
                    })
                })
                .map_err(|e| db_err("Gagal membaca peta kompetensi", e))?;

            let mut skills = Vec::new();
            for row in rows {
                skills.push(row.map_err(|e| db_err("Gagal parsing baris kompetensi", e))?);
            }

            Ok(CompetencyMapResponse {
                target_role: role.to_string(),
                total_jobs: total_jobs as u32,
                is_small_sample: total_jobs < 5,
                skills,
            })
        })
    }

    pub fn get_top_missing_skills_for_role(
        &self,
        user_id: &str,
        target_role: &str,
        limit: usize,
    ) -> Result<Vec<MissingSkillDemand>, AppError> {
        self.with_conn(|conn| {
            // PRD Section 7.3 Query B
            let mut stmt = conn
                .prepare(
                    "SELECT s.id, s.canonical_name, s.category, COUNT(js.job_id) AS demand_count
                     FROM job_skills js
                     JOIN jobs j ON j.id = js.job_id
                     JOIN skills s ON s.id = js.skill_id
                     LEFT JOIN user_skills us ON us.skill_id = s.id AND us.user_id = ?1
                     WHERE j.normalized_role = ?2
                       AND us.skill_id IS NULL
                     GROUP BY s.id
                     ORDER BY demand_count DESC, s.canonical_name ASC
                     LIMIT ?3",
                )
                .map_err(|e| db_err("Gagal menyiapkan query top missing skills", e))?;

            let rows = stmt
                .query_map(params![user_id, target_role.trim(), limit as i64], |r| {
                    let cat_str: String = r.get(2)?;
                    Ok(MissingSkillDemand {
                        skill_id: r.get(0)?,
                        canonical_name: r.get(1)?,
                        category: SkillCategory::from_db_str(&cat_str),
                        demand_count: r.get::<_, i64>(3)? as u32,
                    })
                })
                .map_err(|e| db_err("Gagal membaca top missing skills", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal parsing missing skill", e))?);
            }
            Ok(out)
        })
    }

    pub fn get_missing_skills_for_job(
        &self,
        user_id: &str,
        job_id: &str,
    ) -> Result<Vec<MissingSkillDemand>, AppError> {
        self.with_conn(|conn| {
            let exists: bool = conn
                .query_row("SELECT 1 FROM jobs WHERE id = ?1", [job_id], |_| Ok(true))
                .optional()
                .map_err(|e| db_err("Gagal memeriksa keberadaan lowongan", e))?
                .unwrap_or(false);
            if !exists {
                return Err(AppError::validation_error(format!(
                    "Lowongan '{job_id}' tidak ditemukan."
                )));
            }

            let mut stmt = conn
                .prepare(
                    "SELECT s.id, s.canonical_name, s.category
                     FROM job_skills js
                     JOIN skills s ON s.id = js.skill_id
                     LEFT JOIN user_skills us ON us.skill_id = s.id AND us.user_id = ?1
                     WHERE js.job_id = ?2 AND us.skill_id IS NULL
                     ORDER BY js.weight DESC, s.canonical_name ASC",
                )
                .map_err(|e| db_err("Gagal menyiapkan query missing skills lowongan", e))?;

            let rows = stmt
                .query_map(params![user_id, job_id], |r| {
                    let cat_str: String = r.get(2)?;
                    Ok(MissingSkillDemand {
                        skill_id: r.get(0)?,
                        canonical_name: r.get(1)?,
                        category: SkillCategory::from_db_str(&cat_str),
                        demand_count: 1,
                    })
                })
                .map_err(|e| db_err("Gagal membaca missing skills lowongan", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal parsing missing skill lowongan", e))?);
            }
            Ok(out)
        })
    }

    pub fn insert_roadmap_with_steps(
        &self,
        user_id: &str,
        source_job_id: Option<&str>,
        generated: &GeneratedRoadmap,
    ) -> Result<RoadmapDetail, AppError> {
        let roadmap_id = Uuid::new_v4().to_string();
        self.with_tx(|tx| {
            tx.execute(
                "INSERT INTO learning_roadmaps (
                    id, user_id, target_role, source_job_id, title, total_estimated_weeks
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    roadmap_id,
                    user_id,
                    generated.target_role,
                    source_job_id,
                    generated.title,
                    generated.total_estimated_weeks.max(1),
                ],
            )
            .map_err(|e| db_err("Gagal menyimpan header roadmap", e))?;

            for step in &generated.steps {
                let step_id = Uuid::new_v4().to_string();
                let skill_id = if step.target_skill.trim().is_empty() {
                    None
                } else {
                    Some(Self::upsert_skill(
                        tx,
                        &step.target_skill,
                        SkillCategory::CoreHard,
                        &[],
                    )?)
                };
                let keywords_json = serde_json::to_string(&step.learning_keywords)
                    .map_err(|e| AppError::db_error(format!("Gagal serialisasi keywords: {e}")))?;

                tx.execute(
                    "INSERT INTO roadmap_steps (
                        id, roadmap_id, skill_id, phase_order, phase_name, topic_title,
                        learning_keywords_json, mini_project_idea, estimated_hours, status
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'todo')",
                    params![
                        step_id,
                        roadmap_id,
                        skill_id,
                        step.phase_order,
                        step.phase_name,
                        step.topic_title,
                        keywords_json,
                        step.mini_project_idea,
                        step.estimated_hours.max(1),
                    ],
                )
                .map_err(|e| db_err("Gagal menyimpan langkah roadmap", e))?;
            }

            load_roadmap_detail_conn(tx, &roadmap_id)
        })
    }

    pub fn list_roadmaps(&self, user_id: &str) -> Result<Vec<RoadmapSummary>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT
                        lr.id, lr.user_id, lr.target_role, lr.source_job_id, lr.title,
                        lr.total_estimated_weeks, lr.created_at,
                        COUNT(rs.id) AS total_steps,
                        COALESCE(SUM(CASE WHEN rs.status = 'done' THEN 1 ELSE 0 END), 0) AS completed_steps
                     FROM learning_roadmaps lr
                     LEFT JOIN roadmap_steps rs ON rs.roadmap_id = lr.id
                     WHERE lr.user_id = ?1
                     GROUP BY lr.id
                     ORDER BY lr.created_at DESC, lr.rowid DESC",
                )
                .map_err(|e| db_err("Gagal memuat daftar roadmap", e))?;

            let rows = stmt
                .query_map([user_id], |r| {
                    let total_steps = r.get::<_, i64>(7)? as u32;
                    let completed_steps = r.get::<_, i64>(8)? as u32;
                    let progress_percentage = if total_steps == 0 {
                        0.0
                    } else {
                        ((completed_steps as f64 / total_steps as f64) * 1000.0).round() / 10.0
                    };
                    Ok(RoadmapSummary {
                        id: r.get(0)?,
                        user_id: r.get(1)?,
                        target_role: r.get(2)?,
                        source_job_id: r.get(3)?,
                        title: r.get(4)?,
                        total_estimated_weeks: r.get(5)?,
                        created_at: r.get(6)?,
                        total_steps,
                        completed_steps,
                        progress_percentage,
                    })
                })
                .map_err(|e| db_err("Gagal membaca daftar roadmap", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal parsing roadmap summary", e))?);
            }
            Ok(out)
        })
    }

    pub fn get_roadmap_detail(&self, roadmap_id: &str) -> Result<RoadmapDetail, AppError> {
        self.with_conn(|conn| load_roadmap_detail_conn(conn, roadmap_id))
    }

    pub fn update_roadmap_step_status(
        &self,
        step_id: &str,
        status: StepStatus,
    ) -> Result<StepUpdateResult, AppError> {
        self.with_tx(|tx| {
            let changed = tx
                .execute(
                    "UPDATE roadmap_steps
                     SET status = ?1,
                         completed_at = CASE WHEN ?1 = 'done' THEN CURRENT_TIMESTAMP ELSE NULL END
                     WHERE id = ?2",
                    params![status.as_str(), step_id],
                )
                .map_err(|e| db_err("Gagal memperbarui status langkah roadmap", e))?;

            if changed == 0 {
                return Err(AppError::validation_error(format!(
                    "Langkah roadmap '{step_id}' tidak ditemukan."
                )));
            }

            let (user_id, skill_id, completed_at): (String, Option<i64>, Option<String>) = tx
                .query_row(
                    "SELECT lr.user_id, rs.skill_id, rs.completed_at
                     FROM roadmap_steps rs
                     JOIN learning_roadmaps lr ON lr.id = rs.roadmap_id
                     WHERE rs.id = ?1",
                    [step_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .map_err(|e| db_err("Gagal membaca ulang langkah roadmap", e))?;

            let suggest_add_skill = if status == StepStatus::Done {
                if let Some(sid) = skill_id {
                    let already_owned: bool = tx
                        .query_row(
                            "SELECT 1 FROM user_skills WHERE user_id = ?1 AND skill_id = ?2",
                            params![user_id, sid],
                            |_| Ok(true),
                        )
                        .optional()
                        .map_err(|e| db_err("Gagal memeriksa kepemilikan skill", e))?
                        .unwrap_or(false);

                    if !already_owned {
                        let (canonical_name, cat_str): (String, String) = tx
                            .query_row(
                                "SELECT canonical_name, category FROM skills WHERE id = ?1",
                                [sid],
                                |r| Ok((r.get(0)?, r.get(1)?)),
                            )
                            .map_err(|e| db_err("Gagal memuat detail skill saran", e))?;
                        Some(SkillRef {
                            skill_id: sid,
                            canonical_name,
                            category: SkillCategory::from_db_str(&cat_str),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            Ok(StepUpdateResult {
                step_id: step_id.to_string(),
                status,
                completed_at,
                suggest_add_skill,
            })
        })
    }

    pub fn quick_add_user_skill_row(&self, user_id: &str, skill_id: i64) -> Result<(), AppError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO user_skills (user_id, skill_id, proficiency, evidence)
                 VALUES (?1, ?2, 'beginner', 'Selesai dipelajari melalui Vecta Roadmap')
                 ON CONFLICT(user_id, skill_id) DO NOTHING",
                params![user_id, skill_id],
            )
            .map_err(|e| db_err("Gagal menambahkan skill pengguna", e))?;
            Ok(())
        })
    }
}

fn normalize_alias_list(aliases: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for alias in aliases {
        let t = alias.trim();
        if !t.is_empty() && seen.insert(t.to_lowercase()) {
            out.push(t.to_string());
        }
    }
    out
}

fn merge_skill_aliases(
    tx: &Transaction<'_>,
    skill_id: i64,
    existing_json: &str,
    new_aliases: &[String],
) -> Result<(), AppError> {
    if new_aliases.is_empty() {
        return Ok(());
    }
    let mut merged: Vec<String> = serde_json::from_str(existing_json).unwrap_or_default();
    let mut seen: HashSet<String> = merged.iter().map(|s| s.to_lowercase()).collect();
    let mut changed = false;
    for alias in new_aliases {
        let t = alias.trim();
        if !t.is_empty() && seen.insert(t.to_lowercase()) {
            merged.push(t.to_string());
            changed = true;
        }
    }
    if changed {
        let json = serde_json::to_string(&merged)
            .map_err(|e| AppError::db_error(format!("Gagal serialisasi aliases: {e}")))?;
        tx.execute(
            "UPDATE skills SET aliases_json = ?1 WHERE id = ?2",
            params![json, skill_id],
        )
        .map_err(|e| db_err("Gagal memperbarui alias skill", e))?;
    }
    Ok(())
}

fn load_profile_by_id_conn(
    conn: &Connection,
    user_id: &str,
) -> Result<Option<UserProfileDetail>, AppError> {
    let row = conn
        .query_row(
            "SELECT
                id, profile_name, target_roles_json, raw_story_text, summary,
                experience_level, embedding IS NOT NULL, is_active, created_at, updated_at
             FROM user_profiles
             WHERE id = ?1",
            [user_id],
            |r| {
                let target_roles_json: String = r.get(2)?;
                let level_str: String = r.get(5)?;
                let has_emb: i64 = r.get(6)?;
                let is_active: i64 = r.get(7)?;
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    serde_json::from_str::<Vec<String>>(&target_roles_json).unwrap_or_default(),
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    ExperienceLevel::from_db_str(&level_str),
                    has_emb != 0,
                    is_active != 0,
                    r.get::<_, String>(8)?,
                    r.get::<_, String>(9)?,
                ))
            },
        )
        .optional()
        .map_err(|e| db_err("Gagal memuat profil pengguna", e))?;

    let Some((
        id,
        profile_name,
        target_roles,
        raw_story_text,
        summary,
        experience_level,
        has_embedding,
        is_active,
        created_at,
        updated_at,
    )) = row
    else {
        return Ok(None);
    };

    let mut skill_stmt = conn
        .prepare(
            "SELECT us.skill_id, s.canonical_name, s.category, us.proficiency, us.evidence
             FROM user_skills us
             JOIN skills s ON s.id = us.skill_id
             WHERE us.user_id = ?1
             ORDER BY s.canonical_name ASC",
        )
        .map_err(|e| db_err("Gagal menyiapkan query user_skills", e))?;
    let skill_rows = skill_stmt
        .query_map([&id], |r| {
            let cat_str: String = r.get(2)?;
            let prof_str: String = r.get(3)?;
            Ok(UserSkillItem {
                skill_id: r.get(0)?,
                canonical_name: r.get(1)?,
                category: SkillCategory::from_db_str(&cat_str),
                proficiency: Proficiency::from_db_str(&prof_str),
                evidence: r.get(4)?,
            })
        })
        .map_err(|e| db_err("Gagal membaca user_skills", e))?;

    let mut skills = Vec::new();
    for s in skill_rows {
        skills.push(s.map_err(|e| db_err("Gagal parsing user_skill", e))?);
    }

    let mut kw_stmt = conn
        .prepare(
            "SELECT id, user_id, role_label, fit_type, scrape_queries_json, match_reason, is_active, last_scraped_at
             FROM user_scrape_keywords
             WHERE user_id = ?1
             ORDER BY id ASC",
        )
        .map_err(|e| db_err("Gagal menyiapkan query keywords", e))?;
    let kw_rows = kw_stmt
        .query_map([&id], |r| {
            let fit_str: String = r.get(3)?;
            let queries_json: String = r.get(4)?;
            let active_int: i64 = r.get(6)?;
            Ok(ScrapeKeywordItem {
                id: r.get(0)?,
                user_id: r.get(1)?,
                role_label: r.get(2)?,
                fit_type: FitType::from_db_str(&fit_str),
                scrape_queries: serde_json::from_str(&queries_json).unwrap_or_default(),
                match_reason: r.get(5)?,
                is_active: active_int != 0,
                last_scraped_at: r.get(7)?,
            })
        })
        .map_err(|e| db_err("Gagal membaca keywords", e))?;

    let mut keywords = Vec::new();
    for kw in kw_rows {
        keywords.push(kw.map_err(|e| db_err("Gagal parsing keyword", e))?);
    }

    Ok(Some(UserProfileDetail {
        id,
        profile_name,
        target_roles,
        raw_story_text,
        summary,
        experience_level,
        has_embedding,
        is_active,
        skills,
        keywords,
        created_at,
        updated_at,
    }))
}

fn load_job_skills_conn(conn: &Connection, job_id: &str) -> Result<Vec<JobSkillRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT js.skill_id, s.canonical_name, s.category, js.requirement_type, js.weight
             FROM job_skills js
             JOIN skills s ON s.id = js.skill_id
             WHERE js.job_id = ?1
             ORDER BY js.weight DESC, s.canonical_name ASC",
        )
        .map_err(|e| db_err("Gagal menyiapkan query job_skills", e))?;

    let rows = stmt
        .query_map([job_id], |r| {
            let cat_str: String = r.get(2)?;
            let req_str: String = r.get(3)?;
            Ok(JobSkillRow {
                skill_id: r.get(0)?,
                canonical_name: r.get(1)?,
                category: SkillCategory::from_db_str(&cat_str),
                requirement_type: RequirementType::from_db_str(&req_str),
                weight: r.get(4)?,
            })
        })
        .map_err(|e| db_err("Gagal membaca job_skills", e))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err("Gagal parsing job_skill", e))?);
    }
    Ok(out)
}

fn load_roadmap_detail_conn(
    conn: &Connection,
    roadmap_id: &str,
) -> Result<RoadmapDetail, AppError> {
    let header = conn
        .query_row(
            "SELECT id, user_id, target_role, source_job_id, title, total_estimated_weeks, created_at
             FROM learning_roadmaps
             WHERE id = ?1",
            [roadmap_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i32>(5)?,
                    r.get::<_, String>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|e| db_err("Gagal membaca detail roadmap", e))?;

    let Some((id, user_id, target_role, source_job_id, title, total_estimated_weeks, created_at)) =
        header
    else {
        return Err(AppError::validation_error(format!(
            "Roadmap '{roadmap_id}' tidak ditemukan."
        )));
    };

    let mut stmt = conn
        .prepare(
            "SELECT
                rs.id, rs.roadmap_id, rs.skill_id, s.canonical_name,
                rs.phase_order, rs.phase_name, rs.topic_title,
                rs.learning_keywords_json, rs.mini_project_idea,
                rs.estimated_hours, rs.status, rs.completed_at
             FROM roadmap_steps rs
             LEFT JOIN skills s ON s.id = rs.skill_id
             WHERE rs.roadmap_id = ?1
             ORDER BY rs.phase_order ASC, rs.rowid ASC",
        )
        .map_err(|e| db_err("Gagal menyiapkan query langkah roadmap", e))?;

    let rows = stmt
        .query_map([&id], |r| {
            let keywords_json: String = r.get(7)?;
            let status_str: String = r.get(10)?;
            Ok(RoadmapStepItem {
                id: r.get(0)?,
                roadmap_id: r.get(1)?,
                skill_id: r.get(2)?,
                skill_name: r.get(3)?,
                phase_order: r.get(4)?,
                phase_name: r.get(5)?,
                topic_title: r.get(6)?,
                learning_keywords: serde_json::from_str(&keywords_json).unwrap_or_default(),
                mini_project_idea: r.get(8)?,
                estimated_hours: r.get(9)?,
                status: StepStatus::from_db_str(&status_str),
                completed_at: r.get(11)?,
            })
        })
        .map_err(|e| db_err("Gagal membaca langkah roadmap", e))?;

    let mut steps = Vec::new();
    for row in rows {
        steps.push(row.map_err(|e| db_err("Gagal parsing langkah roadmap", e))?);
    }

    Ok(RoadmapDetail {
        id,
        user_id,
        target_role,
        source_job_id,
        title,
        total_estimated_weeks,
        steps,
        created_at,
    })
}

impl Database {
    pub fn list_saved_watchlists(&self, user_id: &str) -> Result<Vec<SavedWatchlistItem>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, user_id, name, filter_json, created_at
                     FROM saved_watchlists
                     WHERE user_id = ?1
                     ORDER BY created_at DESC",
                )
                .map_err(|e| db_err("Gagal menyiapkan query watchlist", e))?;
            let rows = stmt
                .query_map([user_id], |r| {
                    let filter_json: String = r.get(3)?;
                    let filter: JobFilterInput =
                        serde_json::from_str(&filter_json).unwrap_or_default();
                    Ok(SavedWatchlistItem {
                        id: r.get(0)?,
                        user_id: r.get(1)?,
                        name: r.get(2)?,
                        filter,
                        created_at: r.get(4)?,
                    })
                })
                .map_err(|e| db_err("Gagal membaca daftar watchlist", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal parsing baris watchlist", e))?);
            }
            Ok(out)
        })
    }

    pub fn save_watchlist(
        &self,
        user_id: &str,
        name: &str,
        filter: &JobFilterInput,
    ) -> Result<SavedWatchlistItem, AppError> {
        let clean_name = name.trim();
        if clean_name.is_empty() {
            return Err(AppError::validation_error("Nama watchlist tidak boleh kosong."));
        }
        let id = Uuid::new_v4().to_string();
        let filter_json = serde_json::to_string(filter)
            .map_err(|_| AppError::validation_error("Filter watchlist tidak valid."))?;

        self.with_tx(|tx| {
            tx.execute(
                "INSERT INTO saved_watchlists (id, user_id, name, filter_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, user_id, clean_name, filter_json],
            )
            .map_err(|e| db_err("Gagal menyimpan watchlist", e))?;

            let created_at: String = tx
                .query_row(
                    "SELECT created_at FROM saved_watchlists WHERE id = ?1",
                    [&id],
                    |r| r.get(0),
                )
                .unwrap_or_default();

            Ok(SavedWatchlistItem {
                id,
                user_id: user_id.to_string(),
                name: clean_name.to_string(),
                filter: filter.clone(),
                created_at,
            })
        })
    }

    pub fn delete_watchlist(&self, id: &str) -> Result<(), AppError> {
        self.with_tx(|tx| {
            tx.execute("DELETE FROM saved_watchlists WHERE id = ?1", [id])
                .map_err(|e| db_err("Gagal menghapus watchlist", e))?;
            Ok(())
        })
    }

    pub fn simulate_skill_impact(
        &self,
        user_id: &str,
        target_role: &str,
        candidate_skill_ids: &[i64],
    ) -> Result<Vec<SkillImpactItem>, AppError> {
        if candidate_skill_ids.is_empty() {
            return Ok(Vec::new());
        }
        let profile = self
            .get_profile_for_matching(user_id)?
            .ok_or_else(|| AppError::validation_error("Profil pengguna tidak ditemukan."))?;

        let all_jobs = self.load_all_jobs_for_matching()?;
        let role_filter = target_role.trim();

        let (scoped_jobs, baseline_scores): (Vec<JobMatchRecord>, std::collections::HashMap<String, f64>) =
            self.with_conn(|conn| {
                let mut role_job_ids: Option<HashSet<String>> = None;
                if !role_filter.is_empty() && !role_filter.eq_ignore_ascii_case("semua") {
                    let mut stmt = conn
                        .prepare("SELECT id FROM jobs WHERE normalized_role = ?1")
                        .map_err(|e| db_err("Gagal memfilter lowongan simulator", e))?;
                    let rows = stmt
                        .query_map([role_filter], |r| r.get::<_, String>(0))
                        .map_err(|e| db_err("Gagal membaca lowongan simulator", e))?;
                    let mut set = HashSet::new();
                    for r in rows {
                        if let Ok(id) = r {
                            set.insert(id);
                        }
                    }
                    role_job_ids = Some(set);
                }

                let mut base_stmt = conn
                    .prepare("SELECT job_id, final_score FROM job_matches WHERE user_id = ?1")
                    .map_err(|e| db_err("Gagal membaca baseline skor", e))?;
                let base_rows = base_stmt
                    .query_map([user_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))
                    .map_err(|e| db_err("Gagal iterasi baseline skor", e))?;
                let mut scores = std::collections::HashMap::new();
                for row in base_rows.flatten() {
                    scores.insert(row.0, row.1);
                }

                let scoped: Vec<JobMatchRecord> = all_jobs
                    .into_iter()
                    .filter(|j| {
                        role_job_ids
                            .as_ref()
                            .map(|s| s.contains(&j.job_id))
                            .unwrap_or(true)
                    })
                    .collect();
                Ok((scoped, scores))
            })?;

        let mut results = Vec::new();
        for &cand_id in candidate_skill_ids.iter().take(10) {
            let cand_name: Option<String> = self.with_conn(|conn| {
                conn.query_row(
                    "SELECT canonical_name FROM skills WHERE id = ?1",
                    [cand_id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| db_err("Gagal membaca nama skill simulator", e))
            })?;
            let Some(canonical_name) = cand_name else {
                continue;
            };

            let mut sim_profile = profile.clone();
            sim_profile.skill_ids.insert(cand_id);
            sim_profile
                .skill_names_lower
                .insert(canonical_name.to_lowercase());

            let mut affected_jobs = 0u32;
            let mut delta_sum = 0.0f64;
            let mut crossing_75 = 0u32;

            for job in &scoped_jobs {
                let base_score = baseline_scores
                    .get(&job.job_id)
                    .copied()
                    .map(|v| if v > 1.0 { v / 100.0 } else { v })
                    .unwrap_or_else(|| compute_hybrid_match(&profile, job).final_score);
                let sim_score = compute_hybrid_match(&sim_profile, job).final_score;
                let delta = (sim_score - base_score).max(0.0);
                if delta > 0.005 {
                    affected_jobs += 1;
                    delta_sum += delta * 100.0;
                    if base_score < 0.75 && sim_score >= 0.75 {
                        crossing_75 += 1;
                    }
                }
            }

            results.push(SkillImpactItem {
                skill_id: cand_id,
                canonical_name,
                affected_jobs_count: affected_jobs,
                avg_score_delta: if affected_jobs > 0 {
                    (delta_sum / affected_jobs as f64 * 10.0).round() / 10.0
                } else {
                    0.0
                },
                jobs_crossing_75_count: crossing_75,
            });
        }

        Ok(results)
    }

    pub fn export_backup_json(&self) -> Result<VectaBackupReport, AppError> {
        #[derive(serde::Serialize)]
        struct BackupPayload {
            schema_version: u32,
            exported_at: String,
            active_profile: Option<UserProfileDetail>,
            master_cv: Option<MasterCvProfile>,
            search_preferences_json: Option<String>,
            watchlists: Vec<SavedWatchlistItem>,
        }

        let active_profile = self.get_active_profile()?;
        let (master_cv, watchlists, profiles_count, tracked_jobs_count) =
            if let Some(ref p) = active_profile {
                let cv = self.get_master_cv(&p.id).ok();
                let wl = self.list_saved_watchlists(&p.id)?;
                let tracked: i64 = self.with_conn(|conn| {
                    conn.query_row(
                        "SELECT COUNT(*) FROM job_matches WHERE user_id = ?1 AND application_status != 'new'",
                        [&p.id],
                        |r| r.get(0),
                    )
                    .map_err(|e| db_err("Gagal menghitung lamaran aktif backup", e))
                })?;
                (cv, wl, 1u32, tracked as u32)
            } else {
                (None, Vec::new(), 0u32, 0u32)
            };

        let search_preferences_json = self.get_app_setting("search_preferences")?;
        let exported_at: String = self.with_conn(|conn| {
            conn.query_row("SELECT datetime('now')", [], |r| r.get(0))
                .map_err(|e| db_err("Gagal membaca timestamp backup", e))
        })?;
        let watchlists_count = watchlists.len() as u32;

        let payload = BackupPayload {
            schema_version: 1,
            exported_at: exported_at.clone(),
            active_profile,
            master_cv,
            search_preferences_json,
            watchlists,
        };
        let payload_json = serde_json::to_string_pretty(&payload)
            .map_err(|e| AppError::db_error(format!("Gagal serialisasi backup Vecta: {e}")))?;

        Ok(VectaBackupReport {
            schema_version: 1,
            exported_at,
            file_name: "vecta-backup.vecta".into(),
            payload_json,
            profiles_count,
            watchlists_count,
            tracked_jobs_count,
        })
    }

    pub fn import_backup_json(&self, backup_json: &str) -> Result<VectaImportReport, AppError> {
        #[derive(serde::Deserialize)]
        struct BackupPayloadIn {
            #[serde(default)]
            schema_version: u32,
            active_profile: Option<UserProfileDetail>,
            master_cv: Option<MasterCvProfile>,
            search_preferences_json: Option<String>,
            #[serde(default)]
            watchlists: Vec<SavedWatchlistItem>,
        }

        let parsed: BackupPayloadIn = serde_json::from_str(backup_json)
            .map_err(|e| AppError::validation_error(format!("Format file .vecta tidak valid: {e}")))?;
        if parsed.schema_version == 0 {
            return Err(AppError::validation_error(
                "Versi skema backup .vecta tidak dikenali.",
            ));
        }

        let mut imported_profiles = 0u32;
        let mut imported_watchlists = 0u32;
        let mut imported_preferences = false;
        let mut warnings = Vec::new();
        let mut restored_user_id: Option<String> = None;

        if let Some(prof) = parsed.active_profile {
            let input = ConfirmedProfileInput {
                id: Some(prof.id.clone()),
                profile_name: prof.profile_name,
                target_roles: prof.target_roles,
                raw_story_text: prof.raw_story_text.unwrap_or_default(),
                summary: prof.summary.unwrap_or_default(),
                experience_level: prof.experience_level,
                skills: prof
                    .skills
                    .into_iter()
                    .map(|s| ConfirmedSkillInput {
                        canonical_name: s.canonical_name,
                        category: s.category,
                        proficiency: s.proficiency,
                        evidence: s.evidence,
                    })
                    .collect(),
                keywords: prof
                    .keywords
                    .into_iter()
                    .map(|k| ConfirmedKeywordInput {
                        role_label: k.role_label,
                        fit_type: k.fit_type,
                        scrape_queries: k.scrape_queries,
                        match_reason: k.match_reason,
                        is_active: k.is_active,
                    })
                    .collect(),
            };
            let saved = self.save_profile_atomic(&input, None)?;
            restored_user_id = Some(saved.id);
            imported_profiles += 1;
        } else {
            warnings.push("Backup tidak berisi profil aktif.".into());
        }

        if let Some(mut cv) = parsed.master_cv {
            if let Some(ref uid) = restored_user_id {
                cv.user_id = uid.clone();
            }
            self.save_master_cv(&cv)?;
        }

        if let Some(prefs_json) = parsed.search_preferences_json {
            self.set_app_setting("search_preferences", &prefs_json)?;
            imported_preferences = true;
        }

        if let Some(ref uid) = restored_user_id {
            for wl in parsed.watchlists {
                if self.save_watchlist(uid, &wl.name, &wl.filter).is_ok() {
                    imported_watchlists += 1;
                }
            }
        }

        Ok(VectaImportReport {
            imported_profiles,
            imported_watchlists,
            imported_preferences,
            warnings,
        })
    }

    pub fn get_salary_benchmark(
        &self,
        role_filter: Option<&str>,
        cluster_filter: Option<&str>,
    ) -> Result<SalaryBenchmarkSummary, AppError> {
        let clean_role = role_filter.map(str::trim).filter(|s| !s.is_empty());
        let clean_cluster = cluster_filter.map(str::trim).filter(|s| !s.is_empty());

        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT location_cluster, salary_min_idr, salary_max_idr
                 FROM jobs
                 WHERE (salary_min_idr IS NOT NULL OR salary_max_idr IS NOT NULL)",
            );
            let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();
            let mut p_idx = 1usize;

            if let Some(role) = clean_role {
                sql.push_str(&format!(" AND lower(normalized_role) = lower(?{p_idx})"));
                params_vec.push(Box::new(role.to_string()));
                p_idx += 1;
            }
            if let Some(cluster) = clean_cluster {
                sql.push_str(&format!(" AND location_cluster = ?{p_idx}"));
                params_vec.push(Box::new(cluster.to_string()));
            }

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| db_err("Gagal menyiapkan query benchmark gaji", e))?;
            let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
            let rows = stmt
                .query_map(param_refs.as_slice(), |r| {
                    Ok(SalarySampleRow {
                        location_cluster: r
                            .get::<_, Option<String>>(0)?
                            .unwrap_or_else(|| "indonesia".into()),
                        salary_min_idr: r.get(1)?,
                        salary_max_idr: r.get(2)?,
                    })
                })
                .map_err(|e| db_err("Gagal membaca sampel gaji", e))?;

            let mut samples = Vec::new();
            for row in rows {
                samples.push(row.map_err(|e| db_err("Gagal parsing sampel gaji", e))?);
            }
            Ok(compute_salary_benchmark(clean_role, clean_cluster, &samples))
        })
    }

    pub fn get_company_snapshot(
        &self,
        user_id: &str,
        company_name: &str,
    ) -> Result<CompanySnapshot, AppError> {
        let needle = company_name.trim();
        self.with_conn(|conn| {
            let mut user_stmt = conn
                .prepare(
                    "SELECT s.canonical_name
                     FROM user_skills us
                     JOIN skills s ON s.id = us.skill_id
                     WHERE us.user_id = ?1",
                )
                .map_err(|e| db_err("Gagal membaca skill pengguna", e))?;
            let user_skill_rows = user_stmt
                .query_map([user_id], |r| r.get::<_, String>(0))
                .map_err(|e| db_err("Gagal iterasi skill pengguna", e))?;
            let mut user_skills = Vec::new();
            for r in user_skill_rows {
                user_skills.push(r.map_err(|e| db_err("Gagal parsing skill", e))?);
            }

            let mut job_stmt = conn
                .prepare(
                    "SELECT
                        j.id, j.title, j.work_mode, j.salary_min_idr, j.salary_max_idr,
                        j.quality_score, COALESCE(jm.final_score, 0.0)
                     FROM jobs j
                     LEFT JOIN job_matches jm ON jm.job_id = j.id AND jm.user_id = ?1
                     WHERE lower(j.company) = lower(?2)
                     ORDER BY j.scraped_at DESC
                     LIMIT 60",
                )
                .map_err(|e| db_err("Gagal menyiapkan snapshot perusahaan", e))?;
            let job_rows = job_stmt
                .query_map(params![user_id, needle], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, Option<i64>>(3)?,
                        r.get::<_, Option<i64>>(4)?,
                        r.get::<_, f64>(5)?,
                        r.get::<_, f64>(6)?,
                    ))
                })
                .map_err(|e| db_err("Gagal membaca lowongan perusahaan", e))?;

            let mut skill_stmt = conn
                .prepare(
                    "SELECT s.canonical_name
                     FROM job_skills js
                     JOIN skills s ON s.id = js.skill_id
                     WHERE js.job_id = ?1",
                )
                .map_err(|e| db_err("Gagal menyiapkan skill perusahaan", e))?;

            let mut records = Vec::new();
            for row in job_rows {
                let (jid, title, mode_str, smin, smax, qscore, fscore) =
                    row.map_err(|e| db_err("Gagal parsing lowongan perusahaan", e))?;
                let req_rows = skill_stmt
                    .query_map([&jid], |r| r.get::<_, String>(0))
                    .map_err(|e| db_err("Gagal membaca skill lowongan", e))?;
                let mut required_skills = Vec::new();
                for sr in req_rows {
                    required_skills.push(sr.map_err(|e| db_err("Gagal parsing skill", e))?);
                }
                records.push(CompanyJobRow {
                    title,
                    work_mode: WorkMode::from_db_str(&mode_str),
                    salary_min_idr: smin,
                    salary_max_idr: smax,
                    quality_score: qscore,
                    final_score: fscore,
                    required_skills,
                });
            }

            Ok(build_company_snapshot(needle, &records, &user_skills))
        })
    }

    pub fn list_user_profiles(&self) -> Result<Vec<ProfileSummaryItem>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT
                        p.id, p.profile_name, p.target_roles_json, p.experience_level,
                        p.is_active, p.updated_at, COUNT(us.skill_id) AS skill_count
                     FROM user_profiles p
                     LEFT JOIN user_skills us ON us.user_id = p.id
                     GROUP BY p.id
                     ORDER BY p.is_active DESC, p.updated_at DESC",
                )
                .map_err(|e| db_err("Gagal menyiapkan daftar profil", e))?;

            let rows = stmt
                .query_map([], |r| {
                    let roles_json: String = r.get(2)?;
                    let target_roles: Vec<String> =
                        serde_json::from_str(&roles_json).unwrap_or_default();
                    let lvl_str: String = r.get(3)?;
                    Ok(ProfileSummaryItem {
                        id: r.get(0)?,
                        profile_name: r.get(1)?,
                        target_roles,
                        experience_level: ExperienceLevel::from_db_str(&lvl_str),
                        skill_count: r.get(6)?,
                        is_active: r.get::<_, i64>(4)? != 0,
                        updated_at: r.get(5)?,
                    })
                })
                .map_err(|e| db_err("Gagal membaca daftar profil", e))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| db_err("Gagal parsing profil summary", e))?);
            }
            Ok(out)
        })
    }

    pub fn switch_active_profile(&self, profile_id: &str) -> Result<UserProfileDetail, AppError> {
        self.with_tx(|tx| {
            let exists: bool = tx
                .query_row(
                    "SELECT 1 FROM user_profiles WHERE id = ?1",
                    [profile_id],
                    |_| Ok(true),
                )
                .optional()
                .map_err(|e| db_err("Gagal memeriksa profil tujuan", e))?
                .unwrap_or(false);
            if !exists {
                return Err(AppError::validation_error(format!(
                    "Profil '{profile_id}' tidak ditemukan."
                )));
            }

            tx.execute("UPDATE user_profiles SET is_active = 0", [])
                .map_err(|e| db_err("Gagal menonaktifkan profil lain", e))?;
            tx.execute(
                "UPDATE user_profiles SET is_active = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                [profile_id],
            )
            .map_err(|e| db_err("Gagal mengaktifkan profil terpilih", e))?;

            load_profile_by_id_conn(tx, profile_id)?
                .ok_or_else(|| AppError::db_error("Profil tidak ditemukan setelah diaktifkan"))
        })
    }

    pub fn delete_user_profile(&self, profile_id: &str) -> Result<UserProfileDetail, AppError> {
        self.with_tx(|tx| {
            let total: i64 = tx
                .query_row("SELECT COUNT(*) FROM user_profiles", [], |r| r.get(0))
                .map_err(|e| db_err("Gagal menghitung jumlah profil", e))?;
            if total <= 1 {
                return Err(AppError::validation_error(
                    "Minimal harus tersisa satu profil aktif.",
                ));
            }

            let was_active: Option<i64> = tx
                .query_row(
                    "SELECT is_active FROM user_profiles WHERE id = ?1",
                    [profile_id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| db_err("Gagal memeriksa status profil", e))?;

            let Some(active_flag) = was_active else {
                return Err(AppError::validation_error(format!(
                    "Profil '{profile_id}' tidak ditemukan."
                )));
            };

            tx.execute("DELETE FROM user_profiles WHERE id = ?1", [profile_id])
                .map_err(|e| db_err("Gagal menghapus profil", e))?;

            let next_active_id: String = if active_flag != 0 {
                let fallback_id: String = tx
                    .query_row(
                        "SELECT id FROM user_profiles ORDER BY updated_at DESC LIMIT 1",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(|e| db_err("Gagal mencari profil pengganti", e))?;
                tx.execute(
                    "UPDATE user_profiles SET is_active = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                    [&fallback_id],
                )
                .map_err(|e| db_err("Gagal mengaktifkan profil pengganti", e))?;
                fallback_id
            } else {
                tx.query_row(
                    "SELECT id FROM user_profiles ORDER BY is_active DESC, updated_at DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| db_err("Gagal membaca profil aktif tersisa", e))?
            };

            load_profile_by_id_conn(tx, &next_active_id)?
                .ok_or_else(|| AppError::db_error("Profil pengganti gagal dimuat"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::job::{GapSeverity, PrioritizedGap};
    use crate::models::profile::{ConfirmedKeywordInput, ConfirmedSkillInput};
    use crate::models::roadmap::GeneratedRoadmapStep;

    fn sample_profile_input() -> ConfirmedProfileInput {
        ConfirmedProfileInput {
            id: Some("user-1".into()),
            profile_name: "Primary".into(),
            target_roles: vec!["Backend Engineer".into()],
            raw_story_text: "I build Rust APIs and SQLite apps for local-first desktop products."
                .into(),
            summary: "Rust backend developer focused on local-first systems.".into(),
            experience_level: ExperienceLevel::Junior,
            skills: vec![ConfirmedSkillInput {
                canonical_name: "Rust".into(),
                category: SkillCategory::CoreHard,
                proficiency: Proficiency::Intermediate,
                evidence: Some("Built Tauri core".into()),
            }],
            keywords: vec![ConfirmedKeywordInput {
                role_label: "Backend Engineer".into(),
                fit_type: FitType::PrimaryFit,
                scrape_queries: vec!["Backend Engineer".into(), "Rust Developer".into()],
                match_reason: Some("Strong Rust skills".into()),
                is_active: true,
            }],
        }
    }

    fn sample_job(id: &str, ext_id: &str, role: &str) -> NewJobRecord {
        NewJobRecord {
            id: id.into(),
            source: JobSource::Linkedin,
            external_id: ext_id.into(),
            url: Some(format!("https://linkedin.com/jobs/view/{ext_id}")),
            title: format!("{role} #{ext_id}"),
            normalized_role: role.into(),
            company: "PT Vecta".into(),
            location: Some("Jakarta".into()),
            work_mode: WorkMode::Hybrid,
            experience_level: ExperienceLevel::Junior,
            raw_description: "Full description text".into(),
            extracted_summary_json: Some(r#"{"role_summary":"Build services"}"#.into()),
            embedding: None,
            enrichment_status: EnrichmentStatus::AiEnriched,
            posted_at: None,
            salary_text: None,
            domain_slug: "informatika_data".into(),
            quality_score: 1.0,
            red_flags_json: "[]".into(),
            also_available_on_json: "[]".into(),
            embedding_model: None,
        }
    }

    #[test]
    fn migrations_are_idempotent_and_pragmas_enabled() {
        let db = Database::open_in_memory().expect("open_in_memory");
        db.run_migrations()
            .expect("second migration run should be idempotent");

        let (fk, mig_count): (i64, i64) = db
            .with_conn(|conn| {
                let fk: i64 = conn
                    .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
                    .map_err(|e| AppError::db_error(e.to_string()))?;
                let mig_count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM schema_migrations;", [], |r| r.get(0))
                    .map_err(|e| AppError::db_error(e.to_string()))?;
                Ok((fk, mig_count))
            })
            .expect("query pragmas");

        assert_eq!(fk, 1);
        assert_eq!(mig_count, 5);
    }

    #[test]
    fn cascade_delete_removes_user_children() {
        let db = Database::open_in_memory().expect("db");
        let profile = db
            .save_profile_atomic(&sample_profile_input(), None)
            .expect("save profile");
        db.insert_job_with_skills(&sample_job("job-1", "li-1", "Backend Engineer"), &[])
            .expect("insert job");
        db.upsert_job_matches(&[JobMatchUpsert {
            user_id: profile.id.clone(),
            job_id: "job-1".into(),
            vector_score: 0.8,
            skill_overlap_score: 0.7,
            level_score: 1.0,
            final_score: 0.785,
            matched_skills_json: "[]".into(),
            missing_skills_json: "[]".into(),
        }])
        .expect("upsert match");
        db.insert_roadmap_with_steps(
            &profile.id,
            Some("job-1"),
            &GeneratedRoadmap {
                title: "Plan".into(),
                target_role: "Backend Engineer".into(),
                total_estimated_weeks: 4,
                steps: vec![GeneratedRoadmapStep {
                    phase_order: 1,
                    phase_name: "Fundamental".into(),
                    target_skill: "Docker".into(),
                    topic_title: "Containers".into(),
                    learning_keywords: vec!["docker basics".into()],
                    mini_project_idea: "Containerize API".into(),
                    estimated_hours: 4,
                }],
            },
        )
        .expect("insert roadmap");

        db.with_conn(|conn| {
            conn.execute("DELETE FROM user_profiles WHERE id = ?1", [&profile.id])
                .map_err(|e| AppError::db_error(e.to_string()))?;
            for table in [
                "user_skills",
                "user_scrape_keywords",
                "job_matches",
                "learning_roadmaps",
                "roadmap_steps",
            ] {
                let count: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                    .map_err(|e| AppError::db_error(e.to_string()))?;
                assert_eq!(count, 0, "expected {table} to be empty after cascade delete");
            }
            Ok(())
        })
        .expect("verify cascade");
    }

    #[test]
    fn unique_source_external_id_rejects_duplicate_job() {
        let db = Database::open_in_memory().expect("db");
        db.insert_job_with_skills(&sample_job("job-1", "li-dup", "Backend Engineer"), &[])
            .expect("first insert");

        let found = db
            .job_exists_by_source_external_id(JobSource::Linkedin, "li-dup")
            .expect("exists query");
        assert_eq!(found.as_deref(), Some("job-1"));

        let err = db
            .insert_job_with_skills(&sample_job("job-2", "li-dup", "Backend Engineer"), &[])
            .expect_err("duplicate (source, external_id) must fail");
        assert_eq!(err.code, crate::error::ErrorCode::DbError);
    }

    #[test]
    fn upsert_skill_normalizes_case_and_aliases() {
        let db = Database::open_in_memory().expect("db");
        let (id1, id2, id3) = db
            .with_tx(|tx| {
                let id1 = Database::upsert_skill(
                    tx,
                    "React",
                    SkillCategory::ToolsTech,
                    &["React.js".into(), "ReactJS".into()],
                )?;
                let id2 = Database::upsert_skill(tx, "react", SkillCategory::ToolsTech, &[])?;
                let id3 = Database::upsert_skill(tx, "react.js", SkillCategory::ToolsTech, &[])?;
                Ok((id1, id2, id3))
            })
            .expect("upsert skills");

        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn upsert_job_match_preserves_saved_and_gap_cache() {
        let db = Database::open_in_memory().expect("db");
        let profile = db
            .save_profile_atomic(&sample_profile_input(), None)
            .expect("save profile");
        db.insert_job_with_skills(&sample_job("job-1", "li-save", "Backend Engineer"), &[])
            .expect("insert job");
        db.upsert_job_matches(&[JobMatchUpsert {
            user_id: profile.id.clone(),
            job_id: "job-1".into(),
            vector_score: 0.5,
            skill_overlap_score: 0.5,
            level_score: 1.0,
            final_score: 0.575,
            matched_skills_json: "[]".into(),
            missing_skills_json: "[]".into(),
        }])
        .expect("initial match");

        assert!(db
            .toggle_save_job(&profile.id, "job-1")
            .expect("toggle save"));
        let gap = GapAnalysis {
            verdict_headline: "Cocok secara dasar".into(),
            strength_highlights: vec!["Kuat di Rust".into()],
            prioritized_gaps: vec![PrioritizedGap {
                skill: "Docker".into(),
                severity: GapSeverity::High,
                reason: "Wajib untuk deployment".into(),
            }],
            cv_tailoring_tips: vec!["Tonjolkan proyek SQLite".into()],
        };
        db.save_gap_analysis(&profile.id, "job-1", &gap)
            .expect("save gap");

        db.upsert_job_matches(&[JobMatchUpsert {
            user_id: profile.id.clone(),
            job_id: "job-1".into(),
            vector_score: 0.9,
            skill_overlap_score: 0.9,
            level_score: 1.0,
            final_score: 0.915,
            matched_skills_json: "[]".into(),
            missing_skills_json: "[]".into(),
        }])
        .expect("re-upsert match");

        let detail = db.get_job_detail(&profile.id, "job-1").expect("get detail");
        assert!((detail.final_score - 0.915).abs() < 1e-6);
        assert!(
            detail.is_saved,
            "is_saved bookmark must be preserved on match recompute"
        );
        assert_eq!(detail.ai_gap_analysis, Some(gap));
    }

    #[test]
    fn competency_map_computes_exact_percentages_and_handles_zero_jobs() {
        let db = Database::open_in_memory().expect("db");
        let profile = db
            .save_profile_atomic(&sample_profile_input(), None)
            .expect("save profile");

        for idx in 1..=4 {
            let mut skills = vec![ExtractedJobSkill {
                canonical_name: "Rust".into(),
                category: SkillCategory::CoreHard,
                requirement_type: RequirementType::Mandatory,
                matched_aliases: vec![],
            }];
            if idx <= 3 {
                skills.push(ExtractedJobSkill {
                    canonical_name: "Docker".into(),
                    category: SkillCategory::ToolsTech,
                    requirement_type: RequirementType::Mandatory,
                    matched_aliases: vec![],
                });
            }
            if idx == 1 {
                skills.push(ExtractedJobSkill {
                    canonical_name: "Redis".into(),
                    category: SkillCategory::ToolsTech,
                    requirement_type: RequirementType::NiceToHave,
                    matched_aliases: vec![],
                });
            }
            db.insert_job_with_skills(
                &sample_job(
                    &format!("job-{idx}"),
                    &format!("li-{idx}"),
                    "Backend Engineer",
                ),
                &skills,
            )
            .expect("insert job");
        }

        let map = db
            .get_competency_map(&profile.id, "Backend Engineer")
            .expect("get competency map");
        assert_eq!(map.total_jobs, 4);
        assert!(map.is_small_sample);
        assert_eq!(map.skills.len(), 3);
        assert_eq!(map.skills[0].canonical_name, "Rust");
        assert_eq!(map.skills[0].demand_count, 4);
        assert!((map.skills[0].demand_percentage - 100.0).abs() < 0.01);
        assert!(map.skills[0].is_owned_by_user);
        assert_eq!(
            map.skills[0].user_proficiency,
            Some(Proficiency::Intermediate)
        );

        assert_eq!(map.skills[1].canonical_name, "Docker");
        assert!((map.skills[1].demand_percentage - 75.0).abs() < 0.01);
        assert!(!map.skills[1].is_owned_by_user);

        let empty_map = db
            .get_competency_map(&profile.id, "Nonexistent Role")
            .expect("zero-job role must not error");
        assert_eq!(empty_map.total_jobs, 0);
        assert!(empty_map.is_small_sample);
        assert!(empty_map.skills.is_empty());
    }

    #[test]
    fn upsert_skill_resolves_reverse_order_aliases() {
        let db = Database::open_in_memory().expect("db");
        let (first_id, second_id, row_count) = db
            .with_tx(|tx| {
                let id1 = Database::upsert_skill(tx, "React.js", SkillCategory::CoreHard, &[])?;
                let id2 = Database::upsert_skill(
                    tx,
                    "React",
                    SkillCategory::CoreHard,
                    &["React.js".into()],
                )?;
                let count: i64 = tx
                    .query_row("SELECT COUNT(*) FROM skills", [], |r| r.get(0))
                    .map_err(|e| db_err("count", e))?;
                Ok((id1, id2, count))
            })
            .expect("reverse alias upsert");

        assert_eq!(
            first_id, second_id,
            "incoming alias matching existing canonical_name must resolve to same skill_id"
        );
        assert_eq!(row_count, 1);
    }

    #[test]
    fn insert_job_with_skills_reflects_upgraded_mandatory_weight_on_duplicate_alias() {
        let db = Database::open_in_memory().expect("db");
        let rec = db
            .insert_job_with_skills(
                &sample_job("job-dup-skill", "li-dup-skill", "Frontend Engineer"),
                &[
                    ExtractedJobSkill {
                        canonical_name: "React.js".into(),
                        category: SkillCategory::CoreHard,
                        requirement_type: RequirementType::NiceToHave,
                        matched_aliases: vec![],
                    },
                    ExtractedJobSkill {
                        canonical_name: "React".into(),
                        category: SkillCategory::CoreHard,
                        requirement_type: RequirementType::Mandatory,
                        matched_aliases: vec!["React.js".into()],
                    },
                ],
            )
            .expect("insert job");

        assert_eq!(rec.skills.len(), 1);
        assert_eq!(rec.skills[0].requirement_type, RequirementType::Mandatory);
        assert!((rec.skills[0].weight - 1.0).abs() < 1e-6);
    }

    #[test]
    fn db_recalculate_1000_jobs_loads_and_upserts_under_100ms() {
        use std::time::Instant;

        use crate::core::matching::{
            compute_hybrid_matches_batch, encode_embedding, to_match_upsert,
        };

        let db = Database::open_in_memory().expect("db");
        let embedding = encode_embedding(&vec![0.25f32; 768]);
        let profile = db
            .save_profile_atomic(&sample_profile_input(), Some(embedding.as_slice()))
            .expect("save profile");

        // Seed 1,000 jobs x 5 skills each inside one setup transaction
        db.with_tx(|tx| {
            let mut skill_ids = Vec::new();
            for name in ["Rust", "SQLite", "Docker", "PostgreSQL", "Linux"] {
                skill_ids.push(Database::upsert_skill(
                    tx,
                    name,
                    SkillCategory::CoreHard,
                    &[],
                )?);
            }
            for i in 0..1000 {
                let jid = format!("perf-job-{i}");
                tx.execute(
                    "INSERT INTO jobs (id, source, external_id, title, normalized_role, company, work_mode, experience_level, raw_description, embedding)
                     VALUES (?1, 'linkedin', ?2, 'Backend Engineer', 'Backend Engineer', 'PT Perf', 'remote', 'junior', 'desc', ?3)",
                    params![jid, format!("li-perf-{i}"), embedding],
                )
                .map_err(|e| db_err("seed job", e))?;
                for &sid in &skill_ids {
                    tx.execute(
                        "INSERT INTO job_skills (job_id, skill_id, requirement_type, weight) VALUES (?1, ?2, 'mandatory', 1.0)",
                        params![jid, sid],
                    )
                    .map_err(|e| db_err("seed job_skill", e))?;
                }
            }
            Ok(())
        })
        .expect("seed 1000 jobs");

        let user_profile = db
            .get_profile_for_matching(&profile.id)
            .expect("profile")
            .expect("some profile");

        let _ = vecta_taxonomy::TaxonomyEngine::global();
        let start = Instant::now();
        let jobs = db.load_all_jobs_for_matching().expect("load all jobs");
        let computed_all = compute_hybrid_matches_batch(&user_profile, &jobs);
        let mut upserts = Vec::with_capacity(jobs.len());
        for (job, computed) in jobs.iter().zip(computed_all.iter()) {
            upserts.push(to_match_upsert(&profile.id, &job.job_id, computed).expect("upsert"));
        }
        db.upsert_job_matches(&upserts).expect("upsert matches");
        let elapsed = start.elapsed();

        assert_eq!(jobs.len(), 1000);
        assert!(jobs.iter().all(|j| j.skills.len() == 5));
        assert!(
            elapsed.as_millis() < 100,
            "Full DB load + match + upsert for 1,000 jobs took {:?}, must be < 100ms",
            elapsed
        );
    }

    #[test]
    fn app_settings_persists_and_overwrites_values() {
        let db = Database::open_in_memory().expect("open");
        assert_eq!(db.get_app_setting("worker_url").expect("initial"), None);
        db.set_app_setting("worker_url", "http://127.0.0.1:8000")
            .expect("set 1");
        assert_eq!(
            db.get_app_setting("worker_url").expect("read 1").as_deref(),
            Some("http://127.0.0.1:8000")
        );
        db.set_app_setting("worker_url", "http://localhost:8005")
            .expect("set 2");
        assert_eq!(
            db.get_app_setting("worker_url").expect("read 2").as_deref(),
            Some("http://localhost:8005")
        );
    }

    #[test]
    fn like_search_escapes_percent_and_underscore_wildcards() {
        assert_eq!(escape_like_pattern(r"100%_test\a"), r"100\%\_test\\a");

        let db = Database::open_in_memory().expect("open");
        let profile = db
            .save_profile_atomic(&sample_profile_input(), None)
            .expect("profile");

        let make_job = |id: &str, title: &str| NewJobRecord {
            id: id.into(),
            source: JobSource::Manual,
            external_id: id.into(),
            url: None,
            title: title.into(),
            normalized_role: "Backend Engineer".into(),
            company: "PT Wildcard".into(),
            location: Some("Jakarta".into()),
            work_mode: WorkMode::Remote,
            experience_level: ExperienceLevel::Junior,
            raw_description: "desc".into(),
            extracted_summary_json: None,
            embedding: None,
            enrichment_status: EnrichmentStatus::LocalOnly,
            posted_at: None,
            salary_text: None,
            domain_slug: "informatika_data".into(),
            quality_score: 1.0,
            red_flags_json: "[]".into(),
            also_available_on_json: "[]".into(),
            embedding_model: None,
        };

        db.insert_job_with_skills(&make_job("j-pct", "Backend 100% Remote"), &[])
            .expect("job 1");
        db.insert_job_with_skills(&make_job("j-plain", "Backend Engineer"), &[])
            .expect("job 2");
        db.upsert_job_matches(&[
            JobMatchUpsert {
                user_id: profile.id.clone(),
                job_id: "j-pct".into(),
                vector_score: 80.0,
                skill_overlap_score: 80.0,
                level_score: 100.0,
                final_score: 83.0,
                matched_skills_json: "[]".into(),
                missing_skills_json: "[]".into(),
            },
            JobMatchUpsert {
                user_id: profile.id.clone(),
                job_id: "j-plain".into(),
                vector_score: 80.0,
                skill_overlap_score: 80.0,
                level_score: 100.0,
                final_score: 83.0,
                matched_skills_json: "[]".into(),
                missing_skills_json: "[]".into(),
            },
        ])
        .expect("matches");

        let found = db
            .list_ranked_jobs(
                &profile.id,
                &JobFilterInput {
                    search_query: Some("100%".into()),
                    limit: Some(50),
                    offset: Some(0),
                    ..Default::default()
                },
            )
            .expect("search");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].job_id, "j-pct");
    }

    #[test]
    fn phase_b_multi_filters_watchlists_simulator_and_backup_roundtrip() {
        let db = Database::open_in_memory().expect("db");
        let profile = db
            .save_profile_atomic(&sample_profile_input(), None)
            .expect("profile");

        let mut j1 = sample_job("job-jkt", "jkt-1", "Backend Engineer");
        j1.location = Some("Jakarta Selatan".into());
        j1.work_mode = WorkMode::Hybrid;
        j1.salary_text = Some("Rp 10.000.000 - Rp 15.000.000".into());
        let mut j2 = sample_job("job-bdg", "bdg-1", "Backend Engineer");
        j2.title = "Management Trainee Engineering 2026".into();
        j2.location = Some("Bandung".into());
        j2.work_mode = WorkMode::Onsite;
        j2.salary_text = None;
        j2.raw_description = "Program MT unggulan. Requires Docker and PostgreSQL.".into();

        db.insert_job_with_skills(
            &j1,
            &[ExtractedJobSkill {
                canonical_name: "Rust".into(),
                category: SkillCategory::CoreHard,
                requirement_type: RequirementType::Mandatory,
                matched_aliases: vec![],
            }],
        )
        .expect("j1");
        db.insert_job_with_skills(
            &j2,
            &[ExtractedJobSkill {
                canonical_name: "Docker".into(),
                category: SkillCategory::ToolsTech,
                requirement_type: RequirementType::Mandatory,
                matched_aliases: vec![],
            }],
        )
        .expect("j2");

        db.upsert_job_matches(&[
            JobMatchUpsert {
                user_id: profile.id.clone(),
                job_id: "job-jkt".into(),
                vector_score: 0.70,
                skill_overlap_score: 1.0,
                level_score: 1.0,
                final_score: 0.88,
                matched_skills_json: "[]".into(),
                missing_skills_json: "[]".into(),
            },
            JobMatchUpsert {
                user_id: profile.id.clone(),
                job_id: "job-bdg".into(),
                vector_score: 0.60,
                skill_overlap_score: 0.0,
                level_score: 1.0,
                final_score: 0.48,
                matched_skills_json: "[]".into(),
                missing_skills_json: "[]".into(),
            },
        ])
        .expect("matches");

        // Multi-filter on location_clusters + salary_only
        let filtered = db
            .list_ranked_jobs(
                &profile.id,
                &JobFilterInput {
                    location_clusters: Some(vec!["jabodetabek".into()]),
                    salary_only: Some(true),
                    ..Default::default()
                },
            )
            .expect("filtered");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].job_id, "job-jkt");
        assert_eq!(filtered[0].salary_min_idr, Some(10_000_000));
        assert_eq!(filtered[0].salary_max_idr, Some(15_000_000));

        // Watchlist save/list/delete
        let wl = db
            .save_watchlist(
                &profile.id,
                "JKT Bergaji",
                &JobFilterInput {
                    location_clusters: Some(vec!["jabodetabek".into()]),
                    salary_only: Some(true),
                    ..Default::default()
                },
            )
            .expect("save wl");
        assert_eq!(db.list_saved_watchlists(&profile.id).expect("list wl").len(), 1);

        // What-If simulator on Docker skill
        let docker_id: i64 = db
            .with_conn(|c| c.query_row("SELECT id FROM skills WHERE canonical_name='Docker'", [], |r| r.get(0)).map_err(|e| AppError::db_error(e.to_string())))
            .expect("docker id");
        let sim = db
            .simulate_skill_impact(&profile.id, "Backend Engineer", &[docker_id])
            .expect("sim");
        assert_eq!(sim.len(), 1);
        assert!(sim[0].affected_jobs_count >= 1);
        assert!(sim[0].avg_score_delta > 10.0);

        // Portable .vecta backup roundtrip into fresh DB
        let backup = db.export_backup_json().expect("export");
        assert_eq!(backup.watchlists_count, 1);
        let db2 = Database::open_in_memory().expect("db2");
        let rep = db2.import_backup_json(&backup.payload_json).expect("import");
        assert_eq!(rep.imported_profiles, 1);
        assert_eq!(rep.imported_watchlists, 1);
        assert!(db2.get_active_profile().expect("p2").is_some());

        db.delete_watchlist(&wl.id).expect("del wl");
        assert!(db.list_saved_watchlists(&profile.id).expect("empty wl").is_empty());
    }

    #[test]
    fn analytics_and_multi_profile_crud_work_atomically() {
        let db = Database::open_in_memory().expect("db");
        let mut p1 = sample_profile_input();
        p1.id = Some("prof-1".into());
        p1.profile_name = "Jalur IT".into();
        let saved1 = db.save_profile_atomic(&p1, None).expect("save p1");

        let mut p2 = sample_profile_input();
        p2.id = Some("prof-2".into());
        p2.profile_name = "Jalur Pajak".into();
        p2.target_roles = vec!["Akuntan Pajak".into()];
        let saved2 = db.save_profile_atomic(&p2, None).expect("save p2");

        // p2 became active upon save; verify both appear and switch back to p1
        let list = db.list_user_profiles().expect("list profiles");
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|x| x.id == saved2.id && x.is_active));

        let switched = db.switch_active_profile(&saved1.id).expect("switch to p1");
        assert_eq!(switched.id, saved1.id);
        assert!(switched.is_active);

        let after_del = db.delete_user_profile(&saved1.id).expect("delete active p1");
        assert_eq!(after_del.id, saved2.id);
        assert!(after_del.is_active);

        // Cannot delete the last remaining profile
        let err = db
            .delete_user_profile(&saved2.id)
            .expect_err("must reject deleting final profile");
        assert_eq!(err.code, crate::error::ErrorCode::ValidationError);

        // Seed 2 salaried jobs & test salary benchmark + company snapshot
        let mut j1 = sample_job("jb-1", "ext-b1", "Backend Engineer");
        j1.salary_text = Some("Rp 10 jt - 14 jt".into());
        db.insert_job_with_skills(&j1, &[]).expect("ins j1");
        let mut j2 = sample_job("jb-2", "ext-b2", "Backend Engineer");
        j2.salary_text = Some("Rp 12 jt - 16 jt".into());
        db.insert_job_with_skills(&j2, &[]).expect("ins j2");

        let bench = db
            .get_salary_benchmark(Some("Backend Engineer"), None)
            .expect("bench");
        assert_eq!(bench.sample_count, 2);
        assert!(bench.is_small_sample);
        assert_eq!(bench.min_idr, Some(12_000_000));

        let snap = db
            .get_company_snapshot(&saved2.id, "PT Vecta")
            .expect("snapshot");
        assert_eq!(snap.total_jobs, 2);
        assert_eq!(snap.hybrid_count, 2);
    }
}
