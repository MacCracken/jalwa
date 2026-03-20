//! Benchmarks for jalwa-core library operations.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use jalwa_core::test_fixtures::make_media_item;
use jalwa_core::Library;
use std::path::Path;
use uuid::Uuid;

fn make_library(n: usize) -> Library {
    let mut lib = Library::new();
    for i in 0..n {
        lib.add_item(make_media_item(
            &format!("Song {i}"),
            &format!("Artist {}", i % 50),
            180 + (i % 300) as u64,
        ));
    }
    lib
}

fn bench_find_by_id(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_by_id");
    for size in [100, 1_000, 10_000] {
        let lib = make_library(size);
        // Pick an ID from the middle
        let target_id = lib.items[size / 2].id;

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| lib.find_by_id(black_box(target_id)));
        });
    }
    group.finish();
}

fn bench_find_by_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_by_path");
    for size in [100, 1_000, 10_000] {
        let lib = make_library(size);
        let target_path = lib.items[size / 2].path.clone();

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| lib.find_by_path(black_box(&target_path)));
        });
    }
    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    for size in [100, 1_000, 10_000] {
        let lib = make_library(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| lib.search(black_box("Artist 25")));
        });
    }
    group.finish();
}

fn bench_add_item(c: &mut Criterion) {
    c.bench_function("add_item", |b| {
        b.iter_with_setup(
            || Library::new(),
            |mut lib| {
                let item = make_media_item("Bench Song", "Bench Artist", 200);
                lib.add_item(black_box(item));
            },
        );
    });
}

fn bench_remove_item(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove_item");
    for size in [100, 1_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_with_setup(
                || {
                    let lib = make_library(size);
                    let id = lib.items[size / 2].id;
                    (lib, id)
                },
                |(mut lib, id)| {
                    lib.remove(black_box(id));
                },
            );
        });
    }
    group.finish();
}

fn bench_db_roundtrip(c: &mut Criterion) {
    use jalwa_core::db::LibraryDb;

    c.bench_function("db_save_load_100_items", |b| {
        b.iter_with_setup(
            || {
                let path =
                    std::env::temp_dir().join(format!("jalwa_bench_{}.db", Uuid::new_v4()));
                let db = LibraryDb::open(&path).unwrap();
                let items: Vec<_> = (0..100)
                    .map(|i| make_media_item(&format!("Song {i}"), &format!("Artist {i}"), 200))
                    .collect();
                for item in &items {
                    db.save_item(item).unwrap();
                }
                (db, path)
            },
            |(db, path)| {
                let _lib = db.load_library().unwrap();
                let _ = std::fs::remove_file(&path);
            },
        );
    });
}

criterion_group!(
    benches,
    bench_find_by_id,
    bench_find_by_path,
    bench_search,
    bench_add_item,
    bench_remove_item,
    bench_db_roundtrip,
);
criterion_main!(benches);
