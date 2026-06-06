#!/usr/bin/env python3
"""audit-artifact.py — residue + integrity scan for built artifacts and trees.

Run before ANY publish/push (the residue gate). Scans files (and the entries of
.zip / .dmg / archives) for things that must never ship in a public artifact:
private IPs, internal hostnames, localhost:port, key material, and any patterns
listed in an operator-local file.

GENERIC patterns are baked in. DEPLOYMENT-SPECIFIC patterns (your real domain,
your tailnet name, your username) live in ci/audit-patterns.local — gitignored,
so the public repo never contains the very strings it screens for. Pass it with
--patterns ci/audit-patterns.local.

Usage:
    python3 ci/audit-artifact.py <path> [<path> ...] [--patterns FILE] [--allow REGEX]
Exit code 0 = clean, 1 = residue found (fail the CI job).
"""
from __future__ import annotations

import argparse
import io
import pathlib
import re
import sys
import zipfile

# GENERIC: things that are essentially never legitimate, even in OSS source —
# safe to run on the source tree without false positives.
GENERIC = [
    (r"\b\w[\w-]*\.ts\.net\b", "tailscale MagicDNS host"),
    (r"-----BEGIN [A-Z ]*PRIVATE KEY-----", "private key block"),
    (r"\bBearer\s+[A-Za-z0-9._-]{20,}", "bearer token"),
    (r"\bsk-[A-Za-z0-9]{20,}", "secret-key-style token"),
]

# STRICT (--strict): added when scanning BUILT ARTIFACTS, where a baked private
# IP or loopback is suspicious. In source these are legit dev defaults (a Caddy
# example, a dev API base), so they're off by default to avoid noise.
STRICT_EXTRA = [
    (r"\b(?:10|127)\.\d{1,3}\.\d{1,3}\.\d{1,3}\b", "private/loopback IPv4"),
    (r"\b192\.168\.\d{1,3}\.\d{1,3}\b", "private IPv4 (192.168)"),
    (r"\b172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}\b", "private IPv4 (172.16/12)"),
    (r"localhost:\d{2,5}", "localhost:port"),
]

# File types worth scanning as text; archives are walked entry-by-entry.
TEXT_SUFFIXES = {
    ".txt", ".md", ".json", ".js", ".ts", ".css", ".html", ".svelte", ".py",
    ".rs", ".toml", ".yml", ".yaml", ".cfg", ".conf", ".sh", ".env", ".sql",
}
ARCHIVE_SUFFIXES = {".zip", ".dmg", ".vsix", ".jar", ".whl", ".clap", ".vst3"}


def load_patterns(path: str | None, strict: bool = False) -> list[tuple[re.Pattern, str]]:
    base = GENERIC + (STRICT_EXTRA if strict else [])
    pats = [(re.compile(p), why) for p, why in base]
    if path:
        for line in pathlib.Path(path).read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                pats.append((re.compile(line), f"local pattern: {line}"))
    return pats


def scan_bytes(data: bytes, pats, allow: re.Pattern | None) -> list[str]:
    try:
        text = data.decode("utf-8", errors="ignore")
    except Exception:
        return []
    hits = []
    for rx, why in pats:
        for m in rx.finditer(text):
            frag = m.group(0)
            if allow and allow.search(frag):
                continue
            hits.append(f"{why}: {frag!r}")
    return hits


def scan_path(path: pathlib.Path, pats, allow) -> list[str]:
    findings: list[str] = []
    if path.is_dir():
        for child in path.rglob("*"):
            if child.is_file():
                findings += [f"{child}: {h}" for h in scan_path(child, pats, allow)]
        return findings
    suffix = path.suffix.lower()
    try:
        if suffix in ARCHIVE_SUFFIXES and zipfile.is_zipfile(path):
            with zipfile.ZipFile(path) as z:
                for name in z.namelist():
                    with z.open(name) as f:
                        findings += [f"[{name}] {h}" for h in scan_bytes(f.read(), pats, allow)]
        elif suffix in TEXT_SUFFIXES or suffix == "":
            findings += scan_bytes(path.read_bytes(), pats, allow)
    except Exception as e:  # noqa: BLE001 — never let a read error pass as clean
        findings.append(f"(could not scan: {e})")
    return findings


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("paths", nargs="+")
    ap.add_argument("--patterns", help="operator-local extra patterns file (gitignored)")
    ap.add_argument("--allow", help="regex of fragments to waive (e.g. a baked public host)")
    ap.add_argument("--strict", action="store_true",
                    help="also flag private/loopback IPs + localhost:port (for built artifacts)")
    args = ap.parse_args()

    pats = load_patterns(args.patterns, strict=args.strict)
    allow = re.compile(args.allow) if args.allow else None

    total = 0
    for p in args.paths:
        path = pathlib.Path(p)
        if not path.exists():
            print(f"✗ not found: {p}", file=sys.stderr)
            return 1
        findings = scan_path(path, pats, allow)
        for f in findings:
            print(f"  RESIDUE {f}")
        total += len(findings)

    if total:
        print(f"✗ {total} residue finding(s) — do NOT publish.", file=sys.stderr)
        return 1
    print("✓ residue clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
