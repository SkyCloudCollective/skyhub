"""origin gate — the shared catalog never lists or serves 'licensed' content.

'licensed' = a third-party / commercial pack a member merely collected. It stays
private to the member; the collective view (search / galaxy / facets / a sample /
its file) only ever shows their OWN ('original') work. `shared_only=False` is the
escape hatch for a future per-member private view.
"""
from __future__ import annotations

from app import catalog


def _insert_licensed(conn, contributor: str = "tev") -> int:
    conn.execute(
        "INSERT INTO samples(rel_path,filename,title,contributor,category,kind,instrument,"
        "bpm,duration_ms,brightness,percussiveness,noisiness,loudness,origin) VALUES("
        "'contributors/tev/Packs/IS/AR/commercial_loop.wav','commercial_loop.wav',"
        "'Commercial Loop',?,'loops','loop','synth',120.0,2000,3000.0,0.5,0.3,0.4,'licensed')",
        (contributor,),
    )
    conn.commit()
    return conn.execute("SELECT id FROM samples WHERE origin='licensed'").fetchone()[0]


def test_search_excludes_licensed_by_default(wconn):
    lic = _insert_licensed(wconn)
    hits = catalog.search(wconn)["hits"]
    assert all(h["id"] != lic for h in hits)
    assert all(h["origin"] == "original" for h in hits)


def test_search_can_include_licensed_when_explicit(wconn):
    lic = _insert_licensed(wconn)
    ids = [h["id"] for h in catalog.search(wconn, shared_only=False)["hits"]]
    assert lic in ids


def test_galaxy_excludes_licensed(wconn):
    lic = _insert_licensed(wconn)
    assert all(p["id"] != lic for p in catalog.galaxy(wconn))


def test_get_sample_hides_licensed(wconn):
    lic = _insert_licensed(wconn)
    assert catalog.get_sample(wconn, lic) is None
    assert catalog.get_sample(wconn, lic, shared_only=False) is not None


def test_resolve_file_refuses_to_serve_licensed(wconn):
    lic = _insert_licensed(wconn)
    assert catalog.resolve_file(wconn, lic) is None
    assert catalog.resolve_file(wconn, lic, shared_only=False) is not None


def test_facets_drop_a_licensed_only_contributor(wconn):
    _insert_licensed(wconn, contributor="ghostpacks")
    assert "ghostpacks" not in catalog.facets(wconn)["contributors"]
    assert "ghostpacks" in catalog.facets(wconn, shared_only=False)["contributors"]
