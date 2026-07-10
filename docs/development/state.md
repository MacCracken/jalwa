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
| B — core services | ✅ done — `playlist_io`, `db`, `scanner` (**real audio probe + tags via shravan** — header-only WAV/FLAC/MP3 duration + FLAC-Vorbis/MP3-ID3v2 tags; no tarang), `watcher`, `hardware` (yukti). `jalwa scan` + MCP scan are live. **jalwa-core fully ported.** ADR 0002. |
| C — playback + DSP | ✅ done — `dsp`, `decode_thread`, `engine`, `mpris` (decode/probe/D-Bus stubbed; video_decode_thread backlogged) |
| D — AI (reco/daimon/fingerprint) | ✅ done — `reco`, `daimon`, `fingerprint` (network + fingerprint kernel stubbed). **jalwa-ai fully ported.** |
| E — terminal UI | ✅ **done** — `renderers`, `app`, `widgets`, `tui` incl. the **interactive run loop over darshana** (`jlw_tui_run`: tty raw/alt-screen, poll(2)-gated 50ms tick, full-screen repaint, hand-rolled CSI/SS3 key decoder, real inotify watcher auto-remove, forward-ready engine-event handler). Reviewed adversarially (2 confirmed bugs fixed: stdin-EOF busy-loop, non-arrow-CSI search wipe). **jalwa-ui fully ported.** |
| F — desktop GUI | ✅ core done — `theme` (color palette), `art_cache` (LRU + no_art tracking), `app` (GuiApp state: update_search/list_len/play_item), `views` (portable helpers from library/now_playing/queue/transport/sidebar: truncate_str, nav math, row/title/volume/duration formatting, sidebar entries, select_view). egui `.rs` draw bodies deferred to dhancha/mabda Wayland rewrite; `video`/`equalizer`/`devices` views are pure-draw (deferred; video also aethersafta-blocked). |
| G — binary + MCP | ✅ **done** — `mcp` (8 tools), `mcp_serve` (**real stdio JSON-RPC 2.0 loop** — `jalwa mcp` is a working MCP server; hand-rolled in pure Cyrius mirroring rust-old `run_on`, no bote dep, byte-faithful envelopes incl. serverInfo name=jalwa / protocolVersion 2024-11-05), `main` (args dispatch), **full assembly builds & runs** (968K binary). |

**Modules: all jalwa-core + all non-video jalwa-playback + all jalwa-ai + all jalwa-ui (incl. TUI run loop) + jalwa-gui portable logic + `mcp` + `mcp_serve` + `main` (full assembly) ✅. 24 `.cyr` modules; ~28 / 33 rust modules ported.**
Backlogged (blocked/deferred): `video_decode_thread`, `view_video` (tarang+aethersafta); `equalizer`/`devices` draw + GUI dhancha render loop (dhancha); scanner real-probe/tags (shravan, unblocked — next candidate); real playback/decode (tarang stub); MPRIS export (samvada is D-Bus client-only, hard-blocked).

## Tests

- 29 `.tcyr` suites, **920 assertions, all green**. Includes GUI (`gui_theme` 17 · `gui_art_cache` 19 · `gui_app` 24 · `gui_views` 81) + `mcp_serve` 33 + `tui_runloop` 38 + `scanner_probe` 31 (WAV/FLAC/MP3 real probe via shravan; fixtures at `tests/fixtures/probe.{flac,mp3}`). Bare `cyrius test` (CI) all green. `main`: assembly builds & runs (2.30MB binary — folds shravan/sankoch); `jalwa scan` + `jalwa mcp` end-to-end smoke-tested; `poll(2)` verified on agnos.

> Toolchain drift: `cyrius.cyml` pins 6.4.29; cycc is now 6.4.39. Builds pass against the 6.4.29-vendored `lib/`; benign pin-drift warning. Bump the pin + `cyrius lib sync` when convenient.
> Benign build warning: `duplicate fn 'detect_format'` (shravan audio-format vs sankoch compression-format) — jalwa calls neither (uses its own `jlw_detect_audio_format`), so it's inert like the abaco ERR_INVALID noise.
- Per-module `.tcyr` suites land with each ported module, cross-checked against `rust-old/` `#[test]` blocks.

## Dependencies

- **stdlib** (via `cyrius lib sync`): string, fmt, alloc, vec, str, syscalls, io, args, assert, math, random, chrono, patra, fs, yukti, hashmap, tagged, process, **ganita, thread, fnptr, bayan** (last 4 added for shravan)
- **dist repos** (`[deps.X]` via `cyrius deps`): **dhvani 2.2.1 + abaco 2.3.2 + darshana 0.9.0 + shravan 2.6.7 + sankoch 2.4.9 wired ✅** (48 deps locked); ai-hwaccel, bote, chitra, dhancha to wire per-wave
  - note: abaco emits benign `duplicate symbol ERR_INVALID / MAX_TOKENS (last wins)` warnings; shravan/sankoch add a benign `duplicate fn detect_format` (jalwa calls neither); darshana pulls transitive vani/acoustic externals (DCE-pruned)
  - note: the MCP server needs NO dep — the JSON-RPC loop is hand-rolled in pure Cyrius (rust-old used bote only for the ToolDef *types*, hand-rolling the loop itself)
- **Audio stack (unblocked)**: **shravan** (codecs: decode/probe/tags), **dhvani** (DSP), **vani** (output) — audio is NOT tarang-gated.
- **Blocked (still Rust) — VIDEO ONLY**: **tarang** (video decode/encode), **aethersafta** (video surface). Audio needs neither.

## Next

Waves A–G core are all ported; `jalwa mcp` (Wave G), `jalwa tui` (Wave E), and `jalwa scan` (library
scanning) are all fully functional. The three do-now items from the 2026-07-09 assessment are DONE:
1. ✅ **MCP stdio JSON-RPC loop** (`src/mcp_serve.cyr`).
2. ✅ **TUI run-loop over darshana** (`src/ui/tui.cyr jlw_tui_run`), adversarially reviewed + fixed.
3. ✅ **Scanner real-probe + tags via shravan** (`src/core/scanner.cyr`) — ADR 0002.

**Newly-unblocked by the tarang=video-only correction: real AUDIO PLAYBACK.** `jlw_engine_open/play` +
`decode_loop` were stubbed as "tarang-blocked", but audio decode goes through **shravan**, DSP through
**dhvani** (already wired + ported), output through **vani** — none blocked. This is the next high-value
target: shravan `codec_open`/decode → dhvani DSP/EQ → vani ALSA out, replacing the decode stub, so
`jalwa play` produces real sound. Larger than the scanner (a decode/output pipeline + real engine events).

- **Still blocked (VIDEO only):** `video_decode_thread`/`view_video`, `cmd_info` video probe (**tarang** +
  **aethersafta**); GUI dhancha/Wayland render loop (XL; on-screen present needs **aethersafta**);
  **MPRIS export** (**samvada** is D-Bus client-only — needs a session-bus server it lacks).
See [`roadmap.md`](roadmap.md) and [`port-audit.md`](port-audit.md).
