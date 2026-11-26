use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use specta::Type;
use tokenizers::Tokenizer;

use crate::core::neural_xlmr::{QuantizedXlmrModel, VtxrArchKind};
use crate::core::semantic_index::SemanticIndex;
use crate::error::AppError;

pub const LOCAL_EMBEDDING_MODEL_ID: &str = "local:potion-m128-i8";
pub const NEURAL_V2_MODEL_ID: &str = "local:vecta-neural-v2";
const VEMB_MAGIC: &[u8; 4] = b"VEMB";

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct LocalAiModelStatus {
    pub model_id: String,
    pub engine_tier: String,
    pub is_ready: bool,
    pub reranker_ready: bool,
    pub semantic_index_ready: bool,
    pub dimensions: u32,
    pub vocab_size: u32,
    pub size_mb: f64,
    pub source_path: Option<String>,
}

struct LoadedStaticModel {
    tokenizer: Tokenizer,
    vocab_size: usize,
    dims: usize,
    scales: Vec<f32>,
    weights_i8: Vec<i8>,
    size_mb: f64,
    source_path: String,
}

impl LoadedStaticModel {
    fn load_from_dir(dir: &Path) -> Result<Self, String> {
        let tok_path = dir.join("tokenizer.json");
        let vemb_path = dir.join("embeddings_i8.vemb");
        if !tok_path.exists() || !vemb_path.exists() {
            return Err(format!(
                "Aset model offline belum lengkap di {}",
                dir.display()
            ));
        }

        let tokenizer = Tokenizer::from_file(&tok_path)
            .map_err(|e| format!("Gagal memuat tokenizer offline: {e}"))?;

        let mut file = File::open(&vemb_path)
            .map_err(|e| format!("Gagal membuka embeddings_i8.vemb: {e}"))?;
        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0)
            + tok_path.metadata().map(|m| m.len()).unwrap_or(0);

        let mut header = [0u8; 16];
        file.read_exact(&mut header)
            .map_err(|e| format!("Header VEMB tidak valid: {e}"))?;
        if &header[0..4] != VEMB_MAGIC {
            return Err("Magic bytes VEMB tidak cocok.".into());
        }
        let version = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        if version != 1 {
            return Err(format!("Versi VEMB {version} tidak didukung."));
        }
        let vocab_size =
            u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
        let dims = u32::from_le_bytes([header[12], header[13], header[14], header[15]]) as usize;
        if vocab_size == 0 || dims == 0 || dims > 4096 {
            return Err("Dimensi VEMB tidak valid.".into());
        }

        let mut scale_bytes = vec![0u8; vocab_size * 4];
        file.read_exact(&mut scale_bytes)
            .map_err(|e| format!("Gagal membaca scales VEMB: {e}"))?;
        let mut scales = Vec::with_capacity(vocab_size);
        for chunk in scale_bytes.chunks_exact(4) {
            scales.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }

        let mut raw_weights = vec![0u8; vocab_size * dims];
        file.read_exact(&mut raw_weights)
            .map_err(|e| format!("Gagal membaca weights VEMB: {e}"))?;
        let weights_i8: Vec<i8> = raw_weights.into_iter().map(|b| b as i8).collect();

        Ok(Self {
            tokenizer,
            vocab_size,
            dims,
            scales,
            weights_i8,
            size_mb: (file_len as f64) / (1024.0 * 1024.0),
            source_path: dir.display().to_string(),
        })
    }

    /// Mean-pool dequantized static token embeddings + L2 normalize (< 0.3 ms per text, pure Rust).
    fn embed_text(&self, text: &str) -> Option<Vec<f32>> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let encoding = self.tokenizer.encode(trimmed, false).ok()?;
        let ids = encoding.get_ids();
        if ids.is_empty() {
            return None;
        }

        let mut accum = vec![0.0f32; self.dims];
        let mut count = 0usize;
        for &id in ids {
            let idx = id as usize;
            if idx >= self.vocab_size {
                continue;
            }
            let scale = self.scales[idx];
            let row = &self.weights_i8[idx * self.dims..(idx + 1) * self.dims];
            for (dst, &q) in accum.iter_mut().zip(row.iter()) {
                *dst += (q as f32) * scale;
            }
            count += 1;
        }
        if count == 0 {
            return None;
        }

        let inv_count = 1.0 / (count as f32);
        let mut norm_sq = 0.0f64;
        for v in &mut accum {
            *v *= inv_count;
            let vf = *v as f64;
            norm_sq += vf * vf;
        }
        let norm = norm_sq.sqrt() as f32;
        if norm > 1e-8 && norm.is_finite() {
            for v in &mut accum {
                *v /= norm;
            }
        }
        Some(accum)
    }
}

#[derive(Debug, Clone)]
struct NeuralV2PackMeta {
    dir: PathBuf,
    dims: u32,
    vocab_size: u32,
    size_mb: f64,
    has_reranker: bool,
    has_semantic_index: bool,
}

fn probe_neural_v2_dir(dir: &Path) -> Option<NeuralV2PackMeta> {
    let tok_path = dir.join("tokenizer.json");
    let bi_path = dir.join("e5-base.vtxr");
    if !tok_path.exists() || !bi_path.exists() {
        return None;
    }

    let mut f = File::open(&bi_path).ok()?;
    let mut hdr = [0u8; 24];
    f.read_exact(&mut hdr).ok()?;
    if &hdr[0..4] != b"VTXR" {
        return None;
    }
    let ver = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]);
    let arch = u32::from_le_bytes([hdr[8], hdr[9], hdr[10], hdr[11]]);
    if ver != 1 || arch != VtxrArchKind::BiEncoder as u32 {
        return None;
    }
    let vocab_size = u32::from_le_bytes([hdr[12], hdr[13], hdr[14], hdr[15]]);
    let dims = u32::from_le_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]);

    let reranker_path = dir.join("mmarco-reranker.vtxr");
    let vidx_path = dir.join("semantic_index.vidx");
    let has_reranker = reranker_path.exists();
    let has_semantic_index = vidx_path.exists();

    let total_bytes: u64 = [&tok_path, &bi_path, &reranker_path, &vidx_path]
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
        .sum();

    Some(NeuralV2PackMeta {
        dir: dir.to_path_buf(),
        dims,
        vocab_size,
        size_mb: (total_bytes as f64) / (1024.0 * 1024.0),
        has_reranker,
        has_semantic_index,
    })
}

/// Pure-Rust offline multilingual Neural + static embedding runtime (zero ONNX Runtime, Android/iOS safe).
pub struct LocalEmbedder {
    static_inner: RwLock<Option<LoadedStaticModel>>,
    v2_meta: RwLock<Option<NeuralV2PackMeta>>,
    v2_tokenizer: RwLock<Option<Arc<Tokenizer>>>,
    v2_biencoder: RwLock<Option<Arc<QuantizedXlmrModel>>>,
    v2_semantic_index: RwLock<Option<Arc<SemanticIndex>>>,
    reranker_lock: Mutex<()>,
}

impl Default for LocalEmbedder {
    fn default() -> Self {
        Self::empty()
    }
}

impl LocalEmbedder {
    pub fn empty() -> Self {
        Self {
            static_inner: RwLock::new(None),
            v2_meta: RwLock::new(None),
            v2_tokenizer: RwLock::new(None),
            v2_biencoder: RwLock::new(None),
            v2_semantic_index: RwLock::new(None),
            reranker_lock: Mutex::new(()),
        }
    }

    /// Probe bundled resource directories and register Neural v2 (lazy transformer load) or fallback potion-m128-i8.
    pub fn discover_and_load(candidate_dirs: &[PathBuf]) -> Self {
        let mut found_v2: Option<NeuralV2PackMeta> = None;
        let mut found_static: Option<LoadedStaticModel> = None;

        for dir in candidate_dirs {
            if found_v2.is_none() {
                if let Some(meta) = probe_neural_v2_dir(dir) {
                    eprintln!(
                        "[vecta::local_embedder] Registered Neural v2 pack ({} dims, {:.1} MB, reranker={}, vidx={}) from {}",
                        meta.dims,
                        meta.size_mb,
                        meta.has_reranker,
                        meta.has_semantic_index,
                        meta.dir.display()
                    );
                    found_v2 = Some(meta);
                }
            }
            if found_static.is_none()
                && dir.join("embeddings_i8.vemb").exists()
                && dir.join("tokenizer.json").exists()
            {
                match LoadedStaticModel::load_from_dir(dir) {
                    Ok(model) => {
                        eprintln!(
                            "[vecta::local_embedder] Loaded static fallback model ({} dims, {:.1} MB) from {}",
                            model.dims, model.size_mb, model.source_path
                        );
                        found_static = Some(model);
                    }
                    Err(e) => {
                        eprintln!("[vecta::local_embedder] Skipping {}: {e}", dir.display());
                    }
                }
            }
        }

        let vidx_preloaded = found_v2.as_ref().and_then(|meta| {
            let p = meta.dir.join("semantic_index.vidx");
            if p.exists() {
                SemanticIndex::load_from_path(&p).ok().map(Arc::new)
            } else {
                None
            }
        });

        Self {
            static_inner: RwLock::new(found_static),
            v2_meta: RwLock::new(found_v2),
            v2_tokenizer: RwLock::new(None),
            v2_biencoder: RwLock::new(None),
            v2_semantic_index: RwLock::new(vidx_preloaded),
            reranker_lock: Mutex::new(()),
        }
    }

    pub fn reload_from_dir(&self, dir: &Path) -> Result<LocalAiModelStatus, AppError> {
        if let Some(meta) = probe_neural_v2_dir(dir) {
            let vidx = if meta.has_semantic_index {
                SemanticIndex::load_from_path(&meta.dir.join("semantic_index.vidx"))
                    .ok()
                    .map(Arc::new)
            } else {
                None
            };
            *self.v2_meta.write().unwrap_or_else(|e| e.into_inner()) = Some(meta);
            *self.v2_tokenizer.write().unwrap_or_else(|e| e.into_inner()) = None;
            *self.v2_biencoder.write().unwrap_or_else(|e| e.into_inner()) = None;
            *self.v2_semantic_index.write().unwrap_or_else(|e| e.into_inner()) = vidx;
            return Ok(self.status());
        }

        let model = LoadedStaticModel::load_from_dir(dir).map_err(AppError::validation_error)?;
        let mut guard = self.static_inner.write().unwrap_or_else(|e| e.into_inner());
        *guard = Some(model);
        drop(guard);
        Ok(self.status())
    }

    fn ensure_v2_tokenizer(&self) -> Result<Arc<Tokenizer>, String> {
        if let Some(tok) = self
            .v2_tokenizer
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .cloned()
        {
            return Ok(tok);
        }
        let dir = self
            .v2_meta
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|m| m.dir.clone())
            .ok_or_else(|| "Neural v2 pack is not configured".to_string())?;
        let tok = Arc::new(
            Tokenizer::from_file(dir.join("tokenizer.json"))
                .map_err(|e| format!("Gagal memuat tokenizer Neural v2: {e}"))?,
        );
        *self.v2_tokenizer.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&tok));
        Ok(tok)
    }

    fn ensure_v2_biencoder(&self) -> Result<Arc<QuantizedXlmrModel>, String> {
        if let Some(m) = self
            .v2_biencoder
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .cloned()
        {
            return Ok(m);
        }
        let dir = self
            .v2_meta
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|m| m.dir.clone())
            .ok_or_else(|| "Neural v2 pack is not configured".to_string())?;
        let tok = self.ensure_v2_tokenizer()?;
        let model = Arc::new(QuantizedXlmrModel::load(&dir.join("e5-base.vtxr"), tok)?);
        *self.v2_biencoder.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&model));
        Ok(model)
    }

    pub fn is_neural_v2(&self) -> bool {
        self.v2_meta
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    pub fn is_ready(&self) -> bool {
        self.is_neural_v2()
            || self
                .static_inner
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .is_some()
    }

    pub fn has_reranker(&self) -> bool {
        self.v2_meta
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|m| m.has_reranker)
    }

    pub fn semantic_index(&self) -> Option<Arc<SemanticIndex>> {
        if let Some(existing) = self
            .v2_semantic_index
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .cloned()
        {
            return Some(existing);
        }
        let meta = self
            .v2_meta
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .cloned()?;
        if !meta.has_semantic_index {
            return None;
        }
        let loaded = SemanticIndex::load_from_path(&meta.dir.join("semantic_index.vidx"))
            .ok()
            .map(Arc::new)?;
        *self
            .v2_semantic_index
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&loaded));
        Some(loaded)
    }

    pub fn status(&self) -> LocalAiModelStatus {
        if let Some(meta) = self
            .v2_meta
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            return LocalAiModelStatus {
                model_id: NEURAL_V2_MODEL_ID.into(),
                engine_tier: "neural_v2".into(),
                is_ready: true,
                reranker_ready: meta.has_reranker,
                semantic_index_ready: meta.has_semantic_index,
                dimensions: meta.dims,
                vocab_size: meta.vocab_size,
                size_mb: (meta.size_mb * 10.0).round() / 10.0,
                source_path: Some(meta.dir.display().to_string()),
            };
        }

        let guard = self.static_inner.read().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(m) => LocalAiModelStatus {
                model_id: LOCAL_EMBEDDING_MODEL_ID.into(),
                engine_tier: "potion_fallback".into(),
                is_ready: true,
                reranker_ready: false,
                semantic_index_ready: false,
                dimensions: m.dims as u32,
                vocab_size: m.vocab_size as u32,
                size_mb: (m.size_mb * 10.0).round() / 10.0,
                source_path: Some(m.source_path.clone()),
            },
            None => LocalAiModelStatus {
                model_id: LOCAL_EMBEDDING_MODEL_ID.into(),
                engine_tier: "unavailable".into(),
                is_ready: false,
                reranker_ready: false,
                semantic_index_ready: false,
                dimensions: 0,
                vocab_size: 0,
                size_mb: 0.0,
                source_path: None,
            },
        }
    }

    pub fn embed_query(&self, text: &str) -> Option<Vec<f32>> {
        if self.is_neural_v2() {
            if let Ok(model) = self.ensure_v2_biencoder() {
                if let Ok(v) = model.embed_with_prefix("query: ", text) {
                    return Some(v);
                }
            }
        }
        let guard = self.static_inner.read().unwrap_or_else(|e| e.into_inner());
        guard.as_ref()?.embed_text(text)
    }

    pub fn embed_passage(&self, text: &str) -> Option<Vec<f32>> {
        if self.is_neural_v2() {
            if let Ok(model) = self.ensure_v2_biencoder() {
                if let Ok(v) = model.embed_with_prefix("passage: ", text) {
                    return Some(v);
                }
            }
        }
        let guard = self.static_inner.read().unwrap_or_else(|e| e.into_inner());
        guard.as_ref()?.embed_text(text)
    }

    pub fn embed_one(&self, text: &str) -> Option<Vec<f32>> {
        self.embed_passage(text)
    }

    pub fn embed_batch(&self, texts: &[String]) -> Option<Vec<Option<Vec<f32>>>> {
        if !self.is_ready() {
            return None;
        }
        Some(texts.iter().map(|t| self.embed_passage(t)).collect())
    }

    /// Sequentially loads the 12-layer Cross-Encoder, scores all (query, passage) pairs, and drops weights immediately.
    pub fn rerank_pairs(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, String> {
        let meta = self
            .v2_meta
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .cloned()
            .ok_or_else(|| "Neural v2 reranker not available".to_string())?;
        if !meta.has_reranker {
            return Err("mmarco-reranker.vtxr missing in neural-v2 pack".into());
        }
        let _guard = self
            .reranker_lock
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tok = self.ensure_v2_tokenizer()?;
        let reranker = QuantizedXlmrModel::load(&meta.dir.join("mmarco-reranker.vtxr"), tok)?;
        let mut scores = Vec::with_capacity(passages.len());
        for p in passages {
            scores.push(reranker.score_pair(query, p)?);
        }
        drop(reranker);
        Ok(scores)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::matching::cosine_similarity;

    #[test]
    fn empty_embedder_reports_unavailable_status() {
        let embedder = LocalEmbedder::empty();
        let status = embedder.status();
        assert!(!status.is_ready);
        assert_eq!(status.engine_tier, "unavailable");
        assert!(!status.reranker_ready);
        assert!(!status.semantic_index_ready);
    }

    #[test]
    fn bundled_offline_multilingual_model_ranks_indonesian_cross_domain_semantics() {
        let model_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("models")
            .join("potion-m128-i8");
        if !model_dir.join("embeddings_i8.vemb").exists() {
            return;
        }
        let embedder = LocalEmbedder::discover_and_load(&[model_dir]);
        assert!(embedder.is_ready());
        let status = embedder.status();
        assert_eq!(status.dimensions, 256);

        let v_profile = embedder
            .embed_one("Sarjana Akuntansi berpengalaman menyusun laporan keuangan, pajak Brevet A/B, dan Accurate")
            .expect("profile embedding");
        let v_match = embedder
            .embed_one("Tax & Accounting Staff menyusun laporan keuangan PSAK, e-Faktur, dan rekonsiliasi bank")
            .expect("matching job embedding");
        let v_mismatch = embedder
            .embed_one("Barista Coffee Shop melayani pelanggan membuat espresso dan latte art")
            .expect("unrelated job embedding");

        let sim_match = cosine_similarity(&v_profile, &v_match).unwrap();
        let sim_mismatch = cosine_similarity(&v_profile, &v_mismatch).unwrap();
        assert!(
            sim_match > sim_mismatch + 0.15,
            "expected accounting match ({sim_match:.3}) >> barista ({sim_mismatch:.3})"
        );
    }

    #[test]
    fn bundled_neural_v2_pack_discovers_and_scores_if_present() {
        let v2_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("models")
            .join("neural-v2");
        if !v2_dir.join("e5-base.vtxr").exists() {
            return;
        }
        let embedder = LocalEmbedder::discover_and_load(&[v2_dir]);
        let status = embedder.status();
        assert!(status.is_ready);
        assert_eq!(status.engine_tier, "neural_v2");
        assert_eq!(status.dimensions, 768);
        assert!(status.reranker_ready);

        let q = embedder
            .embed_query("Fresh graduate IT suka bikin website dengan React dan TypeScript")
            .expect("query embedding");
        let p_good = embedder
            .embed_passage("Frontend Web Developer React TypeScript TailwindCSS")
            .expect("good passage");
        let p_bad = embedder
            .embed_passage("Kepala Gudang Logistik Forklift Manajemen Inventori")
            .expect("bad passage");
        let sim_good = cosine_similarity(&q, &p_good).unwrap();
        let sim_bad = cosine_similarity(&q, &p_bad).unwrap();
        let cal_good = crate::core::matching::calibrate_e5_cosine(sim_good as f64);
        let cal_bad = crate::core::matching::calibrate_e5_cosine(sim_bad as f64);
        assert!(
            sim_good > sim_bad + 0.03 && cal_good > cal_bad + 0.10,
            "expected good ({sim_good:.3} cal={cal_good:.3}) >> bad ({sim_bad:.3} cal={cal_bad:.3})"
        );
    }
}
