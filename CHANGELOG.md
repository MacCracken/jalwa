# Changelog

## 2026.3.16

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
- Flat bands skip processing for zero overhead when disabled
- `EqSettings` with enable/disable, per-band gain, band names

**Album Art Extraction**
- `MediaItem` now carries `art_mime` and `art_data` fields
- Scanner extracts embedded album art via lofty (prefers CoverFront, falls back to first picture)
- Supports JPEG, PNG, and other embedded formats from ID3v2, Vorbis Comment, MP4 atoms

**File Watcher (inotify)**
- `LibraryWatcher` monitors library directories for create/modify/remove events
- Cross-platform via `notify` crate (inotify on Linux, FSEvents on macOS)
- Filters to media file extensions only
- Non-blocking `poll()` and blocking `recv_timeout()` APIs

**Testing**
- 134 tests across workspace (51.2% line coverage, +5.6%)
- 18 DSP tests: loudness analysis, normalization, EQ boost/cut/passthrough/stereo/reset
- 7 scanner tests: extension filtering, empty/non-dir, art extraction, tag override
- 5 watcher tests: media filtering, event detection, empty paths

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
- Three views: Library, Now Playing, Queue (Tab to cycle)
- Library browser with live search filtering (/ to search, Esc to cancel)
- Playback controls: Space (play/pause), Left/Right (seek ±10s), +/- (volume), m (mute)
- Queue management: a (enqueue), d (remove), c (clear), n/p (next/prev), r (repeat), s (shuffle)
- 50ms tick rate for smooth progress bar updates

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
