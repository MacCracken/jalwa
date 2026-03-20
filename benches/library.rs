//! Integration benchmarks for library + DB operations.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jalwa_core::db::PersistentLibrary;
use jalwa_core::test_fixtures::make_media_item;
use uuid::Uuid;

fn bench_persistent_library_add(c: &mut Criterion) {
    c.bench_function("persistent_add_100_items", |b| {
        b.iter_with_setup(
            || {
                let path =
                    std::env::temp_dir().join(format!("jalwa_bench_{}.db", Uuid::new_v4()));
                let plib = PersistentLibrary::open(&path).unwrap();
                (plib, path)
            },
            |(mut plib, path)| {
                for i in 0..100 {
                    let item = make_media_item(
                        &format!("Song {i}"),
                        &format!("Artist {i}"),
                        200,
                    );
                    plib.add_item(item).unwrap();
                }
                let _ = std::fs::remove_file(&path);
            },
        );
    });
}

fn bench_persistent_library_reopen(c: &mut Criterion) {
    // Pre-populate a DB
    let path = std::env::temp_dir().join(format!("jalwa_bench_reopen_{}.db", Uuid::new_v4()));
    {
        let mut plib = PersistentLibrary::open(&path).unwrap();
        for i in 0..500 {
            let item = make_media_item(&format!("Song {i}"), &format!("Artist {}", i % 50), 200);
            plib.add_item(item).unwrap();
        }
    }

    c.bench_function("persistent_reopen_500_items", |b| {
        b.iter(|| {
            let _plib = PersistentLibrary::open(black_box(&path)).unwrap();
        });
    });

    let _ = std::fs::remove_file(&path);
}

fn bench_recommendation(c: &mut Criterion) {
    let mut lib = jalwa_core::Library::new();
    for i in 0..1000 {
        let mut item = make_media_item(
            &format!("Song {i}"),
            &format!("Artist {}", i % 20),
            120 + (i % 300) as u64,
        );
        if i % 3 == 0 {
            item.tags = vec!["rock".to_string()];
        }
        if i % 5 == 0 {
            item.play_count = 10;
        }
        lib.add_item(item);
    }
    let seed_id = lib.items[0].id;

    c.bench_function("recommend_1000_items_top10", |b| {
        b.iter(|| jalwa_ai::recommend(black_box(&lib), black_box(seed_id), 10));
    });
}

fn bench_smart_playlist(c: &mut Criterion) {
    let mut lib = jalwa_core::Library::new();
    for i in 0..1000 {
        let mut item = make_media_item(
            &format!("Song {i}"),
            &format!("Artist {}", i % 20),
            120 + (i % 300) as u64,
        );
        item.tags = vec![if i % 2 == 0 { "rock" } else { "jazz" }.to_string()];
        lib.add_item(item);
    }

    c.bench_function("smart_playlist_1000_items", |b| {
        b.iter(|| {
            jalwa_ai::generate_smart_playlist(
                black_box(&lib),
                "Rock",
                &[jalwa_ai::SmartCriteria::Tag("rock".to_string())],
            )
        });
    });
}

criterion_group!(
    benches,
    bench_persistent_library_add,
    bench_persistent_library_reopen,
    bench_recommendation,
    bench_smart_playlist,
);
criterion_main!(benches);
