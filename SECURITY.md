# Security Policy

Thank you for helping keep RanchSamples and the people who self-host it safe.

## Reporting a vulnerability

Please report security issues **privately** — do not open a public issue.

- Preferred: use GitHub's **"Report a vulnerability"** (the repository's **Security**
  tab → _Advisories_ → _Report a vulnerability_), which opens a private advisory.
- Or email **security@ranchsamples.dev**.

Tell us what you found, how to reproduce it, and the impact you expect. We aim to
acknowledge a report within a few days and to keep you posted while we work on a fix.
Please give us reasonable time to address the issue before any public disclosure; we're
glad to credit you once it's resolved.

## Scope

RanchSamples is self-hosted free software (AGPL-3.0): every operator runs their own
instance, so there is no shared production service — report against the **code**, not a
hosted environment. The most useful reports concern:

- **API** (`services/api`) — authentication, path handling, upload validation, injection;
- **Web app** (`apps/web`) — XSS, the embed sanitiser, CSRF;
- **Self-host path** (`Containerfile`, `SELFHOST.md`) — insecure defaults, exposed secrets.

## Supported versions

RanchSamples is pre-1.0 and moves quickly. Security fixes land on `main`; please test
against the latest `main` before reporting.
