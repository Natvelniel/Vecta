-- Vecta V3 migration: expanded portals (Kalibrr, Dealls), cross-domain metadata,
-- cross-portal deduplication, red-flag quality score, offline embedding versioning, and Master CV store.

CREATE TABLE IF NOT EXISTS jobs_v3_new (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL CHECK(source IN ('linkedin', 'glints', 'jobstreet', 'indeed', 'kalibrr', 'dealls', 'manual')),
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
    UNIQUE(source, external_id)
);

INSERT INTO jobs_v3_new (
    id, source, external_id, url, title, normalized_role, company, location,
    work_mode, experience_level, raw_description, extracted_summary_json,
    embedding, enrichment_status, posted_at, salary_text, scraped_at
)
SELECT
    id, source, external_id, url, title, normalized_role, company, location,
    work_mode, experience_level, raw_description, extracted_summary_json,
    embedding, enrichment_status, posted_at, salary_text, scraped_at
FROM jobs;

DROP TABLE jobs;
ALTER TABLE jobs_v3_new RENAME TO jobs;

CREATE INDEX IF NOT EXISTS idx_jobs_normalized_role ON jobs(normalized_role);
CREATE INDEX IF NOT EXISTS idx_jobs_scraped_at ON jobs(scraped_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_enrichment_status ON jobs(enrichment_status);
CREATE INDEX IF NOT EXISTS idx_jobs_domain_slug ON jobs(domain_slug);

ALTER TABLE user_profiles ADD COLUMN embedding_model TEXT;

CREATE TABLE IF NOT EXISTS user_cv_profiles (
    user_id TEXT PRIMARY KEY REFERENCES user_profiles(id) ON DELETE CASCADE,
    full_name TEXT NOT NULL DEFAULT '',
    headline TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    phone TEXT NOT NULL DEFAULT '',
    location TEXT NOT NULL DEFAULT '',
    linkedin_url TEXT NOT NULL DEFAULT '',
    portfolio_url TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    experiences_json TEXT NOT NULL DEFAULT '[]',
    education_json TEXT NOT NULL DEFAULT '[]',
    projects_json TEXT NOT NULL DEFAULT '[]',
    certifications_json TEXT NOT NULL DEFAULT '[]',
    languages_json TEXT NOT NULL DEFAULT '[]',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
