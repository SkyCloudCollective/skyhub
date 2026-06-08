# Releasing SkyHub

TL;DR — the release sequence, in order:

1. Bump version in `Cargo.toml` (`[workspace.package] version`).
2. Build all platforms: `bash scripts/build-all.sh`.
3. Residue audit passes (included in step 2; also run manually if needed).
4. Commit + tag: `git tag v<version>`.
5. Push to origin (Marwan's call, after verifying the Mac build).
6. CI picks up the tag and runs the release workflows.

---

## Step 1 — version bump

Edit the single canonical version field in `Cargo.toml`:

```toml
[workspace.package]
version = "0.2.0"   # bump this
```

All crates inherit it via `version.workspace = true`. Commit it:

```
git add Cargo.toml Cargo.lock
git commit -F - <<'EOF'
chore: bump version to 0.2.0
EOF
```

---

## Step 2 — build

```
bash scripts/build-all.sh
# or with macOS (requires the Mac to be reachable):
RS_MAC_SSH_HOST=youruser@your-mac bash scripts/build-all.sh
```

The version is read automatically from `Cargo.toml`. Override with
`BUILD_VERSION=0.2.0-rc1 bash scripts/build-all.sh` for pre-release stamps.

---

## Step 3 — residue audit

The audit is embedded in `build-all.sh` but can be run independently:

```
python3 ci/audit-artifact.py dist
# With operator-local patterns (never committed):
python3 ci/audit-artifact.py dist --patterns ci/audit-patterns.local
```

Operator-local patterns live in `ci/audit-patterns.local` (gitignored).
Copy from `ci/audit-patterns.local.example` and fill in real values.

If the audit finds anything: fix the source, rebuild, re-audit.

---

## Step 4 — commit + tag

```
git add dist/manifest.json dist/SHA256SUMS
git commit -F - <<'EOF'
release: v0.2.0 — dist manifest + SHA256SUMS
EOF
git tag v0.2.0
```

Do not commit the built binaries themselves (`dist/` is gitignored for `.clap`,
`.vst3`, `.so`, `.dll`). Only the manifest JSON and SUMS file are committed.

---

## Step 5 — push (Marwan's call)

```
git push origin main
git push origin v0.2.0
```

The CI release workflow triggers on the tag and re-builds from source on
runners (not from the local dist/ — CI is the authoritative artifact source).

---

## Step 6 — CI artifacts

The tag triggers:
- `.github/workflows/ci.yml` (rust/plugins/api/web/audit jobs)
- Future: `morph-build.yml`, `phaseplan-build.yml` (per-plugin signed release
  matrix — JAUNE: requires Marwan's `MAC_SIGN_ID` and `WINDOWS_SIGN_CERT`
  secrets in the repo settings).

Until the signed release CI exists, distribute the local `dist/` artifacts
directly. Document the unsigned workaround for users:

  Linux: load from `~/.clap/` or `~/.vst3/` — no gating on Linux.
  Windows: SmartScreen warns. Right-click binary, "Run anyway".
  macOS: Gatekeeper blocks unsigned bundles. Right-click plugin in Finder,
         choose Open, then click Open in the dialog.

---

## Signing (JAUNE — Marwan's keys required)

macOS code signing + notarization:
```
codesign --deep --sign "Developer ID Application: <name> (<team-id>)" \
  dist/macos-universal/plugin-morph.clap
xcrun notarytool submit dist/macos-universal/plugin-morph.clap \
  --apple-id <apple-id> --team-id <team-id> --password <app-specific-pw> --wait
xcrun stapler staple dist/macos-universal/plugin-morph.clap
```

Windows code signing:
```
signtool sign /fd sha256 /tr http://timestamp.sectigo.com /td sha256 \
  /f <cert.pfx> /p <password> dist/windows-x86_64/plugin-morph.clap
```

These are never automated locally. The keys (`MAC_SIGN_ID`, `WINDOWS_SIGN_CERT`,
`WINDOWS_SIGN_KEY`) live in GitHub Actions secrets and are used only in the
release CI workflows.

---

## Checklist (copy-paste)

```
[ ] version bumped in Cargo.toml + committed
[ ] bash scripts/build-all.sh passed (Linux + Windows + macOS if available)
[ ] residue audit: exit 0
[ ] dist/manifest.json + SHA256SUMS committed
[ ] git tag v<version> created
[ ] Marwan reviewed & approved push
[ ] git push origin main && git push origin v<version>
[ ] CI green on tag
[ ] release notes written (GitHub Releases or CHANGELOG.md)
```
