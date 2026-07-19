//! Criterion benches for smart file read (heap vs mmap) + tail percentiles.
//!
//! Rules Rust latência: print P50/P99/P999/P9999 from a fixed sample set in
//! addition to Criterion's mean/CI summary.

use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

fn percentile_ns(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

fn report_percentiles(name: &str, iters: usize, mut f: impl FnMut()) {
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..8 {
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
    group.sample_size(80);
    group.measurement_time(Duration::from_secs(3));
    group.warm_up_time(Duration::from_millis(500));
}

fn bench_read_file_bytes(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();

    let small_path = dir.path().join("small.bin");
    std::fs::write(&small_path, vec![0xAAu8; 1024]).unwrap();

    let large_path = dir.path().join("large.bin");
    std::fs::write(&large_path, vec![0xBBu8; 2 * 1024 * 1024]).unwrap();

    let mut group = c.benchmark_group("read_file_bytes");
    configure_group(&mut group);

    group.throughput(Throughput::Bytes(1024));
    group.bench_function(BenchmarkId::new("path", "1KiB_fs_read"), |b| {
        b.iter(|| {
            atomwrite::file_io::read_file_bytes(black_box(&small_path), u64::MAX).unwrap()
        })
    });

    group.throughput(Throughput::Bytes(2 * 1024 * 1024));
    group.bench_function(BenchmarkId::new("path", "2MiB_mmap"), |b| {
        b.iter(|| {
            atomwrite::file_io::read_file_bytes(black_box(&large_path), u64::MAX).unwrap()
        })
    });
    group.finish();

    report_percentiles("read_1KiB_fs", 200, || {
        black_box(
            atomwrite::file_io::read_file_bytes(black_box(&small_path), u64::MAX).unwrap(),
        );
    });
    report_percentiles("read_2MiB_mmap", 40, || {
        black_box(
            atomwrite::file_io::read_file_bytes(black_box(&large_path), u64::MAX).unwrap(),
        );
    });
}

fn bench_read_file_string(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("text.txt");
    let content: String = "Hello world! This is a benchmark line.\n".repeat(1700);
    std::fs::write(&path, &content).unwrap();

    let mut group = c.benchmark_group("read_file_string");
    configure_group(&mut group);
    group.throughput(Throughput::Bytes(content.len() as u64));
    group.bench_function("~64KiB", |b| {
        b.iter(|| atomwrite::file_io::read_file_string(black_box(&path), u64::MAX).unwrap())
    });
    group.finish();

    report_percentiles("read_string_64KiB", 100, || {
        black_box(atomwrite::file_io::read_file_string(black_box(&path), u64::MAX).unwrap());
    });
}

criterion_group!(benches_read, bench_read_file_bytes, bench_read_file_string);
criterion_main!(benches_read);
