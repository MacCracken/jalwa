# Jalwa Roadmap

## Test Coverage Backlog (ongoing)

Current: 51.2% (800/1563 lines). Target: 80%.

### Tier 1 — Pure rendering (est. +150 lines)
- [ ] `widgets.rs`: ratatui TestBackend tests for all views
- [ ] Test all PlaybackState icons, progress bar, volume display, keybind bar

### Tier 2 — TUI input handlers (est. +70 lines)
- [ ] `tui.rs`: unit test `handle_normal_input` and `handle_search_input`

### Tier 3 — Decode thread + playback engine (est. +55 lines)
- [ ] `decode_thread.rs`: `decode_loop` integration test with NullOutput + minimal WAV
- [ ] `playback/lib.rs`: open temp WAV, test seek with real duration

### Tier 4 — CLI commands (est. +70 lines)
- [ ] `main.rs`: extract `cmd_*` into testable module or integration tests

## Phase 6 — Video Playback
- [ ] tarang-video decode integration (dav1d, openh264, libvpx)
- [ ] Wayland surface for video output (via aethersafha)
- [ ] Subtitle rendering (SRT, VTT, ASS)
- [ ] Audio/video sync

## Phase 7 — Desktop UI
- [ ] egui or Tauri v2 desktop app
- [ ] Album grid / list view
- [ ] Now playing screen with album art
- [ ] Playlist editor
- [ ] Keyboard shortcuts (media keys, spacebar, arrows)
- [ ] System tray / notification integration

## Phase 8 — AI Features
- [ ] Content-based recommendations via hoosh
- [ ] Audio fingerprinting (identify unknown tracks)
- [ ] Transcription overlay for video/podcasts
- [ ] "Play something like this" via semantic search
- [ ] Mood-based playlists

## Phase 9 — AGNOS Integration
- [ ] Marketplace recipe
- [ ] MCP tools registered in daimon
- [ ] agnoshi intents ("play music", "next track", "search library")
- [ ] aethersafha media widget (mini player in compositor panel)
