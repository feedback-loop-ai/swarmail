# 0081 — GLM Flash tests-review seat

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-26; revised 2026-10-26 after the intake-hands refusal.
- **Amends:** [0058](0058-codex-review-routing.md), only its tests-review model/effort assignment. [0057](0057-codex-delivery-recipes.md)'s intake assignments and every other route remain.
- **Source:** operator's 2026-10-26 “GLM-5.3-Flash seats (revised)” direction, supplied in the issue-0 authority packet and verified against its SHA256SUMS. This record accepts no decision.
- **Design:** [spec](../../specs/glm-flash-seats/spec.md), [plan](../../specs/glm-flash-seats/plan.md), [tasks](../../specs/glm-flash-seats/tasks.md).

## Revised operator direction and rationale

> The pinned engine refuses dsh for workspace-writing seats, so intake stays on its current models. Only the tests review member (`codex-review-tests`, read-only) of the codex, codex-careful and codex-fast recipes moves to `glm-flash`. Every other seat keeps its model.

The revised ruling document's SHA256 is
`b68560f6782247377e75e90698f95f33feef390cdbdb71ddd8c9a3cc0fbdfd9e`;
the companion README's is
`4dff0b60b60a41c282518f3440032f2900c6a447471d1379341f1a5bf389e44a`.
Both match the supplied checksums. The README retains the earlier scope
and contains no prior-run entries; the later revised ruling controls.
The original same-day ruling identified `glm-flash` as
`spark-glm/GLM-5.3-Flash-EXL3` on local Spark with no token cost. Reducing
paid tests-review use is the rationale, not a model-quality equivalence claim.
No credentials or tracked links into ignored evidence are included here.

## Earlier refusal and dated scope clarification — 2026-10-26

The initial request moved two intake offices and the tests reviewer. The
previous chief and implementer independently tested those edits with the
candidate-local pinned engine, digest
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Both intake offices resolved blocked, although `agents show` exited zero:

> the provider declares hands unsupported, so the agent's hands cannot be put in the box and the agent would run with the harness's own tools

Standard and Careful compilation consequently failed. The read-only tests
office resolved ok with empty effort; fast compiled with null Spark effort
because it has no intake. The previous attempt stopped at IMPL-BLOCKED.
These findings remain evidence of the capability boundary, not a demand
to change it. The revised ruling resolves the former prerequisite by leaving
intake on Sol and moving only tests review. It supersedes this proposal's
previous intake assignments and its statement that tests-only delivery is
incomplete. No adapter relaxation, hands removal or engine repair is needed
or authorized. The old implementation report is preserved beneath a dated supersession
notice in the [implementation record](../../specs/glm-flash-seats/implementation.md).

## Proposed ruling

1. Keep the existing `codex-review-tests` overlay of `review-tests`; select
   only `glm-flash` via dsh to `spark-glm/GLM-5.3-Flash-EXL3`, local, with
   explicit `efforts: {}`. Effective office effort is empty and compiled
   selected effort absent (omitted or null). Record **not applicable**, never an effort named
   `none`. Do not inherit the parent's Fable/GLM fallback chain.
2. Use that office in Standard, Careful and fast design/engine tests review.
   Standard retains three required results (Fable correctness, official
   Flash security, GLM tests). Careful and both fast full panels retain five
   (Astra correctness, Flash security, GLM tests, Astra guarantees and
   independent Fable). Preserve distinct charters, invocations and results.
3. Preserve Standard Sol/medium and Careful Sol/high intake with all existing
   workspace hands, permissions and limits. Fast has no intake and keeps
   Opus/medium triage without fallback. Every implementer, shared design,
   legacy self route, other model/effort and selector stays. Fast chore
   keeps one combined Flash reviewer; fast feature keeps Qwen/Flash and
   the sole authorized Qwen-to-Sonnet fallback on pre-result dispatch failure.
   DeepSeek Pro remains forbidden. No tests member is added to those lanes.
4. Tests review retains gate/read-only classification, inherited charter,
   limits, permissions, separate result and candidate-change refusal. No
   hands/tools are added. Keep dsh's unsupported native permissions/MCP
   declarations truthful; do not claim equivalent native sandbox enforcement.
   Protected review, hard security hold, bounded returns and all deterministic
   gates remain. Frozen application guarantees and extraction-at-ingest
   remain untouched.
5. No application source, dependency/license policy, engine pin, adapter
   capability, fallback framework, retry policy, benchmark, live endpoint
   probe or credential change belongs to this amendment. A Spark outage
   can stop review; it does not authorize a paid substitution.

## Proof and consequences

Extend the existing Rust source and Python effective-route proofs. Require
exact identity, concrete/local route, singleton chain, successful status,
empty office efforts and null compiled effort; feed negative mutations
through the same comparator. Preserve complete recipe membership and compare
non-target source/resolved fields to baseline d55a89d. Compile all four
bundles using the digest-matched local engine without policy-sweep warnings,
and run every local house gate including exact coverage and selftest on
committed work. Synthetic negative controls are proof of assertion sensitivity,
not live provider dispatch.

Keep full dispatcher and focused resolution results separate. The baseline
full script fails at `1-cross-attempt` because the engine completes instead
of refusing; subsequent cases do not execute. Preserve that assertion and
report the actual outcome. No engine repair, re-pin, gate waiver or claim
of dispatcher-wide safety follows from focused routing success.

**Engine re-pin result (2026-10-29).** The engine is now re-pinned to brokkr
`a9893b4` + three patches (binary `4873928c…`, which includes brokkr's
attempt-correlation fix, decision 0096; see `.brokkr/recipes/self/tooling.json`).
Run against that engine at `2c000e8`, the full routing proof
`python3 tests/codex_review_routing.py --engine <brokkr 4873928c…> --out <dir>`
exits 0. Every `cross-attempt`, `cross-effect`, `missing` and `edit` case
(panels of 1, 2, 3 and 5) parks `awaiting_operator` instead of completing.
The `1-cross-attempt` failure described above was the old engine's
(`731189f`, `ab93555…`). The script now refuses that engine as unrecorded,
so that failure can no longer be reproduced at this pin. A candidate cut
before the re-pin still pins the old engine until it merges this change.

This design adopts one shared office and stronger local comparisons instead
of another routing abstraction. Costs are additional exact assertions, paid
intake retained, no new provider-quality evidence and possible Spark-outage
stoppage. CI owns the existing platform/network/release checks; the operator
owns live Spark availability, credentials, model choice and sealing. Do not
probe the endpoint from a seat or add network/namespaces acceptance criteria.

The implementation updates the existing index row and current route summaries,
and labels superseded evidence. Accepted 0057/0058 bodies remain historical;
any pointer is a dated addendum. This explicit operator direction authorizes
only the narrow tests substitution while 0081 remains proposed; no unrelated
acceptance, push, merge, tag, publication or issue closure is authorized.
