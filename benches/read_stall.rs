//! Microbenchmarks for the costs decision 0030 moved off the async
//! workers — not an old-vs-new compatibility harness: each lens measures
//! one shape against a baseline of the *same* code with one factor
//! changed, so the operator can read that factor's own cost.
//!
//! One process, an in-memory store, a current-thread runtime — the runtime
//! thread is the only async worker, so a parse or a page copy that runs on
//! it is a stall every other task waits out. Size-amplified (~256 KiB
//! default) mails make the copy costs visible. Four lenses:
//!
//! **Large-mail parse occupancy (the await's cost)** — the DATA arm parses
//! before it commits: the pre-0030 shape built the `Email` inline on the
//! runtime thread, so the SMTP session's own await occupied the only async
//! worker; the offloaded shape parks on the blocking pool (`Store::offload`
//! is exactly `spawn_blocking`, measured as such here) and the runtime
//! thread only waits. Measured with a 1 ms ticker on the same runtime,
//! against a yield-only baseline, exactly as `writer_stall` does.
//!
//! **Deep `Email` clone vs `Arc` handle clone** — the pre-0030 page
//! deep-cloned every candidate `Email` before serializing it; the `Arc`
//! page clones handles and serde serializes in place. Both shapes over
//! the same store snapshot: this is the serialization-cost lens, not a
//! search-engine comparison.
//!
//! **The contains fold** — the pre-0030 shape lowercased every haystack
//! into a fresh `String` per candidate; the shipped fold builds one regex
//! per request and matches in place (`swarmail::compat::contains`, not a
//! copy of it).
//!
//! **The per-wake view cost** — the SSE wake rebuilds the thread view per
//! subscriber with no coalescing (spec §2.5): `threads::view` over a
//! `VIEW_CAP`-sized inbox is that per-wake cost.
//!
//! No wall-clock validity assertions anywhere — the numbers are recorded
//! verbatim for the operator to rule on as measured (0023's discipline).
//!
//! Run (release): `cargo bench --bench read_stall`
//! Optional: `--mails N` (default 250 per page/fold lens), `--row-size
//! BYTES` (default 262144 — pass `--row-size 52428800` to reach the real
//! 50 MiB DATA cap, `src/smtp.rs`'s max body), `--iters N` (default 40
//! per measured page, fold and view), `--parses N` (default 100 per
//! parse phase).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use swarmail::compat::{contains, search_needle};
use swarmail::model::Email;
use swarmail::smtp::build_email;
use swarmail::store::{Filter, Store};
use swarmail::threads;

/// A raw mail of ~`row_size` bytes; the body is what lands in the row and
/// the haystacks the fold has to walk.
fn sample(i: usize, row_size: usize) -> Vec<u8> {
    let filler = "x".repeat(row_size.saturating_sub(160));
    format!(
        "From: noreply@x.io\r\nTo: user{i}@example.com\r\nSubject: read-stall {i}\r\n\r\n{filler}\r\n"
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
    let mails = int_arg(&args, "--mails").unwrap_or(250);
    let row_size = int_arg(&args, "--row-size").unwrap_or(256 * 1024);
    let iters = int_arg(&args, "--iters").unwrap_or(40);
    let parses = int_arg(&args, "--parses").unwrap_or(100);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread runtime");
    runtime.block_on(async move {
        let store = Arc::new(Store::new(0));

        // Warm the allocator, the regex machinery and the code paths; none
        // of this is measured.
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
                .expect("stall-bench warmup: an in-memory store has no commit to fail");
        }

        // ---- Lens 1: the parse the DATA arm awaits, inline vs offloaded ----

        // The handoff floor: a spawn_blocking round trip on an empty task.
        let mut handoff = Vec::with_capacity(parses);
        for _ in 0..parses {
            let at = Instant::now();
            tokio::task::spawn_blocking(|| {}).await.unwrap();
            handoff.push(at.elapsed());
        }
        let (h_p50, h_p99, h_max) = percentiles(handoff);

        // Phase A — the old shape: `build_email` inline on the runtime
        // thread. Each sample is one parse's occupancy of the only async
        // worker — the stall every parked session waited out.
        let raws: Vec<Vec<u8>> = (0..parses).map(|i| sample(i, row_size)).collect();
        let mut inline_occupancy = Vec::with_capacity(parses);
        let done = Arc::new(AtomicBool::new(false));
        let ticker = spawn_ticker(done.clone());
        for raw in &raws {
            let at = Instant::now();
            let email = build_email(raw, "stall", &["user@example.com".to_string()], "noreply@x.io".into());
            std::hint::black_box(&email);
            inline_occupancy.push(at.elapsed());
            tokio::task::yield_now().await;
        }
        done.store(true, Ordering::Relaxed);
        let (inline_p99, inline_max, inline_ticks) = ticker.await.expect("ticker task");
        let (i_p50, i_p99, i_max) = percentiles(inline_occupancy);

        // Phase B — the offloaded shape: the parse runs on the blocking
        // pool; the runtime thread's occupancy is the park (the handoff
        // floor plus the pool's own parse, invisible to this thread).
        let mut off_occupancy = Vec::with_capacity(parses);
        let done = Arc::new(AtomicBool::new(false));
        let ticker = spawn_ticker(done.clone());
        for raw in raws {
            let at = Instant::now();
            let email = tokio::task::spawn_blocking(move || {
                build_email(&raw, "stall", &["user@example.com".to_string()], "noreply@x.io".into())
            })
            .await
            .unwrap();
            std::hint::black_box(&email);
            off_occupancy.push(at.elapsed());
        }
        done.store(true, Ordering::Relaxed);
        let (off_p99, off_max, off_ticks) = ticker.await.expect("ticker task");
        let (o_p50, o_p99, o_max) = percentiles(off_occupancy);

        // Baseline — yields only, for a fixed wall-clock window: the
        // ticker's floor on this box, so the two measured phases read as
        // deltas over it rather than against zero.
        let done = Arc::new(AtomicBool::new(false));
        let ticker = spawn_ticker(done.clone());
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(300) {
            tokio::task::yield_now().await;
        }
        done.store(true, Ordering::Relaxed);
        let (baseline_p99, baseline_max, baseline_ticks) = ticker.await.expect("ticker task");

        // ---- Lens 2: the compat search page, deep-clone vs Arc ----

        // A page-sized inbox of the same size-amplified mail.
        for i in 0..mails {
            let raw = sample(i, row_size);
            let email = build_email(
                &raw,
                "page",
                &["user@example.com".to_string()],
                "noreply@x.io".into(),
            );
            store
                .insert(email)
                .expect("page lens warmup: an in-memory store has no commit to fail");
        }
        let filter = Filter {
            to: None,
            from: None,
            subject: None,
            since_ms: None,
        };
        let all = store.list("page", &filter);
        let mut deep_clone_pages = Vec::with_capacity(iters);
        for _ in 0..iters {
            let at = Instant::now();
            // The pre-0030 page: every candidate `Email` cloned whole, then
            // serialized from the clones.
            let page: Vec<Email> = all.iter().take(mails).map(|e| (**e).clone()).collect();
            let bytes = serde_json::to_vec(&page).unwrap();
            std::hint::black_box(&bytes);
            deep_clone_pages.push(at.elapsed());
        }
        let (d_p50, d_p99, d_max) = percentiles(deep_clone_pages);
        let mut arc_pages = Vec::with_capacity(iters);
        for _ in 0..iters {
            let at = Instant::now();
            // The shipped page: handle clones, serde serializes in place.
            let page: Vec<Arc<Email>> = all.iter().take(mails).cloned().collect();
            let bytes = serde_json::to_vec(&page).unwrap();
            std::hint::black_box(&bytes);
            arc_pages.push(at.elapsed());
        }
        let (a_p50, a_p99, a_max) = percentiles(arc_pages);

        // ---- Lens 3: the contains fold, lowercase-alloc vs regex ----

        let needle = "user3";
        let mut lowercase_folds = Vec::with_capacity(iters);
        for _ in 0..iters {
            let at = Instant::now();
            // The pre-0030 fold, verbatim: two full-size allocations per
            // field per candidate, plus the lossy copy for the raw source.
            let mut hits = 0usize;
            for email in &all {
                let mut haystacks: Vec<String> = Vec::new();
                if let Some(text) = &email.text {
                    haystacks.push(text.to_lowercase());
                }
                if let Some(html) = &email.html {
                    haystacks.push(html.to_lowercase());
                }
                if let Some(subject) = &email.subject {
                    haystacks.push(subject.to_lowercase());
                }
                for address in email.to.iter().chain(email.cc.iter()) {
                    haystacks.push(address.address.to_lowercase());
                }
                if let Some(from) = &email.from {
                    haystacks.push(from.address.to_lowercase());
                }
                haystacks.push(String::from_utf8_lossy(&email.raw).to_lowercase());
                let needle = needle.to_lowercase();
                if haystacks.iter().any(|h| h.contains(&needle)) {
                    hits += 1;
                }
            }
            std::hint::black_box(hits);
            lowercase_folds.push(at.elapsed());
        }
        let (l_p50, l_p99, l_max) = percentiles(lowercase_folds);
        let re = search_needle(needle);
        let mut regex_folds = Vec::with_capacity(iters);
        for _ in 0..iters {
            let at = Instant::now();
            let mut hits = 0usize;
            for email in &all {
                if contains(email, &re) {
                    hits += 1;
                }
            }
            std::hint::black_box(hits);
            regex_folds.push(at.elapsed());
        }
        let (r_p50, r_p99, r_max) = percentiles(regex_folds);

        // ---- Lens 4: the per-wake view cost ----

        let mut views = Vec::with_capacity(iters);
        for _ in 0..iters {
            let at = Instant::now();
            let view = threads::view(&store, "page");
            std::hint::black_box(&view);
            views.push(at.elapsed());
        }
        let (v_p50, v_p99, v_max) = percentiles(views);

        // ---- Report ----
        println!("read-stall bench — one process, an in-memory store, current-thread runtime");
        println!(
            "mails per page/fold lens: {mails}  row size: {row_size} B  iters: {iters}  parses per phase: {parses}"
        );
        println!();
        println!("large-mail parse occupancy of the calling thread (the DATA await's cost):");
        println!(
            "  inline build_email (old)  p50={:>7.3} ms  p99={:>7.3} ms  max={:>7.3} ms",
            ms(i_p50),
            ms(i_p99),
            ms(i_max)
        );
        println!(
            "  spawn_blocking handoff    p50={:>7.3} us  p99={:>7.3} us  max={:>7.3} us   (the floor the offloaded shape pays)",
            us(h_p50),
            us(h_p99),
            us(h_max)
        );
        println!(
            "  offloaded await (new)     p50={:>7.3} us  p99={:>7.3} us  max={:>7.3} ms   (handoff + the pool's parse, invisible here)",
            us(o_p50),
            us(o_p99),
            ms(o_max)
        );
        println!();
        println!("1 ms ticker overshoot on the same runtime (what every parked session feels):");
        println!(
            "  yield only (baseline)     {:>18.3} ms {:>18.3} ms {baseline_ticks:>14}",
            ms(baseline_p99),
            ms(baseline_max)
        );
        println!(
            "  inline parse (old shape)  {:>18.3} ms {:>18.3} ms {inline_ticks:>14}",
            ms(inline_p99),
            ms(inline_max)
        );
        println!(
            "  offloaded parse (new)     {:>18.3} ms {:>18.3} ms {off_ticks:>14}",
            ms(off_p99),
            ms(off_max)
        );
        println!();
        println!("compat search page over {mails} large mails (per request, on the calling thread):");
        println!(
            "  deep-clone page (old)     p50={:>7.3} us  p99={:>7.3} us  max={:>7.3} us",
            us(d_p50),
            us(d_p99),
            us(d_max)
        );
        println!(
            "  Arc page (new)            p50={:>7.3} us  p99={:>7.3} us  max={:>7.3} us",
            us(a_p50),
            us(a_p99),
            us(a_max)
        );
        println!();
        println!("contains fold over {mails} large mails (per request, on the calling thread):");
        println!(
            "  lowercase allocs (old)    p50={:>7.3} us  p99={:>7.3} us  max={:>7.3} us",
            us(l_p50),
            us(l_p99),
            us(l_max)
        );
        println!(
            "  regex fold (new)          p50={:>7.3} us  p99={:>7.3} us  max={:>7.3} us",
            us(r_p50),
            us(r_p99),
            us(r_max)
        );
        println!();
        println!("per-wake thread view over a {mails}-mail inbox (per subscriber, per wake):");
        println!(
            "  threads::view             p50={:>7.3} us  p99={:>7.3} us  max={:>7.3} us",
            us(v_p50),
            us(v_p99),
            us(v_max)
        );
        println!();
        println!("verbatim numbers (for the decision note):");
        println!(
            "parse_inline_p50_ms={:.3} parse_inline_p99_ms={:.3} parse_inline_max_ms={:.3}",
            ms(i_p50),
            ms(i_p99),
            ms(i_max)
        );
        println!(
            "handoff_p50_us={:.3} handoff_p99_us={:.3} handoff_max_us={:.3}",
            us(h_p50),
            us(h_p99),
            us(h_max)
        );
        println!(
            "parse_offloaded_p50_us={:.3} parse_offloaded_p99_us={:.3} parse_offloaded_max_ms={:.3}",
            us(o_p50),
            us(o_p99),
            ms(o_max)
        );
        println!(
            "baseline: p99_overshoot_ms={:.3} max_overshoot_ms={:.3} samples={baseline_ticks}",
            ms(baseline_p99),
            ms(baseline_max)
        );
        println!(
            "inline:   p99_overshoot_ms={:.3} max_overshoot_ms={:.3} samples={inline_ticks}",
            ms(inline_p99),
            ms(inline_max)
        );
        println!(
            "offload:  p99_overshoot_ms={:.3} max_overshoot_ms={:.3} samples={off_ticks}",
            ms(off_p99),
            ms(off_max)
        );
        println!(
            "page_deep_clone_p50_us={:.3} page_deep_clone_p99_us={:.3} page_deep_clone_max_us={:.3}",
            us(d_p50),
            us(d_p99),
            us(d_max)
        );
        println!(
            "page_arc_p50_us={:.3} page_arc_p99_us={:.3} page_arc_max_us={:.3}",
            us(a_p50),
            us(a_p99),
            us(a_max)
        );
        println!(
            "fold_lowercase_p50_us={:.3} fold_lowercase_p99_us={:.3} fold_lowercase_max_us={:.3}",
            us(l_p50),
            us(l_p99),
            us(l_max)
        );
        println!(
            "fold_regex_p50_us={:.3} fold_regex_p99_us={:.3} fold_regex_max_us={:.3}",
            us(r_p50),
            us(r_p99),
            us(r_max)
        );
        println!(
            "view_p50_us={:.3} view_p99_us={:.3} view_max_us={:.3}",
            us(v_p50),
            us(v_p99),
            us(v_max)
        );
        // The store is dropped here; an in-memory store holds no file, and
        // the data-file variant of this bench deliberately does not exist —
        // the read plane's stall is CPU, not commit I/O.
    });
}
