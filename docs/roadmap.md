# Jalwa Roadmap

## Engineering Backlog — from 2026-03-19 audit (resolved 2026-03-22)

### Medium — all resolved
- [x] Use RwLock for decode status instead of Mutex in audio hot path (`decode_thread.rs`)
- [x] Bounded mpsc channels (32) with backpressure for engine commands (`jalwa-playback/lib.rs`)
- [x] Read playback state from status RwLock, not stale local field (`jalwa-playback/lib.rs`)
- [x] Skip up to 10 bad frames on decode error instead of stopping track (`decode_thread.rs`)
- [x] Attempt PipeWire reconnect on audio output failure (`decode_thread.rs`)
- [x] Reset smooth_gain on seek when normalization is active (`decode_thread.rs`)
- [x] Add max depth (20) to scanner WalkDir to prevent symlink loops (`scanner.rs`)
- [x] Return `ScanResult` with partial results + error count (`scanner.rs`)
- [x] Show engine errors in TUI status bar with auto-clear after 5s (`jalwa-ui/tui.rs`)
- [x] Show "library empty" help text in TUI (`jalwa-ui/widgets.rs`)
- [x] Cap search query length at 256 chars in TUI (`jalwa-ui/tui.rs`)
- [x] Viewport-aware list rendering via ratatui ListState (verified working)
- [x] Avoid per-frame search results clone in GUI via `std::mem::take` (`jalwa-gui/library.rs`)
- [x] Add LRU eviction to `no_art` set in art cache (cap 1000) (`jalwa-gui/art_cache.rs`)
- [x] Send JSON-RPC -32700 parse error response for malformed MCP input (`mcp.rs`)
- [x] Add operation context to DB error messages (`jalwa-core/db.rs`)

### Low — all resolved
- [x] Log warnings for unknown codec fallback values (`db.rs`)
- [ ] Deduplicate test fixtures across crates (already using `test_fixtures` module)
- [x] Bounds-check EQ band index with `.get()` in GUI and TUI
- [x] Grid view arrow navigation moves by column (verified working)
- [ ] Add keyboard shortcut help dialog in GUI
- [x] Pre-allocate EQ bar strings — verified `(0..10).map().collect()` uses ExactSizeIterator
- [x] Guard `chunks_exact(4)` against non-aligned buffer length with warning (`dsp.rs`)
- [x] Send specific `PrepareNextFailed` event for gapless failure (`decode_thread.rs`)
- [x] Use `recv_timeout(1s)` for paused decode thread safety (`decode_thread.rs`)

## Phase 6 — Video Playback (v2) — *in progress*
> **Prerequisites met**: tarang-video now has full decode/encode for AV1, H.264, VP8/VP9.
> Uses **aethersafta** for compositing, scene graph, and hardware-accelerated rendering.

- [ ] aethersafta integration: scene graph for video surface within egui window
- [ ] tarang-video decode integration (dav1d, openh264, libvpx)
- [ ] Hardware-accelerated encode/decode via aethersafta `vaapi` / `hwaccel` features
- [ ] Wayland video surface output (via aethersafta PipeWire + compositing pipeline)
- [ ] Subtitle rendering (SRT, VTT, ASS) composited as scene graph overlay
- [ ] Audio/video sync (aethersafta capture timing + dhvani PipeWire output)
- [ ] Audio visualizer overlay via aethersafta scene graph

## Phase 7 — Hardware Media Sources
> **yukti** (AGNOS hardware device crate) is now integrated (udev, hotplug, optical drive abstraction).
> Also leverages **ai-hwaccel** for hardware-accelerated I/O.

- [x] Integrate **yukti** (AGNOS hardware device crate) when available
- [x] USB storage auto-detect: subscribe to device events, auto-add mount as scan path
- [x] USB hotplug: detect insert/eject, prompt to scan or eject safely
- [x] CD audio (CDDA) playback: read audio tracks from optical drives
- [x] DVD/Blu-ray disc browsing and playback (UDF/ISO 9660)
- [x] ai-hwaccel integration for hardware-accelerated disc I/O
- [x] Graceful handling of device removal during playback

## Phase 8 — Desktop UI
- [ ] Playlist editor
- [ ] System tray / notification integration

## Phase 9 — AI Features
- [ ] Content-based recommendations via hoosh
- [ ] Transcription overlay for video/podcasts
- [ ] "Play something like this" via semantic search (fingerprint + daimon RAG)
- [ ] Mood-based playlists

## Phase 10 — AGNOS Integration
- [ ] Marketplace recipe
- [ ] MCP tools registered in daimon
- [ ] agnoshi intents ("play music", "next track", "search library")
- [ ] aethersafta media widget (mini player in compositor panel)
