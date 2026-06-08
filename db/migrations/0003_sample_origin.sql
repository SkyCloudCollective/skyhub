-- 0003_sample_origin — tell a member's OWN work apart from collected third-party
-- packs, so the shared library never redistributes commercial content.
--
-- Forward-only. `origin`:
--   'original' = the contributor's own work — safe to share in the collective view.
--   'licensed' = a third-party / commercial pack they merely collected — kept
--                private to the member, NEVER served in the shared catalog.
--
-- Default 'original' so member contributions are shareable unless declared
-- otherwise (the upload flow will make the declaration explicit). Backfill marks
-- anything under a `.../Packs/...` path as a collected pack (the heuristic that
-- separated Tev's 13k commercial-pack files from his originals).
ALTER TABLE samples ADD COLUMN origin TEXT NOT NULL DEFAULT 'original';
UPDATE samples SET origin = 'licensed' WHERE rel_path LIKE '%/Packs/%';
CREATE INDEX IF NOT EXISTS idx_samples_origin ON samples(origin);
