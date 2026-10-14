# Implementation Plan: Commits off async workers (`off-worker-commits`)

Technical context → constitution check → design → project structure → contracts → test
architecture → measurement → quickstart → **Risks** → rejected alternatives. Companion documents:
[spec.md](spec.md) (WHAT and WHY, acceptance criteria) and
[docs/decisions/0023-off-worker-commits.md](../../docs/decisions/0023-off-worker-commits.md)
(status `proposed`).

## Technical context

- Single Rust crate, edition 2024. Tokio `1` with `rt-multi-thread` + `macros`
  (`Cargo.toml:37`) — `tokio::task::spawn_blocking` ships in the `rt` feature, which
  `rt-multi-thread` implies. **No new dependency.**
- Persistence is `src/persist.rs`: one `rusqlite::Connection` behind a `std::sync::Mutex<State>`,
  WAL (`src/persist.rs:83`) + `PRAGMA synchronous = FULL` (`src/persist.rs:84`), per-mutation
  transactions (`record_insert` `src/persist.rs:112`; `record_delete(s)` beside it). The mutex is
  held for one small transaction and never across an `await` — it is already the writer queue.
  `fail_writes()` (`src/persist.rs:190`) is `#[cfg(test)] pub(crate)`.
- `Store` (`src/store.rs`): DashMap-backed; four sync mutation methods — `insert` (`:229`),
  `delete` (`:310`), `clear` (`:327`), `clear_all` (`:347`) — each ending in `self.record(...)`
  (`:251/:322/:342/:359`); the `#[cfg(test)]` pre-mirror gate (`set_pre_mirror_gate` `:200`) and
  the restore path that deliberately bypasses `insert` (`:159-162`). `Store: Send + Sync` is
  proven by the tree itself: `Arc<Store>` is moved into `tokio::spawn`ed futures today
  (`src/smtp.rs:82-95`).
- The 12 production call sites, all inside `async fn`s and all holding `Arc<Store>`:
  `src/smtp.rs:455` (insert); `src/api.rs:349` (clear), `:367` (delete), `:459` (insert);
  `src/mcp.rs:126` (delete), `:134` (clear), `:136` (clear_all), `:185` (insert);
  `src/pop3.rs:357` (delete); `src/compat.rs:254` (clear_all), `:268` (delete batch),
  `:269` (clear_all). `src/feed.rs` and `src/threads.rs` hits are `#[cfg(test)]` — not callers.
  The restore path bypasses `insert` by design.
- `benches/ingest.rs` measures the sync memory-only store and stays untouched. `criterion 0.7`
  and `libc 0.2` are already dev-dependencies. `benches/` is outside the coverage report (the
  gate runs `llvm-cov` without `--all-targets`).

## Constitution check

| Guarantee / rule | Effect of this change | Verdict |
|---|---|---|
| 0001 — accept == stored | Strengthened in placement, identical in behaviour: memory insert + prune + mirror commit all complete **before** the `250`; failure observed before the reply byte; `synchronous = FULL` untouched. Interpretation ruling recorded in decision 0023 (`proposed`). | Compliant |
| 0002 — await subscribes before checking | No change: watchers are subscribed by `await` callers independently; the awaited mirror does not reorder subscribe-before-check. | Compliant |
| 0003 — oldest-only pruning | No change: the prune stays inside the moved mutation, under the same DashMap entry lock, mirrored per-pruned-id in the same transaction. | Compliant |
| 0005 — extraction at ingest | No change: `build_email` stays on the worker at `src/smtp.rs:452-453`. | Compliant |
| Exact 100/100/100 coverage | Every new production line is executed by tests; the `offload` `Err` arm is covered deterministically by the panic-symmetry pin; no `coverage(off)`. | Compliant (by test set) |
| Dependency policy | No new dependency; the tree is byte-identical for `cargo deny`. | Compliant |
| Gates | All six run at verify; the bench and new tests must be clippy-clean under `--all-targets`. | Compliant (by task list) |

## Design

One awaited seam, four wrappers, twelve switched call sites, one doc correction.

### The seam (`src/store.rs`, private)

```rust
/// Runs a whole mutation on tokio's blocking pool. The returned future
/// resolves only after the mutation has run to completion — memory mutated,
/// prune applied, mirror commit attempted — so a `250` written after it still
/// means queryable AND durable (decision 0001). A panicking mutation
/// re-raises in the awaiting caller, exactly as the sync call would.
async fn offload<T: Send + 'static>(task: impl FnOnce() -> T + Send + 'static) -> T {
    match tokio::task::spawn_blocking(task).await {
        Ok(value) => value,
        Err(join_error) => std::panic::resume_unwind(join_error.into_panic()),
    }
}
```

The explicit `match` + `resume_unwind` form is required, not stylistic: the exact 100/100/100
gate counts branches, and an `.await.expect(...)` shape mints an `Err` branch no happy-path suite
executes. The `match` is deterministic — both arms are covered by the test set (Ok by every
real-server suite, Err by the panic-symmetry pin) — *and* it pins the real property: panic
symmetry with the sync call.

### The wrappers (`src/store.rs`, public, additive)

```rust
pub async fn insert_async(self: Arc<Self>, email: Email) -> Arc<Email> {
    offload(move || self.insert(email))
}
pub async fn delete_async(self: Arc<Self>, id: String) -> bool {
    offload(move || self.delete(&id))
}
pub async fn clear_async(self: Arc<Self>, inbox: String) -> usize {
    offload(move || self.clear(&inbox))
}
pub async fn clear_all_async(self: Arc<Self>) -> usize {
    offload(move || self.clear_all())
}
```

The **whole** mutation moves — memory half, prune, pre-mirror gate window, mirror commit,
fan-out. Splitting memory-half from commit-half would drag the drain→gate→mirror seam across a
function boundary to save nanoseconds and re-engineer exactly the window the wave-2 clear-race
tests pin. The sync methods remain the single owner of mutation logic; their unit tests, the
bench, and the gate closures compile unchanged.

### The 12 call sites

Eleven are one-line switches (`store.clone().insert_async(email).await;`). One is a three-line
restructure: `src/compat.rs:268`'s
`ids.iter().filter(|id| state.store.delete(id)).count()` becomes a counted loop of awaited
`delete_async` calls — same semantics (each delete is individually atomic and mirrors its own id;
the batch was never one transaction). Do **not** add a `delete_many` method: a loop reusing the
seam is smaller than new method vocabulary.

### The two lines that must not move

1. `src/smtp.rs:456` — the `250 2.0.0 OK: stored as {id}` write stays immediately **after** the
   awaited mutation. That ordering *is* 0001's disk leg.
2. `src/persist.rs:84` — `PRAGMA synchronous = FULL` stays. The pragma is the disk leg.

### The doc correction

`src/persist.rs:12-13` claims the mutex "cannot stall the runtime" — true of the lock, false of
the commit inside it. Correct the sentence to: the commit runs on a blocking thread when called
from an async handler (via the store's awaited wrappers), so it never occupies a tokio worker.
Mirror the same phrase in `insert`'s doc comment.

## Project structure

| File | Change |
|---|---|
| `src/store.rs` | + the private `offload` helper, + four `*_async` wrappers (~20 production lines); + three in-crate test pins (ordering, panic symmetry, real-protocol propagation) in the existing `#[cfg(test)]` module |
| `src/smtp.rs` | 1 call-site switch (`:455`); `:456` untouched |
| `src/api.rs` | 3 call-site switches (`:349`, `:367`, `:459`) |
| `src/mcp.rs` | 4 call-site switches (`:126`, `:134`, `:136`, `:185`) |
| `src/pop3.rs` | 1 call-site switch (`:357`) |
| `src/compat.rs` | 3 call sites (`:254`, `:269` one-liners; `:268` counted loop) |
| `src/persist.rs` | doc sentence correction only — **zero code changes** |
| `tests/binary.rs` | + the hard-kill leg (A1): burst → record 250s → `SIGKILL` → respawn → assert |
| `tests/` (write-heavy leg) | + real-server concurrency leg with `--data-file` (correctness, no wall-clock asserts) — in `tests/persistence.rs` or a sibling suite at the implementer's discretion |
| `benches/writer_stall.rs` + `Cargo.toml` `[[bench]]` | new measurement target, `harness = false` |
| docs note or PR body | the recorded before/after numbers (verbatim) |
| `docs/decisions/0023-off-worker-commits.md` | the proposed decision (+ its one-line index row in `docs/decisions/README.md`, per the 0009 precedent) |

No new `src/` files, no new dependency, no flag, no mode branch, no config surface.

## Contracts

- **Wrapper contract**: each `*_async` future resolves strictly after the whole mutation has run
  to completion on a blocking-pool thread. Panics propagate to the awaiting caller (panic
  symmetry). Cancellation of the caller does not abort an in-flight commit (tokio blocking tasks
  are not cancelled) — a dropped future leaves the mutation completing or failing on its own
  thread, which is SIGKILL-equivalent for the disk mirror and harmless for memory (the store is
  shared).
- **Caller contract**: reply bytes are written only after `.await`. `src/smtp.rs:456` is the
  load-bearing instance; the other 11 sites have no wire-visible reply after the mutation except
  the JSON/POP3/MCP responses, which follow the same rule.
- **Failure contract (unchanged)**: a mirror error is logged at commit time inside the mutation
  (`record()`, `src/store.rs:186-196`); memory stays authoritative; the `250` (or JSON/POP3 OK)
  is still sent; the restart copy degrades exactly as wave 2 documented. A new failure protocol
  (4xx, `Result`-returning insert) is explicitly not built.

## Test architecture

Three in-crate pins (in `src/store.rs`'s `#[cfg(test)]` module — the only place
`Persist::fail_writes` and the private `persist` field are reachable, so no production injection
surface is added), one integration leg, one hard-kill leg, one bench, plus the existing suites.

1. **Ordering pin** (wave-2 propagation, deterministic): current-thread `#[tokio::test]`; install
   a pre-mirror gate that waits on a release flag; with `fail_writes()` set, spawn
   `insert_async`/`clear_async`; assert the future has **not** resolved while the gate holds
   (red against fire-and-forget and reply-before-commit); release; assert resolution and that
   memory is truthful (mail present / count correct); reopen the data file and assert the row is
   absent (the failure was real and observed; the degraded restart copy is the documented
   semantic).
2. **Structural stall pin** (criterion 1, red in every wrong direction): current-thread
   `#[tokio::test]`; gate holds the mutation on a blocking-pool thread; the test task completes
   deadline-bounded 1 ms sleeps while held and asserts the mutation did not resolve while held;
   a **std-thread watchdog releases the gate after ~2 s unconditionally**, guaranteeing
   termination. Assertions after the dust settles: (a) sleeps completed on time while held — red
   (wedged runtime thread) if the mutation ran inline on the runtime thread; (b) not resolved
   while held — red against fire-and-forget; (c) resolved after release. `block_in_place` is red
   by construction (it panics on current-thread runtimes). *Why a watchdog and not
   `tokio::time::timeout`:* tokio timers cannot fire while the runtime thread is wedged inside
   the gate; the watchdog converts what would be a hang into a fast deterministic red.
   `#[tokio::test]`'s current-thread runtime is sufficient *because* the seam is
   `spawn_blocking` — one more reason `block_in_place` was cut.
3. **Panic-symmetry pin** (`#[should_panic]`, in-crate): a panicking gate closure through
   `clear_async` re-raises in the awaiting caller. Doubles as the deterministic cover of the
   `offload` helper's `Err` arm at the exact branch gate.
4. **Real-protocol propagation leg** (in-crate, criterion 2): a real `TcpListener` +
   `crate::smtp::serve` + a store opened on a temp data file with `fail_writes()` set + a real
   `TcpStream` speaking SMTP: the client still receives `250` (log-and-accept preserved), the
   mail is queryable over the wire, and a reopen loses the row.
5. **Hard-kill leg** (`tests/binary.rs` harness, A1): spawn the real binary with `--data-file`
   (temp dir), open SMTP sessions, burst mail recording **every id observed on a `250` reply**
   (the id is wire-visible in `250 2.0.0 OK: stored as {id}`), `libc::kill(SIGKILL)` the child
   mid-burst, respawn on the same file, then assert every recorded id is in the restart copy
   (via the HTTP API). No timing margins: the kill point is wherever it lands; only observed
   250s are asserted. Red against commit-after-`250`, write-behind, and softened pragma; green
   against old and new code.
6. **Real-server write-heavy leg** (criterion 1's protocol half): a server with `--data-file`,
   more concurrent pipelined writer sessions than runtime workers (reuse the
   `tests/persistence.rs:216` burst pattern), concurrent liveness `NOOP`s on a separate
   connection, then a full count-and-restart correctness check of everything answered `250`.
   Asserts **correctness under concurrency, never wall-clock margins** (hardware belongs in the
   measurement).
7. **Existing suites unchanged in intent**: `tests/persistence.rs` (graceful restarts + the
   wave-2 race legs), `burst`, `rate` (exact 5000/5000), `p2`, api/mcp/pop3/compat — which now
   also execute every wrapper line, keeping the new surface at the exact gate with no new test
   burden beyond the pins above.

## Measurement contract

`benches/writer_stall.rs` (`harness = false`, tokio + criterion already present), a new
`[[bench]]` entry in `Cargo.toml`:

- One process, one real data file in a temp dir. A **current-thread runtime** for the stall
  discriminator, documented: on a multi-thread runtime the old path blocks one worker among
  many, so a ticker only stalls when it lands on that worker — the nondeterminism the ruling
  refuses.
- A ticker task records `max`/`p99` overshoot of 1 ms ticks. Phase A (the finding, isolated): a
  write-heavy loop of sync `insert`s with size-amplified rows (multi-KB/MiB `raw` — the commit
  cost scales with the row) running inline on the runtime thread. Phase B: the same loop through
  awaited `insert_async`. Same file, same row sizes, both phases' numbers printed.
- The real-server probe's observed numbers (from leg 6's run log) are recorded alongside.
- **Honesty clause**: numbers are reported verbatim in a docs note or the PR body — no rounding,
  no batching, no pragma/WAL tuning to prettify. If the win is not clear (fast CI disks make
  WAL+FULL commits tens of µs), that is stated and the operator rules; the quality defect stands
  regardless (workers must not execute disk waits, and `persist.rs:12-13`'s claim is false
  today).

## Quickstart

```bash
cargo test --test persistence --test binary      # durability, propagation, hard-kill legs
cargo test                                       # full suite, real servers
cargo bench --bench writer_stall                 # the measurement (or: cargo run --bench writer_stall)
bash scripts/coverage-gate.sh                    # exact 100/100/100 (missed == 0)
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings
brokkr compile --bundle . && cargo deny check licenses
```

## Risks

1. **The exact-100/100/100 branch gate refuses an uncovered `Err` arm.**
   *Mitigation*: the `match` + `resume_unwind` helper shape and the `#[should_panic]`
   panic-symmetry pin exist precisely for this; the pin covers the arm deterministically. If
   `llvm-cov` attributes no `BRDA` record to the helper in our file, the match form is still the
   safe shape under either attribution. Verify with `scripts/coverage-gate.sh` before reporting.
2. **The structural stall pin hangs instead of going red if the mutation runs inline.**
   *Mitigation*: the std-thread watchdog that releases the gate on a deadline — termination is
   guaranteed by construction, and the "sleeps completed on time while held" assertion converts
   the wedge into a fast red. Never rely on tokio timers firing from a wedged runtime thread.
3. **A flaky red on hardware-dependent assertions.**
   *Mitigation*: the CI pins assert structure, never wall-clock margins; only the bench (outside
   CI gates) and the recorded note carry hardware numbers; the real-server leg asserts
   correctness under concurrency only. Do not tighten any bound to "make it discriminate".
4. **Await-before-reply is enforced by code order at 12 sites, not by the type system.**
   *Mitigation*: one greppable seam (`grep -rn "spawn_blocking" src/` → the helper), the ordering
   pin, A1's hard-kill leg, and `tests/persistence.rs`'s restart legs — a reply-before-commit
   regression shows up as a row missing from the restart copy.
5. **The measurement shows only a modest win (tmpfs / fast NVMe).**
   *Mitigation*: the honesty clause — report verbatim numbers, state the caveat, let the operator
   rule. The repair is a quality fix (~20 production lines); no tuning may be introduced to
   improve the number.
6. **Blocking-pool depth under bursts.**
   *Mitigation*: commits serialize on the persist mutex — at most one blocking thread does real
   work; the rest park on the mutex, bounded by concurrent sessions (rate.rs drives 100; pool
   default is 512). Blocking tasks never occupy async workers.
7. **Shutdown with a commit in flight.**
   *Mitigation*: none needed beyond documentation — runtime drop waits for blocking tasks; a
   hung disk at drop is SIGKILL-equivalent (per-mail transaction under WAL + FULL is
   all-or-nothing; worst case is an unsent `250` and a client retry). No drain machinery — the
   drain is precisely what was cut.
8. **The seq-assignment race gets a new texture (not a new kind).**
   *Mitigation*: name it in the spec (done — "Named windows") so review does not attribute a
   pre-existing cross-session restart-order swap to this change; fixing it (assign `seq` under
   the memory lock) is out of scope.
9. **The bench target must stay clippy/fmt clean under `--all-targets`.**
   *Mitigation*: the bench is ordinary code under the same gates; run the full gate list before
   reporting.
10. **A future edit softens the pragma or reorders the reply.**
    *Mitigation*: A1's hard-kill leg is the standing mechanical net — red against
    commit-after-`250`, write-behind, and `synchronous = NORMAL`; the two lines are pinned in the
    decision record.

## Rejected alternatives (and what each would cost)

- **Dedicated writer thread / actor** — a panicked writer loop is a permanent wedge; a
  rendezvous needs a shutdown drain or `RunningServer::stop` hangs; ~100–150 covered lines
  re-buying the persist mutex (queue) and the blocking pool (threads). Its single advantage —
  narrowing the resurface hazard — does not close it and couples memory visibility to commit
  rate.
- **Async `Store`** — churns the founding API, breaks the wave-2 gate closure (a sync `Fn()`
  inside the drain→mirror window cannot await), zero guarantee gain.
- **Memory-half/commit-half split** (with `Persist` becoming `Arc`) — re-engineers exactly the
  window the clear-race tests pin; the memory half costs nanoseconds and belongs with the
  commit.
- **`block_in_place`** — panics on current-thread runtimes (the pins' own runtime), converts
  rather than frees a worker.
- **Mode branch ("offload only when persisting")** — a permanent dual path to save a µs hop; one
  path survives chaos better than two.
- **Batching / group commit / `synchronous = NORMAL` / WAL tuning** — write-behind by another
  name; the pragma is the disk leg.
- **New failure protocol (4xx on disk failure)** — changes 0001's log-and-accept semantics; its
  own decision document, not asked for.
- **`Persist` trait "for testability"** — the wrappers are executed by the real suites; traits
  are how an exact-100% budget dies.
- **Production failure-injection surface (env-gated)** — a durability-breaking knob in the
  binary forever; the wave-2 seam stays `#[cfg(test)]`.
- **Commit-latency histogram in `/metrics`** — real covered surface bought with gate budget; the
  tracing error path, A1, and the bench are the observability this change ships.
