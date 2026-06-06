"""ingest.py — scan the samples root, analyse each file, upsert the catalog.

Physical layout maps to provenance:
  packs/<slug>/...          -> pack=<slug>
  contributors/<handle>/... -> contributor=<handle>
  incoming/<handle>/...     -> contributor=<handle> (unsorted)

Run:  uv run python ingest.py            # ingest the default samples root
      RANCHSAMPLES_ROOT=/path uv run python ingest.py
"""
from __future__ import annotations

import hashlib
import json
import pathlib
import sys

from db import SAMPLES_ROOT, connect, init_db, upsert_sample

AUDIO_EXT = {".wav", ".flac", ".aiff", ".aif", ".ogg", ".mp3", ".m4a"}


def _sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _provenance(rel: pathlib.PurePosixPath) -> dict:
    parts = rel.parts
    out: dict = {"pack": None, "contributor": None}
    if len(parts) >= 2 and parts[0] == "packs":
        out["pack"] = parts[1]
    elif len(parts) >= 2 and parts[0] in ("contributors", "incoming"):
        out["contributor"] = parts[1]
    return out


def _title(filename: str) -> str:
    stem = pathlib.Path(filename).stem.replace("_", " ").replace("-", " ").strip()
    return stem[:1].upper() + stem[1:] if stem else filename


def ingest_one(conn, root: pathlib.Path, path: pathlib.Path) -> int:
    from analyzer import analyze, auto_tags
    from audio import metadata, peaks

    rel = pathlib.PurePosixPath(path.relative_to(root).as_posix())
    feat = analyze(str(path))
    meta = metadata(str(path))
    prov = _provenance(rel)

    rec = {
        "rel_path": str(rel),
        "filename": path.name,
        "title": _title(path.name),
        "license": "community",
        "peaks_json": json.dumps(peaks(str(path))),
        "sha256": _sha256(path),
        **prov,
        **meta,
        **feat,
    }
    tags = [(t, "auto") for t in auto_tags(feat)]
    if rec.get("category"):
        tags.append((rec["category"], "system"))
    return upsert_sample(conn, rec, tags)


def main(argv: list[str]) -> int:
    root = pathlib.Path(argv[1]) if len(argv) > 1 else SAMPLES_ROOT
    if not root.exists():
        print(f"samples root not found: {root}", file=sys.stderr)
        return 1
    conn = connect()
    init_db(conn)
    files = sorted(p for p in root.rglob("*") if p.is_file() and p.suffix.lower() in AUDIO_EXT)
    print(f"ingesting {len(files)} file(s) from {root}")
    for p in files:
        try:
            sid = ingest_one(conn, root, p)
            print(f"  #{sid}  {p.relative_to(root)}")
        except Exception as e:  # noqa: BLE001 — report and continue the batch
            print(f"  SKIP {p.name}: {e}", file=sys.stderr)
    conn.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
