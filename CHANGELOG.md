# Changelog

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
