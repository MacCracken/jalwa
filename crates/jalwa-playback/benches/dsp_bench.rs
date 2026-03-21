//! Benchmarks for jalwa-playback DSP operations.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use jalwa_playback::dsp::{EqSettings, Equalizer};

#[cfg(feature = "tarang")]
mod tarang_benches {
    use super::*;
    use bytes::Bytes;
    use std::time::Duration;

    fn make_buf(num_samples: usize, channels: u16, sample_rate: u32) -> tarang::core::AudioBuffer {
        let samples: Vec<f32> = (0..num_samples * channels as usize)
            .map(|i| {
                let t = i as f64 / (sample_rate as f64 * channels as f64);
                (t * 440.0 * 2.0 * std::f64::consts::PI).sin() as f32 * 0.5
            })
            .collect();
        let bytes: &[u8] = bytemuck::cast_slice(&samples);
        tarang::core::AudioBuffer {
            data: Bytes::copy_from_slice(bytes),
            sample_format: tarang::core::SampleFormat::F32,
            channels,
            sample_rate,
            num_frames: num_samples,
            timestamp: Duration::ZERO,
        }
    }

    pub fn bench_eq_process(c: &mut Criterion) {
        let mut group = c.benchmark_group("eq_process");
        for &(samples, channels) in &[(1024, 2u16), (4096, 2), (4096, 1)] {
            let label = format!("{samples}s_{channels}ch");
            let buf = make_buf(samples, channels, 48000);

            group.bench_with_input(BenchmarkId::new("flat", &label), &buf, |b, buf| {
                let mut eq = Equalizer::new(48000);
                eq.settings.enabled = true;
                // Flat EQ — should short-circuit
                b.iter(|| eq.process(black_box(buf)));
            });

            group.bench_with_input(BenchmarkId::new("rock_preset", &label), &buf, |b, buf| {
                let mut eq = Equalizer::new(48000);
                eq.settings = EqSettings::preset("rock");
                eq.update_coefficients();
                b.iter(|| eq.process(black_box(buf)));
            });

            group.bench_with_input(BenchmarkId::new("all_bands", &label), &buf, |b, buf| {
                let mut eq = Equalizer::new(48000);
                eq.settings.enabled = true;
                for i in 0..10 {
                    eq.settings.set_band(i, 6.0);
                }
                eq.update_coefficients();
                b.iter(|| eq.process(black_box(buf)));
            });
        }
        group.finish();
    }

    pub fn bench_normalize(c: &mut Criterion) {
        let mut group = c.benchmark_group("normalize");
        for &samples in &[1024, 4096] {
            let buf = make_buf(samples, 2, 48000);
            group.bench_with_input(BenchmarkId::new("analyze", samples), &buf, |b, buf| {
                b.iter(|| jalwa_playback::dsp::analyze_loudness(black_box(buf)));
            });
            group.bench_with_input(BenchmarkId::new("apply", samples), &buf, |b, buf| {
                b.iter(|| jalwa_playback::dsp::normalize(black_box(buf), 0.8));
            });
        }
        group.finish();
    }

    pub fn bench_volume(c: &mut Criterion) {
        let buf = make_buf(4096, 2, 48000);
        c.bench_function("apply_volume_in_place_4096", |b| {
            b.iter_with_setup(
                || buf.clone(),
                |mut buf| {
                    jalwa_playback::decode_thread::apply_volume_in_place(black_box(&mut buf), 0.7);
                },
            );
        });
    }

    criterion_group!(
        tarang_benches_group,
        bench_eq_process,
        bench_normalize,
        bench_volume,
    );
}

// Non-tarang benchmarks for EqSettings (always available)
fn bench_eq_settings(c: &mut Criterion) {
    c.bench_function("eq_preset_load", |b| {
        b.iter(|| EqSettings::preset(black_box("rock")));
    });

    c.bench_function("eq_coefficients_update", |b| {
        let mut eq = Equalizer::new(48000);
        eq.settings = EqSettings::preset("rock");
        b.iter(|| {
            eq.update_coefficients();
        });
    });
}

#[cfg(feature = "tarang")]
criterion_group!(
    benches,
    bench_eq_settings,
    tarang_benches::bench_eq_process,
    tarang_benches::bench_normalize,
    tarang_benches::bench_volume,
);

#[cfg(not(feature = "tarang"))]
criterion_group!(benches, bench_eq_settings,);

criterion_main!(benches);
