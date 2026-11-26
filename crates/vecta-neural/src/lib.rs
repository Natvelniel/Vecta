pub mod neural_xlmr;
pub mod semantic_index;

pub fn quantize_i8_vec(vec: &[f32]) -> (f32, Vec<i8>) {
    let max_abs = vec.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
    let scale = if max_abs > 1e-8 { max_abs / 127.0 } else { 1.0 };
    let inv = 1.0 / scale;
    let q = vec
        .iter()
        .map(|&v| (v * inv).round().clamp(-127.0, 127.0) as i8)
        .collect();
    (scale, q)
}
