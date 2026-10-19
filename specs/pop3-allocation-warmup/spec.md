# Exact POP3 allocation measurements on macOS

Date: 2026-10-19. Design draft for epic #120 batch 5, issue #143, PR #167.
Base: `6d843d2` on `epic120-delivery-constitution`.

## Feature and rationale

Prepare the allocation tests' own synchronization before arming their
thread-local counter. Measure the real POP3 operation once, retaining every
existing exact assertion on Linux and macOS. Implementation is confined to
`src/pop3/alloc_tests.rs`; this design commits only these three spec files.

The macOS receipt reports three requests / 1112 bytes where the counter
control expects one / 1000, and twelve requests where the eight-mail
snapshot expects eleven. The control enters an unprepared `Barrier` inside
`observe`; `snapshot` locks the test-only `Store::snapshot_gate` mutex on
its first call. Store insertion does not warm that mutex. Darwin's lazy
boxed pthread mutex/condition variable account for the reported 64 + 48
extra bytes in the barrier case and an extra request in the snapshot case.
Which thread first reaches the barrier can change which tally sees setup.

The intake's TLS-destructor explanation is not adopted: the counter uses
const-initialized, destructor-free cells, and libc allocation for TLS
registration does not pass through Rust's global allocator. Source inspection
supports synchronization setup as the cause; the failure log supplies no
allocation stacks, and local Linux results cannot certify Darwin behavior.

Framing provenance: `.brokkr/state/tasks/` contains historical run-12 tasks,
not this repair's intake. The current commission and its matching
`.brokkr/state/epic120-run5mac-feature.txt` supply the applicable framing.
Both current positions in `.brokkr/state/design/positions/` were considered;
the old batch-2 SQLite fix-up framing is unrelated.

## Panel settlement

1. **Adopt simplicity's narrow preparation boundary.** Warm the actual
   barrier through a disarmed rendezvous by both participants; warm the
   actual store's empty snapshot gate directly before observing `snapshot`.
   Keep `observe` a single-invocation, literal counter with its existing
   unwind guard. Do not add speculative `thread::current()` initialization:
   no inspected path establishes it as necessary here.
2. **Adopt robustness's per-instance diagnosis and scheduling requirement.**
   Warming unrelated primitives is ineffective. The preliminary barrier
   round must finish on both sides before their measured-round protocol;
   it cannot rely on sleeps or which thread wins scheduling. The worker's
   megabyte allocation stays between the two measured rendezvous.
3. **Reject robustness's blanket rehearsal of production operations.** Do
   not pre-run `snapshot`, `payload_len`, or `retr_reply` to settle counts.
   Rehearsal can hide a new first-use allocation or cache in production.
   The losing alternative's benefit is resilience to new lazy runtime
   objects; the chosen approach pays for that loss with explicit setup
   knowledge and a possible future macOS failure requiring diagnosis.
   The already-passing sizing and RETR scopes need no preparation because
   their measured paths do not touch these harness synchronization objects.
4. **Adopt robustness's local lazy fixture, strengthen its warm half.** A
   local `OnceLock` holding a 64-byte allocation gives Linux a deterministic
   cold/warm proof without TLS teardown or fresh-thread setup noise. Include
   a separate allocation in the warm scope so it cannot pass by suppressing
   all counting. This replaces simplicity's destructor-bearing TLS fixture:
   it loses TLS-specific resemblance, which is acceptable because TLS
   registration is not the diagnosed source. Both proposals agree on testing
   preparation order and on independent broken-code demonstrations.
5. **Reject allocation suppression and relaxed pins.** No caller filtering,
   size allowances, baseline subtraction, retries, mid-scope reset, platform
   expectation, or arbitrary initializer disarming. Such mechanisms could
   hide the extra allocation this harness exists to detect.

## Contract and decision assessment

This is a harness repair, not a semantic change: no public contract, frozen
guarantee, or ruling is amended, so no new decision is proposed. Accepted
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
governs placement and deterministic synchronization. Accepted
[0028](../../docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md)
continues to govern the snapshot posture; the existing size/one-copy work
is described by proposed
[0037](../../docs/decisions/0037-pop3-size-accounting.md), and execution-based
proofs by proposed [0050](../../docs/decisions/0050-delivery-gates.md) §6.
Those proposals remain proposed. Synchronous insert, subscribe-before-check,
oldest-only pruning and ingest extraction remain unchanged.

## Acceptance Criteria

1. **AC1 — Exact cross-platform control.**
   `the_counter_sees_this_threads_allocations_and_no_other` keeps the full
   tally `{requests: 1, requested: 1000, largest: 1000, live: 1000,
   peak: 1000}`. Its other thread allocates 1 MiB during the armed interval,
   synchronized by the existing two measured rounds. A separate completed
   disarmed round prepares that same barrier regardless of arrival order.
2. **AC2 — Exact first snapshot.**
   `the_snapshot_sizes_its_maildrop_without_a_per_mail_allocation` retains
   exactly eleven requests, the existing largest-request bound, and the
   eight-mail order/size oracle. Before the scope it invokes only the
   existing empty `run_snapshot_gate` on that Store; the measured snapshot
   is the first snapshot of that fixture. The seam stays in its original
   location and installed-hook behavior remains unchanged.
3. **AC3 — Linux preparation proof.** A new harness test uses independent
   local lazy fixtures and the actual `observe` counter. First touching a
   retained 64-byte buffer inside a cold scope yields exactly
   `{1, 64, 64, 64, 64}` in Tally field order. Preparing a second such buffer
   before arming, then accessing it and allocating a retained 1000-byte
   buffer inside the scope, yields exactly `{1, 1000, 1000, 1000, 1000}`.
   Removing only the warm fixture's preparation makes that assertion fail
   with `{2, 1064, 1000, 1064, 1064}`. Retain buffers until after disarming;
   use observable values so optimizer removal cannot explain a pass.
4. **AC4 — Existing sensitivity retained.** Zero-allocation `payload_len`,
   exact RETR tally/capacity, real-session memory bounds and wire bytes,
   per-thread isolation, and panic disarming all pass unchanged. No
   platform cfg bypass, range replacing an exact pin, production rehearsal,
   counter filtering, or new dependency is introduced.
5. **AC5 — Independent red proofs.** With one reversible mutation at a
   time, disabling request accounting fails AC1's named test; an observable
   per-call allocation in real `payload_len` fails AC2's named snapshot
   test and the zero-allocation sizing test; deleting AC3's preparation
   fails the new test. Preserve the existing second-RETR-buffer proof and
   identify its receipt, or rerun it if unavailable. Each receipt includes
   patch, command, executed test count, exit status and intended assertion
   failure. A compile error, zero-test filter or timeout is not evidence.
   Restore all mutations and prove the same tests green afterward.
6. **AC6 — Scope and final gates.** The implementation changes only the
   existing declared harness and, if needed, these spec artifacts. Full
   house gates on the restored final source tree pass with exact zero
   missed lines, branches and functions; no coverage exclusions, gate,
   dependency, or production-code changes. The plan assigns commands and
   pinned verify responsibilities. Unexecuted gates remain outstanding.
7. **AC7 — Honest platform and delivery status.** Linux controls establish
   preparation ordering, not macOS execution. Record `test (macos-latest)`
   on the candidate as pending until CI passes the unchanged pins. Append
   signed commits above `6d843d2`, verify their signatures, leave a clean
   tracked tree on the existing branch, and perform no push, PR, merge or
   issue operation. A signing failure blocks delivery; unsigned fallback
   is forbidden. This phase reports only `drafted`.
