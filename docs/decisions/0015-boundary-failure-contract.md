# 0015 — The boundary's failure contract: refusal timing, a partial POP3 prefix, poison and the second writer

- **Status:** accepted (operator, 2026-10-14 — issue #32 finding 5 resolves the backlog's proposed decisions)
- **Date:** 2026-10-14
- **Amends:** [0012](0012-commit-failure-refusal-and-the-mutation-boundary.md)
  (proposed) §5 and §7 **in place** — 0012 still awaits the operator, so the
  amendment is legitimate (the precedent is 0014's); the amendments are
  landing tasks of the run that adopts this decision.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) — the refusal
  contract recorded here is the converse arm that keeps `250 ⇒ stored`
  honest; [0002](0002-awaits-subscribe-before-checking.md) and
  [0003](0003-oldest-only-pruning.md) — untouched; [0020](0020-concurrency-tests-and-the-coverage-gate.md)
  (proposed) — its no-wall-clock discipline governs the contract's new
  instrument; [0014](0014-one-ordered-mutation-boundary.md) (proposed) —
  built upon, not reopened: no line of the boundary changes.

## Context

Issue #31's storage-consistency slice, second run. The boundary (0014,
implemented at `3b158cd`) is at HEAD and no criterion is red — the panel
upheld it. What the panel found instead is that the boundary's *failure
contract* — what a client, a disk, a restart and a hostile peer see when
each thing that can go wrong goes wrong — is carried by code comments and
prose that in one place claims more than the code does:

1. **POP3 `QUIT` is documented atomic and is at-least-once.** 0012 §5 says
   the session is left in TRANSACTION "and its marks unapplied"; the
   `src/pop3.rs` `QUIT` comment says "the update is abandoned whole: the
   marks are unapplied" — and three lines later admits "the already-deleted
   part of a partial batch stays deleted". The code applies marks one
   `delete_async` at a time and answers `-ERR` on the first failure, so a
   mid-loop failure (a trigger refusing one specific id, a second connection
   taking the write lock) leaves a committed **prefix**. The wire legs
   cannot see this because their trigger refuses *every* row: the first
   delete fails, the prefix is empty, and "unapplied" holds in the test
   world. The leg proves refusal; it does not prove atomicity.
2. **The busy bound is load-bearing and undocumented.** A stalled or
   contended data file delays every refusal; the delay is bounded by
   rusqlite's default busy timeout — `sqlite3_busy_timeout(db, 5000)`, set
   by the dependency at open (rusqlite 0.32.1
   `src/inner_connection.rs:119`, the line 0012 §Why cites). No PRAGMA in
   this crate sets or overrides it. Nothing in the documents says the bound
   is part of the refusal contract: whoever tunes it — an override in
   `persist.rs` or a dependency bump — silently rewrites the failure
   semantics of every surface (`451` after a bounded stall, not immediately
   and not never).
3. **Poison, the second writer and the refusal residue are behaviours, not
   accidents — but they live in prose scattered across 0012 §7 and 0014's
   consequences,** with no single statement of what recovery means.

## Decision

1. **POP3 `QUIT` is at-least-once; a mid-batch failure leaves a committed
   prefix.** *(Amends 0012 §5.)* `QUIT` applies `DELE` marks one store
   delete at a time; on the first failure it answers `-ERR
   storage commit failed` with the session left in TRANSACTION and the
   failing suffix unmarked. The already-committed prefix **stays
   committed** — on both trees, live and restart. Nothing reports deleted
   and resurrects; nothing lies. Every client path converges: a retry
   `QUIT` is idempotent over the unmarked suffix, `RSET` unmarks, and a
   fresh session shows the truth. **Atomic `QUIT` is refused for this run**
   — collect the marks, one `record_deletes`, one apply — because it is a
   public-contract change plus a new store API with no red criterion under
   it; the adapter-consistency run owns it. The at-least-once contract is
   bounded and convergent; a document asserting all-or-nothing over an
   at-least-once loop is not.
2. **The busy handler is part of the refusal contract.** A stalled or
   contended data file delays each refusal by at most rusqlite's default
   busy timeout (~5 s per statement), set by the dependency at open and
   **never overridden in this crate**; after the bound, `SQLITE_BUSY`
   surfaces as the fixed-text refusal (451/503/`isError`/`-ERR`). The bound
   is deliberately **not wall-clock-asserted** (0020 rule 5): the wire legs
   prove the *refusal*, never the timing. Changing the bound — an override
   in `persist.rs` or a dependency bump — rewrites the failure semantics of
   every surface and owes a decision document. *(Amended by
   [0016](0016-lag-invariant.md): **disk exhaustion is an ordinary commit
   error.** ENOSPC, EDQUOT and a read-only remount answer the fixed-text
   refusal on every surface after the same bounded stall, with zero new
   code — the contract subsumes the real failure rather than adding a
   second vocabulary for it. **A size cap and a `552` are refused:** a size
   policy enforced atomically is a new promise and a new ordering question
   (cap-check before or inside the boundary?) with no red criterion under
   it, and the counter-move is deployment-owned (a volume quota), not a
   store policy; the cost is named — a hostile peer can fill the disk and
   every send answers `451` until an operator intervenes, bounded,
   convergent, honest. **The bound is pinned by a fact tripwire**, not
   prose: an inline test in `src/persist.rs`'s own `#[cfg(test)] mod tests`
   asserts the opened connection's `PRAGMA busy_timeout` equals the
   dependency default, and a `#[should_panic]` leg proves the instrument
   can fire; its red state is any override reaching the server's own
   connection — including a dependency bump that moves the default — and
   firing it is the decision-document obligation this section already
   imposes.)*
3. **Poison's recovery is restart.** *(Amends 0012 §7.)* A panic inside the
   boundary fail-stops the store (the poison `expect`); the process keeps
   serving readers from memory, which a mid-apply panic may have left
   disagreeing with disk — a write-panicking, read-serving zombie. The
   honest sentence: **restart is the recovery, and the restart copy is
   always exactly the committed set**, so the poison heals by the same
   property the boundary provides. A self-exiting process would be more
   honest; it is refused here (no criterion demands it) and belongs to the
   lifecycle run if ever asked.
4. **Single-writer is a deployment assumption, not a guard.** `Persist::open`
   takes no exclusive lock. SQLite's file locking prevents byte-level
   corruption, but two servers on one data file interleave *logical* state —
   counters flip-flop, prunes fight. The cheap guard (an advisory lock at
   open) is refused for this run and owned by the deployment run; the
   assumption is named here so it is not rediscovered as a bug.
5. **The refusal residue is exactly 0014's list:** a burned UUIDv7 id, a
   burned sequence number (`next_seq` is assigned inside the commit before
   the transaction runs), and an inbox left over cap until the next
   accepted insert. Pruning on refusal would evict committed mail — the
   thing the boundary exists to protect (0003 puts eviction in `insert`) —
   and no surface claims the cap was enforced at the refusal.
6. **The contract is checkable.** A *selective* refusing trigger — refuse
   one id, allow the rest — joins `tests/common/mod.rs`'s real-injection
   vocabulary (test code only), and a POP3 wire leg drives `QUIT` across a
   mid-loop refusal: `-ERR`, TRANSACTION retained, the allowed ids' commits
   landed, the refused id on both trees, retry-after-recovery signing off.
   Red on the reviewed baseline `5b09843` (which swallowed errors and
   answered `+OK`), green on HEAD. Synthetic commit-stage chaos remains
   refused (0014 §7): the injection breaks the *disk*, not the abstraction.

## Consequences

- 0012 §5 and §7 are amended in place with markers naming this decision;
  `docs/decisions/README.md` lists 0015. 0014 stands untouched.
- `src/pop3.rs`'s `QUIT` comment is corrected to the partial-prefix truth —
  a comment-only edit: zero behavior lines change, so the coverage
  denominator is untouched.
- One test-only trigger kind and one wire leg land with this decision; no
  new production line, no new type, no dependency, no wire string.
- **Accepted losses, named:** POP3 batch updates stay at-least-once (a
  client whose `QUIT` fails mid-loop must retry to converge; a selective
  failure leaves a prefix the `-ERR` does not name); the zombie window and
  the second-writer hazard persist until their owning runs name them again —
  documented, not silent.
- Both this decision and 0012 as amended remain `proposed` — acceptance is
  the operator's. Until accepted, the tree is judged against this text only
  by this run's own review.
