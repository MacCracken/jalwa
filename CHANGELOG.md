# Changelog

## 2026.3.16

Initial scaffolding.

- Core types: media library, playlists, play queue, playback state, search
- Playback engine: tarang-based decode, open/play/pause/stop/seek/volume
- UI layer: TUI status bar, progress bar, queue summary, library browser
- AI features: recommendations (artist/album/tag/duration matching), smart playlists, library insights
- CLI: play, info, search, stats, mcp subcommands
- MCP server: 5 tools (jalwa_play, jalwa_pause, jalwa_status, jalwa_search, jalwa_recommend)
- CI/CD: GitHub Actions (check, test, clippy, fmt, multi-arch release)
