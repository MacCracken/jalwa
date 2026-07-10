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
| E — terminal UI | ✅ **done** — `renderers`, `app`, `widgets`, `tui` incl. the **interactive run loop over darshana** (`jlw_tui_run`: tty raw/alt-screen, poll(2)-gated 50ms tick, full-screen repaint, hand-rolled CSI/SS3 key decoder, real inotify watcher auto-remove, forward-ready engine-event handler). Reviewed adversarially (2 confirmed bugs fixed: stdin-EOF busy-loop, non-arrow-CSI search wipe). **jalwa-ui fully ported.** |
| F — desktop GUI | ✅ core done — `theme` (color palette), `art_cache` (LRU + no_art tracking), `app` (GuiApp state: update_search/list_len/play_item), `views` (portable helpers from library/now_playing/queue/transport/sidebar: truncate_str, nav math, row/title/volume/duration formatting, sidebar entries, select_view). egui `.rs` draw bodies deferred to dhancha/mabda Wayland rewrite; `video`/`equalizer`/`devices` views are pure-draw (deferred; video also aethersafta-blocked). |
| G — binary + MCP | ✅ **done** — `mcp` (8 tools), `mcp_serve` (**real stdio JSON-RPC 2.0 loop** — `jalwa mcp` is a working MCP server; hand-rolled in pure Cyrius mirroring rust-old `run_on`, no bote dep, byte-faithful envelopes incl. serverInfo name=jalwa / protocolVersion 2024-11-05), `main` (args dispatch), **full assembly builds & runs** (968K binary). |

**Modules: all jalwa-core + all non-video jalwa-playback + all jalwa-ai + all jalwa-ui (incl. TUI run loop) + jalwa-gui portable logic + `mcp` + `mcp_serve` + `main` (full assembly) ✅. 24 `.cyr` modules; ~28 / 33 rust modules ported.**
Backlogged (blocked/deferred): `video_decode_thread`, `view_video` (tarang+aethersafta); `equalizer`/`devices` draw + GUI dhancha render loop (dhancha); scanner real-probe/tags (shravan, unblocked — next candidate); real playback/decode (tarang stub); MPRIS export (samvada is D-Bus client-only, hard-blocked).

## Tests

- 27 `.tcyr` suites, **888 assertions, all green**. Includes GUI (`gui_theme` 17 · `gui_art_cache` 19 · `gui_app` 24 · `gui_views` 81) + `mcp_serve` 33 + `tui_runloop` 38 (CSI/SS3 decoder, EOF-quit, search-mode CSI no-op, watcher/engine event handlers). Bare `cyrius test` (CI) all green. `main`: assembly builds & runs (985K binary); `jalwa mcp` end-to-end smoke-tested over real stdin→stdout; `poll(2)` verified working on agnos.

> Toolchain drift: `cyrius.cyml` pins 6.4.29; cycc is now 6.4.37. Builds pass against the 6.4.29-vendored `lib/`; benign pin-drift warning. Bump the pin + `cyrius lib sync` when convenient.
- Per-module `.tcyr` suites land with each ported module, cross-checked against `rust-old/` `#[test]` blocks.

## Dependencies

- **stdlib** (via `cyrius lib sync`): string, fmt, alloc, vec, str, syscalls, io, args, assert, **math**
  (to grow: fs, thread, net, http, chrono, random, hashmap, tagged, result, fnptr, bayan, patra, vani, yukti, sakshi, mabda, simd)
- **dist repos** (`[deps.X]` via `cyrius deps`): **dhvani 2.2.1 + abaco 2.3.2 + darshana 0.9.0 wired ✅** (40 deps locked); ai-hwaccel, bote, shravan, chitra, dhancha to wire per-wave
  - note: abaco emits benign `duplicate symbol ERR_INVALID / MAX_TOKENS (last wins)` warnings under the dhvani fold — harmless; darshana pulls transitive vani/acoustic externals (DCE-pruned)
  - note: the MCP server needs NO dep — the JSON-RPC loop is hand-rolled in pure Cyrius (rust-old used bote only for the ToolDef *types*, hand-rolling the loop itself)
- **Blocked (still Rust)**: tarang, aethersafta

## Next

Waves A–G core are all ported AND both Wave G (`jalwa mcp` real MCP server) and Wave E (interactive
TUI over darshana) are now fully functional. From the 6-way assessment workflow (2026-07-09), ranked
by value-per-effort:
1. ✅ **MCP stdio JSON-RPC loop** — DONE (`src/mcp_serve.cyr`).
2. ✅ **TUI run-loop over darshana** — DONE (`src/ui/tui.cyr jlw_tui_run`), adversarially reviewed + fixed.
3. **Scanner real-probe + tags** via **shravan** (replace the `jlw_media_info_stub` in `src/core/scanner.cyr`)
   — the last unblocked do-now candidate. No tarang needed; high correctness value (scanned tracks
   currently get placeholder metadata). CAVEATS: shravan's only duration path for compressed formats is a
   full decode → bump-allocator leak per file on large scans; needs a VORBIS_COMMENT/tag block-walker,
   new byte-blob test fixtures, and an ADR for the permanent album-art divergence. *do-now-with-caveats.*
- **Blocked:** GUI dhancha/Wayland render loop (XL; on-screen present needs **aethersafta**); real
  playback/decode/probe, `cmd_scan`, fingerprint kernel, `video_decode_thread`/`view_video` (**tarang**
  + **aethersafta**); **MPRIS export** (**samvada** is D-Bus client-only — needs a session-bus server it lacks).
See [`roadmap.md`](roadmap.md) and [`port-audit.md`](port-audit.md).
