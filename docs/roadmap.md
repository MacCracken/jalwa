# Jalwa Roadmap

## Phase 6 — Video Playback (v2) — *in progress*
> **Prerequisites met**: tarang-video now has full decode/encode for AV1, H.264, VP8/VP9.
> Uses **aethersafta** for compositing, scene graph, and hardware-accelerated rendering.

- [ ] aethersafta integration: scene graph for video surface within egui window
- [ ] tarang-video decode integration (dav1d, openh264, libvpx)
- [ ] Hardware-accelerated encode/decode via aethersafta `vaapi` / `hwaccel` features
- [ ] Wayland video surface output (via aethersafta PipeWire + compositing pipeline)
- [ ] Subtitle rendering (SRT, VTT, ASS) composited as scene graph overlay
- [ ] Audio/video sync (aethersafta capture timing + dhvani PipeWire output)
- [ ] Audio visualizer overlay via aethersafta scene graph

## Phase 8 — Desktop UI
- [ ] Playlist editor
- [ ] System tray / notification integration
- [ ] Keyboard shortcut help dialog

## Phase 9 — AI Features
- [ ] Content-based recommendations via hoosh
- [ ] Transcription overlay for video/podcasts
- [ ] "Play something like this" via semantic search (fingerprint + daimon RAG)
- [ ] Mood-based playlists

## Phase 10 — AGNOS Integration
- [ ] Marketplace recipe
- [ ] MCP tools registered in daimon
- [ ] agnoshi intents ("play music", "next track", "search library")
- [ ] aethersafta media widget (mini player in compositor panel)
