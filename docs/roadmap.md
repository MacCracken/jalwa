# Jalwa Roadmap

## Test Coverage Backlog (ongoing)

Current: 55.9% (1038/1858 lines). Target: 80%.

### Tier 1 — Pure rendering (est. +198 lines)
- [x] `widgets.rs`: ratatui TestBackend tests for all views (Library, NowPlaying, Queue, Equalizer) *(2026-03-16)*

### Tier 2 — TUI input handlers (est. +100 lines)
- [x] `tui.rs`: unit test `handle_normal_input`, `handle_search_input`, `handle_mpris_command` *(2026-03-16)*

### Tier 3 — Decode thread + MPRIS (est. +80 lines)
- [x] `decode_thread.rs`: integration test with NullOutput + minimal WAV *(2026-03-16)*
- [x] `mpris.rs`: test MPRIS command dispatch *(already covered in tui.rs mpris handler tests)*

## Completed — Tarang + MCP + AI fingerprinting (2026-03-16 → 2026-03-19)
- [x] Bumped tarang from `2026.3.16` to `2026.3.16-1` (26 security fixes, lock-free PipeWire, 110+ new tests) *(2026-03-16)*
- [x] Migrated tarang from 5 git subcrates to published `tarang 0.19.3` crates.io umbrella crate as optional feature *(2026-03-19)*
- [x] Fixed MCP tool stubs — `jalwa_pause`, `jalwa_status`, `jalwa_queue` now use shared `PlaybackEngine` state
- [x] Wired `tarang-ai` fingerprinting directly into `jalwa-ai` for local similarity search (`find_similar_local`)

## Completed — Hardening & Audit Fixes (2026-03-19)
- [x] Gate tarang behind `cfg(feature)` for aarch64 `--no-default-features` build
- [x] Replace `.lock().unwrap()` → error handling in all MCP tool functions
- [x] Add `validate_path()` with canonicalization for MCP file/dir inputs
- [x] HashMap indexes on Library (O(1) `find_by_id`/`find_by_path` instead of O(n))
- [x] Safe `.get()` chains for JSON API responses in daimon.rs
- [x] Cap MCP library list (200 items) and search results (100 results)
- [x] Validate album art dimensions (2048×2048 max) before RGBA conversion

## Engineering Backlog — from 2026-03-19 audit

### High
- [ ] Wrap multi-step DB operations in SQLite transactions (`jalwa-core/db.rs`)
- [ ] Pre-allocate reusable audio buffers in decode pipeline (`jalwa-playback/decode_thread.rs`, `dsp.rs`)
- [ ] Log warning on corrupted UUID instead of silent regeneration (`db.rs:147,205`)
- [ ] Log warning on invalid datetime instead of silent `Utc::now()` fallback (`db.rs:161`)
- [ ] Add max size validation for embedded album art in scanner (`jalwa-core/scanner.rs`)
- [ ] Zeroize API keys / suppress Debug on credential types (`jalwa-ai/daimon.rs`)
- [ ] Parallelize fingerprint computation with rayon + add fingerprint cache (`jalwa-ai/fingerprint.rs`)

### Medium
- [ ] Use RwLock or atomics for decode status instead of Mutex in audio hot path (`decode_thread.rs:288`)
- [ ] Bounded mpsc channels with backpressure for engine commands (`jalwa-playback/lib.rs`)
- [ ] Read playback state from status mutex, not stale local field (`jalwa-playback/lib.rs:293`)
- [ ] Skip up to N bad frames on decode error instead of stopping track (`decode_thread.rs:226`)
- [ ] Attempt PipeWire reconnect on audio output failure (`decode_thread.rs:305`)
- [ ] Reset smooth_gain on seek when normalization is active (`decode_thread.rs:272`)
- [ ] Add max depth to scanner WalkDir to prevent symlink loops (`scanner.rs:37`)
- [ ] Return partial scan results with error count instead of silent skip (`scanner.rs:57`)
- [ ] Show engine errors in TUI status bar instead of `let _ =` (`jalwa-ui/tui.rs`)
- [ ] Show "library empty" help text in TUI (`jalwa-ui/widgets.rs`)
- [ ] Cap search query length at 256 chars in TUI (`jalwa-ui/tui.rs:397`)
- [ ] Viewport-aware list rendering for large libraries in TUI (`jalwa-ui/widgets.rs`)
- [ ] Avoid per-frame search results clone in GUI (`jalwa-gui/library.rs:67`)
- [ ] Add LRU eviction to `no_art` set in art cache (`jalwa-gui/art_cache.rs`)
- [ ] Send JSON-RPC parse error response for malformed MCP input (`mcp.rs:44`)
- [ ] Add operation context to DB error messages (`jalwa-core/db.rs`)

### Low
- [ ] Log warnings for unknown codec fallback values (`db.rs:499-537`)
- [ ] Deduplicate test fixtures across crates (use `test_fixtures` module)
- [ ] Bounds-check EQ band index with `.get()` instead of `.min(9)` (`jalwa-gui`, `jalwa-ui`)
- [ ] Fix grid view arrow navigation to move by column (`jalwa-gui/library.rs:289`)
- [ ] Add keyboard shortcut help dialog in GUI
- [ ] Pre-allocate EQ bar strings instead of per-frame Vec allocation (`jalwa-ui/widgets.rs:291`)
- [ ] Guard `chunks_exact(4)` against non-aligned buffer length (`dsp.rs:337`)
- [ ] Send specific `PrepareNextFailed` event for gapless failure (`decode_thread.rs:192`)
- [ ] Use `recv_timeout` for paused decode thread safety (`decode_thread.rs:139`)

## Phase 6 — Video Playback (v2) — *planned, not started*
> **Prerequisites met**: tarang-video now has full decode/encode for AV1, H.264, VP8/VP9.
> Blocked on aethersafha Wayland surface integration.

- [ ] tarang-video decode integration (dav1d, openh264, libvpx)
- [ ] Wayland surface for video output (via aethersafha)
- [ ] Subtitle rendering (SRT, VTT, ASS)
- [ ] Audio/video sync

## Phase 7 — Desktop UI
- [x] egui desktop app (jalwa-gui with eframe/wgpu backend) *(2026-03-18)*
- [x] Album grid / list view with `LibraryViewMode` toggle *(2026-03-18)*
- [x] Now playing screen with album art *(2026-03-18)*
- [x] Keyboard shortcuts (MPRIS media keys, spacebar, arrows) *(2026-03-16)*
- [ ] Playlist editor
- [ ] System tray / notification integration

## Phase 8 — AI Features
- [x] Audio fingerprinting (local similarity via tarang-ai `compute_fingerprint`/`fingerprint_match`) *(2026-03-16)*
- [ ] Content-based recommendations via hoosh
- [ ] Transcription overlay for video/podcasts
- [ ] "Play something like this" via semantic search (fingerprint + daimon RAG)
- [ ] Mood-based playlists

## Phase 9 — AGNOS Integration
- [ ] Marketplace recipe
- [ ] MCP tools registered in daimon
- [ ] agnoshi intents ("play music", "next track", "search library")
- [ ] aethersafha media widget (mini player in compositor panel)
