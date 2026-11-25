#!/usr/bin/env python3
"""Download and quantize minishlab/potion-multilingual-128M (MIT) to Vecta's INT8 VEMB format.

Outputs ~142 MB of pure-Rust-readable offline assets into:
    apps/client/src-tauri/resources/models/potion-m128-i8/
        - embeddings_i8.vemb (~124 MB, 500,353 vocab x 256 dims)
        - tokenizer.json (~17.8 MB)
        - config.json

Requires Python 3.10+ and numpy (`pip install numpy` or `uv run --with numpy`).
"""

from __future__ import annotations

import json
import struct
import tempfile
import urllib.request
from pathlib import Path

import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent
MODEL_DIR = (
    REPO_ROOT
    / "apps"
    / "client"
    / "src-tauri"
    / "resources"
    / "models"
    / "potion-m128-i8"
)
HF_BASE = "https://huggingface.co/minishlab/potion-multilingual-128M/resolve/main"


def download(url: str, dest: Path) -> None:
    if dest.exists() and dest.stat().st_size > 1000:
        print(f"[SKIP] {dest.name} ({dest.stat().st_size / 1024 / 1024:.2f} MB)")
        return
    print(f"[DOWNLOAD] {url}")
    req = urllib.request.Request(url, headers={"User-Agent": "VectaModelFetcher/1.0"})
    with urllib.request.urlopen(req, timeout=300) as resp, open(dest, "wb") as out:
        while True:
            chunk = resp.read(1024 * 1024)
            if not chunk:
                break
            out.write(chunk)
    print(f"[OK] {dest.name} ({dest.stat().st_size / 1024 / 1024:.2f} MB)")


def main() -> None:
    MODEL_DIR.mkdir(parents=True, exist_ok=True)
    download(f"{HF_BASE}/tokenizer.json", MODEL_DIR / "tokenizer.json")
    download(f"{HF_BASE}/config.json", MODEL_DIR / "config.json")

    vemb_path = MODEL_DIR / "embeddings_i8.vemb"
    if vemb_path.exists() and vemb_path.stat().st_size > 10_000_000:
        print(f"[SKIP] {vemb_path.name} already exists ({vemb_path.stat().st_size / 1024 / 1024:.2f} MB)")
        return

    with tempfile.TemporaryDirectory(prefix="vecta_model_") as tmp_dir:
        raw_st = Path(tmp_dir) / "model.safetensors"
        download(f"{HF_BASE}/model.safetensors", raw_st)

        with open(raw_st, "rb") as f:
            header_len = struct.unpack("<Q", f.read(8))[0]
            header = json.loads(f.read(header_len).decode("utf-8"))
            data_offset = 8 + header_len
            tensor_key = next(k for k in header.keys() if k != "__metadata__")
            meta = header[tensor_key]
            dtype = meta["dtype"]
            shape = meta["shape"]
            start, end = meta["data_offsets"]
            print(f"[QUANTIZE] tensor={tensor_key} dtype={dtype} shape={shape}")
            f.seek(data_offset + start)
            raw_bytes = f.read(end - start)

        np_dtype = {"F32": np.float32, "F16": np.float16}[dtype]
        arr = np.frombuffer(raw_bytes, dtype=np_dtype).reshape(shape).astype(np.float32)
        vocab_size, dims = arr.shape

        max_abs = np.max(np.abs(arr), axis=1)
        scales = np.where(max_abs > 1e-8, max_abs / 127.0, 1.0).astype(np.float32)
        q = np.clip(np.round(arr / scales[:, None]), -127, 127).astype(np.int8)

        with open(vemb_path, "wb") as out:
            out.write(b"VEMB")
            out.write(struct.pack("<III", 1, vocab_size, dims))
            out.write(scales.tobytes())
            out.write(q.tobytes())

        print(
            f"[DONE] Built {vemb_path} ({vemb_path.stat().st_size / 1024 / 1024:.2f} MB) "
            f"vocab={vocab_size} dims={dims}"
        )


if __name__ == "__main__":
    main()
