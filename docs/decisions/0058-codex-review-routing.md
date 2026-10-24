# 0058 — Exact Codex reviewer routing amendment

- **Status:** accepted (operator, 2026-10-24)
- **Date:** 2026-10-24
- **Amends:** [0057](0057-codex-delivery-recipes.md) §§2, 6 and the role-count reference in §8, solely for the operator-directed Codex reviewer matrix and fast-feature dispatch fallback.
- **Source:** current operator directive, run `update-the-codex-delivery-recipe-c79c3f2d`; this record does not invent an earlier fallback acceptance date or receipt.
- **Design:** [spec](../../specs/codex-review-routing/spec.md), [plan](../../specs/codex-review-routing/plan.md), [tasks](../../specs/codex-review-routing/tasks.md).

## Context

0057 currently records four full-review roles, Qwen/Opus feature review and
single-model Codex review chains. The operator now explicitly directs the
following replacement matrix while keeping Codex implementation, triage-first
small lanes, all protected shell gates and hard security hold. This proposal
records precisely that amendment; drafting it does not accept it. 0057's
historical accepted text and unrelated clauses remain as recorded.

## Proposed ruling

| Composition | correctness | security | tests | guarantees | independent |
| --- | --- | --- | --- | --- | --- |
| Standard (`codex`) | Claude Fable 5.1 | official DeepSeek Flash | Claude Sonnet 5.5 | absent | absent |
| High-risk (`codex-fast` design/engine) | Codex Astra | official DeepSeek Flash | Claude Sonnet 5.5 | Codex Astra | Claude Fable 5.1 |
| Careful (`codex-careful`) | Codex Astra | official DeepSeek Flash | Claude Sonnet 5.5 | Codex Astra | Claude Fable 5.1 |
| Fast feature | DashScope Qwen Max, then Sonnet 5.5 only on authorized dispatch failure | official DeepSeek Flash | absent | absent | absent |
| Fast chore | Existing single official Flash reviewer covers all four concerns | combined | combined | combined | absent |

High-risk names existing fast full-risk strategies; no new recipe or strategy
is introduced. Careful retains Sol xhigh implementation. Every implementer in
these Codex recipes remains Codex; existing work/design choices otherwise stay.
The two Astra roles are separate offices with distinct correctness/guarantees
charters, invocations and results. Independent Fable is another required
invocation and verdict; it may inherit the full-diff correctness charter through
a small named alias, with no security-hold authority. Removing Standard's
dedicated guarantees seat does not remove accepted rulings from correctness.

Model identity means the resolved adapter/provider route, not just an alias:
Claude `claude-fable-5-1` and `claude-sonnet-5-5`, Codex `gpt-6-astra`, dsh
`dashscope/qwen3.8-max`, and `deepseek-flash` through `deepseek-official`.
DashScope remains without an effort flag; other reviewer routes use high.
DeepSeek Pro is never allowed in a selected office, inherited chain or fallback.

The only Codex review fallback exception is Qwen-to-Sonnet in fast-feature
correctness for a supported dispatch failure before a valid result. It cannot
substitute after a valid verdict, erase residuals or unknown/malformed safety
facts, replace Flash security, fall back to the author, or select a third model.
Exhaustion refuses shipping. The recorded engine's actual classification and
behavior must prove this boundary; source arrays and compilation alone do not.
Unsupported semantics require a surfaced engine-proof gap, not incidental
broader fallback or a tooling-pin change. Record route/failure evidence without
credentials in existing run evidence. Present directive provenance is enough to
state the requested behavior; it is not evidence of a historical implementation.

## Preserved authority and consequences

All reviewers remain gate-class/read-only with separate result identities and
explicit scope/fixes facts. Worst verdict wins, panel security alone owns hold,
and the single chore reviewer retains combined security responsibility. Unknown
facts refuse; scope/oversized returns remain bounded across the new panel sizes.
Triage-first one-/two-review lanes, no-default strategy refusal, protected review,
verified-candidate binding, inherited design checking, hermetic verify/ship,
script/tooling identities and the complete house gate inventory remain.

No accepted application guarantee, runtime behavior, dependency/license policy,
legacy self route, release rule, journal/concurrency rule, provider or credential
changes. No push, merge or issue closure is authorized. No frozen guarantee may
be weakened. Only the operator accepts this record; no acceptance of unrelated
proposals follows from the directive or a passing design check.

Choosing existing full-risk lanes instead of a new named recipe costs three
explicit five-member panels and equality tests. The independent Fable alias
costs one small office and overlaps correctness; it avoids copying a charter or
giving another member security-hold. Two Astra roles cost two calls and retain
correlated model limits. Those costs implement the specified roles honestly;
five roles are not five independent model families. Provider outages stop all
routes without the one authorized fallback. Application hardening and a general
fallback framework are outside this amendment.

Downstream documentation appends a dated pointer to this proposed amendment in
0057 and adds its index row, preserving the historical body. Implementation
uses the exact current directive as its scope; any further contradiction of
accepted policy requires the operator under 0045 §8. The design seat records
no operator acceptance. AC1–AC11 and the paired tasks are the proof obligations;
implementation, dispatcher/evaluator proofs and full verification are outstanding.
