# jalwa — Current State

> Refreshed every wave. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Plan of record: [`port-audit.md`](port-audit.md).

## Version

**0.1.0** — scaffolded from Rust (2026-07-09) via `cyrius port`.

## Toolchain

- **Cyrius pin**: `6.4.29` (in `cyrius.cyml [package].cyrius`)

## Source / oracle

- **Rust oracle**: 15,355 LOC (40 `.rs` files, full 5-crate workspace) frozen at `rust-old/`. Do not edit.
- **Cyrius port**: `src/error.cyr` + all jalwa-core + all jalwa-playback + all jalwa-ai + all jalwa-ui (`src/ui/{renderers,app,tui,widgets}.cyr`) + jalwa-gui portable logic (`src/gui/{theme,art_cache,app,views}.cyr`) + `src/mcp.cyr` (8 tools) + `src/mcp_serve.cyr` (**real stdio JSON-RPC 2.0 server**) — green. `src/main.cyr` = full assembly + CLI dispatch — build/jalwa builds and runs (968K). (`jlw_format_duration` now lives in `core/types.cyr`.)

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
| G — binary + MCP | ✅ **done** — `mcp` (8 tools), `mcp_serve` (**real stdio JSON-RPC 2.0 loop** — `jalwa mcp` is a working MCP server; hand-rolled in pure Cyrius mirroring rust-old `run_on`, no bote dep, byte-faithful envelopes incl. serverInfo name=jalwa / protocolVersion 2024-11-05), `main` (args dispatch), **full assembly builds & runs** (968K binary). |

**Modules: all jalwa-core + all non-video jalwa-playback + all jalwa-ai + all jalwa-ui + jalwa-gui portable logic + `mcp` + `mcp_serve` + `main` (full assembly) ✅. 24 `.cyr` modules; ~28 / 33 rust modules ported.**
Backlogged (blocked/deferred): `video_decode_thread`, `view_video` (tarang+aethersafta); `equalizer`/`devices` draw + GUI dhancha render loop (dhancha); TUI run-loop (darshana); scanner real-probe/tags (shravan, unblocked — next candidate); real playback (tarang stub); MPRIS export (samvada is D-Bus client-only, hard-blocked).

## Tests

- 26 `.tcyr` suites, **849 assertions, all green**. Includes GUI (`gui_theme` 17 · `gui_art_cache` 19 · `gui_app` 24 · `gui_views` 81) + `mcp_serve` 33 (oracle: rust-old run_on integration tests). Bare `cyrius test` (CI) all green. `main`: assembly builds & runs (968K binary); `jalwa mcp` end-to-end smoke-tested (initialize/tools/list/tools/call/parse-error over real stdin→stdout).

> Toolchain drift: `cyrius.cyml` pins 6.4.29; cycc is now 6.4.36. Builds pass against the 6.4.29-vendored `lib/`; benign pin-drift warning. Bump the pin + `cyrius lib sync` when convenient.
- Per-module `.tcyr` suites land with each ported module, cross-checked against `rust-old/` `#[test]` blocks.

## Dependencies

- **stdlib** (via `cyrius lib sync`): string, fmt, alloc, vec, str, syscalls, io, args, assert, **math**
  (to grow: fs, thread, net, http, chrono, random, hashmap, tagged, result, fnptr, bayan, patra, vani, yukti, sakshi, mabda, simd)
- **dist repos** (`[deps.X]` via `cyrius deps`): **dhvani 2.2.1 + abaco 2.3.2 wired ✅**; ai-hwaccel, bote, shravan, chitra, dhancha, darshana to wire per-wave
  - note: abaco emits benign `duplicate symbol ERR_INVALID / MAX_TOKENS (last wins)` warnings under the dhvani fold — harmless
- **Blocked (still Rust)**: tarang, aethersafta

## Next

Waves A–G core are all ported (structure + portable logic, tarang/aethersafta surfaces stubbed) —
and Wave G is now fully functional (`jalwa mcp` is a real MCP server). A 6-way assessment workflow
(2026-07-09) ranked the remaining unblocked work by value-per-effort:
1. ✅ **MCP stdio JSON-RPC loop** — DONE this pass (`src/mcp_serve.cyr`).
2. **TUI run-loop** (`src/ui/tui.cyr jlw_tui_run`) — unblocked over **darshana** (tty raw/alt-screen/
   key-read verified); shell loop + CSI decode + tty setup/teardown remain. Lands correct-but-quiet
   (engine events are tarang-stubbed). *do-now-with-caveats.*
3. **Scanner real-probe + tags** via **shravan** (replace the `jlw_media_info_stub`) — unblocked (no
   tarang), high correctness value, BUT shravan's only duration path is a full decode → bump-allocator
   leak per file on large scans; needs a VORBIS_COMMENT block-walker + fixtures + an ADR. *do-now-with-caveats.*
- **Blocked:** GUI dhancha/Wayland render loop (XL; on-screen present needs **aethersafta**); real
  playback/decode/probe, `cmd_scan`, fingerprint kernel, `video_decode_thread`/`view_video` (**tarang**
  + **aethersafta**); **MPRIS export** (**samvada** is D-Bus client-only — needs a session-bus server it lacks).
See [`roadmap.md`](roadmap.md) and [`port-audit.md`](port-audit.md).
