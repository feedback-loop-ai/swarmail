# 0016 — The live view lags the commit: the boundary's invariant, disk exhaustion as refusal, and a loud busy bound

- **Status:** accepted (operator, 2026-10-14 — issue #32 finding 5 resolves the backlog's proposed decisions)
- **Date:** 2026-10-13
- **Amends:** [0014](0014-one-ordered-mutation-boundary.md)
  (proposed) — adds the lag invariant as §8 **in place**;
  [0015](0015-boundary-failure-contract.md) (proposed) — extends §2 **in
  place**. Both still await the operator, so the in-place amendment is
  legitimate (0014's own precedent); the amendments are landing tasks of the
  run that adopts this decision.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md) — untouched;
  [0023](0023-off-worker-commits.md) and
  [0020](0020-concurrency-tests-and-the-coverage-gate.md) (proposed) — built
  upon; [0012](0012-commit-failure-refusal-and-the-mutation-boundary.md)
  (proposed) — its amended text stands; 0014 and 0015 (proposed) — confirmed,
  not reopened.

## Context

Third pass over issue #31's storage-consistency slice (reviewed commit
`5b09843`). The boundary (0014, implemented at `3b158cd`) and its failure
contract (0015, landed at `832b3fd`) are at HEAD, and **no criterion is
red**: the C1 probe
(`sixteen_threads_cannot_leave_memory_and_disk_in_two_orders`) was re-run
green on `19cc49d` during this run's design, and the real-server wire legs
are green. The council's rule holds — the mechanism is not reopened, and
nothing in this decision changes a line of behavior. What the panel found
instead is that the design's load-bearing property is nowhere *stated*, one
real failure is never *claimed* by the contract that already answers it, and
the contract's most load-bearing constant is *enforced* by nothing:

1. **The lag invariant is unstated.** The boundary's whole value is a
   property no document names: the live view lags the commit and never leads
   it. 0012 §4/§6 and 0014 §2 describe the mechanism (plan → commit → apply,
   zero-trace refusal) and 0015 §3 names the poison tear — but the invariant
   itself exists only in run-local prose. An unstated invariant cannot be
   defended in review; a future change can walk past it without ever
   meeting it.
2. **Disk exhaustion is unclaimed.** ENOSPC/EDQUOT is an ordinary commit
   error and the refusal contract already answers it — yet 0012 §Why
   mentions ENOSPC only as an example of transience, 0015 never claims the
   case, and no document records why there is no size cap. The claim would
   be rediscovered under a full `/tmp` in CI instead of read from the
   contract.
3. **The busy bound is enforced by nothing.** 0015 §2 documents the ~5 s
   bound as part of the refusal contract and leaves it prose-only. Its true
   location is the dependency's open-time default — invisible in this tree —
   so a dependency bump that moves it rewrites every surface's failure
   timing *silently*, the exact scenario §2 names.

## Decision

1. **The boundary is confirmed as the closing design; the refusal list
   stands without re-litigation.** No criterion is red, so no mechanism line
   changes: the one ordered mutation boundary — the `mutation` lock spanning
   plan → commit → apply for all four persistent mutations
   (`src/store.rs:363/:514/:547/:577`), commit before visibility, refusal
   with zero trace, fan-out only after apply — and 0015's failure contract
   are the design of record for findings 1–3. 0014 §7's refusals (no
   bounded writer, no rollback engine, no per-inbox locks, no inbox-wide
   `DELETE … WHERE inbox = ?`, no degraded mode, no failure counter, no
   commit-stage chaos) and 0012 §Why's refusals carry forward as review
   checks: a successor that wants any of them owes a red criterion first.

2. **The lag invariant, stated once.** *(Added to 0014 as §8.)* **The live
   view lags the commit; it never leads it.** For `insert`, memory ⊆ disk at
   every instant — the plan touches nothing, the commit lands first, apply
   follows — so no reader can ever see an id the disk does not hold. For
   `delete`/`clear`/`clear_all`, memory ⊇ disk at every instant — the mirror
   deletes commit first, memory applies after — so a reader may still see a
   mail whose deletion is committed, but that mail *was* accepted and its
   deletion *is* committed: a delayed view of the past, never a view of a
   state the disk never passed through. The forbidden lies are exactly the
   two directions of *leading*: the **ghost** — visible mail whose commit
   did not land (unrepresentable: the `?` fires before apply, and there is
   nothing to roll back) — and the **resurrection** — a success reply for a
   deletion the restart copy contradicts (unrepresentable: the reply trails
   the commit, and the restart copy is exactly the committed set). The one
   tear is poison's (a mid-apply panic), bounded by the boundary itself and
   healed by restart (0015 §3). Every future change to the boundary is
   tested against this sentence.

3. **Disk exhaustion rides the refusal contract; no size cap is invented.**
   *(Amends 0015 §2.)* ENOSPC, EDQUOT and a read-only remount are ordinary
   commit errors: a full or failing disk answers the fixed-text refusal
   (451/503/`isError`/`-ERR`) on every surface, after the same bounded
   stall, with zero new code. **A size cap and a `552` are refused:** a size
   policy enforced atomically is a new promise and a new ordering question
   (cap-check before or inside the boundary?) with no red criterion under
   it, and the refusal contract already subsumes the real failure. Cost,
   named: a hostile peer can fill the disk and every send answers `451`
   until an operator intervenes — bounded, convergent, honest; the
   counter-move is deployment-owned (a volume quota), not a store policy.

4. **The busy bound is pinned by a fact tripwire, not prose.** *(Amends
   0015 §2's checkability clause.)* A test in `src/persist.rs`'s own
   `#[cfg(test)] mod tests` opens a real `Persist` and asserts the opened
   connection's `PRAGMA busy_timeout` equals the dependency default (5000 —
   `sqlite3_busy_timeout(db, 5000)`, rusqlite 0.32.1
   `src/inner_connection.rs:119`, the line 0012 §Why cites); a second
   `#[should_panic]` leg overrides the pragma on a throwaway connection and
   proves the tripwire fires. It is a **fact** tripwire, never a wall-clock
   assertion (0020 rule 5), it is branch-stable, and it adds zero production
   lines. This does **not** breach 0014 §5's "`src/persist.rs` changes by
   zero lines": that freeze guards the mirror's *behavior* — 0015 already
   corrected a comment there by the same reasoning — and test-only lines in
   the measured inline module shrink no denominator, because every one of
   them runs on every gate.

## Why

- An unstated invariant cannot be defended in review, and a confirmation
  that lives only in a run's spec does not bind the next run: intake seats
  and reviewers read the decisions, not prior specs. The invariant belongs
  where the authority is — 0014, the document a future boundary change must
  amend.
- The honest path for disk-full already exists; the only defect was that no
  document said so. Claiming it costs one paragraph and refuses a new wire
  string, a new ordering question and a new counter for a capability the
  framing does not demand.
- A constant the docs depend on and nothing pins is a silent rewrite waiting
  to happen. The tripwire makes 0015 §2's obligation loud at the moment of
  the change — an override in `persist.rs` or a moved dependency default
  fails the suite — rather than at the next incident, when every surface's
  refusal timing has already changed.

## Consequences

- 0014 gains §8 (the lag invariant) and 0015 §2 gains the exhaustion claim
  and the tripwire clause, each with a marker naming this decision;
  `docs/decisions/README.md` lists 0016. No accepted decision is edited.
- The adopting run's only `src/` change is the two tripwire tests in
  persist.rs's inline module. No new file, type, dependency, wire string,
  counter, or behavior line; the coverage gate stays exact 100/100/100
  because the tripwire's lines execute on every run.
- The tripwire's red state is any override reaching the server's own
  connection — including a dependency bump that moves the default — and
  firing it owes the decision document 0015 §2 already demands.
- This decision, and 0014/0015 as amended, remain `proposed` — acceptance is
  the operator's. Until accepted, the tree is judged against this text only
  by this run's own review.

## Factual correction marker — 2026-10-24 (NEXT-183, findings 15.1–15.2)

This marker corrects factual claims in Context item 3, Decision §§2–4, and
Consequences above. The accepted status and historical text remain as
recorded; this is not a new ruling, amendment, or operator acceptance.

### Insert interval (15.1; §2)

The claim that memory is a subset of disk at every instant for insert, and
that no reader can see an id absent from disk, is false during capped
eviction. At cap 1, the live and disk mail-id sets initially both contain A;
inserting B commits B and deletion of A in one transaction before memory
applies that eviction. In the interval after commit and before apply, the
disk mail-id set is `{B}` and the memory set is `{A}`. This preserves
commit-before-new-visibility: B is not exposed before its commit, and A was
previously committed. Successful completion mirrors the planned eviction;
ordinary refusal applies nothing. Separate memory updates and readers that do
not take the mutation mutex do not establish a whole-store snapshot. This
counterexample and its limits are recorded in
[0014](0014-one-ordered-mutation-boundary.md)'s NEXT-183 marker. The
unqualified subset and no-missing-disk-id wording above is historical error.

### Busy-handler scope (15.2; Context item 3, §§3–4)

`busy_timeout=5000` configures SQLite's busy handler for lock-contention
waits and retries only. It is not a deadline for ENOSPC, EDQUOT, read-only
errors, arbitrary I/O, fsync, mutex or blocking-pool waiting, scheduling, a
whole transaction, request, or shutdown. A conflict need not consume the full
threshold. The tripwire proves the configured PRAGMA value, not an
elapsed-time bound. Accordingly, the statements above that a full-disk
refusal follows the same bounded stall, or that the tripwire enforces a
refusal-time bound, are incorrect. Returned storage errors still use the
existing fixed refusal mapping. See the NEXT-183 marker in
[0015](0015-boundary-failure-contract.md).
