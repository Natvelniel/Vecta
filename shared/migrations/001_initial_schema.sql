PRAGMA foreign_keys = ON;

-- 0. Pelacak versi migrasi skema
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 1. Profil Pengguna (mendukung lebih dari 1 profil karier)
CREATE TABLE IF NOT EXISTS user_profiles (
    id TEXT PRIMARY KEY,
    profile_name TEXT NOT NULL,
    target_roles_json TEXT NOT NULL DEFAULT '[]',
    raw_story_text TEXT,
    summary TEXT,
    experience_level TEXT NOT NULL CHECK (experience_level IN (
        'intern', 'junior', 'mid', 'senior', 'unknown'
    )) DEFAULT 'junior',
    embedding BLOB,                        -- Vec<f32> little-endian (768 * 4 bytes)
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. Kamus Kompetensi Baku (pusat normalisasi & agregasi Peta Kompetensi)
CREATE TABLE IF NOT EXISTS skills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    canonical_name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    category TEXT NOT NULL CHECK (category IN (
        'core_hard', 'tools_tech', 'soft_skill', 'domain'
    )),
    aliases_json TEXT NOT NULL DEFAULT '[]',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 3. Relasi Skill Milik Pengguna (M:N)
CREATE TABLE IF NOT EXISTS user_skills (
    user_id TEXT NOT NULL,
    skill_id INTEGER NOT NULL,
    proficiency TEXT NOT NULL CHECK (proficiency IN (
        'beginner', 'intermediate', 'advanced'
    )) DEFAULT 'intermediate',
    evidence TEXT,
    PRIMARY KEY (user_id, skill_id),
    FOREIGN KEY (user_id) REFERENCES user_profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

-- 4. Rekomendasi & Daftar Aktif Keyword Scraping milik Pengguna
CREATE TABLE IF NOT EXISTS user_scrape_keywords (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    role_label TEXT NOT NULL,
    fit_type TEXT NOT NULL CHECK (fit_type IN (
        'primary_fit', 'adjacent_fit', 'custom'
    )) DEFAULT 'custom',
    scrape_queries_json TEXT NOT NULL DEFAULT '[]',
    match_reason TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    last_scraped_at DATETIME,
    FOREIGN KEY (user_id) REFERENCES user_profiles(id) ON DELETE CASCADE
);

-- 5. Data Lowongan Hasil Scraping atau Input Manual
CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL CHECK (source IN (
        'linkedin', 'glints', 'jobstreet', 'manual'
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
    scraped_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source, external_id)
);

-- 6. Relasi Kompetensi yang Diminta Setiap Lowongan (M:N)
CREATE TABLE IF NOT EXISTS job_skills (
    job_id TEXT NOT NULL,
    skill_id INTEGER NOT NULL,
    requirement_type TEXT NOT NULL CHECK (requirement_type IN (
        'mandatory', 'nice_to_have'
    )),
    weight REAL NOT NULL DEFAULT 1.0,
    PRIMARY KEY (job_id, skill_id),
    FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE,
    FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

-- 7. Hasil Hybrid Smart Matching & Cache Analisis Gap
CREATE TABLE IF NOT EXISTS job_matches (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    vector_score REAL NOT NULL,
    skill_overlap_score REAL NOT NULL,
    level_score REAL NOT NULL,
    final_score REAL NOT NULL,
    matched_skills_json TEXT NOT NULL DEFAULT '[]',
    missing_skills_json TEXT NOT NULL DEFAULT '[]',
    ai_gap_analysis_json TEXT,
    is_saved INTEGER NOT NULL DEFAULT 0,
    computed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, job_id),
    FOREIGN KEY (user_id) REFERENCES user_profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
);

-- 8. Header Roadmap Belajar
CREATE TABLE IF NOT EXISTS learning_roadmaps (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    target_role TEXT NOT NULL,
    source_job_id TEXT,
    title TEXT NOT NULL,
    total_estimated_weeks INTEGER NOT NULL DEFAULT 4,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES user_profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (source_job_id) REFERENCES jobs(id) ON DELETE SET NULL
);

-- 9. Item Checklist Tahapan Belajar
CREATE TABLE IF NOT EXISTS roadmap_steps (
    id TEXT PRIMARY KEY,
    roadmap_id TEXT NOT NULL,
    skill_id INTEGER,
    phase_order INTEGER NOT NULL,
    phase_name TEXT NOT NULL,
    topic_title TEXT NOT NULL,
    learning_keywords_json TEXT NOT NULL DEFAULT '[]',
    mini_project_idea TEXT,
    estimated_hours INTEGER NOT NULL DEFAULT 4,
    status TEXT NOT NULL CHECK (status IN (
        'todo', 'in_progress', 'done'
    )) DEFAULT 'todo',
    completed_at DATETIME,
    FOREIGN KEY (roadmap_id) REFERENCES learning_roadmaps(id) ON DELETE CASCADE,
    FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE SET NULL
);

-- Indeks Performa Query
CREATE INDEX IF NOT EXISTS idx_jobs_normalized_role ON jobs(normalized_role);
CREATE INDEX IF NOT EXISTS idx_job_skills_skill_id ON job_skills(skill_id);
CREATE INDEX IF NOT EXISTS idx_job_matches_ranking ON job_matches(user_id, final_score DESC);
CREATE INDEX IF NOT EXISTS idx_roadmap_steps_order ON roadmap_steps(roadmap_id, phase_order);
CREATE INDEX IF NOT EXISTS idx_user_scrape_keywords_active ON user_scrape_keywords(user_id, is_active);
