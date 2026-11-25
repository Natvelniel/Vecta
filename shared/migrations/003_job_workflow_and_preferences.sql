-- Vecta Migration 003: Decoupled local/AI enrichment, multi-portal support (including Indeed),
-- job metadata (posted_at, salary_text), application pipeline tracker, user notes, and Apply Copilot cache.

-- Rebuild `jobs` table cleanly to expand `source` CHECK constraint ('indeed' added)
-- and add enrichment_status / posted_at / salary_text while preserving all existing rows.
CREATE TABLE IF NOT EXISTS jobs_v3 (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL CHECK (source IN (
        'linkedin', 'glints', 'jobstreet', 'indeed', 'manual'
    )),
    external_id TEXT NOT NULL,
    url TEXT,
    title TEXT NOT NULL,
    normalized_role TEXT NOT NULL,
    company TEXT NOT NULL,
    location TEXT,
    work_mode TEXT NOT NULL CHECK (work_mode IN (
        'onsite', 'hybrid', 'remote', 'unknown'
    )) DEFAULT 'unknown',
    experience_level TEXT NOT NULL CHECK (experience_level IN (
        'intern', 'junior', 'mid', 'senior', 'unknown'
    )) DEFAULT 'unknown',
    raw_description TEXT NOT NULL,
    extracted_summary_json TEXT,
    embedding BLOB,
    enrichment_status TEXT NOT NULL CHECK (enrichment_status IN (
        'local_only', 'ai_enriched'
    )) DEFAULT 'ai_enriched',
    posted_at TEXT,
    salary_text TEXT,
    scraped_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source, external_id)
);

INSERT INTO jobs_v3 (
    id, source, external_id, url, title, normalized_role, company, location,
    work_mode, experience_level, raw_description, extracted_summary_json,
    embedding, enrichment_status, posted_at, salary_text, scraped_at
)
SELECT
    id, source, external_id, url, title, normalized_role, company, location,
    work_mode, experience_level, raw_description, extracted_summary_json,
    embedding,
    CASE WHEN extracted_summary_json IS NOT NULL THEN 'ai_enriched' ELSE 'local_only' END,
    NULL,
    NULL,
    scraped_at
FROM jobs;

DROP TABLE jobs;
ALTER TABLE jobs_v3 RENAME TO jobs;

CREATE INDEX IF NOT EXISTS idx_jobs_normalized_role ON jobs(normalized_role);
CREATE INDEX IF NOT EXISTS idx_jobs_enrichment_status ON jobs(enrichment_status);

-- Extend `job_matches` with Application Pipeline Tracker, notes, and cached Apply Copilot kit
ALTER TABLE job_matches ADD COLUMN application_status TEXT NOT NULL DEFAULT 'new'
    CHECK (application_status IN ('new', 'saved', 'applied', 'interview', 'offer', 'rejected', 'dismissed'));
ALTER TABLE job_matches ADD COLUMN applied_at DATETIME;
ALTER TABLE job_matches ADD COLUMN user_notes TEXT;
ALTER TABLE job_matches ADD COLUMN ai_apply_kit_json TEXT;

-- Sync existing saved bookmarks into the new application_status pipeline cleanly
UPDATE job_matches
SET application_status = 'saved'
WHERE is_saved = 1 AND application_status = 'new';

CREATE INDEX IF NOT EXISTS idx_job_matches_app_status ON job_matches(user_id, application_status);
