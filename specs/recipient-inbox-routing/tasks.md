# Ordered tasks — recipient inbox routing

Implementation obligations under [spec.md](spec.md) and proposed
[0112](../../docs/decisions/0112-recipient-inbox-routing.md). Check a task
only after its paired test is green against the candidate. Dependencies
follow this order.

- [x] **T1 — The route-domain flag (AC7).** Add `--route-domain` /
  `SWARMAIL_ROUTE_DOMAIN` to `ServeArgs` with a `value_parser` that refuses
  an empty value, an empty label, a character outside `[A-Za-z0-9-]`, a
  label over 63 characters and a leading or trailing dot, and lowercases
  the rest; carry it into `SmtpConfig.route_domain` (default `None`).
  **Test:** inline unit table in `src/config.rs` over every refusal and the
  lowercase fold; `tests/config_cli.rs` proves each refusal exits before any
  bind naming the flag and that a mixed-case value starts the server.
- [x] **T2 — The routing function (AC4).** `routed_inbox(domain, rcpt)` in
  `src/smtp.rs` returning `Ok(None)` for a recipient not under the domain
  (including whole-label misses and no `@`), `Ok(Some(label))` for exactly
  one admitted label after lowercasing, and `Err` for every malformed shape
  under the domain. **Test:** inline table covering each arm, plus a test
  that every admitted-alphabet name passes `validate_inbox`.
- [x] **T3 — Routing admission at `RCPT` (AC4).** Between the ceiling check
  and chaos: `Err` → `550 5.1.3 Recipient names no inbox under the route
  domain`; `Some(label)` differing from `open.routed` → `452 4.5.3 One
  routed inbox per transaction`; otherwise append and set `open.routed` on
  the same successful append. **Test:** `tests/inbox_routing.rs` over real
  SMTP: 550 for each malformed shape storing nothing; 452 for a second
  routed inbox, then the retry in a fresh transaction landing both copies;
  the same inbox accepted twice; chaos `rcpt` at 100 on a routed recipient
  leaving the route unset; `RSET` clearing it; a non-matching CC recorded
  in `recipients` without disturbing routing; case-fold and whole-label.
- [x] **T4 — The inbox at `DATA` precedes the park (AC3, AC5).** Destructure
  `routed` from the taken envelope and compute the inbox once as routed,
  else AUTH, else `default`; `park_delivery`, `build_email` and the
  in-flight row all take that name. **Test:** `tests/inbox_routing.rs`:
  an unset server still routes by AUTH and `default` (plus every existing
  0004 test unchanged); a `wait_for_email` registered on a minted,
  never-mailed inbox before the send returns the mail; during a parked
  delivery (the existing parse-submission seam or a chaos-free deterministic
  hold) the `swarmail_deliveries_in_flight` row names the routed inbox.
- [x] **T5 — Thread the domain to MCP.** `AppState.route_domain` from the
  config, `McpContext.route_domain` set by the `/mcp` handler, `None` in
  the four test literals. **Test:** the existing MCP suites compile and
  pass unchanged; `tests/inbox_routing.rs` reaches the tool over HTTP on a
  configured server.
- [x] **T6 — `swarmail_create_inbox` (AC6).** Dispatch arm minting a UUID v7
  simple-hex inbox and `agent@<inbox>.<domain>`; `isError` text naming
  `--route-domain` when unset; descriptor appended after the focused pair
  with a closed empty `inputSchema`; `OUTPUT_SCHEMAS` entry with both
  strings required; `readOnlyHint: true` in the modern projection; the
  15 → 16 assertions in `src/mcp.rs`. **Test:** inline in `src/mcp.rs`:
  both arms, the sentinel-argument `-32602`, the schema key-set equality;
  `tests/inbox_routing.rs`: concurrent calls over real MCP return distinct
  pairs whose inbox passes `validate_inbox` and the alphabet, modern
  `structuredContent` equals the text block; `tests/mcp.rs`, `tests/p2.rs`
  and `tests/mcp_revisions/focused.rs` move to 16 with the focused indices
  unchanged and the March fixtures passing without regeneration.
- [x] **T7 — Three agents, one identity (AC1).** The headline proof in
  `tests/inbox_routing.rs`: three minted inboxes, three concurrent SMTP
  batches under one AUTH username, each inbox's search returns exactly its
  own ids, the AUTH username's inbox does not exist, and `/api/v1/inboxes`
  lists exactly the three. **Test:** that test, real SMTP and real MCP, no
  mocks.
- [x] **T8 — Documentation (AC2 in part).** README's AUTH-is-the-inbox
  sections gain the route-domain rule and the tool; the "15 tools" line,
  `src/static/llms.txt` and the `/mcp` description in
  `src/static/openapi.json` say sixteen; `docs/decisions/README.md` gains
  the 0112 row, status proposed. **Test:** the doc-pinning tests
  (`tests/listener_contract_docs.rs`, `tests/store_contract_docs.rs`, the
  openapi pins) stay green; `grep -c swarmail_create_inbox` on each file is
  non-zero.
- [x] **T9 — The eval harness (AC8).** Runner argv gains `--route-domain
  swarmail.test`; `Identity`/`ProvisionIdentity` gain required
  `smtp_identity`; `identities()` branches on `count > 1` to
  `fixture-app` plus `<name>.<n>@<inbox>.swarmail.test`; the fixture
  authenticates with `smtp_identity`; `mail.rs` header and `evals/README.md`
  reworded; `swarmail_create_inbox` in `SWARMAIL_TOOLS`. **Test:** evals'
  own suite: provision refuses an entry without `smtp_identity`; the
  isolation entries carry `fixture-app` and alphabet-valid inboxes; the
  oracle passes and null fails every task including isolation; trials
  stay sequential under accepted 0076; the proof asserts the `fixture-app`
  inbox holds no mail after the trial; every other task's provision
  payload is byte-equal to before apart from the new field.
- [x] **T10 — Gates and the honest metric (AC9, AC10).** Run every house
  gate locked, the evals crate standalone (fmt, clippy, tests), and record
  the oracle/null results; state that agent success rate is unmeasured.
  Commit unsigned, never push; the delivery names operator acceptance of
  0112 as the merge prerequisite. **Test:** the gate commands in
  `docs/house-rules.md` § Gates exit zero; `cargo deny check licenses`
  shows no new crate.
