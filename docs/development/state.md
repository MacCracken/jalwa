# jalwa — Current State

> Refreshed every wave. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Plan of record: [`port-audit.md`](port-audit.md).

## Version

**1.0.0** — Rust → Cyrius port complete (2026-07-10). Scaffolded 2026-07-09 via `cyrius port`.

## Toolchain

- **Cyrius pin**: `6.4.29` (in `cyrius.cyml [package].cyrius`)

## Source / oracle

- **Rust oracle**: 15,355 LOC (40 `.rs` files, full 5-crate workspace) frozen at `rust-old/`. Do not edit.
- **Cyrius port**: `src/error.cyr` + all jalwa-core (incl. **real audio scan/probe**) + all jalwa-playback (incl. **real audio decode+output** `src/playback/audio.cyr`) + all jalwa-ai + all jalwa-ui (incl. TUI run loop) + jalwa-gui portable logic + `src/mcp.cyr` + `src/mcp_serve.cyr` (**real stdio JSON-RPC server**) — green. `src/main.cyr` = full assembly + CLI dispatch — build/jalwa builds and runs (2.33MB). **Working commands: scan, play, search, stats, library, info, export, import, devices, tui, gui, mcp.** (`jlw_format_duration` in `core/types.cyr`.)

## Port progress

| Wave | Status |
|---|---|
| 0 — scaffold + oracle freeze + `error.cyr` | ✅ done |
| A — core types | ✅ done — all 7 types (6 enums, PlaybackStatus, Uuid, Playlist, PlayQueue, MediaItem, Library); 72 tests. ADR 0001 (linear-scan indexes) |
| B — core services | ✅ done — `playlist_io`, `db`, `scanner` (**real audio probe + tags via shravan** — header-only WAV/FLAC/MP3 duration + FLAC-Vorbis/MP3-ID3v2 tags; no tarang), `watcher`, `hardware` (yukti). `jalwa scan` + MCP scan are live. **jalwa-core fully ported.** ADR 0002. |
| C — playback + DSP | ✅ done — `dsp`, `decode_thread`, `engine`, `mpris`, **`audio` (real decode→output)**. `jalwa play` produces sound: shravan `wav/flac/mp3_decode` → volume → f64→i16 → **vani** ALSA/agnos out (WAV/FLAC/MP3 — shravan's `ogg_decode` errors + `mp4_decode` segfaults, excluded). **Threaded engine playback**: `jlw_engine_play` spawns a background decode thread via an **fnptr audio-backend seam** (keeps engine.cyr free of a compile-time shravan/vani dep — the 9 engine-including unit tests stay on the no-op path); `poll_events`→`TrackFinished`, `stop`→cancel, `position`→live. **Transport control during play**: `pause`/resume (thread writes silence to avoid ALSA underrun), live `set_volume`, and `seek` (jump the sample index + `audio_drop`/prepare to flush queued audio) reach the thread via atomic handle flags (`ctl` seam). **Full transport works live** — the TUI plays in the background, advances the queue, and its play/pause/stop/seek/volume keys all take effect. **DSP chain** (dhvani `jlw_dsp`, upfront per-track via the play seam): graphic **EQ** + loudness-**normalize** — the TUI EQ view's band adjustments + `N` normalize toggle affect playback (per-track). Mid-track-live EQ + gapless deferred (device-only / complex). D-Bus stubbed; video tarang-blocked. |
| D — AI (reco/daimon/fingerprint) | ✅ done — `reco`, `daimon`, `fingerprint` (network + fingerprint kernel stubbed). **jalwa-ai fully ported.** |
| E — terminal UI | ✅ **done** — `renderers`, `app`, `widgets`, `tui` incl. the **interactive run loop over darshana** (`jlw_tui_run`: tty raw/alt-screen, poll(2)-gated 50ms tick, full-screen repaint, hand-rolled CSI/SS3 key decoder, real inotify watcher auto-remove, engine-event handler now fed by **real background playback** — Enter plays a track in a thread, the loop polls `TrackFinished` and advances the queue). Reviewed adversarially (2 confirmed bugs fixed: stdin-EOF busy-loop, non-arrow-CSI search wipe). **jalwa-ui fully ported.** |
| F — desktop GUI | ✅ **done (Wayland present is smoke-only)** — portable logic done (`theme`, `art_cache`, `app`, `views`). **Draw layer Phase 1 ~DONE (`gui/draw.cyr`)**: a pure headless-testable **draw-command IR** (`JlwDrawCmd` + `JLW_DRAW_*`) + `color_to_xrgb` bridge + ALL view builders — **sidebar, transport, library** (own scroll/virtualization + search + selection/playing highlight), **now_playing, queue, equalizer** (10-band gain bars), **devices, video** — plus **`build_frame`** (screen split sidebar\|content\|transport + view dispatch) and **`key_to_action`** (input map). 65 headless asserts. **Phase 2 DONE (`gui/raster.cyr`)**: CPU rasterizer executing the command list into an XRGB8888 buffer (wl_shm layout) — `fill_rect`/`border`/glyph primitives (bounds-clipped), **UTF-8-aware text** over kashi's VGA 8x16 font (codepoint decode + CP437 fold, 1 cell/codepoint), the `jlw_gui_raster` executor, and a PPM dumper. 40 pixel-level asserts + a full-frame render verified visually. **Phase-3 input seam DONE (`gui/input.cyr`)**: pure `jlw_gui_evdev_to_key(code, shift)` (raw evdev keycode → jalwa key space, US QWERTY, ported from puka) feeding `key_to_action` — the full Wayland keyboard path (evdev→key→action) is unit-tested (39 asserts) with no display. **Phase 1/2 adversarially reviewed (2026-07-10, 5 confirmed / 5 refuted)** — fixed a UTF-8 over-read past NUL + 3 rust-old parity gaps (library empty-state / "No matches", two-line now_playing+queue guidance, `a`=enqueue / `d`=remove bindings). **Control layer DONE (`gui/control.cyr`)**: `jlw_gui_apply_action(app, action)` — every GUI action → a concrete, view-aware app/engine mutation (nav-clamp, view-cycle, enqueue/remove, Enter-play, volume, mute, EQ toggle/±1dB nudge, next/prev, normalize, quit), grounded in rust-old's per-view handlers + tui.cyr semantics; plus `jlw_gui_handle_key_event` (the full evdev→key→action→apply path). 37 asserts covering every action's effect. **Phase 3 present shell DONE, smoke-only (`gui/present.cyr` + `gui/wayland.cyr`)**: `jalwa gui` runs a real render loop (open→paint→poll(2)→dispatch keys/resize/engine-events/close→repaint) over a **puka-forked sovereign Wayland client** (`gui/wayland.cyr`, 640 LOC, byte-faithful rename — 620 protocol literals identical to the oracle; memfd/mmap XRGB8888 wl_shm buffer, xdg-shell, evdev keyboard). **UNVERIFIABLE headless** — needs a live compositor on real AGNOS; here it degrades cleanly ("no Wayland compositor"). Path: **CPU framebuffer** (puka `fb` + kashi font dep), NOT mabda GPU; **dhancha = design template, not a dep**. **Polish DONE**: grid-mode library (+ grid nav / `g` toggle), clip-stack (raster honors CLIP_PUSH/POP; list+grid emit it), interactive search text-entry (`/` → live filter, Esc/Enter), lettered album-art placeholder. **Visual-language redesign DONE (2026-07-12, [ADR 0003](../adr/003-visual-language-redesign.md))**: `gui/theme.cyr` now ships a deep near-black "void" palette + **three selectable dark themes** — **Aurora Void** (default, luminous cyan→violet), **Caustic Glass** (aqua→teal), **Sacred Bloom** (gold core) — as the 8-slot active palette every builder already reads; `jlw_gui_theme_apply/cycle`, **`t`** cycles live, **`$JALWA_THEME`** picks at launch. Diverges from rust-old's egui palette by design; `gui_theme.tcyr` is now a 51-assert design-spec test. **Backlog (P3/P4): mouse/pointer input, real album-art blit** (needs an image decoder). video still aethersafta-blocked. |
| G — binary + MCP | ✅ **done** — `mcp` (8 tools), `mcp_serve` (**real stdio JSON-RPC 2.0 loop** — `jalwa mcp` is a working MCP server; hand-rolled in pure Cyrius mirroring rust-old `run_on`, no bote dep, byte-faithful envelopes incl. serverInfo name=jalwa / protocolVersion 2024-11-05), `main` (args dispatch), **full assembly builds & runs** (968K binary). |

**Modules: all jalwa-core + all non-video jalwa-playback + all jalwa-ai + all jalwa-ui (incl. TUI run loop) + jalwa-gui full stack (draw→raster→control→input→Wayland present) + `mcp` + `mcp_serve` + `main` (full assembly) ✅. 31 `.cyr` modules; ~30 / 33 rust modules ported.**
Backlogged (post-v1, priority order in [`roadmap.md`](roadmap.md)): **P1 video** (`video_decode_thread`, `view_video`, video probe — tarang + aethersafta both still Rust); **P2 MPRIS export** (samvada is D-Bus client-only); **P3 mouse/pointer input** (GUI is keyboard-only; needs `wl_pointer` + a hit-test layer); **P4 real album-art blit** (needs an image decoder); then GUI/audio polish — mid-track-live EQ + gapless (device-untestable), OGG/AAC/MP4 decode (shravan decoders broken).

## Tests

- 35 `.tcyr` suites, **1224 assertions, all green**. Includes `scanner_probe` 39 + `audio_decode` 34 (decode → f64→i16 → threaded async-playback → EQ + normalize) + `engine_playback` 27 (threaded play→TrackFinished→Stopped + transport pause/resume/live-volume/seek + open-cancels-playback, over the fnptr seam) + `mcp_serve` 34 + `tui_runloop` 38 + GUI (141 portable logic + `gui_draw` 83, `gui_raster` 51 (fill_rect/border/glyph/UTF-8 text/clip/art + executor + PPM), `gui_input` 39 (evdev→key→action), `gui_control` 55 (apply_action + grid nav + search entry + keystroke path)). Fixtures `tests/fixtures/probe.{wav,flac,mp3}`. Bare `cyrius test` (CI) all green.
- **Adversarial correctness review (2026-07-10)** of the audio/threading/parsing code fixed 5 confirmed bugs: (high) `open()` while PLAYING orphaned the decode thread + spawned a second — restored rust-old's `open()→stop()`; (med) MP3 frame-probe `foff<n`→`foff+4<=n` (3-byte over-read); (low) tools/call tool-name now a depth-aware top-level scan (nested `arguments.name` can't shadow it); (low) JSON string scans made escape-aware; (low) `jalwa_recommend` now honors `max`. `main`: assembly builds & runs (2.33MB); `jalwa scan`/`play`/`info`/`mcp` end-to-end smoke-tested (`info` shows real Format/Duration/Codec/tags; `play` decodes real audio, falls back cleanly when no ALSA device); `poll(2)` verified on agnos.
- **Adversarial GUI review (2026-07-10)** of the new draw+raster layer (5-dimension fan-out, 3-skeptic verify per finding) fixed 5 confirmed / rejected 5 false-positives: (med) `jlw_gui__utf8_at` over-read past NUL on a truncated 3/4-byte lead — now validates each continuation byte before loading the next; (med) library view drew a blank panel when empty — restored rust-old's "Library is empty…" / "No matches"; (low) `now_playing`/`queue` empty states dropped rust-old's 2nd guidance line; (low) `key_to_action` was missing `a`=enqueue / `d`=remove. All 4 have regression tests.

> Toolchain drift: `cyrius.cyml` pins 6.4.29; cycc is now 6.4.42. Builds pass against the 6.4.29-vendored `lib/`; benign pin-drift warning. Bump the pin + `cyrius lib sync` when convenient.
> Benign build warning: `duplicate fn 'detect_format'` (shravan audio-format vs sankoch compression-format) — jalwa calls neither (uses its own `jlw_detect_audio_format`), so it's inert like the abaco ERR_INVALID noise.
- Per-module `.tcyr` suites land with each ported module, cross-checked against `rust-old/` `#[test]` blocks.

## Dependencies

- **stdlib** (via `cyrius lib sync`): string, fmt, alloc, vec, str, syscalls, io, args, assert, math, random, chrono, patra, fs, yukti, hashmap, tagged, process, **ganita, thread, fnptr, bayan** (last 4 added for shravan)
- **dist repos** (`[deps.X]` via `cyrius deps`): **dhvani 2.2.1 + abaco 2.3.2 + darshana 0.9.0 + shravan 2.6.7 + sankoch 2.4.9 + vani 1.0.0 wired ✅** (54 deps locked); ai-hwaccel, bote, chitra, dhancha to wire per-wave. vani needs stdlib freelist/sakshi/atomic/sync/thread_local (added).
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

4. ✅ **Real AUDIO PLAYBACK (first slice)** — `jalwa play` decodes via shravan → volume → vani out
   (`src/playback/audio.cyr`), synchronous/blocking. Testable seam (`jlw_audio_decode` + `jlw_audio_f64_to_i16`)
   covered; the vani output loop is smoke-only (no device under CI).

**Next — audio is essentially complete.** Background threaded playback + FULL transport (play/pause/stop/
seek/live-volume) + graphic **EQ** are all wired end-to-end (engine fnptr seam → TUI plays live, advances
the queue, transport keys + EQ-view band adjustments all take effect). Remaining polish (low value/hard to
verify without a device):
- **Mid-track-live EQ** — EQ is applied upfront per-track; a band change mid-track applies on the next track.
  Making it live means re-EQ'ing per chunk (stateful biquads, per-chunk buffers) — complex + device-only.
- **Loudness-normalize in the loop** (dhvani `jlw_dsp_normalize` is ported) and **gapless / prepare_next**.
Everything else remains VIDEO-blocked (tarang+aethersafta): `video_decode_thread`/`view_video`, GUI
dhancha/Wayland present; and **MPRIS export** (samvada is D-Bus client-only).

- **Still blocked (VIDEO only):** `video_decode_thread`/`view_video`, `cmd_info` video probe (**tarang** +
  **aethersafta**); GUI dhancha/Wayland render loop (XL; on-screen present needs **aethersafta**);
  **MPRIS export** (**samvada** is D-Bus client-only — needs a session-bus server it lacks).
See [`roadmap.md`](roadmap.md) and [`port-audit.md`](port-audit.md).
