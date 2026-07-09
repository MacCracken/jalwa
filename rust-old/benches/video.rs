//! Video pipeline benchmarks — frame conversion and display frame creation.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jalwa_playback::DisplayFrame;
use std::time::Duration;

fn bench_display_frame_creation(c: &mut Criterion) {
    // Simulate creating a 1080p RGB24 display frame
    let rgb_data = vec![128u8; 1920 * 1080 * 3];

    c.bench_function("display_frame_1080p_create", |b| {
        b.iter(|| DisplayFrame {
            data: black_box(&rgb_data).clone(),
            width: 1920,
            height: 1080,
            pts: Duration::from_millis(33),
        });
    });

    // 720p
    let rgb_720 = vec![128u8; 1280 * 720 * 3];
    c.bench_function("display_frame_720p_create", |b| {
        b.iter(|| DisplayFrame {
            data: black_box(&rgb_720).clone(),
            width: 1280,
            height: 720,
            pts: Duration::from_millis(33),
        });
    });
}

fn bench_display_frame_clone(c: &mut Criterion) {
    let frame = DisplayFrame {
        data: vec![128u8; 1920 * 1080 * 3],
        width: 1920,
        height: 1080,
        pts: Duration::from_millis(33),
    };

    c.bench_function("display_frame_1080p_clone", |b| {
        b.iter(|| black_box(&frame).clone());
    });
}

fn bench_rgb_image_construction(c: &mut Criterion) {
    // Benchmark constructing an egui ColorImage from RGB24 data
    // This is what happens every frame in the video view
    let rgb_data = vec![128u8; 1920 * 1080 * 3];

    c.bench_function("color_image_from_rgb_1080p", |b| {
        b.iter(|| egui::ColorImage::from_rgb([1920, 1080], black_box(&rgb_data)));
    });

    let rgb_720 = vec![128u8; 1280 * 720 * 3];
    c.bench_function("color_image_from_rgb_720p", |b| {
        b.iter(|| egui::ColorImage::from_rgb([1280, 720], black_box(&rgb_720)));
    });
}

criterion_group!(
    benches,
    bench_display_frame_creation,
    bench_display_frame_clone,
    bench_rgb_image_construction,
);
criterion_main!(benches);
