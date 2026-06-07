# RanchSamples theme for Jellyfin

`ranchsamples-theme.css` re-skins a Jellyfin 10.x web UI into the RanchSamples
look (Bauhaus · Liquid Glass · Aero · warm near-black) so a self-hosted media
server matches the rest of the hub. It targets the dark UI + the login screen,
and is written to be **safe** — it restyles, it never hides a working control.

## Deploy

Jellyfin serves a `<CustomCss>` block from its `branding.xml`. Inline this file's
contents there (self-contained, no external CDN), then reload the web client:

1. Back up your current `branding.xml`.
2. Put the CSS in the `<CustomCss>…</CustomCss>` element of `branding.xml`
   (in your Jellyfin config directory), **or** paste it into
   *Dashboard → General → Custom CSS* in the web UI.
3. Reload the page (a server restart is only needed if the change isn't picked up).

To revert: restore your backed-up `branding.xml` (or clear the Custom CSS field).

## Notes

- Self-contained CSS — no external theme CDN, for sovereignty + a tighter CSP.
- Accent + glass + gradient come straight from `packages/tokens`; keep them in
  sync if the tokens change.
- Fonts (Space Grotesk / Atkinson Hyperlegible) are referenced with system
  fallbacks; an optional Google-Fonts `@import` is included and degrades safely
  if your CSP blocks it.
