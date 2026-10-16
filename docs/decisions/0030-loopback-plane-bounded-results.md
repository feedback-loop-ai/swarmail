# 0030 — The unauthenticated plane defaults to loopback, and result sizes are bounded

- **Status:** accepted (operator, 2026-10-16 — proposed by the run-10
  family's design panel; the tree implements it, run
  `brokkr-state-run10b-feature-txt-d6fc470d`'s review panel passed the
  repaired tree with no security residual, and it landed as #66, squash
  `8a47b50`)
- **Date:** 2026-10-16
- **Supersedes:** none
- **Records:** the network posture of the unauthenticated control plane
  (issue #48) — the binary's default bind, what the HTTP surface trusts,
  and the webhook SSRF exposure; and the result-size bound (issue #49) —
  `MAX_PAGE_SIZE`, the search fold, and the parse leg's thread placement.
- **Extends:** [0023](0023-off-worker-commits.md) §1 — the offload seam now
  carries the parse leg as well as the commit leg (same mechanism, same
  panic symmetry, no new ruling); [0018 §3](0018-shutdown-observed-waits-and-explicit-bounds.md)
  / [0025](0025-shared-domain-validation.md) — the refusal-not-clamp
  posture applied to a *new* bound (`limit`/`count`, the first bounds the
  two compat dialects publish).
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) — the 250 is
  still written only after the commit outcome is known; the parse merely
  stops occupying an async worker. [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md), [0005](0005-extraction-at-ingest.md)
  — untouched: extraction still happens at ingest (before the 250, never
  at query time); no wait, prune or insert ordering changes.
  [0014](0014-one-ordered-mutation-boundary.md) — the parse runs **before**
  the mutation boundary, never inside it: the boundary stays
  O(row-deletes), and a hostile body cannot serialize every accept behind
  the boundary lock. [0022](0022-bounded-data-raw-bytes.md) — `/raw` still
  serves the stored bytes; POP3 still serves octets.
- **Issue:** [#48](https://github.com/feedback-loop-ai/swarmail/issues/48),
  [#49](https://github.com/feedback-loop-ai/swarmail/issues/49)

## Context

Two closures over the landed state of run 9 (`66a45bb`), verified in code
this run:

**#49 — the workers pay for parsing and for copies that nothing reads.**
`build_email` (`src/smtp.rs:1199-1289`: MIME parse, HTML↔text, extraction
regexes over up to 50 MiB, plus a full `raw.to_vec()` per construction)
runs on the session's async worker at `src/smtp.rs:1002` before
`insert_async`; a few concurrent large messages stall banners and `await`
handlers. The seed path runs the same builder (`build_seed_email` ends in
`build_email`, `src/smtp.rs:1159`). Three REST handlers deep-copy whole
emails including `raw` — the await `Found` arm (`src/api.rs:692`),
`get_message` (`:773`) and `seed` (`:903`) — while `Email.raw` is already
`#[serde(skip)]` (`src/model.rs:52-53`) and serde's `rc` support already
proves the `Vec<Arc<Email>>` page byte-identical (`src/api.rs:446-456`,
pin at `:1020-1043`). The compat shims' `contains`
(`src/compat.rs:365-383`) allocates `String::from_utf8_lossy(raw)` **and**
a `.to_lowercase()` of it per mail per search; `limit`/`count` are
uncapped on every plane (upstream MailHog refuses above 250); each SSE
wake rebuilds and serializes up to `VIEW_CAP = 200`
(`src/threads.rs:38`) full mails per subscriber (`src/feed.rs:64-100`).

**#48 — the unauthenticated plane is wider than its own README.** All
three listeners default to `0.0.0.0` (`src/config.rs:33-42`) while
README:80 says `swarmail serve` means "the same ports on localhost". The
HTTP plane is unauthenticated: it reads every OTP, wipes inboxes, sets
chaos, and registers webhooks on arbitrary hosts — a blind SSRF whose
`/metrics` ledger rows (`src/api.rs:507-569`) reveal whether *something*
answered. The MCP Streamable HTTP transport requires servers to validate
`Origin` on incoming connections; without it a DNS-rebinding page could
drive `/mcp` from a browser. The Dockerfile's ENTRYPOINT passes no binds,
so the container inherits the binary default — and the container *must*
listen on `0.0.0.0` to be reachable.

## Decision

### §1 The binary defaults to loopback; the image and the chart bind explicitly

The three `default_value` literals in `src/config.rs:33-42` become
`127.0.0.1:<port>`. One rule covers all three listeners: **loopback by
default, remote access by explicit env** (`SWARMAIL_*_LISTEN`). The
Dockerfile passes `ENV SWARMAIL_SMTP_LISTEN=0.0.0.0:1025`,
`SWARMAIL_POP3_LISTEN=0.0.0.0:1110`, `SWARMAIL_HTTP_LISTEN=0.0.0.0:8025`
so the container's contract — it always listens on 1025/1110/8025 — is
kept by the image, not by the binary default; the chart passes the same
three through its existing generic `config:` mapping (a values-only edit).
README states the real defaults for both shapes. A per-plane split
(HTTP loopback, SMTP `0.0.0.0`) was refused: one rule is three identical
literals and one README sentence, and the co-tenant SMTP-sharing scenario
the split defends is one env away — loudly.

The cost is named: a bare-binary user who relied on `0.0.0.0` loses it
silently-until-they-read, and repairs it with one env. That is the
README:80 lie repaired in the direction the README already promised.

### §2 What the HTTP surface trusts: a loopback `Host` and a loopback `Origin` when the plane is loopback-bound

One middleware on the whole HTTP plane (not just `/mcp` — the blind-write
class a rebinding page can drive is every `DELETE`/`PUT`/`POST` route,
and `/mcp` is where the MCP transport's MUST bites), checking two
headers, in order:

- **`Host` first.** A request whose `Host` authority is not a loopback
  spelling (`localhost`, `127.0.0.0/8`, `[::1]`) of the bound HTTP port
  is answered **`403`** with its own refusal sentence — and a request
  with no `Host` line at all is refused likewise. This is the
  DNS-rebinding defense, and it exists *because* the Origin leg alone
  has a hole: a rebound page sends **same-origin** requests, and
  same-origin `GET`/`HEAD` carry **no `Origin` header at all** (the
  Fetch spec adds one only to CORS-tainted requests) — but the page
  still fetches its own hostname, so the rebind is visible in the
  `Host` line: `Host: attacker.example:8025`. The check is keyed to the
  bind posture (a loopback spelling *of the bound port*), not a static
  allowlist, so it serves the default bind without new vocabulary; a
  client without a `Host` is not HTTP/1.1 and is refused. The tree's
  axum runs HTTP/1.1 only (no `h2` in the lock), so there is no second
  protocol hop to hold, and an earlier draft's claim that hyper
  synthesizes `HOST` from an HTTP/2 `:authority` was false — the leg
  does not need it: a request that arrives without a `Host` is
  answered **`403`** (the check fails closed).
- **`Origin` second.** A request carrying an `Origin` header is
  answered **`403`** unless the Origin's authority is a loopback
  spelling of the bound HTTP port: the one header a browser sets
  honestly (it cannot be forged by page script and is present on every
  cross-site request), so a cross-site drive is named and refused.
- When the operator binds off-loopback explicitly, both checks are
  **inert**: the bind *is* the vocabulary, and the exposure is the
  operator's recorded choice. No allowlist knob, no new configuration
  surface.
- A request without `Origin` passes the Origin leg untouched — curl,
  agents, the stdio bridge and every test suite are unaffected (they
  name their `Host` honestly, which is exactly what the Host leg
  admits). The web UI is same-origin loopback under the default and
  passes; it does not call `/mcp` (`src/static/ui.js` has no `/mcp`
  reference).

This closes the DNS-rebinding drive on both of its shapes: the page's
*writes* (cross-origin, `Origin: http://attacker.example:8025` present
and non-loopback — refused by the Origin leg) and its *same-origin
reads* (no `Origin`, the attacker's `Host` — refused by the Host leg).
The earlier draft of this section refused Host validation on the
premise that reads were never reachable and a rebind serves
`localhost`; that premise was wrong on both counts — same-origin reads
carry no Origin, and the rebound page spells its own hostname, not
`localhost` — and the Host leg is the repair, landed with the review
that caught it.

### §3 `MAX_PAGE_SIZE = 250`: a page is refused above it, never clamped

One shared check beside the constants it guards (`src/api.rs`, the
`checked_wait_timeout_ms` shape): a `limit`/`count` above **250** refuses,
by name, with the field and the accepted range in one sentence — the 0018
§3 / 0025 posture applied to the first bound the compat dialects publish.
The nine staging sites: REST `list` and `await` (`src/api.rs:638, 691`);
the four compat pages (Mailpit list/search, MailHog v2 list/search,
`src/compat.rs:92-93, 108-109, 212-213, 241-242`); MCP `get_latest`'s
`count`, `swarmail_search_emails`' `limit`, and the wait surfaces' `count`
(`src/mcp.rs:235, 268, 307`). Defaults are unchanged (100 REST/MCP, 50
compat — different dialects, 0025's own ruling); `openapi.json` publishes
`maximum: 250` beside every `limit`; `llms.txt` states the cap.

Three edges ruled explicitly:

- **The wait surfaces' `count` is capped on both planes.** On REST, the
  wait's `count` is the trigger and its page is `limit`'s business; on
  MCP, `Found` truncates to `count` (`src/store.rs`, `wait_for`) — the
  same argument is the page. One domain across planes (0025's lesson)
  means the cap rides on both: a caller cannot trigger at more than 250
  matches and must poll or assert instead. 0010's `count=0` semantics are
  untouched (at least one; the negative assertion).
- **`assert` is exempt.** Its `count` is the *expected exact count*
  (0010) — an assertion of any magnitude is legitimate — and its `limit`
  is inert (reaches no wire, no read). Capping an inert or assertive
  value would be a new rule for no observable; a test pins the inertness.
- **`offset`/`start` are not page sizes** and stay uncapped — a skip is
  cheap, a refusal there would break no adversary's lever.

### §4 The search fold is simple case-insensitive matching

`contains` keeps its "every candidate, no short-circuit" shape and its
case-insensitive semantics, but stops allocating two full-size strings
per mail per search: the needle is escaped into a `(?i)`-anchored regex
(`regex` is already in the tree) matched against the fields and the
lossy raw. The fold is **simple case folding**, not `str::to_lowercase`'s
full mapping — identical on ASCII and every needle this API realistically
serves (codes, links, addresses); the margin is exotic Unicode and is
named here rather than pretended away. The needle was already lowercased
at the call site (`src/compat.rs`, `run_search`), so today's asymmetry
(lowered haystacks, verbatim needle at the leaf) is preserved at the
boundary and the swap is a fold-implementation change, not a semantics
change on realistic inputs.

### §5 Webhooks: the SSRF exposure is documented, not restricted

The server dials attacker-chosen URLs, including internal ranges — that
is the feature (a test tool's webhook target is *any* host) and it is
also a blind SSRF. It stays, documented in the README beside the
registration example: the server makes an outbound HTTP POST to any
registered URL (redirects already refused, `Policy::none`), and the
`/metrics` ledger rows are per-instance, not per-URL — so the readback
signal is coarse. A target allowlist was refused (new vocabulary, breaks
the purpose); HTTP auth on the control plane was refused (a contract
break — 0004's per-test-inbox story, every test suite and the UI assume
an open plane — plus a new secret-distribution failure mode this realm
cannot maintain). One stale fact is corrected here for the record: the
fan-out is **not** uncapped — [0029 §5](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
landed `WEBHOOK_DELIVERY_CAP = 32` per normalized-URL sink key
(`src/webhook.rs:51, 314`), so per-target probing is already bounded at
the fan-out.

### §6 The residuals this run refuses to buy (recorded honestly)

- **A parse panic stays a dead session, not a `451`.** `mail-parser` is
  designed not to panic on input, no acceptance line demands the arm, and
  a panic arm over an unproven path mints a cold branch needing a
  panicking-parser seam plus a 0020-rule-2 harness. If a real panic path
  over hostile bytes is ever evidenced, the `451` arm is its own decision.
  Until then the pre-existing behavior (session task unwinds, peer sees
  the socket close) is preserved exactly, by 0023 §5's panic symmetry.
- **SSE wakes are not coalesced.** Each wake rebuilds ≤ `VIEW_CAP = 200`
  mails per subscriber and the store is the truth (0002 re-applied per
  iteration), so dropping wakes is *safe* — the cost is bounded CPU, not
  correctness. No acceptance line names it; the bench records the wake's
  cost so the number is on the table. The shape of a future fix (a
  per-inbox dirty bit drained by the feed's `unfold` tick) is recorded
  for whoever has evidence.
- **At-most-once webhooks, no outbox; no cert-expiry warning; a chaos
  `delay_ms` can occupy a session slot up to its 300 s bound** — all
  three are the landed postures of 0027/0029 and are unchanged here.

## Why

- The parse move is 0023's own argument carried one leg further: the
  acceptance leg is ordering, not thread identity, and the CPU leg has no
  reason to occupy a worker the banners and `await` handlers owe. The
  parse runs before the boundary, so the boundary's O(row-deletes)
  property — and every other session's 250 — is unreachable from a
  hostile body.
- The `Arc` page swaps delete the last deep copies of bytes serde provably
  skips; the wire is byte-identical (the `EmailList` pin is the pattern),
  and the "slow-reader tail" is **not new**: today's deep clone holds the
  identical liveness tail (the response owns a full copy until written).
  The swap strictly removes the transient duplicate.
- The loopback default is the only posture in which README:80 becomes
  true *and* the container keeps working without new vocabulary: the
  image's explicit `ENV` is the container contract, the chart's values
  are the deployment contract, and the binary's default is the local-dev
  contract — three contracts, three places, one rule.
- The 250 cap is upstream MailHog's own number: the compat shims gain
  conformance for free, and the refusal-not-clamp posture (0018 §3) is
  the already-accepted shape — a clamp would silently truncate pages and
  break every exact-count test that pages past the cap.

## Consequences

- `src/store.rs` ends at a one-word diff (`offload`'s visibility);
  `src/persist.rs` at zero diff; the mutation boundary's logic untouched.
- `AppState` carries the bound HTTP port so the Origin middleware can key
  its check to the bind posture; `serve`'s callers construct it (one
  inline test site).
- `openapi.json` gains `maximum: 250` on every `limit`; the README gains
  the posture paragraph (binds per shape, the Origin rule, the webhook
  SSRF sentence, the page cap); `llms.txt` states the cap.
- A supplied `limit: 251` (or `count: 251` on a wait/get_latest) now
  answers a refusal where it previously answered an unbounded page — the
  one wire-visible behavior change of #49, named in §3 and pinned by test
  on every plane.
- The measured before/after numbers (`benches/read_stall.rs`: large-mail
  await, large compat search, the `contains` fold) are recorded verbatim;
  if the win is not clear, the operator rules on the numbers as measured
  (0023's bench discipline — no wall-clock validity assertions).
- 0018 carries no marker for this decision yet: an `Amended by` line
  lands only upon acceptance (0028's precedent), as a landing task.
