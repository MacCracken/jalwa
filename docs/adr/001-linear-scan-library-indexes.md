# 0001 — Library lookups by linear scan, dropping the private HashMap indexes

**Status**: Accepted
**Date**: 2026-07-09

## Context

`rust-old/crates/jalwa-core/src/lib.rs` `struct Library` carried two private,
`#[serde(skip)]` fields — `id_index: HashMap<Uuid, usize>` and
`path_index: HashMap<PathBuf, usize>` — maintained by `add_item`/`remove`/
`rebuild_indexes` and consulted by `find_by_id`/`find_by_path`. They are a pure
O(1)-lookup performance optimisation: no public API exposes them, and every
observable result (`find_by_id`, `find_by_path`, `reindex`) is identical to a
linear scan over `items`. The Cyrius port has no generic `HashMap<K,V>` for
arbitrary key types (the stdlib `hashmap` keys on cstr/u64), and a UUID→index /
PathBuf→index map would need bespoke hashing and dual-structure maintenance on
every mutation — real complexity for the L0 gate, with no behavioral payoff.

## Decision

Drop `id_index`/`path_index` entirely. `jlw_library_find_by_id` and
`jlw_library_find_by_path` linear-scan the `items` vec (uuid `memeq` / path
`streq`). `jlw_library_reindex` becomes a no-op retained for API parity. Scope:
`Library` only; nothing else in the port assumes the indexes exist.

## Consequences

- **Positive** — far simpler, lower-risk port of the foundational type; no
  UUID-hashing or dual-map-maintenance code to get subtly wrong; behavior is
  provably identical to the oracle (all `library_*` `#[test]`s pass unchanged).
- **Negative** — lookups are O(n) instead of O(1). For a large library
  (10k+ items) hot lookups could matter; today `find_by_*` is not on a hot path.
- **Neutral** — if profiling later shows lookup cost, reintroduce an index
  (uuid→index via a u64-keyed `hashmap` on the uuid's first 8 bytes, path→index
  via a cstr-keyed map) behind the same `find_by_*` API and make `reindex` real.
  This ADR would then be superseded.

## Alternatives considered

- **Port the HashMaps faithfully** — rejected for now: needs a custom UUID hash +
  two maps kept in sync across `add_item`/`remove`/`rebuild`, i.e. meaningful
  complexity and bug surface on the L0 gate, for zero behavioral change.
- **u64-keyed hashmap keyed on uuid bytes** — viable but premature; deferred to
  the "if profiling shows cost" path above.
