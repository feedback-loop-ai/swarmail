# Eval trap scoring

- Date: 2026-10-24.
- Status: design draft; implementation and delivery remain outstanding.
- Scope: Epic #192, issue #198; triage classification: engine.
- Baseline: `dc1822a45e4d2ae8326b44baf22818187a8320e9`.

Decision: 0071 — [Named eval traps invalidate a trial](../../docs/decisions/0071-eval-trap-scoring.md), proposed.

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

The intake title, framing and both panel files are the available issue
evidence. No external issue body or newly executed reproduction is claimed.

## Authority and scope

Accepted [0055](../../docs/decisions/0055-agent-eval-harness.md) §2 gives
satisfied goals precedence, except isolation, and §§1/9 define v1 contracts.
0071 proposes an amendment. Per
[0045](../../docs/decisions/0045-delivery-repairs.md) §8 and the
[charter](../../docs/house-rules.md), incompatible scorer, prompt and version
changes wait for explicit operator acceptance. Design and compatible proof
scaffolding may proceed. Neither this draft nor green tests accepts 0071.

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

## Council synthesis and explicit losses

- Adopt both positions' event policy, truthful facts, one scorer, shared
  action constants, disclosed goals and completed naive controls. Reject
  blanket fail-on-rejection and destructive decoys: they punish required
  reset replay or conceal what the experiment measures.
- Rule for robustness on OTP expiry; reject simplicity's sticky expired_code.
  Cost: recovery after genuine expiry is allowed and this benchmark does not
  measure avoidance of every stale submission. Supersession avoids timing
  dependence.
- Rule for robustness on v2; reject simplicity's documentation-only revision.
  Existing metadata records package version and binary path, not a required
  scoring revision. Cost: consumer updates and archived historical replay;
  benefit: mixed report input refuses without a migration framework.
- Rule for simplicity on exact matching; reject robustness's expanded
  malformed-known-trap validation. Cost: a structurally valid edited mismatch
  may disappear from matching and a satisfied goal may pass. Adversarial
  artifact integrity remains outside the accepted unconfined model.
- Reconcile attribution with actual code: known-Decoy claim refusal can have
  null flow. Neither position's shorthand flow table overrides this producer.
- Adopt robustness's freeze/order proof using existing synchronization and
  simplicity's bounded proof set. Reject exhaustive permutations, model
  sampling, new seams/frameworks and configurable policies. Cost: evidence
  establishes the listed strategies, not universal trap resistance.

## Acceptance Criteria

- **AC1 — Authority:** this design's 0071 is proposed. Incompatible work has
  explicit operator acceptance before proceeding. Accepted records remain
  unchanged. The subsequent index row resolves once and matches actual
  decision status, proved by tests/store_contract_docs.rs.
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
  spec consistently describe v2 and migration, preserving dated history.
  No prohibited root/dependency/gate edits.
- **AC12 — Delivery:** independent evals fmt, locked clippy/tests, the entire
  pinned charter inventory, fail-closed native-tls proof, deterministic design
  check, hermetic verify and four independent engine reviewers with hard
  security hold remain mandatory. Preserve CI-only obligations; missing
  evidence stays outstanding.

## Design handoff boundary

This chief commits exactly spec.md, plan.md, tasks.md and 0071. The explicit
four-file limit excludes docs/decisions/README.md although root inventory
tests require its row. The next implementation/integration seat must add that
row as its first compatible documentation change before any full gate success.
This known gap is not waived: design validation is not feature verification.
Acceptance, implementation, red/green, full gates, review and ship remain open.
