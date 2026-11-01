# 0102 — Review residuals return to implement, five times

- **Status:** accepted (operator, 2026-10-30 — "0102 accepted", given in
  chat).
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

The operator directed this table change and accepted this record on
2026-10-30.

## Dated addendum — 2026-10-31: spec defects return to design

- **Source:** operator direction, 2026-10-31, given in chat: "send spec
  defects back to design".

Under the table above, every review with `spec_defect: true` fell to
`REVIEW-RESIDUAL-UNSAFE` or `REVIEW-CLEAN-UNSAFE`, a hard stop. Four of the
last eight runs stopped that way on defects a redesign would have fixed: an
acceptance criterion that contradicted an accepted decision; a guide that
taught recipient routing when routing is by SMTP AUTH (0004); a test
dependency that landed without an operator ruling; and an overstated size
claim. Each needed a hand restart.

On that direction, `self/policy.json` gains six rows. They stand first in
their groups, ahead of every row this record added:

- `review`/`residual`, in first-match order:
  1. `REVIEW-RESIDUAL-SPEC-EXHAUSTED`: `visits_review_gte: 6`, a spec
     defect, `fixes_applied: false`, `has_security_residual: false`, a
     well-formed severity (`max_residual_severity_at_most: "critical"`) →
     **park**.
  2. `REVIEW-RESIDUAL-SPEC-SECURITY-EXHAUSTED`: the same with
     `has_security_residual: true` → **park**.
  3. `REVIEW-RESIDUAL-SPEC-REDESIGN`: a spec defect, no fixes, no security
     residual, a well-formed severity → `design`.
  4. `REVIEW-RESIDUAL-SPEC-SECURITY-REDESIGN`: the same with a security
     residual → `design`.
- `review`/`clean`, before `REVIEW-CLEAN-NO-FIXES`:
  1. `REVIEW-CLEAN-SPEC-EXHAUSTED`: `visits_review_gte: 6`, a spec defect,
     `fixes_applied: false` → **park**.
  2. `REVIEW-CLEAN-SPEC-REDESIGN`: a spec defect, no fixes → `design`.

Every row they precede reads `spec_defect: false`, so no earlier row can
take a spec defect. Clean rows read no security or severity fact because a
clean verdict carries neither; the pinned aggregate refuses a clean verdict
that reports one.

**Shared budget.** `visits_review` counts every entry into review, wherever
the run was sent, so returns to design and to implement draw on the same
five. The sixth review that still finds a spec defect parks for the
operator, as the sixth that still holds a residual above low does.

**Unknown facts still stop.** A spec defect matches a new row only with
every fact present and well-formed. With `has_security_residual` or
`max_residual_severity` absent, null or malformed, it reaches the final
unconditional stop of its group, as before. So does a spec defect beside
`fixes_applied: true`: panel members are read-only, a panel that changed
code while finding the design wrong built on the defective design, and
`codex-fast`'s reclassification has required read-only facts since 0057.

**A spec defect beside a security residual returns to design and never
ships.** The design is what the panel found wrong, so a security fix made
first in implement would be built on the defective spec. The chief carries
the security finding into the revised tasks, implement fixes it, verify
runs, and the full panel, security member included, reviews again. No row
that ships or re-verifies admits `spec_defect: true` or, for a residual,
`has_security_residual: true`. At the sixth review the run parks: the
design and the risk acceptance are the operator's. A hard stop here would
treat a run with a security residual and a spec defect more harshly than
one with a security residual alone, which this record already returns to
implement. `REVIEW-SECURITY-HOLD` remains an unconditional hard stop.

**Graph.** Review to design is a return edge in the phase order. The engine
gives every step of the design sequence (both positions, the chief and the
check) the review's verdict as `context.returned_from`, as implement
receives it under the table above. The sequence and the deterministic check
are unchanged. The chief revises the same `specs/<slug>/` in a new commit on
top of the earlier design, which therefore stays in git history. Implement,
entered from design, receives `context.returned_from` with phase `design`
and builds to the revised tasks, which carry the review's findings. The
chief must not weaken an acceptance criterion merely to make the review
pass; a criterion that conflicts with an accepted decision is corrected
toward the decision. The chief, position, implementer, reviewer and four
panel charters describe the loop.

**`codex-fast`.** Its own rows still precede self's. Its reclassification
sends a read-only spec defect with no security residual and at most medium
severity to triage, bounded as before; its other residual spec defects with
complete facts now reach self's design rows instead of the hard stop. Its
own clean rows rule every clean verdict ahead of self's, so it removes
self's two clean design rows rather than leave them unreachable, which the
engine refuses; its clean spec defects reclassify or stop as before.

**Costs.** A spec-defect return spends a design round (two positions, the
chief and the check) before implement, verify and review run again, within
the same five returns.

**Enforcement.**
`tests/delivery_constitution.rs::residual_admission_requires_explicit_safe_facts_and_unknowns_stop`,
`::clean_spec_defects_return_to_design_and_unknowns_stop` and
`::a_spec_defect_returns_to_design_or_parks_and_never_ships` pin the rows,
their order, the threshold and the design returns' parking twins;
`tests/delivery_recipes.rs::fast_recipe_preserves_shell_proofs_design_origin_and_hard_review_refusals`
pins `codex-fast`'s removal of the clean design rows and keeps it from
overriding the residual ones. `tests/codex_review_routing.py` drives the
rows through the pinned engine with a live stand-in design phase: a spec
defect redesigns and then ships once clean, in a residual and in a clean
verdict; a persistent one redesigns five times and parks at the sixth
review; one with an absent security fact, or beside applied fixes, stops;
one beside a security residual redesigns five times, parks and never
ships. Every recipe compiles with the pinned engine, and the policy sweep
leaves no group unruled.

The body above is unchanged as the record of what was accepted. This
addendum records the operator's direction and the table change it
authorizes; it accepts nothing new.
