# Persistence facts — NEXT-183

Created: 2026-10-24. Status: draft for implementation and independent review.
Scope: epic #192, issue #183, findings 15.1–15.5. Priority: P2.
Baseline: `45d68df37044de3360439b34c39687975dd59b84`.

## Feature and rationale

Make the accepted persistence records and adjacent comments describe the
implemented boundary accurately. A maintainer must distinguish commit-before-
publication from an instantaneous set invariant, SQLite lock retries from an
I/O deadline, and synchronous execution from lock-free progress. A reader
must not be directed to a nonexistent measurement or told that the refusal
tests permit a queryable refused message.

The deliverable is four dated factual addenda and comment-only changes in
two Rust files. Historical accepted text remains intact. This is observation
and correction of factual descriptions, not a new persistence policy or an
amendment accepting weaker guarantees. Accepted
[0045 §8](../../docs/decisions/0045-delivery-repairs.md) explicitly allows
observed-behavior corrections while preserving that distinction. No new
semantic decision is declared. If implementation needs a different public
contract or ruling, return to design with a proposed decision; contradiction
of an accepted ruling waits for the operator. This design supplies no such
permission.

Authorities remain [0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md),
[0005](../../docs/decisions/0005-extraction-at-ingest.md),
[0014](../../docs/decisions/0014-one-ordered-mutation-boundary.md),
[0015](../../docs/decisions/0015-boundary-failure-contract.md),
[0016](../../docs/decisions/0016-lag-invariant.md),
[0023](../../docs/decisions/0023-off-worker-commits.md) and
[0024](../../docs/decisions/0024-shutdown-refusal-at-the-boundary.md).
These links cite authority; they do not declare or accept a new decision.

## Panel synthesis and rulings

Both run-local positions were read: `simplicity.md` and `robustness.md` under
`.brokkr/state/design/positions/`. They agree on the mechanism and six-file
implementation boundary. There is no genuine architectural conflict to
manufacture; the following rules settle the remaining breadth choices.

- Adopt simplicity's six-file correction scope, unchanged executable code,
  unchanged assertions and use of existing regression tests. Reject new
  snapshot locks, watch replacement, rollback, queues, timers, benchmarks,
  test seams and documentation-lint infrastructure. Cost: existing read
  inconsistency windows and unbounded non-lock storage delays remain;
  this correction does not repair availability or create new measurements.
- Adopt robustness's explicit cap=1 counterexample and prohibition on
  claiming a whole-store read snapshot. Bare replacements such as “usually
  lags” or “approximately five seconds” are rejected. Cost to brevity:
  addenda need enough detail to distinguish new committed mail from old
  mail awaiting eviction, and ordinary refusal from poison recovery.
- Include the inherited shutdown timing caveat in 0024 and the nearby
  `record()` comment, within the same six files. Minimal watch-only editing
  loses here: it would leave a contradictory general deadline in the same
  record. Cost: two related comment locations in `src/store.rs` rather
  than only its stopping-field comment.
- Keep 0012 unchanged; identify its inherited erroneous §Why timing claim
  from the 0015 correction with a precise relative link. Do not replace
  missing provenance with 0001's memory benchmark or 0023's worker benchmark.
  Cost: historical misinformation remains visible until readers reach the
  correction, mitigated by exact claim references and cross-links.
- Use robustness's crash, slow-sink, burst, chaos, hostile-input and TLS
  cases as limits on claims, not additional implementation work. Retry
  duplicates, best-effort notifications, deployment storage repair and
  poison recovery by restart remain existing costs. No new transport/TLS
  test or subsystem sweep is warranted by these five findings. Adjacent
  DashMap “lock-free” wording is not adopted as a technical fact; this issue
  corrects the named watch claim rather than expanding into a reader audit.

## Required content

### Ordered persistence and capped eviction (15.1)

0014 §8 and 0016 §2 require explicit corrections to both “memory subset of
disk at every instant” and “no reader can ever see an id the disk does not
hold” for inserts. Include this counterexample in one addendum and directly
cross-reference it from the other, or repeat it accurately:

At cap=1, A initially occupies memory and disk. Inserting B plans eviction
of A; one SQLite transaction inserts B, deletes A and writes counters. After
commit and before apply, disk={B}, memory={A}. Neither set contains the
other. A was previously committed, and B is not newly exposed before B's
commit. Successful apply mirrors the planned eviction; after successful
completion, absent a later mutation, retained mail and counters agree.

Describe writer ordering as plan → commit → apply. Ordinary commit refusal
applies nothing and emits no success fan-out. Acceptance follows completion.
Readers do not acquire the mutation mutex, and multi-structure apply is not
a whole-store read transaction. Do not promise that every observation is a
complete earlier disk snapshot. A mid-apply panic remains 0015 §3's poison
case: later mutations panic, readers may continue, restart is recovery.
Oldest-only means insertion/acceptance order, not sender timestamps.

### Busy-handler scope and evidence (15.2, 15.3)

0014's serialization consequence, 0015 Context item 2 and Decision §2, and
0016 Context item 3 and Decision §§3–4 must be identified by the addenda.
`busy_timeout=5000` configures SQLite busy-handler lock-contention waiting
and retries. It is not an elapsed-time guarantee for arbitrary I/O, fsync,
full-disk refusal, mutation-lock waiting, blocking-pool admission, scheduling,
a complete transaction, request, or shutdown. Not every lock conflict must
consume five seconds. ENOSPC, EDQUOT and read-only failures use the existing
fixed refusal mapping when the storage operation returns an error.

Keep the dependency-default tripwire and its intentional failing control:
they prove the configuration, not a measured latency bound. No timeout or
refusal vocabulary changes. 0015's addendum must identify the inherited
0012 §Why amendment's overbroad timing claim without editing that file.
0024's Preserves, Decision §3 and Consequences timing arithmetic must be
qualified consistently: queued mutations refuse when they acquire the
boundary and observe stopping; an already admitted commit may finish.
“One busy bound”, microseconds and epsilon are not universal scheduler,
storage, process-exit or shutdown guarantees.

0014's addendum must expressly identify the alleged steady-state ingest
number in 0012 as absent from that record. This work establishes no new
throughput result and imports no unrelated historical measurement.

### Watch and refusal-test comments (15.4, 15.5)

0024's Preserves paragraph and Decision §1 must be corrected by the dated
addendum. `watch::Receiver::borrow()` is synchronous and takes a read lock;
the short boolean read introduces no await. Neither lock-free nor wait-free
progress follows. The stopping-field comment in `src/store.rs` must agree;
its `record()` timing comment must reflect the busy-handler scope above.
The expression, guard lifetime, watch wiring and boundary ownership stay.

The SMTP test's introductory comment must describe fixed 451, refused mail
absent live and after restart, preservation of prior committed mail, and
acceptance after releasing the lock. The REST test's introductory comment
must describe fixed 503, empty live list, await timeout with matched=0 and
empty restart. It must not claim a successful recovery retry it does not
execute. Neither comment may retain the “memory truth stands” explanation
of a queryable refused row. Assertions and fixtures stay unchanged.

## Acceptance Criteria

- **AC1 — History:** Each of 0014, 0015, 0016 and 0024 has one clearly dated
  factual-correction section identifying NEXT-183 and relevant finding
  numbers/sections. Each original file is a byte-for-byte prefix of the
  result, including status and prior markers. Addenda distinguish factual
  observation from accepted normative text and imply no operator acceptance.
- **AC2 — Eviction:** Both relevant addenda explicitly refute the insert
  subset/no-missing-disk-id claims using the cap=1 example directly or by
  exact cross-reference. They state commit-before-new-visibility, zero-trace
  refusal, completed-state agreement and the reader/poison limitations.
- **AC3 — Timing:** The targeted timing claims in all four records and the
  `record()` comment are scoped to lock contention. I/O/full-disk/request/
  shutdown deadlines are explicitly disclaimed, with the existing 5000 ms
  configuration, refusal mapping and dependency tripwire preserved.
- **AC4 — Provenance:** 0014 explicitly states that 0012 does not contain
  the cited steady-state ingest number; no replacement number is invented.
- **AC5 — Watch:** 0024 corrects both watch claims, and the stopping-field
  comment states synchronous read locking and no await without claiming
  lock-free/wait-free progress or changing any executable token.
- **AC6 — Refusal:** Both test descriptions match their own live/restart
  zero-trace assertions; only the SMTP description claims a recovery retry.
- **AC7 — Scope:** Implementation changes exactly the four ADR addenda and
  the named Rust comments, apart from these delivery artifacts. No executable
  token, assertion, fixture, public API, wire text, setting, dependency,
  license allowance, gate, schema or production behavior changes. Acceptance,
  subscription, pruning and ingest-extraction guarantees stand.
- **AC8 — Proof:** Append-only and comment-only review passes; existing real
  persistence, cap, busy-default and shutdown regressions pass. Every gate
  in [house rules](../../docs/house-rules.md#gates--part-of-every-change),
  including fail-closed native-tls absence, is required before implementation
  success. CI-owned proofs remain CI-owned; missing execution is reported
  as outstanding, never inferred from a design check or document review.
- **AC9 — Delivery:** Retain the engine route and four independent review
  lenses selected by triage under 0057. This chief commit contains only
  `spec.md`, `plan.md` and `tasks.md` in this one directory, is unsigned and
  remains HEAD for deterministic design validation. Run-local positions and
  result receipts remain uncommitted. No push, release, issue closure or
  epic-completion claim follows from a draft.
