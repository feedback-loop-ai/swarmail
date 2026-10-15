# Tasks — the validation family #46/#40/#38/#41/#47

Ordered by the framing's delivery order (**#46 → #40 → #38 → #41 →
#47**). Each task is paired with its proving test. Decision 0020's
disciplines bind every edit: no sleeps as synchronization, chaos at the
poles only, per-test fresh stores, no wall-clock assertions, and the
exact 100/100/100 gate is the arbiter — an uncovered arm is dead or
under-tested, and the fix is a test, never `coverage(off)`.

- [ ] **1. Stage every MCP argument; refuse by name (issue #46)** — in
  `src/mcp.rs`, turn `arg_str`/`arg_usize` (`:68-76`) and the `since_ms`
  readers (`:94`, `:156`) into `Result<Option<T>, (i64, String)>`
  staging: absent → `Ok(None)` (its schema default applies at the call
  site); present-but-wrong-typed or explicit `null` → `-32602` with a
  sentence naming the field and domain (one tiny formatter beside the
  helpers is the single owner of the shape). Apply it to every tool:
  `{"inbox":42}` no longer takes the `"default"` arm; `{"to":["a@x.io"]}`
  refuses; `{"limit":-1}` and `{"count":"3"}` refuse; get_latest's
  supplied `count: 0` refuses (schema gains `"minimum": 1`) while the
  wait's `count: 0` keeps 0010's at-least-one semantic (schema documents
  it, as openapi already does). Staging precedes every `store` call
  (0025 §5). MCP search reports the full match count and takes the new
  shared `pub(crate) const DEFAULT_LIST_LIMIT: usize = 100` beside
  api.rs's constants (REST's 100; compat's 50 stays).
  *Test:* legs in `tests/mcp.rs` — `a_wrong_typed_argument_refuses_by_
  name` (one leg per wrong type: string→int, int→string, `null`, negative
  limit; assert `error.code == -32602` and the field's name in the
  message, and that the effect did NOT happen — `clear_inbox` leaves the
  inbox populated); `the_wait_registers_no_waiter_when_staging_refuses`
  (refuse `{"to":[]}`-typed args, then deliver a matching mail and show
  it is *not* consumed by the refused call); `mcp_search_total_is_the_
  full_match_count` (seed 3 matching + 3 non-matching in the page beyond
  the limit, assert `total == 3-matching` and the page length); the
  `count: 0` wait leg is already pinned (`:525`) and must stay green.
  Proves AC1.
- [ ] **2. Give the envelope its codes, its id predicate and its batch
  (issue #46)** — in `src/mcp.rs`: rewrite `handle()` (`:31-54`) around
  the JSON-RPC predicate — a present `id` is a request, a missing `id` is
  a notification (delete the `notifications/` prefix check; a missing or
  non-string `method` on a request → `-32600` carrying the id; an
  unknown method or tool → `-32601` with unchanged message); thread the
  error code through `tool_call` as `(i64, String)` (no new enum); in
  `src/api.rs` `mcp_endpoint` (`:901-915`) take the body as text, parse
  it there (delete the `Json` extractor) and answer `-32700` with
  `id: null` for malformed JSON; add the batch loop (collect `Some`
  responses; all-notifications → empty 202; `[]` and arrays over the new
  `MCP_BATCH_CAP: usize = 16` → a single `-32600` naming the rule); fix
  `initialize` (`:39-43`) to answer `PROTOCOL_VERSION` and stop
  advertising `resources`.
  *Test:* legs in `tests/mcp.rs` — `the_envelope_carries_the_code_
  taxonomy` (pin `-32700`/`-32600`/`-32601`/`-32602` and the preserved
  messages for `unknown tool: …` and `missing id`); `an_idless_request_
  is_a_notification` (nothing on stdout via the bridge, 202 empty body
  over HTTP; `notifications/initialized` *with* an id → `-32601`);
  `a_batch_is_answered_in_order_and_bounded` (3 requests + 1
  notification → 2-element array; `[]` → `-32600`; 17 entries →
  `-32600` naming the cap); `initialize_answers_the_supported_version_
  and_advertises_only_tools`. Proves AC2.
- [ ] **3. Derive the bridge deadline; parse the reply; log to stderr
  (issue #40)** — in `src/stdio.rs`: delete `READ_TIMEOUT` (`:11`); add
  `BRIDGE_SLACK: Duration = 5s` and pass
  `MAX_AWAIT_TIMEOUT_MS + BRIDGE_SLACK` as a `deadline` parameter into
  `rpc_post` (the `dial` seam, `:15-26`); read the status line (non-2xx
  → JSON-RPC error with the request id; a body that *is* a JSON-RPC
  error object is forwarded verbatim), honor content-length, cap the
  read at 10 MiB (`AsyncReadExt::take`) and refuse a non-JSON body
  instead of `unwrap_or("{}")` (`:56`); write nothing for an empty or
  whitespace body; fix the slash join (`path.trim_end_matches('/')`).
  In `src/main.rs`: move the tracing init below the subcommand match and
  send `mcp`'s logs to stderr (`serve` keeps stdout — the `smtp=`
  contract).
  *Test:* legs in `tests/stdio.rs` — delete the blank-line filter
  (`:58-59`); `a_notification_writes_nothing` (empty body → empty
  stdout); `a_trailing_slash_base_does_not_double_the_slash`;
  `a_non_2xx_is_an_error_carrying_the_request_id` and
  `a_non_json_body_is_refused_not_fabricated` (in-process sinks);
  `a_wait_longer_than_ten_seconds_gets_its_result` (real server: wait
  15 s for a mail delivered at ~10.5 s — the old 10 s clamp would error);
  keep `the deadline expiry` hermetic by injecting a short deadline
  (silent server, named error); `serve` startup line stays on stdout
  (binary suite stays green). Proves AC3.
- [ ] **4. De-flatten the await/assert filters (issue #38)** — in
  `src/api.rs`: declare `AwaitQuery`'s fields explicitly (`to`, `from`,
  `subject`, `since_ms`, `limit`, `count`, `timeout_ms` — no
  `#[serde(flatten)]`), route them through the shared `parse_query`
  (`:601-603`) so a malformed value answers
  `400 invalid {what} query: {e}` *before* the park (0002), with omitted
  values keeping their defaults; `offset` is not declared on either
  surface; `assert_messages` (`:694-714`) gains the same filter parse.
  Extend `src/static/openapi.json` (await: add `limit`; assert: add
  `to`/`from`/`subject`/`since_ms`/`limit`) with the same descriptions
  the list endpoints carry.
  *Test:* legs in `tests/api.rs`/`tests/p2.rs` —
  `await_parses_its_filters_like_list` (`since_ms`/`limit` accepted,
  wrong-typed → 400 with the shared sentence, and **no waiter
  registered**); `a_negative_since_ms_behaves_the_same_on_list_await_
  and_assert` (parity); `the_served_schema_matches_the_parser_domain`
  (0025 §6's pattern: every parameter openapi declares for await/assert
  is accepted by the handler, and vice versa). Proves AC4.
- [ ] **5. Validate the chaos `error` line; close the shape; make
  set_chaos atomic (issue #41, decision 0029 §3)** — in `src/chaos.rs`:
  `ChaosRule::validate()` (`:49-58`) gains the reply-line grammar —
  single line (no CR/LF anywhere), 3-octet code with the first in
  `4..=5` and the fourth octet SP-or-end, ≤512 octets (RFC 5321
  §4.5.3.1.2) — with one `pub(crate)` refusal sentence shared by both
  planes; add `deny_unknown_fields` to `ChaosConfig` and `ChaosRule`;
  add `Chaos::set_event(event, rule)` doing the whole match inside one
  write-lock scope, and use it from `set_chaos` (`src/mcp.rs:260-296`),
  which also refuses unknown argument keys
  (`use event|probability|error|delay_ms`). REST keeps its 422
  (extractor's sentence) and its whole-config PUT.
  *Test:* inline unit corpus in `src/chaos.rs` (branch-stable) —
  refuses `"250 2.0.0 OK"`, `"451\r\nx"`, `"not-a-code"`, `"4514"`,
  513 octets, CR-only; accepts `"451"`, `"450 4.7.1 no"`;
  `two_concurrent_set_event_arms_both_survive` (N threads, join, all
  events present — no sleeps). Wire legs in `tests/mcp.rs` /
  `tests/api.rs`: `the_poison_error_line_is_refused_at_both_doors`
  (byte-identical sentence from REST 400 and MCP `-32602`; the rule is
  not stored); `a_legal_chaos_error_reaches_the_smtp_client` (DATA rule
  `"450 4.7.1 no"` at probability 100 → client sees `450`, store count
  0 — 0001's probe); `rest_chaos_put_refuses_an_unknown_event_key`
  (422, serde's sentence). `src/persist.rs` stays at zero diff. Proves
  AC5.
- [ ] **6. Validate webhook wire text at PUT, with the shared parses
  (issue #47)** — in `src/webhook.rs`: extract `parse_webhook_url` and
  `parse_ca_pem` and call them from **both** the new
  `WebhookTarget::validate()` and the delivery path (`post_json`
  `:468-471`, `client_for` `:435-452` — both arms stay warm, the exact
  gate holds); `validate()` checks scheme `http`/`https` + non-empty
  host, the PEM load, and the secret (no CR/LF/NUL, ≤4096 octets) with
  0025-posture sentences; call it from `set_webhooks`
  (`src/api.rs:886-892`) → `400` before any effect.
  *Test:* inline unit corpus in `src/webhook.rs` (branch-stable: the
  refusals and accepts, `localhost:9000` among the refused); wire legs
  in `tests/webhook.rs`/`tests/api.rs` —
  `put_refuses_a_scheme_forgotten_target_before_any_effect`,
  `put_refuses_a_garbage_ca_pem_and_a_header_secret`, and the
  delivery-path warm-arm proof (a delivery through a valid target still
  exercises the same parse — the gate verifies). Proves AC6.1.
- [ ] **7. Budget per target; name the failure; redact the secret
  (issue #47)** — in `src/webhook.rs`: replace the single `JoinSet`
  (`:235-330`) with a `HashMap<String, JoinSet<()>>` keyed by target
  URL, each bounded by the unchanged `WEBHOOK_DELIVERY_CAP = 32`
  (`dropped` counted per target; harvest every set; `abort_all` loops
  the map at the tail); track the last error and put it in the final
  `warn!` (`:397`); in `src/api.rs` `get_webhooks` (`:882-884`) answer
  `secret` redacted with `ca_pem` verbatim, and say so in the endpoint
  description; update the doc comment on `dropped` and the README
  (~`:394`).
  *Test:* legs in `tests/webhook.rs` — `a_dead_target_cannot_starve_a_
  healthy_one` (stalled sink + healthy sink, one burst, healthy
  deliveries complete, `dropped` rows name per-target counts — extends
  the `:1193` bound test); `the_final_warning_names_the_cause`
  (captured-writer pattern, default level); `get_redacts_the_secret_
  and_keeps_the_pem`. Proves AC6.2–AC6.4.
- [ ] **8. Count commits that finish during the drain (issue #47)** —
  split the dispatcher into its own `JoinSet` on `RunningServer`
  (`src/lib.rs`); add a one-shot `drained` watch; `stop()`
  (`:51-56`): send `shutdown` → join the serve tasks → send `drained` →
  join the dispatcher; the dispatcher's post-stop Mail arm counts
  matching fan-outs `aborted` **without breaking early** (keep
  consuming until `drained` or the firehose closes), then the tail
  sweep and `abort_all` as today (`:333-347`); pass
  the new receiver through `spawn_dispatcher` (one call site). 0018's
  fence stays the only bound; `src/store.rs` stays at zero diff; name
  0024's detached-`spawn_blocking` residual in the module comment.
  *Test:* in-process drain test in `tests/webhook.rs` — park a commit
  mid-transaction (a second SQLite connection holding the write lock —
  the established blocked-commit shape), begin `stop()`, release the
  block, join, and assert the mail's fan-out is counted in the
  `aborted` row (gauge-sequenced, no sleeps); the existing shutdown
  suites (`tests/binary.rs`, `tests/p2.rs`) stay green as the
  regression net. Proves AC6.5.
- [ ] **9. Align the docs and verify the gates** — README (per-target
  slots ~`:394`, the chaos `error` grammar ~`:423-444`, MCP codes/cap
  sentence, the redacted GET), `src/static/llms.txt` (webhook PUT
  validation, the batch cap), and any sentence the batch made false.
  Verify: `cargo fmt --all -- --check`; `cargo clippy --workspace
  --all-targets --locked -- -D warnings`; `cargo test --workspace
  --locked`; `brokkr compile --bundle bundles/self`;
  `scripts/coverage-gate.sh` (100/100/100, missed == 0); `cargo deny
  check licenses`; `git diff --stat src/store.rs src/persist.rs
  src/smtp.rs` is empty. Proves AC7. 0025's `Amended by` marker is
  *not* edited (landing task upon operator acceptance).
