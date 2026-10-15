# Plan — the validation family #46/#40/#38/#41/#47 (one refusal posture, five surfaces)

Implementation plan for `specs/validation-family/spec.md`. All anchors
re-verified against the working tree at `6072564` this session.

## Technical context

- Single Rust crate; production under `src/`, integration suites under
  `tests/`. The batch touches **six production files**:
  - `src/mcp.rs` — `handle()` `:31-54` (the envelope), the arg helpers
    `:68-76`, the `since_ms` readers `:94`/`:156`, search `:89-96`
    (limit 50, `total: emails.len()`), get_latest `:116` (`.max(1)`), the
    wait tool `:150-212` (`count` `:158`, strict `timeout_ms` `:165-181`
    is the model), `set_chaos` `:260-296` (the RMW), the `unknown tool`
    arm `:307`, and `TOOLS`/schemas `:317-370` with the parity test near
    `:380-405`.
  - `src/stdio.rs` — `READ_TIMEOUT` `:11`, `dial()` `:15-26` (the
    deadline seam), `rpc_post()` `:29-60` (`:56` fabricates `"{}"`),
    `bridge()` `:72-112` (the write `:86-88`, the `-32000` arm
    `:90-103`).
  - `src/api.rs` — `ListQuery` `:360-368`, `AwaitQuery` `:381-389` (the
    flatten), `parse_query` `:601-603`, `list_messages` `:605-620`
    (limit 100, full-count `total`), `await_messages` `:634-690`,
    `assert_messages` `:694-714`, `set_webhooks` `:886-892`,
    `get_webhooks` `:882-884`, `mcp_endpoint` `:901-915`.
  - `src/chaos.rs` — `ChaosRule`/`validate()` `:49-58`,
    `ChaosConfig::validate()`, `Chaos::set()` `:100-106`; a new
    `set_event()` beside them.
  - `src/webhook.rs` — `WEBHOOK_DELIVERY_CAP` `:51`, the dispatch loop
    `:235-330` (cap checks `:276`/`:296`/`:313`, tail sweep
    `:333-347`), `matching_count` `:349`, `spawn_dispatcher` `:362`,
    `deliver`'s final `warn!` `:397`, `client_for` `:435`,
    `post_json`'s URL arm `:468-471`, the `WebhookTarget` type.
  - `src/lib.rs` — `RunningServer` (`tasks: JoinSet<()>`; the dispatcher
    is spawned first, `:88`) and `stop()` `:51-56`.
  Plus `src/main.rs:9-20` (tracing init → per-subcommand writer) and the
  static docs (`src/static/openapi.json`, `README.md`,
  `src/static/llms.txt`).
- Suites: `tests/mcp.rs` (pins *messages* only — codes are additive;
  `wait_for_email_keeps_minimum_one_for_zero` `:525`,
  `set_chaos_refuses_every_malformed_wire_value_by_name` `:1108`,
  `the_schemas_publish_the_wire_bounds` `:1299`),
  `tests/stdio.rs` (drives `bridge()` in-process; the blank-line filter
  `:58-59`; the deadline seam test `:156`), `tests/api.rs`/`tests/p2.rs`
  (REST legs), `tests/webhook.rs` (real sinks; `stalled_sink` `:1099`,
  the bound test `:1193`, the capture-writer pattern
  `webhook.rs:956-969`), `tests/binary.rs` (the `smtp=` piped contract).
- Coverage is the exact 100/100/100 gate. Every new arm ships with a test
  *in the same shape 0020 prescribes*: branch-stable inline unit tests in
  `src/<module>/` for the pure checks (`validate`, the grammar, the
  staging helpers), real-protocol legs in `tests/` for the wires, and
  `#[cfg(test)]`-only seams for the two things that need them (the
  bridge deadline, the drain parking). No sleeps as synchronization: the
  drain test uses a real blocked commit (a second SQLite connection
  holding the write lock — the established shape) released via a gauge.

## Approach, per issue (delivery order)

1. **#46 (mcp.rs + api.rs):** stage the arg helpers
   (`Result<Option<T>, (i64, String)>`), generalize the file's own
   strict pattern; rewrite `handle()` around the id predicate; own the
   parse in `mcp_endpoint` (delete the `Json` extractor); add the batch
   loop + `MCP_BATCH_CAP`; fix `initialize`, `total`, and the shared
   `DEFAULT_LIST_LIMIT` (declared beside api.rs's other constants).
2. **#40 (stdio.rs + main.rs):** derive and parameterize the deadline;
   parse the status line; cap and validate the body; delete the
   fabrication; move logging per subcommand; fix the slash join.
3. **#38 (api.rs + openapi.json):** de-flatten `AwaitQuery` (declare the
   fields), route both filters through `parse_query`, extend the served
   schema, pin with the served-doc-equals-parser-domain test.
4. **#41 (chaos.rs):** the reply-line grammar in `ChaosRule::validate`
   (hand-checked: 3 octets, first in `4..=5`, then SP-or-end, no
   CR/LF/NUL, ≤512 octets) + one shared sentence; `deny_unknown_fields`
   on both types; `Chaos::set_event` (match inside the write lock).
5. **#47 (webhook.rs, api.rs, lib.rs):** extract `parse_webhook_url` /
   `parse_ca_pem` (called from *both* the PUT validator and the delivery
   path — this is what keeps the gate exact); `WebhookTarget::validate`;
   the `HashMap<String, JoinSet>` budget; the cause in the final `warn!`;
   the redacted GET; the dispatcher's own `JoinSet` + the `drained` watch
   and the drain-complete sequencing in `stop()`.
6. Docs (`README.md` ~:394 per-target slots, ~:423-444 chaos grammar,
   MCP codes/cap; `openapi.json` await/assert parameters + webhook
   descriptions + chaos 400; `llms.txt` one sentence each).

## Constitution check

| Guarantee | Effect of this batch |
|---|---|
| 0001 accept == stored | **Untouched.** No commit path changes; the chaos refusal lands at the *staging* layer, so a poison rule can never reach `write_line`; the SMTP DATA arm keeps its shape (`error.as_deref().unwrap_or(...)` is now only ever fed a validated line or the default). |
| 0002 await subscribes before it checks | **Strengthened.** Every new refusal (MCP staging, REST query parse, webhook PUT, chaos shape) fires before a subscribe/park; AC1.4 pins the no-waiter rule. |
| 0003 oldest-only pruning | Untouched — no pruning path gains a branch. |
| 0025 boundary | **Held:** `src/store.rs`/`src/persist.rs` zero diff; validation never enters the boundary. `Amended by 0029` (marker on acceptance). |
| 0024 fence | The boundary still refuses to start commits once stopping; the drain counts *tracked* commits; the detached-`spawn_blocking` window is inherited and named (0029 §7). |

## Risks

1. **The exact-coverage gate goes red on a duplicated check** — a PUT
   validator that re-implements the URL/PEM parse leaves the
   delivery-time arms (`post_json:468-471`, `client_for:446`) cold.
   *Mitigation:* the shared functions are extracted and called from both
   sides (spec AC6.1); the gate is run after each task, and any dead arm
   is a signal the *extraction* is wrong, not an invitation to
   `coverage(off)`.
2. **`deny_unknown_fields` + `#[serde(flatten)]` interplay** — if
   `ChaosConfig` ever embeds a flattened struct, serde rejects the
   combination. *Mitigation:* the chaos types have no flattening; the
   REST PUT body is the whole config (verified at api.rs). Do not add
   flatten to a deny-unknown type.
3. **The batch loop changes the 202 contract** — a client that asserted
   *silence* on a malformed single body now gets `-32700`/`-32600`.
   *Mitigation:* that is the acceptance; the *messages* are unchanged
   and no test pinned a code; the bridge forwards server-declared error
   objects verbatim so end-to-end behavior is honest.
4. **The drain-complete sequencing could hang `stop()`** if the
   dispatcher's exit predicate is wrong (e.g. it breaks before `drained`
   arrives, or never sees it). *Mitigation:* the predicate is
   `drained.changed()` **or** the firehose closing (a dropped receiver
   must not hang the stop); the dispatcher already exits on channel
   close; the drain test proves the sequencing, and the existing
   shutdown tests (`tests/binary.rs`, `tests/p2.rs`) are the regression
   net. 0018's fence still bounds everything.
5. **MCP clients that sent wrong-typed args "worked" before** and now
   get `-32602`. *Mitigation:* accepted by design (0029 §1) — the schema
   always said `string`/`integer`; the refusal is the point of the
   batch. The `total`/default-limit change is the same class.
6. **The >10 s stdio test costs real time in CI.** *Mitigation:* it is
   one ~11 s leg in a suite that already carries ~22 s legs
   (`tests/webhook.rs` `stalled_sink`); the expiry arm itself stays
   hermetic via the injected deadline (no 65 s test).
7. **Per-target budgets raise worst-case in-flight memory** (32 ×
   n_targets). *Mitigation:* try-shaped spawns (0027 posture unchanged),
   `n` is the operator's own PUT, and the escalation is named in 0029 §5
   rather than smuggled in as a "safety" second tier.
8. **The redacted GET breaks any client that PUTs then GETs and re-PUTs.**
   *Mitigation:* documented as deliberate (endpoint description +
   README), `secret` is the only redacted field, and the operator's own
   payload remains the source of truth.

## Verification

Per-task: the proving test named in `tasks.md`, then the full gate set
(fmt, clippy `-D warnings`, `cargo test --workspace --locked`, brokkr,
`scripts/coverage-gate.sh`, `cargo deny check licenses`). `git diff
--stat src/store.rs src/persist.rs src/smtp.rs` must be empty after the
SMTP-adjacent tasks. The harness-name rule (0020 rule 4) applies if any
new seam is added — none is expected beyond the two named ones.
