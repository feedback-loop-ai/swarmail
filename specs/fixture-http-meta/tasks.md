# Ordered tasks: fixture HTTP metadata

Unchecked items belong to later implementation/delivery; drafting does not
claim they ran. AC references point to spec.md. Every task names its proof.

- [ ] T1 — Confirm the committed design, evidence hashes, local engine digest,
  shim integrity and unchanged routing. Identify the operator-owned index
  integration prerequisite without expanding evals-only implementation scope.
  Proof: packet checksum command, pin/shim checks, pinned design-check and root
  `every_decision_file_has_exactly_one_resolving_index_row`; record any refusal.
- [ ] T2 — Extend evals/tests/wrappers.rs's existing real stdio/HTTP setup with
  complete stream/header capture and bounded child/listener completion. Add
  metadata and GET-error regression tests before touching production. Proof:
  focused actual_stdio tests show nonzero assertion failures for AC1/AC2/AC5;
  record exact command, exit and assertion in implementation receipts.
- [ ] T3 — Add the one defaulted discarded params._meta slot to Call, preserving
  closed fields and already permissive method dispatch. Proof: the same AC2
  matrix becomes green for all revisions, locations and metadata variants;
  AC3 full-stream and HTTP sentinel checks pass.
- [ ] T4 — Split GET-body refusal and expose only http_get_body_not_supported
  as the fixed tool error. Document both decisions in evals/wrappers.md. Proof:
  AC5 real-stdio omitted/present GET matrix, exact tool-result envelope, POST
  JSON-byte controls and generic unrelated-error control pass.
- [ ] T5 — Complete negative controls for unknown/missing/wrong-shaped fields,
  arguments._meta, unknown tool/method, whole-line metadata overflow and no HTTP
  effects. Proof: AC4/AC6 actual_stdio tests, existing wrapper bounds/origin/
  redirect/identity/byte-fidelity assertions and complete EOF sequences pass.
- [ ] T6 — Run standalone eval fmt, locked all-target clippy, focused wrappers
  and full locked tests. Record red/green evidence and exact exits in
  evals/issue151-meta-implementation.md without raw client metadata or secrets.
  Proof: all four independent commands in plan.md exit zero; a timeout or
  missing receipt remains outstanding, never inferred green.
- [ ] T7 — Commit implementation work unsigned within evals/, then run every
  local house gate with the verified candidate-local engine and pinned script
  owners, coverage selftest, guard proofs, design selftest, delivery proof and
  fail-closed native-tls absence. Proof: command/exit receipts on that exact
  candidate, zero missed lines/branches/functions, no unruled compile output,
  clean tracked tree and scope diff. Index integration requires the external
  owner identified in T1 before all gates can pass. Never waive that failure.
- [ ] T8 — Hand off tested committed candidate and remaining externally owned
  proofs through the existing recipe. Proof: AC1–AC7 receipt review, committed
  path/status check, no paid/model execution and no claim of macOS/live smoke/
  baseline success. CI/operator ownership remains as stated in spec.md.
