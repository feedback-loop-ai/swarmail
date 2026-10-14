# Tasks — total shutdown: one owner per task, one ledger per death

Ordered; each task names the test that proves it. Scope guard: no new files under `src/`
beyond none (six existing production files + README), no new dependencies, no decision
document (the spec's non-semantic ruling), no edits to `insert`/`insert_async` or the
commit→250 leg (`smtp.rs:461-462`), no `coverage(off)`, no new harness file unless an arm
proves unwritable warm (0020 rule 2 — none is expected). The ruling lives in
[`spec.md`](spec.md); the mechanism and the coverage driver map in
[`plan.md`](plan.md).

- [ ] 1. Reorder startup and replace the tracker: create the shutdown watch where the
       store is born (before TLS, before any bind); call
       `spawn_dispatcher(store, webhooks, shutdown_rx.clone())` immediately after it and
       push its returned handle into the tracker; change `RunningServer.tasks` from
       `Vec<JoinHandle<()>>` to `tokio::task::JoinSet<()>` and `stop()` to send the
       signal then `while self.tasks.join_next().await.is_some() {}` — join, not abort
       (the serve tasks exit gracefully; aborting the HTTP serve task would kill benign
       in-flight requests). The dispatcher's firehose subscription now provably precedes
       the first bind (0002 one layer down); **no cleanup line is added to `run_on`'s
       error legs** (spec ruling — the store sender and watch sender both drop there and
       the dispatcher self-resolves).
       — *Proved by: `tests/lifecycle.rs`
       `servers_serve_then_stop_and_release_their_ports` still green (stop returns, both
       ports refuse); a new assertion that a mail sent before any listener binds is still
       delivered to a registered webhook (subscription-before-bind, spec adoption 1);
       `cargo clippy` clean on the JoinSet swap.*

- [ ] 2. Build the per-instance ledger on `Webhooks`: five `AtomicU64` fields
       (`delivered`, `dropped`, `lagged`, `exhausted`, `aborted`) on the existing
       `Default` struct — no statics; the `CACHE_TESTS` mutex is the statics precedent's
       price, not its endorsement — plus a `Copy` `snapshot()` accessor in the
       `CaCacheStats` shape (the one new type), and
       `pub const WEBHOOK_DELIVERY_CAP: usize = 32` with its doc sentence.
       — *Proved by: `src/api.rs` metrics assertions — the five
       `swarmail_webhook_{delivered,dropped,lagged,exhausted,aborted}_total` rows render
       for a fresh server at zero (`tests/api.rs`); a `#[tokio::test]` asserting two
       independent `Webhooks::default()` ledgers count independently (the AC5
       cross-contamination refusal, spec ruling 1).*

- [ ] 3. Rewrite `dispatch_loop`: signature `(rx, webhooks, shutdown_rx)`; a **biased**
       `select!` (firehose first, shutdown second); the `Mail` arm's **first statement**
       is the shutdown check (`shutdown_rx.has_changed()`) — if signaled, count the
       mail's matching targets `aborted` and fall through to the tail, never building or
       spawning (stop latency decoupled from backlog × payload size; spec amendment 2).
       Per pass harvest (`while deliveries.try_join_next().is_some() {}`), check the
       bound (`deliveries.len() < WEBHOOK_DELIVERY_CAP`) **before** building the payload
       once per mail as `Arc<str>` (a certain drop allocates nothing; spec adoption 4),
       then per matching target spawn under a re-checked bound or increment `dropped`.
       One tail for both the shutdown and `Closed` arms: a `try_recv` sweep counting
       remaining ring residents' matching targets `aborted` (the sweep robustness named,
       the ledger invariant now requires), then `abort_all()` + drain counting every
       `Err` as `aborted`. Update the classify helper to return the lagged count and add
       it to the ledger. `spawn_dispatcher` returns its `JoinHandle`.
       — *Proved by: the updated inline `loop_tests` — `dispatch_loop_handles_mail_lag_
       and_close` asserts `lagged` after a capacity-1 ring; a new inline stop case
       (watch fired, then two mails sent: the first counted at the arm check, the second
       swept at the tail, the sink records nothing, the loop returns); a new inline
       fast + stalled in-process sink case (deterministic per plan.md's driver map:
       spin-yield on `snapshot().delivered == 1`, then fire the watch) asserting the
       drain counts the finished delivery as its own outcome and the stalled one as
       `aborted` (AC2's counting half); the forced-storm test below drives the
       cap-check-false arm.*

- [ ] 4. Make `deliver` self-report as its final statements: 2xx → `delivered`, budget
       end → `exhausted` (both after the function's last await, so a reported task is
       never also aborted); pass the shared `Arc<str>` body; `X-Swarmail-Secret`,
       `ca_pem` and the URL rules unchanged. The commit→250 leg and the retry budget
       numbers are untouched.
       — *Proved by: the existing `tests/webhook.rs` suites (secret header, https, the
       budget) stay green and now assert the rows move — `delivered` after a 200 sink,
       `exhausted` after a 500-sink budget burn (exact, per-inbox, no statics).*

- [ ] 5. Give each accept loop its session set: `let mut sessions = JoinSet::new()` in
       `smtp::serve` and `pop3::serve`; sessions spawn into it; the existing shutdown
       arm does `sessions.abort_all()` then drains before `return Ok(())`. No goodbye is
       written; the closed socket is the signal.
       — *Proved by: AC3's lifecycle tests (mid-transaction SMTP and POP3 sessions:
       stop completes under the deadline, both ports refuse, each session socket reads
       EOF, exact stored count on inboxes the sessions never acked).*

- [ ] 6. End the feed cooperatively: `feed_stream`/`feed_events` take a
       `watch::Receiver<bool>`; the unfold's post-snapshot poll selects the watcher
       `recv()` against `shutdown.changed()` → `None` (a closed sender rides the same
       arm — the error path's drop-cascade is the dispatcher's startup self-resolution);
       the `Closed` arm and the snapshot-first ordering are unchanged. `AppState` gains
       the receiver; `inbox_feed` passes it.
       — *Proved by: new inline `feed.rs` unit tests (watch fired mid-stream → the
       stream ends after one trailing frame; sender dropped mid-stream → the same) plus
       AC1's lifecycle test; the existing feed unit tests extend their constructors with
       a fresh channel.*

- [ ] 7. Extend `tests/lifecycle.rs` with the shutdown-integration proofs (AC1, AC2,
       AC3): a feed open across `stop()` (stop returns under a real deadline, the feed
       socket reads EOF, ports refuse); a webhook at a stalled sink with a delivery in
       flight — **deadline 10 s, strictly between the millisecond abort path and the
       corrected 22.1 s drain path, so a drain masquerading as an abort fires it**
       (spec correction 3) — asserting stop returned *and* the sink's connection ended;
       mid-transaction sessions per task 5. Real servers, per-test inboxes, no
       sleep-as-sync: every wait is a real deadline or a production observable.
       — *Proved by: the tests themselves, red today (the orphaned delivery's socket
       stays open past the deadline; stop hangs with the feed open) and green after.*

- [ ] 8. Extend `tests/webhook.rs` (and `tests/rate.rs`'s posture) with the bound and
       ledger proofs (AC4, AC5, AC6, AC7): a delaying real sink counting its own
       concurrent requests (max observed concurrency exactly 32, never above); a forced
       storm of N mails against one held-stalled target with `dropped` reading exactly
       N − 32 on `/metrics` while every SMTP send answered 250 and all N stored
       (AC6's never-block-acceptance is the same test's other half); the pruning-
       survival test (sink captures the request, delays the reply; inbox filled past its
       cap; the received body byte-identical to the enqueued payload).
       — *Proved by: the tests themselves; `SINK_LIFETIME` (25 s) already outlives the
       corrected 22.1 s budget, which is what makes a drain distinguishable from an
       abort on the wire.*

- [ ] 9. Write the contract where the behavior is documented: the README Webhooks
       section states best-effort with observable loss, **no durable outbox**, the cap
       value `swarmail::webhook::WEBHOOK_DELIVERY_CAP` = 32, the five
       `swarmail_webhook_*_total` names (AC5 asserts the dropped string), the
       cumulative-since-start caveat, and the stalled-slot bound (at most 22.1 s).
       — *Proved by: a `tests/api.rs`-style assertion that `/metrics` carries the five
       names (AC8); `git diff main -- README.md` shows only the Webhooks paragraph.*

- [ ] 10. Leave every gate green and commit exactly the spec's diff surface: `cargo fmt
        --all -- --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test`;
        `brokkr compile --bundle .`; `bash scripts/coverage-gate.sh` (exact 100/100/100 —
        if an arm is cold, fix the driver per plan.md's map, never the gate); `cargo deny
        check licenses`; `cargo bench --bench ingest` (the recorded criterion must not
        regress) — then a single commit of `src/lib.rs`, `src/webhook.rs`, `src/feed.rs`,
        `src/api.rs`, `src/smtp.rs`, `src/pop3.rs`, `README.md`, and the three suites.
        — *Proved by: all gates exit 0; `git show --stat` lists exactly the ten files;
        the driver map's every line is covered by the preserved
        `target/coverage/lcov.info` (AC2, AC5, and the coverage posture).*
