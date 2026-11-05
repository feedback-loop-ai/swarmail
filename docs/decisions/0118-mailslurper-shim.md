# 0118 — A MailSlurper service-API shim on an opt-in listener, one global mailbox

- **Status:** accepted (operator, 2026-11-02 — "Accept 0116, 0117, 0118",
  given in chat to the dispatcher).
- **Date:** 2026-11-02
- **Scope:** issue #314 (operator request 2026-11-02; it gates Alkemio's
  nightly server-API suite on their test cluster with Swarmail as the mail
  sink). Run `issue-314-a-mailslurper-compatib-df2066a8`, from `main`
  `5645bd9`.
- **Specification:** [MailSlurper shim](../../specs/mailslurper-shim/spec.md).
- **Amends:** accepted [0011](0011-shim-delete-contract.md), only its
  sentence "`DELETE /api/v1/delete-all` is unchanged and remains the only
  cross-inbox wipe". With this decision there are two spellings of the one
  cross-inbox wipe: `DELETE /api/v1/delete-all` on the HTTP plane, and
  `DELETE /mail` with the literal body `{"pruneCode":"all"}` on the
  MailSlurper listener, served only when an operator enables that listener.
  Both call the same store operation (`clear_all`). 0011's rule for
  destructive routes is applied to the new route unchanged: an explicit
  selector, or a `400` that deletes nothing. Also amends accepted
  [0004](0004-per-test-inboxes-via-smtp-auth.md), only its consequence
  "`clear_all` … is the only cross-inbox destructive endpoint". That
  sentence is narrowed: `clear_all` stays the only cross-inbox destructive
  store operation, but it is reached by two endpoints, the second being
  the MailSlurper listener's `DELETE /mail`, served only when an operator
  enables that listener. On acceptance, 0004 and 0011 each get a dated
  marker pointing here, as 0112 did for what it amended. Adding those
  markers belongs to the acceptance, not to the delivery run.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) (the shim only
  reads committed mail, so a `250` is visible to `GET /mail` at once),
  [0002](0002-awaits-subscribe-before-checking.md) (the wipe goes through
  the store's clear path, which releases emptied inboxes for `await`
  watchers to rescan), [0003](0003-oldest-only-pruning.md) (MailSlurper's
  age-based prune codes are refused by name; no time-keyed eviction is
  added), [0005](0005-extraction-at-ingest.md) (every item field comes
  from the `Email` built at ingest; no MIME parsing at request time), 0014
  (a failed commit answers `503 storage commit failed`), 0030 §2 and 0043
  (the listener sits behind the same Host/Origin guard as the HTTP plane),
  0044 §§3, 5 and 6 (closed query, page work off the async worker, a byte
  cap that refuses the whole page by name).

## Context

Alkemio's test suites read and clear mail through MailSlurper's service API
(`lib/src/utils/mailslurper.rest.requests.ts` in `@alkemio/tests-lib`,
branch develop). The operator described the two calls they make against
`MAIL_SLURPER_ENDPOINT` (for example `http://localhost:4437/mail`, or
`https://test-alkem.io/mailslurper-api/mail` behind an ingress):

- `GET <endpoint>` with `Accept: application/json` and
  `Accept-Encoding: gzip, deflate, br`. The client reads
  `body.mailItems`, an array, and `body.totalRecords`. From each item it
  reads `subject`, `body` (regex-scanned for verification and invitation
  links) and `toAddresses` (an array of strings).
- `DELETE <endpoint>` with the JSON body `{"pruneCode":"all"}`.

Real MailSlurper omits `mailItems` for an empty mailbox, and the client
normalises that.

MailSlurper has one global mailbox. Swarmail has per-inbox isolation
(0004, 0112) and already exposes a global view on its HTTP plane: the
MailHog and Mailpit shims read every inbox when no `?inbox=` is given, and
`DELETE /api/v1/delete-all` clears every inbox.

**Verification gap.** No seat in this run can read the upstream source
(`mailslurper/mailslurper`) or `@alkemio/tests-lib`: web access is
switched off and neither is vendored. The consumer contract above is the
operator's description and is binding. The other shapes below are the
design council's recollection of upstream. They are marked *recalled* here
and in the guide, and §7 asks the operator to check them before accepting.

## Ruling

### 1. Where it is served: a dedicated listener, off by default

`swarmail serve --mailslurper-listen <host:port>`
(`SWARMAIL_MAILSLURPER_LISTEN`). There is no default: unset means no
listener, no socket and no route. When set, the listener binds before any
server task is spawned, alongside the SMTP, POP3 and HTTP binds and after
the data file has been restored. A bind failure fails startup. The operator
supplies the address, so it is loopback only if the operator writes a
loopback address. The documented examples use `127.0.0.1:4437` locally and
`0.0.0.0:4437` in containers.

The listener serves the MailSlurper routes and nothing else. It is wrapped
in the HTTP plane's own Host/Origin guard (0030 §2, 0043). The guard's
loopback vocabulary is the MailSlurper listener's own bound port, so a
DNS-rebound page cannot reach the wipe on a loopback bind. It uses plain
HTTP only. TLS terminates at an ingress, as it does for real MailSlurper
behind `https://test-alkem.io/mailslurper-api/`.

**The HTTP port does not serve these routes, under any prefix.**
`GET /mail` on the HTTP plane stays a `404`. An ingress that publishes the
shim under a path prefix strips the prefix and forwards to the listener's
root. That is what Alkemio's ingress already does for MailSlurper.

Why a listener and not routes at the root of the HTTP port:

- **The operator's stated preference**, and the shape MailSlurper clients
  assume: the service API at the root of its own host:port.
- **Enabling MailSlurper semantics is the operator's act, not a client's.**
  A global wipe already exists on the HTTP plane under swarmail's own
  explicit name. A second spelling at a generic path (`/mail`), present on
  every deployment, would let any client of a shared Swarmail adopt
  MailSlurper's one-mailbox semantics. With the listener, a Swarmail runs
  in global-mailbox mode only when its operator says so. 0011 can then be
  amended narrowly, rather than gaining an always-on second wipe.
- **The HTTP plane's namespace stays swarmail's.** `/mail` and
  `/mailcount` at its root would claim generic names for one foreign tool.

What this costs, accepted: one config field, one bind, one optional address
on `RunningServer`, and their startup tests. The Helm chart does not expose
the port in this change: the chart's Service and NetworkPolicy list exactly
SMTP, POP3 and HTTP, and extending them also means changing
`scripts/chart-verify.sh`. A cluster deployment enables the listener
through the chart's existing env contract
(`config: {SWARMAIL_MAILSLURPER_LISTEN: "0.0.0.0:4437"}`) and publishes it
with a Service of its own, written by the deployer, that selects the
release's pods on port 4437 (the guide shows one). With `networkPolicy`
enabled, the chart's policy does not admit 4437. Chart-native exposure is
a follow-up issue.

### 2. Mailbox mapping: one global view, stated plainly

- `GET /mail`, `GET /mailcount` and `GET /mail/{id}` read **every inbox**
  as one mailbox. The list order is newest first by
  `(received_ms, id)`, the order every compat list already uses. This is a
  deliberate difference: upstream orders by `dateSent` descending, and its
  `dateSent` is the parsed `Date` header. Receive time is fixed at ingest
  (0005) and does not depend on the sender's clock, so the mail a suite
  just triggered is the newest one even when a sender's clock is skewed.
- `DELETE /mail {"pruneCode":"all"}` clears **every inbox** through the
  store's `clear_all`.
- **Enabling the listener collapses per-inbox isolation for its clients.**
  A MailSlurper client has no inbox selector. It sees every session's
  mail, and its cleanup clears every session's mail, on that server. The
  isolation boundary is one Swarmail (process and data file) per suite
  that uses the MailSlurper API. No `?inbox=` extension is offered: a
  scoped read beside a global wipe would be half-isolation, and a client
  would believe it owned an inbox that another client's cleanup empties.
- Per-inbox caps still prune oldest-only (0003). A global page therefore
  merges independently pruned inboxes. A mail can leave between two page
  reads, and `GET /mail/{id}` can answer `404` for a mail that was accepted
  and later evicted by its own inbox's cap. No cross-inbox retention rule
  is added.

### 3. Routes

| Route | Answer |
|---|---|
| `GET /mail` | `200 application/json`, `{"mailItems":[…],"totalPages":n,"totalRecords":n}` |
| `GET /mail/{id}` | `200 application/json`, one item; an unknown id is a `404` |
| `GET /mail/{id}/message` | `200 text/html; charset=utf-8`, the item's `body`; an unknown id is a `404` |
| `GET /mailcount` | `200 application/json`, `{"mailCount":n}` |
| `DELETE /mail` | `{"pruneCode":"all"}`: wipe every inbox, then `200 text/plain; charset=utf-8` with the decimal count removed |

- **Paging.** `pageNumber` only: 1-based, default 1. The fixed page size is
  **50**, upstream's hard-coded `length := 50` (review return 2026-11-02,
  §9). `totalPages` is `ceil(totalRecords / 50)`.
  `totalRecords` counts every stored mail in the same snapshot the page was
  cut from, not the page length. `pageNumber` set to `0`, a negative
  number or a non-integer is a `400` naming `pageNumber`. A page past the
  end is a `200` with the totals and no items. Upstream may answer that
  with `404 Page number out of range` (*recalled*, §7). The `200` is kept,
  not built to a recollection.
- **`mailItems` is omitted whenever the page is empty**, including on an
  empty store, where the answer is exactly
  `{"totalPages":0,"totalRecords":0}`. This is the operator's observation
  of real MailSlurper, and the client normalises it.
- **Closed query.** MailSlurper's search, sort and date-range parameters
  (`message`, `from`, `to`, `start`, `end`, `orderby`, `dir`; *recalled*)
  are refused with a `400` naming the key, as is any other key. A filter is
  never silently ignored: ignoring one would return unrelated mail to a
  link scan. The other routes take no query at all, and any key is a `400`
  naming it.
- **Bounded page work** (0044 §§5–6). The page is cut, projected and
  serialized on the blocking pool into one buffer capped at
  `MAX_COMPAT_PAGE_BYTES`. A page over the cap is refused whole, by name,
  with a `400` that points at `GET /mail/{id}`. It is never truncated.
- **Not served:** `/mail/{id}/messageraw`, attachment download,
  `/pruneoptions`, `/version`, login/logout, CORS. Each is a plain `404`
  and a documented omission. Responses use identity encoding: the
  client's `Accept-Encoding` is a preference, and compressing would need a
  new dependency feature.

### 4. The item projection

One borrowed view of the stored `Email`, serialized in this key order:

| Key | Source |
|---|---|
| `id` | the store id |
| `dateSent` | receive time, `YYYY-MM-DD HH:MM:SS` in UTC (upstream's format; upstream formats the parsed `Date` header in the server's local zone, a documented difference) |
| `fromAddress` | the compat sender (envelope `MAIL FROM`, with the header `From` address only for rows stored before the envelope was kept; `""` when neither exists), as the MailHog and Mailpit shims report it |
| `toAddresses` | the envelope recipients (`RCPT TO`) as bare address strings, in envelope order; the header `To` addresses only when the row has no recorded recipients |
| `ccAddresses` | the header `Cc` addresses as bare strings (an extra key relative to upstream's item as the review read it; documented, harmless to a JSON client) |
| `subject` | the stored subject, or `""` |
| `body` | the stored HTML body if present, else the stored text body, else `""`: decoded at ingest, never re-rendered, unescaped or sanitized |
| `attachments` | always `[]` (the existing shims' documented omission) |

Bare addresses make a `toAddresses.includes(addr)` check work for every
recipient, including a Bcc'd one, whatever display name the headers carry.
Using the HTML body keeps an `href`-only verification link literal for the
client's regex.

**Omitted, and documented:** `xmailer`, `mimeVersion`, `boundary`,
`contentType`, `transferEncoding` and `bccAddresses`. Swarmail does not
store these headers as fields. Emitting them would mean either parsing
MIME at request time (against 0005) or inventing values. A synthesized
`contentType` would disagree with upstream for every multipart mail.

*`subject` is amended by the dated addendum of 2026-11-04 below (issue #326): it is served HTML-escaped with Go's `html.EscapeString` set. The operator accepted that addendum on 2026-11-05.*

### 5. The destructive route

Only a request with the JSON content type, no query string, and a body
that parses as an object whose sole key `pruneCode` is exactly the string
`"all"` (case-sensitive) wipes. Every other request is a `400` that
deletes nothing:

- MailSlurper's age-based codes `60plus`, `30plus` and `2wksplus`
  (*recalled*) are refused **by name**. Swarmail prunes only the oldest
  mail past a per-inbox cap (0003) and adds no time-keyed eviction.
- An unknown code, `"ALL"`, `null`, a non-string, a missing key, an extra
  key, `{}`, an empty or absent body, a non-JSON body, a non-JSON content
  type, or any query key is a `400` with a short reason.
- A failed commit is `503 storage commit failed` (0014), never a count.

A wipe that follows a mail's `250` may remove that mail. That is what
"all" means, and it does not contradict 0001: the mail was stored when the
`250` was sent.

### 6. Known differences

`docs/guide/compat-shims.md` lists them under a MailSlurper heading:
everything §§1–5 omits, refuses or answers differently. That includes the
error bodies, which are swarmail's JSON `{"error":…}` envelope (or axum's
plain-text query rejection) rather than MailSlurper's plain text.

### 7. Operator verification before acceptance

*Revised 2026-11-02 at the review return (§9).*

**Reported from upstream source by the review panel** (the guarantees
member, citing `cmd/mailslurper/controllers/ServiceController.go` and
libmailslurper `controllers/MailController.go` at `17e35b8203ac`; no design
seat could read them):

- the page size is 50 (the shim's earlier 25 was wrong and is corrected);
- the routes `/mail`, `/mail/{id}`, `/mail/{id}/message`, `/mailcount` and
  `DELETE /mail` match;
- the prune codes `all`, `60plus`, `30plus` and `2wksplus` match;
- the `dateSent` format matches, but upstream formats the `Date` header in
  local time and orders by it (§§2, 4: deliberate differences);
- `toAddresses` from `RCPT TO` matches;
- the count envelope matches;
- an unknown id is a `500` upstream and a `404` here (documented);
- `ccAddresses` is not in upstream's item (kept, documented);
- upstream's empty list renders `mailItems` as `[]`. That conflicts with the
  operator's observation that real MailSlurper omits the key. The
  operator's observation is binding here, and the client normalises both.

**Still recalled:** the `pageNumber` spelling, the `/mail/{id}/message`
content type, the search-key names, the plain-text DELETE answer, and the
past-the-end `404`.

**Acceptance-blocking.** A mismatch on what the consumer reads blocks
acceptance. That covers `mailItems` and what decides its contents (the
page size and the omission of an empty page), plus `totalRecords`,
`subject`, `body` and `toAddresses`. The page size was in the non-blocking
set before the review return. That was the drafting error the review
found: it decides which mail is on page 1, the only page the consumer
reads. The receive-time order also decides membership once the store
holds more than 50 mails. It is a declared difference, and the operator
accepts or refuses it explicitly with this record. Any other mismatch is
a dated addendum and a follow-up change.

### 8. Implementation notes (2026-11-02, the implement seat)

Three details the build settled, all inside the rulings above:

- **The wipe body is checked by hand.** `DELETE /mail` reads the body as a
  plain JSON value and requires an object with the sole key `pruneCode`.
  A derived struct would also accept a positional array, and `["all"]` wiped
  in the first wire test; the same hazard 0070 closed for seed and chaos.
  Arrays, scalars, `{}` and extra keys share one `400`:
  `deletion requires the JSON body {"pruneCode":"all"}`.
- **`/mail/{id}/message` is hardened for a browser.** The body is the
  sender's HTML, served as `text/html` from the listener that also hosts
  the wipe. The answer carries `Content-Security-Policy: sandbox` and
  `X-Content-Type-Options: nosniff`, so a page opened from it runs no
  script and holds no origin. The body bytes are unchanged.
- **The startup line names the listener.** `swarmail serve` logs
  `mailslurper=Some(addr)` (or `None`) beside the other bound addresses.

### 9. Review return (2026-11-02)

Review of head `439eba5` (run `issue-314-a-mailslurper-compatib-0acb1d84`)
returned the design as defective on one point: the page size. This
record was still proposed, so it is revised in place, and this section
records what changed.

- §3: the page size goes from 25 to 50, with upstream's source cited (§7).
  A `pageNumber`-less `GET /mail` now returns the same 50 items upstream
  would, up to the deliberate order difference of §2.
- §7: the source-checked items are separated from the recalled ones, and
  the page size moves into the acceptance-blocking set.
- **Amends:** accepted 0004's "only cross-inbox destructive endpoint"
  consequence is now named and narrowed, beside 0011's sentence.
- §§2–4: the order, `dateSent` source, past-the-end and `ccAddresses`
  differences are stated, and the guide lists each of them.
- Nothing else in the ruling changes: the listener, the guard, the global
  view, the projection and the destructive contract stand.

**Merge hold.** The PR that carries this shim contradicts accepted 0004's
and 0011's sentences until the operator accepts this record (0045 §8). The
mechanical hold list of 0056's 2026-11-01 addendum does not catch that
case, so the PR body must state the hold in its first line. The operator's
assistant must not merge it on green.

## Alternatives rejected

- **Routes at the root of the HTTP port** (the simplicity position). This
  needs no config and works with the chart as it is. Rejected for the
  reasons in §1. Its strongest point, that a cluster can reach the routes
  without a chart change, is paid for with a deployer-written Service
  (§1).
- **An opt-in path prefix on the HTTP port.** A second spelling of the
  same router, with more vocabulary and branches, for something an ingress
  rewrite already does.
- **A default inbox instead of a global view.** This would avoid amending
  0011, but MailSlurper clients cannot name an inbox: a suite that sends
  through AUTH or routed recipients would find nothing.
- **Age-based pruning.** A second, time-keyed eviction policy beside 0003,
  with wall-clock fixtures that 0020 §5 rules out.
- **`dateSent` in RFC 3339.** Robust to parse, but not MailSlurper's
  format. The consumer does not parse it, and fidelity is the point of a
  shim.
- **Compression.** A `tower-http` feature is a dependency change, and
  identity is a valid answer.
- **Chart support in this change.** It would touch `scripts/chart-verify.sh`
  (a held path) and the chart-bump check, for a port the deployer can
  publish today with one Service. It becomes a follow-up issue.

## Consequences

- With the listener unset, the server is byte-identical to today on every
  existing port.
- The PR is held for the operator until this decision is accepted: the
  wipe route contradicts the accepted sentences of 0011 and 0004 until
  then (accepted 0045 §8; §9 says how the hold reaches the merger). The read routes alone would be compatible, but the issue's
  acceptance criterion requires the delete.
- A deployment that enables the listener is a global-mailbox Swarmail for
  its MailSlurper clients, and the guide says so.

## Dated acceptance note — 2026-11-02

The operator accepted this record on 2026-11-02 ("Accept 0116, 0117,
0118"). It amends accepted [0011](0011-shim-delete-contract.md), only its
sentence that `DELETE /api/v1/delete-all` remains the only cross-inbox
wipe, and accepted [0004](0004-per-test-inboxes-via-smtp-auth.md), only
its consequence that `clear_all` is the only cross-inbox destructive
endpoint, as the Amends line states; both carry a dated marker of this
date. The code merged as `58ad25b` (#320) on 2026-11-02 while this record
still read proposed; §9's merge hold waited on this acceptance, which
legitimises that merge. The ruling as given does not itemise §7's
verification; this note records the acceptance, not the outcome of those
checks.

## Dated addendum — 2026-11-04: the MailSlurper `subject` is HTML-escaped

This addendum is accepted (operator, 2026-11-05; see the acceptance note
at its end). It was proposed when written, and the code that implements
it was held under accepted 0045 §8 until that acceptance. The text above
is unchanged except for the one pointer line at the end of §4.

**Evidence.** Issue #326. The first full Alkemio trial against Swarmail
v0.3.0 (alkem-io/test-suites run 41808342506, `smtp-mode=plain`) ran 160
of 162 test files green. All five failures compare `subject` exactly with
a string that contains `&#34;` where the mail has `"`
(`form-response.it-spec.ts` 219, 299, 319, 353;
`user-invitation-outcomes.it-spec.ts` 441). The same tests pass against
MailSlurper (run 41744902684), so MailSlurper's service API serves the
subject HTML-escaped.

**Amended ruling for §4's `subject`.** The MailSlurper item's `subject` is
the stored subject, or `""`, with exactly these substitutions, which are
the set of Go's `html.EscapeString`: `&` → `&amp;`, `<` → `&lt;`, `>` →
`&gt;`, `"` → `&#34;`, `'` → `&#39;`. The substitution is a single pass,
so each stored character is replaced at most once (`&` first, nothing
double-escaped by a later step). It is unconditional, as Go's is: text
that already reads as an entity is escaped again (`&amp;` →
`&amp;amp;`). It applies on both routes that serve a `MailItem`,
`GET /mail` and `GET /mail/{id}`.

**Observed and inferred.** Only `"` → `&#34;` is observed. The other four
substitutions are inferred from MailSlurper being Go and `&#34;` being
`html.EscapeString`'s spelling. No seat has read upstream (§7's gap
stands). The operator's re-run of the Alkemio trial is the acceptance
evidence. If upstream turns out to differ, the correction is a further
dated addendum.

**Unchanged.** `body` stays as §4 has it: decoded at ingest, never
re-rendered, unescaped or sanitized. Every body-reading test in the trial
passed. `fromAddress`, `toAddresses`, `ccAddresses` and
`GET /mail/{id}/message` are unchanged. The native API, the Mailpit and
MailHog shims, search and filter matching, ingest and storage keep the
sender's subject. Escaping is a view computed when the item is
serialized. It parses no MIME and derives nothing from the raw source, so
it is not extraction, and 0005 is preserved. 0001–0003 are not touched.

**Bounds.** The escape is written into the page's capped buffer as it is
produced, and no copy of the subject is built ahead of the cap. A list
page therefore stays bounded by `MAX_COMPAT_PAGE_BYTES` (0044 §6, §3
above). Escaped bytes count toward that cap: a subject made of escapable
characters reaches 64 MiB from about 12.8 MiB stored. That page is refused
whole and by name, as before. `GET /mail/{id}` has no response cap, as
before; its subject can grow up to five times the stored bytes. This
addendum adds no by-id cap. That is the accepted ruling, not a gap
this addendum leaves: 0044 §6 says "One item may exceed [the page budget];
raw-by-ID remains available", so a by-id cap would contradict an accepted
ruling and could not land under 0045 §8 without the operator's amendment.

*Measured, not new.* JSON already writes a control character as a
six-byte `\u00XX` escape. On this branch's tree a mail whose subject and
body were each 100,000 `U+0001` characters came back from `GET /mail/{id}`
at 1,200,191 bytes and from `GET /mail` at 1,200,239 bytes: six bytes per
character, before this change and unchanged by it. So the by-id response
of a 50 MiB message could already reach about 300 MiB, and a page was
already refused from about 10.7 MiB of stored control characters. The
escape's by-id worst case (five times, about 250 MiB) is below that
300 MiB. Its page refusal (from about 12.8 MiB of stored escapable
characters) comes later in stored bytes than the 10.7 MiB control-character
refusal, and the page output stays capped at 64 MiB either way. The change
adds no new ceiling. A by-id cap, or
a lower page cap, would be a contract change of its own and is the
operator's to rule on, not part of this addendum.

**Records.** Specification:
[MailSlurper subject escape](../../specs/mailslurper-subject-escape/spec.md),
with a dated pointer in the [shim spec](../../specs/mailslurper-shim/spec.md).
No new decision number is taken. The index row keeps this record's status
and names this addendum in its notes.

### Dated acceptance note — 2026-11-05

The operator accepted this addendum on 2026-11-05, in chat to the
dispatcher, choosing "Accept and merge" for the 2026-11-04 note on
0118 §4. The choice was put to them with the scope stated: only `"` →
`&#34;` is observed, and the other four substitutions are inferred. In
the same exchange the operator accepted the low security residual of
the #326 review under accepted 0102 ("Accept the risk"). That residual
is the uncapped by-id growth and the earlier page refusal described
under **Bounds** above. This acceptance lifts the 0045 §8 hold on #326.
If a re-run of the Alkemio trial shows that MailSlurper's escaping
differs from this addendum (for example, bluemonday-style sanitizing
rather than `html.EscapeString`), that is recorded in a further dated
addendum, not by editing this one.
