# Jalwa Benchmarks

Benchmark results tracked over time. Run with `cargo bench` (requires release profile).

**Environment:** Linux 6.12, Rust stable, AMD64. Results are median of 100 samples.

---

## 2026.3.19 — Baseline

### Library Operations (jalwa-core)

| Benchmark | 100 items | 1,000 items | 10,000 items |
|-----------|-----------|-------------|--------------|
| `find_by_id` | 22 ns | 22 ns | 22 ns |
| `find_by_path` | 41 ns | 41 ns | 43 ns |
| `search` | 5.9 µs | 62 µs | 660 µs |
| `remove_item` | 15 µs | 161 µs | — |

| Benchmark | Time |
|-----------|------|
| `add_item` | 329 ns |
| `db_save_load_100_items` | 219 µs |

**Notes:**
- `find_by_id` and `find_by_path` are O(1) via HashMap — constant time across library sizes
- `search` is O(n) with per-item `to_lowercase()` — linear scaling confirmed
- `remove_item` is O(n) due to `retain()` + index rebuild

### Integration (jalwa root)

| Benchmark | Time |
|-----------|------|
| `persistent_add_100_items` | 8.9 ms |
| `persistent_reopen_500_items` | 793 µs |
| `recommend_1000_items_top10` | 167 µs |
| `smart_playlist_1000_items` | 60 µs |

### DSP (jalwa-playback)

| Benchmark | Time |
|-----------|------|
| `eq_preset_load` (all 9) | 41 ns |
| `eq_is_flat` | 2 ns |

**Note:** EQ process, normalize, and volume benchmarks require `tarang` feature with audio buffers. Run with `cargo bench -p jalwa-playback` when tarang is available.

---

## How to Run

```bash
# All benchmarks
cargo bench

# Specific crate
cargo bench -p jalwa-core
cargo bench -p jalwa-playback

# Specific benchmark
cargo bench --bench library
cargo bench --bench dsp

# Compare against saved baseline
cargo bench -- --save-baseline baseline
cargo bench -- --baseline baseline
```

## Key Metrics to Watch

- **`find_by_id`**: Should stay O(1) / <50ns regardless of library size
- **`search`**: Linear in library size; watch for regressions >1ms at 10K items
- **`persistent_reopen`**: DB load time; should stay <2ms for 500 items
- **`recommend`**: Should stay <500µs for 1000 items
- **`add_item`**: Should stay <1µs (index insert overhead)
