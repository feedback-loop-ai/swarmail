# Evaluate MCP resources and subscriptions for #158

Decision: 0106 — [MCP resources evaluation](../../docs/decisions/0106-mcp-resources-evaluation.md)

## Feature and rationale

Issue #158 (epic #164, P3) asks swarmail to expose inboxes and emails as MCP
resources, add `resources/subscribe` with `notifications/resources/updated` on
arrival, and evaluate the 2025-11-25 tasks primitive for waits longer than the
tools allow. Its acceptance criterion is a gate: ship only if a head-to-head
harness run shows a gain over the wait tools, or if a client in the eval
matrix needs resources; otherwise close with the data.

This design rules that the gate stays shut and delivers the record the gate
asks for. Nothing under `src/` or `evals/src/` changes. The delivery is:

1. proposed [0106](../../docs/decisions/0106-mcp-resources-evaluation.md),
   which records the metric as NOT MEASURED with its reason, the matrix-client
   finding, the tasks evaluation, two reopen triggers and the fixed shape any
   reopened build must take;
2. one test-only witness naming the resource and task methods as `-32601` on
   every revision over HTTP and stdio, so the record is checkable by name;
3. one risk note for the operator-owned baseline (the 60 s client timeout that
   equals the server's wait cap).

Design baseline is `d892acb`, which carries the merged MCP work of #152, #153
and #154. The chief read both run-local positions
(`.brokkr/state/design/positions/simplicity.md` and `robustness.md`) and the
resolved intake result; `.brokkr/state/tasks/` is absent in this worktree, so
the task block and the intake result are the framing. Both positions reached
"do not ship"; the adjudication below records where they differed and what the
chief corrected.

## Findings: the data

| Question | Finding | Evidence |
|---|---|---|
| Harness gain | **Not measured.** The head-to-head needs paid trials, which are operator-owned and forbidden in this run. The wait-tool arm has no data either. | `evals/README.md` (paid baseline operator-owned); `docs/evals/README.md` status 2026-10-30: no genuine #151 baseline. Commit `d85584e` (labelled #151) delivered fixture-http `_meta` admission, not a baseline run. |
| Could the harness measure it today | No. No arm variable; the runner's MCP client speaks only `POST /mcp` (`evals/src/mcp.rs`); scoring cannot attribute an outcome to the wait surface used; a second arm at the baseline's shape is another 480 paid slots. | `evals/run-plan.md:36-38` |
| Matrix client need | **No need shown.** The Claude recipe launches `--tools ""` allowing only named `mcp__swarmail__*` and `mcp__fixture__eval_http` tools; no goal, oracle or wrapper document uses a resource. Intake's citation of `evals/wrappers.md:111` describes the fixture helper, not the clients. Authenticated client receipts remain operator-owned and outstanding. | `evals/src/wrappers.rs`, `evals/wrappers.md` |
| Current contract | `initialize` advertises exactly `{"tools": {}}` on all three revisions; `GET /mcp` is `405 Allow: POST` (0043 §4); unknown methods answer `-32601`. | `src/mcp.rs:383`, `src/api.rs:1940`, `tests/mcp_revisions.rs:118`, `tests/mcp.rs:2319` |
| Existing push | The REST feed `GET /api/v1/inboxes/{inbox}/feed` already streams arrivals as server-sent events, subscribing before its first scan (0002) on the per-inbox watcher ring (0021). | `src/api.rs:837`, `src/api.rs:1605` |
| Stdio push | Not independent of HTTP: the bridge is one `POST` per line with `Connection: close`, so any stdio notification needs an HTTP stream underneath. | `src/stdio.rs:117` |
| Wake channel | A 16-slot wake token ring that lags, never blocks, and is repaired by rescan. "One notification per arrival" does not fit it; a coalescing promise does. | `src/store.rs:38-44` |
| 2025-11-25 tasks | Evaluated, not adopted. Waits are read-only, bounded `0..60000` ms (0094) and resumable with `since_ms`; a task would add retained server state (identity, TTL, status, result) to a server 0079 keeps stateless. The catalogue declares no `tasks` capability and no `execution.taskSupport`, so the revision's default (`forbidden`) applies. | `specs/mcp-version-negotiation/reconciliation.md:118,672,679` |
| Tasks normative text | The official source snapshot (0080, commit `dae694f…`) is not in the tree and this seat holds no fetch capability; the evaluation rests on the clause ledger. Stated as a limit, not hidden. | `docs/decisions/0080-mcp-source-reconciliation.md:16` |
| Observed risk | Codex and dsh wrappers set a 60 s client tool timeout, equal to the server's maximum `timeout_ms`; a `60000` ms wait can race the client. Observed in configuration, not reproduced. | `evals/src/wrappers.rs:671,718`; `src/mcp.rs:1019` |

## Panel adjudication

| Question | Ruling and rejected position | Cost retained |
|---|---|---|
| Ship protocol code | Both: no. Adopted. | The feature's wanter waits for a trigger. |
| A measured gain still would not flip the verdict (robustness) | **Rejected.** The gate is the operator's criterion; a record that forbids it is not a record. A measured gain reopens, but only into the fixed shape of 0106 ruling 4. | Robustness keeps every constraint that made the small design unsafe, and loses the permanent refusal. |
| "Close with the data" when there is none (simplicity) | Adopted: say plainly that the head-to-head data does not exist and why. Corrected: the #151-labelled commit is not a baseline. | The issue's phrase is only partly satisfiable; the record says so. |
| Pin the refusal by name | Simplicity: the generic `-32601` pin suffices. Robustness: the capability must not become a per-revision lie. **Ruled for one test-only witness** naming the methods on three revisions over HTTP and stdio. | A few test lines; zero production lines, so the exact gate is untouched. |
| Push surface | Chief's addition: the REST feed is already the product's push surface, uncited by either position. | A future MCP push reuses `feed::feed_stream`'s subscribe-then-scan discipline rather than growing a second owner. |
| Tasks | Both: evaluate only. Adopted, with the snapshot limit recorded. | A client wanting task-augmented calls meets `forbidden`. |

## Contract

What this delivery changes, and what it does not:

- **Unchanged:** `src/`, `evals/src/`, `Cargo.toml`, `evals/Cargo.toml`,
  both lockfiles, `deny.toml`, the OpenAPI document, the tool catalogue, the
  reconciliation ledger and every historical spec.
- **Added:** proposed 0106 and its index row in `docs/decisions/README.md`;
  this specification; one named witness in `tests/mcp_revisions.rs`; one
  dated risk note in `evals/run-plan.md`; a delivery receipt
  `specs/mcp-resources-evaluation/implementation.md`.
- **Witness shape:** for each of `resources/list`, `resources/read`,
  `resources/templates/list`, `resources/subscribe`, `resources/unsubscribe`,
  `tasks/get`, `tasks/list`, `tasks/result`, `tasks/cancel`, and for each
  negotiated revision (2025-03-26 default, 2025-06-18, 2025-11-25), the HTTP
  reply and the binary stdio reply carry the request id and error code
  `-32601`, and `initialize` on that revision advertises capabilities equal to
  `{"tools": {}}`. The witness uses the suite's existing `post`, `Binary` and
  `initialize` helpers; no production seam.

## Acceptance Criteria

- **AC1 — Metric recorded honestly.** 0106 states the head-to-head metric
  as NOT MEASURED, names the reason (paid trials forbidden; no baseline arm),
  and nowhere claims "no gain" or a ship. *Proof:* the record's text; the
  review panel reads it against this criterion.
- **AC2 — No protocol surface lands.** `git diff d892acb..HEAD --stat`
  touches nothing under `src/` or `evals/src/`, no `Cargo.toml`, lockfile or
  `deny.toml`. *Proof:* the receipt records that diff stat; the verify seat
  reproduces it.
- **AC3 — The refusal is pinned by name.** A test in `tests/mcp_revisions.rs`
  proves the witness shape in *Contract* for every listed method on all three
  revisions, over real HTTP and binary stdio. *Proof:*
  `cargo test --locked --test mcp_revisions` green, with the test's name in the
  receipt.
- **AC4 — The capability pin holds.** `tests/mcp_revisions.rs:118` and
  `tests/mcp.rs:2319` continue to assert capabilities exactly `{"tools": {}}`
  and no `resources` key. *Proof:* `cargo test --locked --test mcp --test
  mcp_revisions` green.
- **AC5 — Tasks evaluated, not adopted.** 0106 ruling 5 exists, cites the
  ledger rows, states the snapshot limit, and no `tasks` capability or
  `execution` metadata appears in the catalogue. *Proof:* AC3's witness
  (`tasks/*` answer `-32601`) and the unchanged catalogue under AC2.
- **AC6 — Reopen triggers recorded.** 0106 ruling 3 names the two triggers
  and ruling 4 fixes the shape a reopened build must take, including
  URI-only notifications, O(1) `insert`, a per-inbox revision and the 0043
  guard on any `GET /mcp` stream. *Proof:* the record's text.
- **AC7 — Decision index and links.** `docs/decisions/README.md` carries the
  0106 row with status "proposed — awaiting the operator"; every relative link
  in this spec and in 0106 resolves to a tracked file; nothing links into
  delivery state. *Proof:* `cargo test --manifest-path evals/Cargo.toml
  --locked --test boundary` green.
- **AC8 — Baseline risk noted.** `evals/run-plan.md` carries a dated note
  that the Codex and dsh 60 s client tool timeouts equal the server's wait cap,
  marked observed-not-reproduced, for the baseline to confirm or dismiss.
  *Proof:* the note's text; the evals boundary test still green.
- **AC9 — Gates clean.** Every gate in `docs/house-rules.md` § Gates passes
  on the committed tree, locked, including the evals crate's fmt, clippy and
  tests, the coverage gate with its selftest, publish-guard, seal and
  `cargo deny`. *Proof:* the receipt lists each command and its exit status;
  the verify seat reproduces them.
- **AC10 — Frozen guarantees and dependencies untouched.** 0001–0005 are not
  amended; no dependency is added or widened. *Proof:* AC2's diff stat and
  `cargo deny check licenses`.

## Out of scope, and why

- `resources/list`, `read` and `templates` without subscribe: a second read
  surface beside the tools that does not move the metric.
- Email resources: mail is immutable once stored; there is nothing to notify.
- A `GET /mcp` stream, session or subscription state, a stdio push reader:
  each is the machinery 0079 and 0080 declined, and each needs its own
  proposed decision when a trigger fires.
- A harness arm, wrapper change or prototype: adds a variable to every row
  with no authorised run to feed it.
- Reproducing the 60 s client-timeout race: it belongs to the baseline, not
  to this issue.
