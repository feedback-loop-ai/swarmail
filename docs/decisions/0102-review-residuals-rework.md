# 0102 — Review residuals return to implement, five times

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-30
- **Source:** operator direction, 2026-10-30, given in chat: "make sure the
  graph can go back to implementation up to 5 retries on those runs, so we
  don't have to manually reschedule every residual. Everything above Medium
  leads to rework, Mediums when we exhaust the runs - ask the operator, Lows
  are accepted".
- **Amends:** [0050](0050-delivery-gates.md) §4, only the residual rows of
  the review table. The security hold, the final unconditional hard stop and
  "unknown facts never ship" remain.

## Context

Under 0050 §4 a residual above medium was a hard stop and a medium shipped as
tracked debt. Every stop needed the operator to start a new run by hand.
On 2026-10-29 five runs in a row stopped this way at review, four of them on
the same record gap. Each needed a hand restart, and none returned to the
seat that could fix it.

## Decision

`self/policy.json` moves to `forge.phase-machine/v2` (the schema that may
park) and its `review`/`residual` rows become, in first-match order:

1. `REVIEW-RESIDUAL-EXHAUSTED`: `visits_review_gte: 6`, severity above low,
   no spec defect → **park** for the operator.
2. `REVIEW-RESIDUAL-SECURITY-EXHAUSTED`: `visits_review_gte: 6`, a security
   residual, no spec defect → **park**.
3. `REVIEW-RESIDUAL-REWORK`: severity above low, no spec defect →
   `implement`.
4. `REVIEW-RESIDUAL-SECURITY-REWORK`: a security residual, no spec defect →
   `implement`.
5. `REVIEW-RESIDUAL-OK`: no security residual, at most low, no spec defect,
   no fixes applied → `ship` (flagged, tracked debt).
6. `REVIEW-RESIDUAL-LOW-FIXED`: the same with fixes applied → `verify`.
7. `REVIEW-RESIDUAL-UNSAFE`: unconditional hard stop (spec defects and
   absent, null or malformed facts).

The engine counts `visits_review` from the run's own `phase/entered` events
(brokkr 0022, 0094), so the sixth review is the one after five returns. A
return hands the review's result to the implementer as
`context.returned_from`. The implementer and reviewer charters say what that
means.

Reading the direction: high and critical residuals rework. Mediums rework
too, and once the five returns are spent they park for the operator. A
residual still above medium at that point also parks: the operator rules
instead of a hard stop that would need a hand restart anyway. Lows ship. A
security residual of any severity reworks and never ships as debt (0050's
rule stands).

`codex-fast` composes these rows after its own, so its strategy-gated
medium routes are unchanged; it gains rework instead of a stop above
medium, and must declare v2 because composed layers share one schema.

## Costs

A run may now spend up to five more implement, verify and review rounds,
each a five-member review, before it parks. A park holds the run's worktree
lease until the operator retries or concludes it.

## Enforcement binding

`tests/delivery_constitution.rs::residual_admission_requires_explicit_safe_facts_and_unknowns_stop`
pins the rows, their order, the threshold and the schema;
`tests/delivery_recipes.rs::fast_recipe_preserves_shell_proofs_design_origin_and_hard_review_refusals`
keeps `codex-fast` from overriding them. Every recipe compiles with the
pinned engine, and the policy sweep leaves no group unruled.

The explicit operator direction authorizes this table change while this
record remains proposed; acceptance is still the operator's.
