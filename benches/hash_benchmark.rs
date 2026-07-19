//! Criterion benches for BLAKE3 hashing + explicit tail-latency percentiles.
//!
//! Rules Rust latência: report P50/P99/P999/P9999 (not only mean). Criterion's
//! confidence intervals cover the head; `report_percentiles` prints the tail
//! from a fixed sample set under the same release codegen (`profile.bench`
//! inherits fat LTO / CGU=1).

use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

/// Sorted sample → percentile (nearest-rank, inclusive).
fn percentile_ns(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

/// Collect `iters` wall times of `f` and print tail percentiles to stderr.
fn report_percentiles(name: &str, iters: usize, mut f: impl FnMut()) {
    let mut samples = Vec::with_capacity(iters);
    // Warmup: touch code + caches without recording.
    for _ in 0..16 {
        f();
    }
    for _ in 0..iters {
        let t0 = Instant::now();
        f();
        samples.push(t0.elapsed().as_nanos() as u64);
    }
    samples.sort_unstable();
    let p50 = percentile_ns(&samples, 0.50);
    let p99 = percentile_ns(&samples, 0.99);
    let p999 = percentile_ns(&samples, 0.999);
    let p9999 = percentile_ns(&samples, 0.9999);
    let max = *samples.last().unwrap_or(&0);
    eprintln!(
        "latency_hist {name}: n={iters} P50={p50}ns P99={p99}ns P999={p999}ns P9999={p9999}ns max={max}ns"
    );
}

fn configure_group(group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>) {
    // Longer samples → more stable tail estimates than Criterion defaults.
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(3));
    group.warm_up_time(Duration::from_millis(500));
}

fn bench_hash_bytes(c: &mut Criterion) {
    let data_1k = vec![0xABu8; 1024];
    let data_64k = vec![0xABu8; 64 * 1024];
    let data_1m = vec![0xABu8; 1024 * 1024];

    let mut group = c.benchmark_group("hash_bytes");
    configure_group(&mut group);

    group.throughput(Throughput::Bytes(1024));
    group.bench_function(BenchmarkId::new("size", "1KiB"), |b| {
        b.iter(|| atomwrite::checksum::hash_bytes(black_box(&data_1k)))
    });

    group.throughput(Throughput::Bytes(64 * 1024));
    group.bench_function(BenchmarkId::new("size", "64KiB"), |b| {
        b.iter(|| atomwrite::checksum::hash_bytes(black_box(&data_64k)))
    });

    group.throughput(Throughput::Bytes(1024 * 1024));
    group.bench_function(BenchmarkId::new("size", "1MiB"), |b| {
        b.iter(|| atomwrite::checksum::hash_bytes(black_box(&data_1m)))
    });
    group.finish();

    // Explicit tail histogram (Rules: P50/P99/P999/P9999, not mean alone).
    report_percentiles("hash_bytes_1KiB", 500, || {
        black_box(atomwrite::checksum::hash_bytes(black_box(&data_1k)));
    });
    report_percentiles("hash_bytes_64KiB", 200, || {
        black_box(atomwrite::checksum::hash_bytes(black_box(&data_64k)));
    });
    report_percentiles("hash_bytes_1MiB", 80, || {
        black_box(atomwrite::checksum::hash_bytes(black_box(&data_1m)));
    });
}

fn bench_hash_file(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bench_file.bin");
    std::fs::write(&path, vec![0xCDu8; 1024 * 1024]).unwrap();

    let mut group = c.benchmark_group("hash_file");
    configure_group(&mut group);
    group.throughput(Throughput::Bytes(1024 * 1024));
    group.bench_function("1MiB", |b| {
        b.iter(|| {
            atomwrite::checksum::hash_file_with_len(black_box(&path), u64::MAX).unwrap()
        })
    });
    group.finish();

    report_percentiles("hash_file_with_len_1MiB", 60, || {
        black_box(
            atomwrite::checksum::hash_file_with_len(black_box(&path), u64::MAX).unwrap(),
        );
    });
}

criterion_group!(benches_hash, bench_hash_bytes, bench_hash_file);
criterion_main!(benches_hash);
