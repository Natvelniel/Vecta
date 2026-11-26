use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::core::matching::quantize_i8_vec;

const VIDX_MAGIC: &[u8; 4] = b"VIDX";
const VIDX_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct IndexVectorEntry {
    pub scale: f32,
    pub q: Vec<i8>,
}

#[derive(Debug, Clone)]
pub struct SkillIndexRecord {
    pub skill_id_str: String,
    pub canonical_name: String,
    pub domain_slug: String,
    pub category: String,
    pub prototypes: Vec<IndexVectorEntry>,
}

#[derive(Debug, Clone)]
pub struct RoleIndexRecord {
    pub role_id: String,
    pub role_title: String,
    pub domain_slug: String,
    pub prototypes: Vec<IndexVectorEntry>,
}

#[derive(Debug, Clone)]
pub struct DomainIndexRecord {
    pub domain_slug: String,
    pub vector: IndexVectorEntry,
}

#[derive(Debug, Clone)]
pub struct SemanticSkillHit {
    pub canonical_name: String,
    pub domain_slug: String,
    pub category: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct SemanticRoleHit {
    pub role_id: String,
    pub role_title: String,
    pub domain_slug: String,
    pub score: f32,
}

pub struct SemanticIndex {
    pub dims: usize,
    pub skills: Vec<SkillIndexRecord>,
    pub roles: Vec<RoleIndexRecord>,
    pub domains: Vec<DomainIndexRecord>,
}

fn dot_with_i8(query: &[f32], entry: &IndexVectorEntry) -> f32 {
    if query.len() != entry.q.len() {
        return 0.0;
    }
    let mut sum = 0.0f32;
    for (&qf, &wi) in query.iter().zip(entry.q.iter()) {
        sum += qf * (wi as f32);
    }
    (sum * entry.scale).clamp(0.0, 1.0)
}

fn write_str(w: &mut impl Write, s: &str) -> Result<(), String> {
    let bytes = s.as_bytes();
    if bytes.len() > u16::MAX as usize {
        return Err("String too long for VIDX".into());
    }
    w.write_all(&(bytes.len() as u16).to_le_bytes())
        .map_err(|e| e.to_string())?;
    w.write_all(bytes).map_err(|e| e.to_string())
}

fn read_str(r: &mut impl Read) -> Result<String, String> {
    let mut lb = [0u8; 2];
    r.read_exact(&mut lb).map_err(|e| e.to_string())?;
    let len = u16::from_le_bytes(lb) as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).map_err(|e| e.to_string())?;
    String::from_utf8(buf).map_err(|e| e.to_string())
}

fn write_vec_entry(w: &mut impl Write, entry: &IndexVectorEntry) -> Result<(), String> {
    w.write_all(&entry.scale.to_le_bytes())
        .map_err(|e| e.to_string())?;
    let raw: Vec<u8> = entry.q.iter().map(|&v| v as u8).collect();
    w.write_all(&raw).map_err(|e| e.to_string())
}

fn read_vec_entry(r: &mut impl Read, dims: usize) -> Result<IndexVectorEntry, String> {
    let mut sb = [0u8; 4];
    r.read_exact(&mut sb).map_err(|e| e.to_string())?;
    let scale = f32::from_le_bytes(sb);
    let mut raw = vec![0u8; dims];
    r.read_exact(&mut raw).map_err(|e| e.to_string())?;
    let q = raw.into_iter().map(|b| b as i8).collect();
    Ok(IndexVectorEntry { scale, q })
}

impl SemanticIndex {
    pub fn encode_vec(vec: &[f32]) -> IndexVectorEntry {
        let (scale, q) = quantize_i8_vec(vec);
        IndexVectorEntry { scale, q }
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let f = File::create(path).map_err(|e| e.to_string())?;
        let mut w = BufWriter::new(f);

        w.write_all(VIDX_MAGIC).map_err(|e| e.to_string())?;
        w.write_all(&VIDX_VERSION.to_le_bytes()).map_err(|e| e.to_string())?;
        w.write_all(&(self.dims as u32).to_le_bytes()).map_err(|e| e.to_string())?;
        w.write_all(&(self.skills.len() as u32).to_le_bytes()).map_err(|e| e.to_string())?;
        w.write_all(&(self.roles.len() as u32).to_le_bytes()).map_err(|e| e.to_string())?;
        w.write_all(&(self.domains.len() as u32).to_le_bytes()).map_err(|e| e.to_string())?;

        for sk in &self.skills {
            write_str(&mut w, &sk.skill_id_str)?;
            write_str(&mut w, &sk.canonical_name)?;
            write_str(&mut w, &sk.domain_slug)?;
            write_str(&mut w, &sk.category)?;
            w.write_all(&[sk.prototypes.len().min(u8::MAX as usize) as u8])
                .map_err(|e| e.to_string())?;
            for p in &sk.prototypes {
                write_vec_entry(&mut w, p)?;
            }
        }
        for rl in &self.roles {
            write_str(&mut w, &rl.role_id)?;
            write_str(&mut w, &rl.role_title)?;
            write_str(&mut w, &rl.domain_slug)?;
            w.write_all(&[rl.prototypes.len().min(u8::MAX as usize) as u8])
                .map_err(|e| e.to_string())?;
            for p in &rl.prototypes {
                write_vec_entry(&mut w, p)?;
            }
        }
        for dm in &self.domains {
            write_str(&mut w, &dm.domain_slug)?;
            write_vec_entry(&mut w, &dm.vector)?;
        }
        w.flush().map_err(|e| e.to_string())
    }

    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let f = File::open(path).map_err(|e| format!("Cannot open VIDX: {e}"))?;
        let mut r = BufReader::new(f);
        let mut hdr = [0u8; 24];
        r.read_exact(&mut hdr).map_err(|e| format!("EOF VIDX hdr: {e}"))?;
        if &hdr[0..4] != VIDX_MAGIC {
            return Err("Invalid VIDX magic".into());
        }
        let ver = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]);
        if ver != VIDX_VERSION {
            return Err(format!("Unsupported VIDX version {ver}"));
        }
        let dims = u32::from_le_bytes([hdr[8], hdr[9], hdr[10], hdr[11]]) as usize;
        let num_skills = u32::from_le_bytes([hdr[12], hdr[13], hdr[14], hdr[15]]) as usize;
        let num_roles = u32::from_le_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]) as usize;
        let num_domains = u32::from_le_bytes([hdr[20], hdr[21], hdr[22], hdr[23]]) as usize;

        let mut skills = Vec::with_capacity(num_skills);
        for _ in 0..num_skills {
            let skill_id_str = read_str(&mut r)?;
            let canonical_name = read_str(&mut r)?;
            let domain_slug = read_str(&mut r)?;
            let category = read_str(&mut r)?;
            let mut pb = [0u8; 1];
            r.read_exact(&mut pb).map_err(|e| e.to_string())?;
            let count = pb[0] as usize;
            let mut prototypes = Vec::with_capacity(count);
            for _ in 0..count {
                prototypes.push(read_vec_entry(&mut r, dims)?);
            }
            skills.push(SkillIndexRecord {
                skill_id_str,
                canonical_name,
                domain_slug,
                category,
                prototypes,
            });
        }

        let mut roles = Vec::with_capacity(num_roles);
        for _ in 0..num_roles {
            let role_id = read_str(&mut r)?;
            let role_title = read_str(&mut r)?;
            let domain_slug = read_str(&mut r)?;
            let mut pb = [0u8; 1];
            r.read_exact(&mut pb).map_err(|e| e.to_string())?;
            let count = pb[0] as usize;
            let mut prototypes = Vec::with_capacity(count);
            for _ in 0..count {
                prototypes.push(read_vec_entry(&mut r, dims)?);
            }
            roles.push(RoleIndexRecord {
                role_id,
                role_title,
                domain_slug,
                prototypes,
            });
        }

        let mut domains = Vec::with_capacity(num_domains);
        for _ in 0..num_domains {
            let domain_slug = read_str(&mut r)?;
            let vector = read_vec_entry(&mut r, dims)?;
            domains.push(DomainIndexRecord {
                domain_slug,
                vector,
            });
        }

        Ok(Self {
            dims,
            skills,
            roles,
            domains,
        })
    }

    pub fn search_skills(&self, query_vec: &[f32], top_k: usize) -> (Vec<SemanticSkillHit>, f32) {
        if query_vec.len() != self.dims || self.skills.is_empty() {
            return (Vec::new(), 0.0);
        }
        let mut all_scores = Vec::with_capacity(self.skills.len());
        let mut hits = Vec::with_capacity(self.skills.len());
        for sk in &self.skills {
            let best = sk
                .prototypes
                .iter()
                .map(|p| dot_with_i8(query_vec, p))
                .fold(0.0f32, f32::max);
            all_scores.push(best);
            hits.push(SemanticSkillHit {
                canonical_name: sk.canonical_name.clone(),
                domain_slug: sk.domain_slug.clone(),
                category: sk.category.clone(),
                score: best,
            });
        }
        all_scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = all_scores[all_scores.len() / 2];
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(top_k);
        (hits, median)
    }

    pub fn search_roles(&self, query_vec: &[f32], top_k: usize) -> Vec<SemanticRoleHit> {
        if query_vec.len() != self.dims || self.roles.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<SemanticRoleHit> = self
            .roles
            .iter()
            .map(|rl| {
                let best = rl
                    .prototypes
                    .iter()
                    .map(|p| dot_with_i8(query_vec, p))
                    .fold(0.0f32, f32::max);
                SemanticRoleHit {
                    role_id: rl.role_id.clone(),
                    role_title: rl.role_title.clone(),
                    domain_slug: rl.domain_slug.clone(),
                    score: best,
                }
            })
            .collect();
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(top_k);
        hits
    }

    pub fn classify_domains(&self, query_vec: &[f32]) -> Vec<(String, f32)> {
        if query_vec.len() != self.dims {
            return Vec::new();
        }
        let mut scored: Vec<(String, f32)> = self
            .domains
            .iter()
            .map(|d| (d.domain_slug.clone(), dot_with_i8(query_vec, &d.vector)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_index_vidx_roundtrips_and_ranks_correctly() {
        let dir = std::env::temp_dir().join("vecta_vidx_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.vidx");

        let idx = SemanticIndex {
            dims: 4,
            skills: vec![
                SkillIndexRecord {
                    skill_id_str: "s1".into(),
                    canonical_name: "Inventory Management".into(),
                    domain_slug: "supply_chain".into(),
                    category: "core_hard".into(),
                    prototypes: vec![SemanticIndex::encode_vec(&[0.95, 0.20, 0.10, 0.0])],
                },
                SkillIndexRecord {
                    skill_id_str: "s2".into(),
                    canonical_name: "Docker".into(),
                    domain_slug: "informatika_data".into(),
                    category: "tools_tech".into(),
                    prototypes: vec![SemanticIndex::encode_vec(&[0.0, 0.1, 0.95, 0.2])],
                },
            ],
            roles: vec![RoleIndexRecord {
                role_id: "r1".into(),
                role_title: "Warehouse & Inventory Staff".into(),
                domain_slug: "supply_chain".into(),
                prototypes: vec![SemanticIndex::encode_vec(&[0.98, 0.15, 0.05, 0.0])],
            }],
            domains: vec![DomainIndexRecord {
                domain_slug: "supply_chain".into(),
                vector: SemanticIndex::encode_vec(&[0.96, 0.15, 0.05, 0.0]),
            }],
        };

        idx.save_to_path(&path).expect("save vidx");
        let loaded = SemanticIndex::load_from_path(&path).expect("load vidx");
        assert_eq!(loaded.skills.len(), 2);
        assert_eq!(loaded.roles.len(), 1);

        let (hits, _med) = loaded.search_skills(&[0.95, 0.20, 0.0, 0.0], 2);
        assert_eq!(hits[0].canonical_name, "Inventory Management");
        assert!(hits[0].score > hits[1].score);

        let roles = loaded.search_roles(&[0.95, 0.20, 0.0, 0.0], 1);
        assert_eq!(roles[0].role_title, "Warehouse & Inventory Staff");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
