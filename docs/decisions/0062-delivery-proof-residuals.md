# 0062 — Distinguish delivery proof completion from documented limits

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-24
- **Clarifies:** [0050](0050-delivery-gates.md) §§2, 4–5 and its proof
  obligations; preserves [0045 §8](0045-delivery-repairs.md),
  [0056](0056-signing-policy.md) and [0057](0057-codex-delivery-recipes.md).
- **Design:** [spec](../../specs/delivery-proof-residuals/spec.md),
  [plan](../../specs/delivery-proof-residuals/plan.md),
  [tasks](../../specs/delivery-proof-residuals/tasks.md).
- **Scope:** NEXT-171, epic #192 findings 3.1–3.4.

## Context

Structural policy tests do not execute missing/null/malformed facts through
the real evaluator. Existing script-presence and command-substring checks
can accept altered scripts. Triage found historical self runs but not a
qualifying current pinned end-to-end design/retry receipt. Finally, the
release expansion comparator permits equal-indentation `if true {` to
`if false {` differences without proving they came from debug assertions.
An ordinary `if cfg!(debug_assertions)` can fit that textual allowance.
0050 §2's normative equality requirement remains stronger than this check.

## Proposed clarification

1. Retain one existing policy and proof chain. Bind real evaluator,
   aggregation, actual-seat and live-run evidence to recorded engine,
   candidate and bundle identities. No independent policy interpreter,
   compilation-only substitute or synthetic journal can close those proofs.
2. Add independent fixed canonical script identity expectations and actual
   shim digest comparisons to the existing tests. Prove deleted, disabled,
   commented and substituted checks fail; preserve actual pinned dispatch
   proofs because structural pins alone do not demonstrate execution.
3. Choose the issue's dated-marker alternative for finding 3.4. Append a
   dated known-limit addendum explicitly for 0050 §2, preserving its accepted
   body. Describe the allowance's actual scope, link this proposal and the
   observation, and label the finding documented rather than repaired.
   This proposal does not authorize the allowance, widen it, weaken exact
   coverage, or change the normative requirement that unaccounted release
   differences refuse. Known limits are not accepted exemptions, as in 0035.
4. Reconcile old T08/R04/D04 against their complete obligations, not just
   this issue's four bullets. Missing execution evidence remains open;
   broader missing obligations block closure without authorizing unrelated
   implementation changes. Use current 0057 safe scope/fixes predicates and
   unsigned author commits under 0056 while preserving historical entries.

## Alternatives and costs

Immediate strict comparison would reject legitimate debug-assert expansions
unless backed by a demonstrated compatible mechanism. No such mechanism is
established here; source/indentation/panic-shape heuristics are not macro
provenance. We decline that expansion of this story and accept the cost that
the false acceptance remains visible and unresolved. Future semantic
exceptions need their own explicit operator acceptance under 0045 §8;
future compatible hardening must prove both the real bypass refusal and
honest debug assertions. This record accepts neither exception nor repair.

Fixed script hashes require reviewed updates after legitimate edits. Real
provider and exact-core evidence can block completion. Those costs are
preferable to claiming a proof the artifacts never executed. Compatible
factual documentation and test strengthening may proceed while this record
is proposed; nothing here authorizes contradicting an accepted ruling.

## Proof and preservation

The linked spec AC1–AC7 and paired tasks define the required receipts.
Application runtime and public contracts, frozen guarantees, 0005's ingest
extraction, coverage denominator, dependency policy, protected review and
security hold stay unchanged. No provider fallback, engine upgrade, push,
publish or issue/epic completion is authorized by this draft. Implementation
adds the proposed index entry and dated pointer; chief commits only the
three design artifacts and this proposal as its identifiable HEAD.
