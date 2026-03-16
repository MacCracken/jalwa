# Jalwa Architecture

## Crate Dependency Graph

```
tarang (external — media framework)
  ↑
jalwa-core (library, playlists, queue, state)
  ↑
  ├── jalwa-playback (tarang decode + PipeWire output)
  ├── jalwa-ui (TUI/GUI rendering)
  └── jalwa-ai (recommendations, smart playlists, insights)
        ↑
        └── main binary (CLI + MCP server)
```

## Design Principles

1. **Tarang-native** — All media decoding goes through tarang. No ffmpeg, no GStreamer.

2. **Library-first** — Media is indexed, searchable, and analyzed. Not just a file opener.

3. **AI-integrated** — Recommendations, smart playlists, content classification, and transcription are built in, not plugins.

4. **PipeWire audio** — Native PipeWire for audio output. No PulseAudio compat layer.

5. **Wayland video** — Video frames rendered directly in aethersafha compositor. No X11.
