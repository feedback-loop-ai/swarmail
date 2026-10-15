# Spec — the validation family #46/#40/#38/#41/#47: refuse what is wrong, on every plane

- **Run:** run-9 (design seat) · **Feature slug:** `validation-family`
- **Issues:** #46 (MCP conformance), #40 (stdio bridge), #38 (REST
  await/assert), #41 (chaos wire text), #47 (webhook wire text, budget
  and shutdown counting) — delivered in that order, per the framing.
- **Decision:** [0029](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
  (proposed) — amends 0025; the contract statements below defer to it for
  the *why*.
- **Base:** `6072564`. **Frozen guarantees:** untouched (0001/0002/0003);
  `src/store.rs` and `src/persist.rs` stay at zero diff.

## The problem

Five surfaces accept what they cannot honor, and every one of them lies
over a wire:

1. **The MCP envelope answers for nothing** (`src/mcp.rs:31-54`). A
   methodless body (or any array — `Value::get` on an array is `None`)
   returns `None` → the endpoint's `202` *silence*. The `notifications/`
   prefix check is the wrong predicate (JSON-RPC's predicate is a missing
   `id`), so an id-less `tools/call` is answered with `"id": null`
   (mcp.rs:36). Every error is `-32601` (mcp.rs:52) — an unknown tool, a
   wrong-typed argument and a poisoned staging step share one code that
   means none of those things. Malformed JSON never reaches the handler:
   axum's `Json` rejection is a plain-text 400 (api.rs:901-915). Batches
   — required by the advertised `2025-03-26` revision — are silently
   swallowed. `initialize` echoes the client's `protocolVersion` and
   advertises `resources: {}` whose every method is unknown.
2. **The stdio bridge lies on stdout** (`src/stdio.rs`). A hard
   `READ_TIMEOUT: 10s` (:11) clamps the domain's own 60 000 ms wait cap;
   the reply is `raw.split("\r\n\r\n").nth(1).unwrap_or("{}")` (:56) — a
   fabricated body; the status line is never read, so a non-2xx body is
   printed as if it were a result; the read is uncapped; `POST {path}/mcp`
   produces `//mcp` for a trailing-slash base (:40); a notification's
   empty body is written as a bare newline; and the process-wide tracing
   init (main.rs:9-20) puts `warn!` log lines on the same stdout the
   protocol occupies.
3. **REST await/assert cannot parse their own filters**
   (`src/api.rs:381-389`). `AwaitQuery` embeds `ListQuery` under
   `#[serde(flatten)]`; under `serde_urlencoded`, flattened integers
   arrive as strings and the parse refuses *every* supplied
   `since_ms`/`limit` on the push-based wait — the feature's own filter,
   dead on arrival. openapi documents only part of the filter set, so
   the served contract and the parser disagree even where it works.
4. **Chaos `error` text is written to the SMTP wire verbatim**
   (`src/chaos.rs:53-58` checks probability only; `src/smtp.rs:993-998`
   writes `rule.error` before the insert). `{"probability":100,
   "error":"250 2.0.0 OK"}` answers `250` at DATA while storing nothing —
   a client's success path reads a forged success, and a CR/LF inside the
   rule desynchronizes the session stream. REST ignores unknown event
   keys while MCP refuses unknown events. MCP's `set_chaos` is a
   read-modify-write with nothing serializing the three steps
   (mcp.rs:260-296).
5. **Webhooks accept any wire text and drop silently**
   (`src/api.rs:886-892` validates nothing; `src/webhook.rs:51` has one
   global `JoinSet` budget). A dead target can consume the shared 32
   slots and cause drops for a healthy one; a URL without a scheme is
   accepted at PUT and refused only at delivery (after the mail is
   committed); a garbage `ca_pem` is accepted and fails four retries per
   mail; a secret with a CR/LF is header injection waiting; the final
   `warn!` (:397) names no cause; GET echoes the secret verbatim
   (:882-884); and a commit that resolves during the drain is counted
   nowhere (store.rs:486-489's send has no receiver by then).

## The ruling

Both positions agree on the *rule* and disagree on three structures. The
rulings (full pricing in decision 0029):

- **Adopted (both):** argument staging that refuses by name; the JSON-RPC
  code taxonomy `-32700`/`-32600`/`-32601`/`-32602` (messages unchanged,
  codes sharpened — no test pinned a code); notifications by missing
  `id`; `initialize` answering one constant; `total`/default-limit parity
  across planes; the chaos `error` grammar (single line, 4xx/5xx, ≤512
  octets) with one shared sentence; the closed chaos shape; the webhook
  PUT validation; the failure cause in the final `warn!`; the derived
  bridge deadline.
- **Batches: supported, capped at 16** — the advertised revision requires
  them; refusing them means implementing 2025-06-18 to advertise it
  honestly. An eight-line loop over the existing `handle` is the only arm
  whose advertisement stays true. Over-cap → a single `-32600` naming the
  cap (refusal, never a clamp).
- **Unknown tool → `-32601`** (the acceptance text; the MCP spec example's
  `-32602` divergence is recorded in 0029 §2). Unknown *tool* and unknown
  *method* share the code; a wrong-*typed* argument is `-32602`.
- **The bridge keeps its hand-rolled client** (simplicity's own flip
  condition): the deadline-expiry arm must stay hermetic, and under
  `reqwest` the connect timeout cannot be injected. The client gains a
  status-line parse, honors content-length, caps the body at 10 MiB, and
  refuses rather than fabricating `"{}"`.
- **The webhook budget is per target** (`JoinSet` keyed by URL, each
  capped at the unchanged 32): a dead target cannot cause drops for a
  healthy one is *impossible* under a shared pool at any constant.
  Refused: 4+32 (the lone-sink topology loses its throughput for a
  pathological case) and fair-share (the budget shifts under every PUT).
  Worst case 32 × n_targets is the operator's own config; the global
  backstop is named and refused this run.
- **GET redacts `secret`; `ca_pem` stays verbatim** — a secret echoed
  into CI logs is the one irreversible leak; a certificate is public
  material and is the diagnostic. The round-trip loss is deliberate and
  documented (robustness wins the attribute; simplicity's "document only"
  is refused).
- **Shutdown counting: drain-complete sequencing in `stop()`** — the
  dispatcher joins in its own set; `stop()` sends `shutdown`, joins the
  serve tasks, sends `drained` (a new one-shot watch), then joins the
  dispatcher, which keeps consuming post-stop mail and counts each
  matching fan-out aborted. Event sequencing, not a fence race; every
  tracked commit is counted; `src/store.rs` stays at zero diff. Refused:
  the send-site counter (breaks the 0025 boundary pin for one integer)
  and the coroner receiver (deadlocks — the store outlives the join).
  0024's detached-`spawn_blocking` window is inherited and named.
- **Envelopes stay split per 0025 §4:** staging/wire-text refusals are
  JSON-RPC `error` objects; a wait's `timeout_ms` refusal stays an
  `isError` result; REST wrong-shaped bodies stay the extractor's 422
  (including serde's unknown-field sentence for chaos — *same rule,
  plane-own sentences*; 0029 §3 records the divergence).

## Acceptance Criteria

### AC1 — MCP arguments refuse by name (issue #46)

1. For every tool in `src/mcp.rs`, a *present* argument of the wrong JSON
   type, or explicit `null`, answers a JSON-RPC `error` object with code
   `-32602` whose message names the field and the domain
   (e.g. ``"inbox must be a string"``) — never a silent default:
   `swarmail_clear_inbox {"inbox":42}` does **not** clear `default`;
   `swarmail_wait_for_email {"to":["a@x.io"]}` refuses rather than
   false-greening on an empty filter; `{"since_ms":"yesterday"}` refuses
   on both `search` and `wait`; `{"limit":-1}` and `{"count":"3"}`
   refuse.
2. An *absent* argument keeps exactly its current default (schema-declared):
   `inbox` → `"default"`, search `limit` → `DEFAULT_LIST_LIMIT` (100,
   shared with REST), wait `timeout_ms` → 5000.
3. A supplied `count: 0` on `swarmail_wait_for_email` still means
   **at least one** (0010's declared semantic, pinned by the existing
   test); a supplied `count: 0` on `swarmail_get_latest_email` refuses
   `-32602` and its schema gains `"minimum": 1`.
4. No staging refusal reaches `store.wait_for` (0025 §5): a malformed
   `timeout_ms` and a malformed `to`/`since_ms` register no waiter — the
   next matching mail is not consumed by a refused call.
5. MCP `swarmail_search_emails` reports `total` = the **full match
   count** (byte-for-byte the REST shape), and the default limit is the
   one shared constant.

### AC2 — The MCP envelope answers with the right code or with silence (issue #46)

1. Malformed JSON → HTTP 400 with a JSON-RPC error object, code
   `-32700`, `id: null` (the handler owns the parse; the `Json` extractor
   is gone).
2. `{"jsonrpc":"2.0","id":5}` (no `method`) → `-32600` carrying
   `"id": 5`. A methodless **array** body is not silently swallowed.
3. A request with no `id` gets **no reply** (202, empty body) — whatever
   the method name; the `notifications/` prefix check is deleted.
   `{"jsonrpc":"2.0","id":7,"method":"notifications/initialized"}` →
   `-32601` (a notification never carries an id).
4. `tools/call {"name":"swarmail_nope"}` → `-32601`, message
   `unknown tool: swarmail_nope` (unchanged text).
5. Batches: an array of requests is answered with an array in order; an
   array of notifications with an empty 202; a mixed array with the
   requests' answers only; `[]` and any array over **16** entries → a
   single `-32600` naming the rule.
6. `initialize` answers `PROTOCOL_VERSION` (`2025-03-26`) for any
   requested version, and its capabilities no longer advertise
   `resources`.
7. `set_chaos`'s staging refusals (probability, delay) answer `-32602`
   with unchanged messages; `set_chaos` refuses an unknown argument key;
   tool-level outcomes (not-found, storage commit failed) stay `isError`
   results.

### AC3 — The stdio bridge's stdout is protocol or nothing (issue #40)

1. A reply is parsed: a non-2xx status answers a JSON-RPC error object
   carrying the request id (a server-declared JSON-RPC error body is
   forwarded verbatim); a non-JSON 2xx body refuses instead of
   fabricating `"{}"`; the body is capped at 10 MiB; content-length is
   honored.
2. The read deadline is derived (`MAX_AWAIT_TIMEOUT_MS + slack`), passed
   as a parameter (the `dial` seam), and no longer clamps a wait longer
   than 10 s: a stdio test waits >10 s through the real path and gets its
   result; the deadline-expiry arm stays hermetic via an injected short
   deadline.
3. A notification (202/empty body) writes **nothing** to stdout, and the
   blank-line filter in `tests/stdio.rs` is deleted.
4. `swarmail mcp` logs go to **stderr**; `serve`'s startup line stays on
   stdout (the `smtp=` piped contract).
5. A base URL with a trailing slash does not produce `//mcp`; the
   `http://`-only refusal is unchanged.

### AC4 — REST await/assert parse their filters (issue #38)

1. `AwaitQuery` declares its fields (no `#[serde(flatten)]`):
   `to`/`from`/`subject`/`since_ms`/`limit` parse through the same shape
   as `list` — a malformed value answers `400 invalid {what} query: {e}`
   **before** any subscribe (0002), and an omitted value keeps its
   default. Same for `assert` (minus `offset`, which is not part of
   either surface's domain).
2. A negative `since_ms` behaves identically on list, await and assert
   (accepted — parity).
3. openapi.json documents the full filter set on **both** await and
   assert, and a test pins served-doc-equals-parser-domain (0025 §6's
   pattern).

### AC5 — Chaos wire text is a reply line or nothing (issue #41)

1. A supplied `error` that is not a single line, does not start with a
   4xx/5xx code, or exceeds 512 octets is refused with one shared
   sentence, byte-identical from REST (`400`) and MCP (`-32602`):
   `"250 2.0.0 OK"`, `"451\r\nx"`, `"not-a-code"`, `"4514 no-space"`, and
   a 513-octet line all refuse; `"451"` (bare code) and
   `"450 4.7.1 no"` are legal.
2. A legal DATA rule at probability 100 makes the SMTP client see the
   given non-2xx reply and the store count stays 0 (0001's probe);
   a refused rule never reaches the session stream (no CR/LF can
   desynchronize it).
3. REST `PUT /api/v1/chaos` refuses an unknown event key with the
   extractor's 422; MCP `swarmail_set_chaos` refuses an unknown event
   value and an unknown argument key with its staging sentence. Same
   rule, plane-own sentences (0029 §3).
4. Two concurrent `set_event` arms for *different* events both survive
   (deterministic in-process test, no sleeps).
5. Chaos config is not persisted (`src/persist.rs` zero diff).

### AC6 — Webhook wire text, budget and shutdown counting (issues #47/#41)

1. `PUT /api/v1/webhooks` refuses with `400` + the validator's sentence:
   a URL that does not parse, or whose scheme is not `http`/`https`
   (`localhost:9000` refuses — `Url::parse` reads `localhost` as the
   scheme); a `ca_pem` that does not load through the delivery TLS
   stack; a `secret` containing CR/LF/NUL or over 4096 octets. The same
   parse functions serve the delivery path (both arms warm — the exact
   gate holds with no duplication).
2. Each target URL has its own budget of `WEBHOOK_DELIVERY_CAP` (32);
   two sinks where one is dead/stalled still deliver to the healthy one
   (no shared-pool starvation); excess fan-out for a target is counted in
   `dropped` per target; the storm test's pushback guarantee is unchanged.
3. The final delivery `warn!` names the last error and is visible at the
   default log level (captured-writer test); no new `/metrics` family.
4. `GET /api/v1/webhooks` answers `secret` redacted and `ca_pem`
   verbatim; PUT is the only writer (documented in the endpoint
   description, pinned by test).
5. A commit that resolves during the drain is counted: `stop()` joins the
   serve tasks before telling the dispatcher `drained`; the dispatcher
   counts post-stop matching fan-outs `aborted` without breaking early;
   the in-process test parks a commit mid-drain, releases it, and
   observes the `aborted` row (no sleeps; 0020 disciplines).

### AC7 — Gates and boundaries

1. `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked
   -- -D warnings`, `cargo test --workspace --locked`,
   `brokkr compile --bundle bundles/self`, `cargo deny check licenses`
   and `scripts/coverage-gate.sh` (exact 100/100/100, missed == 0) are
   clean.
2. `src/store.rs`, `src/persist.rs`, `src/smtp.rs` (DATA arm beyond the
   chaos write), `adapters/`, `agents/`, `bundles/`: zero diff. No new
   `src/` module, no new dependency, no `coverage(off)`.
3. Decision 0029 is committed with the batch; 0025's dated marker is a
   landing task upon operator acceptance (not edited now).
