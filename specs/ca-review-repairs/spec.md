# Close the bounded CA review findings

## Feature and rationale

Close exactly the three LOW findings from issue #220's bounded CA executor
review: isolate tests that require successful admission, return the specified
closed-scope outcome at worker entry, and restore the 0078 decision-index row
to its table. A correct process-global refusal currently makes unrelated tests
flaky; dropping a live waiter's sender incorrectly classifies a known scope
closure as worker failure. Neither requires a new executor policy.

## Authority and evidence

The current framing is [intake-low-repairs.md](../bounded-ca-executor/intake-low-repairs.md).
The requested `.brokkr/state/tasks/` directory is absent. The predecessor under
`.forge/tasks/` frames the original executor, not this repair. Current intake,
the supplied task and `.forge/epic192/issue220/review-e3bd9c06.md` supply the
needed local evidence. The packet README has an empty prior-runs section; the
review records REVIEW-RESIDUAL-OK and the earlier parallel api failure.

On 2026-10-25, the chief verified the complete packet against its directory's
SHA256SUMS, including README, operator rulings and review, and verified the
candidate-local engine digest
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
No remote service, journal or sibling worktree supplies authority here.

Accepted [0069](../../docs/decisions/0069-bounded-ca-execution.md) already
requires closed-intake Unavailable and reservation ownership. Existing
[0078](../../docs/decisions/0078-ca-executor-ownership.md) records compatible
process ownership and accounting choices and remains **proposed**. This repair
restores those specified outcomes; it changes no public contract or ruling and
weakens no frozen guarantee. No new ADR is declared or required. Historical
proposal language in 0069 and the packet is superseded by the dated operator
acceptance, not by this seat.

## Panel synthesis and rulings

Both run-local positions were read in full:
`.brokkr/state/design/positions/simplicity.md` and `robustness.md`.

- **Adopt simplicity's implementation boundary:** move the existing async
  `CA_CONTRACTS` fixture to `tests/common/mod.rs`, reuse it from all admitting
  fixtures, split the existing worker-entry cases, and remove one index blank
  line. No new implementation module, dependency or runtime owner is needed.
- **Adopt robustness's ownership proof:** a live receiver gets a charged
  Unavailable reply, the job never executes, the original reservation stays
  charged while the reply is retained, and disposal refunds exactly once.
  Keep cancelled-receiver and genuine Worker-failure proofs distinct. An idle
  wait without inspecting the reply is insufficient and would deadlock on the
  new retained charge.
- **Rule on proof strength:** robustness and current intake win over simplicity's
  illustrative 16-thread/several-round stress suggestion. Require five
  consecutive rounds of all four complete binaries at 32 threads with competing
  processes. Cost: more execution time and logs. The cheaper alternative loses
  because it underspecifies the agreed intake proof. Finite repetition still
  needs a complete static fixture audit.
- **Reconcile teardown:** simplicity's full-fixture lifetime is the default;
  robustness's completion/refund observation is required for fixtures that
  abandon CA work. Server stop alone is not a builder drain. This adds proof
  effort only where an owner actually leaves work behind.
- **Reject broader hardening:** no scheduler, queue, fairness, retry-to-success,
  cross-process lock, larger capacity, public knob, new cancellation framework
  or whole-suite serialization. These obscure or change the accepted policy.
  Retain cross-server refusals, possible starvation and process-lived workers
  as existing costs. Scoped fixture serialization costs test parallelism and
  requires future admitting fixtures to join the convention.

## Acceptance Criteria

1. **Complete fixture isolation.** Audit every server-managed webhook GET/PUT
   admission in `tests/api.rs`, `tests/lifecycle.rs`, `tests/p2.rs` and
   `tests/persistence.rs`, including setup and observations after DELETE.
   Every fixture requiring uncontended admission holds the shared async guard
   through its CA work and owned results, or records a concrete independence
   reason. `tests/webhook.rs` imports that same definition and keeps its existing
   protection. Unrelated tests remain parallel; dedicated contention tests
   retain simultaneous operations and refusal assertions within their fixture.
   Five consecutive rounds of the four complete binaries at 32 test threads
   with competing processes pass, with commands, counts and exit codes retained.
   Isolated reruns, accepting 503 as success, or retries do not satisfy this.
2. **Explicit pre-entry refusal.** A deterministic test enqueues work, closes
   its scope before worker entry and keeps the receiver alive. The job does not
   run; the receiver obtains `Err(TargetPreparationError::Unavailable)` inside
   `Charged`, not RecvError/Worker. Input is disposed before refund; the original
   slot and byte charge survive while the reply is retained, then refund exactly
   once on disposal and capacity is reusable under a fresh live scope. Preserve
   cancelled-receiver disposal and failed-send reclamation. Keep genuine channel,
   execution and panic failures distinct as Worker. No sleep synchronization.
3. **Existing caller contracts preserved.** Existing or extended handler and
   construction tests prove native GET/PUT 503 with exactly
   `webhook CA preparation unavailable`, unchanged installed configuration on
   refused PUT, and the same fixed delivery attempt error with existing retry
   and terminal ownership. Genuine failures retain GET's fixed response-
   preparation 500, PUT's configuration-preparation 500 and delivery's fixed
   worker failure. Existing held-worker real HTTP/SMTP tests still prove DATA
   250 and persistent queryability before builder release. Closure after worker
   entry retains existing finite-work and live-publication rules.
4. **Index repair without acceptance.** The 0078 row occurs exactly once,
   immediately in the decision table with no separating blank line. Both row
   and record remain proposed; the record itself and accepted rulings stay
   unchanged.
5. **Candidate-bound completion evidence.** Implementation is committed unsigned
   before authoritative verification; every house gate in
   [the charter](../../docs/house-rules.md#gates--part-of-every-change) passes,
   including exact zero missed lines/branches/functions, coverage selftest,
   publish/seal proofs, pinned compilation and fail-closed native-tls absence.
   Preserve logs and candidate identity. A failing gate remains unfinished work.
   The chief's structural design check does not assert implementation completion.

## Scope and proof ownership

Preserve synchronous insert before SMTP acceptance, subscribe-before-check,
oldest-only pruning and ingest extraction (0001–0005); Store/SMTP code is outside
this patch. Preserve W = J = 2, logical byte tariff, single builder/cache,
nonblocking admission, no builder drain and dispatcher shutdown order. No
routing, dependency, lockfile, coverage-exclusion or INFO-finding repair.

CI/operator retain macOS, live branch settings, Helm/chart history and baseline,
Docker smoke, network RustSec, privileged port-80 and tag-only release proofs,
sealing and publication. Namespace work is outside scope. None becomes a local
acceptance criterion. No push, merge, tag, issue closure or decision acceptance.
