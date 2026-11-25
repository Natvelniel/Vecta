use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokenizers::Tokenizer;

use crate::error::AppError;

pub const LOCAL_EMBEDDING_MODEL_ID: &str = "local:potion-m128-i8";
const VEMB_MAGIC: &[u8; 4] = b"VEMB";

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct LocalAiModelStatus {
    pub model_id: String,
    pub is_ready: bool,
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

/// Pure-Rust offline multilingual embedding runtime (zero ONNX Runtime, Android/iOS safe).
pub struct LocalEmbedder {
    inner: RwLock<Option<LoadedStaticModel>>,
}

impl Default for LocalEmbedder {
    fn default() -> Self {
        Self::empty()
    }
}

impl LocalEmbedder {
    pub fn empty() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }

    /// Probe bundled resource directories and load the INT8 multilingual model if present.
    pub fn discover_and_load(candidate_dirs: &[PathBuf]) -> Self {
        for dir in candidate_dirs {
            if dir.join("embeddings_i8.vemb").exists() && dir.join("tokenizer.json").exists() {
                match LoadedStaticModel::load_from_dir(dir) {
                    Ok(model) => {
                        eprintln!(
                            "[vecta::local_embedder] Loaded offline multilingual model ({} dims, {:.1} MB) from {}",
                            model.dims, model.size_mb, model.source_path
                        );
                        return Self {
                            inner: RwLock::new(Some(model)),
                        };
                    }
                    Err(e) => {
                        eprintln!("[vecta::local_embedder] Skipping {}: {e}", dir.display());
                    }
                }
            }
        }
        Self::empty()
    }

    pub fn reload_from_dir(&self, dir: &Path) -> Result<LocalAiModelStatus, AppError> {
        let model = LoadedStaticModel::load_from_dir(dir).map_err(AppError::validation_error)?;
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        *guard = Some(model);
        drop(guard);
        Ok(self.status())
    }

    pub fn is_ready(&self) -> bool {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    pub fn status(&self) -> LocalAiModelStatus {
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(m) => LocalAiModelStatus {
                model_id: LOCAL_EMBEDDING_MODEL_ID.into(),
                is_ready: true,
                dimensions: m.dims as u32,
                vocab_size: m.vocab_size as u32,
                size_mb: (m.size_mb * 10.0).round() / 10.0,
                source_path: Some(m.source_path.clone()),
            },
            None => LocalAiModelStatus {
                model_id: LOCAL_EMBEDDING_MODEL_ID.into(),
                is_ready: false,
                dimensions: 0,
                vocab_size: 0,
                size_mb: 0.0,
                source_path: None,
            },
        }
    }

    pub fn embed_one(&self, text: &str) -> Option<Vec<f32>> {
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        guard.as_ref()?.embed_text(text)
    }

    pub fn embed_batch(&self, texts: &[String]) -> Option<Vec<Option<Vec<f32>>>> {
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        let model = guard.as_ref()?;
        Some(texts.iter().map(|t| model.embed_text(t)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::matching::cosine_similarity;

    #[test]
    fn bundled_offline_multilingual_model_ranks_indonesian_cross_domain_semantics() {
        let model_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("models")
            .join("potion-m128-i8");
        if !model_dir.join("embeddings_i8.vemb").exists() {
            // Gracefully skip if offline asset pack hasn't been materialized yet
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
}
