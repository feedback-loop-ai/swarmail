# Tasks — total shutdown: one owner per task, one ledger per death

Ordered; each task names the test that proves it. Scope guard: no new files under `src/`
beyond none (six existing production files + README), no new dependencies, no decision
document (the spec's non-semantic ruling), no edits to `insert`/`insert_async` or the
commit→250 leg (`src/smtp.rs:459-462`), no `coverage(off)`, no new harness file unless an
arm proves unwritable warm (0011 rule 2 — none is expected). The ruling lives in
[`spec.md`](spec.md); the mechanism and the coverage driver map in
[`plan.md`](plan.md).

- [ ] 1. Reorder startup and replace the tracker: create the shutdown watch where the
       store is born (before TLS, before any bind); call
       `spawn_dispatcher(store, webhooks, shutdown_rx.clone())` immediately after it and
       push its returned handle into the tracker; change `RunningServer.tasks` from
       `Vec<JoinHandle<()>>` to `tokio::task::JoinSet<()>` and `stop()` to send the
       signal then `while self.tasks.join_next().await.is_some() {}`. The dispatcher's
       firehose subscription now provably precedes the first bind (0002 one layer down).
       — *Proved by: `tests/lifecycle.rs` `servers_serve_then_stop_and_release_their_ports`
       still green (stop returns, both ports refuse); a new assertion that a mail sent
       before any listener binds is still delivered to a registered webhook
       (subscription-before-bind, spec adoption 1); `cargo clippy` clean on the JoinSet
       swap (AC2's tracker).*

- [ ] 2. Build the per-instance ledger on `Webhooks`: a small default-zeroed `Ledger`
       (`delivered`, `dropped`, `lagged`, `exhausted`, `aborted` `AtomicU64`) with a Copy
       `snapshot()` in the `CaCacheStats` shape, plus
       `pub const WEBHOOK_DELIVERY_CAP: usize = 32` with its doc sentence. No statics —
       the `CACHE_TESTS` mutex is the precedent's price, not its endorsement.
       — *Proved by: `src/api.rs` metrics assertions — the five
       `swarmail_webhook_{delivered,dropped,lagged,exhausted,aborted}_total` rows render
       for a fresh server at zero (`tests/api.rs`); a `#[tokio::test]` asserting two
       independent `Webhooks::default()` ledgers count independently (the AC5
       cross-contamination refusal, spec ruling 1).*

- [ ] 3. Rewrite `dispatch_loop`: signature `(rx, webhooks, shutdown_rx)`; a **biased**
       `select!` (firehose first, shutdown second); per pass harvest
       (`while deliveries.try_join_next().is_some() {}`), build the payload **once** per
       mail as `Arc<str>`, then per matching target spawn under the cap
       (`deliveries.len() < WEBHOOK_DELIVERY_CAP`) or increment `dropped` and skip; on
       shutdown or firehose close, `abort_all()` then drain, counting every `Err` as
       `aborted`. Update the classify helper to return the lagged count and add it to the
       ledger. `spawn_dispatcher` returns its `JoinHandle`.
       — *Proved by: the updated inline `loop_tests` — `dispatch_loop_handles_mail_lag_and_close`
       asserts `lagged` after a capacity-1 ring; a new inline test with a fast + a stalled
       in-process sink (deterministic per plan.md's driver map: spin-yield on
       `snapshot().delivered == 1`, then fire the watch) asserting the drain counts the
       finished delivery as its own outcome and the stalled one as `aborted`, and that
       the second pass reaps the first (AC2's counting half).*

- [ ] 4. Make `deliver` self-report as its final statements: 2xx → `delivered`, budget
       end → `exhausted` (both after the function's last await, so a reported task is
       never also aborted); pass the shared `Arc<str>` body; `X-Swarmail-Secret`,
       `ca_pem` and the URL rules unchanged. The commit→250 leg and the retry budget
       numbers are untouched.
       — *Proved by: `tests/webhook.rs` — the existing happy-path tests now also assert
       `delivered` increments by the exact number of successful POSTs, and the 500-sink
       retry-budget tests assert `exhausted` == the number of budgets spent (no new
       budget behavior); `cargo test --test webhook` green including the https and
       redirect contracts (AC5's ledger arms).*

- [ ] 5. Give each accept loop ownership of its sessions: `smtp::serve` and `pop3::serve`
       spawn sessions into a `JoinSet` and on the shutdown arm
       `abort_all()` + drain before returning. No goodbye write, no session rollback; the
       DATA/commit leg unedited (a commit begun before the abort may land after stop —
       the spec's precise promise).
       — *Proved by: `tests/lifecycle.rs` AC3 tests — an SMTP session held mid-DATA and a
       POP3 session mid-transaction, `stop()` under a real deadline returns, both ports
       refuse new connections, and the store's exact count shows no mail the sessions
       never acked (AC3, AC2's nothing-untracked).*

- [ ] 6. End the feed cooperatively: `AppState` gains
       `shutdown: tokio::sync::watch::Receiver<bool>`; `inbox_feed` clones it into
       `feed_stream`; `feed_events` folds the receiver into its unfold and adds one
       select arm (→ `None`) beside the unchanged `Closed` arm. Snapshot-first is
       unchanged.
       — *Proved by: a new inline `feed.rs` unit test — the watch fires mid-stream and
       the stream ends after the trailing snapshot (the `Closed` test stays green
       alongside it); `tests/ui.rs` and the feed contract tests stay green (spec ruling
       on the feed, AC1's mechanism).*

- [ ] 7. Write the integration shutdown tests in `tests/lifecycle.rs` (real servers, no
       mocks, per 0011 rule 5 — real deadlines, no sleeps): AC1 — open an SSE feed (the
       `common::SseReader` raw-GET pattern), `srv.stop()` under
       `tokio::time::timeout`, assert stop returned and the feed socket reads EOF and
       both ports refuse; AC2 — point a webhook at a stalled sink (accepts, never
       replies) with a delivery in flight and assert `stop()` completes within the
       deadline (red today: the response/dispatcher never completes; red after if the
       dispatcher drained instead of aborting — the stalled budget is ~17 s).
       — *Proved by: the two new tests are green, and each is demonstrably red against
       the pre-change tracker (the deadline fires) — the honest-red check is run once on
       a stashed tree, then reverted (AC1, AC2).*

- [ ] 8. Write the bound/ledger/pruning integration tests in `tests/webhook.rs` (and the
       acceptance posture of `tests/rate.rs`): a delaying sink that counts its own
       concurrent requests — a burst larger than the cap observes max concurrency exactly
       `WEBHOOK_DELIVERY_CAP`, never above (AC4); a forced storm of N mails against one
       held-stalled target reads `swarmail_webhook_dropped_total` == `N −
       WEBHOOK_DELIVERY_CAP` exactly on `/metrics`, with every SMTP send answering `250`
       and the stored count == N (AC5 + AC6); a capturing-then-delaying sink receives a
       byte-identical body for a mail whose inbox was then filled past its configured
       `max_per_inbox` cap (AC7).
       — *Proved by: the three new tests green with exact integer asserts (0010's
       discipline); AC6's test demonstrably red if the bound is temporarily moved onto
       the accept path (a stash-and-probe, then revert) (AC4, AC5, AC6, AC7).*

- [ ] 9. Document the contract in the README Webhooks section: best-effort with
       observable loss, **no durable outbox**, the cap
       (`swarmail::webhook::WEBHOOK_DELIVERY_CAP` = 32), the five metric names with one
       sentence each (dropped = refused at the bound; lagged = firehose skips;
       exhausted = retry budget spent; aborted = ended without an outcome at stop or
       firehose close; delivered = 2xx), and the cumulative-since-start caveat. No other
       README churn.
       — *Proved by: the doc's dropped-row string is byte-identical to the string AC5's
       test asserts (AC8); `git diff main -- README.md` shows only the Webhooks
       paragraph.*

- [ ] 10. Leave every gate green and commit exactly the spec's diff surface: `cargo fmt
       --all -- --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test`;
       `brokkr compile --bundle .`; `bash scripts/coverage-gate.sh` (exact 100/100/100 —
       if an arm is cold, fix the driver per plan.md's map, never the gate); `cargo deny
       check licenses`; `cargo bench --bench ingest` (the recorded criterion must not
       regress) — then a single commit of `src/lib.rs`, `src/webhook.rs`, `src/feed.rs`,
       `src/api.rs`, `src/smtp.rs`, `src/pop3.rs`, `README.md`, and the three suites.
       — *Proved by: all gates exit 0; `git show --stat` lists exactly the eleven files;
       the driver map's every line is covered by the preserved
       `target/coverage/lcov.info` (AC2, AC5, and the coverage posture).*
