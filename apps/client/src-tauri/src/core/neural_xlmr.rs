use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::Arc;

use candle_core::quantized::{GgmlDType, QMatMul, QTensor};
use candle_core::{DType, Device, Module, Tensor, D};
use memmap2::Mmap;
use tokenizers::Tokenizer;

const VTXR_MAGIC: &[u8; 4] = b"VTXR";
const VTXR_VERSION: u32 = 1;
const VTXR_HEADER_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VtxrArchKind {
    BiEncoder = 0,
    CrossEncoder = 1,
}

#[derive(Debug, Clone)]
pub struct VtxrMetadata {
    pub arch_kind: VtxrArchKind,
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub num_layers: usize,
    pub num_heads: usize,
    pub intermediate_size: usize,
    pub max_position_embeddings: usize,
    pub pad_token_id: usize,
    pub layer_norm_eps: f64,
    pub word_emb_offset: usize,
    pub rest_offset: usize,
}

struct MmapWordEmbedding {
    mmap: Mmap,
    scales_offset: usize,
    weights_offset: usize,
    vocab_size: usize,
    hidden_size: usize,
}

impl MmapWordEmbedding {
    fn lookup(&self, ids: &[u32], device: &Device) -> Result<Tensor, String> {
        let mut flat = Vec::with_capacity(ids.len() * self.hidden_size);
        let bytes = &self.mmap[..];
        for &raw_id in ids {
            let idx = (raw_id as usize).min(self.vocab_size.saturating_sub(1));
            let s_pos = self.scales_offset + idx * 4;
            let scale = f32::from_le_bytes([
                bytes[s_pos],
                bytes[s_pos + 1],
                bytes[s_pos + 2],
                bytes[s_pos + 3],
            ]);
            let w_pos = self.weights_offset + idx * self.hidden_size;
            let row = &bytes[w_pos..w_pos + self.hidden_size];
            for &b in row {
                flat.push(((b as i8) as f32) * scale);
            }
        }
        Tensor::from_vec(flat, (1, ids.len(), self.hidden_size), device)
            .map_err(|e| format!("Failed creating embedding tensor: {e}"))
    }
}

struct QuantizedLinear {
    weight: QMatMul,
    bias: Tensor,
}

impl QuantizedLinear {
    fn forward(&self, x: &Tensor) -> Result<Tensor, String> {
        let y = self
            .weight
            .forward(x)
            .map_err(|e| format!("QMatMul forward failed: {e}"))?;
        y.broadcast_add(&self.bias)
            .map_err(|e| format!("Linear bias add failed: {e}"))
    }
}

struct DenseLinear {
    weight: Tensor,
    bias: Tensor,
}

impl DenseLinear {
    fn forward(&self, x: &Tensor) -> Result<Tensor, String> {
        let wt = self.weight.t().map_err(|e| format!("transpose failed: {e}"))?;
        let y = x.matmul(&wt).map_err(|e| format!("matmul failed: {e}"))?;
        y.broadcast_add(&self.bias)
            .map_err(|e| format!("bias add failed: {e}"))
    }
}

struct LayerNormParams {
    weight: Tensor,
    bias: Tensor,
    eps: f64,
}

impl LayerNormParams {
    fn forward(&self, x: &Tensor) -> Result<Tensor, String> {
        candle_nn::ops::layer_norm(x, &self.weight, &self.bias, self.eps as f32)
            .map_err(|e| format!("LayerNorm failed: {e}"))
    }
}

struct XlmrLayer {
    q: QuantizedLinear,
    k: QuantizedLinear,
    v: QuantizedLinear,
    attn_out: QuantizedLinear,
    attn_ln: LayerNormParams,
    ffn_in: QuantizedLinear,
    ffn_out: QuantizedLinear,
    out_ln: LayerNormParams,
    num_heads: usize,
    head_dim: usize,
    scale: f64,
}

impl XlmrLayer {
    fn forward(&self, hidden: &Tensor) -> Result<Tensor, String> {
        let (_b, seq_len, hidden_size) = hidden
            .dims3()
            .map_err(|e| format!("hidden dims3 failed: {e}"))?;

        let q = self
            .q
            .forward(hidden)?
            .reshape((1, seq_len, self.num_heads, self.head_dim))
            .and_then(|t| t.transpose(1, 2))
            .and_then(|t| t.contiguous())
            .map_err(|e| format!("q reshape failed: {e}"))?;
        let k = self
            .k
            .forward(hidden)?
            .reshape((1, seq_len, self.num_heads, self.head_dim))
            .and_then(|t| t.transpose(1, 2))
            .and_then(|t| t.contiguous())
            .map_err(|e| format!("k reshape failed: {e}"))?;
        let v = self
            .v
            .forward(hidden)?
            .reshape((1, seq_len, self.num_heads, self.head_dim))
            .and_then(|t| t.transpose(1, 2))
            .and_then(|t| t.contiguous())
            .map_err(|e| format!("v reshape failed: {e}"))?;

        let kt = k.transpose(2, 3).map_err(|e| format!("k.t failed: {e}"))?;
        let scores = (q.matmul(&kt).map_err(|e| format!("qk matmul: {e}"))? * self.scale)
            .map_err(|e| format!("qk scale: {e}"))?;
        let probs = candle_nn::ops::softmax(&scores, D::Minus1)
            .map_err(|e| format!("attn softmax failed: {e}"))?;
        let ctx = probs
            .matmul(&v)
            .and_then(|t| t.transpose(1, 2))
            .and_then(|t| t.reshape((1, seq_len, hidden_size)))
            .map_err(|e| format!("attn context reshape: {e}"))?;

        let attn_proj = self.attn_out.forward(&ctx)?;
        let res1 = (hidden + &attn_proj).map_err(|e| format!("attn residual: {e}"))?;
        let norm1 = self.attn_ln.forward(&res1)?;

        let inter = self
            .ffn_in
            .forward(&norm1)?
            .gelu_erf()
            .map_err(|e| format!("gelu_erf failed: {e}"))?;
        let ffn_proj = self.ffn_out.forward(&inter)?;
        let res2 = (&norm1 + &ffn_proj).map_err(|e| format!("ffn residual: {e}"))?;
        self.out_ln.forward(&res2)
    }
}

fn read_u32_le(r: &mut impl Read) -> Result<u32, String> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|e| format!("EOF u32: {e}"))?;
    Ok(u32::from_le_bytes(b))
}

fn read_u8(r: &mut impl Read) -> Result<u8, String> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b).map_err(|e| format!("EOF u8: {e}"))?;
    Ok(b[0])
}

fn read_f32_tensor(r: &mut impl Read, shape: &[usize], device: &Device) -> Result<Tensor, String> {
    let tag = read_u8(r)?;
    if tag != 0 {
        return Err(format!("Expected F32 tensor tag 0, got {tag}"));
    }
    let rows = read_u32_le(r)? as usize;
    let cols = read_u32_le(r)? as usize;
    let total = rows * cols;
    let mut raw = vec![0u8; total * 4];
    r.read_exact(&mut raw)
        .map_err(|e| format!("EOF F32 tensor payload ({rows}x{cols}): {e}"))?;
    let mut floats = Vec::with_capacity(total);
    for ch in raw.chunks_exact(4) {
        floats.push(f32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]));
    }
    Tensor::from_vec(floats, shape, device)
        .map_err(|e| format!("Tensor::from_vec {:?} failed: {e}", shape))
}

fn read_q8_linear_weight(r: &mut impl Read, device: &Device) -> Result<QMatMul, String> {
    let tag = read_u8(r)?;
    if tag != 1 {
        return Err(format!("Expected INT8 linear tag 1, got {tag}"));
    }
    let rows = read_u32_le(r)? as usize;
    let cols = read_u32_le(r)? as usize;

    let mut scale_bytes = vec![0u8; rows * 4];
    r.read_exact(&mut scale_bytes)
        .map_err(|e| format!("EOF linear scales: {e}"))?;
    let mut weight_bytes = vec![0u8; rows * cols];
    r.read_exact(&mut weight_bytes)
        .map_err(|e| format!("EOF linear weights: {e}"))?;

    let mut f32_buf = Vec::with_capacity(rows * cols);
    for (r_idx, sc) in scale_bytes.chunks_exact(4).enumerate() {
        let scale = f32::from_le_bytes([sc[0], sc[1], sc[2], sc[3]]);
        let row_slice = &weight_bytes[r_idx * cols..(r_idx + 1) * cols];
        for &b in row_slice {
            f32_buf.push(((b as i8) as f32) * scale);
        }
    }

    let tmp_tensor = Tensor::from_vec(f32_buf, (rows, cols), device)
        .map_err(|e| format!("Temp tensor ({rows}x{cols}) failed: {e}"))?;
    let qtensor = QTensor::quantize(&tmp_tensor, GgmlDType::Q8_0)
        .map_err(|e| format!("QTensor Q8_0 quantize ({rows}x{cols}) failed: {e}"))?;
    QMatMul::from_qtensor(qtensor).map_err(|e| format!("QMatMul::from_qtensor failed: {e}"))
}

fn read_qlinear(
    r: &mut impl Read,
    out_dim: usize,
    in_dim: usize,
    device: &Device,
) -> Result<QuantizedLinear, String> {
    let weight = read_q8_linear_weight(r, device)?;
    let bias = read_f32_tensor(r, &[out_dim], device)?;
    let _ = in_dim;
    Ok(QuantizedLinear { weight, bias })
}

pub struct QuantizedXlmrModel {
    pub meta: VtxrMetadata,
    tokenizer: Arc<Tokenizer>,
    device: Device,
    word_emb: MmapWordEmbedding,
    pos_emb: Tensor,
    tok_type_0: Tensor,
    emb_ln: LayerNormParams,
    layers: Vec<XlmrLayer>,
    cls_dense: Option<QuantizedLinear>,
    cls_out: Option<DenseLinear>,
}

impl QuantizedXlmrModel {
    pub fn parse_header(path: &Path) -> Result<VtxrMetadata, String> {
        let mut f = File::open(path)
            .map_err(|e| format!("Cannot open VTXR {}: {e}", path.display()))?;
        let mut h = [0u8; VTXR_HEADER_BYTES];
        f.read_exact(&mut h)
            .map_err(|e| format!("Invalid VTXR header {}: {e}", path.display()))?;
        if &h[0..4] != VTXR_MAGIC {
            return Err("Invalid VTXR magic bytes".into());
        }
        let version = u32::from_le_bytes([h[4], h[5], h[6], h[7]]);
        if version != VTXR_VERSION {
            return Err(format!("Unsupported VTXR version {version}"));
        }
        let arch_raw = u32::from_le_bytes([h[8], h[9], h[10], h[11]]);
        let arch_kind = match arch_raw {
            0 => VtxrArchKind::BiEncoder,
            1 => VtxrArchKind::CrossEncoder,
            _ => return Err(format!("Unknown VTXR arch_kind {arch_raw}")),
        };
        let vocab_size = u32::from_le_bytes([h[12], h[13], h[14], h[15]]) as usize;
        let hidden_size = u32::from_le_bytes([h[16], h[17], h[18], h[19]]) as usize;
        let num_layers = u32::from_le_bytes([h[20], h[21], h[22], h[23]]) as usize;
        let num_heads = u32::from_le_bytes([h[24], h[25], h[26], h[27]]) as usize;
        let intermediate_size = u32::from_le_bytes([h[28], h[29], h[30], h[31]]) as usize;
        let max_position_embeddings = u32::from_le_bytes([h[32], h[33], h[34], h[35]]) as usize;
        let pad_token_id = u32::from_le_bytes([h[36], h[37], h[38], h[39]]) as usize;
        let layer_norm_eps = f32::from_le_bytes([h[40], h[41], h[42], h[43]]) as f64;
        let word_emb_offset =
            u64::from_le_bytes([h[44], h[45], h[46], h[47], h[48], h[49], h[50], h[51]]) as usize;
        let rest_offset =
            u64::from_le_bytes([h[52], h[53], h[54], h[55], h[56], h[57], h[58], h[59]]) as usize;

        if vocab_size == 0 || hidden_size == 0 || num_heads == 0 || hidden_size % num_heads != 0 {
            return Err("Invalid VTXR model geometry".into());
        }

        Ok(VtxrMetadata {
            arch_kind,
            vocab_size,
            hidden_size,
            num_layers,
            num_heads,
            intermediate_size,
            max_position_embeddings,
            pad_token_id,
            layer_norm_eps,
            word_emb_offset,
            rest_offset,
        })
    }

    pub fn load(vtxr_path: &Path, tokenizer: Arc<Tokenizer>) -> Result<Self, String> {
        let meta = Self::parse_header(vtxr_path)?;
        let device = Device::Cpu;
        let file = File::open(vtxr_path)
            .map_err(|e| format!("Cannot open VTXR {}: {e}", vtxr_path.display()))?;
        let mmap = unsafe {
            Mmap::map(&file).map_err(|e| format!("mmap failed for {}: {e}", vtxr_path.display()))?
        };
        let scales_offset = meta.word_emb_offset;
        let weights_offset = scales_offset + meta.vocab_size * 4;
        let needed_end = weights_offset + meta.vocab_size * meta.hidden_size;
        if mmap.len() < needed_end || mmap.len() < meta.rest_offset {
            return Err("Truncated VTXR file".into());
        }
        let word_emb = MmapWordEmbedding {
            mmap,
            scales_offset,
            weights_offset,
            vocab_size: meta.vocab_size,
            hidden_size: meta.hidden_size,
        };

        let stream_file = File::open(vtxr_path)
            .map_err(|e| format!("Cannot reopen VTXR for tensors: {e}"))?;
        let mut reader = BufReader::with_capacity(256 * 1024, stream_file);
        std::io::copy(&mut (&mut reader).take(meta.rest_offset as u64), &mut std::io::sink())
            .map_err(|e| format!("Seek rest_offset failed: {e}"))?;

        let h = meta.hidden_size;
        let inter = meta.intermediate_size;
        let pos_emb = read_f32_tensor(&mut reader, &[meta.max_position_embeddings, h], &device)?;
        let tok_type_all = read_f32_tensor(&mut reader, &[1, h], &device)?;
        let tok_type_0 = tok_type_all
            .narrow(0, 0, 1)
            .and_then(|t| t.unsqueeze(0))
            .map_err(|e| format!("tok_type_0 slice failed: {e}"))?;
        let emb_ln = LayerNormParams {
            weight: read_f32_tensor(&mut reader, &[h], &device)?,
            bias: read_f32_tensor(&mut reader, &[h], &device)?,
            eps: meta.layer_norm_eps,
        };

        let head_dim = h / meta.num_heads;
        let scale = 1.0 / (head_dim as f64).sqrt();
        let mut layers = Vec::with_capacity(meta.num_layers);
        for _ in 0..meta.num_layers {
            let q = read_qlinear(&mut reader, h, h, &device)?;
            let k = read_qlinear(&mut reader, h, h, &device)?;
            let v = read_qlinear(&mut reader, h, h, &device)?;
            let attn_out = read_qlinear(&mut reader, h, h, &device)?;
            let attn_ln = LayerNormParams {
                weight: read_f32_tensor(&mut reader, &[h], &device)?,
                bias: read_f32_tensor(&mut reader, &[h], &device)?,
                eps: meta.layer_norm_eps,
            };
            let ffn_in = read_qlinear(&mut reader, inter, h, &device)?;
            let ffn_out = read_qlinear(&mut reader, h, inter, &device)?;
            let out_ln = LayerNormParams {
                weight: read_f32_tensor(&mut reader, &[h], &device)?,
                bias: read_f32_tensor(&mut reader, &[h], &device)?,
                eps: meta.layer_norm_eps,
            };
            layers.push(XlmrLayer {
                q,
                k,
                v,
                attn_out,
                attn_ln,
                ffn_in,
                ffn_out,
                out_ln,
                num_heads: meta.num_heads,
                head_dim,
                scale,
            });
        }

        let (cls_dense, cls_out) = if meta.arch_kind == VtxrArchKind::CrossEncoder {
            let dense = read_qlinear(&mut reader, h, h, &device)?;
            let out_w = read_f32_tensor(&mut reader, &[1, h], &device)?;
            let out_b = read_f32_tensor(&mut reader, &[1], &device)?;
            (
                Some(dense),
                Some(DenseLinear {
                    weight: out_w,
                    bias: out_b,
                }),
            )
        } else {
            (None, None)
        };

        Ok(Self {
            meta,
            tokenizer,
            device,
            word_emb,
            pos_emb,
            tok_type_0,
            emb_ln,
            layers,
            cls_dense,
            cls_out,
        })
    }

    fn forward_sequence(&self, ids: &[u32]) -> Result<Tensor, String> {
        if ids.is_empty() {
            return Err("Empty token sequence".into());
        }
        let max_usable_pos = self
            .meta
            .max_position_embeddings
            .saturating_sub(self.meta.pad_token_id + 1);
        let seq_len = ids.len().min(max_usable_pos.max(1));
        let clipped_ids = &ids[..seq_len];

        let word = self.word_emb.lookup(clipped_ids, &self.device)?;
        let pos_ids: Vec<u32> = (0..seq_len)
            .map(|i| {
                ((self.meta.pad_token_id + 1 + i)
                    .min(self.meta.max_position_embeddings.saturating_sub(1))) as u32
            })
            .collect();
        let pos_idx = Tensor::from_vec(pos_ids, seq_len, &self.device)
            .map_err(|e| format!("pos_idx tensor failed: {e}"))?;
        let pos = self
            .pos_emb
            .index_select(&pos_idx, 0)
            .and_then(|t| t.unsqueeze(0))
            .map_err(|e| format!("pos index_select failed: {e}"))?;

        let sum = word
            .add(&pos)
            .and_then(|t| t.broadcast_add(&self.tok_type_0))
            .map_err(|e| format!("embedding sum failed: {e}"))?;
        let mut hidden = self.emb_ln.forward(&sum)?;

        for layer in &self.layers {
            hidden = layer.forward(&hidden)?;
        }
        Ok(hidden)
    }

    fn encode_single_with_e5_prefix(&self, prefix: &str, text: &str) -> Option<Vec<u32>> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let formatted = format!("{prefix}{trimmed}");
        let enc = self.tokenizer.encode(formatted, true).ok()?;
        let mut ids = enc.get_ids().to_vec();
        if ids.is_empty() {
            return None;
        }
        let max_len = self
            .meta
            .max_position_embeddings
            .saturating_sub(self.meta.pad_token_id + 1)
            .min(512);
        if ids.len() > max_len && max_len >= 2 {
            let last_tok = *ids.last().unwrap_or(&2);
            ids.truncate(max_len - 1);
            ids.push(last_tok);
        }
        Some(ids)
    }

    pub fn hidden_size(&self) -> usize {
        self.meta.hidden_size
    }

    pub fn embed_with_prefix(&self, prefix: &str, text: &str) -> Result<Vec<f32>, String> {
        let ids = self
            .encode_single_with_e5_prefix(prefix, text)
            .ok_or_else(|| "Empty or untokenizable input".to_string())?;
        let hidden = self.forward_sequence(&ids)?;
        let mean = hidden
            .mean(1)
            .and_then(|t| t.squeeze(0))
            .and_then(|t| t.to_dtype(DType::F32))
            .map_err(|e| format!("mean pool failed: {e}"))?;
        let mut vec = mean
            .to_vec1::<f32>()
            .map_err(|e| format!("to_vec1 failed: {e}"))?;
        l2_normalize_inplace(&mut vec);
        Ok(vec)
    }

    pub fn score_pair(&self, query: &str, passage: &str) -> Result<f32, String> {
        if self.meta.arch_kind != VtxrArchKind::CrossEncoder {
            return Err("score_pair requires CrossEncoder VTXR".into());
        }
        let q_enc = self
            .tokenizer
            .encode(query.trim(), false)
            .map_err(|e| format!("tokenize query failed: {e}"))?;
        let p_enc = self
            .tokenizer
            .encode(passage.trim(), false)
            .map_err(|e| format!("tokenize passage failed: {e}"))?;

        let mut q_ids = q_enc.get_ids().to_vec();
        let mut p_ids = p_enc.get_ids().to_vec();
        q_ids.truncate(160);
        p_ids.truncate(220);

        let mut pair = Vec::with_capacity(4 + q_ids.len() + p_ids.len());
        pair.push(0u32); // <s>
        pair.extend_from_slice(&q_ids);
        pair.push(2u32); // </s>
        pair.push(2u32); // </s>
        pair.extend_from_slice(&p_ids);
        pair.push(2u32); // </s>

        let hidden = self.forward_sequence(&pair)?;
        let cls = hidden
            .narrow(1, 0, 1)
            .and_then(|t| t.squeeze(1))
            .map_err(|e| format!("CLS slice failed: {e}"))?;
        let dense = self
            .cls_dense
            .as_ref()
            .ok_or_else(|| "Missing cls_dense".to_string())?;
        let out = self
            .cls_out
            .as_ref()
            .ok_or_else(|| "Missing cls_out".to_string())?;

        let h1 = dense
            .forward(&cls)?
            .tanh()
            .map_err(|e| format!("cls tanh failed: {e}"))?;
        let logits = out.forward(&h1)?;
        let val = logits
            .squeeze(0)
            .and_then(|t| t.to_vec1::<f32>())
            .map_err(|e| format!("logits read failed: {e}"))?;
        let raw = *val.first().unwrap_or(&0.0);
        let prob = 1.0f32 / (1.0f32 + (-raw).exp());
        Ok(prob.clamp(0.0, 1.0))
    }
}

pub fn l2_normalize_inplace(vec: &mut [f32]) {
    let mut sq = 0.0f64;
    for &v in vec.iter() {
        let f = v as f64;
        sq += f * f;
    }
    let norm = sq.sqrt();
    if norm > 1e-8 && norm.is_finite() {
        let inv = (1.0 / norm) as f32;
        for v in vec.iter_mut() {
            *v *= inv;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn build_tiny_tokenizer(vocab_size: usize) -> Arc<Tokenizer> {
        let mut entries = vec![
            r#""<s>":0"#.to_string(),
            r#""<pad>":1"#.to_string(),
            r#""</s>":2"#.to_string(),
            r#""<unk>":3"#.to_string(),
            r#""query:":4"#.to_string(),
            r#""passage:":5"#.to_string(),
            r#""rust":6"#.to_string(),
            r#""backend":7"#.to_string(),
            r#""akuntansi":8"#.to_string(),
        ];
        for i in 9..vocab_size {
            entries.push(format!(r#""tok_{i}":{i}"#));
        }
        let json = format!(
            r#"{{"version":"1.0","truncation":null,"padding":null,"added_tokens":[],"normalizer":null,"pre_tokenizer":{{"type":"WhitespaceSplit"}},"post_processor":null,"decoder":null,"model":{{"type":"WordLevel","vocab":{{{}}},"unk_token":"<unk>"}}}}"#,
            entries.join(",")
        );
        let tok = Tokenizer::from_bytes(json.as_bytes()).expect("tiny wordlevel tokenizer");
        Arc::new(tok)
    }

    fn write_f32_rec(buf: &mut Vec<u8>, rows: usize, cols: usize, val: f32) {
        buf.push(0u8);
        buf.extend_from_slice(&(rows as u32).to_le_bytes());
        buf.extend_from_slice(&(cols as u32).to_le_bytes());
        for i in 0..(rows * cols) {
            let v = val + ((i % 7) as f32) * 0.01;
            buf.extend_from_slice(&v.to_le_bytes());
        }
    }

    fn write_i8_rec(buf: &mut Vec<u8>, rows: usize, cols: usize) {
        buf.push(1u8);
        buf.extend_from_slice(&(rows as u32).to_le_bytes());
        buf.extend_from_slice(&(cols as u32).to_le_bytes());
        for r in 0..rows {
            let s = 0.02f32 + ((r % 3) as f32) * 0.005;
            buf.extend_from_slice(&s.to_le_bytes());
        }
        for i in 0..(rows * cols) {
            let q: i8 = ((i % 21) as i8) - 10;
            buf.push(q as u8);
        }
    }

    fn write_synthetic_vtxr(path: &Path, arch_kind: VtxrArchKind) {
        let vocab_size = 64usize;
        let hidden = 32usize;
        let num_layers = 2usize;
        let num_heads = 4usize;
        let inter = 64usize;
        let max_pos = 32usize;
        let pad_id = 1usize;

        let mut word_sec = Vec::new();
        for v in 0..vocab_size {
            let s = 0.05f32 + ((v % 5) as f32) * 0.01;
            word_sec.extend_from_slice(&s.to_le_bytes());
        }
        for v in 0..vocab_size {
            for d in 0..hidden {
                let q = (((v * 7 + d) % 31) as i8) - 15;
                word_sec.push(q as u8);
            }
        }

        let mut rest = Vec::new();
        write_f32_rec(&mut rest, max_pos, hidden, 0.01);
        write_f32_rec(&mut rest, 1, hidden, 0.0);
        write_f32_rec(&mut rest, 1, hidden, 1.0);
        write_f32_rec(&mut rest, 1, hidden, 0.0);

        for _ in 0..num_layers {
            for _ in 0..4 {
                write_i8_rec(&mut rest, hidden, hidden);
                write_f32_rec(&mut rest, 1, hidden, 0.0);
            }
            write_f32_rec(&mut rest, 1, hidden, 1.0);
            write_f32_rec(&mut rest, 1, hidden, 0.0);
            write_i8_rec(&mut rest, inter, hidden);
            write_f32_rec(&mut rest, 1, inter, 0.0);
            write_i8_rec(&mut rest, hidden, inter);
            write_f32_rec(&mut rest, 1, hidden, 0.0);
            write_f32_rec(&mut rest, 1, hidden, 1.0);
            write_f32_rec(&mut rest, 1, hidden, 0.0);
        }

        if arch_kind == VtxrArchKind::CrossEncoder {
            write_i8_rec(&mut rest, hidden, hidden);
            write_f32_rec(&mut rest, 1, hidden, 0.0);
            write_f32_rec(&mut rest, 1, hidden, 0.05);
            write_f32_rec(&mut rest, 1, 1, 0.0);
        }

        let word_offset = VTXR_HEADER_BYTES as u64;
        let rest_offset = word_offset + word_sec.len() as u64;

        let mut hdr = Vec::with_capacity(VTXR_HEADER_BYTES);
        hdr.extend_from_slice(VTXR_MAGIC);
        hdr.extend_from_slice(&VTXR_VERSION.to_le_bytes());
        hdr.extend_from_slice(&(arch_kind as u32).to_le_bytes());
        hdr.extend_from_slice(&(vocab_size as u32).to_le_bytes());
        hdr.extend_from_slice(&(hidden as u32).to_le_bytes());
        hdr.extend_from_slice(&(num_layers as u32).to_le_bytes());
        hdr.extend_from_slice(&(num_heads as u32).to_le_bytes());
        hdr.extend_from_slice(&(inter as u32).to_le_bytes());
        hdr.extend_from_slice(&(max_pos as u32).to_le_bytes());
        hdr.extend_from_slice(&(pad_id as u32).to_le_bytes());
        hdr.extend_from_slice(&1e-5f32.to_le_bytes());
        hdr.extend_from_slice(&word_offset.to_le_bytes());
        hdr.extend_from_slice(&rest_offset.to_le_bytes());
        hdr.extend_from_slice(&[0u8; 4]);
        assert_eq!(hdr.len(), VTXR_HEADER_BYTES);

        let mut f = File::create(path).expect("create temp vtxr");
        f.write_all(&hdr).unwrap();
        f.write_all(&word_sec).unwrap();
        f.write_all(&rest).unwrap();
    }

    #[test]
    fn quantized_xlmr_biencoder_and_cross_encoder_run_pure_rust() {
        let dir = std::env::temp_dir().join("vecta_xlmr_test");
        std::fs::create_dir_all(&dir).unwrap();
        let bi_path = dir.join("bi.vtxr");
        let ce_path = dir.join("ce.vtxr");
        write_synthetic_vtxr(&bi_path, VtxrArchKind::BiEncoder);
        write_synthetic_vtxr(&ce_path, VtxrArchKind::CrossEncoder);

        let tok = build_tiny_tokenizer(64);
        let bi = QuantizedXlmrModel::load(&bi_path, Arc::clone(&tok)).expect("load bi");
        let v1 = bi.embed_with_prefix("query: ", "rust backend").expect("embed 1");
        let v2 = bi.embed_with_prefix("passage: ", "rust backend").expect("embed 2");
        let v3 = bi.embed_with_prefix("query: ", "akuntansi").expect("embed 3");
        assert_eq!(v1.len(), 32);
        let norm: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);
        assert_ne!(v1, v2);
        assert_ne!(v1, v3);

        let ce = QuantizedXlmrModel::load(&ce_path, tok).expect("load ce");
        let prob = ce.score_pair("rust backend", "backend engineer rust").expect("score_pair");
        assert!(prob.is_finite() && (0.0..=1.0).contains(&prob));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
