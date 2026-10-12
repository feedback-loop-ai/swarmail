//! The async worker stall a write-heavy loop inflicts on the runtime thread,
//! measured before and after the off-worker commit move (decision 0010).
//!
//! One process, one data file (`--data-file`), a current-thread runtime —
//! the runtime thread is the only async worker, so any commit that runs on
//! it is a stall every other task waits out. Size-amplified (~256 KiB) rows
//! make the commit cost visible. Two lenses:
//!
//! **Direct occupancy** — how long the mutation call itself occupies the
//! thread that calls it. The sync phase calls `Store::insert` on the runtime
//! thread (the old shape: memory half + prune + mirror commit inline), so
//! its per-mail duration *is* the per-mail async-worker stall. The
//! `spawn_blocking` round trip on an empty task is the floor the offloaded
//! shape pays instead (`Store::insert_async` parks the runtime thread on the
//! join handle while tokio's blocking pool commits).
//!
//! **1 ms ticker overshoot** — a ticker task on the same runtime records how
//! late each 1 ms tick fired while the writer loop runs. The baseline phase
//! (yields only, no mutations) establishes the ticker's floor — tokio's
//! timer granularity plus scheduler noise on the box — so the two measured
//! phases read as deltas over it rather than against zero. `yield_now`
//! between mails stands in for the await points a real session has.
//!
//! Every mail is parsed before its phase starts, so the measured loops are
//! the mutation alone.
//!
//! Run (release): `cargo bench --bench writer_stall -- --data-file /tmp/writer-stall.db`
//! Optional: `--rows N` (default 400 per phase), `--row-size BYTES`
//! (default 262144).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use swarmail::model::Email;
use swarmail::smtp::build_email;
use swarmail::store::Store;

/// A raw mail of ~`row_size` bytes; the body is what lands in the row.
fn sample(i: usize, row_size: usize) -> Vec<u8> {
    let filler = "x".repeat(row_size.saturating_sub(160));
    format!(
        "From: noreply@x.io\r\nTo: user{i}@example.com\r\nSubject: writer-stall {i}\r\n\r\n{filler}\r\n"
    )
    .into_bytes()
}

/// A 1 ms ticker task: every tick records how much later than 1 ms it
/// actually fired. Returns (p99 overshoot, max overshoot, samples).
fn spawn_ticker(done: Arc<AtomicBool>) -> tokio::task::JoinHandle<(Duration, Duration, usize)> {
    tokio::spawn(async move {
        let mut overshoots: Vec<Duration> = Vec::new();
        while !done.load(Ordering::Relaxed) {
            let at = Instant::now();
            tokio::time::sleep(Duration::from_millis(1)).await;
            overshoots.push(at.elapsed().saturating_sub(Duration::from_millis(1)));
        }
        let n = overshoots.len();
        overshoots.sort();
        let max = overshoots.last().copied().unwrap_or_default();
        let p99 = overshoots
            .get(n.saturating_sub(1).saturating_mul(99) / 100)
            .copied()
            .unwrap_or_default();
        (p99, max, n)
    })
}

/// (p50, p99, max) of a sample set.
fn percentiles(mut samples: Vec<Duration>) -> (Duration, Duration, Duration) {
    samples.sort();
    let n = samples.len();
    (
        samples[n / 2],
        samples[n.saturating_sub(1).saturating_mul(99) / 100],
        samples[n - 1],
    )
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn us(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000_000.0
}

fn int_arg(args: &[String], flag: &str) -> Option<usize> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let data_file = args
        .iter()
        .position(|a| a == "--data-file")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("swarmail-writer-stall.db"));
    let rows = int_arg(&args, "--rows").unwrap_or(400);
    let row_size = int_arg(&args, "--row-size").unwrap_or(256 * 1024);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread runtime");
    let cleanup_file = data_file.clone();
    runtime.block_on(async move {
        let store = Arc::new(Store::open(0, &data_file).expect("open the data file"));

        // Warm the file, the page cache and the allocator; these rows are
        // not measured.
        for i in 0..16usize {
            let raw = sample(i.wrapping_sub(16), row_size);
            let email = build_email(
                &raw,
                "warmup",
                &["user@example.com".to_string()],
                "noreply@x.io".into(),
            );
            store
                .insert(email)
                .expect("stall-bench warmup: a memory-only store has no commit to fail");
        }

        // Pre-parse every measured mail: the loops below must be the
        // mutation alone, or the parse would sit in both phases and dilute
        // the delta.
        let raws: Vec<Vec<u8>> = (0..rows * 2).map(|i| sample(i, row_size)).collect();
        let mails: Vec<Email> = raws
            .iter()
            .map(|raw| {
                build_email(
                    raw,
                    "stall",
                    &["user@example.com".to_string()],
                    "noreply@x.io".into(),
                )
            })
            .collect();
        let mut mails = mails.into_iter();

        // ---- Lens 1: direct per-mail occupancy of the calling thread ----

        // The handoff floor: a spawn_blocking round trip on an empty task.
        // The offloaded shape's per-mail runtime-thread cost is this, plus a
        // park — the commit itself runs on the blocking pool.
        let mut handoff = Vec::with_capacity(rows * 4);
        for _ in 0..rows * 4 {
            let at = Instant::now();
            tokio::task::spawn_blocking(|| {}).await.unwrap();
            handoff.push(at.elapsed());
        }
        let (h_p50, h_p99, h_max) = percentiles(handoff);

        // Phase A — the old shape: the whole mutation inline on the runtime
        // thread. Each sample is one mail's occupancy of the only async
        // worker.
        let mut sync_occupancy = Vec::with_capacity(rows);
        let done = Arc::new(AtomicBool::new(false));
        let ticker = spawn_ticker(done.clone());
        let started = Instant::now();
        for email in mails.by_ref().take(rows) {
            let at = Instant::now();
            store
                .insert(email)
                .expect("sync insert leg: a memory-only store has no commit to fail");
            sync_occupancy.push(at.elapsed());
            tokio::task::yield_now().await;
        }
        let sync_elapsed = started.elapsed();
        done.store(true, Ordering::Relaxed);
        let (sync_p99, sync_max, sync_ticks) = ticker.await.expect("ticker task");
        let (s_p50, s_p99, s_max) = percentiles(sync_occupancy);
        let sync_rate = rows as f64 / sync_elapsed.as_secs_f64();

        // Phase B — the new shape: the awaited wrapper runs the whole
        // mutation on the blocking pool; the runtime thread only parks.
        let done = Arc::new(AtomicBool::new(false));
        let ticker = spawn_ticker(done.clone());
        let started = Instant::now();
        for email in mails {
            store
                .clone()
                .insert_async(email)
                .await
                .expect("offloaded insert leg: a memory-only store has no commit to fail");
        }
        let off_elapsed = started.elapsed();
        done.store(true, Ordering::Relaxed);
        let (off_p99, off_max, off_ticks) = ticker.await.expect("ticker task");
        let off_rate = rows as f64 / off_elapsed.as_secs_f64();

        // ---- Lens 2: the 1 ms ticker's overshoot, per phase ----

        // Baseline — the same writer loop with the mutation removed: pure
        // yields for a fixed wall-clock window. The overshoot it records is
        // the ticker's floor (timer granularity + scheduler noise on this
        // box), so the two measured phases read as deltas over it rather
        // than against zero.
        let done = Arc::new(AtomicBool::new(false));
        let ticker = spawn_ticker(done.clone());
        let started = Instant::now();
        let mut baseline_iters = 0u64;
        while started.elapsed() < Duration::from_millis(300) {
            tokio::task::yield_now().await;
            baseline_iters += 1;
        }
        let baseline_elapsed = started.elapsed();
        done.store(true, Ordering::Relaxed);
        let (baseline_p99, baseline_max, baseline_ticks) = ticker.await.expect("ticker task");
        let baseline_rate = baseline_iters as f64 / baseline_elapsed.as_secs_f64();

        // ---- Report ----
        println!("writer-stall bench — one process, one data file, current-thread runtime");
        println!(
            "data file: {}  rows per phase: {rows}  row size: {row_size} B  PRAGMA synchronous=FULL",
            data_file.display()
        );
        println!();
        println!("per-mail async-worker occupancy (direct, on the calling thread):");
        println!(
            "  sync insert (old shape)   p50={:>7.3} ms  p99={:>7.3} ms  max={:>7.3} ms",
            ms(s_p50),
            ms(s_p99),
            ms(s_max)
        );
        println!(
            "  spawn_blocking handoff    p50={:>7.3} us  p99={:>7.3} us  max={:>7.3} us   (the floor the offloaded shape pays)",
            us(h_p50),
            us(h_p99),
            us(h_max)
        );
        println!();
        println!("1 ms ticker overshoot on the same runtime (what every other task feels):");
        println!(
            "  phase                        ops/s   tick p99 overshoot   tick max overshoot   tick samples"
        );
        println!("  (ops = mails in the two mutation phases, yields in the baseline)");
        println!(
            "  yield only (baseline)      {baseline_rate:>7.0}   {:>18.3} ms {:>18.3} ms {baseline_ticks:>14}",
            ms(baseline_p99),
            ms(baseline_max)
        );
        println!(
            "  sync insert (old shape)    {sync_rate:>7.0}   {:>18.3} ms {:>18.3} ms {sync_ticks:>14}",
            ms(sync_p99),
            ms(sync_max)
        );
        println!(
            "  insert_async (offloaded)   {off_rate:>7.0}   {:>18.3} ms {:>18.3} ms {off_ticks:>14}",
            ms(off_p99),
            ms(off_max)
        );
        println!();
        println!("verbatim numbers (for the decision note):");
        println!(
            "occupancy_sync_p50_ms={:.3} occupancy_sync_p99_ms={:.3} occupancy_sync_max_ms={:.3}",
            ms(s_p50),
            ms(s_p99),
            ms(s_max)
        );
        println!(
            "handoff_p50_us={:.3} handoff_p99_us={:.3} handoff_max_us={:.3}",
            us(h_p50),
            us(h_p99),
            us(h_max)
        );
        println!(
            "baseline: p99_overshoot_ms={:.3} max_overshoot_ms={:.3} samples={baseline_ticks}",
            ms(baseline_p99),
            ms(baseline_max)
        );
        println!(
            "sync:     p99_overshoot_ms={:.3} max_overshoot_ms={:.3} mails_per_sec={sync_rate:.0} samples={sync_ticks}",
            ms(sync_p99),
            ms(sync_max)
        );
        println!(
            "offload:  p99_overshoot_ms={:.3} max_overshoot_ms={:.3} mails_per_sec={off_rate:.0} samples={off_ticks}",
            ms(off_p99),
            ms(off_max)
        );
    });

    // Best-effort scratch cleanup.
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", cleanup_file.display()));
    }
}
