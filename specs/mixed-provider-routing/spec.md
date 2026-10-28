# Mixed-provider routing verification

Created: 2026-10-28
Status: Draft
Candidate under examination: `c2f76e26989bbd5eb7aabcf28f57e4ef8073d136`
Baseline: `4bc752b`, the candidate's parent
Run: `verify-and-independently-review--53f33089`
Decision: 0090 — [Separate mixed-provider route](../../docs/decisions/0090-mixed-provider-routing.md)

## Feature and rationale

Verify and independently review the operator-directed `mixed-opus-terra`
recipe for future runs. It inherits `codex-careful`, replacing only intake
with Codex Terra 6.1 and implementation with Claude Opus 5.5. The careful
five-member gate panel and every protected delivery rule remain inherited.
This gives the operator the requested work models without creating a second
policy owner or changing any existing recipe's work assignments.

The present operator directive is the authority for this additional route.
[0057](../../docs/decisions/0057-codex-delivery-recipes.md),
[0058](../../docs/decisions/0058-codex-review-routing.md) and the revised
operator direction recorded in proposed
[0081](../../docs/decisions/0081-glm-flash-seats.md) govern inheritance.
0090 records the narrow new route; it accepts no decision. Existing accepted
bodies, 0081's proposed status, earlier manifests and prior verdicts remain
historical. In particular, the Codex-only work rule for existing Codex recipes
is not reinterpreted as having always allowed this separate route.

The supplied framing agrees with `.forge/evidence/feature.txt` and intake
result `bdd06909-13c2-43b0-913b-5940bb1e13a2`. The requested legacy
`.brokkr/state/tasks/` is absent; `.forge/tasks/` contains unrelated tasks.
This provenance gap is recorded, not repaired by inventing a historical task
or journal entry. Both panel positions were read from the ignored run state.
Their `pass` results establish written positions, not completed verification.

## Synthesis and explicit rulings

1. **Adopt simplicity's two-seat overlay.** Keep named offices extending
   existing charters, the `recipes -> bundles` discovery alias, shared design,
   inherited policy and proof owners. Accept two small hands declarations
   with equality tests; reject another inheritance layer merely to factor
   them. The cost is duplicated configuration and continued coupling to
   careful, contained by exact comparisons.
2. **Adopt robustness's evidence chain.** Source intent, effective engine
   resolution, deterministic gates, actual provider/tool/result delivery and
   CI evidence are distinct. Reject any interpretation of simplicity that
   permits source-only assertions or a successful compile to finish review.
   The losing shortcut would be cheaper, but could silently count unavailable
   reviewers; the chosen rule costs five attributable results and may park.
   Neither panel actually advocates waiving live review.
3. **Require small proof additions in existing suites.** Both positions
   identify the same missing exact override/alias assertions, effective
   mixed-route comparisons and mixed-specific CI negative cases. Adopt
   those concrete additions over a conditional no-change approach. Reject
   new routing frameworks, engine repairs, provider-health services and fresh
   benchmark suites. The cost is more exact regression assertions, not more
   production abstractions.
4. **Preserve application guarantees without expanding this feature.**
   Robustness's crash, lost-wake, pruning, webhook, shutdown, hostile-input
   and certificate scenarios explain why inherited gates remain mandatory.
   They do not justify new application code or protocol requirements here.
   Accept full gate cost; reject unrelated hardening work. Existing limits,
   including best-effort webhooks and slow-session residuals, stay visible.
5. **Read-only review wins over completion pressure.** Historical 0017's
   operator-level write grant is evidence of an earlier repair, not permission
   in this run. If requested dispatch or result delivery needs broader writes,
   preserve the failure and park. No substitute model, copied verdict, missing
   reviewer, paid trial or sandbox relaxation is an acceptable reconciliation.

## Required route

Every listed chain is a singleton, selected at index zero with no skipped
alternative. High means the effective office's selected effort; tests effort
is absent, not a string named `none`.

| Seat / office | Provider / concrete model | Effort | Contract |
| --- | --- | --- | --- |
| intake / mixed-terra-intake | codex / `gpt-6.1-terra` (`terra61`) | high | work, existing boxed workspace hands |
| implement / mixed-opus-implementer | claude / `claude-opus-5-5` (`opus`) | high | work, existing boxed workspace hands |
| review:correctness / codex-review-astra-correctness | codex / `gpt-6-astra` | high | gate, correctness result |
| review:security / codex-review-security | dsh / `deepseek-flash`, `deepseek-official` | high | gate, sole panel security-hold authority |
| review:tests / codex-review-tests | dsh / `spark-glm/GLM-5.3-Flash-EXL3`, local Spark | not applicable | gate, tests result |
| review:guarantees / codex-review-guarantees | codex / `gpt-6-astra` | high | gate, separate guarantees result |
| review:independent / codex-review-independent | claude / `claude-fable-5-1` | high | gate, independent result |

Historical `terra` remains `gpt-5.6-terra`. Two Astra roles are separate
charters/invocations/results, not separate model families. Fable differs
from Opus but shares its provider. Five results do not mean five independent
providers. 0058's fast-feature Qwen-to-Sonnet exception does not apply here;
DeepSeek Pro and every other fallback are excluded.

## User scenarios and edge cases

For a future run selecting this recipe, the operator can inspect resolved
Terra/Opus work seats and the inherited five-member panel before dispatch.
For a compile failure or policy warning on the final mixed entry in CI,
the pinned bundle job fails rather than reporting the earlier recipes green.
For an unavailable Spark/Astra/Fable/Flash provider or undeliverable result,
the run retains the failure and stops without changing the requested roster.
For a changed candidate, stale/partial/foreign-attempt results cannot satisfy
the current review. An unknown availability value is an unproved fact, not
an outage finding or evidence of a successful provider invocation.

## Acceptance Criteria

- **AC1 — Scope and authority.** Record the full eight-file candidate diff
  against its parent and subsequent design/implementation commits separately.
  Mixed extends careful and its exact seat keys and override seat set are
  `{intake, implement}`. There is no copied policy, review panel or exec gate.
  Existing recipes and historical `terra` retain their behavior. Only the
  new proposal records the current directive; no existing decision is
  rewritten or accepted, and no older pinned run is claimed to have changed.
- **AC2 — Admitted engine and compile.** Before executing it, verify the
  candidate-local `.forge/bin/brokkr` digest and version against
  `bundles/self/tooling.json`. Compile all five recipes (`self`, `codex`,
  `codex-careful`, `codex-fast`, `mixed-opus-terra`) successfully without
  policy-sweep warnings. Retain candidate, engine identity and complete logs.
  A matching version alone and a sibling/PATH binary do not pass.
- **AC3 — Effective work route.** Engine office/manifest resolution and
  adapter model pins agree with the table, report `ok`, singleton selection
  and intended efforts. Check the historical alias, inherited charters,
  tools, permissions, limits and equality of work hands to careful, including
  network setting, Cargo credential masks and read-only Rustup bind. Preserve
  layer distinctions: implement office limit 14400 seconds and recipe seat
  limit 7200 seconds are recorded separately, with three seat attempts.
- **AC4 — Inherited review and gates.** Compare effective mixed and careful
  review entries, capabilities, tools/hands, boundaries, exec drivers,
  limits, shared design and source policy composition. Require exactly five
  gate roles with the table's concrete routes; tests office efforts are `{}`
  and compiled selected effort is absent/null. Preserve protected review,
  worst-verdict aggregation, bounded returns, explicit scope/fixes facts,
  unknown/malformed-fact refusal, security-only hold authority and the
  verified-candidate fence. No added reviewer writes or native capability
  claims; unsupported dsh native permissions/MCP remain truthful.
- **AC5 — Sensitive proof assertions.** Extend existing source and effective
  tests. The same effective comparator that accepts real observations must
  reject wrong concrete model/provider/route/effort/status, fallback,
  historical alias drift, added seat override, missing/merged/replaced member
  and changed hands/boundary observations. A synthetic rejection is only
  assertion-sensitivity evidence. Preserve every existing dispatcher check.
- **AC6 — CI coverage.** The pinned bundle job includes mixed in its exact
  five-recipe invocation list and still admits digest/version before use.
  Existing CI fixtures reject omission, mixed compile nonzero exit and mixed
  stdout/stderr policy warnings, as well as prior identity failure cases.
  Rename the stale four-recipes test description when extending it.
- **AC7 — Local gate evidence.** Execute the complete current inventory in
  [house rules](../../docs/house-rules.md#gates--part-of-every-change), offline,
  installing nothing, with locked Cargo dependencies and the admitted engine.
  Check shim hashes and execute pinned proof owners. Require exact zero missed
  lines, branches and functions, coverage selftest, design-check selftest,
  delivery tamper-boundary proof, publish guard, seal proof, licenses and
  fail-closed native-tls absence. Report each failure/unexecuted check; no
  floor reduction, exclusion or allowlist widening. Final evidence binds to
  the final committed candidate, not just the original route commit.
- **AC8 — Real execution and review evidence.** Retain actual engine receipts
  identifying requested work-provider invocation, boxed work-tool use and
  consumed result where claimed. Obtain five distinct attributable review
  invocations/results through the existing delivery sequence on the verified
  candidate. Record resolved route versus provider-reported identity honestly;
  compile metadata is not a provider-served identity receipt. Reviewers read
  the candidate and deliver only their own result through the existing allowed
  path. If any required evidence is unavailable or delivery requires broader
  writes, preserve the exact failure and park; never write a review verdict
  for another seat. Preserve every residual and security hold in aggregation.
- **AC9 — Residual and ownership accounting.** Run focused resolution and
  the full existing dispatcher proof separately. Preserve proposed 0081's
  reported `1-cross-attempt` failure (engine completed where refusal was
  expected) until a fresh authorized execution establishes its status; list
  cases not reached. No assertion weakening or engine repin discharges it.
  Keep CI-owned platform/port-80, Helm/chart baseline/history/selftests,
  Docker persistence/context, network RustSec and separate tag rate/release
  checks marked CI-owned, with receipts if available, never locally inferred.
  No standalone eval suite starts without explicit same-user non-overlap
  coordination; this feature requires no new benchmark or paid trial.
- **AC10 — Delivery discipline.** Commit the design as exactly these three
  spec files and proposed 0090, unsigned, with the design commit last for the
  deterministic check. Later implementation updates the decision index and
  runs verification/review through the policy. Preserve frozen 0001–0003 and
  extraction-at-ingest 0005. No push, merge, publication, tag, issue closure,
  decision acceptance, credential changes or review-boundary widening.

## Evidence state at synthesis

The panel reports digest-matched brokkr 0.12.0, mixed/careful compilation with
76 valuations over 14 groups and zero unruled, and seven offices resolving
`ok` with provider presence `unknown`. The chief independently checked the
binary digest/version. These are structural observations. Full gate outcomes,
actual requested-provider delivery and independent review remain separate
obligations; this draft is not a verification or shipping verdict. Any chief
checks are retained in ignored run evidence and do not mark later tasks done.
