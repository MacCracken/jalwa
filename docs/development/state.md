# jalwa — Current State

> Refreshed every wave. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Plan of record: [`port-audit.md`](port-audit.md).

## Version

**0.1.0** — scaffolded from Rust (2026-07-09) via `cyrius port`.

## Toolchain

- **Cyrius pin**: `6.4.29` (in `cyrius.cyml [package].cyrius`)

## Source / oracle

- **Rust oracle**: 15,355 LOC (40 `.rs` files, full 5-crate workspace) frozen at `rust-old/`. Do not edit.
- **Cyrius port**: `src/error.cyr` + all jalwa-core + all jalwa-playback + all jalwa-ai + all jalwa-ui (`src/ui/{renderers,app,tui,widgets}.cyr`) + jalwa-gui portable logic (`src/gui/{theme,art_cache,app,views}.cyr`) — green. `src/main.cyr` = full assembly + CLI dispatch — build/jalwa builds and runs (957K). (`jlw_format_duration` now lives in `core/types.cyr`.)

## Port progress

| Wave | Status |
|---|---|
| 0 — scaffold + oracle freeze + `error.cyr` | ✅ done |
| A — core types | ✅ done — all 7 types (6 enums, PlaybackStatus, Uuid, Playlist, PlayQueue, MediaItem, Library); 72 tests. ADR 0001 (linear-scan indexes) |
| B — core services | ✅ done — `playlist_io`, `db`, `scanner`, `watcher`, `hardware` (yukti). **jalwa-core fully ported.** |
| C — playback + DSP | ✅ done — `dsp`, `decode_thread`, `engine`, `mpris` (decode/probe/D-Bus stubbed; video_decode_thread backlogged) |
| D — AI (reco/daimon/fingerprint) | ✅ done — `reco`, `daimon`, `fingerprint` (network + fingerprint kernel stubbed). **jalwa-ai fully ported.** |
| E — terminal UI | ✅ done — `renderers`, `app`, `tui`, `widgets` (draw-loop + darshana tty deferred). **jalwa-ui fully ported.** |
| F — desktop GUI | ✅ core done — `theme` (color palette), `art_cache` (LRU + no_art tracking), `app` (GuiApp state: update_search/list_len/play_item), `views` (portable helpers from library/now_playing/queue/transport/sidebar: truncate_str, nav math, row/title/volume/duration formatting, sidebar entries, select_view). egui `.rs` draw bodies deferred to dhancha/mabda Wayland rewrite; `video`/`equalizer`/`devices` views are pure-draw (deferred; video also aethersafta-blocked). |
| G — binary + MCP | ✅ core done — `mcp` (8 tools), `main` (args dispatch), **full assembly builds & runs** (957K binary). stdio JSON-RPC loop deferred. |

**Modules: all jalwa-core + all non-video jalwa-playback + all jalwa-ai + all jalwa-ui + jalwa-gui portable logic + `mcp` + `main` (full assembly) ✅. 23 `.cyr` modules; ~27 / 33 rust modules ported.**
Backlogged (blocked/deferred): `video_decode_thread`, `view_video` (tarang+aethersafta); `equalizer`/`devices` draw (dhancha); GUI dhancha render loop; real playback (tarang stub); MPRIS export (samvada client-only).

## Tests

- 25 `.tcyr` suites, **815 assertions, all green**. Includes GUI: `gui_theme` 17 · `gui_art_cache` 19 · `gui_app` 24 · `gui_views` 81. Bare `cyrius test` (CI) all green. `main`: assembly builds & runs (957K binary; stats/search/library/play/mcp/gui verified).

> Toolchain drift: `cyrius.cyml` pins 6.4.29; cycc is now 6.4.35. Builds pass against the 6.4.29-vendored `lib/`; benign pin-drift warning. Bump the pin + `cyrius lib sync` when convenient.
- Per-module `.tcyr` suites land with each ported module, cross-checked against `rust-old/` `#[test]` blocks.

## Dependencies

- **stdlib** (via `cyrius lib sync`): string, fmt, alloc, vec, str, syscalls, io, args, assert, **math**
  (to grow: fs, thread, net, http, chrono, random, hashmap, tagged, result, fnptr, bayan, patra, vani, yukti, sakshi, mabda, simd)
- **dist repos** (`[deps.X]` via `cyrius deps`): **dhvani 2.2.1 + abaco 2.3.2 wired ✅**; ai-hwaccel, bote, shravan, chitra, dhancha, darshana to wire per-wave
  - note: abaco emits benign `duplicate symbol ERR_INVALID / MAX_TOKENS (last wins)` warnings under the dhvani fold — harmless
- **Blocked (still Rust)**: tarang, aethersafta

## Next

Waves A–G core are all ported (structure + portable logic, tarang/aethersafta surfaces stubbed).
Remaining toward **1.0.0** is the deferred draw/IO layer, all gated on ecosystem dists:
- **dhancha/darshana render loops** — TUI run-loop + GUI Wayland window (retained-mode rewrite of the
  deferred egui/ratatui draw bodies: sidebar/transport/library/now_playing/queue/equalizer/devices).
- **tarang** (still Rust) — real decode/probe: `decode_loop`, `engine.open/play`, `cmd_info`/`cmd_scan`,
  fingerprint kernel. Unblocks `video_decode_thread` + `view_video` (with **aethersafta**).
- **MCP stdio JSON-RPC loop**; **MPRIS D-Bus export** (samvada is client-only).
See [`roadmap.md`](roadmap.md) and [`port-audit.md`](port-audit.md).
