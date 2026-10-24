# Ordered implementation tasks — eval privacy

These unchecked tasks implement [spec.md](spec.md); they are not execution
receipts. Proposed [0062](../../docs/decisions/0062-eval-privacy.md) is not accepted.

- [ ] **T1 — Reconcile authority/index (AC9).** Add one proposed 0062 row to
  docs/decisions/README.md during implementation, confirm no accepted-ruling
  contradiction, preserve the four-file chief commit. **Proof:** root
  `cargo test --locked --test store_contract_docs`, especially decision inventory
  and status agreement; review against 0055 §§6–9.
- [ ] **T2 — Make subprocess probes discriminating (AC1, AC2, AC8).** Move
  helper modes/markers/barriers to test-owned wrappers/configuration or literal
  synthetic arguments. Capture whole environment/OS bytes before wrapper additions.
  Add parent-only and independent-output canaries. **Proof:** runner/proof helpers
  require execution markers and correct generated inputs, show old-path red controls,
  and retain working external-command/barrier controls.
- [ ] **T3 — Resolve commands and close runtime maps (AC1, AC2).** Implement
  original-cwd/PATH resolution, /tmp ownership and exact role maps for Swarmail,
  fixture, agents and matrix workers, without global mutation/test exceptions.
  **Proof:** runner/report actual-role observations with stale/admin/arbitrary/
  non-UTF-8 canaries, eight inputs/prompt equality, distinct private dirs,
  absolute/relative/bare commands, shebang/subprocess execution, literal
  metacharacters and safe missing-program failure.
- [ ] **T4 — Close preparation (AC3).** Implement named toolchain/cache map,
  bounded Cargo records, suppressed stderr and once-per-matrix discovery.
  **Proof:** real offline/locked builds with ordinary and custom targets, supported
  system/rustup discovery on their owning platform, counted matrix preparation,
  exact build map and no selectors in runtime roles; controlled build launch/exit/
  malformed/oversized-output failures produce safe categories. Untested supported
  platforms remain explicit outstanding evidence, never presumed green.
- [ ] **T5 — Remove retention and bound announcements (AC4, AC5).** Replace Log
  tails/unbounded lines with discard drains and a first-valid typed loopback slot;
  cap safe diagnostic facts. **Proof:** runner split/invalid/no-newline/oversized
  output on both streams, pre/post-ready floods, oversized-then-valid announcements,
  noisy real-agent completion, structural storage bounds and deadline/cleanup checks.
- [ ] **T6 — Close all publication sinks (AC4, AC6).** Map errors in runner/CLI/
  matrix, install entry panic hook, bound/validate worker row, discard stderr,
  null binary metadata and write only safe artifact bytes before rename.
  **Proof:** runner/report scan streams and all final/temp artifacts after unsafe
  path/argv/parse/HTTP-response cases, startup/spawn failure, server death, panic,
  interruption, artifact failure and matrix build/launch/row failure. Assert useful
  categories, cardinality and exit precedence; independent child emission must
  fail an old-publication control.
- [ ] **T7 — Prove auth migration and repair docs (AC2, AC9).** Test a caller
  wrapper reading synthetic private config. Revise README and earlier eval spec/
  plan with dated environment/capture/metadata clarification. **Proof:** actual
  trial and matrix wrapper execution, literal argv/prompt tests, doc review against
  exact maps and explicit-input/unconfined limits; no real credentials in evidence.
- [ ] **T8 — Reproduce controls and behavior (AC6–AC8).** Run separate focused
  mutation-red/restored-green controls for role environments, raw publication and
  unsafe metadata/errors, then restore all mutations. **Proof:** full eval runner,
  report, fixture/proof suites: oracle all-pass, null idle failures, wrong-action
  classes, actual exits/scoring, noisy/deadline/interrupt/panic/server-death cleanup
  and two overlapping isolation trials using real SMTP/HTTP/MCP. Record tested
  source identities and unresolved proof, not historic verdicts.
- [ ] **T9 — Execute full gates (AC10).** Run eval fmt, locked clippy/full tests
  including proof.rs and every current house gate through the pinned chain:
  exact coverage/selftest, publish/seal proofs, recorded-engine compile, licenses
  and fail-closed native-tls proof included. **Proof:** command/exit/source-bound
  receipts and restored clean candidate; root green cannot substitute for evals.
  CI-only obligations stay separate; unavailable execution remains outstanding.
- [ ] **T10 — Complete protected verification/review (AC10).** Submit the
  implemented candidate to hermetic verification and four independent lenses,
  resolve returns and honor security hold before the engine's ship gate.
  **Proof:** actual candidate-bound journal/engine receipts, no author
  self-certification, push/merge or completion claim based on this draft.
