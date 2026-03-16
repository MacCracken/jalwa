# Jalwa Roadmap

## Phase 1 — Foundation (Complete)
- [x] Core types: library, playlists, queue, playback state
- [x] Playback engine: tarang-based open/probe, state management, seek, volume
- [x] UI: TUI status bar, progress bar, queue summary, library rendering
- [x] AI: recommendations, smart playlists, library insights
- [x] CLI + MCP server (5 tools)
- [x] CI/CD pipelines

## Phase 2 — Audio Playback
- [ ] Full tarang-audio decode pipeline (not just probe)
- [ ] PipeWire audio output (cpal or direct PipeWire client)
- [ ] Gapless playback (pre-buffer next track)
- [ ] ReplayGain / volume normalization
- [ ] Equalizer (10-band)

## Phase 3 — Library Management
- [ ] Directory scanner (recursive, watch for changes)
- [ ] SQLite persistence (library, playlists, play counts, ratings)
- [ ] Metadata extraction (ID3, Vorbis comments, MP4 atoms)
- [ ] Album art extraction and display
- [ ] Import/export playlists (M3U, PLS)

## Phase 4 — Video Playback
- [ ] tarang-video decode integration (dav1d, openh264, libvpx)
- [ ] Wayland surface for video output (via aethersafha)
- [ ] Subtitle rendering (SRT, VTT, ASS)
- [ ] Audio/video sync

## Phase 5 — Desktop UI
- [ ] egui or Tauri v2 desktop app
- [ ] Album grid / list view
- [ ] Now playing screen with album art
- [ ] Playlist editor
- [ ] Keyboard shortcuts (media keys, spacebar, arrows)
- [ ] System tray / notification integration

## Phase 6 — AI Features
- [ ] Content-based recommendations via hoosh
- [ ] Audio fingerprinting (identify unknown tracks)
- [ ] Transcription overlay for video/podcasts
- [ ] "Play something like this" via semantic search
- [ ] Mood-based playlists

## Phase 7 — AGNOS Integration
- [ ] Marketplace recipe
- [ ] MCP tools registered in daimon
- [ ] agnoshi intents ("play music", "next track", "search library")
- [ ] aethersafha media widget (mini player in compositor panel)
