-- 0002_social_generic_subject — make comments/reactions subject-generic.
--
-- Forward-only. comments/reactions originally targeted only samples. RanchTube
-- needs them to target videos too, so we add a `subject_type` discriminator
-- (default 'sample', so every existing row keeps its meaning).
--
-- comments: PK is `id`, so a plain ADD COLUMN is enough.
-- reactions: the PK must include subject_type, otherwise sample #1 and video #1
-- would collide on (sample_id, handle, emoji). SQLite can't alter a PK in place,
-- so we rebuild the table (copy rows as subject_type='sample', then swap).

ALTER TABLE comments ADD COLUMN subject_type TEXT NOT NULL DEFAULT 'sample';
CREATE INDEX IF NOT EXISTS idx_comments_subject ON comments(subject_type, sample_id);

CREATE TABLE reactions_new (
    subject_type TEXT NOT NULL DEFAULT 'sample',   -- 'sample' | 'video'
    sample_id    INTEGER NOT NULL,                 -- subject id
    handle       TEXT NOT NULL,
    emoji        TEXT NOT NULL,
    created_at   TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (subject_type, sample_id, handle, emoji)
);
INSERT INTO reactions_new (subject_type, sample_id, handle, emoji, created_at)
    SELECT 'sample', sample_id, handle, emoji, created_at FROM reactions;
DROP TABLE reactions;
ALTER TABLE reactions_new RENAME TO reactions;
CREATE INDEX IF NOT EXISTS idx_reactions_subject ON reactions(subject_type, sample_id);
