# 0136 — Choose an accounting source and a prospective token reference

- **Status:** accepted (operator, 2026-11-23; see the dated acceptance note at the end)
- **Date:** 2026-11-20
- **Specification:** [Trial evidence](../../specs/trial-evidence/spec.md)
- **Operator questions:** #391, part of #164

## Which source should count paid tokens?

Prefer the journal channel in accepted [0119 §§5–6](0119-eval-brokkr-recipe.md). It puts usage and the seat that incurred it under one trial identity and one missing-data rule. It costs an authenticated observation for each family, confirmed executable and authentication binds, and a follow-up to forward only the observed channel. Until then the driver refuses before launch and direct paid matrices still discard command output.

Alternatively, the direct wrapper could read only a pinned CLI's final usage summary (for example Claude Code JSON usage or Codex exec JSON token counts), decode finite input, output and cached counts, discard all other output and type missing, malformed or ambiguous totals as unavailable. This could measure ordinary direct matrices. Each CLI version would need a bounded streaming parser, privacy proofs, verification of units and terminal-event semantics, and reconciliation with journal totals. Reading this stream is a further exception to [0075 §1](0075-eval-privacy.md). Recommend deferring it. Only operator acceptance could authorize implementation under 0045 §8. Neither candidate source supplies a historical measurement.

## Which run defines “30% below the baseline”?

Designate prospectively the first paid main matrix under a recorded source with complete checked accounting and a defined positive tokens-per-success reference for every predeclared comparison group. Before launch fix tasks, repetitions, model/profile, tool surface, budgets, grouping and token units. Record the main revision, harness and target binary hashes, CLI/model versions, source and exact scored population. An incomplete group leaves the planned comparison pending. Selecting only the cheapest groups after seeing the figures would bias it.

For each matched group compare the §3 sum of input plus output over all scored attempts divided by its pass count, reporting cached tokens separately. The target is a candidate figure at most 0.70 times the positive reference under comparable settings. Disclose changes in success counts and versions. A changed cohort or accounting definition needs a separately labeled reference.

#151 measured revision a4f552d after #152–#157 had merged, without token facts. A future reference supports a prospective comparison only. It cannot quantify pre-#152 costs, attribute historical savings to those changes or establish that the epic already met its target. Both choices await the operator.

## Dated acceptance note — 2026-11-23

The operator accepted this record on 2026-11-23, in chat to the dispatcher ("accept the decision"), with both recommendations as written:

- **Accounting source:** paid tokens are counted through accepted 0119 §§5–6's journal channel, once the per-family authenticated via-brokkr smoke observes it and its addendum records the channel. The direct-launch reader of a CLI's final usage summary stays deferred and unimplemented: it would be a further exception to 0075 §1, and it is not authorized by this note.
- **The token reference for #164's "30% below the baseline":** the first paid main matrix with complete checked accounting under a recorded source, with its cohort, settings and comparison groups fixed before launch. It is a prospective reference only; it says nothing about pre-#152 costs.

The text above is unchanged.

## Dated addendum — 2026-11-23 (the accounting source for via-brokkr rows)

The smoke recorded in 0119's 2026-11-23 addendum found that brokkr's `exec` adapter drops a nested driver's stdout. The journal channel this record prefers therefore cannot carry via-brokkr tokens in any brokkr version up to 0.15.2.

The operator ruled route (b) on 2026-11-23. Paid via-brokkr token counts come from the nested driver's own forge-driver/v1 `checkpoint` and `result` fields, which `swarmail-evals` reads as that driver's engine and records as a typed harness fact. The direct-launch reader of a CLI's own usage summary stays deferred and unimplemented.

The token reference rule above is unchanged, with one addition: the reference run's recorded source is this via-brokkr driver channel. The operator fixed the reference cohort before launch, on 2026-11-23:

- **Tasks:** the 16 default-tier tasks.
- **Models:** claude-opus, claude-sonnet, codex-sol, codex-luna, glm-spark and glm-dashscope.
- **Trials:** five per task and model, which matches the 2026-11-01 baseline's cohort.

The text above is unchanged.
