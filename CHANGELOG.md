# Changelog

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
