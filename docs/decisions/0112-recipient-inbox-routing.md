# 0112 — Recipient inbox routing under a configured route domain

- **Status:** proposed
- **Date:** 2026-10-31
- **Scope:** Epic #164, issue #157 (isolate parallel agents' inboxes,
  provisioning one per run). Priority P2.
- **Specification:** [recipient inbox routing](../../specs/recipient-inbox-routing/spec.md).
- **Amends:** accepted [0004](0004-per-test-inboxes-via-smtp-auth.md) §1
  for servers configured with a route domain only — a routed recipient
  outranks the AUTH username there; an unconfigured server is byte-identical
  to today. Accepted [0055](0055-agent-eval-harness.md) §6 for the eval
  catalogue's `inbox-isolation` task only — its three agents share one SMTP
  AUTH identity and are separated by routed recipients, not by three AUTH
  usernames; every other task keeps 0055's wire shape.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) (routing is
  settled at `RCPT TO`, before the parse, the park and the synchronous
  insert; the `250` still follows the commit), [0002](0002-awaits-subscribe-before-checking.md)
  (the in-flight row is parked under the routed inbox, so a waiter on that
  inbox sees the delivery it will receive), [0003](0003-oldest-only-pruning.md)
  (the cap applies per routed inbox, oldest-only, unchanged), 0005 (ingest
  extraction receives an inbox name and nothing else changes), 0012 (a
  failed commit still refuses), 0027 (the recipient ceiling is checked
  first), 0032 (no inbox exists without mail: the minted address allocates
  nothing), 0034 §3 (refusal order at `RCPT`), 0044 §3 and 0046 §4 (the one
  inbox rule: every routed name is admissible by it), proposed 0082 and
  0094 §7 (appended descriptor, object-root `outputSchema`, modern
  annotations).
- **Number:** drafted as 0106; renumbered to 0112 at integration on 2026-10-31,
  after open PR #292 held 0106–0107, main held up to 0109, #159 held 0110 and
  #156 took 0111.

## Context

Today the SMTP AUTH username names the session's inbox and an
unauthenticated session lands in `default` (0004 §1; `src/smtp.rs`
around the DATA park). The recipient has no vote. An application that
sends with one AUTH identity therefore fills one inbox, and three agents
driving that application in parallel read one shared stream. The search
`to` filter is a substring match over that stream, not isolation.

The design panel verified the issue's premise against the eval fixture
and found it wrong in one respect: `evals/src/fixture.rs` authenticates
per account, with the agent's inbox as the AUTH username, so today's
`inbox-isolation` task passes only because the app is configured with
every agent's inbox — the reconfiguration the issue wants gone. Routing
alone would leave that metric untouched. This record therefore also
moves the isolation task to one AUTH identity, which is why it amends
0055 §6.

Only the operator accepts this record. A default-off override of an
accepted ruling is still an amendment: merge waits for acceptance (house
rules, "code under a proposed decision"; 0045 §8). Compatible work — the
flag, the tool, the tests — may be built and reviewed meanwhile.

## Proposed ruling

1. **One route domain, one rule.** `swarmail serve --route-domain <domain>`
   (`SWARMAIL_ROUTE_DOMAIN`), default unset. The value is one or more DNS
   labels joined by `.`, each `[A-Za-z0-9-]{1,63}`, stored ASCII-lowercased;
   anything else refuses at startup. With the domain set, an SMTP envelope
   recipient `<local>@<label>.<domain>` names the inbox `<label>`,
   ASCII-lowercased. The host is compared whole-label: `x@notswarmail.test`
   is not under `swarmail.test`. The local part is free, so one inbox may
   own many addresses. Only the envelope (`RCPT TO`) is read; no header
   ever chooses an inbox.
2. **Under the route domain, every recipient names an inbox or is refused.**
   A recipient whose host is the route domain itself or ends with
   `.<domain>` must yield exactly one label of `[a-z0-9-]{1,63}` after
   lowercasing. Otherwise (`agent@swarmail.test`, `a.b.swarmail.test`,
   `bad_label.swarmail.test`, a 64-character label) the `RCPT` is refused
   `550 5.1.3 Recipient names no inbox under the route domain` and nothing
   is appended. Silent fallback to the AUTH inbox is the re-merge the
   feature exists to prevent. Recipients outside the route domain have no
   vote and are recorded in `recipients` as today.
3. **Precedence:** routed recipient, then AUTH identity, then `default`.
   The routed inbox is fixed on the first routed `RCPT` of a transaction.
   A later `RCPT` naming a different routed inbox is refused
   `452 4.5.3 One routed inbox per transaction`, after the ceiling check
   and before chaos (0034 §3); the transaction stays open with the
   recipients accepted so far. RFC 5321 makes 452 at `RCPT` the reply a
   sender answers by delivering to the accepted recipients and retrying
   the rest in another transaction, so a message to two agents reaches
   both inboxes as two messages. `RSET` and the consumption of the
   envelope at `DATA` clear the routed inbox with the rest of the envelope.
4. **Routing precedes the park.** The inbox named by rule 3 is the one
   `park_delivery` receives and the one `build_email` writes; both are
   computed from the already-collected envelope at `DATA`, before the
   parse is offloaded. A derived name is admissible by the one inbox rule
   by construction (the alphabet is a strict subset), so a `250` never
   admits mail no read surface can name.
5. **`swarmail_create_inbox` mints, it does not allocate.** A zero-argument
   MCP tool returning `{"inbox": "<32 lowercase hex characters>",
   "address": "agent@<inbox>.<route-domain>"}`, the inbox a fresh UUID v7
   from the crate already in the tree. It takes no lock, writes no store
   or data-file state, registers nothing and expires nothing: an inbox
   with no mail does not exist (0032) and the first routed mail creates
   it (0004). Waiting on the minted name before any mail works because
   `subscribe` creates the channel lazily (0002). On a server without a
   route domain the tool answers an `isError` text result naming
   `--route-domain`. The descriptor is appended after the focused waits
   (0094 §7), closed `inputSchema`, object-root `outputSchema` with both
   members required, modern `readOnlyHint: true`; the tool count moves
   15 → 16 and March keeps its thirteen pinned entries.
6. **Isolation is addressing, not authentication.** With one AUTH identity
   any sender can address any run, and every read surface already accepts
   any inbox name; 0004's loopback dev-tool posture is unchanged. Three
   parallel agents never see each other's mail because each is given its
   own address and reads its own inbox, which is what the acceptance
   criterion means. A misbehaving agent that names a peer's inbox is not
   stopped here, and the eval's foreign-token veto is what scores that.
7. **The isolation task sends with one AUTH identity.** The eval runner
   always starts Swarmail with `--route-domain swarmail.test` (inert for
   `@example.test` identities, so the other fifteen tasks keep their wire
   shape and the #151 baseline stays comparable). The provision entry
   gains a required `smtp_identity`, which the fixture uses as the AUTH
   username for that account's deliveries; the isolation task provisions
   all three accounts with `smtp_identity: "fixture-app"` and identities
   `<name>.<n>@<inbox>.swarmail.test`. Scoring still reads only the frozen
   app snapshot (0055 §3); the harness's own proof additionally asserts
   that the `fixture-app` inbox holds no mail after the trial.
8. **What is not done.** No `+tag` or second address syntax, no pattern
   language, no inbox registry, lease, TTL or delete tool, no fan-out of
   one message to several inboxes, no per-inbox read authorization, no
   REST provisioning endpoint, no Helm values (the chart renders
   `config:` as `SWARMAIL_*` environment already). Agent success rate on
   the isolation task needs paid model trials and is reported as
   unmeasured; the oracle and null controls run locally.

## Alternatives ruled and costs

- **Subdomain form over `+tag` (robustness preferred `+tag` on a fixed
  domain).** One parser, no precedence question between two syntaxes, the
  local part stays free for the sibling-addresses shape, and the issue
  names this form first. Cost: an application restricted to its own
  sending domain cannot use it, and a caller that rewrites the recipient
  domain silently loses routing; a later `+tag` decision is not foreclosed.
- **Refuse a malformed label under the domain (robustness) rather than
  ignore it (simplicity).** A loud 550 beats a silent re-merge. Cost: a
  configured server refuses addresses it accepted before, for its route
  domain only.
- **452 per conflicting recipient (simplicity) over 550 for the whole
  transaction (robustness).** Real senders deliver the accepted recipients
  and retry the deferred one, so a CC to two agents reaches both. Cost: a
  sender that treats 452 as fatal loses the copy it would also have lost
  under a 550.
- **A stateless minter (both) over a registry.** Required by 0032; a
  counter would re-mint after restart and merge two runs. Cost: a minted
  inbox is absent from `/api/v1/inboxes` until its first mail, and nothing
  tells a caller a name is "taken" — it never is.
- **Route-domain validation at startup (robustness; simplicity conceded).**
  A typo'd domain fails before any session, not inside one. Cost: a small
  parser and its tests.
