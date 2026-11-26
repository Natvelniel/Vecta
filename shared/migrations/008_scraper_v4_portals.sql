-- Vecta V4 migration (Scraper v4 portals: Karirhub Kemnaker RI & Remotive)
-- Recreates `jobs` table with expanded `source` CHECK constraint while preserving
-- all columns from migrations 004, 005, and 007 and retaining all foreign key rows
-- (executed under PRAGMA foreign_keys = OFF in db.rs).

CREATE TABLE IF NOT EXISTS jobs_v8_new (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL CHECK(source IN (
        'linkedin', 'glints', 'jobstreet', 'indeed', 'kalibrr', 'dealls', 'karirhub', 'remotive', 'manual'
    )),
    external_id TEXT NOT NULL,
    url TEXT,
    title TEXT NOT NULL,
    normalized_role TEXT NOT NULL,
    company TEXT NOT NULL,
    location TEXT,
    work_mode TEXT NOT NULL DEFAULT 'unknown'
        CHECK(work_mode IN ('remote', 'hybrid', 'onsite', 'unknown')),
    experience_level TEXT NOT NULL DEFAULT 'unknown'
        CHECK(experience_level IN ('intern', 'junior', 'mid', 'senior', 'unknown')),
    raw_description TEXT NOT NULL,
    extracted_summary_json TEXT,
    embedding BLOB,
    enrichment_status TEXT NOT NULL DEFAULT 'ai_enriched'
        CHECK(enrichment_status IN ('local_only', 'ai_enriched')),
    posted_at TEXT,
    salary_text TEXT,
    domain_slug TEXT NOT NULL DEFAULT 'umum_lintas_bidang',
    quality_score REAL NOT NULL DEFAULT 1.0,
    red_flags_json TEXT NOT NULL DEFAULT '[]',
    also_available_on_json TEXT NOT NULL DEFAULT '[]',
    embedding_model TEXT,
    scraped_at TEXT NOT NULL DEFAULT (datetime('now')),
    location_cluster TEXT NOT NULL DEFAULT 'indonesia_lainnya',
    salary_min_idr INTEGER,
    salary_max_idr INTEGER,
    opportunity_type TEXT NOT NULL DEFAULT 'full_time',
    emb_role BLOB,
    emb_skills BLOB,
    emb_context BLOB,
    UNIQUE(source, external_id)
);

INSERT INTO jobs_v8_new (
    id, source, external_id, url, title, normalized_role, company, location,
    work_mode, experience_level, raw_description, extracted_summary_json,
    embedding, enrichment_status, posted_at, salary_text, domain_slug,
    quality_score, red_flags_json, also_available_on_json, embedding_model,
    scraped_at, location_cluster, salary_min_idr, salary_max_idr,
    opportunity_type, emb_role, emb_skills, emb_context
)
SELECT
    id, source, external_id, url, title, normalized_role, company, location,
    work_mode, experience_level, raw_description, extracted_summary_json,
    embedding, enrichment_status, posted_at, salary_text, domain_slug,
    quality_score, red_flags_json, also_available_on_json, embedding_model,
    scraped_at, location_cluster, salary_min_idr, salary_max_idr,
    opportunity_type, emb_role, emb_skills, emb_context
FROM jobs;

DROP TABLE jobs;
ALTER TABLE jobs_v8_new RENAME TO jobs;

CREATE INDEX IF NOT EXISTS idx_jobs_normalized_role ON jobs(normalized_role);
CREATE INDEX IF NOT EXISTS idx_jobs_scraped_at ON jobs(scraped_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_enrichment_status ON jobs(enrichment_status);
CREATE INDEX IF NOT EXISTS idx_jobs_domain_slug ON jobs(domain_slug);
CREATE INDEX IF NOT EXISTS idx_jobs_location_cluster ON jobs(location_cluster);
CREATE INDEX IF NOT EXISTS idx_jobs_opportunity_type ON jobs(opportunity_type);
CREATE INDEX IF NOT EXISTS idx_jobs_salary_min ON jobs(salary_min_idr);
