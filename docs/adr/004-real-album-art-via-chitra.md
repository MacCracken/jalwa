# 0004 — Real album art: extract from tags, decode via chitra, blit (reverses 0002's art drop)

**Status**: Accepted
**Date**: 2026-07-12

## Context

[ADR 0002](002-audio-probe-via-shravan-no-album-art.md) dropped album art: the scanner
probes audio via **shravan**, which has no picture/`APIC`/`PICTURE` parser, and there was
no Cyrius image decoder to turn cover bytes into pixels. So `art_mime`/`art_data` stayed
`None` and the GUI drew a lettered placeholder box for every cover.

Two things changed:

1. **chitra** (sibling Cyrius pkg, v0.3.0, `dist/chitra.cyr`) is a pure-CPU image decoder —
   PNG (all depths + Adam7) and **baseline** JPEG → canonical RGBA8, via
   `chitra_image_decode(src, len, err_out)`. It is stdlib-stripped and binds to jalwa's
   existing bump allocator + `sankoch` (already a dep), so it composes with zero new
   stdlib work.
2. The tag-side gap is small and self-contained: the FLAC `METADATA_BLOCK_PICTURE`
   (type 6) and ID3v2 `APIC` frame are simple front-of-file structures we can hand-parse.

So the decoder is no longer the blocker, and real album art becomes implementable on the
CPU-framebuffer GUI.

## Decision

Add real album art as a **three-stage, load-on-demand pipeline**, and **keep art out of
the DB** (0002's DB decision stands):

- **Extract** (`src/core/scanner.cyr`) — jalwa-owned parsers (shravan has no picture API,
  and `lib/` is read-only): `jlw_flac_find_picture` + `jlw_flac_picture_data` walk to the
  type-6 PICTURE image bytes; `jlw_id3_find_apic` walks ID3v2 frames to the `APIC` image
  bytes. `jlw_extract_cover_art(path, out_len)` reads the file into a **single reusable
  4 MiB buffer** (covers live in front metadata; the shared 1 MiB scan buffer is too small
  and is mutated per file) and returns the encoded bytes. FLAC + MP3 only.
- **Decode** — `chitra_image_decode` on those bytes → RGBA8 (new `[deps.chitra]`).
- **Blit** — a new alloc-free `jlw_gui_fb_blit_rgba` (nearest-neighbor scale + integer
  alpha-over, clip-aware) renders the cover in `gui/raster.cyr`'s IMAGE command; the
  lettered placeholder remains the no-art / decode-fail fallback.
- **Cache, not persist** — `gui/art_cache.cyr` gains decoded-pixel storage (`tex_pix`/
  `tex_w`/`tex_h`, LRU-aligned with the existing keys) and a `jlw_gui_art_get(id, path)`
  that **re-extracts from the item's file path** on a miss, decodes once, and caches the
  RGBA keyed by media-item Uuid (a lazily-created process-global cache). This mirrors
  rust-old's `ArtCache::load_art` and means art survives a restart (re-derived from the
  path) with **no DB schema change**. The IMAGE draw-command carries the item pointer in
  its `aux` slot so the raster can resolve id + path.

## Consequences

- **Positive** — real covers render in Now-Playing, the Library grid, and the Mini-player,
  in all three themes (they blit over the active palette). No DB bloat, no `library.db`
  migration, art survives restart. Decode happens **once per cover** (never per frame).
- **Negative** —
  - **Re-extraction re-reads the file** on a cache miss (bounded: 200-entry LRU, one 4 MiB
    reusable read buffer). The first paint of a large grid does N bounded reads.
  - **Format limits**: chitra decodes baseline JPEG + 8-bit PNG only. Progressive/CMYK
    JPEG and interlaced/16-bit PNG covers fail cleanly → placeholder (recorded no-art).
  - **LRU caps references, not RSS**: the bump allocator never frees and
    `chitra_image_free` is a no-op, so a long session that touches many covers retains
    their RGBA. `MAX_TEXTURES = 200` bounds the *referenced* working set; acceptable for a
    desktop session (same intent as the Rust cap).
- **Neutral** — WAV/OGG/MP4 cover extraction is deferred (FLAC + MP3 only). The decoded
  RGBA8 is also exactly what a future **P7** adaptive-album-palette (k-means dominant) would
  consume.

## Alternatives considered

- **Persist art as a DB BLOB** — rejected: `jlw_db_load_library` would pull every track's
  100 KB–1 MB cover into a never-freed bump arena at startup (hundreds of MB for art most
  tracks never show); patra prepared-statement binds can't carry a BLOB (only INT/STR), so
  the insert would have to be rewritten onto `patra_insert_row`; and adding columns breaks
  every existing `library.db` (CREATE is a no-op on an existing table → `PATRA_ERR_COLCOUNT`).
  Load-on-demand keeps only visible art resident and needs no migration.
- **Add a picture parser to shravan** — rejected: `lib/` is vendored and read-only. The
  parse is small; hand-rolling it in `scanner.cyr` (borrowing only framing math) is clean.
- **Store art bytes on the in-memory `JlwMediaItem` at scan time** — rejected as the
  primary path: DB round-trip erases it, so GUI items always have `art_data = None`
  anyway. Re-extracting from `path` is the only route that works after a restart; a
  scan-time fast-path would only help freshly-scanned in-session items and adds an
  `art_len` field the struct lacks.
