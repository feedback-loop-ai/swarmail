# Ordered tasks — coverage selftest recovery

Updated: 2026-10-24 — attributed scope return. Follow [spec.md](spec.md),
[plan.md](plan.md) and [proposed 0062](../../docs/decisions/0062-coverage-reservation-tooling.md).
Preserve all dated spec clarifications and the engine's existing return count.

T1's retained full-context/direct traces now justify the chosen production
exception; they do not close every audit field or any final-candidate proof.
Audit T1 before production edits. T2's regression uses the controller and
assertions built in T3–T4; finish that proof before closing T2. All tasks remain
unchecked until each paired test has actual evidence, including retained
candidate-bound evidence where applicable. Initial #174 code is not a pass.

- [ ] **T1 — Audit attributed causality and tool availability (AC10, AC16).**
  Bind `.forge/epic192/issue174-recovery-t1/attributed/` and the oversized
  receipt to f3cee46, recovery 310b969, 16841aa and #173 ff9bf8a. Preserve
  full-context, direct ordinary and direct injected evidence and its limits.
  **Test:** command/Bash/options/overrides, working directory, exact path,
  worker identity, descriptors/traps and cleanup records corroborate one
  kernel success plus EEXIST and two utility/resolver successes without
  intervening removal. Fill missing fields explicitly; no passing rerun
  substitutes for the failing trace. Check isolated Python stdlib and required
  POSIX lifecycle capabilities in the box; record verifier/platform evidence
  still owed. No blind installation, dependency widening or fallback.
- [ ] **T2 — Restore direct exclusive leaf creation (AC8, AC14, AC16).**
  Replace only the explicit leaf operation with checked isolated inline Python
  `os.mkdir`, preserve parent/default paths, exact spelling, umask and caller
  ownership. Distinguish tool/protocol failure from filesystem refusal; add
  truthful comments and index 0062 as proposed without accepting it.
  **Test:** actual resolver admits a fresh leaf, refuses retained/file/symlink
  targets and other creation errors with category/path, and sets success
  metadata only after validated completion. Missing/broken Python and malformed
  completion fail distinctly before builds; default allocation still works
  without Python. Mode/path controls preserve behavior. After T3–T4, the old
  leaf-operation mutation with witnessed EEXIST-to-zero override produces 0/0
  and fails the shared proof; repaired canonical code under that override
  reaches the direct operation and passes, as do direct EEXIST and normal race.
- [ ] **T3 — Build the finite local controller (AC11–AC12).**
  One selftest-local two-worker owner uses private channels, actual canonical
  resolver definitions and checked identity-bound readiness/release. Bound
  setup, opens, writes, reads, resolver descendants, delivery and joins by
  30 seconds monotonic time plus at most 5 seconds teardown grace.
  **Test:** two live ready workers receive checked tokens before resolution;
  death before readiness, post-release failure, blocked coordination and
  nested-child failure cause bounded failure with all owned processes joined,
  deadline machinery finished and inherited descriptor sentinels intact.
  Exercise partial setup and conditional Bash invocation. No sleep establishes
  readiness, unrelated process is signaled or unbounded final wait is hidden.
- [ ] **T4 — Attribute complete independent outcomes (AC11–AC12).**
  Preserve resolver status separately from result delivery and worker exit;
  validate fresh framed records and inspect only the identified loser's log.
  **Test:** actual race yields statuses {0,1}, exact actual winner path and
  ownership 0, and the status-1 child's own literal category/path. Missing
  completion, failed result write, missing/truncated/malformed/extra records,
  wrong identity/token, unrelated nonzero status and refusal text only in the
  other log fail the same controller/assertions finitely, including conditional
  execution. Deliberate failures never reset the real parent's failure count.
- [ ] **T5 — Prove caller-owned lifetime (AC13, AC16).**
  Keep the leaf through both outcomes; test cleanup and retained-path retry
  using observed ownership, not manufactured metadata.
  **Test:** sentinel bytes survive actual cleanup with ownership 0 and retry
  refusal naming category/path; children never delete the leaf. Outer root
  cleanup follows joins/assertions. Preserve explicit/default ownership,
  empty/dirty targets, regular/dangling symlinks, files, relative/space/trailing
  paths, parent/leaf errors and failures after successful creation.
- [ ] **T6 — Revalidate the full selftest and preservation (AC1–AC8, AC14–AC16).**
  Run syntax check, focused controls and accumulated canonical selftest. Audit
  the diff to 16841aa and byte-compare scanner/unaffected non-selftest regions
  to ff9bf8a, explicitly reviewing AC8's operation/tooling exception.
  **Test:** cause-specific helper controls, malformed successful df, real
  derive-before-offender, minted green attribute premise, independent binary
  red/green, offline locked builds, inventory/partial-output, raw identifiers,
  clean boundary and all reservation controls pass. Adapted failure injections
  prove execution at the live boundary. No application/scanner/shim/checksum,
  dependency graph, floor/denominator, policy/recipe or #175 changes appear.
- [ ] **T7 — Prove the final candidate through every gate (AC9, AC15–AC16).**
  Execute the sole house-rule inventory and additional pinned verifier proofs.
  **Test:** candidate/pin-bound receipts show fmt, locked clippy/tests,
  digest-matched bundle compile, separate actual coverage and selftest,
  positive exact 100/100/100, publish-guard, keyless seal, licenses, design-check
  selftest, delivery tamper proof, strict shims and native-tls absence by its
  specific package-not-found diagnostic. Verify Python capability in the
  actual hermetic environment. Failed/blocked/not-run checks stay outstanding;
  design preflight and older receipts cannot certify a repaired candidate.
- [ ] **T8 — Preserve protected reviews and honest handoff (AC9, AC15).**
  Keep the engine lane, four configured independent reviews and security hold.
  **Test:** actual candidate-bound engine receipts identify Qwen Max
  correctness, Claude Opus security, official DeepSeek Flash guarantees and
  DashScope GLM 5.3 tests, never Pro or fallback. CI-only proofs retain their
  charter owner. Keep author commits unsigned; no push, merge, tag or publish.
  Report no feature completion with required work outstanding.
