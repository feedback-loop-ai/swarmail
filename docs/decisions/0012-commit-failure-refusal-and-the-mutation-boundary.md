# 0012 — A failed commit refuses, and the memory half of a mutation is one unit

- **Status:** proposed (awaiting operator acceptance)
- **Date:** 2026-10-12
- **Supersedes:** none
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) — strengthened, not weakened; [0003](0003-oldest-only-pruning.md) — untouched; [0010](0010-off-worker-commits.md) (proposed) — built upon, its load-bearing lines not redone

## Context

Epic #13, contracts #1 + #2. Two lies and one race are in the tree today.

1. **A failed data-file write is acknowledged.** `Store::record`
   (`src/store.rs:243-253`) swallows the persist error with
   "memory store remains authoritative", and the SMTP DATA path
   (`src/smtp.rs:461-462`) writes `250 2.0.0 OK: stored as {id}`
   unconditionally after `insert_async` resolves. A client completing
   `DATA` against a failing data file is told the mail is stored when the
   restart copy will never have it — the exact lie decision 0001's
   converse forbids: 0001 promises `250 ⇒ stored`; it never promised
   `stored ⇒ 250`, but a *known*-failed commit answered with `250` is an
   acceptance the code cannot defend.
2. **The memory half of a mutation is not one unit.** `insert` pushes to
   the per-inbox deque, prunes, and inserts into the global ID index as
   separate steps over two `DashMap`s; `delete`, `clear` and `clear_all`
   likewise span both maps. Concurrent mutations can observe
   `count(inbox)` disagreeing with the set of ids `get(id)` retrieves —
   concretely: a ghost (an index entry with no deque entry, so
   `count == 0` while `get(id)` is `Some`) and a panic
   (`delete`'s `"indexed mail always has its inbox list"` expect fires
   when a clear drains the inbox between delete's two steps).

The framing (`.forge/intake/epic13-boundary-framing.md`) argued no decision
document was required because the refusal *enforces* 0001/0010. That is true
of the guarantee, but the change also re-shapes two public contracts — the
`Store::insert` signature and the SMTP/REST/MCP reply vocabulary — so the
ruling is recorded here for the operator to accept, as the house rules
require for public-contract changes.

## Decision

1. **A known-failed commit refuses; it never acknowledges.**
   `record` returns `rusqlite::Result<()>` (it keeps its own
   `tracing::error!` — the log stays the one witness at the single owner of
   the mirror). `Store::insert` propagates it with `?`, so its signature
   becomes `Result<Arc<Email>, rusqlite::Error>` — one failure mode (the
   mirror commit; the memory phase has no fallible step), one variant that
   is already public (`Store::open -> rusqlite::Result<Self>`); no new
   error type. `insert_async` inherits through the existing generic
   `offload`.
2. **The refusal is a fixed, distinct line on every surface.** SMTP DATA
   replies exactly `451 4.3.0 storage commit failed` (RFC 5321 §4.2.2,
   requested action aborted — transient; the client's correct move is to
   retry after repair). The REST seed answers `503`, the MCP seed an
   `isError: true` result, both with the same fixed text. The SQLite
   detail is a log concern (`tracing::error!` carries `what` and the
   error); no internal error string reaches a peer. The text is distinct
   from chaos's fixed `451 4.3.0 chaos`, so the wire distinguishes
   *never stored* from *stored but uncommitted*.
3. **Fan-out is part of acceptance: no commit, no fan-out.** The `?`
   sits before the per-inbox watcher send and the `global_tx` send. A
   commit-failed mail fires no webhook, wakes no watcher. `subscribe_all`
   is documented as "every *accepted* email"; a refused mail was not
   accepted. Decision 0002 is untouched: *(amended by
   [0014](0014-one-ordered-mutation-boundary.md), under which the refused
   mail exists in neither tree, so there is no ghost to fire a channel or
   to be found by a rescan — a waiter that was mid-poll at the refusal
   simply keeps waiting, exactly as if the mail had never been sent)*.
4. **A failed commit is a refusal with zero trace.** *(Amended by
   [0014](0014-one-ordered-mutation-boundary.md), which supersedes the
   original "memory truth stands" wording: the ghost is unrepresentable,
   not bounded.)* A refused insert applies nothing — the mail is
   queryable through no surface, evicts no committed mail, fires no
   watcher and no webhook, increments no counter and mints no phantom
   inbox. Rollback is refused: under the one ordered boundary there is
   *nothing to roll back* — the memory phase never ran — so the second,
   inverse mutation over the two-map seam (which must un-increment
   `emails_dropped`, which 0003 requires to count every eviction
   *exactly*, and which would double the cold path with an undo that must
   itself be infallible) is never needed. The refusal leaves memory and
   data file agreeing: the same wire path can simply retry after repair.
5. **The refusal seam is every persistent mutation.** *(Amended by
   [0014](0014-one-ordered-mutation-boundary.md), which widens the seam
   from `insert` to `delete`/`clear`/`clear_all` — the mirror-image debt
   named below is closed, not tracked.)* `delete` reports
   `Result<bool, rusqlite::Error>` and `clear`/`clear_all`
   `Result<usize, rusqlite::Error>`; a failed commit propagates through
   every applicable adapter — REST `DELETE`/clear `503`, the compat shims
   `503`, MCP `isError: true` with the fixed text, POP3 `QUIT` `-ERR` with
   the session left in TRANSACTION and its marks unapplied — never
   acknowledging work that did not commit. The original refusal to widen
   (below) was a scoping decision for epic #13's contracts #1 + #2, not a
   claim that the widening is wrong: issue #31's finding 3 *is* the
   hazard, and 0014's boundary is what makes the widened seam defensible
   — the refusal leaves memory and disk agreeing, so an id the store
   never removed is an honest `Ok(false)`/`404`/not-found, and a failed
   removal is a `503` the client retries.
6. **One ordered boundary spans each mutation: plan → commit → apply.**
   *(Amended by [0014](0014-one-ordered-mutation-boundary.md): the
   original text held the lock across the memory phase only and claimed a
   boundary spanning the commit "adds nothing" — that claim treated two
   serializations as one order and is exactly issue #31's finding 1.)*
   The `std::sync::Mutex<()>` covers the whole mutation, the SQLite
   transaction included: `insert` plans the eviction and the
   post-counters, commits (`record_insert`), and only on `Ok` applies the
   memory phase (push + prune + pruned-id index removals + global index
   insert + the counters); `delete` commits its mirror delete, then
   applies (index remove + retain + watcher release); `clear`/
   `clear_all` commit their per-id mirror deletes, then drain. After any
   mutation completes, `count(inbox)` equals the number of ids
   retrievable via `get(id)` whose inbox is that inbox — on both trees,
   live and restart. The lock is **never held across an `await`** — the
   whole mutation still runs on tokio's blocking pool (0010) — and no
   callback inside it re-enters the store: the pre-mirror gate fires
   pre-commit inside the boundary, so its closures block or panic, never
   re-enter (re-entry deadlocks by construction). Readers (`count`,
   `get`, `list`, `inboxes`, `wait_for`, `subscribe`) stay lock-free.
   Pruning stays oldest-only and `emails_dropped` still counts every
   eviction exactly (0003) — the eviction choice is untouched.
7. **Poison is fail-stop, in the house's existing vocabulary.** A panic
   inside the boundary means memory truth is already broken; the guard
   uses `.expect("store mutation lock poisoned")` — panic and propagate,
   exactly as `persist.rs` guards its own state mutex — not a new typed
   refusal, which would be new semantics plus a new cold path.

## Why

- The only *knowable* lie is alive-but-commit-failed. Crash windows are
  already at-least-once (0010's ordering: a crash before the reply sent
  nothing; a crash after commit before the reply is SMTP's normal
  duplicate). This closes the window that code could close and does not
  pretend to close the ones it cannot.
- `451` over `5xx`: a failing data file is usually transient — ENOSPC, a
  full volume, a read-only remount — and `SQLITE_BUSY` is literally
  transient (the 5 s busy handler is at
  `rusqlite-0.32.1/src/inner_connection.rs:119`). A permanent-failure
  code would tell every harness the send is dead forever; a well-behaved
  client's correct move is to retry after repair, which the wire legs
  prove works (250 on retry).
- One external lock instead of a restructure: DashMap shard locks are per
  map, so no internal ordering makes a cross-map mutation atomic against
  `delete`/`clear`; a per-inbox lock cannot span `clear_all`'s
  cross-inbox wipe; and merging the maps rewrites every read path the
  benches measure. One `Mutex<()>` over the memory phase is the whole
  mechanism, and the critical section holds no I/O — the cost is
  nanosecond-scale serialization against a commit-rate-bound ingest
  (0001's own bench).
- Refused cuts, so they are not silently re-discovered: no degraded-mode
  latch and no write-behind queue (0010 §3 extends to the failure side —
  a failed commit is *this mutation's* outcome; a recovered disk resumes
  accepting on the next mail, no restart); no disk-first reordering (it
  needs the shard guard held across the SQLite commit and resurrects the
  clear-race); no `persist_failures_total` counter in this epic (the
  refusal itself is the detector — 451/503 on the wire, the log carries
  the detail; a counter is an ordinary follow-up); no
  `insert_checked` beside an infallible `insert` (two entry points for
  one mutation is vocabulary bloat); no commit-stage chaos event (it
  would need the ghost semantics and a third wire vocabulary).

## Consequences

- `Store::insert`/`insert_async` change signature in a published crate —
  a breaking change to the public API. Callers: exactly three production
  sites (`src/smtp.rs:461`, `src/api.rs:454`, `src/mcp.rs:202`) and two
  benches (`benches/ingest.rs`, `benches/writer_stall.rs`); the ~50
  remaining call sites are inline tests, a mechanical `let _ =` ripple
  forced by `Result`'s `#[must_use]`. The restore path deliberately
  bypasses `insert` (`Store::open`) and is untouched.
- On refusal the memory copy and the disk copy **agree** *(amended by
  [0014](0014-one-ordered-mutation-boundary.md); the original bullet
  described the ghost the pre-boundary tree produced)*: nothing was
  applied, so there is no divergence to bound and no restart copy that
  contradicts what the client was told. A client that got `451`, retried
  after repair, and got `250` has one mail for the one accepted
  transaction.
- Cross-inbox serialization of the memory phase: nanoseconds, and the
  persist mutex already serializes far more.
- Every new production line ships with the test that executes it: the
  rewritten degraded test (`src/store/persist_tests.rs`, the harness
  0011 gives cold failure paths), the 8-thread barrier probe (inline,
  0011 rule 1), and wire legs against real servers with no mocks —
  a second real SQLite connection holding the write lock on the real
  data file, which makes the server's real commit fail with a real
  `SQLITE_BUSY`. No new production API, no seam, and no privileges exist
  to serve the tests.
