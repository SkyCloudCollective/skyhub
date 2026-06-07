"""The RanchMap timbre projection (/v1/galaxy): only samples whose timbre
features are analysed, with just the columns the scatter plot needs."""
from __future__ import annotations

from app import catalog

_FIELDS = {
    "id", "title", "contributor", "category", "bpm", "duration_ms",
    "brightness", "percussiveness", "noisiness", "loudness",
}


def test_galaxy_empty_without_features(client):
    # the seed samples have no timbre features computed -> nothing to plot
    assert client.get("/v1/galaxy").json() == []


def test_galaxy_returns_analysed_samples(wconn):
    wconn.execute(
        "INSERT INTO samples(rel_path,filename,title,contributor,category,kind,"
        "brightness,percussiveness,noisiness,loudness,bpm,duration_ms) "
        "VALUES('a.wav','a.wav','Bright Stab','tev','one-shots','oneshot',"
        "0.8,0.6,0.2,-9.0,128,500)"
    )
    wconn.commit()
    g = catalog.galaxy(wconn)
    assert len(g) == 1
    p = g[0]
    assert p["title"] == "Bright Stab"
    assert p["brightness"] == 0.8 and p["percussiveness"] == 0.6
    assert _FIELDS <= set(p.keys())


def test_galaxy_limit_is_bounded(wconn):
    # clamps to >= 1 without crashing on odd inputs
    assert isinstance(catalog.galaxy(wconn, limit=0), list)
    assert isinstance(catalog.galaxy(wconn, limit=10_000), list)
