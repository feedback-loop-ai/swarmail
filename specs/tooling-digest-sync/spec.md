# Reconcile the reviewed tooling digest

Date: 2026-10-24. Status: draft. Scope: epic #192, issue #171 integration
follow-up after the reviewed #214 Brokkr main pin. Classification: engine,
as recorded by intake; this design does not select a new delivery lane.

## Feature and rationale

Restore the independent tooling identity assertion in
`tests/delivery_constitution.rs::pinned_bundle_shims_and_proof_dispatch_are_digest_bound`
by replacing its fixed expected SHA-256 for `bundles/self/tooling.json`:

- Old: `e6eaeff867eb71fab5a1ce93f4ff1d46ae9deb2944b517867787a63a966f3fa3`.
- Reviewed current bytes: `6e4d19c03dc40b59ed48c9a23156f9bd63bf3d29ecee68068a3f707a8fda507d`.

The tracked file and working file independently hash to the new value. Local
history identifies `238372dc18997fe704cb1fb4ebd2cfa2a64a61fd` as the pin change;
the task supplies its #214 review provenance. This design neither audits the
external review nor reproduces the engine build. The old literal demonstrates
an integration mismatch; it does not justify weakening the identity guard.

The only implementation change is that literal. The chief's three design
artifacts are separate from the subsequent one-literal implementation diff.
Tooling, all scripts, policy, decisions, dependencies and product code remain
unchanged. No public API, frozen guarantee or accepted ruling changes, so
no new decision is proposed or declared. Accepted
[0045](../../docs/decisions/0045-delivery-repairs.md),
[0050](../../docs/decisions/0050-delivery-gates.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md) are authorities.

## Panel synthesis and ruling

Adopt simplicity's single owning assertion, fixed literal and rejection of a
new registry, updater, manifest, abstraction or duplicate test infrastructure.
Adopt robustness's independent tracked-byte comparison, actual pinned proof
execution and distinction between identity and runtime correctness. Both
positions support the engine classification and the same implementation;
there is no genuine disagreement to manufacture between them.

Rule for minimal implementation with the existing full engine proof/review
route. Reject the tempting chore shortcut: this changes a delivery identity
assertion and accepted 0057 sections 6–7 retains full review for gate repairs.
The losing shortcut would save execution/provider time; retaining the full
route deliberately pays that cost for independent delivery-boundary review.
The chore-only single official DeepSeek Flash instruction is conditional and
its condition is not met. Preserve the configured full review mapping,
including official DeepSeek Flash; never substitute Pro or a fallback.
Chief does not dispatch reviewers or decide the next phase.

Reject dynamic/self-derived expectations, accepting both hashes, suppressing
the assertion, reverting tooling and bulk pin regeneration. These alternatives
remove review friction at the cost of weakening independent identity or
blessing unrelated changes. Accept manual fixed-pin maintenance instead.
Robustness's runtime failure analysis is preservation context, not permission
for an outbox, persistence redesign, TLS change or protocol refactor. Its
existing SMTP retry, best-effort webhook and retention limits remain unchanged;
this hash repair provides no new runtime or durability guarantee.

## Acceptance Criteria

1. **AC1 — Exact identity correction.** Independently hash tracked tooling
   bytes at implementation start and require the new SHA above, identical to
   the working file. The sole implementation diff is the old-to-new literal
   replacement in the named test. If the input has moved again, reconcile its
   provenance and scope before changing any expectation.
2. **AC2 — Preserve the guard.** All six other fixed resource digests, exactly
   seven shim paths and their comparisons, every assertion and helper remain
   byte-for-byte unchanged. Tooling, canonical/public scripts, decisions,
   policy, dependency inputs and product files have no implementation diff.
   The expected tooling hash stays a single fixed literal: stale or altered
   bytes still mismatch. Prove preservation by the exact replacement diff
   and independent hash checks, retaining the existing tamper proof chain.
3. **AC3 — Execute regression proof.** The exact named digest test and the
   complete `delivery_constitution` suite pass against the unmodified tracked
   inputs after the correction. No ignored test, dynamic oracle, additional
   accepted digest or bypass is introduced.
4. **AC4 — Retain all gates.** Before implementation/verify success, retain
   actual verdicts for every required command in the
   [single house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
   the fail-closed native-tls absence proof and existing pinned design/delivery
   selftests. Verify uses canonical pinned resources and the candidate-local
   digest-matched `.forge/bin/brokkr`. Structural checks, compile success and
   old receipts cannot replace current execution. CI-owned obligations retain
   their documented ownership; any failed/unavailable proof stays explicit.
5. **AC5 — Honest residual status.** Issue #171's AC2/R04/D04 remain open
   exactly as distinguished in the
   [existing receipt index](../delivery-proof-residuals/receipts.md).
   Those names belong to the earlier spec, not this spec's numbered criteria.
   No separate live self-bundle receipt, operator acceptance, issue/epic
   completion, push or merge is inferred or fabricated. Existing
   [proposed 0072](../../docs/decisions/0072-delivery-proof-residuals.md)
   remains proposed and supplies no exception.
6. **AC6 — Reviewable design handoff.** Chief HEAD touches exactly
   `specs/tooling-digest-sync/spec.md`, `plan.md` and `tasks.md`, all committed,
   non-empty and clean. The deterministic design check validates that HEAD;
   chief reports only `drafted`. Position and result files remain run-local.
