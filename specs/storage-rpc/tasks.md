# Ordered implementation tasks — storage and RPC boundaries

These tasks are uncompleted implementation work. See [spec.md](spec.md) for
AC1–AC10 and [plan.md](plan.md) for placement, sequencing and gate rules.
Names below are planned tests; final commit bodies must use actual names.

> **Superseded for #129 (2026-10-18):** task 2's data-file guard is replaced
> by the identity sidecar of [macos-identity-lock](../macos-identity-lock/tasks.md)
> (amended 0047 §1); task 1's branch creation is done. Other tasks stand.

- [ ] 1. Establish `epic120-storage-rpc` from current fetched main, verify
  `2a325c9` ancestry and decision numbering, and sign exactly the four design
  artifacts. **Proof:** ancestry check, artifact validator, commit signature
  and `git show --name-only` contain exactly the promised four paths;
  read-only Git/signing refusal is reported, never counted as success.
- [ ] 2. Add the nontruncating data-file identity guard after sidecar
  acquisition with correct Unix creation mode and drop/error behavior (#129,
  AC1–AC2). **Tests:** inline `data_file_aliases_refuse_in_both_orders`,
  `identity_guards_release_after_drop_and_failed_open`,
  `identity_lock_preserves_raw_sqlite_and_busy_bound`,
  `identity_open_preserves_contents_and_permissions`, plus existing typed
  error/lock-before-database tests and coverage of new open/lock failures.
- [ ] 3. Prove alias refusal with real serve children and preserve restart
  behavior (#129, AC1–AC3). **Tests:** extend
  `a_second_server_on_a_held_data_file_refuses_and_a_dead_owner_frees_it`
  in `tests/binary.rs` for symlink/hard-link acquisition orders, named
  nonzero/no-readiness refusal, healthy first owner, accepted mail/cap
  fidelity, graceful restart and immediate SIGKILL/reap reuse; preserve
  existing lifecycle commit-failure and raw SQLite inspection regressions.
- [ ] 4. Replace overflow with LF and correct the attribution contract
  (#130, AC4). **Tests:** inline
  `overflow_is_inadmissible_and_distinct_after_prometheus_encoding` and
  `tracked_uncounted_stays_exact_after_registry_capacity`; assert 4096 real
  rows plus at most one overflow, exact folded count/total, existing-name
  increments and distinguish legal literal backslash-n via independent
  decoding. Add `cumulative_scrape_uses_one_registry_snapshot` to prove total
  and rows are taken under one lock, with deterministic observation if needed;
  retain externally maintained totals in other families. Use a fresh local
  registry, never reset shared counters.
- [ ] 5. Share reply obligation through `reply_id` and `handle` (#132,
  AC5–AC6). **Tests:** extend
  `the_envelope_classification_answers_every_row`, invert the existing
  malformed-object silence test, and add dispatch parity for missing and
  non-string methods, unknown notifications, explicit null/duplicate IDs,
  mixed batches, sixteen malformed members and invalid batch envelopes.
- [ ] 6. Prove malformed-object behavior on both real transports (#132,
  AC5–AC6). **Tests:** `malformed_objects_answer_over_http_and_stdio` in
  existing MCP/binary suites asserts each 200/-32600/null case, exactly one
  mixed-batch element, initialized 202/empty, one bridge error line and
  following ping; local transport-failure table proves matching null position.
- [ ] 7. Reuse authority parsing for default-port dialing without altering
  Host (#133, AC7). **Tests:** `bridge_authority_defaults_only_missing_ports`
  covers hostname/IPv4/bracketed IPv6 and invalid supplied ports;
  `portless_bridge_pings_port_eighty` uses real serve/mcp children on port 80;
  request-head construction test pins verbatim Host and explicit ports.
  Failure to provide the port-80 environment leaves this task unchecked.
- [ ] 8. Share HTTP oversize wording and add local pre-dial refusal with
  reusable envelope error shaping (#133, AC8–AC9). **Tests:**
  `bridge_body_limit_is_byte_exact` covers equality/one-over, multibyte text,
  delimiter trimming and parse-error precedence;
  `local_refusal_preserves_envelope_positions` covers Silent, Single,
  InvalidBatch, ordered Batch, string/null/duplicate IDs and malformed
  members; instrument actual dial entry under cfg(test) to prove zero calls
  on refusal and a call for the valid positive control.
- [ ] 9. Prove real bridge oversize output and recovery (#133, AC8–AC9).
  **Tests:** `oversized_bridge_requests_keep_ids_without_effects` sends 3 MiB
  and 40 MiB ID-7 requests, a two-request batch and notification/mixed
  variants; assert exact line/array counts, -32600/shared HTTP-413 wording,
  no effects and following ping. Retain concurrent wait/seed, EOF drain,
  malformed UTF-8 recovery, stdout failure and transport-error regressions.
- [ ] 10. Update README/module contracts, stale comments, decision index and
  dated proposal pointers (#129/#130/#132/#133, AC3/AC4/AC9).
  **Proof:** existing documentation contract tests plus a review matching
  exact lock exclusions, overflow spelling, notification rule, default port,
  input/output/version-skew residuals and proposed status to 0047; accepted
  text is unchanged and no implementation claim precedes behavior/tests.
- [ ] 11. Capture a behavioral red on `2a325c9` behavior and a green for
  every repaired behavior (AC10). **Tests:** run steps 2–9 against baseline
  and repaired behavior, recording named assertions; if private fixtures
  require a behavior-restored control, state it. The 40 MiB proof must fail
  for old network-dependent code/message/no-dial behavior, not merely check
  ID 7, which the baseline broken-pipe path may already preserve.
- [ ] 12. Run final-tree formatting, locked clippy/tests, exact coverage and
  selftest, publish-guard selftest, license check and native-tls absence
  proof (AC10); verify runs pinned brokkr compile. **Proof:** fresh exit/status
  receipts and exact zero-missed coverage, with formatting before each check,
  no TMPDIR override or dependency allowlist widening. Required blocked gates
  remain unproven; no previous batch's receipt substitutes for this tree.
- [ ] 13. Sign the implementation commit with an imperative subject citing
  #129, #130, #132, #133 and a body mapping each issue to its actual tests.
  **Proof:** signature verification, final diff review, `git status --short`
  clean on `epic120-storage-rpc`; no pushes, PRs, merges or issue closures.
