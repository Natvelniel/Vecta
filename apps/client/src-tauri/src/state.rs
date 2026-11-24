use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use crate::core::config::{DEFAULT_WORKER_URL, SCRAPE_GEMINI_PACING_MS};
use crate::core::db::Database;
use crate::core::gemini::{GeminiProvider, HttpGeminiClient};
use crate::core::keychain::{OsKeyringStore, SecretStore};
use crate::core::matching::{compute_hybrid_matches_batch, to_match_upsert};
use crate::core::worker::{HttpWorkerClient, WorkerClient};
use crate::error::AppError;

pub struct ScrapeRunGuard<'a> {
    flag: &'a AtomicBool,
}

impl Drop for ScrapeRunGuard<'_> {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

pub struct AppState {
    pub db: Database,
    pub secrets: Arc<dyn SecretStore>,
    pub gemini: Arc<dyn GeminiProvider>,
    pub worker: Arc<dyn WorkerClient>,
    worker_url: RwLock<String>,
    pub scrape_pacing_ms: u64,
    scrape_in_progress: AtomicBool,
}

impl AppState {
    pub fn production(db: Database) -> Result<Self, AppError> {
        Ok(Self {
            db,
            secrets: Arc::new(OsKeyringStore),
            gemini: Arc::new(HttpGeminiClient::new()?),
            worker: Arc::new(HttpWorkerClient::new()?),
            worker_url: RwLock::new(DEFAULT_WORKER_URL.to_string()),
            scrape_pacing_ms: SCRAPE_GEMINI_PACING_MS,
            scrape_in_progress: AtomicBool::new(false),
        })
    }

    pub fn get_worker_url(&self) -> String {
        self.worker_url
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_worker_url(&self, url: String) {
        let mut guard = self.worker_url.write().unwrap_or_else(|e| e.into_inner());
        *guard = url;
    }

    pub fn try_acquire_scrape_guard(&self) -> Result<ScrapeRunGuard<'_>, AppError> {
        self.scrape_in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ScrapeRunGuard {
                flag: &self.scrape_in_progress,
            })
            .map_err(|_| {
                AppError::validation_error(
                    "Proses scraping sedang berjalan. Tunggu hingga selesai sebelum menjalankan ulang.",
                )
            })
    }

    pub fn require_api_key(&self) -> Result<String, AppError> {
        self.secrets
            .get_api_key()?
            .filter(|k| !k.trim().is_empty())
            .ok_or_else(AppError::api_key_missing)
    }

    pub fn recalculate_all_matches_for_user(&self, user_id: &str) -> Result<(), AppError> {
        let Some(profile) = self.db.get_profile_for_matching(user_id)? else {
            return Ok(());
        };
        let jobs = self.db.load_all_jobs_for_matching()?;
        if jobs.is_empty() {
            return Ok(());
        }

        let computed_all = compute_hybrid_matches_batch(&profile, &jobs);
        let mut upserts = Vec::with_capacity(jobs.len());
        for (job, computed) in jobs.iter().zip(computed_all.iter()) {
            upserts.push(to_match_upsert(user_id, &job.job_id, computed)?);
        }
        self.db.upsert_job_matches(&upserts)
    }
}

#[cfg(test)]
pub mod test_support {
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;

    use crate::core::db::Database;
    use crate::core::gemini::{
        EmbeddingTaskType, GapAnalysisPromptInput, GeminiProvider, RoadmapPromptInput,
    };
    use crate::core::keychain::InMemorySecretStore;
    use crate::core::worker::{
        WorkerClient, WorkerScrapeRequest, WorkerScrapeResponse, WorkerScrapeStats,
    };
    use crate::error::AppError;
    use crate::models::job::{
        ExtractedJobSkill, GapAnalysis, GapSeverity, JobExtractionResult, PrioritizedGap,
        RequirementType, WorkMode,
    };
    use crate::models::profile::{
        ExperienceLevel, ExtractedSkillDraft, FitType, Proficiency, RecommendedKeywordDraft,
        SkillCategory, StoryProfileDraft, WorkerHealth,
    };
    use crate::models::roadmap::{GeneratedRoadmap, GeneratedRoadmapStep};

    use super::AppState;

    pub struct MockGemini {
        pub fail_key_validation: bool,
        pub extract_calls: AtomicUsize,
        pub gap_calls: AtomicUsize,
        pub roadmap_calls: AtomicUsize,
        pub story_calls: AtomicUsize,
        pub embed_calls: AtomicUsize,
        pub custom_extraction: Mutex<Option<JobExtractionResult>>,
        pub custom_roadmap: Mutex<Option<GeneratedRoadmap>>,
        pub last_extract_description: Mutex<Option<String>>,
        pub fail_extract_error: Mutex<Option<AppError>>,
    }

    impl Default for MockGemini {
        fn default() -> Self {
            Self {
                fail_key_validation: false,
                extract_calls: AtomicUsize::new(0),
                gap_calls: AtomicUsize::new(0),
                roadmap_calls: AtomicUsize::new(0),
                story_calls: AtomicUsize::new(0),
                embed_calls: AtomicUsize::new(0),
                custom_extraction: Mutex::new(None),
                custom_roadmap: Mutex::new(None),
                last_extract_description: Mutex::new(None),
                fail_extract_error: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl GeminiProvider for MockGemini {
        async fn test_api_key(&self, _api_key: &str) -> Result<(), AppError> {
            if self.fail_key_validation {
                Err(AppError::api_key_invalid("API Key ditolak oleh mock."))
            } else {
                Ok(())
            }
        }

        async fn extract_job(
            &self,
            _api_key: &str,
            raw_description: &str,
            _known_skills: &[String],
        ) -> Result<JobExtractionResult, AppError> {
            self.extract_calls.fetch_add(1, Ordering::SeqCst);
            *self.last_extract_description.lock().unwrap() = Some(raw_description.to_string());
            if let Some(err) = self.fail_extract_error.lock().unwrap().clone() {
                return Err(err);
            }
            if let Some(custom) = self.custom_extraction.lock().unwrap().clone() {
                return Ok(custom);
            }
            Ok(JobExtractionResult {
                normalized_role: "Backend Engineer".into(),
                experience_level: ExperienceLevel::Junior,
                work_mode: WorkMode::Hybrid,
                role_summary: "Membangun layanan API performa tinggi dengan Rust dan SQLite.".into(),
                skills: vec![
                    ExtractedJobSkill {
                        canonical_name: "Rust".into(),
                        category: SkillCategory::CoreHard,
                        requirement_type: RequirementType::Mandatory,
                        matched_aliases: vec![],
                    },
                    ExtractedJobSkill {
                        canonical_name: "Docker".into(),
                        category: SkillCategory::ToolsTech,
                        requirement_type: RequirementType::NiceToHave,
                        matched_aliases: vec![],
                    },
                ],
            })
        }

        async fn analyze_gap(
            &self,
            _api_key: &str,
            _context: &GapAnalysisPromptInput,
        ) -> Result<GapAnalysis, AppError> {
            self.gap_calls.fetch_add(1, Ordering::SeqCst);
            Ok(GapAnalysis {
                verdict_headline: "Kandidat sangat potensial untuk peran ini.".into(),
                strength_highlights: vec!["Menguasai Rust dan SQLite".into()],
                prioritized_gaps: vec![PrioritizedGap {
                    skill: "Docker".into(),
                    severity: GapSeverity::Medium,
                    reason: "Dipakai untuk orkestrasi layanan".into(),
                }],
                cv_tailoring_tips: vec!["Sorot benchmark matching < 100ms".into()],
            })
        }

        async fn generate_roadmap(
            &self,
            _api_key: &str,
            context: &RoadmapPromptInput,
        ) -> Result<GeneratedRoadmap, AppError> {
            self.roadmap_calls.fetch_add(1, Ordering::SeqCst);
            if let Some(custom) = self.custom_roadmap.lock().unwrap().clone() {
                return Ok(custom);
            }
            let target_skill = context
                .missing_skills
                .first()
                .cloned()
                .unwrap_or_else(|| "Docker".into());
            Ok(GeneratedRoadmap {
                title: format!("Roadmap {} 4 Minggu", context.target_role),
                target_role: context.target_role.clone(),
                total_estimated_weeks: 4,
                steps: vec![
                    GeneratedRoadmapStep {
                        phase_order: 1,
                        phase_name: "Fundamental".into(),
                        target_skill: target_skill.clone(),
                        topic_title: format!("Dasar {target_skill}"),
                        learning_keywords: vec![format!("{target_skill} fundamentals")],
                        mini_project_idea: format!("Latihan CLI {target_skill}"),
                        estimated_hours: 4,
                    },
                    GeneratedRoadmapStep {
                        phase_order: 2,
                        phase_name: "Praktik".into(),
                        target_skill,
                        topic_title: "Integrasi Layanan Lokal".into(),
                        learning_keywords: vec!["docker compose multi-service".into()],
                        mini_project_idea: "Orkestrasi worker + database lokal".into(),
                        estimated_hours: 6,
                    },
                ],
            })
        }

        async fn parse_user_story(
            &self,
            _api_key: &str,
            _raw_story: &str,
            _known_skills: &[String],
        ) -> Result<StoryProfileDraft, AppError> {
            self.story_calls.fetch_add(1, Ordering::SeqCst);
            Ok(StoryProfileDraft {
                professional_summary: "Junior Backend Developer fokus pada Rust dan Python.".into(),
                estimated_experience_level: ExperienceLevel::Junior,
                extracted_skills: vec![ExtractedSkillDraft {
                    canonical_name: "Rust".into(),
                    category: SkillCategory::CoreHard,
                    proficiency: Proficiency::Intermediate,
                    evidence: "Membangun desktop engine lokal".into(),
                }],
                recommended_keywords: vec![
                    RecommendedKeywordDraft {
                        role_label: "Backend Engineer".into(),
                        fit_type: FitType::PrimaryFit,
                        match_reason: "Pengalaman langsung membangun API".into(),
                        scrape_queries: vec!["Backend Engineer".into(), "Rust Developer".into()],
                    },
                    RecommendedKeywordDraft {
                        role_label: "Systems Developer".into(),
                        fit_type: FitType::AdjacentFit,
                        match_reason: "Dasar Rust dan memori kuat".into(),
                        scrape_queries: vec!["Systems Developer".into()],
                    },
                ],
            })
        }

        async fn embed_text(
            &self,
            _api_key: &str,
            _text: &str,
            _task_type: EmbeddingTaskType,
        ) -> Result<Vec<f32>, AppError> {
            self.embed_calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![0.1f32; 768])
        }
    }

    pub struct MockWorker {
        pub response: Mutex<Result<WorkerScrapeResponse, AppError>>,
        pub health_ok: bool,
        pub last_health_url: Mutex<Option<String>>,
        pub last_scrape_url: Mutex<Option<String>>,
        pub scrape_delay_ms: AtomicU64,
    }

    impl Default for MockWorker {
        fn default() -> Self {
            Self {
                response: Mutex::new(Ok(WorkerScrapeResponse {
                    jobs: vec![],
                    stats: WorkerScrapeStats {
                        total_fetched: 0,
                        deduplicated_in_batch: 0,
                        duration_ms: 5,
                    },
                    warnings: vec![],
                })),
                health_ok: true,
                last_health_url: Mutex::new(None),
                last_scrape_url: Mutex::new(None),
                scrape_delay_ms: AtomicU64::new(0),
            }
        }
    }

    #[async_trait]
    impl WorkerClient for MockWorker {
        async fn check_health(&self, base_url: &str) -> Result<WorkerHealth, AppError> {
            *self.last_health_url.lock().unwrap() = Some(base_url.to_string());
            if self.health_ok {
                Ok(WorkerHealth {
                    status: "ok".into(),
                    version: "0.1.0".into(),
                    available_sources: vec![
                        "linkedin".into(),
                        "glints".into(),
                        "jobstreet".into(),
                    ],
                })
            } else {
                Err(AppError::scraper_unreachable("Mock worker offline"))
            }
        }

        async fn scrape(
            &self,
            base_url: &str,
            _request: &WorkerScrapeRequest,
        ) -> Result<WorkerScrapeResponse, AppError> {
            *self.last_scrape_url.lock().unwrap() = Some(base_url.to_string());
            let delay = self.scrape_delay_ms.load(Ordering::SeqCst);
            if delay > 0 {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            self.response.lock().unwrap().clone()
        }
    }

    pub fn build_test_state(
        initial_key: Option<&str>,
        gemini: Arc<MockGemini>,
        worker: Arc<MockWorker>,
    ) -> AppState {
        AppState {
            db: Database::open_in_memory().expect("in-memory db"),
            secrets: Arc::new(InMemorySecretStore::new(initial_key.map(str::to_string))),
            gemini,
            worker,
            worker_url: std::sync::RwLock::new("http://127.0.0.1:8000".into()),
            scrape_pacing_ms: 0,
            scrape_in_progress: std::sync::atomic::AtomicBool::new(false),
        }
    }
}
