# Require the missing merge checks

Created: 2026-10-24. Status: draft for scoped implementation.
Issue: [#205](https://github.com/feedback-loop-ai/swarmail/issues/205), epic #192.
Run: `triage-and-continue-epic-192-iss-9881cf64`.

## Feature and rationale

Add exactly `brokkr bundle (pinned)` and `docker smoke` to the required checks
on `feedback-loop-ai/swarmail` branch `main`, preserving every existing
requirement and configured producer identity, strict mode, admin enforcement
and unrelated protection. Read back the effective state and document its
complete merge-blocker inventory in the existing house-rules Gates section.
Both CI jobs already exist. Requiring their results closes the reported
configuration gap without introducing another delivery system.

The task supplies confirmed admin access and both omissions. Chief verified
the exact CI names and inspected prior failure evidence, but holds workspace
capability only: no current raw protection/permission receipt is available
here and no live setting was inspected or changed by chief. Supplied findings
are a reason to inspect, not a replacement for a fresh complete baseline.
Missing external capability remains a downstream prerequisite, not permission
to seek host credentials, use an ungranted network path or fabricate evidence.

This is compatible enforcement of existing obligations under accepted
[0007](../../docs/decisions/0007-hermetic-gates-single-model.md),
[0045](../../docs/decisions/0045-delivery-repairs.md) and
[0050](../../docs/decisions/0050-delivery-gates.md). It changes no frozen
guarantee, public mail contract or accepted ruling and declares no new ADR.
Preserve [0056](../../docs/decisions/0056-signing-policy.md),
[0057](../../docs/decisions/0057-codex-delivery-recipes.md) and accepted
[0058](../../docs/decisions/0058-codex-review-routing.md) verbatim. If execution
reveals a required contradiction, return to design for a proposed amendment;
only the operator may accept it. No frozen guarantee may be weakened.

## Dated scope clarification — 2026-10-24

The current intake supersedes execution scope from run
`epic-192-issue-205-continuation--a4c15323`. The broader design remains in Git
at `bb8b040:specs/required-merge-checks/`. Its prior AC3–AC7 and integrated
completion are not satisfied or erased by this continuation:

| Historical obligation | Current disposition |
|---|---|
| Old AC3–AC4: identify, repair, review and activate the actual delivery watcher; prove complete/current readiness | Outstanding, outside this run; unknown ownership no longer blocks the two-context settings repair |
| Old AC5: genuine Brokkr-only red PR and attributable server refusal | **PENDING**; current no-throwaway-PR/no-push/no-merge charter prevents execution |
| Old AC6: separate genuine Docker-only red PR and attributable server refusal | **PENDING** for the same reason |
| Old AC7: durable probe receipts and verified cleanup | Unproved historical obligation; this run creates no probes or refs and performs no cleanup of others' resources |
| Old AC8: root integration/CI and complete issue evidence | Outstanding; local configuration verification does not discharge it |

No throwaway PR, remote ref, push, merge, bypass, release or issue closure is
authorized. No watcher discovery/implementation/activation is an execution
task in this scoped run. The older probe authorization is obsolete. Keep
#205 open and distinguish scoped configuration verification from full issue
completion. API read-back cannot prove either live refusal experiment.

## Council synthesis and rulings

Read both current run-local positions, the current framing and prior proposal.
The panel largely agrees; do not manufacture disagreement to average away
its common requirements. The chief rules the remaining scope and cost choices:

| Choice | Ruling and cost |
|---|---|
| Scope | Adopt simplicity's bounded settings/documentation repair and robustness's explicit historical disposition. Reject the older watcher/probe execution plan. Cost: watcher behavior, live refusal and integrated issue completion remain unproved. |
| Preservation | Adopt both positions' context/application union. Robustness makes the writable branch set distinct from effective ruleset contributions; this is mandatory. Reject app-flattened string append and copying the effective union into branch protection. Cost: more careful identity/source inspection. |
| Concurrency | Adopt one coordinated writer plus an immediate baseline re-read. Read/compare alone is insufficient against another writer. Reject a new lock service or drift reconciler. Cost: a reserved administrative interval and residual race against an uncoordinated writer; no claim of GitHub compare-and-swap. |
| Recovery | Adopt robustness's small run-local intent/receipt record and fresh reconciliation after interruption. Reject blind retry and full-snapshot rollback. Cost: unknown outcomes can delay completion; no permanent workflow engine is introduced. |
| Proof machinery | Adopt simplicity's existing jobs/tests and run-local semantic comparison, including robustness's negative controls. Reject a new CLI, GitHub App, mock service, aggregate CI sentinel or permanent validator. Cost: later repairs must repeat deliberate evidence collection. |
| Delivery bar | Adopt both positions' full gate inventory and 0058 five-result engine review. Reject treating a small settings diff as a cheap lane. Cost: full verification and review even without Rust changes. |

## Requirements

Let `B0` be the complete writable branch required-check set immediately before
mutation, retaining exact context strings and configured application constraints.
Let `Q0` represent applicable effective ruleset requirements, retaining their
source and identities. Let `A` contain only the two exact named additions,
with producer identities established from genuine CI/check receipts and the
supported API representation. Existing unbound requirements stay unbound;
existing bound requirements keep their binding. Do not guess application IDs,
replace a binding with any-producer, or deduplicate different bindings by name.
An equivalent existing addition is a no-op; conflicting bindings require
reconciliation before writing.

The writable postcondition is `B1 = B0 union A`. Rulesets stay unchanged:
`Q1 = Q0`. The effective result is the baseline effective set plus only `A`;
retain which policy source imposes each constraint. Do not migrate ruleset
requirements into branch protection. Fresh baseline drift invalidates a
prepared request: recompute and inspect scope under the same coordination.
An unreadable, malformed or incomplete response means unknown, never an empty
set. Confirm admin permission, `strict=true` and `enforce_admins=true` before
writing; discrepancies require resolution, not an out-of-scope repair.

Use a supported operation scoped to required status checks; preserve strict
when the endpoint requires it in the payload. Do not replace full protection.
Read full protection and effective rules back and compare all semantic fields.
Normalize ordering and transport metadata only, never context case/spacing,
producer constraints or policy ownership. Preserve all unrelated settings.
A failed or ambiguous update/read-back requires fresh remote reconciliation,
not a blind retry or stale snapshot restoration. Re-read at final handoff;
receipts are dated observations, not a continuous drift guarantee.

Only after verified read-back, record the complete resulting exact effective
blocker set and relevant producer constraints under
[house rules — Gates](../../docs/house-rules.md#gates--part-of-every-change).
This is the sole maintained inventory. Keep local commands, PR merge blockers
and CI/tag-only obligations distinct; workflow job enumeration is not the
baseline. Do not add tag-only rate/publication requirements or rename jobs.

## Acceptance Criteria

- **AC1 — Authoritative baseline.** Retained timestamped nonsecret raw receipts
  identify the repository/branch, admin actor permission, full branch protection,
  applicable effective rules and genuine producers for both names. All pages
  and context/app constraints are accounted for; strict/admin are true.
  Unknown access/state prevents mutation and remains explicitly outstanding.
- **AC2 — Lossless scoped operation.** A recorded single-writer interval,
  immediate pre-write read and intended narrow request prove `B1 = B0 union A`
  and unchanged `Q0`. Every old identity survives, both exact names are covered,
  equivalent presence is idempotent and drift triggers reconciliation. No
  unrelated protection, permission, review, bypass or ruleset field changes.
- **AC3 — Verified read-back and recovery.** Raw response, full/effective
  read-back and semantic comparison establish the intended union and unchanged
  strict/admin and unrelated fields. Negative controls reject either missing
  addition, removed old tuple, altered app binding, strict/admin change,
  unrelated change and stale baseline. An already-applied control passes.
  Interruption is reconciled from fresh remote truth; unavailable read-back is
  recorded UNKNOWN, never verified. A final re-read confirms the recorded state.
- **AC4 — One accurate inventory.** The exact effective blocker set in
  `docs/house-rules.md` Gates equals the verified read-back with relevant app
  constraints. A retained set comparison proves equality. Existing command
  gates, CI responsibilities and tag distinctions remain intact.
- **AC5 — Honest scope and evidence.** Run evidence retains prior failures,
  settings request/response/read-back and comparison provenance without secrets.
  Both independent live refusal proofs remain explicitly PENDING; historical
  watcher/cleanup/integration obligations remain unclaimed. No prohibited
  remote action occurs and #205 stays open. No live protection claim rests
  solely on YAML, an all-green CI run or a generic BLOCKED result.
- **AC6 — Required delivery proof.** The final implementation candidate passes
  every gate in the authoritative house inventory, pinned hermetic verification
  and the triage-selected engine review under 0058: distinct Astra correctness,
  official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees and
  independent Fable 5.1 results. No engine-lane fallback or DeepSeek Pro.
  Preserve worst verdict, protected review and hard security hold. CI-only
  obligations and unexecuted proof stay separately outstanding; this design
  draft claims no implementation, gate, reviewer or issue-completion success.
