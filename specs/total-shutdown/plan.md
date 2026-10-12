# Plan — total shutdown: one owner per task, one ledger per death

Companion to [`spec.md`](spec.md) (WHAT/WHY). How the feature lands: the exact mechanism
per file, the files it touches, the coverage driver map that keeps the exact 100/100/100
gate honest, and the risks with mitigations. The panel positions are
[`.forge/design/positions/simplicity.md`](../../../.forge/design/positions/simplicity.md)
and
[`.forge/design/positions/robustness.md`](../../../.forge/design/positions/robustness.md);
the ruling lives in the spec.

## Technical context

Anchors read from the working tree; every claim re-verifiable in seconds.

- `src/lib.rs` — `RunningServer.tasks` is `Vec<tokio::task::JoinHandle<()>>` (`:40`),
  awaited in order by `stop()` (`:45-50`); `webhook::spawn_dispatcher` is called at `:67`,
  **before** the watch channel exists (`:95`) but also before the binds (`:87-93`); the
  three serve tasks are pushed at `:104`, `:126`, `:140`. `run_on` is the only
  construction path (`main.rs:20`).
- `src/webhook.rs` — `spawn_dispatcher(store, webhooks)` at `:199`; `dispatch_loop`
  matches a classify helper over the firehose, warns on `Lagged` (`:159`) and on retry
  exhaustion (`:215`); the payload is built per (mail, target) with
  `json!({..}).to_string()` inside the fan-out loop (`:184-188`) and spawned at `:189`;
  the retry budget is four attempts under a 5 s `POST_TIMEOUT` with 100/400/1600 ms
  backoff. The statics `CA_CACHE_BUILDS`/`CA_CACHE_EVICTIONS` (`:60-61`) are the precedent
  robustness refuses to follow; `delivery_tests`' `CACHE_TESTS` mutex is the price that
  precedent already charges.
- `src/feed.rs` — `feed_events` is an `unfold` with a `warm` snapshot arm and a `recv()`
  match (`Ok | Lagged | Closed`) (`:58-89`); `feed_stream` maps frames onto wire events
  (`:94-100`); called once, from `api.rs:422`.
- `src/api.rs` — `AppState` (`:28-33`) is `Clone` and carries `store`, `chaos`,
  `webhooks`, `started`; `serve` uses `axum::serve(..).with_graceful_shutdown(..)`
  (`:56-58`); `metrics()` builds rows from `state.*` (the `swarmail_webhook_ca_cache_*`
  family at `:214-224` is the shape to extend); `inbox_feed` (`:417-423`) is the only
  `feed_stream` caller.
- `src/smtp.rs` / `src/pop3.rs` — accept loops select on `listener.accept()` vs the pinned
  shutdown future and `tokio::spawn` each session (`smtp.rs:71-99`, `pop3.rs:32-56`);
  DATA writes `250` only after `insert_async(..).await` resolves (`smtp.rs:459-462`) —
  the commit→250 leg this feature must not touch.
- `src/store.rs` — the firehose is `broadcast::channel(8192)` (`:188`); `subscribe_all`
  (`:313`); commits run on the blocking pool (`:541-556`), uncancellable; restore does not
  re-fire watchers or webhooks (`:209-215`).
- `tests/` — `common::SseReader` (`common/mod.rs:423-478`) opens a raw GET and holds both
  halves; `tests/webhook.rs` has real sink helpers (`http_sink(status, expect)` with
  re-arming accepts, `SINK_LIFETIME = 25 s`); `tests/lifecycle.rs` is the graceful-stop
  suite. `common::start_with` overrides `Config` (`max_per_inbox` at `config.rs:111`).
- `scripts/coverage-gate.sh` — exact 100/100/100 (`covered == count` in lines, branches,
  functions; `-` BRDA records refuse; harness-named files under `src/` refuse;
  `coverage(off)` refuses). The gate runs `cargo +nightly llvm-cov --workspace --locked
  --branch`.
- Worktree facts: `commit.gpgsign` resolves **false** (brokkr's config overlay; recent
  history is unsigned) — commit like the rest of history; the harness exports a broken
  `GIT_CONFIG_COUNT=1` without `GIT_CONFIG_KEY_0`, so drive `git` with
  `GIT_CONFIG_COUNT=0` (the gate script's own sidestep). Benches: `benches/ingest.rs`
  holds the ingest criterion (~630k mails/s/core) the verify gate must not regress.

## Constitution check — frozen guarantees

| Guarantee | Verdict |
|---|---|
| 0001 accept == stored | **Upheld.** `Store::insert`, `insert_async` and the DATA commit→250 leg are unedited. The cap and every counter live strictly downstream of `recv()`; the only accept-path edit in the whole diff is zero lines (AC6 is a regression test, not a change). |
| 0002 await subscribes before it checks | **Tightened.** The feed still subscribes before its first scan; the dispatcher's firehose subscription is frozen *before the first listener binds* (the watch creation moves up, the spawn stays ahead of TLS and binds). A consumer created after a bind could miss a webhook for mail accepted in between. |
| 0003 pruning is oldest-only | **Untouched.** The enqueued payload is an owned `Arc<str>`; pruning never touches a delivery in flight (AC7 proves it). |
| 0004 per-test inboxes / no mocks | Upheld: all new tests speak real protocol against real servers on per-test inboxes. |
| 0005 extraction at ingest | Untouched. |
| Coverage floor (exact 100/100/100) | **Not weakened.** Every new line is warm on every run (driver map below); no `coverage(off)`; no new harness file — no new line is cold-path-only. |
| Decision 0010 (proposed, watcher lifetime) | Composes: the feed's `Closed` arm and the new shutdown arm coexist in one select; the dispatcher's subscription ordering is untouched by 0010's release semantics. |
| Decision 0011 (proposed, test placement) | Followed: inline `#[cfg(test)]` stays the default; every new test is branch-stable, so no harness move is demanded. |

## Project structure

No new files, no new dependencies, no new types beyond one small struct. Six existing
production files, one doc, three suites:

```
src/lib.rs        # watch-first ordering; JoinSet tracker; dispatcher handle joins it
src/webhook.rs    # ledger; cap; biased loop; reap; self-reporting deliver; shutdown drain
src/feed.rs       # the shutdown watch folded into feed_events' unfold
src/api.rs        # AppState +1 field; five metric rows
src/smtp.rs       # per-serve session JoinSet; abort + drain on the shutdown arm
src/pop3.rs       # same
README.md         # the Webhooks contract paragraph (AC8)
tests/lifecycle.rs, tests/webhook.rs, tests/rate.rs   # AC1-AC7
```

## Mechanism — per file

1. **`src/lib.rs`** — create the watch channel where the store is born (before TLS, before
   any bind); call `webhook::spawn_dispatcher(store.clone(), webhooks.clone(),
   shutdown_rx.clone())` immediately after; push its returned handle into the tracker.
   `tasks: Vec<JoinHandle<()>>` becomes `tokio::task::JoinSet<()>`; `stop()` sends the
   signal then `while self.tasks.join_next().await.is_some() {}`. The three serve spawns
   are unchanged except that their shutdown futures now come from the earlier channel.
2. **`src/webhook.rs`** —
   - `pub const WEBHOOK_DELIVERY_CAP: usize = 32;` (documented; the `CA_CLIENT_CAP`
     precedent for a plain documented const).
   - `Ledger` (default-zeroed): `delivered`, `dropped`, `lagged`, `exhausted`, `aborted`
     as `AtomicU64`, with a Copy `snapshot()` accessor in the `CaCacheStats` shape. Lives
     on `Webhooks` (which already derives `Default`); `AppState` reaches it through the
     existing `Arc<Webhooks>`.
   - `dispatch_loop(rx, webhooks, shutdown_rx)`: a **biased** `select!` — firehose first,
     shutdown second — so a buffered mail is always dispatched or dropped (counted)
     before the loop can end; the mail arm harvests finished deliveries
     (`while deliveries.try_join_next().is_some() {}`), builds the payload **once** per
     mail (`Arc<str>`), then for each matching target either spawns into the set
     (`deliveries.len() < WEBHOOK_DELIVERY_CAP`) or increments `dropped` and skips. Both
     the shutdown arm and the firehose-`Closed` arm fall through to one tail:
     `deliveries.abort_all(); while let Some(res) = deliveries.join_next().await { if
     res.is_err() { ledger.aborted … } }` — an `Err` is a delivery that ended without
     reporting (cancelled at stop, or orphaned by firehose close); an `Ok` already
     reported its own outcome.
   - `deliver` gains the `Arc<Webhooks>` and **self-reports as its final statements**: a
     2xx increments `delivered`; the budget's end increments `exhausted`. Both increments
     follow the function's last await, so a task that reported can never also be counted
     aborted (cancellation takes effect only at a yield point), and a task counted
     aborted never reported. The lag arm moves into the ledger: the classify helper
     returns the lagged count and the loop adds it (`Lagged(n)`).
   - `spawn_dispatcher(store, webhooks, shutdown) -> tokio::task::JoinHandle<()>`.
3. **`src/feed.rs`** — `feed_events` and `feed_stream` take a
   `tokio::sync::watch::Receiver<bool>`; the unfold's post-snapshot poll selects between
   the watcher `recv()` and `shutdown.changed()` (→ `None`). The `Closed` arm is
   unchanged. Snapshot-first is unchanged: one trailing frame at stop, then the response
   completes.
4. **`src/api.rs`** — `AppState` gains `shutdown: tokio::sync::watch::Receiver<bool>`
   (cloned per feed connection — `watch` receivers are cheap). `inbox_feed` passes it to
   `feed_stream`. `metrics()` gains the five `swarmail_webhook_{delivered,dropped,lagged,
   exhausted,aborted}_total` rows from `state.webhooks`' snapshot.
5. **`src/smtp.rs` / `src/pop3.rs`** — each serve owns `let mut sessions =
   JoinSet::new()`, spawns sessions into it, and on the shutdown arm does
   `sessions.abort_all(); while sessions.join_next().await.is_some() {}` before
   `return Ok(())`. No goodbye is written; the commit→250 leg is untouched.

## Coverage — the driver map (exact 100/100/100)

Every new line/branch must execute on every `cargo test` run. Named drivers:

| Line / branch | Warm driver |
|---|---|
| reap `try_join_next().is_some()` = true | inline unit test: mail 1 to a fast + a stalled sink, spin-yield on `ledger.delivered == 1` (a production observable — report ⇒ past the last await ⇒ uncancellable), then mail 2: the second pass reaps the finished delivery |
| reap = false (nothing finished) | every pass in every suite |
| cap `len() < CAP` = true | every spawn (all happy-path webhook tests) |
| cap = false → `dropped`++ | the forced-storm cap test (AC4/AC5/AC6) |
| drain `Ok` arm (already reported) | the same inline unit test: the fast delivery finished before the watch fired |
| drain `Err` arm → `aborted`++ | the same inline unit test (stalled delivery cancelled at its POST await) and AC2's stalled-sink stop |
| `delivered`++ | every happy-path webhook test |
| `exhausted`++ | the existing 500-sink retry-budget tests |
| `lagged`++ | the updated classify/dispatch-loop unit test (ring capacity 1) |
| feed shutdown arm → `None` | a new inline feed unit test (watch fires mid-stream) and AC1's lifecycle test |
| session drain body (`join_next` = Some) | AC3's mid-transaction-session lifecycle tests |
| `stop()`'s `join_next` body | every lifecycle test |
| the five metric rows | the metrics assertions of AC4/AC5/AC6 |

The yield-spin is not sleep-as-sync: it waits on a production counter, no deadline, and
terminates exactly when the fact it needs is true. If any arm proves unwritable warm
(e.g. an abort-during-await residue), it moves to `src/<module>/<name>_tests.rs` declared
`#[cfg(test)]` beside its owner per 0011 rule 2 — but the design intends none.

## Risks

| Risk | Mitigation |
|---|---|
| A cold arm refuses the exact gate (the usual way 100/100/100 lies) | The driver map above is part of the change; each driver is deterministic by construction (biased select, no-await-after-report, production-observable spin). If a driver still races, fix the driver — never `coverage(off)`, never a harness landfill. |
| The biased select delays shutdown under a sustained storm | Bounded by the firehose backlog (≤8192 mails, microseconds each to dispatch-or-drop); AC2's deadline still holds. Named cost of the alternative (shutdown-biased select): a silently skipped mail at stop, uncounted — refused. |
| Mail lands after `stop()` returns (fence refused) | The precise promise in the spec; a duplicate on retry, never a loss; test discipline — exact post-stop counts only on inboxes the aborted session never touched. |
| A stalled target holds bound slots ~17 s worst case (`POST_TIMEOUT` + budget), starving other targets | Accepted, named in the spec; per-URL fairness maps are refused (an unbounded map keyed by attacker-chosen URLs). |
| `aborted` over-counts a delivery that dies by panic (no report, `Err` join) | Documented: the row means "ended without reporting its outcome"; `deliver` has no panic path (every fallible operation returns `Result`), so the case is a bug signal, not a counted class. |
| The ledger's cumulative-since-start semantics mislead an operator across restarts | AC8's doc text says so explicitly; the CA-cache counters set the same precedent. |
| A frame races the closing feed socket | Harmless by construction: the client's `EventSource` reconnect re-subscribes before its first scan (0002) and re-snapshots (0010 §5). |
| 0010 acceptance later changes the firehose/watcher release semantics | The dispatcher and the feed both speak the watch vocabulary 0010's machinery already uses; its proposed release policy touches the per-inbox ring, not `global_tx`. Composition is a sentence in 0010, not a code fork here. |
| Ingest bench regresses (the bound sits on the post-insert fan-out) | Nothing is added to `insert`; the dispatcher's per-mail work grows by one harvest check and one `len()` compare. `cargo bench` gates the change. |
| GPG/signing divergence | Commit unsigned like the rest of this worktree's history (`commit.gpgsign` resolves false under brokkr's overlay); never push. |

## Quickstart (reviewer)

```bash
cargo test --test lifecycle            # AC1, AC2, AC3
cargo test --test webhook              # AC4, AC5, AC6, AC7 (+ the inline unit drivers)
cargo test                             # everything, real servers
bash scripts/coverage-gate.sh          # exact 100/100/100 — the driver map holds
cargo bench --bench ingest             # the recorded criterion must not regress
brokkr compile --bundle . && cargo deny check licenses
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings
curl -s localhost:8025/metrics | grep swarmail_webhook   # the five rows (AC8's strings)
```
