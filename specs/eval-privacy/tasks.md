# Ordered proof tasks — eval privacy continuation

Clarification — 2026-10-24: these tasks replace the original prospective
implementation checklist after cf3aa2f/c232602. Unchecked means this continuation
still needs an attributable proof disposition, not that committed code is absent.
Existing semantic [spec](spec.md) and proposed [0062](../../docs/decisions/0062-eval-privacy.md)
remain; historical results are preserved. T4 keeps its supported-platform meaning.

- [ ] **T1 — Bind the candidate and authority (AC9, AC11).** Preserve c232602;
  record ancestry, candidate commit/tree, clean tracked state, OS and candidate-local
  engine digest against tooling.json. Confirm existing 0062 index/status agreement.
  **Test:** git ancestry/diff checks, SHA-256 equality, and root
  `cargo test --locked --test store_contract_docs`; no duplicate index addition.
- [ ] **T2 — Execute c232602's focused controls (AC4, AC12).** Run
  `admin_error_categories_discard_injected_response_text` and
  `automatic_build_uses_a_cleared_named_preparation_map` without zero-test success.
  **Test:** non-200 response canary absent with admin_response_status and
  admin_response_rejected, plus an actual debug/swarmail file under custom target;
  capture commands, exits and candidate identity.
- [ ] **T3 — Prove runtime maps and compatibility (AC1, AC2).** Exercise actual
  Swarmail, fixture, agent and matrix roles with complete environment observations
  and execution markers, synthetic arbitrary/non-UTF-8/stale/admin values, private
  directories, eight inputs and prompt equality. **Test:** existing runner/report
  cases for role maps, caller-boundary resolution, literal argv and explicit
  synthetic file-config wrapper authentication; no inherited helper exceptions.
- [ ] **T4 — Prove preparation on supported platforms (AC3, AC13).** Verify closed
  named preparation maps, offline/locked ordinary/custom targets, once-per-matrix
  discovery and safe malformed/oversized/failed build handling. **Test:** real
  automatic-build and Cargo protocol controls on owning system/rustup toolchains;
  require macOS execution for macOS. Record Linux results separately and leave this
  task open while the macOS system/rustup/toolchain receipt is missing. Do not
  assume current CI provides it or invent a platform pass; continue independent
  Linux tasks while this obligation stays open.
- [ ] **T5 — Prove publication, draining and lifecycle (AC4–AC6, AC12).** Keep
  independent both-stream canaries absent from stdout/stderr and final/temp
  artifacts while preserving useful categories and actual outcomes. **Test:**
  runner/report flood/readiness, bad CLI/path/HTTP input, spawn/startup failure,
  deadline, panic, interruption, service-death, artifact and matrix failure cases;
  assert cleanup, exit precedence and full matrix cardinality. HTTP non-200 proof
  alone does not cover transport/read/invalid-response paths or response allocation.
- [ ] **T6 — Establish discriminating controls and real behavior (AC7, AC8).**
  Preserve prior red/green evidence only with exact relevant source/test binding;
  reproduce missing necessary controls narrowly using synthetic inputs and restore
  source before final proofs. **Test:** role inheritance, raw publication and
  metadata/error mutations fail their focused controls; restored tests pass.
  Full proof.rs proves oracle, null idle classes, wrong actions and overlapping
  real SMTP/HTTP/MCP isolation; no mock acceptance or sleep-based interleaving.
- [ ] **T7 — Run separate eval checks and reconcile documentation (AC9, AC10).**
  Verify README and earlier owning eval docs agree with exact maps, wrapper auth,
  safe metadata and unconfined limits. **Test:** eval fmt, locked all-target clippy,
  full locked tests including proof.rs; retain commands/exits/source identities.
  Root tests cannot replace this suite. Only a demonstrated defect permits code
  repair; preserve its failing proof and rerun affected checks after repair.
- [ ] **T8 — Execute the complete pinned house proof chain (AC10, AC11).** Follow
  the sole inventory in docs/house-rules.md, with actual recorded-engine compile,
  checked shims, exact coverage/selftest, design-check selftest, delivery tamper
  boundary, publish/seal proofs, licenses and fail-closed native-tls absence.
  **Test:** each actual command exits successfully on the final candidate, compile
  has no policy-sweep refusal warning, and actual pinned design/verify dispatch and
  policy-evaluator proofs produce attributable receipts. Offline verify installs
  nothing; missing execution remains outstanding, never a substituted green.
- [ ] **T9 — Complete protected independent review (AC14).** Run correctness and
  security plus official DeepSeek Flash guarantees and GLM 5.3 tests review on
  the verified candidate. **Test:** engine/journal receipts bind identities and
  verdicts to that candidate; Haiku only after documented GLM dispatch failure,
  never after an adverse verdict. Resolve returns and preserve security hold;
  no recipe/policy edits or author self-certification.
- [ ] **T10 — Publish an honest evidence handoff (AC10, AC13, AC14).** Report local
  executions, historical receipts, macOS T4 and charter CI obligations separately.
  **Test:** each claimed pass resolves to a candidate/platform-bound receipt;
  unrun chart/history/selftest, Docker, advisory, tag/rate/publication and privileged
  port-80 proofs stay attributed to CI with their actual status. Do not mark full
  completion while required evidence/review remains; no push, merge or issue close.
