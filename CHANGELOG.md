# Changelog

## [1.4.3] - 2026-08-02

### Changed — cyrius pin 6.4.71 -> 6.5.5; mishran 0.5.3, setu 0.7.1, rupa 0.1.2, vani 1.1.3, kashi 1.0.4

Part of the whole-desktop-stack toolchain catch-up cut on this date.

⭐ **This bump clears a real P0 for this repo specifically.** cyrius **6.4.75** fixed *"`fn_table`
growth past 8192 silently corrupted six fn-indexed side tables"*, and jalwa's compiled unit is the
largest in the desktop stack. Every build at 6.4.71 was made with those tables corrupted, silently.
⚠ The exact function count was never established — a `grep`-based estimate is unsound because
`lib/` carries modules the compiler never prepends, and only `CYRIUS_STATS=1` answers it. The bump
makes the question moot rather than answering it.

⚠ **`[deps.sankoch]` deliberately held at 2.4.9** while 2.7.6 is on disk — three minors, and this
repo is not the sweep's to decide. Its own bite.

### Verification

Host build green; **35 suites** pass.

## [1.4.2] - 2026-07-23

### Changed — setu 0.7.0 (`SETU_SURF_PREMULTIPLIED`) + dep refresh

No behaviour change: the flag is opt-in and this client does not set it.

## [1.4.1] - 2026-07-23

### Changed — setu 0.6.0: client buffers are GPU-visible on agnos

Picks up `setu` **0.6.0**, whose `setu_buf_create` now asks for `shm_create_gpu` **#86** before falling back
to `shm_create` **#71**.

⚠ **Why this matters beyond a version number.** `#71` allocates **system RAM**, which the agnos GPU cannot
reach at all — bus-master is off by design and the engines see only the framebuffer aperture. The kernel
rejects a `#71` slot at both GPU entry points (`gpu_blit_shm` #87: `src_mc == 0 ⇒ the GPU cannot read it`;
`gpu_shader_op` #92: `GPO_E_BADSLOT`). Every shared surface in the desktop was allocated that way, so the
whole iron-proven ring-3 GPU band had **no reachable consumer**. Buffers from this release are eligible for
a hardware blit.

No API change and no call-site change here — the buffer id behaves identically, and `#86` falls back to
`#71` automatically on a machine with no GPU carveout (every QEMU boot).

### Changed — cyrius pin → 6.4.71

## 1.4.0 — "System (Desktop)" theme: follow the shared desktop theme (2026-07-12)

jalwa keeps its own Aurora / Caustic / Sacred identity and gains a **fourth theme option,
"System (Desktop)"**, that pulls in the shared AGNOS desktop theme (**rupa**) so the player can
match the compositor and every other app. Founder-directed; additive only — the existing themes
and the ~50 draw call sites are untouched.

### Added
- **`[deps.rupa]` (0.1.0)** — the shared desktop theme-token core (MUDRA / SHANTA).
- **`JLW_GUI_THEME_SYSTEM` (id 3)** — a new selectable theme (COUNT 3 → 4). Selecting it (the
  `t` cycle, `JALWA_THEME=system`, or `jlw_gui_theme_apply`) reseats the active `JLW_GUI_*`
  palette from rupa's active theme (bg/panel/widget/accent/text ← rupa tokens, packed logical
  `0xRRGGBB` → jalwa's `0xRRGGBBAA`; `ACCENT_DIM` derived as a dimmed accent). Re-apply to
  re-sync if the desktop theme changes at runtime. `jlw_gui_theme_from_name("system")` +
  `jlw_gui_theme_name` → "System (Desktop)".
- **`tests/gui_theme.tcyr`** — asserts System pulls rupa's MUDRA · Carbon default (and follows a
  SHANTA switch); suite 50 → **64**. `tests/gui_control.tcyr`'s theme-cycle test updated for the 4th
  theme (3 cycles → System, 4 → Aurora). Both targets build (host 2.66 MB / agnos 2.60 MB); full
  test suite **1364/1364** across 36 files.

### Note
jalwa is a parallel-agent repo; this change was founder-directed (2026-07-12) and is deliberately
additive (only the SYSTEM branch of `gui/theme.cyr` + the rupa dep) to avoid conflicting with
in-flight work.

## 1.3.0 — Real album art (2026-07-12)

Embedded cover art now renders in the GUI ([ADR 0004](docs/adr/004-real-album-art-via-chitra.md),
reversing [ADR 0002](docs/adr/002-audio-probe-via-shravan-no-album-art.md)'s art drop) — the
P4 backlog item.

### Added
- **`chitra` dependency** (`[deps.chitra]` → `dist/chitra.cyr`, v0.3.0) — a pure-Cyrius CPU
  image decoder (PNG + baseline JPEG → RGBA8).
- **Embedded cover extraction** (`src/core/scanner.cyr`) — jalwa-owned FLAC
  `METADATA_BLOCK_PICTURE` (type 6) and ID3v2 `APIC` parsers (`jlw_extract_cover_art`);
  shravan has no picture API. FLAC + MP3.
- **Real album-art blit** — a new alloc-free `jlw_gui_fb_blit_rgba` (nearest-neighbor scale
  + integer alpha-over, clip-aware) renders decoded covers in Now-Playing, the Library grid,
  and the Mini-player; the lettered placeholder remains the no-art fallback.
- **Decoded-art cache** — `gui/art_cache.cyr` now stores decoded RGBA (LRU by Uuid) and
  `jlw_gui_art_get(id, path)` extracts+decodes **on demand from the file path**, once per
  cover (never per frame). Art is **not** persisted in the DB (no schema change, no bloat;
  re-derived from the path across restarts).

### Notes
- Baseline JPEG + 8-bit PNG covers only; progressive/CMYK JPEG and interlaced PNG fall back
  to the placeholder. WAV/OGG/MP4 cover extraction is deferred.

## 1.2.3 — Mini-player (design→code arc complete) (2026-07-12)

Final step of the [design→code arc](docs/development/roadmap.md): the **Mini-player**
(from `docs/development/design/Jalwa Mini-Player.dc.html`). With this, every non-visualizer
surface of the "Jalwa Visual Language" is in code.

### Added
- **Compact mini-player window mode** — press **`z`** to collapse the full
  sidebar│content│transport frame into a single compact player: album art (framed),
  title/artist/album, transport glyphs (prev/play/next), and a luminous seek + `M:SS / M:SS`.
  Press `z` again to return to the full frame. New render-mode flag `jlw_gui_mini_mode`
  (toggled by `JLW_GUI_ACT_TOGGLE_MINI`); `jlw_gui_build_frame` renders `build_mini`
  instead of the full split when it's on. Picks up all three themes via the active palette.

## 1.2.2 — Queue and Search views (2026-07-12)

Third step of the [design→code arc](docs/development/roadmap.md): the **Queue** and
**Search** screens adopt the visual language (from
`docs/development/design/Jalwa Queue & Search.dc.html`).

### Added
- **Queue header + chrome** — an accent `QUEUE` eyebrow and a `N tracks` count; the
  current-position row reads in accent and the selected row gets an accent edge bar.
- **Search screen** — entering search (`/`) now presents the Library as a dedicated
  Search view: a `SEARCH` eyebrow, a focused query field showing the live query with a
  cursor (`> vela_`) and an `esc` affordance, a `N matches` count, and footer hints
  (`Up/Down navigate · Enter play · Esc clear`). Previously the typed query was invisible.

### Changed
- Queue rows render below the header (virtualization adjusts). Both screens pick up all
  three themes automatically via the active palette.

## 1.2.1 — Library view (2026-07-12)

Second step of the [design→code arc](docs/development/roadmap.md): the **Library view**
adopts the visual language (from `docs/development/design/Jalwa Library.dc.html`).

### Added
- **Library header** — an accent `LIBRARY` eyebrow and a live, search-aware count
  (`N tracks`, or `N matches` while filtering) above the list/grid.
- **Consistent selection chrome** — the selected list row gets an accent edge bar
  (matching the sidebar and Settings), and the selected grid cell gets an accent frame.

### Changed
- The list/grid now renders in the region **below** the header (virtualization row count
  adjusts accordingly). All three themes apply automatically via the active palette.

## 1.2.0 — Visual language: themes, settings, luminous chrome (2026-07-12)

The start of the **design→code arc** (see the 1.2.x plan in
[`roadmap.md`](docs/development/roadmap.md)): jalwa's GUI adopts the "Jalwa Visual
Language" north-star. This release lands the color system, an in-app theme switcher,
and the first restyled surfaces (Now-Playing hero + shared chrome); subsequent 1.2.x
releases port the remaining per-view designs.

### Added
- **Settings view with a theme picker** (`Tab` to it, or the `Settings` sidebar entry).
  Up/Down select a theme, `Enter` applies it; each row previews the theme's accent as a
  swatch and tags the active one. `t` still cycles from anywhere. New view
  `JLW_GUI_VIEW_SETTINGS`; the picker reseats the active palette via `jlw_gui_theme_apply`.
- **Now-Playing hero restyle** — an accent `NOW PLAYING` eyebrow, a framed album-art
  panel, the primary/secondary/muted type hierarchy, and a luminous in-view seek line
  (glowing accent fill + playhead knob + `M:SS / M:SS` readout).
- **Luminous shared chrome** — the active sidebar row gains an accent edge bar; the
  transport seek bar glows with an accent playhead knob and shows an elapsed/total time
  readout.

### Changed
- **Visual-language redesign of the GUI palette** ([ADR 0003](docs/adr/003-visual-language-redesign.md),
  from the "Jalwa Visual Language" north-star). `src/gui/theme.cyr` keeps jalwa's
  dark-and-cyan soul but deepens the base to a near-black **void** the accent glows
  against, and brightens primary text to ~#F2F3F7. This intentionally diverges from the
  rust-old egui palette, so `tests/gui_theme.tcyr` is now a design-spec test (not a
  parity test).

### Added
- **Three selectable dark themes**, colors taken from the designer's per-view mockups
  in `docs/development/design/`. **Aurora Void** (default — luminous cyan `#3DE7FF`),
  **Caustic Glass** (bright cyan `#6FE6FF`), and **Sacred Bloom** (gold core `#FFD76B`
  over a warm void), all over a near-black void with `#F2F3F7` text. They reseat the
  same 8 `JLW_GUI_*` slots every view
  builder already reads, so switching is free downstream. Press **`t`** in the running
  GUI to cycle themes; set **`JALWA_THEME=aurora|caustic|sacred`** to pick one at launch
  (`jlw_gui_theme_apply` / `jlw_gui_theme_cycle` / `jlw_gui_theme_init_from_env`).

## 1.1.0 — AGNOS desktop window (2026-07-10)

jalwa now runs as a **window on the sovereign AGNOS desktop** — composited by the
aethersafha compositor over the setu display protocol, alongside crab and cyrius-doom.
All AGNOS changes are `#ifdef CYRIUS_TARGET_AGNOS`-gated; Linux/Wayland behavior is
byte-unchanged.

> ⛔ **RETRACTED 2026-08-03 — this headline is a FALSE GREEN.** jalwa never ran as a window on an
> ordinary AGNOS boot. The only run that showed it was `aethersafha-jalwa-smoke.sh`, whose kernel was
> built with `AETHERSAFHA_SETU_SELFTEST=1` — the hook assigned `net_ip = 0x7F000001`, which on the
> agnos of that date (**before `net_src_for`, agnos 1.56.34**) was the sole reason a setu-over-TCP
> connect could match its 4-tuple. Hook and script are deleted. ⚠ Scope note: after `net_src_for`
> un-rigged setu clients did connect and present (QEMU `-smp 1`, 2026-08-02); jalwa was not one of
> them, so this headline stays void. TCP-on-loopback is retired as the desktop transport for being
> the **wrong primitive**, not for being broken (replacement: the agnos socket `anu`, agnos
> `docs/development/planning/ipc.md` §9/§10). The `#ifdef`-gated backend code stands; the AGNOS
> desktop claim does not, and must be re-proven on `anu`.

### Added
- **AGNOS desktop launch.** Spawned bare on AGNOS (no args — how the compositor launches
  a resident as `/bin/puka`), `main()` defaults to the GUI window instead of printing usage,
  mirroring a desktop app booting its own window. The existing `src/gui/setu_present.cyr`
  backend presents each frame through the `sys_shm` shared-buffer path
  (`setu_buf_create → buf_write → attach_buf fmt=1 → commit`) — the same path cyrius-doom
  uses. ~~Validated: `agnos scripts/aethersafha-jalwa-smoke.sh` (QEMU; gnoboot+OVMF+NVMe) —
  jalwa **and** crab both present surfaces in one run (a real multi-window desktop).~~
  ⛔ **RETRACTED 2026-08-03 — FALSE GREEN.** `aethersafha-jalwa-smoke.sh` built its kernel with
  `AETHERSAFHA_SETU_SELFTEST=1`, a hook that assigned `net_ip = 0x7F000001` so a setu client's
  TCP connect to the compositor could match a 4-tuple that, **before `net_src_for` (agnos
  1.56.34)**, it otherwise could not. On an ordinary boot of that era jalwa could not have
  connected at all. (`net_src_for` later fixed that defect and un-rigged clients did connect —
  QEMU `-smp 1`, 2026-08-02 — but jalwa was not among them.) Hook and script are **deleted**. The
  `setu_present.cyr` backend code is not withdrawn — only the claim that it was proven on agnos,
  and the TCP transport under it. See agnos `docs/development/planning/ipc.md` §9-§10.
- **`jlw_plib_new_empty()`** — an in-memory empty-library fallback (`src/core/db.cyr`). A
  fresh desktop with no on-disk library db (`jlw_open_library()` returns 0) now opens an
  empty library instead of faulting on a null handle at first paint.

### Changed
- **AGNOS GUI window is 800×600** (was 900×600). The AGNOS setu present copies each whole
  frame through a single 2 MB shm page (kernel `SHM_MAX_SIZE`); 900×600×4 = 2.06 MB
  overflowed it, so present failed. 800×600 = 1.92 MB fits (and sits inside the 1280-wide
  desktop). Linux/Wayland keeps the 900×600 default.
- Audio deps bumped to the two-proc mixer stack: **mishran 0.4.1** (cooperative-yield pump)
  + **vani 1.1.0** (non-blocking sink API).

### Notes
- GUI startup touches **no audio** (audio fires only on a play action), so the compositing
  path needs no mishran daemon resident.

## 1.0.0 — Rust → Cyrius port

Complete port of the 5-crate Rust workspace (frozen at `rust-old/` as the parity
oracle) to pure **Cyrius**. Every non-video subsystem is ported and green against the
oracle; the binary builds and every non-blocked subcommand runs end-to-end.

### Core (jalwa-core → `src/core`)
- Domain types, enums→int codes, Uuid/Playlist/PlayQueue/MediaItem/Library (ADR 0001: linear-scan indexes)
- DB over **patra**, playlist I/O (M3U), inotify watcher, hardware (**yukti**) + `jalwa devices`
- **Real audio scanner** — header-only WAV/FLAC/MP3 duration + FLAC-Vorbis / MP3-ID3v2 tags via **shravan** (ADR 0002)

### Playback (jalwa-playback → `src/playback`)
- **Real decode → output**: shravan `wav/flac/mp3_decode` → f64→i16 → **vani** ALSA/agnos output (not PipeWire)
- **Threaded engine** over an fnptr audio-backend seam; full transport: play/pause/resume/stop/seek/live-volume
- **DSP chain** via **dhvani**: 10-band graphic EQ + loudness normalize (per-track)
- OGG/AAC/MP4 excluded (shravan decoders broken); mid-track-live EQ + gapless deferred

### AI (jalwa-ai → `src/ai`)
- Recommendations, daimon, fingerprint scaffold (network + kernel stubbed)

### Terminal UI (jalwa-ui → `src/ui`)
- Interactive `jalwa tui` run loop over **darshana** — raw/alt-screen, poll(2) tick, hand-rolled CSI/SS3 decoder, live background playback + queue auto-advance

### Desktop GUI (jalwa-gui → `src/gui`)
- Headless-testable **draw-command IR** → **CPU rasterizer** (kashi VGA 8x16 font, UTF-8-aware, XRGB8888 wl_shm buffer) → **control layer** (action → app/engine mutation) → evdev input map
- All views: sidebar, transport, library (list + grid), now-playing, queue, equalizer, devices; interactive search text-entry; clip-stack; lettered album-art placeholder
- `jalwa gui` — a real Wayland present shell over a puka-forked sovereign client (smoke-only headless; validate on real AGNOS). CPU framebuffer path, not GPU
- Real album-art blit + mouse input backlogged

### Binary + MCP (jalwa-mcp → `src/mcp*`, `src/main.cyr`)
- `jalwa mcp` — real stdio JSON-RPC 2.0 server (8 tools), hand-rolled, no bote dep
- Full-assembly `main` with CLI dispatch (scan/play/info/search/stats/library/export/import/devices/tui/gui/mcp)

### Quality
- 35 `.tcyr` suites, 1200+ assertions, green in CI (`cyrius test`)
- Two adversarial-review passes (audio/threading/parsing; GUI draw+raster) — real bugs found + fixed

### Blocked
- **Video (P1)** — tarang + aethersafta are still Rust; video decode/surface deferred (see `docs/development/roadmap.md`)
- MPRIS export deferred (samvada is D-Bus client-only)

---

> Entries below are the **pre-port Rust history** (calendar-versioned), kept for provenance.

## 2026.3.22

Hardware device integration, full engineering backlog resolution, and audit/refactoring pass.

### Phase 7 — Hardware Media Sources (complete)
- Integrated **yukti 0.22.3** (AGNOS device abstraction layer) for USB, optical, and udev hotplug
- New `jalwa-core::hardware` module: `HardwareManager` wraps yukti with media-player-specific events
- USB auto-detect: mounted USB storage emits `UsbMounted` event, auto-adds mount point as scan path
- Optical drives: disc insert/eject detection, TOC reading (`read_toc`), tray control (`open_tray`/`close_tray`)
- Hotplug: raw udev events translated to `HardwareEvent` variants (UsbMounted, UsbRemoved, DiscInserted, DiscEjected, PlaybackDeviceRemoved, DeviceError)
- Graceful device removal: detects when a playing device is removed, stops playback and notifies UI
- `is_on_removable_device()` helper for checking file paths against mounted devices
- New CLI command: `jalwa devices` lists detected USB storage and optical drives
- New GUI view: Devices panel in sidebar with hardware notifications and device listing

### Playback engine refactoring
- `Arc<Mutex<DecodeStatus>>` replaced with `Arc<RwLock<DecodeStatus>>` — readers no longer block the decode thread
- Engine command channel changed from unbounded to `sync_channel(32)` with backpressure
- `state()` now reads from RwLock instead of stale local field
- Decode errors: skip up to 10 consecutive bad frames before stopping (was: stop immediately)
- PipeWire output: retry once on write failure before giving up (was: fail immediately)
- `smooth_gain` reset on seek to prevent normalization artifacts
- New `PrepareNextFailed(String)` event for gapless transition errors (was: silent drop)
- Paused decode thread uses `recv_timeout(1s)` instead of indefinite blocking
- Section-header comments added to the 250-line `decode_loop()` function

### Scanner + database improvements
- Scanner WalkDir capped at `max_depth(20)` to prevent symlink loop infinite traversal
- New `ScanResult` struct returns `files`, `errors`, and `dirs_walked` (was: just `Vec<ScannedFile>`)
- DB error messages include table/operation context (e.g. "prepare media_items select" instead of "prepare")
- Unknown codec fallback values now log `tracing::warn!`

### TUI improvements
- Engine errors shown in status bar as `[ERR: message]` in bold red, auto-clears after 5 seconds
- "Library is empty" help text shown when no items
- Search query input capped at 256 characters
- EQ band access via `.get()` for bounds safety

### GUI improvements
- Search results use `std::mem::take` instead of `.clone()` per frame — zero-cost move
- `no_art` HashSet clears at 1000 entries to prevent unbounded memory growth
- EQ band access via `.get().copied().unwrap_or(0.0)` for bounds safety

### MCP + DSP fixes
- Malformed JSON input now returns proper JSON-RPC `-32700` parse error response (was: silently skipped)
- `chunks_exact(4)` guarded with alignment check and `tracing::warn!` on non-aligned buffers

### Production safety
- 3 `.unwrap()` calls removed from production code (video_decode_thread, PlaybackEngine)
- PlayQueue `advance()`/`go_back()` use `.get()` instead of direct indexing
- `detect_optical_type()` correctly maps iso9660 to CdData, udf to DvdRom
- MPRIS server probes channel every 5s and shuts down cleanly when receiver drops
- `#[inline]` on trivial getters (`is_audio`, `is_video`, `current`, `len`, `is_empty`, `progress`)

### Testing
- 411 tests (was 337), +74 new tests (+22%)
- 7 new integration tests in `tests/integration.rs` (hardware lifecycle, library persistence, playlist I/O)
- 6 new playlist_io tests (roundtrip, empty, comments, nonexistent, creates file, empty playlist)
- 27 unit tests in `hardware.rs`
- Tests for TUI error display, search cap, MPRIS shutdown, optical detection, DSP alignment

### Benchmarks
- 18 benchmarks across 4 suites (was 6 across 2)
- New `benches/hardware.rs`: event processing, device lookup, removable device check, display formatting
- New `benches/video.rs`: frame creation, clone, RGB conversion at 720p and 1080p

### Infrastructure
- `VERSION` bumped to 2026.3.22
- `Makefile` expanded: audit, deny, coverage, doc targets; `check` = fmt + clippy + test + audit
- `scripts/bench-history.sh`: CSV benchmark tracking + 3-point trend markdown generation
- `scripts/version-bump.sh`: version sync across workspace
- Engineering backlog from 2026-03-19 audit: 23 of 25 items resolved, completed items removed from roadmap
- Phase 7 items removed from roadmap (complete)

## 2026.3.19

Tarang crates.io migration, aarch64 build fix, and full security/performance audit.

### Tarang crates.io migration
- Replaced 5 git-pinned subcrate dependencies (`tarang-core`, `tarang-demux`, `tarang-audio`, `tarang-video`, `tarang-ai` from `github.com/MacCracken/tarang` tag `2026.3.16-1`) with a single `tarang = "0.19.3"` from crates.io
- Tarang is now an optional feature (`tarang`) included in `default` — build with `--no-default-features` to compile without it
- Feature propagates through workspace: `jalwa/tarang` enables `jalwa-core/tarang`, `jalwa-playback/tarang`, `jalwa-ai/tarang`, `jalwa-ui/tarang`
- Updated all import paths from subcrate style (`tarang_core::`, `tarang_audio::`, `tarang_ai::`) to umbrella module style (`tarang::core::`, `tarang::audio::`, `tarang::ai::`)
- Adapted to published API: `MediaInfo::audio_streams()` returns an iterator (`.next()`) instead of a slice (`.first()`)

### aarch64 build fix
- Gated tarang usage behind `cfg(feature = "tarang")` across all crates — aarch64 release builds with `--no-default-features` now compile cleanly
- Defined fallback `ContainerFormat`, `AudioCodec`, `VideoCodec` enums (with `Display`) when tarang feature is disabled
- Gated `scanner`, `fingerprint`, `decode_loop`, DSP functions, and `MediaItem::from_probe` behind feature flag
- Added stub `open`/`play`/`scan`/`info` implementations that return helpful errors when tarang is unavailable

### Security hardening
- **MCP mutex safety**: Replaced all `.lock().unwrap()` in MCP tool functions with error-returning match — prevents server crash on poisoned mutex
- **MCP path validation**: Added `validate_path()` with `canonicalize()` for all file/directory inputs from MCP clients — prevents path traversal
- **API key redaction**: Manual `Debug` impls on `DaimonConfig` and `HooshConfig` redact `api_key` as `[REDACTED]` — prevents credential leakage in logs
- **Safe JSON access**: Replaced unsafe `result["content"][0]["text"]` array indexing in daimon.rs with `.get()` chains — prevents panic on malformed API responses
- **Album art size limits**: Scanner rejects embedded art >5MB; GUI rejects art images >2048×2048 before RGBA conversion — prevents memory exhaustion from malformed media

### Performance
- **O(1) library lookups**: Added `HashMap<Uuid, usize>` and `HashMap<PathBuf, usize>` indexes to `Library` — `find_by_id` and `find_by_path` are now O(1) instead of O(n)
- **Audio buffer reuse**: Added reusable scratch buffer to `Equalizer`; new `apply_volume_in_place` avoids allocating a new `AudioBuffer` per decode loop iteration
- **Parallel fingerprinting**: `find_similar_local` now uses `rayon::par_iter()` for concurrent fingerprint computation across library items
- **MCP response pagination**: Library list capped at 200 items, search results at 100 — prevents multi-MB JSON responses

### Data integrity
- **SQLite transactions**: `save_playlist` and `delete_item` now wrapped in BEGIN/COMMIT/ROLLBACK — prevents inconsistent state on crash
- **Corruption logging**: UUID parse failures and datetime parse failures in database loading now emit `tracing::warn!` with raw values instead of silently falling back

### Version bump
- All crates bumped to 2026.3.19

## 2026.3.18

Polish release: MCP stdio integration tests, GUI headless tests, library grid view, lru advisory tracking.

### MCP stdio integration tests
- Refactored `mcp::run()` to delegate to generic `run_on<R, W>()` accepting any `AsyncBufRead` + `AsyncWrite` — enables testing the full JSON-RPC loop without a real terminal
- 6 new async integration tests: `run_initialize`, `run_tools_list`, `run_tool_call_status`, `run_unknown_method`, `run_malformed_json_skipped`, `run_multiple_requests`
- Covers protocol handshake, tool listing, tool dispatch, error handling, malformed input resilience, and multi-request sequencing

### GUI headless integration tests
- Added `GuiApp::new_headless()` constructor (test-only) that bypasses MPRIS D-Bus and filesystem watcher — no display server or D-Bus daemon required
- 10 new tests using `egui::Context::default()` + `ctx.run()` for headless frame simulation
- Tests: `headless_library_view_empty`, `headless_now_playing_view`, `headless_queue_view`, `headless_equalizer_view`, `update_search_empty_query`, `update_search_filters`, `list_len_library`, `play_item_valid_index`, `play_item_invalid_index`, `view_switching`

### Library grid view
- New `LibraryViewMode` enum (`List` / `Grid`) with toggle buttons in library search bar
- Grid view renders 120x120 album art thumbnails with title/artist text in a responsive wrapping grid
- Album art loaded via existing `ArtCache` (LRU texture cache); placeholder music note for items without art
- 4-directional arrow key navigation in grid mode (left/right within row, up/down between rows)
- Click to select, double-click to play, Enter to play, A to enqueue — same bindings as list view

### ratatui lru advisory tracking (RUSTSEC-2026-0002)
- Added `deny.toml` with `cargo-deny` configuration
- `RUSTSEC-2026-0002` (lru 0.12.5 Stacked Borrows unsoundness in `IterMut`) documented and ignored pending upstream ratatui fix
- License allowlist, ban policy, and source policy configured for CI integration

### Version bump
- All crates bumped to 2026.3.18 (calendar versioning)

## 2026.3.16-1

Audio pipeline security audit, tarang upgrade, MCP fixes, fingerprint integration, test coverage push.

### Security & Correctness (audit: 2 HIGH, 3 MEDIUM, 2 LOW)

#### HIGH severity
- **jalwa-playback/dsp.rs**: Unsafe `from_raw_parts` cast from `*const u8` → `*const f32` without alignment check (UB on unaligned `Bytes`) — replaced with `bytemuck::try_cast_slice` + fallback copy via `buf_to_f32_safe()`
- **jalwa-playback/decode_thread.rs**: `apply_volume` same alignment UB — now uses `bytemuck::cast_slice`
- **jalwa-ai/fingerprint.rs**: Decoded audio blindly cast to F32 regardless of actual sample format — added `decode_samples_to_f32()` with I16/I32/F32 dispatch

#### MEDIUM severity
- **jalwa-playback/decode_thread.rs**: EQ biquad state not reset on seek or track change — click/pop transients at seek points. Now calls `equalizer.reset()` on Seek command and gapless transition
- **jalwa-playback/dsp.rs**: EQ hardcoded to 2 channels — channels 3+ passed through unfiltered. Expanded state to `MAX_EQ_CHANNELS = 8`
- **jalwa-playback/decode_thread.rs**: Resample/channel-mix errors silently passed wrong-format buffers to PipeWire — now skips buffer and sends error event instead of outputting at wrong rate/channel count

#### LOW severity
- **jalwa-playback/decode_thread.rs**: Per-buffer normalization gain caused pumping/breathing — added exponential moving average smoothing (fast attack 0.3, slow release 0.05)
- **jalwa-playback/decode_thread.rs**: Volume unity check used `f32::EPSILON` (~1.19e-7) — widened to `1e-4` to avoid unnecessary per-sample multiply from float drift after repeated UI adjustments

### Tarang upgrade: 2026.3.16 → 2026.3.16-1
- Picks up 26 upstream security fixes (18 HIGH, 8 MEDIUM) including: MP3 magic byte panic, `bytes_to_f32` assert panics, unsafe alignment in PipeWire output, NaN panics, MP4 OOM on size-0 boxes, dav1d plane slicing
- Lock-free PipeWire SPSC ring buffer (replaces sleep-based loop)
- openh264 0.6 → 0.9 (fixes RUSTSEC-2025-0008 heap overflow)
- libvpx-sys → env-libvpx-sys 5.1 (eliminates RUSTSEC-2023-0018, RUSTSEC-2018-0017)
- 110+ new upstream tests (200 → 310)

### MCP server fixes
- `jalwa_play`, `jalwa_pause`, `jalwa_status` now use shared `Arc<Mutex<PlaybackEngine>>` — no longer create throwaway engines per call
- `jalwa_pause` actually calls `engine.pause()` and returns real playback status
- `jalwa_status` polls events and returns live engine state
- `jalwa_queue list` reports currently playing track from shared engine
- `jalwa_queue clear` stops playback via shared engine

### Local audio fingerprinting (jalwa-ai)
- New `fingerprint.rs` module: `fingerprint_file()` and `find_similar_local()`
- Decodes first 30s of audio via tarang, downmixes to mono, computes Chromaprint-style hash
- `find_similar_local()` compares seed file against all library items by Hamming distance
- Format-aware: handles I16, I32, F32 decoded buffers correctly
- Dependencies added: `tarang-audio`, `bytes`, `bytemuck`

### Test coverage: 235 tests (was 167)
- **widgets.rs** +14: TestBackend rendering for Library, NowPlaying, Queue, Equalizer views, status bar, keybinds, search mode
- **tui.rs** +28: `handle_normal_input` (quit, tab, nav, volume, mute, search, repeat, shuffle, enqueue, EQ, normalize, delete, clear), `handle_search_input` (type, backspace, escape, enter, nav), `handle_mpris_command` (all 7 MPRIS command variants)
- **decode_thread.rs** +8: play-to-end integration, stop command, volume command, nonexistent file, pause/resume, defaults, debug/clone
- **fingerprint.rs** +3: serialization, nonexistent file, empty library

### Refactoring
- **Shared test fixtures**: Consolidated 5 copies of `make_item()` and 3 copies of `make_test_wav()` into `jalwa_core::test_fixtures` module, used by all crate test suites
- **MCP tool handlers**: Extracted 254-line `handle_tool_call()` into 8 focused functions (`tool_play`, `tool_pause`, `tool_status`, `tool_search`, `tool_recommend`, `tool_queue`, `tool_library`, `tool_playlist`) + `mcp_ok()`/`mcp_err()` response helpers
- **Dead code removed**: `VectorSearchResponse`, `VectorSearchResult` structs and unused `Context` import from `daimon.rs`
- Zero compiler warnings workspace-wide

### Dependencies added
- `bytemuck = "1"` (features: derive) — safe transmute for audio buffer alignment

### Roadmap updates
- Test coverage tiers 1-3 marked complete
- Phase 8 audio fingerprinting marked done
- Phase 6 (Video) annotated as prerequisites-met, planned not started

## 2026.3.16

### Audio Gap Closure

**MPRIS2 D-Bus Media Key Support**
- Hardware media keys: play/pause, next, previous, stop via D-Bus
- Desktop integration: visible to KDE/GNOME/Sway media controls
- MPRIS2 Player interface: PlayPause, Play, Pause, Stop, Next, Previous, Seek, Volume
- Runs on dedicated background thread, non-blocking

**Play Count Tracking**
- Wired into TUI: play count increments on track finish and gapless transition
- Persists to SQLite via `PersistentLibrary::update_play_count()`
- Tracks `last_played` timestamp for recently-played queries

**File Watcher (Auto-Rescan)**
- `LibraryWatcher` now wired into TUI event loop
- New media files in library directories auto-added on creation
- Removed files auto-cleaned from library
- Filters to media extensions only (no false triggers on non-audio files)

**EQ Presets**
- 9 named presets: Rock, Pop, Jazz, Classical, Bass, Treble, Vocal, Electronic, Acoustic
- `Enter` in EQ view cycles through presets
- `EqSettings::preset("rock")` API for programmatic access
- `EqSettings::preset_names()` lists all available presets
- All presets validated to ±12 dB range

### Audio Polish (Phase 5)

**Volume Normalization / ReplayGain**
- Loudness analysis: RMS + peak measurement per buffer
- Normalization gain computation targeting -18 dBFS reference level
- Peak limiter prevents clipping when applying positive gain
- Gain clamped to 0.1x–10x range for safety

**10-Band Graphic Equalizer**
- ISO standard center frequencies: 31, 62, 125, 250, 500, 1k, 2k, 4k, 8k, 16k Hz
- Peaking EQ biquad filters with configurable ±12 dB gain per band
- Per-channel filter state (stereo-aware)
- DSP chain: decode → resample → mix → EQ → normalize → volume → output

**Album Art Extraction**
- `MediaItem` carries `art_mime` and `art_data` fields
- Scanner extracts embedded album art via lofty (prefers CoverFront)
- Supports JPEG, PNG from ID3v2, Vorbis Comment, MP4 atoms

**File Watcher (inotify)**
- `LibraryWatcher` monitors directories for create/modify/remove events
- Cross-platform via `notify` crate

**Testing**
- 146 tests across workspace (46.6% line coverage)

### MVP Release

**Audio Playback (MVP-1)**
- Decode thread with `FileDecoder` → resample → channel mix → volume → PipeWire output
- Channel-based engine commands: Play, Pause, Resume, Stop, Seek, Volume, Mute
- Engine events: StateChanged, PositionUpdate, TrackFinished, NearEnd, TrackChanged, Error
- Real-time position tracking via shared decode status
- `jalwa play <file>` produces audible sound through PipeWire with Ctrl+C handling

**Library Management (MVP-2)**
- Directory scanner: recursive walk, extension filtering, tarang probe + lofty tag extraction
- SQLite persistence: media_items, playlists, playlist_items, scan_paths tables
- PersistentLibrary wrapper: write-through to both in-memory Library and SQLite
- M3U playlist import/export
- CLI: `jalwa scan`, `jalwa library`, `jalwa search`, `jalwa stats`, `jalwa export`, `jalwa import`
- MCP `jalwa_recommend` now returns actual AI-scored recommendations from library

**Interactive TUI (MVP-3)**
- ratatui + crossterm terminal UI launched via `jalwa` (default) or `jalwa tui`
- Four views: Library, Now Playing, Queue, Equalizer (Tab to cycle)
- Library browser with live search filtering (/ to search, Esc to cancel)
- Playback controls: Space (play/pause), Left/Right (seek ±10s), +/- (volume), m (mute)
- Queue management: a (enqueue), d (remove), c (clear), n/p (next/prev), r (repeat), s (shuffle)
- EQ controls: e (toggle/open), Left/Right (adjust band), Enter (cycle presets), N (normalize)

**Gapless Playback (MVP-4)**
- PrepareNext command pre-opens next decoder in decode thread
- NearEnd event fired with <2s remaining for pre-buffering
- Seamless decoder swap on EndOfStream without closing audio output
- Queue-driven auto-advance with gapless transitions

**Bug Fixes**
- Fixed FK constraint order in playlist deletion (children before parent)

### Initial Scaffold

- Core types: media library, playlists, play queue, playback state, search
- Playback engine: tarang-based decode, open/play/pause/stop/seek/volume
- UI layer: TUI status bar, progress bar, queue summary, library browser
- AI features: recommendations (artist/album/tag/duration matching), smart playlists, library insights
- CLI: play, info, search, stats, mcp subcommands
- MCP server: 5 tools (jalwa_play, jalwa_pause, jalwa_status, jalwa_search, jalwa_recommend)
