"""gen_samples.py — synthesise a handful of royalty-free demo samples.

Pure numpy → WAV (no external assets, no copyright). Enough variety to exercise
the analyzer (one-shots vs loops, percussive vs tonal, with/without a key) and
to populate the local library for development.

Run:  uv run python gen_samples.py
"""
from __future__ import annotations

import pathlib

import numpy as np
import soundfile as sf

from db import SAMPLES_ROOT

SR = 44100


def _env(n: int, attack: float = 0.005, release: float = 0.2) -> np.ndarray:
    a = int(attack * SR)
    r = int(release * SR)
    e = np.ones(n, dtype=np.float32)
    if a:
        e[:a] = np.linspace(0, 1, a)
    if r and r < n:
        e[-r:] = np.linspace(1, 0, r)
    return e


def _write(rel: str, y: np.ndarray) -> pathlib.Path:
    y = np.clip(y, -1.0, 1.0).astype(np.float32)
    path = SAMPLES_ROOT / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(path, y, SR, subtype="PCM_16")
    return path


def kick() -> np.ndarray:
    n = int(0.6 * SR)
    t = np.arange(n) / SR
    freq = 120 * np.exp(-t * 18) + 45
    phase = 2 * np.pi * np.cumsum(freq) / SR
    body = np.sin(phase) * np.exp(-t * 6)
    click = np.random.randn(n).astype(np.float32) * np.exp(-t * 120) * 0.3
    return (body + click) * 0.9


def hat() -> np.ndarray:
    n = int(0.18 * SR)
    noise = np.random.randn(n).astype(np.float32)
    # crude high-pass: difference + decay
    hp = np.diff(noise, prepend=0.0)
    return hp * _env(n, 0.001, 0.12) * 0.5


def arp_loop(bpm: float, root_hz: float, semis: list[int]) -> np.ndarray:
    beat = 60.0 / bpm
    step = beat / 2  # eighth notes
    steps = 16
    n = int(steps * step * SR)
    y = np.zeros(n, dtype=np.float32)
    for i in range(steps):
        semi = semis[i % len(semis)]
        f = root_hz * (2 ** (semi / 12.0))
        s0 = int(i * step * SR)
        ln = int(step * SR)
        t = np.arange(ln) / SR
        voice = (np.sin(2 * np.pi * f * t) + 0.5 * np.sin(2 * np.pi * 2 * f * t)) * _env(ln, 0.004, step * 0.6)
        y[s0:s0 + ln] += voice.astype(np.float32) * 0.4
    return y


def pad(root_hz: float, dur_s: float = 4.0) -> np.ndarray:
    n = int(dur_s * SR)
    t = np.arange(n) / SR
    y = np.zeros(n, dtype=np.float32)
    for mult, amp in ((1.0, 0.5), (1.5, 0.25), (2.0, 0.15), (3.0, 0.08)):
        lfo = 1 + 0.005 * np.sin(2 * np.pi * 0.2 * t)
        y += amp * np.sin(2 * np.pi * root_hz * mult * t * lfo)
    return (y * _env(n, 0.6, 1.2)).astype(np.float32)


def groove(bpm: float = 90.0) -> np.ndarray:
    beat = 60.0 / bpm
    bars = 2
    n = int(bars * 4 * beat * SR)
    y = np.zeros(n, dtype=np.float32)
    k, h = kick(), hat()
    for b in range(bars * 4):
        pos = int(b * beat * SR)
        y[pos:pos + len(k)] += k[: max(0, len(y) - pos)]
        hp = int((b + 0.5) * beat * SR)
        y[hp:hp + len(h)] += h[: max(0, len(y) - hp)]
    return y * 0.8


def main() -> int:
    np.random.seed(7)  # deterministic demo set
    written = [
        _write("packs/aurora/kick_aurora.wav", kick()),
        _write("packs/aurora/hat_aurora.wav", hat()),
        _write("packs/aurora/arp_dawn_124_Am.wav", arp_loop(124, 220.0, [0, 3, 7, 12, 7, 3])),
        _write("contributors/tev/pad_glass_C.wav", pad(130.81)),
        _write("contributors/ama/groove_90.wav", groove(90)),
    ]
    for p in written:
        print(f"  wrote {p.relative_to(SAMPLES_ROOT)}")
    print(f"{len(written)} demo samples in {SAMPLES_ROOT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
