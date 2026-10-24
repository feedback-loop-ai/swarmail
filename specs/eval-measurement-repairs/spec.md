# Repair eval measurement accounting

- Date: 2026-10-19; design handoff for epic #164, #148.
- Baseline: signed `4dac267`, following signed `fd66988`, on `epic164-harness`.
- Scope: the three resume findings only; implementation remains pending.
- Existing authority: [0055](../../docs/decisions/0055-agent-eval-harness.md)
  (accepted by the operator on 2026-10-19) and the [original specification](../agent-eval-harness/spec.md).

## Feature and rationale

Restore the documented infrastructure precedence and matrix accounting before
the harness measures agents. A fixture that cannot deliver its mail must not
be counted as agent success, inactivity or budget exhaustion. A matrix whose
Swarmail binary is unavailable must retain every requested observation as a
harness error. Separately, repair the standalone crate's formatting failure.

This is enforcement of existing contracts, not a semantic change to a frozen
guarantee, public schema or ruling. 0055 sections 2, 3 and 9 already require
infrastructure precedence, snapshot-based scoring and complete matrix rows.
No new decision or amendment to accepted 0055 is required. This specification
clarifies those requirements without accepting that decision on the operator's
behalf. The code's current behavior is the defect, not a competing contract.

## Requirements

**R1 — Delivery eligibility and precedence.** After provisioning and agent
execution, inspect the frozen snapshot before budget or business scoring.
Any delivery whose final status is `failed` or `parked` makes the trial
`harness_error`, with failure class `harness_error` and CLI exit 2. This
includes decoy deliveries and applies even if the business predicate is
satisfied or an agent reached its deadline. Missing/malformed final state
also remains infrastructure failure. Existing process/cleanup errors retain
their infrastructure precedence and the preparation-time refusal stays.

The rule depends on final status, never on the mere presence of a non-250
attempt. `delivered` after a 451 then 250 remains eligible. `held` alone is
not infrastructure failure: unreleased slow mail still gets ordinary business
or deadline scoring. Examine all deliveries; do not infer fault attribution
from transcripts or agent behavior. Diagnostics identify the failed receipt
and its last attempt when present, without exposing secrets.

**R2 — Replay agreement.** `check` applies the same snapshot eligibility rule
as live scoring. A retained snapshot with a failed/parked delivery yields
`harness_error/harness_error` and exit 2. Preserve the actual snapshot and
child exit statuses. Replay does not reconstruct process failures or deadlines
that the snapshot does not record; only snapshot-derived classification is
promised. Keep snapshot/result schema 1 and the closed failure enum.

**R3 — Complete matrix accounting.** Once task IDs, agent specifications and
trial count define a valid product, resolve Swarmail once. If resolution
fails, emit exactly one schema-valid harness-error row per requested
`(task, agent, trial)` element in the existing traversal order, then exit 2.
Each row has the requested labels, a fresh distinct trial ID, null singular
child status, no child exits and existing unconfined metadata. Emit the
shared cause once to stderr, with only JSONL results on stdout. Launch no
trial child and perform no per-element resolution/build/retry in this case.
Invalid matrix inputs retain their existing row-less exit-2 refusal.

**R4 — Standalone formatting.** Run `cargo fmt --all` from `evals/`, preserving
the meaning of the report test assertion. Validate formatting from that
directory as well as the root; root formatting does not cover evals.

## Panel rulings and costs

Adopt simplicity's existing `tasks::validate_snapshot` error path as the
single owner of snapshot eligibility, reached by `tasks::score` and by the
runner before the deadline branch. Adopt robustness's exact Failed/Parked
rule, decoy coverage, replay exit 2 and infrastructure-before-budget proof.
Reject robustness's new public `delivery_failure` helper and successful
`Ok(Verdict::harness_error())` scoring result: the existing `Err` path already
maps to exit 2 in `check`. The losing design keeps structural validity and
infrastructure state conceptually separate; we accept losing that separation
and broaden validator/error documentation to mean eligibility for scoring.
We also accept a second small validation scan on ordinary live scoring.
There is no unchecked scorer or second classification API to maintain.

Adopt both panels' once-only matrix resolution, existing `refused_row` and
fresh IDs. Choose simplicity's single product traversal, retaining resolution
failure until each row is emitted, over a separate failure-only product loop.
Cost: one error/success branch inside the loop; benefit: one definition of
ordering and cardinality. Reject repeated binary resolution and a single
summary error row; either would violate the documented accounting or repeat
build work. N identical-cause rows are intentional observations.

Adopt robustness's live post-launch chaos-at-100 proof and simplicity's
additional real transport-failure fixture/replay proof. A server-death-only
runner test cannot prove the repaired branch: the existing process monitor
already classifies it as infrastructure. Reject sleep-based ordering; wait
for the public action's completion, then use a pending child for the real
deadline variant. Cost: several focused proofs instead of a lone guard test.

Accept the existing unconfined limit: an agent can induce chaos and thereby
cause a harness error; snapshot-only scoring cannot safely attribute intent.
Infrastructure counts remain visible in reports. Held deliveries racing a
freeze remain subject to ordinary scoring; this repair adds no delivery
monitor, retries, scheduler, new schema, dependency or fault-injection API.

## Acceptance Criteria

- **AC1 — Final receipts:** Pure scoring tests reject Failed/connect/0 and
  Parked/data/451 receipts before otherwise satisfied and idle business
  outcomes, including a decoy receipt. Held/no-attempt slow mail remains
  fail/wait_timeout; Delivered with 451 then 250 remains eligible. Existing
  malformed snapshot rejection still works.
- **AC2 — Real Failed receipt:** Successfully provision a real slow-delivery
  fixture, stop and reap its owned Swarmail child, then invoke public release
  and freeze. Assert the recorded Failed transport receipt, scoring refusal
  and `check` harness-error JSON with exit 2 from that actual snapshot.
- **AC3 — Live precedence:** A real trial's test agent induces DATA chaos at
  100 only after provisioning, completes a public release/resend and leaves
  an observed Parked/data/451 receipt. Both normal child completion and a
  child kept alive until its deadline yield harness_error, exit 2. Assert the
  receipt in retained state so unrelated startup/transport errors cannot
  satisfy the test. No sleep establishes delivery ordering.
- **AC4 — Replay:** Replay retained snapshots from AC3 through `check`; both
  yield the live row's outcome/class and exit 2. Live rows retain truthful
  normal/deadline child statuses. Infrastructure-free deadline tests remain
  fail/budget and ordinary pass/fail replay keeps exits 0/1.
- **AC5 — Matrix:** With a subprocess-local missing `SWARMAIL_BIN`, two tasks,
  two agent labels and two trials produce exactly eight valid JSONL rows,
  each harness_error with correct labels, distinct IDs and no child statuses;
  pair multiplicity is two and order matches the requested product. Exit is
  2 and stderr reports the missing path once. Invalid-input tests still emit
  no rows; existing mixed-result accounting continues to pass.
- **AC6 — Regression evidence:** Record relevant assertion failures on
  `4dac267` for the added delivery, deadline/replay and matrix regression
  tests, then passes on the repair. Use an isolated baseline copy without
  rewriting either existing signed commit. Compilation failure is not the
  required red proof.
- **AC7 — Controls and gates:** Evals fmt/clippy/tests pass independently,
  including all 16 oracle passes, all 16 null failures and isolation proofs.
  The unchanged full house gates pass, including exact coverage and its
  selftest. Commands and evidence requirements are in [plan.md](plan.md).
- **AC8 — Scope and delivery:** Implementation changes stay under `evals/`;
  no dependency/lockfile changes and no edits under root `src/`, `scripts/`,
  `bundles/` or `.github/`. This design commit contains exactly this spec,
  plan and tasks. Later commits append to `4dac267`, cite #148 and follow
  accepted 0056's unsigned-commit policy. Never push, merge or mutate issues.
  An unavailable gate blocks claiming implementation completion.

## Framing provenance

Read both run-local positions at
`.brokkr/state/design/positions/{simplicity,robustness}.md`.
`.brokkr/state/tasks/` is absent; the supplied resume framing and journal
intake result are the available intake evidence. Position/result files remain
gitignored and uncommitted. This artifact commits the design, not a claim
that any pending implementation criterion has passed.
