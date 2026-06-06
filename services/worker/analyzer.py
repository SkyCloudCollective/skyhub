"""analyzer.py — audio feature analysis (BPM, key, timbre, category).

librosa does the heavy lifting; the key detector is a Krumhansl-Schmuckler
template match over the mean chroma. Returns a dict of catalog columns.
"""
from __future__ import annotations

import numpy as np

# Krumhansl-Schmuckler major/minor key profiles.
_MAJOR = np.array([6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88])
_MINOR = np.array([6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17])
_NOTES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]


def _detect_key(chroma_mean: np.ndarray) -> str | None:
    if chroma_mean.sum() <= 0:
        return None
    c = chroma_mean / (np.linalg.norm(chroma_mean) + 1e-9)
    best_score, best = -2.0, None
    for i in range(12):
        maj = np.roll(_MAJOR, i)
        minr = np.roll(_MINOR, i)
        for prof, suffix in ((maj, "maj"), (minr, "m")):
            p = prof / np.linalg.norm(prof)
            score = float(np.dot(c, p))
            if score > best_score:
                best_score, best = score, f"{_NOTES[i]}{suffix}"
    return best


def _categorize(duration_s: float, percussiveness: float, has_beat: bool) -> tuple[str, str | None, str]:
    """Return (category, instrument, kind) from coarse features."""
    if duration_s < 1.6:
        kind = "oneshot"
        # crude instrument guess by percussiveness (refined later / by user tags)
        instrument = "perc" if percussiveness > 0.55 else "synth"
        category = "drumkits" if percussiveness > 0.55 else "one-shots"
    elif has_beat:
        kind, instrument, category = "loop", None, "loops"
    else:
        kind, instrument, category = "loop", "pad", "melodies"
    return category, instrument, kind


def analyze(path: str) -> dict:
    import librosa  # local import: heavy (numba); keep module import cheap

    y, sr = librosa.load(path, sr=None, mono=True)
    if y.size == 0:
        return {"duration_ms": 0}
    duration_s = float(len(y) / sr)

    tempo, _ = librosa.beat.beat_track(y=y, sr=sr)
    bpm = float(np.atleast_1d(tempo)[0])
    has_beat = 60.0 <= bpm <= 200.0 and duration_s >= 1.6

    centroid = float(np.mean(librosa.feature.spectral_centroid(y=y, sr=sr)))
    flatness = float(np.mean(librosa.feature.spectral_flatness(y=y)))
    rms = float(np.mean(librosa.feature.rms(y=y)))

    harm, perc = librosa.effects.hpss(y)
    perc_energy = float(np.sum(perc**2))
    total_energy = float(np.sum(harm**2) + perc_energy) + 1e-12
    percussiveness = perc_energy / total_energy

    chroma_mean = np.mean(librosa.feature.chroma_cqt(y=y, sr=sr), axis=1)
    key = _detect_key(chroma_mean) if not (percussiveness > 0.7) else None

    category, instrument, kind = _categorize(duration_s, percussiveness, has_beat)

    return {
        "duration_ms": int(duration_s * 1000),
        "bpm": round(bpm, 2) if has_beat else None,
        "bpm_source": "analyzed",
        "musical_key": key,
        "brightness": round(centroid, 2),
        "noisiness": round(flatness, 4),
        "percussiveness": round(percussiveness, 4),
        "loudness": round(rms, 4),
        "category": category,
        "instrument": instrument,
        "kind": kind,
    }


def auto_tags(feat: dict) -> list[str]:
    """Small set of transparent auto-tags derived from features."""
    tags: list[str] = []
    if feat.get("kind"):
        tags.append(feat["kind"])
    if (feat.get("percussiveness") or 0) > 0.55:
        tags.append("percussive")
    elif (feat.get("noisiness") or 0) > 0.4:
        tags.append("noisy")
    else:
        tags.append("tonal")
    if (feat.get("brightness") or 0) > 4000:
        tags.append("bright")
    elif (feat.get("brightness") or 0) < 1200:
        tags.append("dark")
    return tags
