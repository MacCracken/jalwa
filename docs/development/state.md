# jalwa — Current State

> Refreshed every wave. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Plan of record: [`port-audit.md`](port-audit.md).

## Version

**0.1.0** — scaffolded from Rust (2026-07-09) via `cyrius port`.

## Toolchain

- **Cyrius pin**: `6.4.29` (in `cyrius.cyml [package].cyrius`)

## Source / oracle

- **Rust oracle**: 15,355 LOC (40 `.rs` files, full 5-crate workspace) frozen at `rust-old/`. Do not edit.
- **Cyrius port**: `src/error.cyr` + `src/core/{types,playlist_io,db,scanner,watcher,hardware}.cyr` — **all of jalwa-core**, green. `src/main.cyr` still the smoke stub.

## Port progress

| Wave | Status |
|---|---|
| 0 — scaffold + oracle freeze + `error.cyr` | ✅ done |
| A — core types | ✅ done — all 7 types (6 enums, PlaybackStatus, Uuid, Playlist, PlayQueue, MediaItem, Library); 72 tests. ADR 0001 (linear-scan indexes) |
| B — core services | ✅ done — `playlist_io`, `db`, `scanner`, `watcher`, `hardware` (yukti). **jalwa-core fully ported.** |
| C — playback + DSP | ⏳ next — dsp→dhvani (clean win), decode_thread (tarang stub), engine facade; mpris shape-only |
| D — AI | ☐ |
| E — terminal UI | ☐ |
| F — desktop GUI | ☐ |
| G — binary + MCP | ☐ |

**Modules: `error.cyr` + `core/{types,playlist_io,db,scanner,watcher,hardware}.cyr` ✅ = all 6 jalwa-core rust modules. 6 / 33 rust modules fully ported.**
Backlogged (blocked): `video_decode_thread`, `view_video` (tarang+aethersafta); real playback (tarang stub);
MPRIS export (samvada client-only).

## Tests

- `error` 19 · `core_types` 72 · `playlist_io` 17 · `db_helpers` 34 · `db` 27 · `scanner` 31 · `watcher` 20 · `hardware` 55 — all green (275). Bare `cyrius test` all green.

> Toolchain drift: `cyrius.cyml` pins 6.4.29; cycc is now 6.4.32. Builds pass against the 6.4.29-vendored `lib/`; benign pin-drift warning. Bump the pin + `cyrius lib sync` when convenient.
- Per-module `.tcyr` suites land with each ported module, cross-checked against `rust-old/` `#[test]` blocks.

## Dependencies

- **stdlib** (via `cyrius lib sync`): string, fmt, alloc, vec, str, syscalls, io, args, assert, **math**
  (to grow: fs, thread, net, http, chrono, random, hashmap, tagged, result, fnptr, bayan, patra, vani, yukti, sakshi, mabda, simd)
- **dist repos** (to wire in Wave 0): dhvani, ai-hwaccel, bote, shravan, chitra, dhancha, darshana, abaco
- **Blocked (still Rust)**: tarang, aethersafta

## Next

Wave 0: wire `cyrius.cyml` deps + write `src/error.cyr`. Then Wave A: `jalwa-core/lib.rs` →
`src/core/types.cyr`. See [`roadmap.md`](roadmap.md).
