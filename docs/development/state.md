# jalwa — Current State

> Refreshed every wave. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Plan of record: [`port-audit.md`](port-audit.md).

## Version

**0.1.0** — scaffolded from Rust (2026-07-09) via `cyrius port`.

## Toolchain

- **Cyrius pin**: `6.4.29` (in `cyrius.cyml [package].cyrius`)

## Source / oracle

- **Rust oracle**: 15,355 LOC (40 `.rs` files, full 5-crate workspace) frozen at `rust-old/`. Do not edit.
- **Cyrius port**: `src/error.cyr` + `src/core/types.cyr` (full `jalwa-core/lib.rs` type surface) — green. `src/main.cyr` still the smoke stub.

## Port progress

| Wave | Status |
|---|---|
| 0 — scaffold + oracle freeze + `error.cyr` | ✅ done |
| A — core types | ✅ done — all 7 types (6 enums, PlaybackStatus, Uuid, Playlist, PlayQueue, MediaItem, Library); 72 tests. ADR 0001 (linear-scan indexes) |
| B — core services (db/playlist_io/scanner/watcher/hardware) | ⏳ next |
| C — playback + DSP | ☐ |
| D — AI | ☐ |
| E — terminal UI | ☐ |
| F — desktop GUI | ☐ |
| G — binary + MCP | ☐ |

**Modules: `error.cyr` ✅, `core/types.cyr` ✅ (= `jalwa-core/lib.rs`). 1 / 33 rust modules fully ported.**
Backlogged (blocked): `video_decode_thread`, `view_video` (tarang+aethersafta); real playback (tarang stub);
MPRIS export (samvada client-only).

## Tests

- `tests/error.tcyr` — 19/19 green. `tests/core_types.tcyr` — 72/72 green. `tests/jalwa.tcyr` — scaffold smoke. Bare `cyrius test` all green.
- Per-module `.tcyr` suites land with each ported module, cross-checked against `rust-old/` `#[test]` blocks.

## Dependencies

- **stdlib** (via `cyrius lib sync`): string, fmt, alloc, vec, str, syscalls, io, args, assert, **math**
  (to grow: fs, thread, net, http, chrono, random, hashmap, tagged, result, fnptr, bayan, patra, vani, yukti, sakshi, mabda, simd)
- **dist repos** (to wire in Wave 0): dhvani, ai-hwaccel, bote, shravan, chitra, dhancha, darshana, abaco
- **Blocked (still Rust)**: tarang, aethersafta

## Next

Wave 0: wire `cyrius.cyml` deps + write `src/error.cyr`. Then Wave A: `jalwa-core/lib.rs` →
`src/core/types.cyr`. See [`roadmap.md`](roadmap.md).
