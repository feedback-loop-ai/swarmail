# Repair the fast Luna test oracle

## Feature and rationale

Correct the stale implementation expectations in
`tests/delivery_recipes.rs::fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers`.
The test currently requires Sol/high for every fast strategy, while the selected
`codex-fast-implementer` office already requires exactly Luna/xhigh. This false
failure blocks the recorded Epic #192 delivery continuation. Repairing the
oracle restores useful gate evidence without changing the accepted route.

Authority is accepted [0057](../../docs/decisions/0057-codex-delivery-recipes.md),
including its dated fast implementation clarification, and accepted
[0058](../../docs/decisions/0058-codex-review-routing.md). Sol/medium remains the
triage route. No public contract, frozen guarantee or ruling changes; no new
proposed decision is warranted.

## Evidence and synthesis rulings

Read both run-local positions, `.brokkr/state/design/positions/simplicity.md`
and `robustness.md`, and the current intake framing
`.brokkr/state/tasks/tiny-epic-192-delivery-repair-fi-9a870543.md`.
The earlier continuation framing identifies historical context, not additional
scope for this repair. Current source inspection confirms the stale equalities,
all four selectors naming the Luna office, and the neighboring test already
pinning that office and its `gpt-6-luna` adapter identity. These are inspected
facts; they are not Rust execution or historical journal receipts.

1. **Adopt simplicity's two-line repair.** Use literal exact equalities for
   `["luna"]` and `efforts.luna == "xhigh"`. Reject shared constants, a resolver,
   new fixtures, new helpers and deriving expected values from the office being
   tested. Cost: future accepted model changes must update explicit expectations.
2. **Adopt robustness's independent protection.** Preserve the four-strategy
   loop, exact singleton assertion, diagnostics, selector shape and reviewer
   equalities, plus the adjacent office/adapter test. Reject weakening equality
   to membership, accepting either model, or dropping apparently duplicate
   coverage. Cost: intentional duplication remains; the two tests protect
   different links in the selection chain.
3. **Reconcile runtime analysis as a scope boundary.** Adopt robustness's
   preservation argument for accept-before-250 ordering, subscribe-before-check,
   oldest-only pruning and ingest-time extraction. Reject turning its crash,
   webhook, hostile-input, retention or TLS scenarios into new implementation
   work. Existing runtime residuals remain under their existing rulings; this
   patch supplies no new durability, exactly-once or live-provider assurance.
4. **Rule on the cheaper-lane tension.** The positions converge on retaining
   the recorded `TRIAGE-ENGINE` result. The task prefers chore when eligible;
   intake found this executable delivery-policy guard ineligible under 0057 §7.
   Retain that classification. This chief neither reclassifies the run nor
   edits its journal or policy. Under 0058 the recorded engine route requires
   five separate results: Astra correctness, official Flash security, Sonnet
   tests, Astra guarantees and independent Fable. The losing cheap-lane option
   would save four reviewer invocations; retaining engine costs those calls and
   the design sequence. A duly recorded reclassification is an engine-process
   matter, not something this spec fabricates. The existing chore route remains
   one combined official Flash reviewer; its test protections stay intact.

## Scope

The later implementation changes exactly two assertion lines in the named test.
Required design artifacts are separate: this spec, plan.md and tasks.md in this
one directory. Recipes, reviewers, adapters, gate scripts, dependencies, runtime
code, privacy policy and other Epic #192 work are outside implementation scope.
If a semantic or security change becomes necessary, stop the bounded repair and
surface the evidence. Never use DeepSeek Pro. No push, merge, tag or issue closure
is authorized; authorized commits are unsigned under
[0056](../../docs/decisions/0056-signing-policy.md).

## Acceptance Criteria

- **AC1 — Exact implementation oracle.** For each of `chore`, `feature`,
  `design` and `engine`, the existing loop requires
  `overlay["models"] == json!(["luna"])` and
  `overlay["efforts"]["luna"] == "xhigh"`. A different model, an extra model or
  another effort cannot satisfy these equalities.
- **AC2 — Selection and review protections.** The selector-on-strategy,
  allowed selector keys, exact four cases, absence of a default, single Flash
  chore office, two-member feature panel and five-member design/engine panel
  assertions remain byte-for-byte unchanged, including their diagnostics.
- **AC3 — Independent regression proof.** Capture the original named test's
  Luna-versus-Sol assertion failure, then its passing result after the repair.
  The complete `delivery_recipes` target passes, including the unchanged
  `fast_implementation_uses_luna_at_xhigh_without_provider_fallback` test.
- **AC4 — Bounded diff.** Excluding these three design documents, the repair
  diff contains only the two named assertion replacements. No helper, recipe,
  reviewer, adapter, runtime, privacy, dependency or accepted-decision changes.
- **AC5 — Honest delivery evidence.** The complete gate inventory in
  [house rules](../../docs/house-rules.md#gates--part-of-every-change), including
  fail-closed native-tls absence, remains mandatory on the repaired candidate.
  Exact coverage and its selftest, pinned proof ownership, protected review and
  hard security hold remain intact. Distinguish CI-owned evidence from local
  execution; unavailable or failing checks remain outstanding. A draft or a
  targeted green test cannot certify delivery completion.
- **AC6 — Committed design identity.** The chief's last commit touches exactly
  `specs/fast-luna-oracle/{spec,plan,tasks}.md`; all three are nonempty, committed
  and clean, with the required acceptance, risks and checkbox structure. The
  pinned sequence check owns design advancement under
  [0050](../../docs/decisions/0050-delivery-gates.md).
