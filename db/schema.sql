-- ============================================================================
-- RanchSamples — canonical SQLite schema (v2).
--
-- Single source of truth for the catalog AND the community layer. The API
-- (services/api) and the worker (services/worker) both read this. Categories
-- are TAGS, not physical folders; physical layout = packs/<slug> +
-- contributors/<handle> + incoming/<handle>.
--
-- Forward-only migrations live in db/migrations/ and are tracked in
-- schema_migrations. This file is the *current* shape (apply on a fresh DB);
-- existing/production DBs evolve via the numbered migrations.
--
-- Identity = Matrix handle (localpart). No separate account system.
-- Design choice (v2): favourites are NOT a table — they are a system
-- collection (collections.kind='favorites'), so "organise however you want"
-- and "favourite" share one model, one UI, one share mechanism.
-- ============================================================================

PRAGMA foreign_keys = ON;

-- ── catalog ──────────────────────────────────────────────────────────────--
CREATE TABLE IF NOT EXISTS samples (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    rel_path    TEXT UNIQUE NOT NULL,
    filename    TEXT NOT NULL,
    title       TEXT NOT NULL,
    contributor TEXT,                 -- handle (matrix localpart) or NULL
    pack        TEXT,                 -- pack slug or NULL
    category    TEXT,                 -- demos,beats,instrumentals,loops,one-shots,melodies,vocals,sfx,...
    kind        TEXT,                 -- 'oneshot' | 'loop' | 'fx' | 'file' | 'unknown'
    instrument  TEXT,                 -- 'kick','snare','hat','bass','synth','pad','vox','perc','fx',...
    bpm         REAL,
    musical_key TEXT,                 -- e.g. 'Am', 'Cmaj'
    bpm_source  TEXT DEFAULT 'filename',  -- 'filename' | 'analyzed'
    duration_ms INTEGER,
    brightness  REAL,                 -- spectral centroid (Hz)      — galaxy axis
    noisiness   REAL,                 -- spectral flatness 0..1       — galaxy axis
    percussiveness REAL,              -- HPSS percussive ratio 0..1   — galaxy axis
    loudness    REAL,                 -- RMS (0..~1)                  — galaxy axis
    samplerate  INTEGER,
    channels    INTEGER,
    bitdepth    INTEGER,
    bytes       INTEGER,
    sha256      TEXT,
    license     TEXT DEFAULT 'community',
    original_format TEXT,             -- source extension; the original is always kept
    preview_rel TEXT,                 -- transcoded preview path if original isn't browser-playable
    peaks_json  TEXT,                 -- compact [[min,max],...] for the waveform
    created_at  TEXT DEFAULT (datetime('now')),
    indexed_at  TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_samples_category    ON samples(category);
CREATE INDEX IF NOT EXISTS idx_samples_kind        ON samples(kind);
CREATE INDEX IF NOT EXISTS idx_samples_instrument  ON samples(instrument);
CREATE INDEX IF NOT EXISTS idx_samples_contributor ON samples(contributor);
CREATE INDEX IF NOT EXISTS idx_samples_pack        ON samples(pack);

CREATE TABLE IF NOT EXISTS tags (
    sample_id  INTEGER NOT NULL REFERENCES samples(id) ON DELETE CASCADE,
    tag        TEXT NOT NULL,
    source     TEXT NOT NULL DEFAULT 'system',   -- 'system' | 'auto' | 'user'
    added_by   TEXT,                              -- handle for user tags (transparency)
    created_at TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (sample_id, tag)
);
CREATE INDEX IF NOT EXISTS idx_tags_tag ON tags(tag);

CREATE TABLE IF NOT EXISTS projects (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    daw         TEXT,                 -- 'flstudio' | 'ableton' | 'other'
    handle      TEXT,                 -- contributor
    name        TEXT NOT NULL,
    rel_path    TEXT UNIQUE NOT NULL,
    main_file   TEXT,
    files_count INTEGER,
    bytes       INTEGER,
    notes       TEXT,
    created_at  TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_projects_daw    ON projects(daw);
CREATE INDEX IF NOT EXISTS idx_projects_handle ON projects(handle);

-- ── community: profiles + follows ───────────────────────────────────────────
CREATE TABLE IF NOT EXISTS profiles (
    handle       TEXT PRIMARY KEY,    -- matches auth subject (clean handle)
    display_name TEXT,
    bio          TEXT,
    avatar_path  TEXT,                -- uploaded asset, served like previews
    links_json   TEXT,                -- [{label,url}] (public only after legal pass)
    created_at   TEXT DEFAULT (datetime('now')),
    updated_at   TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS follows (
    follower   TEXT NOT NULL,
    followed   TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (follower, followed)
);
CREATE INDEX IF NOT EXISTS idx_follows_followed ON follows(followed);

-- ── community: collections (crates / favourites / smart) + membership ───────-
-- A user organises samples however they want: nestable crates, a system
-- 'favorites' crate, and 'smart' crates backed by a saved query.
CREATE TABLE IF NOT EXISTS collections (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    owner       TEXT NOT NULL,                       -- handle
    name        TEXT NOT NULL,
    slug        TEXT,                                 -- for shareable URL
    parent_id   INTEGER REFERENCES collections(id) ON DELETE CASCADE,  -- nesting
    kind        TEXT NOT NULL DEFAULT 'crate',        -- 'crate' | 'favorites' | 'smart'
    visibility  TEXT NOT NULL DEFAULT 'private',      -- 'private' | 'unlisted' | 'public'
    share_token TEXT,                                  -- for unlisted share links
    position    INTEGER NOT NULL DEFAULT 0,            -- manual ordering
    query_json  TEXT,                                  -- for kind='smart' (saved search)
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_collections_owner  ON collections(owner);
CREATE INDEX IF NOT EXISTS idx_collections_parent ON collections(parent_id);

CREATE TABLE IF NOT EXISTS collection_items (
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    sample_id     INTEGER NOT NULL REFERENCES samples(id) ON DELETE CASCADE,
    position      INTEGER NOT NULL DEFAULT 0,
    added_at      TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (collection_id, sample_id)
);

-- ── community: comments (threadable, editable) + reactions ──────────────────-
CREATE TABLE IF NOT EXISTS comments (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    sample_id  INTEGER NOT NULL REFERENCES samples(id) ON DELETE CASCADE,
    parent_id  INTEGER REFERENCES comments(id) ON DELETE CASCADE,   -- threads (v1 gap)
    handle     TEXT NOT NULL,
    body       TEXT NOT NULL,
    edited_at  TEXT,                                                 -- edit support (v1 gap)
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_comments_sample ON comments(sample_id);
CREATE INDEX IF NOT EXISTS idx_comments_parent ON comments(parent_id);

CREATE TABLE IF NOT EXISTS reactions (
    sample_id  INTEGER NOT NULL REFERENCES samples(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    emoji      TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (sample_id, handle, emoji)   -- one of each emoji per member per sample
);
CREATE INDEX IF NOT EXISTS idx_reactions_sample ON reactions(sample_id);

-- ── community: notifications (poll-based at this scale) ─────────────────────-
CREATE TABLE IF NOT EXISTS notifications (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    recipient    TEXT NOT NULL,        -- handle
    kind         TEXT NOT NULL,        -- 'comment'|'reaction'|'follow'|'collect'|'system'
    actor        TEXT,                 -- handle who caused it
    subject_type TEXT,                 -- 'sample'|'collection'|'profile'
    subject_id   INTEGER,
    data_json    TEXT,                 -- small render payload
    read_at      TEXT,                 -- NULL = unread
    created_at   TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_notif_recipient_unread ON notifications(recipient, read_at);

-- ── transparency: feedback + board + roadmap ────────────────────────────────
CREATE TABLE IF NOT EXISTS feedback (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    handle     TEXT,
    message    TEXT NOT NULL,
    context    TEXT,                 -- page/route it was sent from
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS feature_requests (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    handle     TEXT NOT NULL,
    title      TEXT NOT NULL,
    body       TEXT,
    status     TEXT NOT NULL DEFAULT 'open',        -- open|planned|in-progress|done|declined
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_freq_status ON feature_requests(status);

CREATE TABLE IF NOT EXISTS feature_votes (
    request_id INTEGER NOT NULL REFERENCES feature_requests(id) ON DELETE CASCADE,
    handle     TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (request_id, handle)              -- one vote per member per request
);

CREATE TABLE IF NOT EXISTS roadmap_entries (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT NOT NULL,
    body       TEXT,
    status     TEXT NOT NULL DEFAULT 'planned',     -- planned|in-progress|shipped|paused
    eta        TEXT,
    sort       INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_roadmap_status ON roadmap_entries(status);

-- ── migration bookkeeping (forward-only) ────────────────────────────────────
CREATE TABLE IF NOT EXISTS schema_migrations (
    version    TEXT PRIMARY KEY,     -- e.g. '0001_init'
    applied_at TEXT DEFAULT (datetime('now'))
);
