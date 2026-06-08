# Governance

SkyHub is the platform of **the SkyCloudCollective** — a small community of
musicians and builders who share samples, instruments, and projects. This file
describes how the project is steered. It is intentionally light: enough structure
to be legible and fair, not so much that it gets in the way of making music.

## Principles

- **Community-owned, operator-curated.** The collective drives *what* gets built;
  a maintainer keeps the project coherent and decides *when* and *how*.
- **Transparency over ceremony.** Decisions, the roadmap, and the reasoning behind
  them are visible in the app and the repo. We prefer a public note to a private one.
- **Calm and accessible by default.** This applies to the product *and* to how we
  work together (see [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)).
- **Sovereign and auditable.** AGPL-3.0, self-hostable, reproducible builds. The
  source is the source of truth.

## How decisions are made — consultative democracy

We are **consultative, not majoritarian**: members have a real, visible voice, and
the maintainer carries the final call and the accountability that comes with it.

1. **Propose.** Anyone can open a feature request on the in-app **Board** (`/board`),
   or a discussion/issue in the repo.
2. **Upvote.** Members upvote requests (one vote each). The Board sorts by support,
   so what the community cares about is plain to see.
3. **Cluster.** Recurring themes and feedback are grouped into a coherent proposal —
   never silently implemented.
4. **Decide.** The maintainer accepts, defers, or declines, and reflects the outcome
   on the public **Roadmap** with a status (`planned` / `in-progress` / `shipped` /
   `paused`). A decline comes with a reason.

Upvotes inform; they do not bind. This keeps the project coherent while making sure
the community is heard and can hold decisions up to the light.

## Code changes — RFC by lazy consensus

For anything beyond a small fix:

- Open a short proposal (a Board entry, an issue, or a PR description) describing the
  change and why.
- If no one objects within a few days, it carries (**lazy consensus**). Objections are
  resolved by discussion; the maintainer breaks ties.
- Each change lands as a focused, tested commit (see [`CONTRIBUTING.md`](CONTRIBUTING.md)).

## Roles

- **Members** — propose, upvote, comment, react, follow, and contribute samples,
  projects, and code.
- **Maintainer(s)** — review and merge, curate the roadmap, edit request/roadmap
  status, and hold release/deploy keys. (`RS_ADMIN_HANDLES` lists admin handles.)
- Roles are about responsibility, not rank. The aim is to grow more maintainers over
  time, not to concentrate power.

## Safety gates

Some surfaces stay **off by default** until a human clears them:

- **Public (open) collaboration** — projects with `open` visibility expose
  public user-generated content. This is disabled in code (`RS_OPEN_UGC`, default
  off) until a legal/consent review is complete. Private and unlisted collaboration
  are always available.

## Money

The platform is **not a commercial product** and has no paywall. Hosting is covered
by voluntary mutual aid within the community; contribution is optional and confers no
special access. AGPL keeps the code a commons.

## Changing this document

Governance evolves by the same process it describes: propose a change, let it sit for
discussion, and land it once it carries.
