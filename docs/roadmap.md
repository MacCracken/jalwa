# Jalwa Roadmap

## Phase 1 — Foundation (Complete)
- [x] Core types: library, playlists, queue, playback state
- [x] Playback engine: tarang-based open/probe, state management, seek, volume
- [x] UI: TUI status bar, progress bar, queue summary, library rendering
- [x] AI: recommendations, smart playlists, library insights
- [x] CLI + MCP server (5 tools)
- [x] CI/CD pipelines

## Phase 2 — Audio Playback (Complete)
- [x] Full tarang-audio decode pipeline (decode thread with FileDecoder)
- [x] PipeWire audio output (dedicated thread, ring buffer architecture)
- [x] Gapless playback (PrepareNext command, NearEnd event, seamless decoder swap)
- [ ] ReplayGain / volume normalization
- [ ] Equalizer (10-band)

## Phase 3 — Library Management (Complete)
- [x] Directory scanner (recursive via walkdir, extension filtering)
- [x] SQLite persistence (library, playlists, play counts, ratings, scan paths)
- [x] Metadata extraction (lofty: ID3v2, Vorbis comments, MP4 atoms)
- [x] Import/export playlists (M3U)
- [ ] Album art extraction and display
- [ ] File watcher (inotify for auto-rescan)

## Phase 3.5 — Interactive TUI (Complete)
- [x] ratatui + crossterm terminal UI
- [x] Library browser with search
- [x] Now Playing view
- [x] Queue view with add/remove/reorder/clear
- [x] Keybindings: play/pause, seek, volume, mute, next/prev, repeat, shuffle
- [x] Default command (`jalwa` launches TUI)

## Phase 4 — Test Coverage to 80%+

Current: 45.6% (638/1398 lines). Target: 80% (1118 lines, need 480 more).

### Tier 1 — Pure rendering (est. +150 lines)
- [ ] `widgets.rs`: ratatui TestBackend tests for all views (Library, NowPlaying, Queue)
  - Construct `App` with temp DB, render to TestBackend, assert buffer content
  - Test all PlaybackState icons, progress bar, volume display, keybind bar
  - Test search mode header, queue position indicators, repeat/shuffle badges
  - ~12 tests covering status area, library view (empty/populated/search), now playing, queue view

### Tier 2 — TUI input handlers (est. +70 lines)
- [ ] `tui.rs`: unit test `handle_normal_input` and `handle_search_input`
  - These are private fns — add `#[cfg(test)] mod tests` inside the module
  - Test each keybinding: q/quit, Space/toggle, Tab/cycle, +/-/volume, m/mute, r/repeat, s/shuffle
  - Test search: Esc/cancel, Char/append, Backspace/pop, Enter/confirm
  - Test nav: Up/Down selection, Enter play from library/queue, a/enqueue, d/dequeue, c/clear
  - ~17 tests, each creates an App and asserts state changes

### Tier 3 — Scanner (est. +35 lines)
- [ ] `scanner.rs`: add tests with temp directories
  - `scan_directory_not_a_dir`: error case
  - `scan_directory_empty`: empty result
  - `scan_directory_skips_non_media`: temp dir with `.txt` files
  - `scanned_to_media_item_overrides`: lofty tags override probe title
  - `scanned_to_media_item_no_tags`: probe title preserved
  - For probe-dependent tests: create a minimal valid WAV (44-byte header + samples)

### Tier 4 — Decode thread + playback engine (est. +55 lines)
- [ ] `decode_thread.rs`: test pure functions (`apply_volume`)
  - `apply_volume_unity`, `apply_volume_half`, `apply_volume_muted`
  - `create_output` returns correct type based on feature
  - `decode_loop` integration test with NullOutput + minimal WAV: verify TrackFinished event
- [ ] `playback/lib.rs`: additional state machine tests
  - Open a temp WAV file, test seek/seek_relative with real duration
  - Test toggle from Paused state

### Tier 5 — CLI commands (est. +70 lines)
- [ ] `main.rs`: extract `cmd_*` functions to be testable, or add integration tests
  - `cmd_info` with a valid temp WAV
  - `cmd_search` / `cmd_stats` / `cmd_library` with temp DB
  - `cmd_scan` with temp directory containing a WAV
  - `cmd_export` / `cmd_import` roundtrip

### Projected coverage after all tiers: ~86%

## Phase 5 — Video Playback
- [ ] tarang-video decode integration (dav1d, openh264, libvpx)
- [ ] Wayland surface for video output (via aethersafha)
- [ ] Subtitle rendering (SRT, VTT, ASS)
- [ ] Audio/video sync

## Phase 6 — Desktop UI
- [ ] egui or Tauri v2 desktop app
- [ ] Album grid / list view
- [ ] Now playing screen with album art
- [ ] Playlist editor
- [ ] Keyboard shortcuts (media keys, spacebar, arrows)
- [ ] System tray / notification integration

## Phase 7 — AI Features
- [ ] Content-based recommendations via hoosh
- [ ] Audio fingerprinting (identify unknown tracks)
- [ ] Transcription overlay for video/podcasts
- [ ] "Play something like this" via semantic search
- [ ] Mood-based playlists

## Phase 8 — AGNOS Integration
- [ ] Marketplace recipe
- [ ] MCP tools registered in daimon
- [ ] agnoshi intents ("play music", "next track", "search library")
- [ ] aethersafha media widget (mini player in compositor panel)
