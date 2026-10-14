# Plan — the queued-write shutdown bound

HOW for [spec.md](spec.md). Ordering: the docs pass that frees the
decision number first, then the boundary change bottom-up (field → check
→ wiring → unit probe → wire legs), then the lifecycle documentation, then
the ride-along comments, then the gate sweep. Every step carries its
proving test ([tasks.md](tasks.md)). `src/persist.rs` is never opened;
no adapter mapping site changes behavior.

## Technical context

- The boundary: `src/store.rs` — `mutation: std::sync::Mutex<()>` (the
  one ordered boundary, 0014); `record()` (`:258`) is the single commit
  chokepoint called by all four mutations (`:393`, `:519`, `:567`,
  `:587`); `offload` (`:612`) runs each whole mutation on
  `spawn_blocking`; the `_async` wrappers (`:632-652`) call it.
- The shutdown wiring: `src/lib.rs` — the store is built at `:63-69`, the
  watch is born at `:79`, `stop()` sends the watch and drains the JoinSet
  (`:51-55`); `src/main.rs` — SIGINT → `ctrl_c()` → `server.stop()`.
  Every stop path already flows through one watch and one `stop()`.
- The consumers: `src/api.rs` — `with_graceful_shutdown` (`:84-86`), the
  seed handler maps any `Err` to the fixed `503` (`:566-571`),
  `shutdown_signal` (`:73-77`) is the 0018 wait primitive (borrowed as
  the model for the check's discipline).
- The bound: `src/persist.rs` — `sqlite3_busy_timeout(db, 5000)` is the
  dependency default, pinned by the fact tripwire (`:570-599`); 0015 §2
  and 0016 §4 make it part of the refusal contract. **Zero diff this
  run.**
- The toolchain fact the design hangs on: tokio 1.53.2's runtime drop
  waits forever for detached blocking tasks — an adapter-level `select!`
  moves the burn to process exit instead of removing it (0024 Context).

## Constitution check

- **0001 (accept == stored)**: preserved — the refusal precedes any
  acceptance; nothing acknowledged is uncommitted.
- **0002 (subscribe before check)**: reused — the check reads the watch
  with the same `borrow_and_update` discipline; the wiring happens before
  any listener binds.
- **0003 (oldest-only pruning)**: preserved — a refused insert evicts
  nothing (the apply phase never runs).
- **0012 §1/§2**: preserved — one public variant (`rusqlite::Error`), no
  new error type, the fixed refusal text on every surface, the detail in
  the log only.
- **0014 §1/§2/§3**: preserved — the boundary's span, lock order and
  no-await rule untouched; the check adds no serialization layer and
  reorders nothing; a refusal applies nothing.
- **0015 §2 / 0016 §4**: preserved — the busy bound stays the dependency
  default; the tripwire stays green; no override anywhere.
- **0018**: amended *through* 0024 (the 0019 pattern — a dated
  `Amended by` marker on the accepted file, the amendment carried by the
  proposed one). Never an in-place rewrite of the accepted text.
- **0011 rule 1/4/5**: the new lines are deterministic branches, so they
  stay inline and measured; no `coverage(off)`; no wall-clock validity
  assertion (the budget asserts are upper bounds on real elapsed exit
  time, the same class as the existing `STOP_BUDGET` legs); no shared
  global state (the flag lives on the store instance).
- **Modular monolith / dependency policy**: no new `src/` file, no new
  dependency, no bundle edit; `deny.toml` untouched.

## Project structure (files touched)

| File | Change |
|---|---|
| `src/store.rs` | `stopping` field + `observe_shutdown` setter + the `record()` check + inline probe |
| `src/lib.rs` | one wiring line + comment in `run_on` |
| `src/api.rs` | one stale comment corrected (seed-handler refusal arm) |
| `src/pop3.rs` | QUIT deviation documented at the handler (comment) |
| `src/smtp.rs` | — none (the 0018 §2 SMTP posture is already true) |
| `tests/lifecycle.rs` | the `stop()` leg |
| `tests/binary.rs` | the SIGINT process leg + restart copy |
| `docs/decisions/` | four `git mv`s, four provenance notes, 0018 marker + dates, 0024 (committed by design), README index |
| `docs/house-rules.md`, `docs/decisions/*.md`, `specs/*`, `src/*` comments, `tests/*` comments, `benches/` | the citation sweep |
| `README.md`, `src/static/llms.txt`, `src/static/openapi.json` | lifecycle/budget wording aligned |

## The change, per file

### 1. `src/store.rs` — the field, the setter, the check

```rust
/// One source of "stopping" (decision 0024): the receiver half of the
/// server's own shutdown watch, wired once by `run_on` before any
/// listener binds. A store never wired keeps today's behavior.
stopping: std::sync::OnceLock<tokio::sync::watch::Receiver<bool>>,
```

- `pub(crate) fn observe_shutdown(&self, rx: watch::Receiver<bool>)` —
  `let _ = self.stopping.set(rx);` (one-shot; documented as such).
- In `record()`, first thing:
  ```rust
  if let Some(stopping) = self.stopping.get()
      && *stopping.borrow_and_update()
  {
      tracing::info!(what, "refused: shutting down; nothing applied");
      return Err(rusqlite::Error::SqliteFailure(
          rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ABORT),
          Some("shutting down".into()),
      ));
  }
  ```
  (let-chains are the house style — `record()`'s own persist arm uses
  one). `SQLITE_ABORT` is "aborted prior to completion" — truthful for a
  commit never started; the payload is in-crate only (every mapper is
  `Err(_)`); the info line is the one witness. The check sits inside the
  boundary but does no I/O and awaits nothing (0014 §1); the refusal
  applies nothing (0014 §3's list) — for `insert` the plan has only
  peeked; for `delete`/`clear`/`clear_all` the check precedes their
  commit exactly as their persist errors do.
- Inline probe (the existing `#[cfg(test)]` module): a `Store::new`,
  wired; `insert` → `Err` with the constructed code, then assert nothing
  applied (`count == 0`, no inbox minted, `watcher_count` unmoved,
  counters unmoved); wire a second store, don't stop, `insert` → `Ok`
  (the false arm); the unwired arm is the existing suite, unchanged.

### 2. `src/lib.rs` — the wiring

One line after the watch is born (`:79`) and before the dispatcher/listeners:
`store.observe_shutdown(shutdown_rx.clone());` with a two-line comment
naming 0024 and the bind-order rule. `stop()` itself is untouched — the
flag travels on the watch it already sends.

### 3. `tests/lifecycle.rs` — the `stop()` leg

`queued_mutations_do_not_burn_one_busy_timeout_each`: `start_with` a real
data file; `common::hold_write_lock(&path)`; three
`common::http_json` POSTs to `/api/v1/inboxes/{s1,s2,s3}/seed` spawned
concurrently; 300ms sequencing pause (all three parked on the boundary;
the burn is 5s — the pause is provably inside it, the R11 discipline);
`tokio::time::timeout(STOP_BUDGET, srv.stop())` with the elapsed asserted
`< STOP_BUDGET` (upper bound only); all three requests answered
`503 {"error":"storage commit failed"}`; then a fresh `rusqlite`
connection on the file asserts zero `emails` rows (the refused set stored
nothing).

### 4. `tests/binary.rs` — the process leg

`sigint_exits_within_the_budget_with_a_held_write_lock`: spawn the real
binary on a disposable data file (unique per run, like the restart test);
`wait_http_up`; `common::hold_write_lock` on the same path **from the
test**; three concurrent seed POSTs; 300ms pause; `libc::kill(SIGINT)`;
assert exit status success and elapsed `< 10s` (upper bound, measured
from the kill); collect the three `503`s with a generous per-request
timeout (they are written during the graceful drain, before exit); drop
the lock, reopen the file, assert the restart copy holds exactly the
committed set — empty here — never a partial row. Sequence-independent:
the assertions name no arrival order.

### 5. The citation sweep and the index (slice 5)

- `git mv` the four movers to 0020–0023 (landing order, spec R3); add the
  one-line provenance note under each moved file's header
  (`> Renumbered from 0011 → 0020 in <run> — earliest-landed file keeps
  the number; landing order = committer date.`).
- Repoint every citation by subject. The grep surface: `decision 0010`,
  `decision 0011`, `0010 (`, `0011 (`, `0011 rule`, `0010-`, `0011-`
  across `src/`, `tests/`, `benches/`, `docs/`, `specs/`, `README.md`,
  `src/static/`. Classification: watcher channel lifetime/release → 0021;
  off-worker commits/blocking pool → 0023; exact counts/shared seed →
  0010 (stays); concurrency tests, the harness, `rule N` → 0020; the
  loud shim DELETE → 0011 (stays); bounded DATA/raw bytes → 0022. A
  citation whose sentence names two subjects gets both.
- `docs/decisions/README.md`: renumber the moved rows, add 0020–0024.
  Status texts stay verbatim; the observed 0018 `proposed`-vs-`accepted`
  drift is left for the operator (0024 §Consequences names it).
- 0018: the `Amended by` marker gains 0024; Status/Date lines read
  2026-10-13. 0012/0014/0015/0016's status lines untouched.
- `docs/house-rules.md`: the regression-test-placement section cites 0020.

### 6. Lifecycle documentation (slice 1's docs + slice 5's dates)

0018 §2's mutation mechanics and the README lifecycle row state the
enforced policy in the budget's own words; `llms.txt` and `openapi.json`
descriptions that mention the seed 503 or the stop budget are checked and
aligned (they currently describe the refusal, which does not change — the
budget sentence gains the queued-mutation bound).

### 7. The ride-along comments (slice 6 + one found while grounding)

- `src/store.rs` `delete`: name the deliberate 404-commit cost and the
  guard it buys (0024-adjacent, comment-only).
- `src/pop3.rs` QUIT arm: state the TRANSACTION-retention as the
  deliberate deviation (protocol CLOSE, Mailpit keeps the session, the
  client can retry or `RSET` on the same connection).
- `src/store.rs` insert fan-out: the whole truth about the shard guard
  (held across the sends *because* they are non-blocking; "outside the
  boundary tail" refers to the commit).
- `src/api.rs` seed arm: replace the dead "memory truth stands" line with
  0014 §3's contract and the 0024 refusal-family note.

## Quickstart

```bash
cargo test --test lifecycle  queued_mutations          # the stop() leg
cargo test --test binary     sigint_exits_within       # the process leg
cargo test -p swarmail --lib store::tests::refuses      # the inline probe
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings \
  && cargo test --workspace --locked \
  && brokkr compile --bundle bundles/self \
  && bash scripts/coverage-gate.sh && cargo deny check licenses
```

The legs need no fixture beyond `common::hold_write_lock` (a real second
connection's `BEGIN IMMEDIATE`) — the same real layer the issue's
reproduction uses, no mocks, no shims.

## Risks

1. **The timing legs flake on a slow runner.** Mitigation: the bound
   asserted is the contract's 10s against a 5s burn — 2× headroom on any
   realistic runner; the sequencing pause (300ms) is provably inside the
   5s burn, so the stop always lands while a request is parked; asserts
   are upper bounds only and sequence-independent. If a flake ever
   appears, the renegotiation is with the *operator* over the ADR bound
   (a contract), never the test's honesty (no sleeps, no lower bounds).
2. **A commit passes the check, then the flag flips before its write.**
   Accepted residual, priced in 0024 §3: the boundary serializes, so at
   most one such commit exists per stop — it burns at most one busy bound
   and lands or refuses atomically. The budget absorbs it; no code
   defends against it.
3. **The SIGINT leg meets the aborted-session residual.** A detached
   blocking task from an aborted SMTP session can burn one busy bound at
   runtime drop — the 0018 §2 posture, inside the same budget. The leg's
   fixture holds the lock from before the seeds, so its own copy has no
   committed set to race; the exit-time bound is asserted over the worst
   case, not the happy path.
4. **The citation sweep misclassifies a `0010`/`0011`.** Mitigation: the
   rule is subject-first, never blind rename (`sed s/0011/0020/` would
   corrupt the shim-delete citations); every moved-file link is updated
   in the same commit as the rename so no relative link dangles; the
   historical `specs/` records are included because their links would
   otherwise point at the wrong decision.
5. **The check refuses a mutation the operator expected to succeed.**
   The flag is set only by `stop()`/SIGINT — the server is leaving;
   0024 §1 documents that a memory-only store refuses too (the policy is
   the boundary's, not the mirror's). The observable window is dying
   connections; nothing internal mutates during stop (the webhook
   dispatcher writes only its own ledger).
6. **Coverage floor.** The check adds one branch with three arms; each
   arm is executed by a named test (unwired — the existing suite; wired
   not stopping — the probe's false arm; stopping — the probe's true
   arm). No harness relocation, no exclusion: the floor may only rise.
7. **The renumbering races nothing.** The movers take 0020–0023 and the
   design commit already pins 0024, so no later run can re-collide the
   numbers mid-run; the provenance notes make the move self-describing
   for any reader who meets the file without its index.
