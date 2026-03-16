# Jalwa Roadmap

## Pending: Tarang Release Migration
- [ ] Update Cargo.toml path deps (`../tarang`) to crates.io/git registry once tarang is released
- [ ] Remove CI workaround (dual checkout of tarang as sibling repo)
- [ ] Pin tarang version in workspace dependencies

## Test Coverage Backlog (ongoing)

Current: 51% (907/1778 lines). Target: 80%.

### Tier 1 — Pure rendering (est. +198 lines)
- [ ] `widgets.rs`: ratatui TestBackend tests for all views (Library, NowPlaying, Queue, Equalizer)

### Tier 2 — TUI input handlers (est. +100 lines)
- [ ] `tui.rs`: unit test `handle_normal_input`, `handle_search_input`, `handle_mpris_command`

### Tier 3 — Decode thread + MPRIS (est. +80 lines)
- [ ] `decode_thread.rs`: integration test with NullOutput + minimal WAV
- [ ] `mpris.rs`: test MPRIS command dispatch

### Tier 4 — CLI commands (est. +70 lines)
- [ ] `main.rs`: extract `cmd_*` into testable module or integration tests

## Phase 6 — Video Playback (v2)
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
