ALTER TABLE user_profiles ADD COLUMN emb_role BLOB;
ALTER TABLE user_profiles ADD COLUMN emb_skills BLOB;
ALTER TABLE user_profiles ADD COLUMN emb_story BLOB;
ALTER TABLE user_profiles ADD COLUMN dominant_domain TEXT;

ALTER TABLE jobs ADD COLUMN emb_role BLOB;
ALTER TABLE jobs ADD COLUMN emb_skills BLOB;
ALTER TABLE jobs ADD COLUMN emb_context BLOB;

ALTER TABLE job_matches ADD COLUMN domain_score REAL;
ALTER TABLE job_matches ADD COLUMN reranker_score REAL;
ALTER TABLE job_matches ADD COLUMN match_explanation_json TEXT NOT NULL DEFAULT '[]';
