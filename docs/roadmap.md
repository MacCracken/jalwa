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

## Phase 10 — Streaming Service Integrations

Connect jalwa to external streaming services as playback sources. Each adapter implements a common trait for auth, search, playback, and library sync.

### Adapters

| Service | Protocol | Priority | Notes |
|---------|----------|----------|-------|
| **Apple Music** | MusicKit / Apple Music API | High | Requires Apple Developer token. Streaming via HLS. Library sync for playlists, favorites, recently played. |
| **Spotify** | Spotify Web API + Connect | High | OAuth 2.0 PKCE. Streaming via Spotify Connect (jalwa as controller). Library sync. |
| **Tidal** | Tidal API | Medium | Hi-res audio (MQA/FLAC). OAuth 2.0. |
| **YouTube Music** | YouTube Data API v3 | Medium | OAuth 2.0. Video-backed audio. |
| **SoundCloud** | SoundCloud API | Low | OAuth 2.0. Indie/DJ content. |
| **Bandcamp** | Web scraping (no official API) | Low | Purchase-based. DRM-free downloads. |
| **Local / NAS** | SMB, NFS, DLNA/UPnP | Medium | Network-attached media libraries. No auth needed. |
| **Podcast feeds** | RSS/Atom + enclosures | Medium | Standard podcast protocol. No auth. |

### Adapter Architecture

```
jalwa-core
  └── StreamingAdapter trait
        ├── authenticate() → Token
        ├── search(query) → Vec<Track>
        ├── play(track_id) → AudioStream
        ├── pause() / resume() / seek()
        ├── library() → Playlists, Favorites, History
        └── sync() → merge remote ↔ local state
```

Each adapter is feature-gated in `jalwa-playback`:
```toml
[features]
apple-music = []
spotify = []
tidal = []
youtube-music = []
soundcloud = []
local-network = []
podcasts = []
streaming-full = ["apple-music", "spotify", "tidal", "youtube-music"]
```

### Requirements

- All auth via OAuth 2.0 / PKCE where possible — no storing passwords
- All network requests via reqwest with rustls — no openssl dependency
- Offline queue: mark tracks for offline, download when connected
- Unified search: one search bar queries all enabled adapters simultaneously
- Library merge: remote playlists appear alongside local playlists seamlessly
- Playback routing: stream audio through dhvani regardless of source

---

## Phase 11 — AGNOS Integration
- [ ] Marketplace recipe (in zugot)
- [ ] MCP tools registered in daimon
- [ ] agnoshi intents ("play music", "next track", "search library", "play my Apple Music playlist")
- [ ] aethersafta media widget (mini player in compositor panel)
- [ ] vinimaya integration for premium content purchases
