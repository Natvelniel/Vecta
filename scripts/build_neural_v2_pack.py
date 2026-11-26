#!/usr/bin/env python3
"""Download and quantize Vecta Neural Engine v2 offline models into pure-Rust VTXR format.

Builds into apps/client/src-tauri/resources/models/neural-v2/:
  - tokenizer.json        (~17 MB, shared XLM-RoBERTa 250k vocabulary)
  - e5-base.vtxr          (~283 MB, intfloat/multilingual-e5-base 768-dim Bi-Encoder, mmap INT8 + Q8)
  - mmarco-reranker.vtxr  (~118 MB, cross-encoder/mmarco-mMiniLMv2-L12-H384 Reranker, mmap INT8 + Q8)
  - manifest.json         (versions, byte sizes, SHA-256 checksums)

Uses only Python stdlib + numpy (zero PyTorch/ONNX dependency).
"""

from __future__ import annotations

import hashlib
import json
import struct
import tempfile
import urllib.request
from pathlib import Path

import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = (
    REPO_ROOT
    / "apps"
    / "client"
    / "src-tauri"
    / "resources"
    / "models"
    / "neural-v2"
)

VTXR_MAGIC = b"VTXR"
VTXR_VERSION = 1
HEADER_SIZE = 64

MODELS = [
    {
        "repo": "intfloat/multilingual-e5-base",
        "out_name": "e5-base.vtxr",
        "arch_kind": 0,  # BiEncoder
        "prefix": "",
    },
    {
        "repo": "cross-encoder/mmarco-mMiniLMv2-L12-H384-v1",
        "out_name": "mmarco-reranker.vtxr",
        "arch_kind": 1,  # CrossEncoder
        "prefix": "roberta.",
    },
]


def download(url: str, dest: Path) -> None:
    if dest.exists() and dest.stat().st_size > 1000:
        print(f"[SKIP] {dest.name} ({dest.stat().st_size / 1024 / 1024:.2f} MB)")
        return
    print(f"[DOWNLOAD] {url}")
    req = urllib.request.Request(url, headers={"User-Agent": "VectaNeuralBuilder/2.0"})
    with urllib.request.urlopen(req, timeout=600) as resp, open(dest, "wb") as out:
        while True:
            chunk = resp.read(2 * 1024 * 1024)
            if not chunk:
                break
            out.write(chunk)
    print(f"[OK] {dest.name} ({dest.stat().st_size / 1024 / 1024:.2f} MB)")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while True:
            blk = f.read(4 * 1024 * 1024)
            if not blk:
                break
            h.update(blk)
    return h.hexdigest()


class SafetensorsReader:
    def __init__(self, path: Path) -> None:
        self._f = open(path, "rb")
        header_len = struct.unpack("<Q", self._f.read(8))[0]
        self._header = json.loads(self._f.read(header_len).decode("utf-8"))
        self._data_offset = 8 + header_len

    def close(self) -> None:
        self._f.close()

    def keys(self) -> set[str]:
        return {k for k in self._header.keys() if k != "__metadata__"}

    def read_f32(self, key: str) -> np.ndarray:
        meta = self._header[key]
        dtype = meta["dtype"]
        shape = meta["shape"]
        start, end = meta["data_offsets"]
        self._f.seek(self._data_offset + start)
        raw = self._f.read(end - start)
        if dtype == "F32":
            return np.frombuffer(raw, dtype=np.float32).reshape(shape).copy()
        if dtype == "F16":
            return np.frombuffer(raw, dtype=np.float16).reshape(shape).astype(np.float32)
        if dtype == "BF16":
            u16 = np.frombuffer(raw, dtype=np.uint16).astype(np.uint32) << 16
            return u16.view(np.float32).reshape(shape).copy()
        raise ValueError(f"Unsupported dtype {dtype} for {key}")


def quantize_rows_i8(arr: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    if arr.ndim == 1:
        arr = arr.reshape(1, -1)
    max_abs = np.max(np.abs(arr), axis=1)
    scales = np.where(max_abs > 1e-8, max_abs / 127.0, 1.0).astype(np.float32)
    q = np.clip(np.round(arr / scales[:, None]), -127, 127).astype(np.int8)
    return scales, q


def write_f32_rec(out, arr: np.ndarray) -> None:
    if arr.ndim == 1:
        rows, cols = 1, arr.shape[0]
    else:
        rows, cols = arr.shape[0], arr.shape[1]
    out.write(struct.pack("<BII", 0, rows, cols))
    out.write(arr.astype("<f4", copy=False).tobytes())


def write_i8_rec(out, arr: np.ndarray) -> None:
    rows, cols = arr.shape[0], arr.shape[1]
    scales, q = quantize_rows_i8(arr)
    out.write(struct.pack("<BII", 1, rows, cols))
    out.write(scales.astype("<f4", copy=False).tobytes())
    out.write(q.tobytes())


def convert_to_vtxr(st_path: Path, cfg: dict, dest_path: Path, arch_kind: int, prefix: str) -> None:
    reader = SafetensorsReader(st_path)
    try:
        available = reader.keys()
        actual_prefix = prefix if f"{prefix}embeddings.word_embeddings.weight" in available else ""

        def k(suffix: str) -> str:
            return f"{actual_prefix}{suffix}"

        vocab_size = int(cfg["vocab_size"])
        hidden = int(cfg["hidden_size"])
        num_layers = int(cfg["num_hidden_layers"])
        num_heads = int(cfg["num_attention_heads"])
        inter = int(cfg["intermediate_size"])
        max_pos = int(cfg["max_position_embeddings"])
        pad_id = int(cfg.get("pad_token_id", 1))
        eps = float(cfg.get("layer_norm_eps", 1e-5))

        word_emb = reader.read_f32(k("embeddings.word_embeddings.weight"))
        assert word_emb.shape == (vocab_size, hidden), f"Unexpected word_emb {word_emb.shape}"
        w_scales, w_q = quantize_rows_i8(word_emb)

        word_offset = HEADER_SIZE
        rest_offset = word_offset + w_scales.nbytes + w_q.nbytes

        tmp_out = dest_path.with_suffix(".vtxr.tmp")
        with open(tmp_out, "wb") as out:
            hdr = struct.pack(
                "<4sIIIIIIIIIfQQI",
                VTXR_MAGIC,
                VTXR_VERSION,
                arch_kind,
                vocab_size,
                hidden,
                num_layers,
                num_heads,
                inter,
                max_pos,
                pad_id,
                eps,
                word_offset,
                rest_offset,
                0,
            )
            assert len(hdr) == HEADER_SIZE
            out.write(hdr)
            out.write(w_scales.astype("<f4", copy=False).tobytes())
            out.write(w_q.tobytes())

            write_f32_rec(out, reader.read_f32(k("embeddings.position_embeddings.weight")))
            write_f32_rec(out, reader.read_f32(k("embeddings.token_type_embeddings.weight")))
            write_f32_rec(out, reader.read_f32(k("embeddings.LayerNorm.weight")))
            write_f32_rec(out, reader.read_f32(k("embeddings.LayerNorm.bias")))

            for i in range(num_layers):
                lp = f"encoder.layer.{i}."
                for proj in [
                    "attention.self.query",
                    "attention.self.key",
                    "attention.self.value",
                    "attention.output.dense",
                ]:
                    write_i8_rec(out, reader.read_f32(k(f"{lp}{proj}.weight")))
                    write_f32_rec(out, reader.read_f32(k(f"{lp}{proj}.bias")))

                write_f32_rec(out, reader.read_f32(k(f"{lp}attention.output.LayerNorm.weight")))
                write_f32_rec(out, reader.read_f32(k(f"{lp}attention.output.LayerNorm.bias")))

                write_i8_rec(out, reader.read_f32(k(f"{lp}intermediate.dense.weight")))
                write_f32_rec(out, reader.read_f32(k(f"{lp}intermediate.dense.bias")))
                write_i8_rec(out, reader.read_f32(k(f"{lp}output.dense.weight")))
                write_f32_rec(out, reader.read_f32(k(f"{lp}output.dense.bias")))

                write_f32_rec(out, reader.read_f32(k(f"{lp}output.LayerNorm.weight")))
                write_f32_rec(out, reader.read_f32(k(f"{lp}output.LayerNorm.bias")))

            if arch_kind == 1:
                write_i8_rec(out, reader.read_f32("classifier.dense.weight"))
                write_f32_rec(out, reader.read_f32("classifier.dense.bias"))
                write_f32_rec(out, reader.read_f32("classifier.out_proj.weight"))
                write_f32_rec(out, reader.read_f32("classifier.out_proj.bias"))

        tmp_out.replace(dest_path)
        print(f"[BUILT] {dest_path.name} ({dest_path.stat().st_size / 1024 / 1024:.2f} MB)")
    finally:
        reader.close()


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    tok_path = OUT_DIR / "tokenizer.json"
    download(
        "https://huggingface.co/intfloat/multilingual-e5-base/resolve/main/tokenizer.json",
        tok_path,
    )

    with tempfile.TemporaryDirectory(prefix="vecta_neural_v2_") as tmp_dir:
        tmp = Path(tmp_dir)
        for m in MODELS:
            dest = OUT_DIR / m["out_name"]
            if dest.exists() and dest.stat().st_size > 10_000_000:
                print(f"[SKIP] {dest.name} ({dest.stat().st_size / 1024 / 1024:.2f} MB)")
                continue
            repo = m["repo"]
            cfg_path = tmp / f"{m['out_name']}.config.json"
            st_path = tmp / f"{m['out_name']}.safetensors"
            download(f"https://huggingface.co/{repo}/resolve/main/config.json", cfg_path)
            download(f"https://huggingface.co/{repo}/resolve/main/model.safetensors", st_path)
            cfg = json.loads(cfg_path.read_text(encoding="utf-8"))
            print(f"[CONVERT] {repo} -> {m['out_name']}")
            convert_to_vtxr(st_path, cfg, dest, m["arch_kind"], m["prefix"])
            st_path.unlink(missing_ok=True)

    manifest = {
        "pack_version": 2,
        "engine": "vecta-neural-v2",
        "files": {},
    }
    for fname in ["tokenizer.json", "e5-base.vtxr", "mmarco-reranker.vtxr"]:
        fpath = OUT_DIR / fname
        if fpath.exists():
            manifest["files"][fname] = {
                "size_bytes": fpath.stat().st_size,
                "sha256": sha256_file(fpath),
            }
    (OUT_DIR / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    total_mb = sum(v["size_bytes"] for v in manifest["files"].values()) / (1024 * 1024)
    print(f"[DONE] Neural v2 pack ready in {OUT_DIR} ({total_mb:.2f} MB before semantic_index.vidx)")


if __name__ == "__main__":
    main()
