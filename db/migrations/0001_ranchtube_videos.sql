-- 0001_ranchtube_videos — RanchTube phase 1 (community video, embeds only).
--
-- Forward-only. Mirrors the `videos` block in db/schema.sql so that existing
-- DBs (created before RanchTube) gain the table on next boot. We store only the
-- validated (provider, video_ref); the embed URL is rebuilt app-side.
CREATE TABLE IF NOT EXISTS videos (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    handle      TEXT NOT NULL,
    title       TEXT NOT NULL,
    provider    TEXT NOT NULL,
    video_ref   TEXT NOT NULL,
    url         TEXT NOT NULL,
    description TEXT,
    category    TEXT,
    created_at  TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_videos_created ON videos(created_at);
CREATE INDEX IF NOT EXISTS idx_videos_handle  ON videos(handle);
