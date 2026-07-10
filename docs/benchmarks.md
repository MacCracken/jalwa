# Jalwa Benchmarks

Hot-path microbenchmarks for the Cyrius port. **Status: deferred.** No jalwa
benchmark numbers are published yet — the v1.0.0 criterion "`.bcyr` hot-path
benchmarks" is intentionally unchecked (see [`roadmap.md`](development/roadmap.md)),
and the audio-DSP benches live in **dhvani** (the gold-standard audio-engine port),
not here.

Benches run via `cyrius bench` over `.bcyr` files, using the stdlib `bench` harness
(`bench_new` → `bench_batch_start` → tight loop → `bench_batch_stop` → `bench_report`).
There is no `cargo`/criterion in this project — the Rust workspace at `rust-old/` is a
frozen parity oracle, and its `benches/*.rs` are reference only.

---

## Current state

A single scaffold bench exists at **`tests/jalwa.bcyr`** — a `noop` baseline that
validates the harness and the clock-overhead/batching pattern. It carries no jalwa
workload yet.

```sh
cyrius bench tests/jalwa.bcyr    # run one bench file
cyrius bench                     # discover + run every benches/*.bcyr and tests/bcyr/*.bcyr
```

Sub-microsecond ops should batch (`batch_size >= 1000`) to amortize the
`clock_gettime` start/stop overhead — see the `bench.cyr` header for the
overhead-vs-batching guidance.

---

## Deferred surface (what to bench, when un-deferred)

Candidate hot paths, mapped to the ported Cyrius modules. Do **not** assume the
old Rust complexity notes — the Cyrius library uses **linear-scan indexes**
(ADR 0001, `src/core/types.cyr`), so `find_by_id`/`find_by_path` are O(n), not the
Rust HashMap O(1).

| Area | Module | Candidate benches |
|---|---|---|
| Library ops | `src/core/types.cyr` | `jlw_library_find_by_id`, `find_by_path`, `search`, add/remove item |
| DB round-trip | `src/core/db.cyr` | save/load N items (patra) |
| Scanner probe | `src/core/scanner.cyr` | header-only WAV/FLAC/MP3 duration + tag parse |
| AI | `src/ai/reco.cyr` | recommend top-N, smart-playlist eval |
| DSP | **dhvani** (`jlw_dsp`) | EQ process, loudness-normalize — **owned upstream by dhvani's benches** |

---

## Notes

- **DSP benches belong to dhvani.** jalwa consumes dhvani's `jlw_dsp` EQ +
  normalize; the per-block DSP microbenchmarks are maintained in the dhvani repo.
- **Alloc-free hot paths.** The bump allocator never frees — a per-sample/per-block
  `alloc()` leaks across a render. Any bench of the audio path must reuse
  struct-owned scratch and write to caller out-buffers (see CLAUDE.md).
- **Video is out of scope.** Video decode is P1-blocked on tarang + aethersafta
  (both still Rust); no video benches until that unblocks.
