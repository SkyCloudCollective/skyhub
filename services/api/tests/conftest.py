"""Shared fixtures: a fresh temp catalog seeded with a couple of samples."""
from __future__ import annotations

import pathlib
import wave

import pytest
from fastapi.testclient import TestClient

from app import db as dbmod
from app import main as mainmod
from app.db import connect, init_db
from app.main import app

# rel_paths seeded below; real (tiny) files are written here so preview/download
# have something to serve.
_SEED_FILES = ["packs/aurora/kick_01.wav", "contributors/ama/loop_dawn.wav"]


def _write_tiny_wav(path: pathlib.Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(44100)
        w.writeframes(b"\x00\x00" * 256)


def _seed(conn) -> None:
    rows = [
        # rel_path, filename, title, contributor, pack, category, kind, instrument, bpm, key
        ("packs/aurora/kick_01.wav", "kick_01.wav", "Aurora Kick 01", "tev", "aurora",
         "drumkits", "oneshot", "kick", 0.0, None),
        ("contributors/ama/loop_dawn.wav", "loop_dawn.wav", "Dawn Loop", "ama", None,
         "loops", "loop", "synth", 124.0, "Am"),
    ]
    for rp, fn, ti, co, pk, ca, ki, ins, bpm, key in rows:
        conn.execute(
            "INSERT INTO samples(rel_path,filename,title,contributor,pack,category,kind,"
            "instrument,bpm,musical_key,duration_ms,peaks_json) "
            "VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
            (rp, fn, ti, co, pk, ca, ki, ins, bpm, key, 1500, "[[0.0,0.1],[0.2,0.3]]"),
        )
    sid = conn.execute("SELECT id FROM samples WHERE filename='loop_dawn.wav'").fetchone()[0]
    conn.execute(
        "INSERT INTO tags(sample_id,tag,source,added_by) VALUES(?,?,?,?)",
        (sid, "ambient", "auto", None),
    )
    conn.commit()


@pytest.fixture()
def client(tmp_path: pathlib.Path, monkeypatch):
    db_file = tmp_path / "catalog.db"
    samples_root = tmp_path / "samples"
    monkeypatch.setattr(dbmod, "DB_PATH", db_file)
    monkeypatch.setattr(mainmod, "SAMPLES_ROOT", samples_root)
    for rel in _SEED_FILES:
        _write_tiny_wav(samples_root / rel)
    init_db(db_file)
    conn = connect(db_file)
    try:
        _seed(conn)
    finally:
        conn.close()
    with TestClient(app) as c:
        yield c
