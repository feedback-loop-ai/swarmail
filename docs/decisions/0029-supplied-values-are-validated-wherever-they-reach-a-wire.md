# 0029 — Supplied values are validated wherever they reach a wire, and the MCP envelope answers for its own words

- **Status:** accepted (operator, 2026-10-15 — proposed by the validation
  family's design panel, run `run-9-the-validation-family-refu-0380f30b`;
  the tree implements it, run `forge-run9b-feature-txt-4d2070ed`'s review
  panel passed the repaired tree, and it landed as #62, squash `ad4cb45`)
- **Date:** 2026-10-15
- **Amends:** [0025](0025-shared-domain-validation.md) (accepted) — §1's
  residual list is closed, §4's envelope posture is *extended, not
  rewritten*, and §5's "refused call registers no waiter" is carried to
  every new refusal. The dated `Amended by` marker in 0025 itself is a
  landing task upon acceptance (0028 set the precedent: no marker while
  the amender is `proposed`).
- **Records:** the wire-text rule (issues #41/#47), the JSON-RPC code
  taxonomy and bounded batches (issue #46), the stdio bridge's honest
  transport (issue #40), and the REST await/assert parser domain
  (issue #38) — postures the tree documents for the first time, with the
  alternatives rejected and priced.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md) — untouched: every refusal in this
  decision fires *before* a subscribe (0002) and *before* any insert
  (0001); no commit path gains a branch; `src/store.rs` and
  `src/persist.rs` stay at zero diff. [0010](0010-exact-count-asserts-and-the-shared-seed.md)
  — its declared `count=0` semantics on the *wait* surfaces are kept on
  both planes. [0018](0018-shutdown-observed-waits-and-explicit-bounds.md)
  — the wait ceiling owns the bridge's deadline, and no clamp appears.
  [0024](0024-shutdown-refusal-at-the-boundary.md) — the boundary still
  refuses to start commits once stopping; §5's detached-`spawn_blocking`
  window is inherited and named (§7). [0025](0025-shared-domain-validation.md)
  — one validator per rule beside its type, shared refusal sentences, the
  422 posture for wrong-shaped REST bodies. [0027](0027-living-listeners-bounded-sessions.md)
  — the webhook fan-out stays try-shaped; no budget pushes back on the
  accept path. [0020](0020-concurrency-tests-and-the-coverage-gate.md) —
  its disciplines bind every test this batch adds.

## §1 The rule: a supplied value reaches a wire only through its validator

0025 validated *one* rule (probability) and named the rest as residual:
`count`/`limit`/`since_ms` silently defaulting on the MCP plane, and wire
text (`error` lines, webhook URLs, TLS material, header secrets) written
verbatim. This decision closes the residual list with one generalization:

> **A value supplied by a client and written to a wire — an SMTP reply
> line, an HTTP request, a URL, a header, a notification body — is either
> accepted into the domain its schema advertises or refused, by name,
> before any effect. Silence is not validation; a default is a decision
> made by nobody.**

Concretely, per surface:

- **MCP argument staging** (`src/mcp.rs`): `arg_str`/`arg_usize` and the
  `since_ms` readers become `Result<Option<T>, (i64, String)>`-staged —
  an *absent* key keeps its default (the schema's own shape); a *present*
  key of the wrong JSON type, or explicit `null`, refuses `-32602` with a
  sentence naming the field and the domain ("`inbox must be a string`",
  "`count must be an integer of at least 1`"). The strict pattern the
  file already has (`timeout_ms`, `set_chaos`'s probability) becomes the
  file's *only* pattern. No newtype, no `Args` layer — 0025 §1 refused
  that once already.
- **`count=0` is not a wrong-typed value.** On the *wait* surfaces — REST
  `await` and MCP `swarmail_wait_for_email` — a supplied `count: 0`
  means "at least one", which is [0010](0010-exact-count-asserts-and-the-shared-seed.md)'s
  declared and *pinned* semantic (openapi documents it;
  `tests/mcp.rs` `wait_for_email_keeps_minimum_one_for_zero` pins it).
  Both planes keep it, documented in their schemas. On non-wait surfaces
  (`swarmail_get_latest_email`) a supplied `0` is out of the advertised
  domain and refuses; the schema gains `"minimum": 1`.
- **`since_ms` parity:** a negative `since_ms` is accepted on *both*
  planes (REST's `ListQuery` is an `i64`; the MCP reader now refuses
  non-integers but keeps negatives). "Not yet" is a legal filter, not a
  defect. If the operator later wants it refused, that is one shared
  sentence — but it must change both planes or neither.
- **Chaos wire text** (§3): the `error` line is validated where it is
  stored, not where it is written.
- **Webhook wire text** (§5): URL, TLS material and the header secret are
  validated at PUT, with the *same* parse the delivery path uses.

The rule is stated once and applied by name; no shared validation module
returns (0025 §1). The *sentences* are the shared thing; each sentence
lives beside the validator that owns it.

## §2 The JSON-RPC envelope: codes answer for the words (issue #46)

The envelope's defects were one function (`handle`): a methodless body
became `None` → `202` silence; the `notifications/` prefix was the wrong
predicate; every error was `-32601`; malformed JSON never reached the
handler at all. The taxonomy:

- `-32700` — the body is not JSON. `mcp_endpoint` owns the parse (the
  `Json` extractor is deleted) and answers a JSON-RPC error object with
  `id: null`, HTTP 400.
- `-32600` — the body is JSON but not a request: an empty array, a batch
  over the cap, a missing or non-string `method` on a *request*. The `id`
  is carried when present.
- `-32601` — an unknown *method* and an unknown *tool*. The MCP spec's
  own example uses `-32602` for an unknown tool; **this tree uses
  `-32601`** because the batch's acceptance text pins it and because the
  method/tool distinction the example draws does not survive `tools/call`
  (a tool *is* a method here). The divergence is deliberate and recorded
  here; the *message* (`unknown tool: …`) is unchanged, so no client
  matching text breaks.
- `-32602` — a supplied argument that is missing where required,
  wrong-typed, or null. This includes `set_chaos`'s staging refusals and
  the §3 wire-text refusal. Messages unchanged, codes sharpened: no test
  pinned a code before this batch (only `error.message`), so the new
  codes are additive pins.
- `-32000` — transport-level failures *inside the stdio bridge*
  (unreachable server, deadline expiry, non-JSON reply), already pinned.

**Notifications:** the JSON-RPC predicate is *a missing `id`*, not a
method-name prefix. An id-less request is answered with nothing — the
`notifications/` check is deleted; `notifications/initialized` with an
`id` is an unknown method (`-32601`). The 202-with-empty-body answer is
now *earned* (a true notification), and the stdio bridge writes nothing
for it.

**Batches are supported, bounded at 16** (`MCP_BATCH_CAP` in `src/mcp.rs`):
the advertised `2025-03-26` revision requires them, so refusing them
would mean implementing 2025-06-18's deltas to advertise *that* honestly
— strictly more machinery than an eight-line loop. A body array is a loop
over the existing `handle`; the responses are collected; all-notifications
→ empty 202; an empty array or an over-cap batch → a single `-32600`
naming the rule (refusal, never a clamp). A per-entry answer is exactly
what the single-request answer would have been.

**`initialize`** answers `PROTOCOL_VERSION` unconditionally (with exactly
one supported version, "respond with the version you support" *is* the
constant — the echo was a lie) and stops advertising `resources: {}`,
whose every method stays `-32601`. An empty `resources/list` is refused:
one deleted line is smaller than a new method arm plus a new capability
lie.

**MCP search parity:** `total` is the full match count and the default
limit is one shared `DEFAULT_LIST_LIMIT = 100` beside api.rs's constants,
used by both planes (the REST contract always promised this; the MCP side
was the lie). `compat.rs`'s 50 stays — a different wire, out of scope.

**The envelope split stays split** (0025 §4, unchanged): a wait's
`timeout_ms` refusal is an `isError` result; argument staging and wire
text refuse in the JSON-RPC `error` envelope. 0025 pinned those shapes
byte-for-byte; unifying them would rewrite two accepted decisions for
zero acceptance lines. The *text* is what is shared.

## §3 Chaos wire text: `error` is an SMTP reply line or it is nothing (issue #41)

`ChaosRule::validate` (the one owner, beside its type) gains: a supplied
`error` is a **single line** (no CR or LF anywhere — a rule could
desynchronize the session by injecting a forged `250` line into the
reply stream), it **starts with a 4xx/5xx code** (three octets, first in
`4..=5`; the fourth octet, if any, is a space — a bare `451` is legal per
RFC 5321's reply-line grammar; `250 2.0.0 OK`, the poison case that broke
[0001](0001-synchronous-insert-lossless.md) through the control plane, is
not), and it is **at most 510 octets of text**. RFC 5321 §4.5.3.1.2 caps
the reply line at 512 octets *including* the CRLF that `write_line`
appends, so the text a rule may carry is 510 — stated here so no
off-by-two hides inside a quoted "512" in a repo whose brand is protocol
exactness. One `pub(crate)` refusal sentence, byte-identical from the
REST `400` and the MCP `-32602`.

**`delay_ms` is bounded by the same validator.** It was the one supplied
value the struct's check waved through: `u64::MAX` parks a session task
on a bare `tokio::time::sleep` in the DATA arm (smtp.rs:992) with no
bound but stop's abort — a denial-of-service dial on the very struct
this decision arms. A supplied `delay_ms` over **300 000** refuses with
one shared sentence from both planes. The ceiling is RFC 5321 §4.5.3.2's
stage-timer scale (five minutes covers every stage timer a lawful client
applies) and *deliberately not* the 60 s `MAX_AWAIT_TIMEOUT_MS` — that
constant governs the control plane's waits, not the wire's timers;
importing it here would couple two planes that fail differently. The
cost is named: a chaos test wanting a longer-than-five-minute wire stall
arms successive rules or relies on the stage's own timeout. A refusal
(not a clamp) per 0018 §3.

**The chaos shape is closed:** `deny_unknown_fields` on `ChaosConfig` and
`ChaosRule`. REST answers an unknown event key with serde's own 422
rejection (0025 §4's pre-existing envelope for wrong-shaped bodies —
*no custom deserializer*); MCP keeps its staging sentence
("`unknown event: … (use connect|mail_from|rcpt|data)`") and refuses an
unknown argument key the same way. **Same rule, plane-own sentences** —
byte-identical words would require exactly the machinery 0025 §4 refused.
The divergence is recorded here, not hidden.

**`Chaos::set_event(event, rule)`** makes the MCP `set_chaos`
read-modify-write one lock-scoped step: two concurrent arms of *different*
events cannot erase each other. It is not a multi-event transaction (a
concurrent PUT still sees a per-event intermediate) — that residual is
named. Validation stays at the staging layer (refuse before any effect);
the setter performs none. REST's whole-config PUT is unchanged.

Chaos persistence is refused (both positions agree): a test fixture that
survives restart is a footgun that outlives the test run, and
`src/persist.rs` stays at zero diff.

## §4 The stdio bridge: stdout carries protocol, or nothing (issue #40)

- The bridge keeps its **hand-rolled HTTP/1.1 client**. Simplicity's own
  flip condition decides it: the deadline-expiry arm must stay hermetic
  (0020's whole spine), and under `reqwest` the connect timeout cannot be
  injected — the test that proves the named error would need a real
  65-second sleep. The hand-rolled client keeps the existing `dial()`
  seam; it gains a status-line parse.
- `READ_TIMEOUT` is deleted. The read deadline is **derived**:
  `MAX_AWAIT_TIMEOUT_MS + BRIDGE_SLACK` (slack: 5 s, named) — the one
  constant that owns the wait ceiling now owns the bridge's budget, so no
  future cap change can re-create the clamp. It is passed as a parameter
  into `rpc_post`, exactly as `dial` already takes one.
- The response is parsed: the status line is read and a non-2xx becomes a
  JSON-RPC error carrying the request id; a body that *is* a JSON-RPC
  error object is forwarded verbatim (a server-declared error is a
  JSON-RPC error); a non-JSON 2xx body refuses rather than fabricating
  `"{}"`; `content-length` is honored; the body is capped at **10 MiB**
  (`AsyncReadExt::take`; over-cap → error object). A chunked reply falls
  out through the non-JSON arm — loudly, with a named residual: the
  hand-rolled client remains HTTP/1.1 with identity encoding only.
- A notification (202 with an empty or whitespace body) writes **nothing**
  to stdout. The blank-line filter in `tests/stdio.rs` is deleted — it
  was hiding this defect.
- **A failed notification stays silent.** The missing-`id` predicate is
  not only the server's envelope rule — the bridge applies it locally in
  both arms: a line that parses and has no `id` is a notification, and
  when its POST fails (connect refused, deadline expiry, a non-JSON
  reply) the bridge still writes nothing. An error object carrying
  `id: null` for a notification is the stdout twin of the bare-newline
  bug — it desyncs a line-parsing client identically. The parse-once
  id-extraction helper serves both arms.
- Logging: tracing initialization moves into the subcommand arms; the
  `mcp` subcommand logs to **stderr** (its stdout is protocol); `serve`
  keeps stdout (the binary tests' `smtp=` piped-line contract).
- The base-URL path join is slash-safe (`trim_end_matches('/')`), and the
  `SWARMAIL_URL must be http://` refusal survives untouched (a pinned,
  honest sentence).
- A stdio test waits longer than 10 s through the real path and gets its
  result — the clamp the old constant imposed is dead.

## §5 Webhook wire text and the per-target budget (issues #47/#41)

- **PUT validation:** `WebhookTarget::validate()` beside the type, called
  by `set_webhooks` before any effect → 400 in the 0025 posture: the URL
  parses and its scheme is `http`/`https` with a non-empty host
  (`Url::parse` reads `localhost:9000` as *scheme* `localhost` — today
  that misconfig surfaces only at delivery); `ca_pem`, if present, loads
  through the **delivery TLS stack itself**; `secret`, if present, is a
  single-line header value with no CR/LF/NUL and at most 4096 octets.
  **The gate consequence is structural:** the URL parse/scheme check and
  the PEM parse are *extracted into shared functions called from both*
  the PUT validator and the delivery path (`post_json`, `client_for`) —
  one owner, both arms warm. A duplicated check would drive the
  delivery-time arms cold and the exact 100/100/100 gate red.
- **The delivery budget is per target:** one `JoinSet` keyed by the
  target's **parsed, normalized `Url`** — never a raw string, because a
  duplicate spelling of one sink must not buy a second budget and
  re-open the starvation the budget exists to close — each bounded by
  the unchanged `WEBHOOK_DELIVERY_CAP = 32`. Entries whose target has
  vanished from the config and whose set has drained are retired, or
  every PUT/PUT/PUT cycle leaks a dead key. The two-sink acceptance —
  *a dead target cannot cause drops for a healthy one* — is impossible
  under a shared pool at any constant, and structurally impossible under
  a per-target budget at any n: the healthy sink's slots are its own.
  Refused, with the losers' prices: 4-per-target plus a global 32 (the
  common lone-sink topology delivers 4-at-a-time instead of 32, and the
  acceptance's failure mode *survives a band* — with 32 global slots a
  healthy sink starves once ≥8 peers are stalled simultaneously) and
  fair-share division (the budget would shift under every PUT). The
  honest loss of the winner: worst-case in-flight is 32 × n_targets,
  where n is the operator's own authored config — a documented loss,
  never a discovered one; the escalation (a global backstop) is named
  and refused this run. Every spawn stays try-shaped — 0027's sizing
  posture is untouched. A delivery's payload stays fixed at spawn (the
  retry replays its bytes, never a store re-read) — pinned by test so
  the budget remains *slots* and cannot be coupled to 0003's pruning.
- **Failure cause:** the final `warn!` carries the last delivery error at
  the default level (no new `/metrics` family, no per-attempt spam — the
  acceptance offers "or").
- **GET redacts the secret; everything else stays verbatim.** A secret
  echoed into CI logs is the one *irreversible* leak in the batch; a
  certificate is public material and `ca_pem` stays (it is the
  diagnostic). The round-trip loss is deliberate and documented: PUT is
  the only writer, and the operator retains their own payload. Pinned by
  test; the endpoint description says so.
- **Shutdown counting** (the one real tension): the framing demands
  *notifications from commits that finish during shutdown are counted*
  **and** pins `src/store.rs` at zero diff. Ruled: the **drain-complete
  sequencing in `RunningServer::stop`** — the dispatcher joins in its own
  set; `stop()` sends `shutdown`, joins the serve tasks (their awaited
  commits resolve), then sends `drained` (a new one-shot watch), then
  joins the dispatcher, which keeps consuming post-stop mail, counts each
  matching fan-out aborted, and exits on `drained` or channel close
  (before the tail sweep and `abort_all`, as today). This is *event*
  sequencing, not a fence+ε race: it covers every tracked commit, keeps
  0018's fence the only bound, and touches `webhook.rs`/`lib.rs` only.
  Refused: a counter at the store's send site (exact, but it breaks the
  0025 boundary pin for one integer); a "coroner receiver" (it cannot
  observe channel closure before `stop()` returns — the store outlives
  the join — so it deadlocks); an outbox (there is no outbox, by design).

## §6 Rejected alternatives, priced

- **A typed `Args` layer / `McpError` enum / full request struct:** every
  variant of the "enum" maps to an integer literal; the tuple
  `(i64, String)` is the whole taxonomy. 0025 §1 refused the module once.
- **Per-request deadline sizing in the bridge** (parse `timeout_ms` per
  line): more parsing for a bound the domain cap already fixes.
- **A custom deserializer for REST chaos events:** 0025 §4's precedent is
  explicit, and the divergence is recorded instead.
- **Refusing an already-expired `ca_pem` at PUT** (the robustness seat's
  "cheap slice" of expiry): it is not cheap — honouring it needs an
  X.509 validity parser (a new dependency, or hand-rolled ASN.1 beside a
  TLS stack that already knows), and 0020 rule 5's no-wall-clock
  discipline would demand per-run fixture generators for a check the
  delivery path surfaces loudly anyway (AC's default-level cause and the
  ledger). A cert that *parses* but is expired fails its first delivery,
  by name; a validator cannot check the future, and pretending otherwise
  is the false-green this run deletes.
- **`resources/list` as an empty arm:** one deleted line beats a new
  method plus a capability lie.
- **Chaos persistence, a metrics family for causes, an unredacted-or-
  documented-only GET:** priced in §3/§5 and refused.

## §7 Residuals, named

- The rest of the MCP tool surface still ignores unknown argument keys
  (a typo'd `inbx` on `swarmail_clear_inbox` silently clears
  `default`). The closed-shape rule lands on `set_chaos` this run; the
  general refusal is a named defect for the next batch — it is a
  mechanical repeat of one pattern, but it rewrites every schema and
  test in one sweep and deserves its own.
- 0024's detached-`spawn_blocking` window: a commit whose *session* task
  was aborted after the drain began, but whose blocking task lands after
  everything joined, is observed by no one. At most one commit, bounded
  by the busy bound; inherited from 0024, not created here.
- The hand-rolled bridge remains HTTP/1.1 with identity transfer coding;
  a chunked reply refuses loudly rather than being parsed. If a real
  target needs chunked replies, that is the reqwest swap reopening —
  with the deadline-expiry arm's hermeticity as the price to solve.
- A `ca_pem` that parses but is expired (or expires mid-run) is a
  *delivery outcome*, not a PUT refusal (§6): the first failing delivery
  names the TLS cause at the default level. A cert expiring *later* is
  the future, which no validator can check.
- `jsonrpc` version members are not validated this run (a body declaring
  `"jsonrpc": "1.0"` is dispatched like any other). The taxonomy is the
  contract; the member check is ceremony until a client lies.
- The `compat.rs` default limit (50) is a different wire and stays.
- Negative `since_ms` is accepted on both planes (recorded option).

## §8 Landing

The code and this decision ship together (0028's precedent). Upon
operator acceptance, 0025 gains its dated `Amended by` marker, and the
README/openapi sentences this decision promises (per-target slots, the
chaos `error` grammar, the batch cap and codes, the redacted GET) are
already in the tree — the docs were *not* left ahead of the code.

## Pointer — 2026-10-18 (proposed 0047)

[0047](0047-storage-rpc.md) §3, which is **proposed**, would amend §2's notification
predicate (issue #132): an object is silent only when it has no `id` key *and* a string
`method`. Any other `id`-less object is an invalid request answered `-32600` with a null
id, over HTTP and on the stdio bridge alike. Supplied ids, null included, keep their
behavior. Nothing above is rewritten by this pointer, and its status is unchanged.
