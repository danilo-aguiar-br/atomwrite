use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_hash_bytes(c: &mut Criterion) {
    let data_1k = vec![0xABu8; 1024];
    let data_64k = vec![0xABu8; 64 * 1024];
    let data_1m = vec![0xABu8; 1024 * 1024];

    let mut group = c.benchmark_group("hash_bytes");
    group.bench_function("1KiB", |b| {
        b.iter(|| atomwrite::checksum::hash_bytes(black_box(&data_1k)))
    });
    group.bench_function("64KiB", |b| {
        b.iter(|| atomwrite::checksum::hash_bytes(black_box(&data_64k)))
    });
    group.bench_function("1MiB", |b| {
        b.iter(|| atomwrite::checksum::hash_bytes(black_box(&data_1m)))
    });
    group.finish();
}

fn bench_hash_file(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bench_file.bin");
    std::fs::write(&path, vec![0xCDu8; 1024 * 1024]).unwrap();

    c.bench_function("hash_file_1MiB", |b| {
        b.iter(|| atomwrite::checksum::hash_file(black_box(&path), u64::MAX).unwrap())
    });
}

criterion_group!(benches_hash, bench_hash_bytes, bench_hash_file);
criterion_main!(benches_hash);
