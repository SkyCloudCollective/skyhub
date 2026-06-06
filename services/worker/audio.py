"""audio.py — file metadata + waveform peaks (soundfile, no ffmpeg needed)."""
from __future__ import annotations

import pathlib

import numpy as np
import soundfile as sf

_SUBTYPE_BITS = {
    "PCM_16": 16, "PCM_24": 24, "PCM_32": 32, "PCM_S8": 8, "PCM_U8": 8,
    "FLOAT": 32, "DOUBLE": 64,
}


def metadata(path: str) -> dict:
    info = sf.info(path)
    return {
        "samplerate": info.samplerate,
        "channels": info.channels,
        "bitdepth": _SUBTYPE_BITS.get(info.subtype, None),
        "bytes": pathlib.Path(path).stat().st_size,
        "original_format": pathlib.Path(path).suffix.lstrip(".").lower() or None,
    }


def peaks(path: str, buckets: int = 400) -> list[list[float]]:
    """Downsample to [[min,max], ...] for a compact waveform canvas."""
    y, _ = sf.read(path, dtype="float32", always_2d=True)
    mono = y.mean(axis=1)
    n = mono.shape[0]
    if n == 0:
        return []
    buckets = max(1, min(buckets, n))
    edges = np.linspace(0, n, buckets + 1, dtype=int)
    out: list[list[float]] = []
    for i in range(buckets):
        seg = mono[edges[i]:edges[i + 1]]
        if seg.size == 0:
            out.append([0.0, 0.0])
        else:
            out.append([round(float(seg.min()), 4), round(float(seg.max()), 4)])
    return out
