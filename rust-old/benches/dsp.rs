//! Top-level DSP benchmarks (delegates to playback crate).
//! This allows running `cargo bench --bench dsp` from the workspace root.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jalwa_playback::dsp::EqSettings;

fn bench_eq_presets(c: &mut Criterion) {
    let names = EqSettings::preset_names();
    c.bench_function("load_all_presets", |b| {
        b.iter(|| {
            for name in names {
                let _ = EqSettings::preset(black_box(name));
            }
        });
    });
}

fn bench_eq_is_flat(c: &mut Criterion) {
    let flat = EqSettings::default();
    let rock = EqSettings::preset("rock");

    c.bench_function("is_flat_true", |b| {
        b.iter(|| black_box(&flat).is_flat());
    });
    c.bench_function("is_flat_false", |b| {
        b.iter(|| black_box(&rock).is_flat());
    });
}

criterion_group!(benches, bench_eq_presets, bench_eq_is_flat,);
criterion_main!(benches);
