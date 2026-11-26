use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};
use tokenizers::Tokenizer;
use vecta_neural::neural_xlmr::{l2_normalize_inplace, QuantizedXlmrModel};
use vecta_neural::semantic_index::{
    DomainIndexRecord, RoleIndexRecord, SemanticIndex, SkillIndexRecord,
};
use vecta_taxonomy::TaxonomyEngine;

fn sha256_file_hex(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed reading {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

fn default_pack_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/client/src-tauri/resources")
        .join("models")
        .join("neural-v2")
}

fn resolve_pack_dir() -> PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--pack-dir" {
            if let Some(dir) = args.next() {
                return PathBuf::from(dir);
            }
        }
    }
    default_pack_dir()
}

fn update_manifest(pack_dir: &Path, vidx_path: &Path) -> Result<(), String> {
    let manifest_path = pack_dir.join("manifest.json");
    let mut root = if manifest_path.exists() {
        let raw = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed reading manifest.json: {e}"))?;
        serde_json::from_str::<serde_json::Value>(&raw)
            .unwrap_or_else(|_| serde_json::json!({ "pack_version": 2, "engine": "vecta-neural-v2", "files": {} }))
    } else {
        serde_json::json!({ "pack_version": 2, "engine": "vecta-neural-v2", "files": {} })
    };

    let size_bytes = std::fs::metadata(vidx_path)
        .map_err(|e| format!("Failed stating semantic_index.vidx: {e}"))?
        .len();
    let sha256 = sha256_file_hex(vidx_path)?;

    let files_obj = root
        .as_object_mut()
        .ok_or_else(|| "manifest.json root is not an object".to_string())?
        .entry("files")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| "manifest.json files is not an object".to_string())?;

    files_obj.insert(
        "semantic_index.vidx".to_string(),
        serde_json::json!({
            "size_bytes": size_bytes,
            "sha256": sha256,
        }),
    );

    let pretty =
        serde_json::to_string_pretty(&root).map_err(|e| format!("Failed serializing manifest: {e}"))?;
    std::fs::write(&manifest_path, pretty)
        .map_err(|e| format!("Failed writing manifest.json: {e}"))?;
    Ok(())
}

fn main() -> Result<(), String> {
    let pack_dir = resolve_pack_dir();
    let tok_path = pack_dir.join("tokenizer.json");
    let bi_path = pack_dir.join("e5-base.vtxr");
    let vidx_path = pack_dir.join("semantic_index.vidx");

    if !tok_path.exists() || !bi_path.exists() {
        return Err(format!(
            "Missing tokenizer.json or e5-base.vtxr in {}. Run scripts/build_neural_v2_pack.py first.",
            pack_dir.display()
        ));
    }

    println!("[build_semantic_index] Loading tokenizer & e5-base.vtxr from {}", pack_dir.display());
    let tokenizer = Arc::new(
        Tokenizer::from_file(&tok_path).map_err(|e| format!("Failed loading tokenizer: {e}"))?,
    );
    let model = QuantizedXlmrModel::load(&bi_path, tokenizer)?;
    let dims = model.hidden_size();

    let tax = TaxonomyEngine::global();
    let all_skills = tax.all_skills();
    let all_roles = tax.roles_catalog();
    println!(
        "[build_semantic_index] Encoding {} skills & {} roles ({} dims)...",
        all_skills.len(),
        all_roles.len(),
        dims
    );

    let mut domain_accum: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    let mut skills_out = Vec::with_capacity(all_skills.len());

    for (idx, sk) in all_skills.iter().enumerate() {
        let domain_clean = sk.domain.replace('_', " ");
        let p1 = format!("{} ({}, {})", sk.canonical_name, domain_clean, sk.category);
        let aliases_short: Vec<&str> = sk.aliases.iter().take(6).map(|s| s.as_str()).collect();
        let p2 = if aliases_short.is_empty() {
            format!("keahlian {} {} skill", sk.canonical_name, domain_clean)
        } else {
            format!("{}: {}", sk.canonical_name, aliases_short.join(", "))
        };
        let p3 = format!(
            "pekerjaan menggunakan {} dalam bidang {} dan {}",
            sk.canonical_name, domain_clean, sk.category
        );

        let v1 = model.embed_with_prefix("passage: ", &p1)?;
        let v2 = model.embed_with_prefix("passage: ", &p2)?;
        let v3 = model.embed_with_prefix("passage: ", &p3)?;

        let acc = domain_accum
            .entry(sk.domain.clone())
            .or_insert_with(|| vec![0.0f32; dims]);
        for (dst, &src) in acc.iter_mut().zip(v1.iter()) {
            *dst += src;
        }

        skills_out.push(SkillIndexRecord {
            skill_id_str: sk.id.clone(),
            canonical_name: sk.canonical_name.clone(),
            domain_slug: sk.domain.clone(),
            category: sk.category.clone(),
            prototypes: vec![
                SemanticIndex::encode_vec(&v1),
                SemanticIndex::encode_vec(&v2),
                SemanticIndex::encode_vec(&v3),
            ],
        });

        if (idx + 1) % 100 == 0 || idx + 1 == all_skills.len() {
            println!("  encoded skills: {}/{}", idx + 1, all_skills.len());
        }
    }

    let mut roles_out = Vec::with_capacity(all_roles.len());
    for (idx, role) in all_roles.iter().enumerate() {
        let domain_clean = role.domain.replace('_', " ");
        let p1 = format!("{} ({}, {})", role.role_title, role.prodi_label, domain_clean);
        let mut combined_skills = role.mandatory_skills.clone();
        combined_skills.extend(role.preferred_skills.iter().take(6).cloned());
        let p2 = if combined_skills.is_empty() {
            format!("karir profesi {} jurusan {}", role.role_title, role.prodi_label)
        } else {
            format!("{}: {}", role.role_title, combined_skills.join(", "))
        };

        let v1 = model.embed_with_prefix("passage: ", &p1)?;
        let v2 = model.embed_with_prefix("passage: ", &p2)?;

        let acc = domain_accum
            .entry(role.domain.clone())
            .or_insert_with(|| vec![0.0f32; dims]);
        for (dst, &src) in acc.iter_mut().zip(v1.iter()) {
            *dst += src * 0.5;
        }

        roles_out.push(RoleIndexRecord {
            role_id: role.id.clone(),
            role_title: role.role_title.clone(),
            domain_slug: role.domain.clone(),
            prototypes: vec![
                SemanticIndex::encode_vec(&v1),
                SemanticIndex::encode_vec(&v2),
            ],
        });

        if (idx + 1) % 60 == 0 || idx + 1 == all_roles.len() {
            println!("  encoded roles: {}/{}", idx + 1, all_roles.len());
        }
    }

    let mut domains_out = Vec::with_capacity(domain_accum.len());
    let mut seen_domains: HashMap<String, usize> = HashMap::new();
    for (slug, mut vec) in domain_accum {
        if seen_domains.contains_key(&slug) {
            continue;
        }
        seen_domains.insert(slug.clone(), domains_out.len());
        l2_normalize_inplace(&mut vec);
        domains_out.push(DomainIndexRecord {
            domain_slug: slug,
            vector: SemanticIndex::encode_vec(&vec),
        });
    }

    let index = SemanticIndex {
        dims,
        skills: skills_out,
        roles: roles_out,
        domains: domains_out,
    };
    index.save_to_path(&vidx_path)?;
    update_manifest(&pack_dir, &vidx_path)?;

    let mb = std::fs::metadata(&vidx_path)
        .map(|m| m.len() as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);
    println!(
        "[build_semantic_index] Wrote {} ({:.2} MB, {} skills, {} roles, {} domains)",
        vidx_path.display(),
        mb,
        index.skills.len(),
        index.roles.len(),
        index.domains.len()
    );
    Ok(())
}
