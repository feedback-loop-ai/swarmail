# 0106 — MCP resources and subscriptions: evaluated, not shipped, metric not measured

- **Status:** proposed
- **Date:** 2026-10-31
- **Scope:** Epic #164 issue #158 (P3), delivered from main `d892acb` on the
  zero-Codex `claude-careful` recipe (0098/0099, rework per 0102).
- **Specification:** [MCP resources evaluation](../../specs/mcp-resources-evaluation/spec.md).
- **Preserves:** 0001–0005; accepted 0021 (watcher ring, lag repaired by
  rescan), 0043 §4 (`GET /mcp` is `405`, no CORS), 0044, 0047; proposed
  0079/0080 (no session registry, no SSE or session machinery on the MCP
  endpoint), 0094 (bounded `0..60000` ms waits). Nothing here accepts an
  earlier proposal.
- **Number:** provisional 0106, the next free number on `d892acb`;
  collisions at integration are renumbered the way 0105 was.

## Context

Issue #158 asks for inboxes and emails as MCP resources, `resources/subscribe`
with `notifications/resources/updated` on arrival, and an evaluation of the
2025-11-25 tasks primitive for long waits. Its single acceptance criterion is
a gate, not a feature request: *ship it only if the harness shows a gain, or
if a client in the matrix needs it; otherwise close with the data.*

The design panel (simplicity and robustness positions, run
`issue-158-expose-inboxes-as-mcp--1bcfceba`) reached the same verdict from
opposite directions, and the chief checked their facts against the tree:

1. **The metric is not measured.** The head-to-head (wait tool versus
   subscription) needs paid model trials, which are operator-owned
   (`evals/README.md`: the paid baseline remains operator-owned) and
   forbidden in this run. The wait-tool arm has no data either:
   `docs/evals/README.md` (status 2026-10-30) records *no genuine #151
   baseline*. Commit `d85584e`, labelled #151, delivered the fixture-http
   `_meta` admission and the #151 notes, not a baseline run. The harness has
   no arm variable, its MCP client is `POST`-only (`evals/src/mcp.rs`), and
   scoring cannot attribute an outcome to which wait surface an agent used.
   **Not measured is not "no gain."**
2. **No matrix client is shown to need resources.** Intake cited
   `evals/wrappers.md:111`, which describes the fixture helper's accepted
   methods, not the clients. The supportable evidence is narrower and still
   sufficient: the Claude recipe launches with `--tools ""` and allows only
   named `mcp__swarmail__*` and `mcp__fixture__eval_http` tools
   (`evals/src/wrappers.rs`), no catalogue goal or oracle uses a resource, and
   nothing in the eval sources or wrapper docs mentions one. Authenticated
   client receipts (Claude Code, Codex, Inspector, Desktop) remain
   operator-owned and outstanding. **No need is shown; no need is proved.**
3. **Push already exists where the product can keep its promise.** The REST
   feed `GET /api/v1/inboxes/{inbox}/feed` is a server-sent-event stream that
   subscribes before its first scan (`src/api.rs`, `inbox_feed`; 0002) and
   rides the per-inbox watcher ring (0021). What #158 asks for is an MCP
   spelling of a capability the server already has, on an endpoint whose
   `GET` is `405` by accepted ruling.
4. **The small MCP design is a different, weaker product.** Advertising
   `resources.subscribe` on a `POST`-only endpoint is a per-revision lie
   (`src/mcp.rs:81`, `:383`); a stdio notification path still needs the HTTP
   stream underneath it, because the bridge is one `POST` per line with
   `Connection: close` (`src/stdio.rs`); a subscription is state on a server
   that 0079 keeps stateless; a notification per arrival does not fit a
   16-slot wake ring that coalesces by design; a body in the notification
   multiplies one hostile `DATA` by the subscriber count on the `250` path;
   and the client's list-then-subscribe flow re-opens, in client code, the
   check-then-subscribe gap 0002 closed on the server.

## Proposed ruling

1. **No resources surface, no subscriptions and no tasks land for #158.**
   `initialize` keeps advertising exactly `{"tools": {}}` on every revision;
   `resources/list`, `resources/read`, `resources/templates/list`,
   `resources/subscribe`, `resources/unsubscribe` and the `tasks/*` methods
   keep answering `-32601` over HTTP and stdio. The refusal is pinned by name
   in `tests/mcp_revisions.rs` so that the record is checkable, not merely
   stated.
2. **The metric is recorded as NOT MEASURED, with its reason** (paid trials
   forbidden; no baseline arm exists), never as "no gain" and never as a
   ship claim. Closing #158 is the operator's act; this record supplies the
   data the issue asked for, which is the absence of data and the reasons.
3. **Two reopen triggers, neither a standing permission.** (a) A recorded
   receipt in which a matrix client fails or degrades without resources.
   (b) The operator-owned #151 baseline's failure taxonomy attributes
   `wait_timeout` or long-wait failures to the wait surface, and the operator
   then authorises a head-to-head. The baseline costs nothing extra once it
   exists; read it before spending the second arm's 480 slots.
4. **If a trigger fires, the shape is fixed, not the small design.** A new
   proposed decision reconciling 0079 and 0080 must carry: inbox-only
   resources under the same inbox admission as the tools (0004); a push
   transport that is a `GET /mcp` stream behind the 0043 origin guard
   (stdio push rides it; no third transport); notifications carrying URI and a
   monotonic per-inbox revision only, never a body (0005's read limits stay on
   `resources/read`; a pruned id answers a typed "gone", never a stale body);
   `resources/subscribe` returning the scan's revision after subscribing
   before scanning (0002 on the server, and the revision is what closes the
   client-side gap); fan-out riding the existing per-inbox broadcast with
   per-subscriber filtering on the subscriber's task, so `Store::insert`
   stays O(1) in subscribers (0001); an RAII guard, a per-session cap and a
   `swarmail_subscriptions_active` gauge; coalescing notifications ("at least
   one after the last commit", never one per mail). Every production line
   ships with its test under the exact gate.
5. **2025-11-25 tasks: evaluated, not adopted.** Swarmail's waits are
   read-only, bounded at `0..60000` ms (0094), and resumable by re-calling with
   `since_ms`: a retried wait is the poll a task would otherwise provide, and
   the answer is already durable in the store. Task identity, TTL, status and
   result retrieval would be retained server state on a server with no
   sessions. The catalogue keeps no `tasks` capability and no
   `execution.taskSupport`, so the revision's default (`forbidden`) applies
   and clients cannot request task-augmented calls. This evaluation rests on
   the clause ledger in `specs/mcp-version-negotiation/reconciliation.md`
   (ToolExecution, TaskAugmentedRequestParams: *inapplicable, default
   forbidden*); the official source snapshot (0080, commit `dae694f…`) is not
   in the tree and this seat could not fetch it, so the normative text was not
   re-read here. A later run that cites task details must read the snapshot.
6. **An observed risk for the baseline, not reproduced here.** The Codex and
   dsh wrappers set a 60 s client tool timeout (`tool_timeout_sec`,
   `toolCallTimeoutMs` in `evals/src/wrappers.rs`), equal to the server's
   maximum `timeout_ms`, so a `60000` ms wait can race the client. If the
   baseline shows `wait_timeout` failures, check this first: the
   proportionate remedy is guidance or wrapper configuration, not a second
   delivery path.

## Panel adjudication

| Question | Ruling | Cost to the losing side |
|---|---|---|
| Ship protocol code | Both positions: no. Adopted. | Whoever wanted the feature waits for a trigger. |
| Would a measured gain flip the verdict | Robustness said no, even then. **Rejected**: the issue's gate belongs to the operator, and a record that forbids the operator's own criterion is not a record. Adopted instead: a measured gain reopens only into the fixed shape of ruling 4. | Robustness loses the permanent refusal; it keeps every constraint that made the small design unsafe. |
| What "close with the data" can mean | Simplicity: the data is that no data exists, said plainly. Adopted, with the chief's correction that the #151-labelled commit is not a baseline. | The issue's phrase is only partly satisfiable; the record says so instead of pretending. |
| Pin the refusal by name | Simplicity: already pinned generically (`-32601` for unknown methods). Robustness: the capability must not lie per revision. **Ruled**: one test-only witness names the resource and task methods on all three revisions over HTTP and stdio. | A few test lines for a feature that does not exist; zero production lines, so the exact gate is unaffected. |
| Where push belongs | Chief's addition: the REST feed already is the product's push surface. Neither position cited it. | A future MCP push must reuse `feed::feed_stream`'s discipline, not invent a second subscribe-then-scan owner. |
| Tasks primitive | Both: evaluate only, do not adopt. Adopted, with the snapshot caveat recorded. | A client that wants task-augmented calls gets `forbidden` by default. |

## Consequences

- `src/`, `evals/src/`, `deny.toml` and both lockfiles are unchanged by this
  delivery; the coverage denominator does not move. The change is this record,
  its index row, the specification, one named test witness and one risk note.
- The reconciliation ledger's "inapplicable" rows for resources and tasks stay
  inapplicable with a reason, rather than becoming live obligations across
  three revisions.
- 0045 §8 applies: nothing here contradicts an accepted ruling, so the
  docs-only change and the test witness may land while this record is
  proposed. Only the operator accepts it, and only the operator closes #158.
