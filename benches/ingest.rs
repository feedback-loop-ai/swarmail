//! Published throughput numbers live here. Run: `cargo bench`
//! (release-mode; results in target/criterion).

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use swarmail::smtp::build_email;
use swarmail::store::{Filter, Store};
use swarmail::threads;

/// The production default cap (`--max-per-inbox`, `default_value_t =
/// 100_000` in `src/config.rs`, pinned by `tests/config_cli.rs`) — the
/// steady-state benches pre-fill to exactly this, so a sample measures
/// inserts that each trigger one eviction, never growth. Hardcoded (not
/// read from config) so the bench is self-contained; the pin makes silent
/// drift between the two loud.
const DEFAULT_CAP: usize = 100_000;

fn sample(i: usize) -> Vec<u8> {
    format!(
        "From: noreply@x.io\r\nTo: user{i}@example.com\r\nSubject: bench {i}\r\n\r\nVerify https://x.io/v?t={i}. Your code is {i}2345.\r\n"
    )
    .into_bytes()
}

/// Fill the `bench` inbox to exactly `cap` mails — the steady-state shape.
/// Never timed: criterion measures only the inserts that follow it.
fn prefill(store: &Store, cap: usize) {
    for i in 0..cap {
        let raw = sample(i);
        let email = build_email(
            &raw,
            "bench",
            &["user@example.com".to_string()],
            "noreply@x.io".into(),
        );
        store
            .insert(email)
            .expect("bench prefill: a memory-only store has no commit to fail");
    }
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
            ));
            i += 1;
        });
    });
    group.finish();
}

/// The uncapped control: parse + extract + insert into `Store::new(0)`. The
/// store grows without bound during its (short) sampling window — this bench
/// measures the uncapped configuration, not a steady state; the cap-steady
/// sibling below is the shape production runs at, and the delta between the
/// two is the eviction cost decision 0003's law carries.
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
            );
            store
                .insert(email)
                .expect("uncapped insert leg: a memory-only store has no commit to fail");
            i += 1;
        });
    });
    group.finish();
}

/// Ingest at the shipped default cap, in steady state. Memory-only regime —
/// no `--data-file` (the persisted 250 cost is documented behavior under
/// decision 0001, not this bench's claim).
///
/// The store is prefilled to exactly [`DEFAULT_CAP`] OUTSIDE the timed
/// region; every measured insert then triggers exactly one eviction, so a
/// sample measures steady-state cap eviction (the shape production runs at)
/// instead of unbounded growth. After sampling, the steady-state canary
/// asserts the store never drifted: the sampled inbox still holds exactly
/// the cap, and every insert's eviction was counted. The canary proves the
/// measurement's honesty — eviction kept pace, the store never grew during
/// the sample — not the fix: it passes on a pre-fix tree too (slow eviction
/// is still honest eviction), which is what makes the published `before`
/// numbers trustworthy. Both counters are read outside the timed region, so
/// the check costs the measurement nothing.
fn bench_capped_ingest(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingest");
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("store", "insert@cap-steady"), |b| {
        let store = Store::new(DEFAULT_CAP);
        prefill(&store, DEFAULT_CAP);
        let inserted_before = store.emails_inserted();
        let dropped_before = store.emails_dropped();
        let mut i = DEFAULT_CAP;
        b.iter(|| {
            let raw = sample(black_box(i));
            let email = build_email(
                &raw,
                "bench",
                &["user@example.com".to_string()],
                "noreply@x.io".into(),
            );
            store
                .insert(email)
                .expect("cap-steady insert leg: a memory-only store has no commit to fail");
            i += 1;
        });
        let inserted = store.emails_inserted() - inserted_before;
        let dropped = store.emails_dropped() - dropped_before;
        let len = store.count("bench", &Filter::default());
        assert_eq!(
            len, DEFAULT_CAP,
            "steady-state drift: the sampled inbox holds {len} mails, not the {DEFAULT_CAP} cap"
        );
        assert_eq!(
            dropped, inserted,
            "steady-state drift: {dropped} evictions for {inserted} inserts — the store grew during the sample"
        );
    });
    group.finish();
}

/// The capped query paths, on a store prefilled to the shipped default cap
/// (memory-only regime — no `--data-file`):
///
/// - `threads/view@cap` — the thread view the REST `/threads` surface and
///   the SSE feed render per frame (per accepted mail, per subscriber);
///   the view covers `VIEW_CAP` = 200 mails, but today it walks the whole
///   inbox before applying that cap.
/// - `store/count@cap` — the default-filter count that `/await`, `/assert`
///   and the burst harnesses poll.
/// - `store/count@cap-filtered` — a needle filter that matches nothing: the
///   pure-waste regime where `Filter::matches` lowers its needle per mail
///   (this regime's first measurement; a needle hoist is judged against
///   this number, not a prediction).
fn bench_capped_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingest");
    group.throughput(Throughput::Elements(1));

    group.bench_function(BenchmarkId::new("threads", "view@cap"), |b| {
        let store = Store::new(DEFAULT_CAP);
        prefill(&store, DEFAULT_CAP);
        b.iter(|| black_box(threads::view(&store, "bench")));
    });

    group.bench_function(BenchmarkId::new("store", "count@cap"), |b| {
        let store = Store::new(DEFAULT_CAP);
        prefill(&store, DEFAULT_CAP);
        b.iter(|| black_box(store.count("bench", &Filter::default())));
    });

    group.bench_function(BenchmarkId::new("store", "count@cap-filtered"), |b| {
        let store = Store::new(DEFAULT_CAP);
        prefill(&store, DEFAULT_CAP);
        let needle = Filter {
            to: Some("nobody@nowhere".into()),
            ..Default::default()
        };
        b.iter(|| black_box(store.count("bench", &needle)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_extract,
    bench_full_insert,
    bench_capped_ingest,
    bench_capped_queries
);
criterion_main!(benches);
