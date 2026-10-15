# Run 9 — intake framing: the validation family (#46, #40, #38, #41, #47)

## Goal (one sentence)

Across every plane — REST, the MCP JSON-RPC envelope, the stdio bridge and
the webhook fan-out — a supplied value that is present but wrong refuses
with the shared 0025 posture (naming field and accepted range) instead of
silently defaulting, and the two MCP planes become conformant transports of
what they advertise; delivered in the order **#46 → #40 → #38 → #41 → #47**
(the envelope must be conformant before the bridge can be a faithful
client of it).

## Delivery order and dependencies

1. **#46 first** — the server's JSON-RPC envelope (arguments, error codes,
   notifications, batches, `initialize`) is the contract the bridge
   implements; a faithful bridge (#40) is meaningless against a
   non-conformant server.
2. **#40** — the stdio bridge: deadline, notification silence, error
   mapping, stderr logging, slash-safe URL join.
3. **#38** — REST `await`/assert `since_ms`+`limit` parse (the 0025
   residual class), openapi.json pinned to the full filter set.
4. **#41** — chaos `error` validation; **lands its decision amending 0025
   in the batch** (status `proposed`; the operator accepts post-review).
5. **#47** — per-target webhook budgets and `PUT /api/v1/webhooks`
   validation.

## Frozen-guarantee proximity (0001–0003)

- **#41 is the only issue that touches a frozen guarantee — 0001
  (accept == stored).** `{"probability":100,"error":"250 2.0.0 OK"}`
  answers `250` for DATA while the chaos arm continues before the insert:
  mail the client saw accepted was never stored, broken through the
  control plane. The fix must keep 0001's shape: a fired DATA rule never
  answers 2xx, and the refusal/chaos arm ordering around
  `Store::insert` is untouched.
- **#38 and #46's validation refusals sit near 0002.** Per 0025 §5, a
  refused call registers no waiter: the new `since_ms`/`limit`/argument
  refusals must precede subscribe, not merely precede the reply.
- **#47 is adjacent to 0001's honesty muscle** (a mail delivered to no
  sink must be counted, not silently lost) and to 0024's shutdown
  ledger; it must not touch 0003 — pruning is oldest-only and out of
  scope. `WEBHOOK_DELIVERY_CAP`'s shared pool is not itself a frozen
  guarantee, but the per-target budget must not weaken the bounded
  fan-out (0027's sizing posture).
- No frozen guarantee is weakened by this batch; nothing here needs to
  amend 0001–0003. The one decision in play amends **0025** (not frozen)
  and is explicitly mandated by the task.

## Acceptance criteria (checkable claims)

### #46 — MCP envelope + wrong-typed arguments (`src/mcp.rs`; `tests/mcp.rs`, `tests/api.rs`)

1. A supplied wrong-typed argument — wrong JSON type or explicit `null`
   — refuses with the shared validation sentence naming the field and
   accepted range, on every tool that reads it (`arg_str`, `arg_usize`,
   the `as_i64` `since_ms` reader). `swarmail_clear_inbox {"inbox":42}`
   returns an error and does **not** clear `default`;
   `swarmail_wait_for_email {"to":["a@x.io"]}` refuses rather than
   dropping the filter and false-greening.
2. A batch array (`[...]` at the POST body top level) is either executed
   per the advertised `2025-03-26` revision (each entry answered) or
   refused with the `2025-06-18` revision negotiated in `initialize` —
   never an empty `202`. A test pins which behavior the advertised
   revision carries.
3. `{"jsonrpc":"2.0","id":5}` with no `method` answers error `-32600`
   carrying the id — never an empty `202`.
4. A request with a missing `id` is a notification: no response body at
   all — no `"id":null` answer. `tools/call` without an id produces no
   reply.
5. Error codes match the spec: `-32602` for invalid params, `-32700`
   with a JSON-RPC error body for malformed JSON (replacing axum's
   plain-text 400), `-32601` **only** for unknown methods/tools.
6. `initialize` negotiates `protocolVersion` honestly (refuses or
   downgrades a mismatch per the advertised revision) and does not
   advertise `resources: {}` while `resources/list` is unknown.
7. MCP `swarmail_search_emails` `total` equals REST's full-match-count
   semantics, and the default limit is one shared constant on both
   planes (or the divergence is pinned by a test and a comment saying
   why).

### #40 — stdio bridge is a faithful NDJSON transport (`src/stdio.rs`; `tests/stdio.rs`)

1. The bridge's deadline covers the longest wait its tools accept
   (60000 ms): a stdio test waits **longer than 10 s** and gets its
   result. No clamp that ends in an error (0018: refuse, never clamp) —
   the bridge derives its budget from the request or has no short
   clamp; the `READ_TIMEOUT` constant as a silent ceiling is gone.
2. A notification produces **no stdout output** — and the test stops
   filtering blank lines (the filter's removal is part of the change, so
   a bare `\n` would go red).
3. Non-2xx or non-JSON HTTP responses become JSON-RPC error objects
   **carrying the request id** — axum plain-text, 404, 413, 415 never
   reach stdout verbatim.
4. The bridge's own logs go to stderr (at least for the `mcp`
   subcommand); stdout is byte-clean protocol. The test asserts it
   without tolerating blank lines, and the binary startup-line contract
   (`smtp=` literal, plain when piped) stays intact.
5. The base-URL join is slash-safe: a trailing slash in the base URL
   posts to `/mcp`, not `//mcp` (tested with a trailing-slash base).

### #38 — REST await/assert accept `since_ms`+`limit` (`src/api.rs`; `tests/api.rs`)

1. `since_ms` and `limit` parse on `await` and assert exactly as on
   list — real-HTTP tests pass both to await and assert and observe the
   filtered result (today: `invalid type: string "0", expected i64`).
2. A malformed supplied value refuses with 400 and the **same shared
   sentence** list uses (0025); an omitted value keeps its default.
3. `openapi.json` documents the full filter set assert/await accept,
   pinned by the inline test pattern of 0025 §6 (served openapi.json ==
   the documented filter set).

### #41 — chaos `error` is validated wire text (`src/chaos.rs`, `src/smtp.rs`, `src/api.rs`, `src/mcp.rs`; `tests/smtp_edges.rs` + a real SMTP test)

1. `ChaosRule::validate` refuses an `error` that is not a single-line
   4xx/5xx SMTP reply: any CR or LF refuses; a leading code outside
   4xx/5xx refuses; the refusal sentence is **byte-identical** from REST
   and MCP (both call the one validator in `src/chaos.rs`).
2. A real SMTP test proves a fired DATA rule (e.g. probability 100,
   `error: "450 4.7.1 no"`) never answers 2xx and the mail is not
   stored (0001 held).
3. REST `PUT /api/v1/chaos` refuses unknown chaos events exactly as MCP
   refuses an unknown event — same rule, same sentence.
4. The decision amending 0025 lands **with the batch**, status
   `proposed`: the chaos `error` is wire text, so its validation is a
   domain rule; the decision also records the `set_chaos`
   read-modify-write concurrency gap honestly — serialized, or named as
   a residual with its file:line map.
5. Tests use chaos at the poles only (0/100), per-test fresh
   stores/inboxes, no sleep-as-synchronization (0020 disciplines).

### #47 — per-target webhook budgets + PUT validation (`src/webhook.rs`, `src/api.rs`, `src/config.rs`; `tests/webhook.rs`)

1. A dead or stalled target cannot cause drops for a healthy one:
   per-target budget (or provable fair share), pinned by a two-sink test
   — one stalled sink + a burst, the healthy sink receives all its mail
   and counts no `aborted`.
2. `PUT /api/v1/webhooks` refuses with 400 + the shared 0025 sentence:
   non-http(s) or unparsable URL, malformed `ca_pem`, secret containing
   CR/LF.
3. Delivery failure causes are visible at the default log level or in
   `/metrics` — a test asserts observability without enabling debug.
4. `GET /api/v1/webhooks` redacts `secret` and `ca_pem`, or the
   exposure is documented as deliberate (in the decision/comment) and
   pinned by test.
5. A commit that finishes during shutdown is either delivered or counted
   `aborted` — never silently lost (test against 0024's fence; do not
   re-litigate the 10 s fence or the stalled-body fence+ε budget of
   0027 §7 / `6e845aa`).

## Standing constraints carried to design

- Do not edit `agents/`, `adapters/`, `bundles/` — operator edits go to
  `.forge/run9-operator-edits.md`.
- No issue closes, no PRs; the operator lands and cites CI.
- Frozen CI context names keep their exact strings.
- 0020 binds every test edit: cold-path tests in
  `src/<module>/<name>_tests.rs` under `#[cfg(test)]`; harness names are
  reserved; every new production line is measured by the exact gate.
- `src/store.rs`/`src/persist.rs` stay at zero diff (0025's boundary);
  validation is pure and sits above the boundary (0014/0015).
- The tracing subscriber emits ANSI only on a terminal; the bridge's
  stdout must stay protocol-clean for its own client tests too.
- A refused seed may lawfully see EOF (`913e097`) — do not re-pin the
  503.

## Verify gate

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked          # plus targeted:
#   cargo test --test mcp --test stdio --test api --test webhook --test smtp_edges
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh            # exact 100/100/100 — floor may rise, never fall
cargo deny check licenses
```

Verify seat additionally: the coverage selftest, the publish-guard proof
and a fail-closed native-tls probe, with network denied, under a 3-hour
budget.

## Blocked check

Nothing in the batch contradicts a frozen guarantee — #41 *restores*
0001's control-plane reach and #38/#46's refusals reinforce 0002's
refuse-before-subscribe posture. The one decision required (#41's 0025
amendment) is explicitly mandated and lands `proposed` with the batch,
which is the lawful path. **Resolved.**
