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
   accepted. Decision 0002 is untouched: the ghost never fires a channel,
   and a scanning awaiter finds it via the store-is-truth rescan — the
   same shape as every other stored-but-unnotified mail.
4. **Memory truth stands; there is no rollback.** A refused mail stays
   queryable in memory, and the lifetime counters count it. Rollback is
   refused: it is a second, inverse mutation over the very two-map seam
   contract #2 exists to make atomic; it must un-increment
   `emails_dropped`, which 0003 requires to count every eviction
   *exactly*; and it doubles the cold path with an undo that must itself
   be infallible, for a cosmetic payoff — the ghost is bounded by the
   per-inbox cap and "stored but refused" breaks no frozen guarantee.
5. **The refusal seam is the acceptance seam — `insert` only.**
   `delete`/`clear`/`clear_all` keep their logged swallow
   (`let _ = self.record(...)`). No acknowledgement is minted on their
   behalf in this epic; widening would drag refusal shapes into REST
   `DELETE`, the compat shims and POP3 `DELE` session semantics — a
   protocol change with zero framed criteria, each new line owing its own
   leg under the exact 100/100/100 gate. The hazard the widening would
   address — a failed delete commit leaves the row behind, so a restart
   resurrects mail the live store shows deleted — is real, pre-existing,
   and *not closed by widening either* (on refusal the row stays behind
   until the next successful mirror); it is tracked as the mirror-image
   debt noted in `tests/persistence.rs:401-407`. A follow-up epic owns it
   if the operator wants it.
6. **The memory half of each mutation becomes one unit.** One
   `std::sync::Mutex<()>` field spans the memory phase of all four
   mutations: `insert` (push + prune + pruned-id index removals + global
   index insert + the counter), `delete` (index remove + retain + watcher
   release), `clear`/`clear_all` (the drain). After any mutation
   completes, `count(inbox)` equals the number of ids retrievable via
   `get(id)` whose inbox is that inbox. The lock is **never held across
   the commit, an `await`, or a gate callback** — the pre-mirror gate's
   closures re-enter the store, so a boundary spanning the gate would
   deadlock the existing clear-race tests by construction; and it adds
   nothing across `record`, which is already serialized by
   `persist::State`'s own mutex. Readers (`count`, `get`, `list`,
   `inboxes`, `wait_for`, `subscribe`) stay lock-free: the invariant is a
   rest-state invariant, proven at barriers, not a mid-mutation observer
   guarantee. Pruning stays oldest-only and `emails_dropped` still counts
   every eviction exactly (0003) — the eviction choice is untouched.
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
- On refusal the memory copy and the disk copy diverge, bounded by the
  per-inbox cap: the ghost is queryable in memory, REST, MCP, POP3 and
  the UI until restart; invisible to watchers and webhooks; absent from
  the restart copy; and the counters on restart read lower (they come
  from disk). A client that got `451`, retried after repair, and got
  `250` can observe the refused copy still in memory — two mails for two
  transactions, which SMTP already is.
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
