# Implementation plan — MCP resources evaluation

Implements [spec.md](spec.md) under proposed
[0106](../../docs/decisions/0106-mcp-resources-evaluation.md).

## Technical context

One Rust crate plus the `evals` crate; real-protocol integration suites under
`tests/`. The delivery adds no production code, no dependency and no feature
flag. The named `.specify/templates/plan-template.md` is absent in this
checkout; this file supplies the required technical context, constitution
check, structure, research, data model, contracts, quickstart and risks.

This is a design-only commit. It commits the spec, plan, tasks and the
proposed decision; it does not add the index row, the witness test, the risk
note or the receipt, and it attests no gate. The engine's deterministic check
rules the design artifacts; implementation, verify and protected review remain
owed.

## Constitution check

- 0001–0005 untouched: no store, await, prune, protocol or extraction code
  moves. The nearest guarantee, 0002, keeps exactly one subscribe-then-scan
  owner per wait path and one for the REST feed; no second owner is added.
- 0043 §4: `GET /mcp` stays `405` with no CORS. 0079/0080: no session
  registry, no SSE or session machinery on the MCP endpoint. 0094: wait
  bounds unchanged.
- 0045 §8: the change preserves every accepted ruling, so it may land while
  0106 is proposed. Only the operator accepts 0106 and closes #158.
- 0020: the witness is a public-API, wire-shaped test and lives in `tests/`
  (rule 3); it needs no crate-private seam. The coverage denominator is
  unchanged because no production line is added; the exact gate still runs.
- Dependencies: none added; `deny.toml` and both lockfiles unchanged.
- Commits unsigned, never pushed (0056).

## Project structure

| Path | Change |
|---|---|
| `docs/decisions/0106-mcp-resources-evaluation.md` | the proposed record (this commit) |
| `docs/decisions/README.md` | one index row for 0106 (implementation, T1) |
| `specs/mcp-resources-evaluation/{spec,plan,tasks}.md` | this design (this commit) |
| `specs/mcp-resources-evaluation/implementation.md` | the delivery receipt (implementation, T5) |
| `tests/mcp_revisions.rs` | one named witness for resource and task methods (implementation, T2) |
| `evals/run-plan.md` | one dated risk note on the 60 s client timeout (implementation, T3) |

No file under `src/`, `evals/src/`, `benches/` or `charts/` changes.

## Research and decisions

- **Why no code:** the issue's gate (measured gain or matrix need) is unmet
  and cannot be met in this run; both panel positions and the chief agree.
  Details and evidence are in the spec's *Findings* and in 0106.
- **Why a named witness anyway:** the generic unknown-method pin already
  covers these methods, but the record claims a specific refusal on specific
  methods across revisions and transports. A test that names them turns the
  claim into something a reader can run. Test-only: it adds nothing to the
  production denominator.
- **Why the REST feed matters:** `inbox_feed` (`src/api.rs:1605`) already
  gives the product a push surface with the right discipline. A reopened
  design must reuse `feed::feed_stream`'s subscribe-then-scan shape and the
  0021 ring rather than grow a second owner. Recording that now saves a
  future panel the rediscovery.
- **Why not reproduce the 60 s timeout race:** it needs an agent CLI in the
  loop or a paid trial; it is the baseline's business. A dated note is the
  proportionate action.
- **Tasks snapshot limit:** the 0080 source snapshot is outside the tree and
  this seat cannot fetch; the evaluation cites the ledger and says so.

## Data model

None. No type, schema, table or wire shape is added or changed.

## Contracts

- **MCP:** unchanged. Capabilities remain `{"tools": {}}`; the listed
  `resources/*` and `tasks/*` methods answer `-32601` with the request id,
  on 2025-03-26 (and absent-version default), 2025-06-18 and 2025-11-25, over
  HTTP and binary stdio.
- **REST, SMTP, POP3, webhooks, OpenAPI:** unchanged.
- **Decision index:** a row whose status column reads
  "proposed — awaiting the operator", matching the record's own status
  header.

## Quickstart (implementation seat)

```bash
# 1. Index row for 0106, after the 0105 row, in the index's style.
# 2. Witness in tests/mcp_revisions.rs using the suite's post/Binary/initialize helpers.
cargo test --locked --test mcp_revisions
# 3. Dated risk note in evals/run-plan.md.
cargo test --manifest-path evals/Cargo.toml --locked --test boundary
# 4. The whole inventory, then the receipt.
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
(cd evals && cargo fmt --all -- --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked)
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh && bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
git diff d892acb..HEAD --stat
```

## Risks

| Risk | Mitigation |
|---|---|
| The record is read as "subscriptions bring no gain" | 0106 and the spec say NOT MEASURED with the reason in the first sentence of the finding; AC1 makes that wording reviewable. |
| A reader treats the docs-only delivery as closing #158 | 0106 states that closing is the operator's act; the receipt reports its own result, never `closed`. |
| The witness duplicates the generic `-32601` pin and drifts from it | It uses the same helpers and asserts the same code; if the generic refusal changes, both fail together, which is the point of naming the methods. |
| The witness accidentally exercises a new production branch and moves the denominator | No production line is added; the coverage gate's exact floor is run and its summary is recorded in the receipt. |
| The 0106 number collides at integration | Provisional, like 0105's 0091; renumbered at integration with a dated note in the record's header. |
| The index row's link text or a spec link is refused by the tracked-markdown guard | Links point only at tracked files; no link into `.brokkr/state/` or `.forge/`; the evals boundary test is in the quickstart. |
| The tasks evaluation misstates the 2025-11-25 text | The record cites the ledger rows and states that the snapshot was not re-read; any later citation of task details must read the snapshot first. |
| The 60 s client-timeout note is taken as a measured failure | It is marked observed-in-configuration, not reproduced, and is addressed to the baseline's failure taxonomy. |
| A future run builds the small design under this proposal | 0106 ruling 4 fixes the minimum shape (URI-only, O(1) insert, revision cursor, 0043-guarded stream, RAII guard, per-session cap) and requires a new proposed decision reconciling 0079/0080. |
