//! Published throughput numbers live here. Run: `cargo bench`
//! (release-mode; results in target/criterion).

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use swarmail::smtp::build_email;
use swarmail::store::Store;

fn sample(i: usize) -> Vec<u8> {
    format!(
        "From: noreply@x.io\r\nTo: user{i}@example.com\r\nSubject: bench {i}\r\n\r\nVerify https://x.io/v?t={i}. Your code is {i}2345.\r\n"
    )
    .into_bytes()
}

fn bench_parse_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingest");
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("build_email", "parse+extract"), |b| {
        let mut i = 0usize;
        b.iter(|| {
            let raw = sample(black_box(i));
            black_box(build_email(
                &raw,
                "bench",
                &["user@example.com".to_string()],
                "noreply@x.io".into(),
                0,
            ));
            i += 1;
        });
    });
    group.finish();
}

fn bench_full_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingest");
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("store", "insert"), |b| {
        let store = Store::new(0);
        let mut i = 0usize;
        b.iter(|| {
            let raw = sample(black_box(i));
            let email = build_email(
                &raw,
                "bench",
                &["user@example.com".to_string()],
                "noreply@x.io".into(),
                0,
            );
            store.insert(email);
            i += 1;
        });
    });
    group.finish();
}

criterion_group!(benches, bench_parse_extract, bench_full_insert);
criterion_main!(benches);
