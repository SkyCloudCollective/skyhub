# Collaboration — "a GitHub of music"

A new pillar: members share demos, records, loops and **full DAW projects** (stems
+ samples + the .flp/.als), **synced** across collaborators like a cloud share,
with **private** projects (invite specific members) and **open** projects (free
collaboration). The app becomes the social / discovery / permission layer; a
proven file engine does the heavy sync.

## Principle: don't reinvent sync

True bidirectional file sync (conflict handling, partial transfer, versioning,
desktop clients on Mac/Win/Linux) is a solved, hard problem. We **reuse** it
rather than rebuild it. Two self-hostable engines the Ranch already runs:

- **Nextcloud** — group/shared folders give real desktop sync + file versioning +
  conflict resolution out of the box. This is exactly "comme un partage Nextcloud".
- **Forgejo** — gives literal *GitHub* semantics (branches, pull-requests, diff,
  history) for the project files that benefit from it (MIDI, project XML), with
  large audio via Git-LFS.

**Recommended architecture (hybrid):**
1. **Storage + sync = Nextcloud group folder per project.** Each collaborative
   project maps to a shared folder; collaborators are granted access; the desktop
   app (or the user's Nextcloud client) syncs it. Versioning & conflicts are
   Nextcloud-native.
2. **Collaboration layer = RanchSamples API + DB.** Projects, members & roles,
   invitations, open-join, an activity log (the "git log": who added/updated what,
   when), comments, and a files index (mirrored from the folder via WebDAV).
3. **"GitHub" semantics, phased.** Start with synced files + activity + Nextcloud
   file-versions surfaced as a timeline. Later: optional **Forgejo-backed repo per
   project** for true branch/PR/diff on MIDI & project files (audio via LFS) — for
   projects that want it. Most music projects only need sync + history; power users
   get the repo model.

## Data model (additive, SQLite)

```sql
CREATE TABLE collab_projects (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  owner       TEXT NOT NULL,            -- handle
  title       TEXT NOT NULL,
  slug        TEXT,
  description TEXT,
  kind        TEXT,                      -- 'track'|'loop-pack'|'album'|'remix'|...
  daw         TEXT,                      -- 'ableton'|'flstudio'|'bitwig'|'other'
  visibility  TEXT NOT NULL DEFAULT 'private', -- 'private'|'unlisted'|'open'
  storage     TEXT NOT NULL DEFAULT 'nextcloud', -- backend
  store_ref   TEXT,                      -- group-folder id / repo path (operator-set)
  created_at  TEXT DEFAULT (datetime('now')),
  updated_at  TEXT DEFAULT (datetime('now'))
);
CREATE TABLE project_members (
  project_id INTEGER NOT NULL REFERENCES collab_projects(id) ON DELETE CASCADE,
  handle     TEXT NOT NULL,
  role       TEXT NOT NULL DEFAULT 'editor',  -- 'owner'|'editor'|'viewer'
  invited_by TEXT,
  accepted_at TEXT,
  PRIMARY KEY (project_id, handle)
);
CREATE TABLE project_invites (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES collab_projects(id) ON DELETE CASCADE,
  handle     TEXT NOT NULL,             -- invited member (in-app, not email)
  role       TEXT NOT NULL DEFAULT 'editor',
  token      TEXT NOT NULL,
  status     TEXT NOT NULL DEFAULT 'pending', -- 'pending'|'accepted'|'declined'|'revoked'
  invited_by TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);
CREATE TABLE project_files (             -- indexed from the synced folder
  project_id INTEGER NOT NULL REFERENCES collab_projects(id) ON DELETE CASCADE,
  rel_path   TEXT NOT NULL,
  kind       TEXT,                       -- 'stem'|'loop'|'demo'|'record'|'project'|'sample'|'midi'
  bytes      INTEGER, sha256 TEXT,
  version    INTEGER DEFAULT 1,
  updated_by TEXT, updated_at TEXT DEFAULT (datetime('now')),
  PRIMARY KEY (project_id, rel_path)
);
CREATE TABLE project_activity (          -- the "git log"
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES collab_projects(id) ON DELETE CASCADE,
  actor TEXT, action TEXT,               -- 'add'|'update'|'remove'|'join'|'comment'|'release'
  target TEXT, data_json TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);
CREATE TABLE project_comments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES collab_projects(id) ON DELETE CASCADE,
  rel_path TEXT,                         -- nullable: comment on a file or the project
  handle TEXT NOT NULL, body TEXT NOT NULL,
  created_at TEXT DEFAULT (datetime('now'))
);
```

## Invitation & join flows

- **Private project**: owner invites a member by handle → `project_invites` row →
  the invitee sees it **in-app** (notifications) and accepts/declines → on accept,
  `project_members` row + folder access granted. **In-app + consented; never an
  auto-email or unsolicited message.**
- **Open project**: anyone may request to join, or contribute freely (per the
  owner's setting). Visible in a public projects browse.

## App surfaces

- `/projects` — browse open projects + your projects.
- `/project/[slug]` — files (by kind), members, activity timeline, comments,
  "open synced folder" (desktop), invite (owner), join/request (open).
- Desktop: the synced folder lives under the library root; the app opens it / shows
  sync status. Upload a stem → it lands in the folder → syncs to collaborators.

## Governance — the gates (non-negotiable)

- **Open / public collaboration = public user-generated content → BLOCKED until
  Amir's E1 legal pass** (license/consent/DMCA/GDPR/ToS). Ship **private +
  invite-only** first; the `open` visibility stays feature-flagged off until cleared.
- **Sending invitations** must stay **in-app, member-initiated, consented** — no
  auto-emails, no unsolicited messages to members.
- **Provisioning folder access / group folders** needs the storage admin token →
  operator-applied (JAUNE), not the agent. The app records intent; the operator
  (or an explicit, approved automation) grants access.
- Per-project **licensing**: each project carries a license + a contributors list,
  so "who owns what" is explicit before anything is shared or released.

## Phasing

- **C1 (locally verifiable now)**: DB + API + web UI for projects, members,
  in-app invites, activity, comments, files index — with a **local/stub storage
  backend** so the whole flow is testable without touching real infra.
- **C2 (JAUNE)**: wire the real Nextcloud group-folder backend (operator provides
  admin token + base URL); desktop folder sync + status.
- **C3 (optional)**: Forgejo-backed repos per project for branch/PR/diff + LFS.
- **Open projects** unlocked only after **Amir E1**.
