# 0090 — Separate operator-directed mixed-provider delivery route

- **Status:** accepted (operator, 2026-11-02 — "accept all as recommended",
  given in chat to the dispatcher).
- **Date:** 2026-10-28
- **Source:** current operator directive for run `verify-and-independently-review--53f33089`, examining candidate `c2f76e26989bbd5eb7aabcf28f57e4ef8073d136`.
- **Scope:** additional future recipe under [0057](0057-codex-delivery-recipes.md) and [0058](0058-codex-review-routing.md); inherits the revised tests direction recorded in proposed [0081](0081-glm-flash-seats.md).
- **Design:** [spec](../../specs/mixed-provider-routing/spec.md), [plan](../../specs/mixed-provider-routing/plan.md), [tasks](../../specs/mixed-provider-routing/tasks.md).

## Context and authority

The operator now requests future delivery with Claude Opus 5.5 implementing,
Codex Terra 6.1 performing intake, Astra reviewing, and a mix including DSH
GLM-5.3-Flash on Spark. Candidate c2f76e2 adds that separate recipe and two
work-office overlays. Existing 0057/0058 describe Codex implementation in
their named Codex recipes. This record states the narrow new route explicitly
instead of silently broadening those historical rulings. The current directive
authorizes this future addition; it is not acceptance of this document, 0081
or any unrelated proposal. Only the operator accepts decisions.

The chief examined both contrarian positions and the current feature/intake
framing. Legacy `.brokkr/state/tasks/` is absent. That missing historical copy
is not replaced with invented journal evidence. Prior pinned runs and their
verdicts remain exactly as recorded.

## Proposed ruling

1. Keep `bundles/mixed-opus-terra` extending `codex-careful`, with exactly
   intake and implement seat overrides. Intake uses `mixed-terra-intake`,
   Codex `terra61 -> gpt-6.1-terra`, high effort. Implementation uses
   `mixed-opus-implementer`, Claude `opus -> claude-opus-5-5`, high effort.
   Historical `terra -> gpt-5.6-terra` and existing recipe assignments remain.
   Offices inherit existing role charters and preserve boxed workspace hands,
   masked Cargo credentials, read-only Rustup and existing permissions/limits.
2. Inherit all five required gate reviewers without overrides: Astra
   correctness and Astra guarantees (`gpt-6-astra`, distinct roles/results),
   official DSH Flash security (`deepseek-flash`, `deepseek-official`), local
   Spark DSH tests (`spark-glm/GLM-5.3-Flash-EXL3`, efforts empty and selected
   effort absent), and independent Claude Fable (`claude-fable-5-1`). Other
   selected efforts are high. Every work/review chain is a singleton.
3. Preserve protected review, worst-verdict aggregation, explicit scope/fixes
   and safety facts, unknown-fact refusal, bounded returns, security-only
   panel hold authority, candidate binding and every deterministic gate.
   Neither duplicated Astra roles nor shared Claude provider implies five
   independent model families; all five invocations/results remain required.
   No fallback is permitted here, including the fast-feature-only Qwen/Sonnet
   exception. DeepSeek Pro, paid substitutions and author-as-reviewer escape
   paths remain forbidden.
4. Keep review read-only with its existing own-result delivery contract.
   Compilation and `ok` resolution establish structure only. Actual provider
   invocation, boxed work-tool use and engine-consumed results require their
   own attributable evidence. Preserve unsupported dsh native permissions/MCP
   declarations and do not claim equivalent sandbox enforcement. If requested
   provider or result delivery cannot occur without broader review writes,
   park and retain the failure. Historical [0017](0017-codex-review-seat-delivery-sandbox.md)
   does not authorize a write grant in this run.
5. Verify the full candidate diff, exact effective roster, unchanged careful
   policy/gates and pinned CI compilation of all five recipes. Extend the
   existing tests for two-seat scope, historical alias, effective identities,
   hands and mixed-specific compile failure/warnings. Keep source, compile,
   deterministic proof, live delivery and CI evidence separate. All house
   gates remain required, including exact coverage and selftest, publish and
   seal proofs and licenses. Use only the admitted candidate-local engine.
6. Preserve 0081's unresolved dispatcher `1-cross-attempt` finding until
   fresh authorized evidence establishes its status. Focused route success
   cannot clear a failing full dispatcher proof. No engine repair, repin,
   assertion waiver, provider probe or new benchmark belongs to this route.
   Standalone evals require same-user non-overlap coordination. Preserve all
   review residuals and mark CI-only checks as CI-owned.

## Consequences and rejected alternatives

Adopt simplicity's small inherited overlay and robustness's complete evidence
chain. Two small hands declarations and strict comparison tests cost less
maintenance than another inheritance abstraction. Mandatory real results and
full gates cost execution time and can stop progress on a provider outage;
this cost is preferable to silently losing a reviewer. Reject source-only
acceptance, duplicated panels/policy, merged verdicts, generalized fallback
machinery and broader review writes. Keep application crash/concurrency and
webhook/transport concerns covered by their existing guarantees and suites;
this change adds no production code and weakens none of 0001–0005.

The chief commits only the three design files and this proposal; implementation
adds its index row. No existing decision body/status is changed. The design
check proves artifact structure, not route completion. No push, merge,
publication, tag, issue closure, paid trial or acceptance follows. Any later
ship remains subject to the existing policy, complete evidence and unsigned
author-commit discipline under 0056.

## Dated inherited-panel pointer — 2026-10-29

Accepted [0098](0098-review-panel-flash-and-deadline.md) records the
operator's 2026-10-28 direction that careful's correctness and guarantees
members select official DeepSeek Flash through dsh (`deepseek-flash`,
`deepseek-official`, high) instead of `gpt-6-astra`, and that careful's review
seat allows 14400 s. `mixed-opus-terra` inherits both without an override;
ruling 2's Astra wording above describes the route as of 2026-10-28. This
pointer accepts no decision.

## Dated addendum — 2026-11-02 (at acceptance)

The operator accepted this record on 2026-11-02. The text above is unchanged
and remains the record of what was proposed. This addendum states what has
moved since it was written, checked against `main` at `c336c95`; it rules
nothing beyond the acceptance.

- **The recipe moved.** Ruling 1's `bundles/mixed-opus-terra` is
  `.brokkr/recipes/mixed-opus-terra/` since accepted
  [0100](0100-one-brokkr-folder.md). The bundle is as ruled: it extends
  `codex-careful` and overrides only intake (`mixed-terra-intake`, `terra61`
  → `gpt-6.1-terra`, high) and implement (`mixed-opus-implementer`, `opus` →
  `claude-opus-5-5`, high).
- **The inherited panel.** Ruling 2's Astra correctness and guarantees
  predate accepted [0098](0098-review-panel-flash-and-deadline.md), as the
  2026-10-29 pointer above says. On `main` the careful panel
  (`codex-careful`, which `mixed-opus-terra` inherits) and the `codex-fast`
  design and engine panels seat correctness on
  `codex-review-astra-correctness` (the office keeps its old name) and
  guarantees on `codex-review-guarantees`; both select `flash`, which
  `.brokkr/adapters/dsh.json` maps to official DeepSeek `deepseek-flash`
  through dsh, at high effort. Security stays on `codex-review-security`
  (`flash`, high), tests on `codex-review-tests` (`glm-flash`, `efforts:
  {}`) and independent review on `codex-review-independent` (`fable`, high).
- **§6's residual.** The dispatcher `1-cross-attempt` finding §6 keeps open
  has the evidence §6 asks for: [0081](0081-glm-flash-seats.md)'s 2026-10-29
  and 2026-10-30 pin results record the full routing proof exiting 0 on the
  re-pinned engine, the pin `.brokkr/recipes/self/tooling.json` holds today.
- **One record for the route.** [0103](0103-mixed-work-seats.md) records the
  same route from another candidate branch; this record is the one that
  governs the `mixed-opus-terra` recipe.

## Addendum — 2026-11-10 (operator: Fable 5.2)

The `fable` alias in `.brokkr/adapters/claude.json` now resolves to
`claude-fable-5-2`. Operator, 2026-11-10: "swap fable 5.1 with 5.2 in all
recipes". Every seat this record routes to Fable 5.1 (`claude-fable-5-1`) runs
Fable 5.2. Nothing else in this record changes.
