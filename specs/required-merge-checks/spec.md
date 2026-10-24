# Require the complete merge-check set

Created: 2026-10-24. Status: draft for implementation.
Issue: [#205](https://github.com/feedback-loop-ai/swarmail/issues/205), epic #192.
Run: `epic-192-issue-205-continuation--a4c15323`.

## Feature and rationale

Require `brokkr bundle (pinned)` and `docker smoke` on Swarmail's `main`,
preserving every existing required context and its configured application
identity, strict required checks and admin enforcement. Make the actual
delivery driver's merge watcher wait for that same complete set, and document
it in the existing house-rules gate inventory. A green subset cannot establish
that pinned delivery proofs or container persistence were exercised.

The continuation supplies fresh findings of admin permission, strict checks,
`enforce_admins=true` and both omissions. Those are supplied observations, not
raw API receipts collected by chief. CI already defines both exact job names.
The full live required set and the actual watcher owner/path remain unknown in
this workspace. Implementation must resolve them; neither is filled in by
inference from workflow job keys or the presence of Brokkr tooling.

This is a compatible repair of existing gate enforcement under accepted
[0007](../../docs/decisions/0007-hermetic-gates-single-model.md),
[0045](../../docs/decisions/0045-delivery-repairs.md) and
[0050](../../docs/decisions/0050-delivery-gates.md), using the full engine lane
of [0057](../../docs/decisions/0057-codex-delivery-recipes.md).
It changes no public mail contract, frozen guarantee or accepted ruling;
there is no new decision declaration. If owner discovery reveals a necessary
contradiction with an accepted ruling, return to design with a proposed
amendment before incompatible implementation. This spec grants no acceptance.

## Scope and authority

Swarmail owns its protection settings, workflow producers and house rules.
The watcher must be changed in its actual owning repository and launch path;
a separate owner change and review are required if external. The supplied
operator authorization covers the #205 settings repair and temporary proof
operations through the authorized operator/driver. Ordinary delivery seats
remain subject to [0056](../../docs/decisions/0056-signing-policy.md) and
0057: unsigned local commits, no push or merge. Chief has workspace capability
only and performs no live GitHub action.

Keep existing CI jobs and gates. No runtime, dependency, release, tag,
publisher, ruleset migration, unrelated repository setting or new delivery
service belongs here. Do not close #205 until the entire acceptance set,
including root integration and CI, is verified.

## Council synthesis and rulings

The chief read both run-local positions and both intake framings. Their
agreement on actual ownership and independent live proof is adopted.

| Question | Ruling and cost of the rejected alternative |
|---|---|
| Size of the repair | Adopt simplicity: edit the existing wait path and one house-rules inventory; no replacement watcher, reconciler, permanent proof runner or aggregate CI job. Both positions reject a new service; preserve that shared boundary. Cost: future drift still requires deliberate reconciliation and external ownership may delay delivery. |
| Concurrent settings edits | Adopt robustness: coordinate one settings writer through the existing operator/driver and compare fresh state immediately before mutation. Reject simplicity's read/compare alone as sufficient coordination: an update between comparison and write could lose another lane's requirement. Cost: a short operational reservation; no distributed locking subsystem or claim of GitHub CAS. |
| Source of required checks | Reconcile: use the owner's existing explicit set or live reader, with a reviewed baseline and a checked equality relation to effective live requirements. Do not create two alternative mechanisms. Reject silently trusting a reduced live set or `--required`. Cost: policy drift, including additional legitimate requirements, blocks readiness until reconciled. |
| Candidate identity | Adopt robustness's head/base/test-merge association, application identity, pagination and current-attempt rules. Reject literal head-SHA-only matching where GitHub runs CI on a synthetic merge commit; reject any historical-green selection. Cost: extra reads and conservative refusal of ambiguous results. |
| Restart and waiting | Keep simplicity's existing bounded polling/error conventions; adopt robustness's fresh reads after restart and a small run-local operation ledger for side effects. Reject a new durable workflow engine. Cost: interrupted operations require explicit remote reconciliation. |
| Proof and completion | Adopt both: two separate genuine red-job PRs, server refusals attributable to one context each, retained receipts, cleanup, reviewed actual watcher change and root CI. Reject source-string tests or generic BLOCKED as substitutes. Cost: CI time and an open issue whenever external proof remains unavailable. |

## Requirements

Let `R0` be the freshly captured effective required set, represented as exact
context names plus configured producer/application constraints. Let `A` contain
the two named contexts with producer identities established from genuine CI.
The intended post-state is `R = R0 union A`; an already-present equivalent
requirement is idempotent. Preserve all old identity constraints. Conflicting
identities or concurrent changes require reconciliation, never widening an
app-bound check to any producer. Effective rules include applicable rulesets;
read them without migrating or changing unrelated rules.

The watcher's reviewed expected set `E`, the documented blocker set `D` and
live effective set `R` must agree. Documentation records names and applicable
producer constraints, not every workflow job. Local gates and CI/tag-only
obligations keep their current distinct scopes. The expected set cannot shrink
merely because the server or CLI reports fewer requirements.

Ready means every expected identity has an authoritative successful current
result for the observed candidate and unchanged policy. Missing or pending
checks wait within the existing bound; failed, cancelled, timed-out, skipped,
neutral, malformed, wrong-producer or ambiguous evidence cannot yield ready.
For these ordinary required jobs, successful execution is required. Enumerate
all necessary pages and support check runs and legacy statuses when present
in the captured baseline. Reruns must resolve to current authoritative
attempts, never any successful historical run. Both push and pull-request
checks must be associated with the actual candidate; a synthetic test-merge
SHA is valid only when authoritatively tied to its current head/base pair.

Re-read PR head, relevant base and policy before returning ready. On change,
discard the observation and evaluate again. API/auth/schema/rate-limit errors,
timeout or cancellation never become ready; restart discards cached readiness.
The actual merge caller pins the observed expected head, and server-side
strict protection remains authoritative for the remaining race. Readiness is
an observation, not permanent permission to merge or an admin bypass.

## Acceptance Criteria

- **AC1 — Capture and preserve.** Retain timestamped raw before/immediate-before/
  after protection and effective-rule receipts for `feedback-loop-ai/swarmail`
  `main`, with full context/app identities and actor permission. Show coordinated
  writer ownership and a narrowly scoped settings diff proving `R0 union A`,
  `strict=true`, `enforce_admins=true`, and unchanged unrelated fields. A changed
  baseline is reconciled before writing; failed read-back stops admission and
  never triggers blind restoration of a stale snapshot.
- **AC2 — One accurate inventory.** `docs/house-rules.md` names the complete
  resulting exact blocker set and relevant app constraints under Gates. A
  retained comparison establishes `D = E = R`. Existing local, PR and CI/tag
  distinctions remain accurate; publication and tag-only rate jobs are not
  accidentally added as PR obligations.
- **AC3 — Actual owner changed.** Record watcher owner/repository, exact source
  and configuration paths, launch caller and before/after revision. The actual
  wait path has a reviewed change and owner behavioral tests. If external,
  retain the owner review/change link and proof the launched revision consumes
  it; a Swarmail-only patch cannot satisfy this criterion.
- **AC4 — Complete, current readiness.** Tests through the real owner's parser/
  wait path prove each addition omitted by `--required` cannot be ignored;
  each independently red context refuses; absent/pending and all non-success
  conclusions cannot pass; complete current success does pass. Cover paging,
  legacy statuses if used, wrong app, ambiguous duplicates, stale head and
  stale successful rerun, valid/invalid synthetic merge association, head/base/
  policy change during observation, API/auth/parse errors, bounded timeout/
  cancellation and restart with cached green. Include a regression that fails
  against the old behavior. The actual caller uses the observed expected head.
- **AC5 — Brokkr refusal alone.** A dedicated throwaway PR has the genuine
  `brokkr bundle (pinned)` job terminally red while Docker and every other
  required check and merge prerequisite pass for that exact candidate/base.
  Retain an actual server refusal attributable to Brokkr, with no bypass and
  no successful merge. A generic BLOCKED or unrelated authorization failure
  is insufficient.
- **AC6 — Docker refusal alone.** A different throwaway PR meets AC5 with only
  the genuine `docker smoke` job red and Brokkr green. Neither case may use a
  forged same-name status, another app, an extra artificial blocker or a
  production fault switch. Keep protection strict throughout both experiments.
- **AC7 — Evidence survives cleanup.** Before cleanup retain request method/
  path/nonsecret payload, response status/body/request ID where available,
  timestamps, actor/bypass posture, PR URLs/numbers, head/base/test-merge SHAs,
  exact settings and required results with app/run/attempt identities and URLs,
  refusal attribution and watcher observation. Preserve raw receipts in durable
  operator-retained evidence, with checksums and a durable handoff link. Close
  both PRs and delete only their owned temporary refs; verify closed/unmerged
  state and ref absence. Interrupted work is reconciled by run-specific names.
- **AC8 — Integrated completion.** The final integrated revision passes every
  [house gate](../../docs/house-rules.md#gates--part-of-every-change), hermetic
  verify, protected full review, owner tests and applicable root CI. Record the
  integrated SHA and required check/run identities. Re-read final settings
  after cleanup and integration, confirm strict/admin enforcement and set
  equality, and record no release or unrelated settings changes. Missing
  receipts, ownership, review, cleanup or CI keep #205 open. A design draft
  and structural validation establish none of these implementation outcomes.
