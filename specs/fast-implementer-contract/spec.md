# Specification: Restore the fast implementer test contract

Created: 2026-10-24. Status: draft for implementation.
Scope: swarmail, exact #201 candidate verification failure reported by run
`continue-epic-192-issue-201-at-v-648baa6d`.
Design run: `triage-and-fix-the-exact-201-can-0b93d47f`; triage class: engine.

## Feature and rationale

Restore the existing delivery recipe regression to the accepted fast implementation
route. PR #213 selects `codex-fast-implementer` for chore, feature, design and
engine. That office declares singleton `luna` at `xhigh`; the final loop in
`fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers`
still expects singleton `sol` at `high`. This obsolete oracle rejects the
intended configuration and blocks candidate verification.

The repair changes only that loop in `tests/delivery_recipes.rs`: pin the named
office for every strategy, then check its exact model array and Luna effort.
The existing dedicated Luna regression remains the office/adapter proof. Expected
values remain independent literals, so a configuration regression cannot update
its own oracle. No public behavior, frozen guarantee or accepted ruling changes;
no new decision is declared. Accepted [0057](../../docs/decisions/0057-codex-delivery-recipes.md),
including its 2026-10-24 implementation clarification, and
[0058](../../docs/decisions/0058-codex-review-routing.md) are authorities.

## Synthesis rulings

Both run-local positions were read in full: simplicity.md and robustness.md under
`.brokkr/state/design/positions/`, together with the matching intake framing under
`.brokkr/state/tasks/`. They agree on the implementation; there is no genuine
conflict to disguise as a compromise.

- Adopt simplicity's existing-loop repair, existing `office` helper and unchanged
  dedicated Luna test. Reject additional files, shared model registries, generic
  validation frameworks and broad Sol substitutions. The accepted cost is manual
  test maintenance when an operator deliberately changes the route.
- Adopt robustness's exact per-strategy office, singleton-model and effort checks,
  with strategy-specific failure diagnostics. Reject deleting the loop, a mere
  contains-Luna assertion or expectations derived from the configuration. The
  small overlap with the dedicated office test is intentional: selecting the
  right office and defining that office correctly are separate obligations.
- Reconcile robustness's runtime failure analysis with simplicity's narrow scope:
  preserve the existing storage, SMTP, await, pruning, extraction, webhook and TLS
  contracts through unchanged owners and their existing gates. Add no runtime
  hardening, fixtures or protocol tests here. This leaves existing crash/retry,
  retention, best-effort webhook and resource-limit residuals unchanged; these
  structural assertions provide no new durability, capacity or live-dispatch proof.
- Retain the intake's engine classification and five separate review results.
  Reject a cheaper chore reroute based on patch size. The accepted cost is the
  full gate and review expense for a small change to delivery proof machinery.

## Requirements and boundaries

For each of exactly chore, feature, design and engine, the test must require
`codex-fast-implementer`, resolve the selected office through the existing helper,
and require `models == ["luna"]` and `efforts.luna == "xhigh"`. Each assertion
identifies the strategy. Keep the test name, selector/no-default checks and
reviewer assertions unchanged. Keep
`fast_implementation_uses_luna_at_xhigh_without_provider_fallback` unchanged.

Implementation is limited to the final assertion loop. Recipe, office, charter,
adapter, provider, policy, tooling, gate, dependency and accepted-decision edits
are excluded, as are production code and the inherited #201 eval/stdout work.
Triage remains Sol/medium; other intentional Sol routes remain intact. Preserve
accept == stored, subscribe-before-check, oldest-only pruning and ingest extraction.
No new dependency, license allowance or coverage exclusion is warranted.

Accepted 0058 remains exact: chore has one combined Flash reviewer; feature has
Qwen correctness and Flash security, with Sonnet only after authorized Qwen
pre-result dispatch failure; design/engine have Astra correctness, official
Flash security, Sonnet tests, Astra guarantees and independent Fable as five
separate results. No other provider fallback is permitted. DeepSeek Pro is
forbidden. Preserve protected review, hard security hold, unknown-fact refusal,
worst-verdict aggregation, candidate binding and pinned proof ownership.
No push, merge or issue closure is authorized.

## Acceptance Criteria

1. **AC1 — Exact route:** Every existing implementation strategy is checked for
   the named fast office, exact singleton Luna and xhigh, with per-strategy
   diagnostics. A strategy selecting another office (even a Luna office), an
   extra fallback model, or a non-xhigh Luna effort cannot satisfy these assertions.
2. **AC2 — Reproduce and repair:** Implementation records the named selector test
   failing on the old Sol expectation before the edit and passing afterward.
   The unchanged dedicated Luna regression and full `delivery_recipes` suite pass
   using `--locked`. An environmental failure is recorded as such, never as the
   expected assertion failure or a pass.
3. **AC3 — Preserve routing proof:** Existing exact strategy set, no-default,
   one/two/five reviewer, fallback and inherited proof/refusal assertions remain
   unchanged and pass in the full suite. Office, adapter and recipe files have
   no implementation diff. These checks do not certify live provider dispatch.
4. **AC4 — Bound the diff:** Apart from these three design artifacts, implementation
   changes only the final implementation loop of `tests/delivery_recipes.rs`.
   Review confirms no unrelated Sol route, application guarantee or proof owner
   was changed and no additional dependency or framework was introduced.
5. **AC5 — Complete downstream proof:** The full authoritative
   [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
   passes in downstream verification, including fail-closed native-TLS absence
   and candidate executable/tooling provenance. CI-only proofs remain CI-owned.
   Targeted green tests do not discharge unavailable or failing gates.
6. **AC6 — Retain the selected review:** The engine lane obtains the five distinct
   results required by accepted 0058 through the existing protected workflow.
   No provider fallback change, DeepSeek Pro, push, merge or issue closure occurs.

This document records a design, not implementation completion, full verification,
review acceptance or resolution of the earlier run. Gate failures outside this
bounded repair remain explicit outstanding work for the owning workflow.
