# Ordered tasks — coverage selftest reservation follow-up

Updated: 2026-10-24. These are downstream obligations, not claims made by the
chief commit. Initial #174 code exists at `16841aa`, but verification failed;
retained requirements must be revalidated. Follow [spec.md](spec.md) and
[plan.md](plan.md). Keep dated clarifications on any subsequent return.

- [ ] **T1 — Establish causality before patch selection (AC10).** Preserve
  the receipt, identify `16841aa` and #173 ancestry `ff9bf8a`, and compare the
  actual full-context fixture with a direct race through the same resolver.
  **Test:** candidate/command/Bash/command-resolution/path/lifetime and
  individual output/status receipts reproduce and attribute the two-success
  failure. A passing reduced control alone fails this evidence requirement.
  Unreplicated causality stays unresolved; do not close the residual.
- [ ] **T2 — Apply the evidenced repair within AC8 (AC10, AC14–AC15).** Fix
  only the responsible selftest execution path. A proved production resolver
  fault requires scope return and reconciled documents before production edits.
  **Test:** a control through actual functions fails when the demonstrated
  fault is restored and passes after repair; diff/byte checks confirm scope.
  A speculative readiness patch cannot substitute for causal repair.
- [ ] **T3 — Make coordination explicit and finite (AC11–AC12).** Use private
  endpoints, checked readiness/release and a bound covering opens, writes,
  reads and waits. Preserve inherited descriptors; stop/reap fixture children
  before root cleanup.
  **Test:** two live children acknowledge readiness before release and request
  the same leaf; death before readiness and failure after release yield bounded
  failed proofs with no surviving workers or descriptor damage. Expiry never
  yields success and no sleep establishes readiness.
- [ ] **T4 — Attribute each outcome (AC11–AC12).** Capture actual winner path
  and ownership, loser status/output, complete fresh records and independent
  worker completion; check errors under conditional shell execution.
  **Test:** real race yields {0,1}, exact winner path/ownership 0 and the status-1
  child's own named refusal/path. Missing completion, unrelated nonzero result
  and refusal text present only in the other log each fail the actual local
  assertion logic without resetting the real parent's failure count.
- [ ] **T5 — Prove caller-owned lifetime (AC13).** Retain the winning leaf
  through both outcomes and run cleanup/retry against sentinel evidence.
  **Test:** actual cleanup with observed ownership 0 and retry naming the
  refusal category/path leave leaf/sentinel unchanged; outer cleanup follows
  joins and assertions. Existing explicit/default ownership, path-shape and
  parent/leaf-error controls retain their outcomes.
- [ ] **T6 — Revalidate the full selftest and preservation boundaries
  (AC1–AC8, AC14–AC15).** Run syntax check and canonical selftest, audit diff
  to `16841aa` and scanner/non-selftest bytes to `ff9bf8a`.
  **Test:** diagnostic helper controls, malformed successful df, derive before
  independent offender, actual green minted attribute, independent binary
  red/green, offline locked builds, inventory/partial-output, raw identifiers,
  clean boundaries and reservations all pass. Setup errors never substitute
  for refusals; no scanner/runtime/shim/policy/dependency/#175 change appears.
- [ ] **T7 — Prove the final candidate through every local gate (AC9, AC15).**
  Run the full house-rule inventory and pinned verifier's additional proofs.
  **Test:** candidate/pin-bound receipts show fmt, locked clippy/tests,
  digest-matched bundle compile, separate actual coverage and selftest,
  positive exact 100/100/100 counts, publish-guard, keyless seal, licenses,
  design-check selftest, delivery tamper proof, strict shims and native-tls
  absence by its specific diagnostic. Failed/blocked/not-run checks remain
  outstanding; baseline and structural draft checks do not certify repair.
- [ ] **T8 — Retain engine-selected reviews and honest handoff (AC9, AC15).**
  Preserve the four configured seats, protected review and security hold.
  **Test:** actual engine receipts for the implemented candidate identify Qwen
  Max correctness, Claude Opus security, official DeepSeek Flash guarantees
  and DashScope GLM 5.3 tests, with no Pro or fallback. CI-only proofs keep their
  charter owner until evidenced. Keep unsigned author commits and no push,
  merge, tag or publish; report no completion with required work outstanding.
