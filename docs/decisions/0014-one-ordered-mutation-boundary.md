# 0014 — One ordered mutation boundary: commit before visibility

- **Status:** proposed (awaiting operator acceptance)
- **Date:** 2026-10-13
- **Amends:** [0012](0012-commit-failure-refusal-and-the-mutation-boundary.md)
  (proposed) §4, §5 and §6 **in place** — 0012 still awaits the operator, so
  the amendment is legitimate; the amendment itself is a landing task of the
  run that adopts this decision. Accepted decisions are never edited; this
  one is not accepted yet either.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) — strengthened:
  a `250` implies live **and** committed; [0002](0002-awaits-subscribe-before-checking.md)
  — untouched; [0003](0003-oldest-only-pruning.md) — the eviction choice is
  byte-identical, its counter exactness settled in §4 below;
  [0010](0010-off-worker-commits.md) (proposed) — built upon, not redone.

## Context

Issue #31's three P1 storage-consistency findings (reviewed commit `5b09843`)
are one bug wearing three hats: `Store::insert` releases the memory mutation
lock *before* taking the persistence lock (`src/store.rs:348-410`), and
`delete`/`clear`/`clear_all` release before their mirrors
(`:514`/`:541`/`:574`). Two serializations are **two orders**: the persist
mutex orders commits, the mutation lock orders memory phases, and nothing
orders commit *k* against memory phase *k+1*. With `max_per_inbox = 1` and
16 barrier-released threads on public `Store::insert`, a later insert prunes
an earlier id from memory and deletes its row, and the earlier transaction
then persists an already-evicted message (reproduced three times, no
injected storage failures). The same gap exposes a refused insert as
queryable mail (0012 §4's "memory truth stands" ghost) and lets a failed
delete answer `200 {"deleted":true}` and resurrect after restart. ADR0012
§6's claim that separate serialization "adds nothing across `record`" is the
exact error — it treats two orders as one.

## Decision

1. **One store-wide ordered boundary, held across the whole mutation.** The
   existing `mutation: std::sync::Mutex<()>` field's span is widened to
   cover plan → commit → apply, the SQLite transaction included. Lock order
   is fixed and one-directional: `mutation` (outer) → `persist::State`
   (inner), with no path back. Inside the boundary there are no callbacks
   that re-enter the store, no `await`, and no I/O except the one SQLite
   transaction; the whole mutation still runs on tokio's blocking pool
   (0010), so a std mutex is never held across an `await`. Readers stay
   lock-free.
2. **Commit before visibility.** `insert` *plans* (peeks the tentative
   eviction: the front `overflow` ids, and post-counters `inserted + 1`,
   `dropped + overflow`), *commits* (`record_insert`), and only on `Ok`
   *applies* the memory phase (push, prune, index, counters) — then fans
   out, outside the commit and outside the lock's tail (a hung webhook
   target can never hold the boundary; 0012 §3 preserved). On `Err` nothing
   in memory ever changed; there is **no rollback**, because there is
   nothing to roll back. `delete`/`clear`/`clear_all` commit their mirror
   deletes first and apply the memory half only on `Ok`; their signatures
   become `Result<bool, rusqlite::Error>` and
   `Result<usize, rusqlite::Error>` (0012 §1's precedent: one public
   variant, `rusqlite::Error` already public, no new error type), and
   failures propagate through every applicable adapter — REST `503`, MCP
   `isError: true`, compat shims `503`, POP3 `QUIT` `-ERR` with the session
   left in TRANSACTION and its marks unapplied (`DELE` itself remains only a
   mark) — with 0012 §2's fixed text and never a leaked SQLite detail.
3. **A refused insert is invisible, everywhere.** It never enters memory, so
   it is queryable through no surface, evicts no committed mail, fires no
   watcher and no webhook, increments no counter, and mints no phantom
   empty-inbox entry (the plan peeks; `entry().or_default()` dies). A failed
   deletion reports failure, removes nothing, and cannot resurrect.
4. **Counters are events of the commit.** The transaction writes post-
   counters; the memory bump happens in apply. A refused insert's tentative
   eviction never happened and is never counted — live or restart.
   `emails_dropped` counts every eviction of an accepted insert exactly
   once, at its commit; the eviction *choice* (front ids, in order) is
   unchanged (0003).
5. **`src/persist.rs` changes by zero lines.** `record_insert`/
   `record_delete`/`record_deletes` are already atomic and exact-id; the
   mirror was never wrong — the sequencing around it was. Its own mutex
   stays (it protects `Persist`'s direct users), and because `next_seq` is
   assigned inside the commit, seq order == boundary order == memory push
   order: the restart copy's per-inbox ordering provably equals the live
   order.
6. **The `pre_mirror_gate` seam survives, moved and contracted.** It fires
   pre-commit, inside the boundary, and its closures must never re-enter
   the store (they panic, block or observe) — re-entry deadlocks by
   construction. It remains `#[cfg(test)]`-only, `pub(crate)`, never a
   feature flag, and is the deterministic cover for `offload`'s
   `Err`/`resume_unwind` arm and the await-resolution ordering pin. Its two
   re-entry race tests are deleted: their window no longer exists, and the
   race property stays with the wire legs and the 16-thread probe.
7. **Refused alternatives, reaffirmed or newly refused:** no bounded writer
   (a queue between SMTP and the store, a full-queue policy and a
   shutdown-drain obligation — 0010 §3 forbids write-behind); no
   undo/rollback engine (inverse paths over the two-map seam versus a
   reorder that makes the ghost unrepresentable); no per-inbox locks (they
   cannot span `clear_all` and re-derive finding 1); no inbox-wide
   `DELETE … WHERE inbox = ?` (the per-id mirror stays — exact ids are
   self-verifying); no degraded mode; no `persist_failures_total` (the
   refusal is the detector); no new commit-stage chaos event (injection
   stays real: triggers and lock-holders on the real data file).
8. **The invariant the boundary buys.** *(Added by
   [0016](0016-lag-invariant.md) during run 3 of the storage-consistency
   slice; 0014 still awaits the operator, so the addition is legitimate.)*
   **The live view lags the commit; it never leads it.** For `insert`,
   memory ⊆ disk at every instant — the plan touches nothing, the commit
   lands first, apply follows — so no reader can ever see an id the disk
   does not hold. For `delete`/`clear`/`clear_all`, memory ⊇ disk at every
   instant — the mirror deletes commit first, memory applies after — so a
   reader may still see a mail whose deletion is committed, but that mail
   *was* accepted and its deletion *is* committed: a delayed view of the
   past, never a view of a state the disk never passed through. The
   forbidden lies are exactly the two directions of *leading*: the
   **ghost** — visible mail whose commit did not land (unrepresentable: the
   `?` fires before apply, and there is nothing to roll back) — and the
   **resurrection** — a success reply for a deletion the restart copy
   contradicts (unrepresentable: the reply trails the commit, and the
   restart copy is exactly the committed set). The one tear is poison's (a
   mid-apply panic), bounded by the boundary itself and healed by restart
   ([0015](0015-boundary-failure-contract.md) §3). Every future change to
   the boundary is tested against this sentence.

## Consequences

- **Public contracts change** on `delete`/`clear`/`clear_all` and their
  `_async` wrappers — hence this document. Eight production call sites map
  the failure (two REST, two compat, three MCP, one POP3 `QUIT`); SMTP has
  no delete surface; the UI and stdio are read-only or REST-bound.
- **The crash window moves and shrinks.** The only new window is *commit
  succeeded, memory apply not yet run*; a process death there leaves the
  row on disk with no `250` sent, so a client retry delivers the duplicate —
  at-least-once, strictly better than today's memory-first window (live in
  memory, never on disk, process dead). The restart copy is always exactly
  the committed set.
- **Serialization is total across inboxes.** A slow or failing disk delays
  every mutation behind it (bounded by the 5 s busy handler) — the price of
  a shared order; readers answer throughout, and `tests/burst.rs` plus the
  `benches/ingest.rs` steady-state number (recorded in 0012's amendment)
  hold the regression line.
- **An apply-phase panic after a successful commit fail-stops** via the
  poison `expect` — disk holds what the poisoned store does not. No
  recovery machinery is added; invariant-break territory fail-stops, as it
  already does.
- **A refused insert burns its UUIDv7 id** and nothing else. An inbox left
  over cap by a refused insert stays over cap until the next accepted
  insert — pruning on refusal would evict committed mail the boundary
  protects (0003 puts eviction in `insert`).
- **ADR0012 §4/§5/§6 are amended in place** and
  `docs/decisions/README.md` lists 0012; the mirror-image debt note at
  `tests/persistence.rs:401-407` (an "epoch/generation scheme" would be
  needed) is retired — the boundary closes that race structurally.
- **Every new production line ships with the test that executes it** — the
  rewritten `src/store/persist_tests.rs` failure paths (0011 rule 2), the
  16-thread barrier probe inline in `src/store.rs` (0011 rule 1), and
  real-protocol wire legs with no mocks (0004): a real rejecting trigger on
  the real data file, a second connection holding the write lock, restarts
  against the same file. No new `src/` file, no new type, no new
  dependency, no new wire string.
- **Both this decision and 0012 as amended remain `proposed`** — acceptance
  is the operator's. Until accepted, the tree must not be judged against
  the amended text by anything but this run's own review.
