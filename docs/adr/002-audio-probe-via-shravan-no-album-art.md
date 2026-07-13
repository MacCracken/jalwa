# 0002 — Audio probe/tags via shravan (tarang is video-only); album art dropped

**Status**: Accepted — *album-art drop superseded by [ADR 0004](004-real-album-art-via-chitra.md)*
**Date**: 2026-07-09

> **Update (2026-07-12):** the "no album art" consequence below is reversed by
> [ADR 0004](004-real-album-art-via-chitra.md) — jalwa now extracts embedded covers from
> tags and decodes them via `chitra`. The rest of this ADR (audio probe via shravan,
> art *not stored in the DB*) still holds.

## Context

`rust-old/crates/jalwa-core/src/scanner.rs` `scan_file` obtained a `MediaInfo`
(format, duration, sample rate, codecs) from `tarang::audio::probe_audio` and
read tags + album art from `lofty` (`read_from_path` → `primary_tag` → text
frames + `CoverFront` picture). The Cyrius port initially stubbed all of this as
"scanning requires the tarang feature", because tarang is still Rust.

That stub reflected a **scoping mistake**: it treated tarang as the gate for
*all* media. The correct scope (project decision, 2026-07-09) is that **tarang is
VIDEO decode/encode only**. Audio decode/probe/tags belong to the dedicated
audio stack — **shravan** (codecs), **dhvani** (DSP), **vani** (output) — none of
which are blocked. So routing the audio probe through shravan is not a divergence
to be deferred; it is how the port is *supposed* to work.

shravan (`dist/shravan.cyr`) exposes header-only probing and tag reading, but
has **no picture/album-art parser** anywhere (flac/ogg/tag modules carry zero
cover-art code), and no single "probe" entry point — the consumer drives the
per-format header parsers.

## Decision

Port `jlw_scanner_scan_file` to probe audio via shravan, header-only (no full
decode), reusing one bounded read buffer across a directory scan (O(1) memory):

- **format** — jalwa-owned magic-byte detector (`jlw_detect_audio_format`), NOT
  shravan's `detect_format` (which collides by name with sankoch's
  compression-format `detect_format`; "last definition wins" is too fragile).
- **WAV** — jalwa-owned RIFF `fmt`/`data` chunk parse → rate/channels/duration.
- **FLAC** — shravan `flac_parse_metadata` (STREAMINFO) → duration; jalwa-owned
  VORBIS_COMMENT (metadata `block_type == 4`) walker feeds shravan
  `tag_read_vorbis` for title/artist/album.
- **MP3** — shravan `mp3_id3v2_skip` + `mp3_parse_frame_header` (after a one-time
  `mp3_init()` — that function reads the bitrate tables without the lazy-init
  guard its siblings have) → rate + a CBR duration estimate from the true file
  size (`lseek`); shravan `tag_read_id3v2` for tags.
- **MP4/OGG/AAC/OPUS/AIFF** — format + codec detected; duration/tags deferred
  (shravan derives those via full decode; not worth the cost in a scan).

**Album art is dropped**: `art_mime`/`art_data` stay `None`. `cmd_scan` and the
MCP `jalwa_library` `scan` action are un-stubbed to call the real scanner.

## Consequences

- **Positive** — `jalwa scan` and the MCP scan tool actually work, tarang-free:
  real per-file duration (WAV/FLAC exact, MP3 CBR estimate) + real tags
  (FLAC Vorbis, MP3 ID3v2). Header-only + a reused buffer keep a large-library
  scan cheap and O(1) in memory (no per-file full-decode allocation).
- **Negative / divergences from rust-old (lofty/tarang)**:
  - **No album art** — permanent until a Cyrius picture parser exists. This is
    the load-bearing divergence this ADR records.
  - **MP3/AAC/OGG/MP4 duration** — MP3 is a CBR estimate (wrong for VBR without a
    Xing/Info header, which shravan does not parse); AAC/OGG/MP4 duration is
    deferred (`None`) rather than full-decoded.
  - Tags for OGG/MP4 are deferred (only FLAC/MP3 tags are read so far).
- **Neutral** — when a Cyrius tag/art library or a Xing-header parser lands, or
  when the deferred formats matter, extend `scan_file`; the shape (bounded read,
  per-format branch, `scanned_to_media_item` override) already accommodates it.

## Alternatives considered

- **Keep scanning stubbed until a Cyrius tarang** — rejected: tarang is video;
  audio scanning has no reason to wait, and shravan already provides the probe.
- **Full-decode for duration on every format** — rejected: shravan derives
  MP3/AAC/OGG/MP4 duration by fully decoding, which on a large scan is slow and
  leaks a decode buffer per file (bump allocator never frees). Header-only
  probing avoids the hazard; missing durations are honest `None`s.
- **Rely on shravan's `detect_format` winning the symbol clash** — rejected:
  depends on dep fold order (sankoch also defines `detect_format`); a jalwa-owned
  magic-byte detector is robust and self-documenting.
