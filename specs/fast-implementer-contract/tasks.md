# Tasks: Restore the fast implementer test contract

Ordered implementation and delivery obligations; unchecked means not yet proved.
The design commit does not claim these downstream tasks have run.

- [ ] T1 — Capture baseline and reproduce the obsolete Sol expectation before
  editing. Test: `cargo test --locked --test delivery_recipes fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers -- --exact`
  fails specifically on the current Luna-versus-Sol assertion; record setup or
  compilation failures separately. Confirms AC2's before-state.
- [ ] T2 — Repair only the final four-strategy loop: exact named fast office,
  singleton Luna and Luna/xhigh, all with strategy diagnostics. Keep the selected
  office helper lookup. Test: rerun T1's command and require success; review each
  AC1 rejection condition against the independent equality assertions. Preserve
  the no-default/reviewer assertions verbatim (AC1–AC4).
- [ ] T3 — Retain and run the existing office/adapter regression and all surrounding
  recipe guards. Tests: `cargo test --locked --test delivery_recipes fast_implementation_uses_luna_at_xhigh_without_provider_fallback -- --exact`
  and `cargo test --locked --test delivery_recipes` both pass. Confirm no edit to
  the dedicated test, reviewer matrix, fallback assertions or input JSON (AC2–AC3).
- [ ] T4 — Check scope and commit the bounded implementation unsigned. Proof:
  `git diff --check` passes; diff review shows only the final loop changed in
  `tests/delivery_recipes.rs` beyond the three design artifacts. Recipe, office,
  adapter, provider, source, dependency, gate and accepted-decision inputs are
  unchanged; no global Sol replacement or new framework is present (AC4).
- [ ] T5 — Run the complete authoritative house gate inventory through the
  established verifier, preserving candidate artifact provenance, digest-matched
  candidate-local engine, exact coverage/selftest, publish/seal proofs, licenses
  and fail-closed native-TLS absence. Test: every required downstream gate has
  successful candidate-bound execution evidence; report each missing/failing proof
  separately, and retain CI-only obligations as CI-owned. Targeted success alone
  cannot check off this task (AC5).
- [ ] T6 — Retain the triage-selected fast engine review and its authority.
  Proof: the existing workflow records five distinct required results (Astra
  correctness, Flash security, Sonnet tests, Astra guarantees, independent Fable),
  with protected aggregation and security hold enforced. Confirm unchanged fallback
  boundaries, no DeepSeek Pro, and no push, merge or issue closure. Do not substitute
  static assertions for actual review results or manually advance the phase (AC6).
