# Jalwa Architecture

jalwa is a Cyrius port (v1.0.0) of a Rust media player. The full Rust
workspace is frozen at `rust-old/` as a read-only parity oracle; jalwa itself
is flat `src/*.cyr` modules — no crates, no cargo. Build with
`cyrius build src/main.cyr build/jalwa`; test with `cyrius test`.

## Module Map

```
main.cyr            CLI dispatch (scan/play/info/search/stats/library/
  │                 export/import/devices/tui/gui/mcp) + full assembly
error.cyr           shared error type
  │
core/               library: types, db (patra), scanner (real probe+tags
  │                 via shravan), playlist_io, watcher (inotify), hardware
  │
playback/           audio (decode→output), engine (threaded transport via
  │                 fnptr seam), decode_thread, dsp (dhvani EQ+normalize),
  │                 mpris (D-Bus stub)
  │
ai/                 reco, daimon, fingerprint (network + kernel stubbed)
  │
ui/                 TUI over darshana: app, renderers, widgets, tui (run loop)
  │
gui/                Wayland shell: draw (IR) → raster (CPU framebuffer) →
  │                 control (actions) → present/wayland; theme, app, views,
  │                 art_cache, input (evdev→key)
  │
mcp.cyr / mcp_serve.cyr   stdio JSON-RPC 2.0 server (8 tools, hand-rolled)
```

## Dependency Graph

```
shravan (decode/probe/tags) ─┐
dhvani  (DSP: EQ, loudness)  ─┼─→ playback ─┐
vani    (ALSA/agnos output) ─┘              │
darshana (terminal render)  ────→ ui ───────┼─→ main (CLI + MCP)
kashi   (VGA 8x16 font)     ────→ gui ───────┘

tarang (video) — still Rust — VIDEO ONLY, P1-blocked
```

Deps are declared in `cyrius.cyml` and resolved with `cyrius deps`
(dhvani, shravan, sankoch, abaco, darshana, vani wired); stdlib via
`cyrius lib sync`. The MCP server needs no dep — its loop is pure Cyrius.

## Design Principles

1. **Cross-check against `rust-old/`** — the bar is "matches what Rust did".
   Diverge only with an ADR.

2. **Library-first** — media is indexed, searchable, and analyzed
   (`core/db.cyr` on patra), not just a file opener.

3. **Real audio pipeline** — shravan decode (WAV/FLAC/MP3) → dhvani DSP
   (graphic EQ + loudness normalize) → vani ALSA/agnos output. Threaded
   engine drives full transport (play/pause/stop/seek/live-volume) over an
   fnptr backend seam. No ffmpeg, no GStreamer; ALSA/agnos, not PipeWire.

4. **AI-integrated** — recommendations, smart playlists, fingerprinting are
   built in (`ai/`), not plugins (network + kernel currently stubbed).

5. **Wayland GUI** — the desktop shell rasterizes a draw-command IR to an
   XRGB8888 `wl_shm` framebuffer (kashi font) and presents over a
   puka-forked sovereign Wayland client. CPU path, not GPU; smoke-only
   headless — validate on real AGNOS.

## Blocked / Backlog

- **Video (P1)** — `tarang` and `aethersafta` are still Rust, so video
  decode/surface (`video_decode_thread`, `view_video`, video probe) is the
  one blocked feature. Audio needs neither.
- **MPRIS export** — deferred; samvada is D-Bus client-only.
- **Polish backlog** — OGG/AAC/MP4 decode (shravan decoders broken),
  mid-track-live EQ + gapless (device-only), GUI grid-mode library /
  interactive search text-entry / real album-art blit, mouse input.
