# jalwa — Roadmap

> Milestone/wave sequencing for the Rust → Cyrius port. Live status lives in
> [`state.md`](state.md); the full plan + module ledger in
> [`port-audit.md`](port-audit.md). This file is the *order* and the *gates*.

## Status: **v1.2.3 — visual-language design→code arc COMPLETE (all non-visualizer surfaces)** ✅

All of jalwa-core, jalwa-playback (real audio), jalwa-ai, jalwa-ui, jalwa-gui, and
the binary + MCP server are ported to Cyrius and green against `rust-old/`. **v1.1.0**
shipped jalwa as a **window on the sovereign AGNOS desktop** (aethersafha over setu;
`aethersafha-jalwa-smoke.sh`). The **v1.2.x design→code arc** (below) is now **complete**:
the "Jalwa Visual Language" color system, three switchable dark themes, an in-app Settings
picker, and every restyled surface (Now-Playing, Library, Queue, Search, Mini-player) are
in code. What remains is the **GPU-era visualizer** layer (aurora/caustic/mandala, glass
blur, 60fps bloom), backlogged with **video** — both hard-blocked on still-Rust
dependencies (see P1 below).

## v1.2.x — design → code arc

Port the [`docs/development/design/`](design/README.md) mockups (the designer's
per-view `.dc.html` files) into the CPU-raster GUI, one surface per release. Scope is
the **color/layout/typography** layer only — the GPU-era visualizers (aurora/caustic/
mandala), frosted-glass blur, and 60fps bloom in the north-star need a GPU path jalwa
doesn't have yet and stay backlogged with video (P1).

| Release | Surface | Source design | Status |
|---|---|---|---|
| **1.2.0** | Palette + 3 themes, Settings/theme-switcher, Now-Playing hero, sidebar + transport chrome | `Jalwa Visual Language` + `Jalwa Now-Playing` | ✅ shipped ([ADR 0003](../adr/003-visual-language-redesign.md)) |
| **1.2.1** | Library view (header + eyebrow + count, list/grid selection chrome) | `Jalwa Library.dc.html` | ✅ shipped |
| **1.2.2** | Queue view + Search screen (eyebrow/count/chrome, focused query field + hints) | `Jalwa Queue & Search.dc.html` | ✅ shipped |
| **1.2.3** | Mini-player (compact window mode, `z` toggle) | `Jalwa Mini-Player.dc.html` | ✅ shipped |

**Arc complete.** All five design mockups are ported (color/layout/typography). The only
un-ported design content is the GPU-era **visualizer** (aurora/caustic/mandala shaders,
glass blur, 60fps bloom) — it needs a GPU path jalwa doesn't have and is backlogged with
video (P1).

> The 8-slot active-palette contract (`src/gui/theme.cyr`) means each per-view port is
> layout/typography work only — colors already resolve through the active theme, so all
> three themes come free with every ported view.

## v1.0 criteria

- [x] Every non-blocked module ported and green vs `rust-old/` (function-level parity)
- [x] `.tcyr` suites cover the ported surface; `cyrius test` green in CI
- [x] `jalwa` binary builds; non-blocked subcommands run end-to-end
      (`scan · play · info · search · stats · library · export · import · devices · tui · gui · mcp`)
- [ ] `.bcyr` hot-path benchmarks — deferred (dhvani carries the DSP benches)
- [ ] CHANGELOG complete from v0.1.0 — pending
- [ ] Security audit pass — pending

## Shipped waves (v0.1.0 → v1.0.0)

| Wave | Delivered |
|---|---|
| M0 — scaffold | `cyrius port`; 5-crate Rust workspace frozen at `rust-old/`; toolchain pinned |
| M1 — foundations | `error.cyr`, `core/types.cyr` (domain types, enums→codes) |
| M2 — core services | DB (patra), playlist_io, **scanner (real audio probe + tags via shravan)**, watcher, hardware |
| M3 — playback + AI | **real decode→output** (shravan→dhvani→vani), threaded engine, transport, EQ + normalize; reco/daimon/fingerprint |
| M4 — terminal UI | renderers, widgets, `jalwa tui` interactive run loop over darshana |
| M5 — desktop GUI | draw-command IR → CPU rasterizer (kashi font) → control layer → **`jalwa gui`** Wayland present shell (smoke-only) |
| M6 — binary + MCP | `jalwa mcp` (8-tool stdio JSON-RPC), full-assembly `main` |

## Post-v1 backlog (priority order)

### P1 — **Video playback** — blocked on tarang + aethersafta (both still Rust) — *top of the roadmap*
The single largest remaining parity gap vs `rust-old/`. Deferred modules:
`jalwa-playback/video_decode_thread.rs`, `jalwa-gui/views/video.rs`, and the video branch
of the scanner probe. **Unblock sequence:**
1. **tarang → Cyrius** — a demux/decode path (tarang is currently the only video codec dep, still Rust).
2. **Video surface** — a linkable Cyrius video output: either `aethersafta` ported (it's Rust;
   `aethersafha` is a compositor *app*, not a lib), or a `wl_shm` video-buffer path added to
   `src/gui/wayland.cyr` (the present shell already owns an XRGB8888 wl_shm buffer).
3. **Port the deferred modules** — `video_decode_thread`, `view_video`, the scanner's video-probe branch.
4. **Wire the GUI** — replace the `jlw_gui_build_video` placeholder (draw.cyr) with a real frame
   blit; restore the engine's auto-switch-to-Video-view on video playback (app.rs:371 in rust-old).

### P2 — MPRIS / desktop media control — blocked on server-side D-Bus
`samvada` is D-Bus **client-only**. Options: extend samvada with object export, or adopt an
AGNOS-native media-control convention (`majra` pub/sub or a `bote`/`setu` endpoint). Decision pending.

### P3 — Mouse / pointer input — GUI interaction parity
rust-old's egui GUI was **mouse-first** (clickable sidebar nav, list rows + grid cells, transport
play/prev/next buttons, drag seek bar + volume + EQ band sliders, scroll-wheel list scroll). jalwa's
Wayland shell is currently **keyboard-only** — `gui/wayland.cyr` binds `wl_keyboard` but not
`wl_pointer`. To add:
1. **Bind `wl_pointer`** off `wl_seat` and handle `enter`/`leave`/`motion`/`button`/`axis` events in
   `gui/wayland.cyr`, alongside the keyboard ring (smoke-only, like the rest of the Wayland backend).
2. **A testable hit-test layer** — `jlw_gui_hit_test(app, x, y, win_w, win_h) → (action, payload)`
   mapping a click coordinate to a GUI action (select sidebar entry / library row|cell / transport
   play·prev·next / seek-to-ratio / EQ-band-set / grid·list toggle), reusing the same layout math as
   the draw builders. Headless-testable exactly like `jlw_gui_key_to_action` — no display needed.
3. **Route** pointer events through `hit_test → apply_action` in the present loop; add hover highlight
   and scroll-wheel list scroll. The hit-test + action layers are unit-tested; only the `wl_pointer`
   wire handling is smoke-only.

### P4 — GUI real album-art blit — needs an image decoder
`gui/raster.cyr`'s IMAGE command draws a lettered placeholder box. A real album-art blit needs a
JPEG/PNG decoder (none wired — tarang is video-only) plus scaled-RGBA alpha blit. Add a Cyrius
image codec (or a `chitra` path), then blit decoded art into the now-playing + grid art slots.

### P5 — GUI + audio polish
- Clip-stack → multi-level (scroll viewports currently use one clip level; fine today).
- Mid-track-live EQ + gapless playback (device-untestable / complex — decode/output/transport/EQ/normalize all done).
- OGG / AAC / MP4 decode (shravan's `ogg_decode` errors, `mp4_decode` segfaults — excluded in v1).

### P6 — Later features (carried from the Rust roadmap)
- Streaming-service adapters (Apple Music, Spotify, Tidal, YouTube Music, SoundCloud, Bandcamp,
  local/NAS, podcasts) — OAuth2/PKCE, unified search, library merge.
- Subtitle rendering, A/V sync, audio visualizer (all post-video).
- Playlist editor, tray/notification, shortcut help dialog.
- AGNOS integration: zugot marketplace recipe, daimon MCP tools, agnoshi intents, compositor mini-player.
