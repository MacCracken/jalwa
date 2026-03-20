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
