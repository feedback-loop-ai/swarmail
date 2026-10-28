# Ordered tasks — MCP refusal proof

All checkboxes below describe downstream work. Chief drafting is not test
completion. Check each only with its paired proof; missing rows and external
prerequisites remain explicit even when the bounded assessment is finished.

- [x] **T1 — Freeze provenance and assess the ledger (AC1, AC8).** Verify the
  packet, three supplied evidence manifests, local engine digest and shims;
  preserve historical path hashes and all prior member findings. Record baseline
  HEAD/tree and the absence of the legacy intake directory.
  **Test:** manifest/hash checks plus focused owner and mcp_revisions commands
  with actual exits; each T8 row names its revision/adapter/witness or gap.
- [x] **T2 — Add the modern report argument matrix (AC2).** Use existing Wire
  for omitted/empty arguments, null/numeric ID, malformed container and extra
  member across June/November HTTP/binary; include id:null request control.
  **Test:** `cargo test --locked --test mcp_revisions modern_report_argument_refusals`
  pins complete -32602 envelopes/correlation/no-result, then drains children;
  historical fixture hashes remain identical.
- [x] **T3 — Pin exact unavailable and timeout envelopes (AC3, AC4).** Strengthen
  current held-worker and modern timeout assertions with independent literals.
  **Test:** `cargo test --locked report_workers_do_not_hold_store_guards` and
  `cargo test --locked --test mcp_revisions modern_storage_errors_and_revision_isolation`
  prove exact errors and retained worker-entry/SMTP/prune/delete/isolation behavior.
- [x] **T4 — Prove modern notification combinations (AC5).** Exercise admitted
  seed once and malformed-report silence over both modern HTTP/binary pairs.
  **Test:** `cargo test --locked --test mcp_revisions modern_notification_refusal_contract`
  checks empty HTTP 202, correlated ping, no trailing reply after EOF, exact
  final transcript and stored effect counts; no ping-as-barrier assumption.
- [x] **T5 — Prove unavailable notification silence in the owner (AC3, AC5).**
  Use a separate held-server case if necessary; preserve panic-safe cleanup.
  **Test:** `cargo test --locked report_unavailable_notifications_keep_bridge_silent`
  checks June/November HTTP and real bridge-function notification/ping/EOF,
  with all workers held through refusal. Label the in-memory stream boundary.
- [x] **T6 — Execute retained T8/invariant proofs and document remaining gaps
  (AC1, AC4, AC6).** Run the full mcp_revisions, stdio, deliverability suites and
  existing direct-dispatch rate/worker-wait tests; record names and evidence
  classes in this supplement's implementation receipt.
  **Test:** locked focused-suite exits, raw legacy/default byte equality,
  shared-rate owner assertions and existing cap/continuation/shutdown witnesses;
  diff/hash checks show no historical, runtime, dependency or bound change.
  Binary-unavailable and shared-router exhaustion stay explicitly unproved;
  this task assesses those gaps and does not mark historical T8 complete.
- [ ] **T7 — Commit unsigned and run all local gates (AC7).** Complete the
  implementation commit and execute the entire house inventory plus framed
  design/delivery proofs, even after any failure. Preserve evidence, fix only
  scoped failures, and bind reruns to the actual clean candidate.
  **Test:** command/exit/HEAD/tree/log-hash receipts for fmt, locked clippy/tests,
  digest-pinned compile, exact zero-missed coverage and selftest, publish/seal
  proofs, licenses, fail-closed native-tls absence, shims and supplemental proofs.
  CI-owned checks remain separately owed, never represented as local results.
- [ ] **T8 — Hand off faithfully to protected verification/review (AC8).**
  Preserve HIGH inherited review-integrity, prior severities and #152 tests
  indeterminate; retain the selected five reviewer identities and all merge
  prerequisites, including actual OPTIONS-candidate review reconciliation.
  **Test:** compare handoff to original checksum-verified findings; current
  engine-owned verify and five separate review receipts must bind their actual
  candidate/attempt. Leave this task unchecked until those receipts exist;
  a handoff alone supplies none. #151 baseline, #201 coordination, actual #152
  baseline/review, clients, CI and operator seal require their own evidence.
