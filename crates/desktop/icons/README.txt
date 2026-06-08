Icon assets required by tauri.conf.json — not committed to git.

Required files (generate with `cargo tauri icon <source.png>` from a 1024x1024 source):
  32x32.png
  128x128.png
  128x128@2x.png
  icon.ico          (Windows)
  icon.icns         (macOS, best-effort)

Generate command (run from crates/desktop/, requires @tauri-apps/cli):
  npx @tauri-apps/cli icon ../../docs/screenshots/library-light.png

The docs/screenshots/library-light.png exists in the repo and can serve as a
placeholder source until a proper branded icon is designed. The resulting icons
land here. They are gitignored via the workspace .gitignore (no binary blobs).

Alternatively the CI job can generate them on the fly before `tauri build`.
See .github/workflows/release.yml (desktop-linux / desktop-windows jobs).
