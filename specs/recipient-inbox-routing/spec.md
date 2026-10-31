# Recipient inbox routing and `swarmail_create_inbox`

Decision: 0106 — [Recipient inbox routing under a configured route domain](../../docs/decisions/0106-recipient-inbox-routing.md)

## Feature and rationale

Issue #157 (epic #164, P2): let parallel agents on one application and one
Swarmail each own a private inbox without reconfiguring the application.

Today the SMTP AUTH username names the session's inbox and an
unauthenticated session lands in `default` (accepted
[0004](../../docs/decisions/0004-per-test-inboxes-via-smtp-auth.md) §1;
`src/smtp.rs`, the DATA arm's park). The recipient has no vote. An
application that sends with one AUTH identity fills one inbox, and three
agents driving it in parallel read one shared stream; the search `to`
filter is a substring match over that stream, not isolation. The gap the
issue names is real.

The panel also verified the issue's claim that the eval fixture "is exactly
that case", and it is not: `evals/src/fixture.rs` sends every delivery with
that account's inbox as the AUTH username (`evals/src/mail.rs` says so in
its header), so today's `inbox-isolation` task passes because the app was
provisioned with every agent's inbox. Routing alone would leave the metric
untouched. The isolation task therefore moves to a single AUTH identity in
the same delivery, and proposed 0106 amends accepted
[0055](../../docs/decisions/0055-agent-eval-harness.md) §6 for that task
alone.

The change amends accepted 0004 for configured servers. Under the house
rules a change that contradicts an accepted ruling waits for operator
acceptance of the decision that amends it: the work is built, reviewed and
gated, and **merge waits for the operator's acceptance of 0106**. Nothing
here accepts it.

## Panel adjudication

Both positions agreed on the spine: routing is a pure function of the
envelope and immutable config, computed at `RCPT` and used before the park;
the routed recipient beats AUTH; one message goes to one inbox; the create
tool allocates nothing; isolation is addressing, not authentication; the
override is default-off. The rulings below settle where they differed.

| Question | Ruling | What the losing side costs |
| --- | --- | --- |
| Address shape | Simplicity's single subdomain form, `<local>@<label>.<domain>`, under one `--route-domain` flag. Robustness's `inbox_routing = off\|plus\|subdomain` mode and `+tag` are rejected: two syntaxes mean a precedence rule and a second parser for no criterion. | An app restricted to its own sending domain cannot use the feature; a caller that rewrites the recipient domain loses routing silently. A later `+tag` decision is not foreclosed. |
| A malformed label under the domain | Robustness: refuse at `RCPT` (`550 5.1.3`) rather than simplicity's "no match, no vote". A silent fall-through to the AUTH inbox is the re-merge the feature exists to prevent. Admitted labels are `[a-z0-9-]{1,63}` after lowercasing, a strict subset of the one inbox rule, so `validate_inbox` cannot fail on a routed name and is not called on it. | A configured server refuses, for its route domain only, addresses it would have accepted before. |
| Two routed inboxes in one transaction | Simplicity: refuse the conflicting `RCPT` with `452 4.5.3`, keep the transaction open. Robustness's whole-transaction `550` is rejected: RFC 5321 makes 452 at `RCPT` the reply a sender answers by delivering to the accepted recipients and retrying the deferred one, so a CC to two agents reaches both inboxes as two messages. The reply text is routing's own, not the ceiling's. | A sender that treats 452 as fatal loses the copy it would also have lost under a 550. |
| Minted name | Both: stateless. UUID v7 simple hex from the crate already in the tree (simplicity); robustness's random token in the admitted alphabet is the same property with a second renderer, rejected. | A minted inbox is absent from `/api/v1/inboxes` until its first mail (0032 working as written). |
| Route-domain validation | Robustness: validate the flag at startup with the same label rule; simplicity conceded. | A small parser and its tests. |
| Where the eval change lands | Simplicity's minimal change: `smtp_identity` on the provision entry, one branch in `identities()`, the runner always passing `--route-domain swarmail.test`. The `fixture-app` emptiness check lives in the harness's proof test, not in scoring, because scoring reads only the frozen app snapshot (0055 §3). | Whether agents discover and call `swarmail_create_inbox` stays unmeasured; the tool is proved on the wire in `tests/`. |

Rejected by both and by this spec: an inbox registry, lease, TTL or delete
tool (contradicts 0032); fan-out of one message to several inboxes (several
inserts behind one `250` with no honest reply for a partial failure, pressure
on 0001 and 0012); per-inbox read tokens (touches every read surface against
an actor 0004's posture declines to defend against); a REST provisioning
endpoint; Helm values; always-on routing with a compiled domain (amends 0004
for every deployment, and the issue says "configured").

## Contract

### Configuration

- `swarmail serve --route-domain <domain>`, environment
  `SWARMAIL_ROUTE_DOMAIN`, default unset. The value is one or more DNS
  labels joined by `.`, each `[A-Za-z0-9-]{1,63}`; it is stored
  ASCII-lowercased. An empty value, an empty label, a label outside the
  alphabet or over 63 characters, or a leading or trailing `.` refuses at
  startup with a message naming the flag; no listener is bound.
- `SmtpConfig` gains `route_domain: Option<String>`; `Default` is `None`.
  An unset server is byte-identical to today on every plane.

### The routing rule (0106 §1–§4)

- Input is the `RCPT TO` envelope path only. No header chooses an inbox.
- A recipient is **under the route domain** when its host, ASCII-lowercased,
  equals the domain or ends with `.` followed by the domain. The comparison
  is whole-label: with `swarmail.test`, `x@notswarmail.test` is not under
  it and has no vote.
- A recipient under the domain **names the inbox** `<label>` when the
  remainder before `.<domain>` is exactly one label of `[a-z0-9-]{1,63}`
  after lowercasing. `Agent@ADA.Swarmail.TEST` names `ada`.
- A recipient under the domain that names no inbox (`agent@swarmail.test`,
  `a@x.y.swarmail.test`, `a@bad_label.swarmail.test`, a 64-character label)
  is refused `550 5.1.3 Recipient names no inbox under the route domain`
  and is not appended. The transaction stays open.
- Recipients not under the domain have no vote and are recorded in the
  stored `recipients` exactly as today.
- The transaction's routed inbox is fixed by its first routed `RCPT`. A
  later `RCPT` naming a different routed inbox is refused
  `452 4.5.3 One routed inbox per transaction` and is not appended; the
  transaction stays open with the recipients accepted so far. A later
  `RCPT` naming the same routed inbox is accepted.
- Order at `RCPT`: syntax (501/555) → recipient ceiling (`452 4.5.3 Too
  many recipients`, 0027) → routing admission (550, then 452 above) →
  chaos → append (0034 §3). A refused or chaos-failed `RCPT` leaves the
  transaction's routed inbox unchanged; the inbox is set only on the
  successful append.
- At `DATA` the inbox is `routed`, else the AUTH identity, else `default`,
  computed once from the taken envelope before the parse is offloaded. That
  one name is what `park_delivery` parks under, what `build_email` writes,
  and what the `swarmail_deliveries_in_flight` row names. `RSET` and the
  envelope's consumption at `DATA` clear it with the rest of the envelope.
- Everything after the inbox is named is unchanged: synchronous insert and
  the `250` after the commit (0001), the lazy watcher channel (0002),
  per-inbox oldest-only pruning (0003), ingest extraction (0005), the
  commit-failure 451 (0012), persistence and restore.

### `swarmail_create_inbox` (0106 §5)

- Name `swarmail_create_inbox`; no arguments; `inputSchema` is the closed
  empty object (`additionalProperties: false`), so a sentinel key refuses
  `-32602` naming it before any effect, like every other tool.
- Success: `{"inbox": "<uuid-v7 as 32 lowercase hex characters>",
  "address": "agent@<inbox>.<route-domain>"}` as the text block and, on
  2025-06-18 and 2025-11-25, the same value as `structuredContent`. The
  `outputSchema` is an object root with `inbox` and `address` strings, both
  required, `additionalProperties: false`.
- It takes no store lock, writes no store or data-file state, parks nothing,
  registers nothing and expires nothing. Concurrent calls return distinct
  pairs by construction. The minted inbox passes `validate_inbox` and the
  routing alphabet.
- On a server without a route domain the result is an `isError` text
  result whose text names `--route-domain`; nothing else changes.
- The descriptor is appended after the two focused waits (0094 §7), so
  every earlier index is unchanged; modern revisions annotate it
  `readOnlyHint: true`. The advertised count moves 15 → 16 in
  `src/mcp.rs`, `tests/mcp.rs`, `tests/p2.rs` and
  `tests/mcp_revisions/focused.rs`; March keeps its thirteen pinned entries
  and existing bytes. The tool sits under the existing `ToolRate`.
- `McpContext` gains the route domain, supplied from the server's config by
  the HTTP endpoint; the stdio bridge needs nothing, it speaks to that
  endpoint.

### The eval harness (0106 §7)

- The runner starts Swarmail with `--route-domain swarmail.test` for every
  trial. It is inert for `@example.test` identities, so the fifteen other
  tasks keep their wire shape and the #151 baseline stays comparable.
- The provision entry (`ProvisionIdentity`) gains a required
  `smtp_identity`; the fixture authenticates each account's deliveries with
  it instead of the account's `inbox`. The `evals/src/mail.rs` header
  sentence and the `evals/README.md` isolation sentence change to match.
- `identities()` holds the one branch: for `count > 1` (the isolation task)
  every entry has `smtp_identity: "fixture-app"` and identity
  `<name>.<n>@<inbox>.swarmail.test` with inbox `eval-<short>-<name>`
  (lowercase, within the alphabet and 63 characters); otherwise
  `smtp_identity` is the inbox and the identity stays `@example.test`.
- `swarmail_create_inbox` is added to `SWARMAIL_TOOLS` in
  `evals/src/wrappers.rs`. The oracle is unchanged (it reads `EVAL_INBOX`).
- The harness proof for the isolation task additionally asserts, through
  the real MCP or REST surface after the trial, that inbox `fixture-app`
  holds no mail — otherwise the task cannot tell routing from AUTH fallback.
- The metric: oracle and null controls run locally and are reported. Agent
  success rate on the isolation task needs paid model trials and is
  **reported as unmeasured**.

### Documentation

README (the AUTH-is-the-inbox sections and the "15 tools" line),
`src/static/llms.txt` and the `/mcp` description in `src/static/openapi.json`
(the "fifteen in all" sentences) describe the route domain, the tool and the
new count. `docs/decisions/README.md` gains the 0106 index row, status
proposed. No release note is cut here.

## Acceptance Criteria

- [ ] AC1 — **Three parallel agents, one AUTH identity, one Swarmail.** A
  `tests/` suite starts a server with a route domain, mints three addresses
  with `swarmail_create_inbox` over real MCP, sends three concurrent batches
  over real SMTP with one AUTH username to those addresses, and proves each
  inbox's search returns exactly its own message ids while the AUTH
  username's inbox does not exist. No mocks.
- [ ] AC2 — **The routing rule is recorded.**
  `docs/decisions/0106-recipient-inbox-routing.md` exists with status
  `proposed`, names the 0004 §1 and 0055 §6 amendments, and records that
  0001–0003 and 0005 are preserved. Its index row is in
  `docs/decisions/README.md`.
- [ ] AC3 — **Default-off.** With no route domain, an AUTH session lands in
  its AUTH inbox, an unauthenticated one in `default`, and every existing
  0004 test passes unchanged. The tool answers `isError` naming
  `--route-domain`.
- [ ] AC4 — **The rule on the wire.** Case-folding (`Agent@ADA.Swarmail.TEST`
  → `ada`), whole-label comparison (`notswarmail.test` has no vote), a
  non-matching CC that does not disturb routing and is still recorded, the
  `550 5.1.3` refusal for each malformed shape under the domain storing
  nothing, the `452 4.5.3` refusal of a second routed inbox followed by a
  successful retry in a new transaction, the same routed inbox accepted
  twice, and `RSET` clearing the route are each proved against a real SMTP
  server.
- [ ] AC5 — **Routing precedes the park (0002).** A `wait_for_email` on a
  minted, never-mailed inbox started before the send returns that mail;
  and during a parked delivery the in-flight row names the routed inbox,
  not the AUTH identity.
- [ ] AC6 — **The tool's shape.** `tools/list` advertises 16 tools on every
  revision with `swarmail_create_inbox` last; modern revisions carry its
  object-root `outputSchema` and `readOnlyHint: true`; a sentinel argument
  refuses `-32602`; concurrent calls return distinct pairs whose inbox
  passes `validate_inbox`; the success includes `structuredContent` on
  modern revisions and March bytes for the thirteen pinned entries are
  unchanged.
- [ ] AC7 — **Startup validation.** `--route-domain` with an empty value, an
  empty label, a bad character, a 64-character label or a leading/trailing
  dot refuses before any bind, naming the flag; a mixed-case value is
  accepted and routes lowercased.
- [ ] AC8 — **The eval harness.** With the runner's `--route-domain
  swarmail.test`, the isolation task's three accounts share
  `smtp_identity: "fixture-app"`, the oracle passes all three, null fails
  with its idle class, two overlapping isolation trials pass, and the proof
  asserts the `fixture-app` inbox holds no mail. Every other task's wire
  shape is unchanged. `evals/` fmt, clippy and tests pass standalone.
- [ ] AC9 — **Gates.** Every house gate in `docs/house-rules.md` § Gates is
  clean and locked: fmt, clippy, tests, `brokkr compile`, the coverage gate
  at exact 100% with its selftest, the publish-guard and seal proofs,
  `cargo deny check licenses`. No new dependency.
- [ ] AC10 — **Honest metric.** The delivery reports the oracle and null
  controls it ran and states that agent success rate is unmeasured without
  paid trials.
