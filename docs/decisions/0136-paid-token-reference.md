# 0136 — Choose an accounting source and a prospective token reference

- **Status:** proposed
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
