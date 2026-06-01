use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_read_file_bytes(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();

    let small_path = dir.path().join("small.bin");
    std::fs::write(&small_path, vec![0xAAu8; 1024]).unwrap();

    let large_path = dir.path().join("large.bin");
    std::fs::write(&large_path, vec![0xBBu8; 2 * 1024 * 1024]).unwrap();

    let mut group = c.benchmark_group("read_file_bytes");
    group.bench_function("1KiB_fs_read", |b| {
        b.iter(|| atomwrite::file_io::read_file_bytes(black_box(&small_path), u64::MAX).unwrap())
    });
    group.bench_function("2MiB_mmap", |b| {
        b.iter(|| atomwrite::file_io::read_file_bytes(black_box(&large_path), u64::MAX).unwrap())
    });
    group.finish();
}

fn bench_read_file_string(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("text.txt");
    let content: String = "Hello world! This is a benchmark line.\n".repeat(1700);
    std::fs::write(&path, &content).unwrap();

    c.bench_function("read_file_string_64KiB", |b| {
        b.iter(|| atomwrite::file_io::read_file_string(black_box(&path), u64::MAX).unwrap())
    });
}

criterion_group!(benches_read, bench_read_file_bytes, bench_read_file_string);
criterion_main!(benches_read);
