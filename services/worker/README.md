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

### Real library (deploy)

`gen_samples.py` is dev/demo only. To serve the real curated library, import a
prior catalog — its rows + human/system tags are kept verbatim; the timbral
features are recomputed from the audio:

```sh
uv run python import_v1.py --src-db <prior catalog.db> --src-root <prior samples root> \
    --dest-db ../../data/deploy.db --dest-root ../../samples/library --replace
```

Then point the API at it: `RANCHSAMPLES_DB=data/deploy.db RANCHSAMPLES_ROOT=samples/library`.
Both paths are git-ignored — the real audio never lives in the repo.

## Pieces

- `analyzer.py` — librosa features + Krumhansl-Schmuckler key + coarse category.
- `audio.py` — file metadata + waveform peaks (soundfile; no ffmpeg needed).
- `ingest.py` — scan → analyse → `db.upsert_sample` (provenance from path layout).
- `gen_samples.py` — deterministic synthetic demo set (kick/hat/arp/pad/groove).
- `import_v1.py` — import a prior catalog (curated rows + tags) → real deploy library.
- `db.py` — worker-side writes against `db/schema.sql`.
