# jalwa

**AI-native media player for AGNOS** — written in [Cyrius](https://github.com/MacCracken/cyrius).

jalwa (Persian: *manifestation / display*) is a media library + player: index, search,
smart playlists, real audio playback with a DSP chain (EQ + loudness normalization), a
terminal UI, a Wayland desktop GUI, AI recommendations, and an MCP server. It sits on the
**tarang** media framework and drives **dhvani** for audio DSP.

> **Status: v1.0.0.** Every non-blocked subsystem is ported to Cyrius and green against
> the Rust oracle. The one remaining gap is **video**, hard-blocked on still-Rust
> dependencies (see [the roadmap](docs/development/roadmap.md)).

## The port

jalwa is a **Rust → Cyrius port**. The original 5-crate Rust workspace is frozen at
[`rust-old/`](rust-old/) as a read-only **parity oracle** — the port's bar is "matches
what Rust did". See [`docs/development/port-audit.md`](docs/development/port-audit.md) for
the plan and the per-module ledger.

## Architecture

Pure-Cyrius modules under `src/` (no crates; the entry file / `cyrius.cyml` imposes order):

```
src/core       — library, playlists, play queue, playback state, search;
                 scanner (real audio probe + tags via shravan), DB (patra),
                 watcher (inotify), hardware (yukti)
src/playback   — real decode → output pipeline (shravan decode → dhvani EQ/normalize
                 → vani ALSA/agnos out), threaded engine, transport (play/pause/seek/vol)
src/ai         — recommendations, daimon, fingerprint scaffold
src/ui         — terminal UI (interactive run loop over darshana)
src/gui        — desktop GUI: draw-command IR → CPU rasterizer (kashi VGA 8x16 font,
                 XRGB8888 wl_shm buffer) → control layer → Wayland present shell (puka-forked)
src/mcp*       — MCP stdio JSON-RPC 2.0 server (8 tools)
```

Key deps (resolved via `cyrius.cyml`): **dhvani** (audio DSP), **shravan** (codecs),
**vani** (audio out), **patra** (DB), **darshana** (TUI), **kashi** (font), plus **yukti**,
**abaco**, **sankoch**.

## Build

```sh
cyrius lib sync                          # vendor stdlib modules from the pin
cyrius deps                              # resolve repo deps (dhvani/shravan/…) into lib/
cyrius build src/main.cyr build/jalwa    # compile the binary
cyrius test                              # run all tests/*.tcyr (what CI runs)
```

## Usage

```sh
jalwa scan ~/Music              # index a directory (real probe + tags)
jalwa play ~/Music/song.flac    # decode → DSP → audio out (WAV/FLAC/MP3)
jalwa info song.flac            # probe: format, duration, codec, tags
jalwa search "coltrane blue"    # search the library
jalwa stats                     # library statistics
jalwa library                   # list the library
jalwa export <playlist> out.m3u # export a playlist
jalwa import in.m3u             # import a playlist
jalwa devices                   # list USB / optical media devices
jalwa tui                       # interactive terminal UI
jalwa gui                       # Wayland desktop GUI (needs a compositor)
jalwa mcp                       # run as an MCP server (stdio JSON-RPC)
```

## MCP server

`jalwa mcp` is a working stdio JSON-RPC 2.0 server (protocol `2024-11-05`), hand-rolled in
pure Cyrius. Tools: `jalwa_play`, `jalwa_pause`, `jalwa_status`, `jalwa_search`, `jalwa_library`,
`jalwa_queue`, `jalwa_playlist`, `jalwa_recommend`. See [`docs/mcp-tools.md`](docs/mcp-tools.md).

## Documentation

- [`docs/development/state.md`](docs/development/state.md) — live status
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — wave sequencing + backlog (video = P1)
- [`docs/development/port-audit.md`](docs/development/port-audit.md) — the port plan + module ledger
- [`docs/adr/`](docs/adr/) — architecture decision records

## License

GPL-3.0-only.
