# jalwa — Roadmap

> Milestone/wave sequencing for the Rust → Cyrius port. Live status lives in
> [`state.md`](state.md); the full plan + module ledger in
> [`port-audit.md`](port-audit.md). This file is the *order* and the *gates*.

## v1.0 criteria

- [ ] Every non-blocked module ported and green vs `rust-old/` (function-level parity)
- [ ] `.tcyr` suites cover the ported surface; `cyrius test` green in CI
- [ ] Hot-path `.bcyr` benchmarks captured (no regression on the DSP path)
- [ ] `jalwa` binary builds; non-blocked subcommands run end-to-end
- [ ] CHANGELOG complete from v0.1.0
- [ ] Security audit pass

## Milestones (waves)

### M0 — Port scaffold (v0.1.0) — ✅ 2026-07-09
- `cyrius port` scaffold landed; full 5-crate workspace (15,355 LOC) frozen at `rust-old/`
- `crates/` relocated into the oracle; `LINES_OF_RUST.txt` corrected to 15,355
- Skeleton `src/main.cyr` builds + runs; toolchain pinned `6.4.29`
- Plan of record written: `port-audit.md`, this roadmap, `state.md`, `CLAUDE.md`

### M1 — Foundations (Wave 0 + A) — *next*
- `cyrius.cyml` stdlib deps (incl. `yukti`, `patra`, `vani`, `mabda`, `bayan`, …) + dist wiring (dhvani, ai-hwaccel, bote, shravan, chitra, dhancha, darshana, abaco)
- `src/error.cyr` (`JLW_ERR_*` codes) — included first by every entry
- `jalwa-core/lib.rs` → `src/core/types.cyr` (domain types + enums→codes)

### M2 — Core services (Wave B)
- DB (patra), playlist_io (fs), scanner (fs + shravan, tarang stubbed), watcher (inotify), hardware (yukti)

### M3 — Playback + AI (Waves C, D — parallel after A)
- DSP/EQ/loudness over dhvani (the clean win); playback engine facade (tarang stubbed)
- reco (direct), daimon HTTP (net/http + bayan), fingerprint scaffold (kernel stubbed)

### M4 — Terminal UI (Wave E)
- string renderers (early win); TUI over darshana + hand-rolled ANSI CSI widgets

### M5 — Desktop GUI (Wave F)
- egui → dhancha retained-mode rewrite; mabda/Wayland run loop (puka pattern); 8 views; art via chitra

### M6 — Binary + MCP (Wave G) → v1.0
- `mcp.rs` → bote (8 JSON-RPC tools); `main.rs` → args dispatch; full `[lib].modules` assembly; integration suites

## Backlog (post-v1 / blocked)

### Video playback — **blocked on tarang + aethersafta (both still Rust)**
- `jalwa-playback/video_decode_thread.rs` and `jalwa-gui/views/video.rs` are deferred
- Needs: a Cyrius tarang demux/decode path + a linkable Cyrius video surface (aethersafta is
  Rust; `aethersafha` is a compositor app, not a lib). Unblock, then port the deferred modules.

### Real playback — **blocked on tarang**
- Decode/probe/`AudioBuffer`/`MediaInfo`/fingerprint are stubbed in v1. When a Cyrius tarang
  (or a shravan+dhvani+vani facade) exists, replace the stubs for real end-to-end playback.

### MPRIS / desktop media control — **blocked on server-side D-Bus**
- `samvada` is client-only. Options: extend samvada with object export, or adopt an AGNOS-native
  media-control convention (`majra` pub/sub or a `bote`/`setu` endpoint). Decision pending.

### Later features (carried from the Rust roadmap)
- Streaming-service adapters (Apple Music, Spotify, Tidal, YouTube Music, SoundCloud, Bandcamp,
  local/NAS, podcasts) — OAuth2/PKCE, unified search, library merge
- Subtitle rendering, A/V sync, audio visualizer (all post-video)
- Playlist editor, tray/notification, shortcut help dialog
- AGNOS integration: zugot marketplace recipe, daimon MCP tools, agnoshi intents, compositor mini-player
