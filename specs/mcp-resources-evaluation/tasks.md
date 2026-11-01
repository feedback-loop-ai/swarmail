# Ordered tasks — MCP resources evaluation

These are implementation and proof obligations, not completed work. Check a
task only after its paired witness is recorded against the actual candidate.
The chief commits the design and proposed 0114; the engine's deterministic
check owns the design verdict. Dependencies follow this order.

- [ ] **T1 — Add the 0114 index row (AC7).** Append one row to
  `docs/decisions/README.md` after 0105, status
  "proposed — awaiting the operator", citation "(Epic #164, issue #158;
  provisional number)". **Test:**
  `cargo test --manifest-path evals/Cargo.toml --locked --test boundary` green
  (tracked links resolve; nothing links into delivery state).
- [ ] **T2 — Pin the refusal by name (AC3, AC4, AC5).** Add one test to
  `tests/mcp_revisions.rs` that, for each negotiated revision (absent-version
  default, 2025-06-18, 2025-11-25), asserts `initialize` capabilities equal
  `{"tools": {}}` and that `resources/list`, `resources/read`,
  `resources/templates/list`, `resources/subscribe`, `resources/unsubscribe`,
  `tasks/get`, `tasks/list`, `tasks/result` and `tasks/cancel` answer error
  `-32601` carrying the request id, over HTTP and binary stdio, using the
  suite's existing helpers. **Test:** the new test itself and the existing
  pins, `cargo test --locked --test mcp --test mcp_revisions` green; the
  test's name goes in the receipt.
- [ ] **T3 — Note the 60 s client-timeout risk for the baseline (AC8).** Add a
  dated paragraph to `evals/run-plan.md` stating that the Codex
  `tool_timeout_sec` and dsh `toolCallTimeoutMs` values equal the server's
  maximum `timeout_ms`, that a `60000` ms wait can therefore race the client,
  that this is observed in configuration and not reproduced, and that the
  baseline's `wait_timeout` rows should be read against it first. **Test:**
  the evals boundary test green; the paragraph quoted in the receipt.
- [ ] **T4 — Prove nothing else moved (AC2, AC10).** Record
  `git diff d892acb..HEAD --stat` showing no path under `src/`, `evals/src/`,
  `benches/`, `charts/`, no `Cargo.toml`, lockfile or `deny.toml`; the only
  Rust change is the test file of T2. **Test:** `cargo deny check licenses`
  green and the diff stat in the receipt.
- [ ] **T5 — Run the inventory and write the receipt (AC9, AC1, AC6).** Run
  every gate in `docs/house-rules.md` § Gates on the committed tree, locked,
  including the evals crate's fmt, clippy and tests, the coverage gate and its
  selftest, publish-guard, seal and `cargo deny`; write
  `specs/mcp-resources-evaluation/implementation.md` with each command, its
  exit status, the coverage summary, T2's test name, T3's paragraph and T4's
  diff stat; state that the metric remains NOT MEASURED and that closing #158
  is the operator's act. **Test:** every listed command exits 0; the receipt
  names any that did not.
