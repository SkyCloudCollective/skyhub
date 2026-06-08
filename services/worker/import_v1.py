"""import_v1.py — import a prior SkyHub catalog into this one.

A prior catalog carries curated metadata the analyser cannot re-derive:
category / kind / instrument / musical key, and the human + system tags that
shaped the library by hand. So we copy those rows and tags verbatim, copy the
audio into the destination samples root, and fill ONLY the timbral feature
columns (brightness / noisiness / percussiveness / loudness) the prior schema
predates, by running the worker analyser on the audio files.

Source paths are passed in — nothing is hardcoded — so the repo stays portable
and carries no operator-local path.

    uv run python import_v1.py --src-db <catalog.db> --src-root <samples-root> \
        [--dest-db data/deploy.db] [--dest-root samples/library] [--replace]

--replace wipes samples + tags first, to build a clean real-only library for a
deploy DB. Without it the import is idempotent (upsert by rel_path; each
sample's tags are rebuilt from the source).
"""
from __future__ import annotations

import argparse
import os
import pathlib
import shutil
import sqlite3
import sys

from db import classify_origin, connect, init_db

AUDIO_EXT = {".wav", ".flac", ".aiff", ".aif", ".ogg", ".mp3", ".m4a"}

# Recomputed from the audio, never copied from the source row.
_FEATURES = ("brightness", "noisiness", "percussiveness", "loudness")


def _columns(conn: sqlite3.Connection, table: str) -> list[str]:
    return [r[1] for r in conn.execute(f"PRAGMA table_info({table})")]


def _is_audio(rel: str) -> bool:
    return pathlib.PurePosixPath(rel).suffix.lower() in AUDIO_EXT


def main() -> int:
    ap = argparse.ArgumentParser(description="Import a prior catalog into this one.")
    ap.add_argument("--src-db", required=True, help="prior catalog.db")
    ap.add_argument("--src-root", required=True, help="prior samples root (holds the audio)")
    ap.add_argument("--dest-db", default=os.environ.get("RANCHSAMPLES_DB"),
                    help="destination DB (default: $RANCHSAMPLES_DB or data/catalog.db)")
    ap.add_argument("--dest-root", default=os.environ.get("RANCHSAMPLES_ROOT"),
                    help="destination samples root (default: $RANCHSAMPLES_ROOT)")
    ap.add_argument("--replace", action="store_true",
                    help="wipe samples + tags first (build a clean real-only library)")
    ap.add_argument("--no-features", action="store_true",
                    help="skip the analyser (leave timbral features NULL)")
    args = ap.parse_args()

    src_db = pathlib.Path(args.src_db).resolve()
    src_root = pathlib.Path(args.src_root).resolve()
    if not src_db.is_file():
        print(f"error: source DB not found: {src_db}", file=sys.stderr)
        return 2
    if not src_root.is_dir():
        print(f"error: source root not found: {src_root}", file=sys.stderr)
        return 2

    src = sqlite3.connect(src_db)
    src.row_factory = sqlite3.Row
    dest = connect(args.dest_db)        # honours --dest-db / $RANCHSAMPLES_DB
    init_db(dest)

    dest_root = pathlib.Path(args.dest_root).resolve() if args.dest_root else None
    if dest_root is None:
        # mirror the worker default (db.SAMPLES_ROOT) when not overridden
        from db import SAMPLES_ROOT
        dest_root = SAMPLES_ROOT.resolve()

    src_cols = _columns(src, "samples")
    dest_cols = set(_columns(dest, "samples"))
    # copy every shared column except the surrogate id and the recomputed features
    shared = [c for c in src_cols if c in dest_cols and c != "id" and c not in _FEATURES]

    if args.replace:
        dest.execute("DELETE FROM tags")
        dest.execute("DELETE FROM samples")
        dest.commit()
        print("replace: cleared samples + tags")

    analyze = None
    if not args.no_features:
        from analyzer import analyze as _analyze  # heavy (librosa/numba); import lazily
        analyze = _analyze

    # Always classify origin (a v1 source has no such column): own work vs a
    # collected commercial pack, so the import never floods the shared catalog.
    insert_cols = shared + ([] if "origin" in shared else ["origin"])
    placeholders = ",".join("?" for _ in insert_cols)
    updates = ",".join(f"{c}=excluded.{c}" for c in insert_cols if c != "rel_path")
    insert_sql = (
        f"INSERT INTO samples ({','.join(insert_cols)}) VALUES ({placeholders}) "
        f"ON CONFLICT(rel_path) DO UPDATE SET {updates}, indexed_at=datetime('now')"
    )
    feature_sql = (
        "UPDATE samples SET " + ",".join(f"{f}=?" for f in _FEATURES) + " WHERE id=?"
    )

    n_samples = n_tags = n_features = n_missing = 0
    for row in src.execute("SELECT * FROM samples ORDER BY rel_path"):
        rel = row["rel_path"]
        src_file = src_root / rel
        dst_file = dest_root / rel

        if src_file.is_file():
            dst_file.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src_file, dst_file)
        else:
            n_missing += 1
            print(f"  ! missing audio (metadata only): {rel}", file=sys.stderr)

        vals = [row[c] for c in shared] + ([] if "origin" in shared else [classify_origin(rel)])
        dest.execute(insert_sql, vals)
        sid = dest.execute("SELECT id FROM samples WHERE rel_path=?", (rel,)).fetchone()[0]
        n_samples += 1

        # recompute the timbral features from the copied audio
        if analyze is not None and _is_audio(rel) and dst_file.is_file():
            try:
                feat = analyze(str(dst_file))
                dest.execute(feature_sql, [feat.get(f) for f in _FEATURES] + [sid])
                n_features += 1
            except Exception as exc:  # an unreadable file shouldn't abort the import
                print(f"  ! analyse failed for {rel}: {exc}", file=sys.stderr)

        # rebuild this sample's tags verbatim from the source
        dest.execute("DELETE FROM tags WHERE sample_id=?", (sid,))
        for t in src.execute(
            "SELECT tag, source, added_by FROM tags WHERE sample_id=?", (row["id"],)
        ):
            dest.execute(
                "INSERT OR IGNORE INTO tags(sample_id,tag,source,added_by) VALUES(?,?,?,?)",
                (sid, t["tag"], t["source"], t["added_by"]),
            )
            n_tags += 1
        dest.commit()

    total = dest.execute("SELECT count(*) FROM samples").fetchone()[0]
    total_tags = dest.execute("SELECT count(*) FROM tags").fetchone()[0]
    src.close()
    dest.close()

    print(
        f"imported {n_samples} samples ({n_tags} tags, features on {n_features}"
        f"{f', {n_missing} missing audio' if n_missing else ''}) "
        f"→ {total} samples / {total_tags} tags total in {args.dest_db or 'default DB'}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
