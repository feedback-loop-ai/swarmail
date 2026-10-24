# Ordered tasks — eval privacy recorded-gap repair

Clarification — 2026-10-24: the earlier continuation checklist required evidence
for preserved cf3aa2f/c232602. This revision follows recovered review of f6dbbb53
and intake `continue-epic-192-issue-196-from-119060a2`; it adds demonstrated
repairs while preserving historical receipts and T4's supported-platform meaning.
Unchecked work is downstream, not a claim all existing code is absent. The
[spec](spec.md) owns every finding's disposition and unchanged contracts. No
decision, dependency, root source, gate or recipe change is authorized.

Tasks are ordered below; suffixes keep historical T4/T6/T8/T9 evidence references
stable. Each paired proof needs candidate/platform/command/exit binding. Missing
prerequisites or zero-test runs never check a task.

- [ ] **T1 — Bind scope and evidence (AC9, AC11, AC19).** Verify packet hashes/
  lengths, ancestry, clean starting source, OS and recorded engine identity;
  inventory every request/resolver caller and retain every finding disposition.
  **Test:** manifest equality, git ancestry/diff, tooling digest equality and
  unchanged proposed-0062/index status; record actual starting identities.
- [ ] **T2 — Repair native worker transport (AC2, AC16).** Carry OsString through
  trial CLI, TrialRequest, validation/run_body/agent_argv and resolution; convert
  AgentSpec JSON once and adapt necessary callers.
  **Test:** actual matrix CLI executes a bare program in non-UTF-8 PATH, records
  completion/literal bytes, emits one matching schema-1 row with real status and
  correct aggregate exit. Direct trial preserves native later args; old String
  worker boundary fails the matrix control. Built-in/missing/relative/absolute/
  literal-wrapper cases still pass.
- [ ] **T3 — Repair lazy HOME defaults and prove maps (AC1–AC3, AC18).** Consult
  HOME only for missing Cargo/Rustup home, preserving empty refusal and locators.
  Keep exact per-role maps and private directories.
  **Test:** real preparation with both absolute homes and no HOME succeeds;
  each missing default without HOME and each empty selector safely refuses.
  Actual role-map, prompt equality, custom-target and synthetic wrapper-auth
  controls pass without new environment exceptions.
- [ ] **T4a — Repair executed Cargo overflow proof (AC3, AC5, AC15).** Fix script
  newlines and literal path quoting. Emit valid >1 MiB JSON with an independent
  canary, followed by real Cargo records.
  **Test:** require launch, successful byte count, post-write completion, real
  offline/locked Cargo success and full drains under a bounded deadline/cleanup;
  oversized refusal is safe, short equivalent succeeds and no raw output reaches
  output/artifacts. Missing launch, short write, Cargo failure or missing completion
  fails. Bypassed overflow rejection fails the intended assertion; stopped drainage
  fails completion/deadline.
- [ ] **T5a — Repair portable canary and cfg (AC4–AC6, AC17).** Extend helper to
  count/complete >64 KiB split invalid-byte output per stream. Portable case uses
  unsafe_output and ordinary Fail/GaveUp with agent exit 0. Retain separate Linux
  death control and cfg-contain probe/oracle code.
  **Test:** portable CLI exits 1 with private artifacts, null executable metadata
  and no raw/canary bytes in output or any final/temp artifact. Completion follows
  both streams. Linux actually kills its owned server and yields HarnessError.
  All-target clippy uses -D warnings without suppression. T4 requires actual
  macOS execution, never inferred from Linux.
- [ ] **T5b — Complete adjacent worker controls (AC6, AC20).** Add independent
  wrong-label, exit-mismatch and CR-row cases in existing validation tests.
  **Test:** each refuses without echo; valid acceptance and existing wrong-task/
  extra/malformed/overflow/cardinality/aggregate-exit cases pass. Preserve process
  group/drop behavior and explicitly retain its interruption debt.
- [ ] **T4 — Complete supported-platform preparation proof (AC3, AC13, AC17).**
  Run system/rustup/toolchain, ordinary/custom-target, once-per-batch discovery
  and repaired Cargo/default/canary controls on supported platforms.
  **Test:** actual macOS eval all-target locked clippy/full tests and preparation
  receipts, separately from Linux. Leave T4 open until macOS genuinely executes;
  no cross-compile/assumed CI coverage closes it. Continue independent tasks while
  that evidence is unavailable.
- [ ] **T5c — Preserve publication/lifecycle proofs (AC4–AC6, AC12).** Run scoped
  runner/report safe CLI/path/panic/admin errors, readiness/floods, server death,
  deadline/interruption, artifact failure and cleanup controls.
  **Test:** nonzero counts, safe categories/outcomes, drained pipes and actual
  exits; run admin_error_categories_discard_injected_response_text and
  automatic_build_uses_a_cleared_named_preparation_map with a real built
  custom-target executable. Record /usr/bin/python3 availability for its existing
  readiness test; absence is failed/missing evidence, never skipped success.
- [ ] **T6 — Bind or reproduce privacy discrimination (AC8, AC19).** Inspect old
  source/test regions and receipts; rerun missing inheritance, raw-publication
  and unsafe metadata/error controls. Reassess changed canary; keep AC15/16
  controls separate.
  **Test:** each mutation fails its intended assertion with nonzero counts, then
  restored code passes. Record command, source/test/diff, OS, exit and log; final
  diff has no mutation. Old blob names and empty logs alone are insufficient.
- [ ] **T7 — Reconcile docs and prove evals separately (AC7, AC9, AC10).** Preserve
  dated evidence, update only affected owning docs/evidence and retain each
  LOW/INFO disposition.
  **Test:** separate eval fmt, locked all-target clippy/full tests including
  proof.rs real oracle/null/wrong-action/overlapping isolation. Every production
  change has executing regression proof; no exclusions or root denominator change.
  Root green cannot substitute for eval proof.
- [ ] **T8 — Execute complete pinned house proof (AC10, AC11).** Follow the sole
  house-rules inventory on the final restored candidate; preserve exact coverage/
  selftest, publish/seal, licenses, native-tls absence, shims/tooling identity.
  **Test:** every required command succeeds, plus warning-free pinned compile,
  design-check/selftest, actual prescribed design/verify dispatch and policy/
  delivery tamper proofs with receipts. Offline verify installs nothing; old
  run passes remain distinct.
- [ ] **T9 — Obtain accepted five-role review (AC14).** Run Astra correctness,
  official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees and Fable
  5.1 independent as five separate offices/results on the changed verified
  candidate.
  **Test:** engine/journal receipts bind exact routes, candidate and verdicts;
  preserve worst verdict, bounded returns and security hold. Never DeepSeek Pro,
  author substitution or invented fallback; superseded GLM/Haiku does not apply.
- [ ] **T10 — Hand off evidence honestly (AC10, AC13, AC14, AC19).** Distinguish
  fixed defects, retained residuals, local execution, bound historical controls
  and outstanding macOS/CI evidence.
  **Test:** every claimed pass resolves to actual candidate/platform receipts;
  unrun chart/history/selftest, Docker, RustSec, tag/rate/publication and privileged
  port-80 proofs remain with charter owners and actual status. Never mark required
  incomplete work complete. No push, PR, merge or issue close.
