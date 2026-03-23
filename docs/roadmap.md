# Jalwa Roadmap

## Engineering Backlog — from 2026-03-19 audit

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
