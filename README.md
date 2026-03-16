# Jalwa

**AI-native media player for AGNOS**

Jalwa (Persian: manifestation/display) is an audio and video player built on the Tarang media framework. Library management, playlists, AI-powered recommendations, smart playlists, and transcription routing via hoosh.

## Architecture

```
jalwa-core      — media library, playlists, play queue, playback state, search
jalwa-playback  — tarang decode pipeline, PipeWire audio output, seek, volume
jalwa-ui        — TUI status bar, progress bar, queue summary, library browser
jalwa-ai        — recommendations, smart playlists, library insights
```

## Usage

```bash
# Play a media file
jalwa play ~/Music/song.flac

# Probe and analyze a file
jalwa info movie.mp4

# Search the library
jalwa search "queen bohemian"

# Library stats
jalwa stats

# Run as MCP server
jalwa mcp
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `jalwa_play` | Play a media file (audio or video) |
| `jalwa_pause` | Pause current playback |
| `jalwa_status` | Get playback state, position, volume |
| `jalwa_search` | Search media library by title, artist, album, tag |
| `jalwa_recommend` | AI-powered recommendations based on a seed item |

## Built on Tarang

All media decoding goes through Tarang — pure Rust audio via symphonia, no ffmpeg dependency. Video decoding via thin C FFI wrappers (dav1d for AV1, openh264 for H.264, libvpx for VP8/VP9).

## License

GPL-3.0
