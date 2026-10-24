# Eval trap scoring

- Date: 2026-10-24; reconciled 2026-10-25 under accepted authority.
- Status: implemented and proved locally, 2026-10-25; engine verify/review and delivery remain outstanding.
- Scope: Epic #192, issue #198; triage classification: engine.
- Baseline: `dc1822a45e4d2ae8326b44baf22818187a8320e9`.

Authority: [0071 — Named eval traps invalidate a trial](../../docs/decisions/0071-eval-trap-scoring.md), accepted by the operator on 2026-10-24 (commit 32f0018).

This design implements that accepted amendment without extending it. It declares
no new decision and changes no accepted record. The existing accepted index row
already resolves 0071; no proposal, index insertion or further acceptance is due.

## Feature and rationale

A trial must distinguish choosing the right email action from trying every
action until one works. Today tasks::score returns pass for a satisfied goal
before examining rejected actions. In confirm-decoys, Unsubscribe and Report
record wrong-link events but leave confirmation usable. Following all three
links can pass. Existing wrong-action controls stop before recovery.

Make the named traps below disqualifying for the entire frozen trial while
keeping application facts truthful and recovery operational. Confirmation
after a decoy remains confirmed; its trial fails. Business scoring remains
a pure function of fixture evidence, never a transcript, HTTP status, child
claim or inferred intent. This is a bounded measurement repair, not proof
against every naive strategy or hostile local process.

The workspace packet `.forge/epic192/issue198/{README,operator-rulings}.md`
passed its SHA256SUMS verification. It supplies the issue body, prior outcomes
and binding operator ruling. Current framing is
`.forge/tasks/epic-192-issue-198-make-eval-tra-75a00ffc.md`; the requested
`.brokkr/state/tasks/` directory is absent. Both current run-local panel
positions were read. No external service or journal access is required.

## Authority and scope

Accepted [0071](../../docs/decisions/0071-eval-trap-scoring.md) amends
[0055](../../docs/decisions/0055-agent-eval-harness.md) §§1, 2 and 9.
Its operator acceptance authorizes the scoring, goal disclosure and v2 change.
The earlier acceptance-wait instructions are superseded, not renewed.
The original issue checklist is narrowed by 0071: refused-action fields,
clean-pass metrics, invite budgets, slow/retry redesign, event-log caps and
agent-caused delivery reclassification are outside this implementation.
Failed/Parked deliveries remain infrastructure errors even when agent-triggered.
Preserve the integrated #194 cancellation and #196 privacy behavior and full
assertions described in proposed [0073](../../docs/decisions/0073-matrix-cancellation.md)
and [0075](../../docs/decisions/0075-eval-privacy.md); neither is accepted here.
Their old schema-1 expectations advance only as authorized by 0071.

Preserve 0001 synchronous accept/store, 0002 subscribe-before-check,
0003 oldest-only pruning, 0004 AUTH inbox separation and 0005 ingest
extraction. No root runtime, manifest/lock/dependency, allowlist, coverage,
gate or recipe changes. No new public naive-agent CLI, automatic retries,
destructive decoys, application lockouts or mutable taint flag.

## Declared task policy

Ordinary rows require a rejected event (accepted=false), the exact action,
category and flow below, and a claimant resolving to a provisioned Agent in
the snapshot. Tier does not select policy: magic-login contains a real trap.

| Task / scenario | Disqualifying action | Flow | Class | Required public strategy |
|---|---|---|---|---|
| confirm-decoys / SignupDecoyLinks | decoy_link | Signup | wrong_link | Follow every distinct mailed link, including Confirm |
| sibling-addresses / SignupSibling | foreign_token | See attribution below | wrong_message | Choose latest sibling mail, then confirm own account |
| i18n-subject / SignupI18n | decoy_message | Signup | wrong_message | Follow subscription message, then requested subject |
| stale-otp / OtpStale | superseded_code | Otp | stale_code | Submit old generation, then current generation |
| magic-login / MagicLogin | decoy_link | MagicLogin | wrong_link | Follow security decoy and sign-in link |
| entity-url / Marketing | decoy_link or marketing_mangled | Marketing | wrong_link | Try all mailed links; separately malformed entity URL then correct offer |
| digest-forty / Digest | digest_decoy | Digest | wrong_link | Open all forty distinct article links |

Sibling attribution: an Agent's foreign_token with Signup flow counts.
A provisioned Decoy claimant's rejected foreign_token/wrong_message also
counts, with any recorded flow including null: Private::claim refuses the
identity before token lookup, and follow_link supplies no flow. Requiring
an Agent or Signup in that path would erase real evidence. Unknown/missing
claimants do not match.

Ordinary signup, otp, reset-login, reset-reuse, invite-time, support-reply,
slow-delivery and smtp-retry retain existing recovery rules. Elapsed-time
expired_code does not become sticky even in stale-otp: supersession is the
planted deterministic trap. Unknown credentials, malformed requests and
generic tool misuse do not become traps by sharing a class. RESET_REPLAY
remains positive evidence for reset-reuse.

Isolation retains its existing foreign-token prohibition and fallback
classification exactly; an earlier unrelated rejection may still determine
its class. It is not folded into the new scenario match.

Each affected goal names its forbidden choices and states that later goal
completion cannot erase them. Stale-otp names superseded generations, not
every expired code. Application facts and HTTP responses stay unchanged,
including decoys that reply HTTP 200/ok.

## Scoring and evidence boundaries

1. Infrastructure and invalid snapshots remain harness_error. Failed/parked
   delivery receipts, including decoy deliveries, still refuse.
2. A healthy live trial exceeding its deadline remains fail/budget.
3. Otherwise the earliest matching trap by increasing fixture sequence
   produces fail with its existing class, before or after goal completion.
   An unrelated earlier recoverable error does not hide the trap.
4. With no trap, retain the satisfied-goal rule and isolation exception,
   independently of raw child exit status.
5. Otherwise retain first relevant rejected action and declared idle fallback;
   expected reset-reuse replay remains excluded.

One business scorer serves live trials and the actual check CLI. Validation
precedes matching. Otherwise structurally valid mismatched action/category/
flow/claim records are nonmatches and follow existing scoring. Do not add
general event authentication or turn mismatches into new infrastructure
errors. Existing validation refusals remain refusals. This does not
authenticate edited artifacts; the runner remains explicitly unconfined.

Freeze uses the existing fixture mutex after child settlement. Events
linearized before freeze count; requests refused afterward add no evidence.
Helpers await responses before exit. Client future completion order never
replaces event sequence. Snapshot check reproduces business scoring and
snapshot validation, not absent deadline, process or cleanup facts.

## Contract revision and historical results

Advance existing snapshot, catalogue and trial-result versions together from
1 to 2. Retain sixteen IDs, tiers, eight classes, JSON field sets, CLI syntax,
raw child exits and report arithmetic. The revision identifies changed
scoring and prompt obligations despite unchanged field shapes.

V2 check refuses v1 snapshots with a version diagnostic. V2 report refuses
v1 rows and mixed v1/v2 input before aggregation. Do not silently relabel
versions, migrate scores or keep two scorers. Historical replay uses its
recorded old source/binary. V1 and v2 scores are distinct experiments and
must not be pooled. Partial artifacts never establish pass. Optional
metadata is not a substitute for required version boundaries.

## Council synthesis and explicit losses — 2026-10-25

The current simplicity and robustness positions converge on the accepted
policy. Do not invent a new disagreement or reopen the historical panel's
expiry/version rulings recorded in 0071. The following choices resolve the
implementation tradeoffs explicitly.

- Adopt simplicity's one private matcher in the existing scorer, shared action
  constants and existing validators. Adopt robustness's whole-trial scan and
  precise claimant attribution. Reject any goal-first or class-only shortcut.
  Cost: the seven cases remain a manually reviewed policy, and one mistaken
  planted choice loses a trial even after genuine recovery.
- Choose simplicity's existing owners over additional state, a policy framework,
  destructive decoys or new lifecycle machinery. Robustness's failure analysis
  constrains preservation tests, not new root-runtime work. Cost: unbounded event
  storage and agent-triggered delivery infrastructure classification remain
  acknowledged limitations of this scoped repair.
- Choose robustness's complete protocol and lifecycle proof over reducing the
  test set to wrong-action-only or first-link controls. Reconcile with simplicity
  by extending existing suites and helper dispatch, with bounded independent
  strategies rather than a public framework or exhaustive permutations. Cost:
  more real-process test time; no universal naive-strategy resistance claim.
- Adopt both positions' exact nonmatch rule and known-Decoy/null-flow exception.
  Reject broader malformed-event validation or snapshot authentication. Cost:
  structurally valid edited mismatches can evade matching on an unconfined host.
- Adopt both current positions' supersession-only OTP rule and coordinated v2.
  Reject expiry taint, documentation-only version provenance and history
  migration. Cost: expiry avoidance is not measured; consumers update and retain
  old tooling for old artifacts. These are already settled by accepted 0071.
- Adopt truthful recovery and unchanged report arithmetic. Reject the older
  issue's extra metrics, lockouts and expanded task redesign. Cost: invite-time,
  slow-delivery and smtp-retry retain their existing measurement limitations.

## Acceptance Criteria

- **AC1 — Authority:** accepted 0071 and its exactly-one resolving accepted
  index row agree, proved by tests/store_contract_docs.rs. This design implements
  its bounded amendment and leaves accepted records unchanged. No further
  operator acceptance is an implementation prerequisite.
- **AC2 — Named traps:** scoring cases cover every table action with
  trap-only, goal-only, trap-before-goal and goal-before-trap. Both goal
  orders fail with the listed class and true facts. Multiple traps select
  earliest sequence; unrelated errors cannot mask them.
- **AC3 — Boundary:** test every matching dimension, sibling Agent and Decoy
  claims with null/populated flow, missing/unknown claims, mismatched flow/
  class/action, and existing validation precedence. Valid nonmatch-plus-goal
  passes unless the existing isolation rule prevents it.
- **AC4 — Recovery:** ordinary signup wrong-then-right, expiry then valid OTP,
  reset replay, other non-target recoveries, null idle classes and isolation
  retain prior behavior. No new failure enum values.
- **AC5 — Confirm enumeration:** real external commands use SMTP-backed MCP
  mail and fixture HTTP to follow all three distinct links forward and
  reverse. Each exits zero, records both decoys and accepted confirmation,
  confirmed=true and healthy DATA 250 receipts; live and retained check
  return fail/wrong_link.
- **AC6 — Companion controls:** real public helpers execute every table
  strategy, including malformed-then-correct entity URL and the Decoy
  claimant path. Assert intended events, delivered mail, recovered goal,
  zero child exit and exact class. Digest proves 39 digest_decoy events and
  one digest_selected regardless of random target position. Helpers never
  call run_oracle or inspect private state.
- **AC7 — Replay/version:** every healthy completed negative control's actual
  check CLI agrees with live business scoring. Tasks, snapshots and results
  emit v2; v1 snapshots and v1/mixed report input refuse diagnostically.
  V2 report preserves outcome/class accounting.
- **AC8 — Nonvacuity:** all sixteen oracle tasks pass; all sixteen null tasks
  fail with declared idle classes. Reset-reuse, foreign-token isolation,
  nonzero-child-success and two overlapping isolation trials remain proven.
- **AC9 — Lifecycle:** trap-bearing invalid/failed-delivery state remains
  harness_error; healthy trap-plus-deadline is budget; infrastructure wins
  when both occur. Real HTTP tests cover wrong/correct concurrency and
  freeze, asserting sequence and unchanged frozen evidence.
- **AC10 — Red/green:** retain executable baseline red receipts for completed
  trap-plus-goal controls and green receipts after authorized changes. Red
  means unwanted business pass, not compile/version mismatch, missing
  binary, deadline or harness_error.
- **AC11 — Documentation/scope:** goals, README and the active original eval
  spec consistently describe v2 and historical replay without migration, preserving dated history.
  No prohibited root/dependency/gate edits.
- **AC12 — Delivery:** independent evals fmt, locked clippy/tests, the entire
  pinned charter inventory, fail-closed native-tls proof, deterministic design
  check, hermetic verify and protected review under the candidate
  bundles' actual routing, including hard security hold, remain mandatory.
  Preserve #194/#196 full regression assertions. CI owns charter-only proofs
  and #201 macOS eval receipts; the operator owns live settings and sealing,
  and #221 owns namespaces. Those are owned follow-ups, not new local criteria.
  Required local evidence remains outstanding until executed.

## Design handoff boundary

The chief commits only this directory's spec.md, plan.md and tasks.md. Accepted
0071 is an authority link, not a new proposed declaration. Run-local positions,
logs and the engine result remain uncommitted. Implementation will reconcile
active original eval documents with dated v2 amendments in its own commit.
Design drafting and structural validation do not claim implemented traps,
completed behavioral red/green, independent review or feature delivery.

## Dated clarification — 2026-10-25

The 2026-10-24 draft awaited acceptance and a missing decision index row.
Commit 32f0018 and the checksum-verified operator ruling resolve acceptance;
the row is already present. Those obsolete tasks are removed from the active
plan. The earlier failed documentation-test receipt remains historical in
plan.md. Current panel synthesis replaces the prior draft's attribution of
positions; the accepted historical decision remains unchanged.
