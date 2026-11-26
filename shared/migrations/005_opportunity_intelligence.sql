-- 005: Opportunity Intelligence (Location Clusters, Salary Extraction IDR, Opportunity Types, Watchlists)

ALTER TABLE jobs ADD COLUMN location_cluster TEXT NOT NULL DEFAULT 'indonesia_lainnya';
ALTER TABLE jobs ADD COLUMN salary_min_idr INTEGER;
ALTER TABLE jobs ADD COLUMN salary_max_idr INTEGER;
ALTER TABLE jobs ADD COLUMN opportunity_type TEXT NOT NULL DEFAULT 'full_time';

CREATE TABLE IF NOT EXISTS saved_watchlists (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    filter_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES user_profiles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_jobs_location_cluster ON jobs(location_cluster);
CREATE INDEX IF NOT EXISTS idx_jobs_opportunity_type ON jobs(opportunity_type);
CREATE INDEX IF NOT EXISTS idx_jobs_salary_min ON jobs(salary_min_idr);
CREATE INDEX IF NOT EXISTS idx_watchlists_user ON saved_watchlists(user_id);
