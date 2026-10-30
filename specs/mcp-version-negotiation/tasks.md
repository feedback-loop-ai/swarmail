# Tasks — operator-directed MCP framing

Run: `epic-164-issue-152-reframe-the-m-a4a37e5f`. Follow [spec.md](spec.md)
and [plan.md](plan.md). Local completion is proved in the
[current receipt](implementation.md#operator-directed-reframing--2026-10-26). Prior completed T1–T11
belong to the source-reconciliation run; its evidence remains in
[implementation.md](implementation.md) and the ledger's historical receipts.

- [x] T1 — Reverify evidence and scope (AC1, AC4). **Test:** packet SHA256SUMS,
  candidate engine digest and shim hashes pass; compare operator ruling and
  review-998279d4 C1 finding to this design. Confirm initial commit/tree and both
  proposed statuses; no journal, remote, sibling worktree or network access.
- [x] T2 — Reframe the existing reconciliation register (AC1, AC2). **Test:**
  source-grounded review finds exactly C1–C6 with separate March/June/November
  observations, source strength/actor, operator disposition, cost and existing
  witnesses. Check C1 status attribution, C2 error-ID distinctions, C3 external
  client duties, C4 sender tolerance, C5 example strength and C6 complete-value
  forwarding. Source hashes, clause/schema rows and W1–W9 identifiers survive.
- [x] T3 — Reconcile current owner documents and proposal pointers (AC1, AC4,
  AC6). **Test:** inspect spec/plan/tasks, ledger, 0080, 0079/index and relevant
  dated pointers: no current command to repair C1–C6, no acceptance prerequisite
  for retaining C1, no false mandate for 200, no accepted body/status rewrite.
  Historical receipts stay dated; all four live-client receipts remain owed.
- [x] T4 — Align public descriptions and scoped documentation assertions (AC3).
  **Test:** README/llms/served OpenAPI name negotiation, March-only batching,
  withheld full revision conformance and operator-chosen deviations with a tracked
  pointer; revision_served_contract passes through the real server. OpenAPI parses
  and is structurally equal to 9c0c9d4 outside description values; inspect wording
  for unchanged statuses, codes, headers, caps and schemas. Correct the C1 test
  comment without changing its 200/-32600 assertions.
- [x] T5 — Prove preservation and scope (AC2, AC4). **Test:** existing
  `cargo test --locked --test mcp_revisions` and `cargo test --locked --test stdio`
  pass; review cumulative diff from 9c0c9d4 for no executable/schema/dependency/
  lockfile/routing/gate/coverage-policy change or altered behavioral assertions.
  Check changed tracked Markdown links resolve and do not target `.forge/`.
- [x] T6 — Commit implementation unsigned and run every local gate (AC5).
  **Test:** all house gates and plan supplements pass with current commit/tree,
  commands, exit codes and log receipts, verified local engine, no compile policy
  warning and literal zero missed coverage plus passing selftest. Run every gate
  despite failures, retain honest results and rerun affected proofs after fixes.
  No push, sealing, merge, tag, publication or decision acceptance.
- [x] T7 — Finish the evidence record and reconcile completion claims (AC1–AC6).
  **Test:** final diff/status and receipt audit show all local requirements met
  on the committed candidate; any evidence-only follow-up identifies both tested
  tree and final commit without laundering old logs. Current documentation and
  checkboxes agree. Original issue AC3 remains operator-owned and owed; no claim
  of full revision conformance, issue completion or external proof is introduced.

## External work — separate from local acceptance

- [ ] E1 — Operator supplies Claude Code, Codex, MCP Inspector and Claude Desktop
  receipts. **Proof:** client/version, candidate, transport, selected revision,
  initialize/list/call results, location and verdict. Current status: owed.
- [ ] E2 — CI supplies its platform/network/chart/Docker/port-80 proofs.
  **Proof:** CI-owned candidate receipts; local tests do not stand in for them.
- [ ] E3 — Operator owns decision acceptance, #151 baseline, sealing and merge
  prerequisites. **Proof:** explicit operator records; both 0079/0080 remain
  proposed and the PR merge hold remains. This seat accepts nothing further.
