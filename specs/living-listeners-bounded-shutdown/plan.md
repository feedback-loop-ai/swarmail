# Plan — living listeners and a bounded shutdown

The implementation approach for
[spec.md](spec.md): five measured `src/` files, one Dockerfile line, zero new files, zero new
dependencies, zero new routes. Ground truth verified at HEAD `cefe392` (positions and this
plan agree on every line number below).

## Technical context

- **The two `?` arms** (`src/smtp.rs:121-128`, `src/pop3.rs:62`) each sit in a loop that owns a
  `JoinSet<session>`, reaps with `try_join_next` at the top of every iteration (0018 §2), and
  selects `accept()` against a pinned `shutdown` future whose arm aborts + drains + returns
  `Ok(())`. The fix is confined to the error arm plus a flag pair; the reap and shutdown arms
  are untouched.
- **The escalation surface exists twice over**: `AppState.shutdown`
  (`src/api.rs:29-38`) is the one watch (0018 §1/0024), `/healthz` is the endpoint the chart's
  three probes read (0026), and `shutdown_signal(rx)` (`src/api.rs:97-101`) is the established
  resolve-at-once-if-stopped primitive.
- **The read chokepoints** are `read_line_capped` (`src/smtp.rs:287`, `src/pop3.rs:104`), built
  on `fill_buf` — cancellation-safe — so a deadline wrapper at the call sites cannot corrupt
  line-buffering state. The bounded-wait precedent is `tls_handshake_timeout`
  (`src/smtp.rs:457`).
- **The test-override precedent** is `tests/smtp_edges.rs:174` (`SmtpConfig { max_message_size:
  64, ..Default::default() }`); `config.rs` constructs with `..SmtpConfig::default()`, so new
  defaulted `SmtpConfig` fields keep `config.rs` at **zero diff** and the CLI byte-identical
  (0022 §5).
- **The seam precedent** is `REAPED` (`src/smtp.rs:82-88`, `src/pop3.rs:34-40`): a
  `#[cfg(test)]` static whose only reader is the inline probe that installs it (0020 rule 4).
- **The watcher** (`src/main.rs:32-36`) spawns before the servers bind and feeds a `watch`
  channel that `main` awaits at `:49` before `server.stop().await`. `tokio`'s `signal` feature
  is already enabled (`Cargo.toml:37`); CI is unix-only, so `cfg(unix)` arms execute in every
  gate run.
- **The drain** is `axum::serve(...).with_graceful_shutdown(shutdown)` (`src/api.rs:269-273`);
  the vendored axum 0.8.9 spawns and detaches one task per connection and exposes no
  per-connection handle, so the deadline's force is *ending the wait* (spec, "the drain"
  ruling).
- **`main.rs` is outside the measured coverage report** (0020's worked proof: 18 `SF:` records
  for 19 `src/*.rs` files); its exits are proven only through `tests/binary.rs`, which is why
  the force-exit arm lives there and is wire-proven.

## Constitution check — frozen guarantees

- **0001 (accept == stored)** — untouched, and *protected* three ways: the idle deadline is
  never armed across the insert (the commit is neither raced nor timed); a mid-DATA expiry
  inherits EOF-means-nothing-stored (nothing is ever stored by a deadline); and a forced
  `exit(70)` is SQLite-crash territory — journal replay, refused-or-committed (0024 §4's
  outcomes survive). The SIGTERM leg re-proves accept==stored across the new path.
- **0002 (await subscribes before it checks)** — no new channel and no new handler: the
  watcher extends the existing registration; `shutdown_signal` (subscribe, then check) remains
  the only stop-read shape; `/healthz` reads the watch statelessly (`borrow()`), which is a
  read, not a wait — no subscription ordering to violate.
- **0003 (oldest-only pruning)** — every new refusal (421, `-ERR too many connections`,
  `452 4.5.3`, expiry closes) precedes any insert or append: nothing was promised, so nothing
  is torn. No pruning semantics change; no eviction enters this batch.
- **0018 (the 10 s fence) / 0024 (one busy wait)** — built on, not re-litigated: the drain
  deadline **is** 0018's 5 s margin given a job; worst-case wall stays ≤ the fence (drain 5 s
  + one busy tail 5 s); 0018 §2 and 0022 carry dated `Amended by 0027` markers via the 0019
  protocol (never rewritten — the amendment lives in 0027).
- **Gates** — every new production line ships with its executing test (§ Coverage); the
  coverage floor stays exact 100/100/100; `deny.toml` untouched (no new dependency).

## Project structure (files touched)

| File | Change |
|---|---|
| `src/smtp.rs` | accept-error arm + degraded flag param; `ACCEPT_BACKOFF` const; idle wrap at every peer read/write call site (command goodbye `421 4.4.2`, DATA silent expiry); session cap check + bounded `421 4.7.0` refusal; `RCPT` cap + `452 4.5.3`; `SmtpConfig` gains `idle_timeout`, `max_sessions` (defaulted consts) |
| `src/pop3.rs` | the same error arm, cap (`-ERR too many connections`), idle wrap (`-ERR idle timeout`); `serve` gains `idle_timeout: Duration`, `max_sessions: usize` |
| `src/api.rs` | `/healthz` reads the flag pair + the stop watch (`ok`/`degraded`/`stopping`, 503 for the latter two); `serve` races the graceful serve against `shutdown_signal(state.shutdown.clone())` + `sleep(DRAIN_DEADLINE)`, returning `Ok(())` at the deadline; `DRAIN_DEADLINE` const documented as 0018's margin |
| `src/lib.rs` | the flag pair (`Arc<AtomicBool>` × 2) created beside the watch, cloned into the three serve spawns and `AppState`; no signature changes |
| `src/main.rs` | the watcher selects SIGINT + `cfg(unix)` SIGTERM → the same watch; a second select in the same task → `std::process::exit(70)`; both signal streams owned for the process lifetime |
| `Dockerfile` | `STOPSIGNAL SIGTERM` |
| `tests/binary.rs`, `tests/lifecycle.rs`, `tests/smtp_edges.rs`, `tests/pop3.rs` | the wire legs (§ Test map) |
| `README.md` | the wire-bound table gains idle/cap/RCPT/exit-code/healthz rows |
| `docs/decisions/0018…`, `docs/decisions/0022…` | dated `Amended by 0027` markers only |
| `docs/decisions/0027-living-listeners-bounded-sessions.md` | the decision (already committed with this spec) |

No change: `config.rs` (zero diff — `..SmtpConfig::default()`), `store.rs`/`persist.rs`
(0001/0003/0024 inputs, not surfaces), `webhook.rs`/`feed.rs` (already try-shaped / watch-racing),
`charts/` (k8s default grace 30 s clears the fence), `Cargo.toml` (no new dependency).

## Mechanism — per file

- **The accept arm** (both loops): `match listener.accept() { Ok(v) => { flag.clear(); …spawn
  as today } Err(e) => { flag.set(); warn!(once per step); select! { _ = &mut shutdown => {…
  existing shutdown arm}, _ = sleep(ACCEPT_BACKOFF) => {} } continue } }`. The refusal and cap
  checks sit after the top-of-loop reap; the cap refusal writes under the shared deadline and
  drops the stream — no session state exists.
- **The bounded-I/O wrapper**: one helper, `async fn bound<T>(dur, fut) -> io::Result<T>`
  mapping `Elapsed` to `io::ErrorKind::TimedOut` in **one** arm; every peer read and reply
  write in both protocols' session paths goes through it; the SMTP command loop maps
  `TimedOut` → goodbye-`421`-then-end, the DATA arm maps `TimedOut` → the exact
  EOF statement (nothing stored), POP3 maps `TimedOut` → `-ERR`-then-end. A timed-out write
  propagates through the existing `?` (silent end — a peer that won't read cannot hear a
  goodbye; no new arm, so no cold branch).
- **The flags**: `smtp_degraded`, `pop3_degraded` — `Arc<AtomicBool>` created in `run_on`
  beside the shutdown watch; `serve` signatures gain one parameter each; `AppState` gains both;
  `/healthz` answers `503 {"status":"degraded","degraded":[…],"uptime_seconds":…}` when any is
  set, `503 {"status":"stopping","uptime_seconds":…}` when the watch has fired (stopping
  wins), else today's `ok` body.
- **The drain**: inside `api::serve`, `let rx = state.shutdown.clone();` *before*
  `router(state)`; `tokio::select! { res = axum::serve(…).with_graceful_shutdown(shutdown) =>
  res, _ = async { shutdown_signal(rx).await; sleep(DRAIN_DEADLINE).await } => Ok(()) }` —
  `shutdown_signal` resolves at once if stop already fired (0018 §1), so the clock starts
  correctly; at the deadline the serve future is dropped and the wait ends.
- **The watcher** (`main.rs`): under `cfg(unix)` build both streams
  (`signal(SignalKind::interrupt())`, `signal(SignalKind::terminate())`) once, owned by the
  task; `select!` their first `recv()` → send the watch (unchanged channel); loop and `select!`
  again → any further signal is `std::process::exit(70)`. Non-unix keeps today's `ctrl_c()`
  first-signal path (CI is unix-only, so every arm is warm).

## Contracts (the wire-visible deltas)

- `/healthz`: `200 {"status":"ok",…}` (unchanged) | `503 {"status":"degraded","degraded":["smtp"|"pop3",…],…}` | `503 {"status":"stopping",…}`.
- SMTP: `421 4.7.0 Too many connections, closing transmission channel` (cap), `421 4.4.2 … idle timeout, closing transmission channel` (expiry), `452 4.5.3 Too many recipients` (RCPT > 100).
- POP3: `-ERR too many connections` (cap), `-ERR idle timeout` (expiry).
- Exit codes: `0` graceful, `70` forced (second signal), `1` startup failure (unchanged).
- Compiled defaults: SMTP idle 5 min (RFC 5321 §4.5.3.2), POP3 idle 10 min (RFC 1939 §3), cap 256/listener, RCPT 100 (RFC 5321 §4.5.3.1.8), accept backoff 1 s, drain 5 s (0018's margin). No knob (0022 §5).

## Coverage — the driver map (exact 100/100/100)

Every new measured line is warm on every run; no `coverage(off)`; no new harness file
(**no cold path exists in this design** — the one unreachable-by-design expiry, a *write*
deadline firing on a peer that buffers small replies anyway, is deliberately *not* a distinct
arm: the shared wrapper owns the single timeout arm).

| New lines | Executed by |
|---|---|
| SMTP error arm + flag set/clear + backoff select | inline seam test: one injected error, real client delivers, flag set→cleared (`src/smtp.rs` `#[cfg(test)]`) |
| POP3 error arm (same) | inline seam test (`src/pop3.rs` `#[cfg(test)]`) |
| Wrapper `Elapsed`→`TimedOut` mapping | every read-side deadline leg (below) |
| SMTP command-path expiry arm (`421 4.4.2`) | in-process leg, tiny deadline, silent client answered then EOF |
| SMTP DATA expiry arm (silent, nothing stored) | in-process leg, `DATA` then stall, store count exact |
| POP3 expiry arm (`-ERR idle timeout`) | in-process leg, `tests/pop3.rs` |
| Cap check + `421`/`-ERR` refusal arms | in-process legs with `max_sessions: 1` (SMTP) / cap param (POP3); first session still delivers |
| RCPT `452 4.5.3` arm | `tests/smtp_edges.rs` wire leg at 101 recipients |
| `/healthz` `degraded`/`stopping`/`ok` arms | direct-handler tests (all four states) + the wire red-proof (persistent seam → real 503) + the stalled-body wire leg |
| Drain deadline arm | `tests/lifecycle.rs` stalled-body leg (real deadline) |
| `main.rs` watcher arms (unmeasured file) | `tests/binary.rs`: SIGTERM→exit 0→restart-queryable; second-signal→exit 70 |

## Test map — where each proof lives

- **Inline, branch-stable (0020 rule 1)**: both accept-error seam tests, the cap-refusal
  ordering, the RCPT refusal ordering — driven in `src/smtp.rs` / `src/pop3.rs` `#[cfg(test)]`
  modules against in-process `serve` calls with overridden bounds.
- **Seams (0020 rule 4)**: one counted one-shot accept-error seam per loop module
  (`#[cfg(test)]`, `pub(crate)` only if a sibling harness needs it — none does), each with its
  installing test; the C36.2 wire red-proof reuses the same seam across a persistent setting.
- **Wire-shaped (0020 rule 3)**: `tests/smtp_edges.rs` (RCPT 101, idle legs, cap leg),
  `tests/pop3.rs` (POP3 idle/cap legs), `tests/lifecycle.rs` (stalled-body `stop()` deadline +
  healthz `stopping` mid-drain), `tests/binary.rs` (SIGTERM graceful + restart-queryable;
  second-signal forced exit 70, sequenced on the binary's own `shutting down` stdout line,
  never a sleep).
- **Verify-seat proof filters**: `cargo test --test binary`, `cargo test --test lifecycle`,
  `cargo test --test smtp_edges`, `cargo test --test pop3`, then the full
  `cargo test --workspace --locked` and the exact gate.

## Risks

- **Dropping the serve future does not close the detached connection tasks** — the deadline
  ends the *wait*, not the sockets; remaining sockets close at the process exit that follows
  (ε after `stop()` in the binary) and at the runtime drop for embedders. *Mitigation:* the
  stalled-body test proves `stop()` returns within a real deadline; the SIGTERM-with-stalled-
  body leg proves the *process* exits; the residual is named in 0027 (the 0024 §6 posture,
  extended to connection tasks). If a future axum exposes per-connection handles, the deadline
  arm upgrades to active close — an ordinary change, red-proven by the same test.
- **The degraded 503 can flap once on a transient blip** — the flag auto-clears on the next
  successful accept; probe thresholds (0026's cadence) smooth one sample. *Mitigation:* named
  residual in 0027; honesty over probe-stability was the ruled trade.
- **The forced `exit(70)` lands mid-commit** — SQLite crash territory: journal/WAL replay,
  refused-or-committed. *Mitigation:* the persistence suite's restart legs already prove
  replay; 0027 records the arithmetic; the SIGTERM leg re-proves durability on the graceful
  path.
- **A `SIGTERM` before the watcher's first poll** kills at the default disposition —
  microseconds, nothing stored (accepted by both positions; the watcher spawns before the
  bind, as today).
- **macos CI's fd budget** shapes *test* values only — every bound is overridden in-process;
  the defaults never appear in a test's critical path (the `max_message_size: 64` precedent).
- **The cap's burst residual** — a burst landing inside one reap window can refuse one honest
  connection, which retries; priced in 0027 (never a lost mail: the refusal precedes any
  insert).
- **The accept-backoff const vs a demonstrated rate** — if the implement seat finds a concrete
  transient rate the fixed 1 s mishandles, the answer is one constant's value, not a backoff
  machine (the burden of demonstration is on the demonstration, per the position's own
  mind-change condition).
- **The stalled-body leg depends on axum's spawned-task shape** (verified in the vendored
  0.8.9 source). If an axum bump changes the drain topology, the same test goes red and names
  the repair — the test is the tripwire either way.
