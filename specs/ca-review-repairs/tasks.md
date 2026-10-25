# Ordered tasks — CA review repairs

Implementation tasks remain unchecked at design. Requirements and decisions are
in [spec.md](spec.md); ownership details and commands are in [plan.md](plan.md).

- [ ] T1 (AC1): Inventory all admitting webhook setup/GET/PUT/delivery and post-
  DELETE observations in api, lifecycle, p2 and persistence; record independent
  sites with reasons. Move CA_CONTRACTS to common and reuse it in webhook and
  each affected fixture for the full ownership lifetime. **Proof:** review the
  complete route/setup inventory against guard owners; run all five integration
  binaries with normal parallel scheduling, preserving success assertions and
  dedicated overload behavior.
- [ ] T2 (AC2): Extend the existing executor harness with a deterministic live-
  receiver, closed-before-entry assertion, distinct from cancelled receiver.
  Assert Unavailable, job nonexecution/disposal, retained original slot/bytes,
  exact refund and fresh-scope capacity reuse. **Proof:** the new test rejects
  prior sender-drop behavior; controlled channels/keyed observations require no
  sleeps. Retain cancelled receiver, failed send and genuine Worker coverage.
- [ ] T3 (AC2): Split execute_envelope's receiver-cancelled and scope-closed cases;
  send the latter's Charged(Unavailable) using the original reservation, with
  input disposed before any refund. **Proof:** T2 and the entire
  `webhook::ca_executor::` suite pass, including retained-reply and closed-worker-
  channel tests. Production lines/branches remain subject to exact coverage.
- [ ] T4 (AC3): Confirm caller mappings with existing tests; extend assertions
  only where needed for exact 503/Worker distinctions and unchanged PUT state.
  **Proof:** `api::webhook_handler_tests::`, `webhook::ca_construction_tests::`
  and `tests/webhook.rs` pass; retain real held-worker persistent SMTP progress,
  panic recovery, closed-scope publication/send refusal and terminal accounting.
  Record final test names and outcomes.
- [ ] T5 (AC4): Remove the separating blank line in docs/decisions/README.md.
  **Proof:** one 0078 row directly follows the prior table row, remains proposed,
  and the linked 0078 record is byte-identical with proposed status; diff review
  shows no accepted decision edits or additional documentation cleanup.
- [ ] T6 (AC1): Execute five consecutive complete api/lifecycle/p2/persistence
  rounds at 32 test threads with competing test processes, using plan quickstart.
  **Proof:** retain suite counts, individual exit codes, commands, timestamps and
  overlapping process execution for all 20 runs; all pass. Keep success assertions
  unchanged and record the static audit alongside this finite stress evidence.
- [ ] T7 (AC5): Commit the implementation unsigned, then run every current house
  gate with the verified `.forge/bin/brokkr` and pinned scripts, plus shim checks,
  design/delivery self-proofs and fail-closed native-tls absence. **Proof:** clean
  committed candidate, candidate-bound logs/receipts for each command, no bundle
  policy-sweep warning, exact zero missed lines/branches/functions and passing
  coverage selftest, publish guard and seal-branch proof. Resolve each local
  failure before claiming implementation success; no dependency/routing change.
- [ ] T8 (AC5): Deliver that committed candidate through existing hermetic verify
  and protected review routing. **Proof:** candidate-matching verify/review
  results; unresolved work is stated honestly. CI/operator retain external
  proofs and sealing; no push, merge, tag, publication, closure or ADR acceptance.
