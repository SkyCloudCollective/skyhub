# RanchSamples worker

Audio analysis (BPM, key, timbre) + ingest into the catalog. Writes; the API
only reads. Same SQLite DB + canonical schema as the API.

## Run

```sh
uv sync
uv run python gen_samples.py   # synthesise royalty-free demo samples (numpy → WAV)
uv run python ingest.py        # scan RANCHSAMPLES_ROOT, analyse, upsert the catalog
uv run pytest                  # analyzer/audio unit tests
```

## Pieces

- `analyzer.py` — librosa features + Krumhansl-Schmuckler key + coarse category.
- `audio.py` — file metadata + waveform peaks (soundfile; no ffmpeg needed).
- `ingest.py` — scan → analyse → `db.upsert_sample` (provenance from path layout).
- `gen_samples.py` — deterministic synthetic demo set (kick/hat/arp/pad/groove).
- `db.py` — worker-side writes against `db/schema.sql`.
