# Deliverability reports

[← README](../../README.md) · [Guide index](README.md)

GET /api/v1/messages/{id}/deliverability takes only the optional query key `reachability` (see Opt-in reachability below) and returns
{rule_set_id, findings:[{rule_id,severity,location,fix_hint}]}. Severity is
info < warning < error. Locations are tagged header {name,occurrence:null|u64},
mime_part {path:[u64]} or link {part_path:[u64],byte_offset:u64}; link locators
are transfer-decoded source-byte coordinates, not URLs. Reports are sorted by
rule, typed location, severity and hint, with exact duplicates removed.
The bulk-sender checklist adds seven source-citing rules (first versioned swarmail-bulk/1, now part of swarmail-html/4):
one_click, message_id and date are errors; plain_text is warning-level presence
advice. DMARC policy presence, From alignment and passing aligned DKIM-or-SPF
are independent checks. Their error-level *.not_evaluated.v1 findings explicitly
say "Not evaluated:" until trusted observations exist. Sender-written
Authentication-Results and DKIM-Signature never supply evidence. #255 owns that
verification and strict/relaxed alignment; #253/#255 own expiry and refresh.
Production therefore excludes every message at every report predicate threshold;
exact zero assertions count qualifying messages when every required analysis succeeds;
an analysis failure aborts the assertion. Waits still require a qualifying message.

Headers use original bytes and complete singleton values, with root lowercase
names and occurrence null/0/1 for absent/invalid/duplicate. Message-ID and Date
support current RFC 5322 syntax and surrounding comments/folding; obsolete-only
forms are unsupported by this checklist, not universally invalid Internet mail.
Date validates calendar and weekday, numeric zones and optional seconds (including
60), without a clock/freshness check. Message-ID checks syntax, not uniqueness.
One-click requires a complete URI list containing HTTPS and the exact case-sensitive
List-Unsubscribe-Post: List-Unsubscribe=One-Click value. This structural check
proves neither signed-header DKIM coverage nor endpoint operation; no URI is fetched.
Text advice checks genuine outer sender text/plain, including empty/default/encoded
bodies and nested inline multiparts, excluding attachments/ancestry and embedded
message bodies (including multipart/digest defaults). It proves neither content
equivalence nor an unconditional provider mandate. HTML is never converted here.
Hints cite https://support.google.com/mail/answer/81126 and
https://senders.yahooinc.com/best-practices/ plus applicable RFC 8058, 5322, 2046
or 7489 sections. These are fixed published references, not live verification or
provider certification. The proposed rule record is decision 0089.

`swarmail-html/4` retains the synthetic demo.marker.v1: the first root
X-Swarmail-Demo field, unfolded and space/tab trimmed, selects info|warning|error
and emits "Remove the X-Swarmail-Demo header." The seven HTML rules are
html.css.position.v1, html.css.display-flex.v1, html.element.video.v1,
html.img.alt-missing.v1, html.image-only.v1, html.gmail-clipping.v1 and
html.tracking-pixel.v1. They warn on literal position CSS, display:flex/inline-flex,
video, absent alt (empty alt passes), image-only HTML and clipping risk; a remote
HTTP(S) img with explicit integer width/height each 0 or 1 gets an info disclosure.
Each affected HTML MIME part contributes at most one finding per rule, at its
structural child-index path (root []). Plain-text alternatives do not cure HTML.
Attachment/filename-bearing subtrees and attached messages are excluded.

Clipping means strictly more than 104448 decoded UTF-8 HTML bytes per part
(102 KiB), after transfer/charset decoding and before entity decoding; this is
an approximate Gmail risk heuristic, not its live boundary or an SMTP limit.
The applicability matrix covers Gmail web/mobile, classic Outlook for Windows
(Word renderer), and Apple Mail macOS/iOS. CSS rows target classic Outlook;
video targets Gmail and classic Outlook. There is no Apple-specific unsupported
feature row. New Outlook is different; unlisted features are unassessed.

The embedded, versioned catalogue src/deliverability/html-rules.json names
Can I email (CSS position/display:flex and HTML video), WHATWG alternative-text,
Mailchimp Outlook/Gmail-clipping guidance, and MDN Tracking pixel. Its summaries
are Swarmail-authored heuristics dated 2026-10-26, not an imported dataset or
fresh client-version measurements; current empirical measurements were not
supplied. Source URLs, client scopes, selectors, thresholds and fixed hints are
tracked there. No runtime network access or new dependency is needed.

This finite lexical model respects source quotes, comments, CSS strings/functions
and nested media blocks. Hidden conditional comments and template content are
inert; head style blocks are checked. CSS escaped identifiers, cascade, layout,
CSS visibility and CSS-only pixel sizing are unassessed. Hidden text can defeat
image-only detection, and decorative tiny remote images can trigger disclosure.
A lone pixel can receive both image-only and tracking findings. Empty findings
do not certify client compatibility. Empty findings do not imply deliverability
or authentication success.
CSS declaration values retain balanced braces, parentheses and brackets as
opaque components; unused custom-property text cannot create declarations.
Literal unquoted url(...) keeps slash-star as URL data, with bounded escape/
whitespace validation. Malformed supported syntax still fails, including an
unclosed head or recognized tag and stray CSS closers; there is no browser repair.
Exactly zero stored raw bytes contribute no HTML findings before MIME parsing.
The historical demo/HTML composition returns {"rule_set_id":"swarmail-html/2",
"findings":[]} for that message; other collectors and report limits still run.
This historical result makes no authentication claim. In the current combined
report, SMTP accepts and stores an exactly empty message, but its missing MIME
metadata makes analysis fail: REST returns 503 and MCP returns isError.
Assertions and waits needing that report also fail rather than count it clean.
Nonempty parse failures still fail.
Analysis is lazy from stored bytes, off SMTP; extraction stays at ingest.
The existing message page renders the same report without polling.

swarmail_get_deliverability_report requires string id and admits no other keys.
Under MCP 2025-03-26, missing-version requests included, it returns report
JSON in one text block, isError=false. On 2025-06-18 and 2025-11-25 the same
report is also structuredContent under a closed outputSchema matching the REST
DeliverabilityReport: severity enum, tagged header/mime_part/link locations, u64
coordinates, at most 256 findings (the 65536-byte bound, order and deduplication
stay producer rules).
Missing mail and failed/unavailable analysis use isError=true without structured
content; malformed arguments use -32602. The March descriptor keeps main's
"with demo and bulk-sender checks" wording as its historical text, with its
(2025-03-26) suffix; modern descriptors state the full current scope and the structured output (proposed 0105).

REST assert/await and MCP wait_for_email admit optional no_findings_at_or_above
(info|warning|error). It qualifies ordinary-filter candidates before paging;
failed reports abort, never count as clean. Exact assert counts its snapshot,
including zero, and never subscribes. Example: assert?count=0&no_findings_at_or_above=warning
holds when zero candidates qualify; it is not a universal inbox health assertion.
Wait still requires at least one, even count=0, and keeps ordinary totals/order.
Zero timeout uses completed cache entries only: cold/busy required reports return
unavailable. An incomplete scan at deadline is unavailable; a complete insufficient
scan keeps the usual timeout with its exact count. Shutdown keeps its existing error.
The option is refused on list/count; absent means existing behavior.

Cache: 128 Weak-identity slots per Store, exact immutable inputs and rule version,
four admitted blocking jobs retained through cancellation. Up to 256 findings and
65536 compact JSON bytes per cached report. Inclusive metadata limits are
8388608 raw bytes, 65536 per header block (including separator), 262144 aggregate
header bytes, 256 outer entities including root, and depth 32 (root zero).
Complete bounded MIME inspection includes closing boundaries and attached
containers; missing separators, ambiguous/unsupported metadata, encoded multipart,
truncation or exhausted budgets fail without partial success. Embedded messages
stay opaque. No body transfer decoding, SMTP admission change or stored-field
rewrite follows. HTML analysis refuses raw inputs over 8 MiB before MIME parsing,
aggregate decoded HTML over 4 MiB, more than 256 outer MIME parts, depth above 32
(root 0), or lexical nesting above 32. Detected eligible transfer-decode errors,
MIME parse failure and incomplete supported lexical constructs fail the whole
report, without a partial count or success cache entry. Charset replacement
follows pinned mail-parser behavior. These workload limits do not bound prior
parser allocations or total RSS. The lexical inspection proof counts helper reads and length-bounded operations, separately from cursor progress; it excludes MIME-parser internals, allocation, hash collisions and latency. Its fixed linear bound is tested with adversarial inputs and a real rescanning negative control. REST: 404 missing, 503 "deliverability
analysis unavailable" (capacity
or deadline) / "deliverability analysis failed" (analysis/size), native JSON error.
Selected pre-removal reads may finish; fresh lookups cannot resurrect removed mail.
Cache loss at restart costs recomputation. These are cache/job bounds, not total
memory or latency promises. #253 owns DNS observation/policy/expiry/wakeup semantics;
no DNS/authentication verification, external spam scorer, external service or new dependency
ships here. The cache key includes every effective authentication fact and the
immutable policy/observation identity, alongside selected Weak mail identity.

## Link rules

Proposed decision 0121 (issue #258) adds five per-link rules to the default
report, in rule set `swarmail-html/4`. They read the same eligible outer
`text/html` parts as the HTML heuristics above (attachments and attached
messages excluded, same budgets) and look at every visible `<a>` start tag
that has an `href`, in source order. "Visible" and the link label follow the
anchor-text rules of the stored `anchors` (#154): the label is the decoded
text up to the next `<a>` or `</a>`, ASCII whitespace collapsed, block
boundaries as one space. Anchors inside `<head>`, `<template>`, script, style,
title and conditional comments do not exist for these rules. `<img src>`,
`<area>`, CSS `url()` and URLs in text/plain parts are not checked.

The `href` is read once-decoded for character references and ASCII-trimmed,
then parsed as a WHATWG URL (tabs and newlines inside it are dropped, scheme
and host are lowercased).

| rule id | severity | fires when |
|---|---|---|
| `link.scheme.unsafe.v1` | error | scheme is `javascript`, `vbscript`, `data` or `file` |
| `link.url.invalid.v1` | warning | the value is empty or not an absolute URL: a relative path, `//host`, an unrendered `{{placeholder}}`, an invalid host. A value that starts with `#` never fires |
| `link.scheme.http.v1` | warning | scheme `http`, unless the host is `localhost`, `*.localhost`, in `127.0.0.0/8` or `::1` |
| `link.shortener.v1` | warning | an `http`/`https` host equals an entry of the built-in shortener list |
| `link.anchor-host-mismatch.v1` | warning | the label names at least one host-like token, and the `http`/`https` target host matches none of them |

Other schemes (`https`, `mailto`, `tel`, `cid`, `sms`, `ftp`, app schemes)
produce no scheme finding. Empty link findings mean "no findings from this
rule set", never "the links are safe".

**Plain `http` is a warning, never an error.** Swarmail does not rewrite,
upgrade or fetch the link. Loopback and `localhost` targets are exempt
because a development mail catcher mostly receives `http://localhost:3000/...`
links. There is no allowlist and no knob.

**The shortener list is built in and versioned**, `swarmail-shorteners/1`
(basis date 2026-11-03; Swarmail-authored, not fetched, no network lookup):
`bit.ly`, `bl.ink`, `buff.ly`, `cutt.ly`, `goo.gl`, `is.gd`, `lnkd.in`,
`ow.ly`, `rb.gy`, `rebrand.ly`, `s.id`, `shorturl.at`, `t.co`, `t.ly`,
`tiny.cc`, `tinyurl.com`, `v.gd`. Matching is exact on the host (one trailing
dot removed): `notbit.ly`, `bit.ly.evil.test` and `www.bit.ly` do not match. A
test pins the list's digest to the list version and the rule-set id, so an
edit moves both. A shortener's redirect target is not known without a network
request; see the opt-in reachability check for that.

**Host-like tokens** in the label are found by splitting on ASCII whitespace.
A token is skipped when it contains `@` or any non-ASCII character. Then, in
this order: it is lowercased; leading `(` `[` `<` `"` `'` are trimmed; a
leading `http://` or `https://` is removed; it is cut at the first `/`, `?`,
`#` or `:`; trailing `.` `,` `;` `:` `!` `?` `)` `]` `>` `"` `'` are trimmed;
and one leading `www.` is removed. It counts when what is left has at least
two dot-separated labels, each `[a-z0-9-]`, and an alphabetic last label of two
or more characters. So `paypal.com`, `(https://paypal.com)`,
`<https://paypal.com/>` and `https://paypal.com/login` all give `paypal.com`,
while `Version 1.2`, `3.14`, `e.g.` and `www.com` (which is `com` after the
`www.` is removed) do not count. The target host, after one trailing dot and
one leading `www.` are removed, matches a token when they are equal or the host
is a subdomain of the token.
`example.com` -> `click.example.com` passes. `paypal.com` ->
`paypal.com.evil.test` fires, and so does a label naming a host with an IP
literal target. Known gaps: there is no public-suffix list, so a label that
names a shared suffix (`github.io`) lets any subdomain pass; labels such as
`Node.js` or `README.md` look like hosts and can warn; internationalized
(non-ASCII) names are skipped, not compared.

**Locations.** A link finding is located at `link {part_path, byte_offset}`,
where `byte_offset` is the first non-whitespace source byte of the `href`
value in the part's transfer-decoded payload (before charset and entity
decoding). That is only a true source offset when the part's decoded text is a
prefix of its transfer-decoded payload and the rest of the payload is ASCII
whitespace (a trailing newline does not move an offset). When it is not (for
example `charset=iso-8859-1` with an 8-bit byte), or when the transfer decode
fails, the finding is located at `mime_part {path}`, the contract's own
fallback. Swarmail never emits a guessed offset.

A hostile message with more than 256 findings fails the report (503), by the
existing limits, rather than returning a comfortable subset.

## Opt-in reachability

Whether a link answers, and where its redirect chain ends, can only be known
by asking the network. That never happens by default, never on the SMTP path,
and never inside the cached report. Two explicit steps are needed:

1. The operator starts the server with `--link-reachability off|public|loopback`
   (env `SWARMAIL_LINK_REACHABILITY`, default `off`).
2. A caller asks: `GET /api/v1/messages/{id}/deliverability?reachability=true`.

```bash
swarmail serve --link-reachability loopback
curl 'http://127.0.0.1:8025/api/v1/messages/<id>/deliverability?reachability=true'
```

On an `off` server the request is `400 {"error":"link reachability is not
enabled"}` and opens no connection; a missing message is still `404`.
`reachability=false` or no parameter is the default report. Any other value or
key is the usual `400`. MCP, the message page, `assert`, `await` and the MCP
wait never probe.

**Modes.** The address policy is **default-deny**: an address is contacted
only when the policy proves it globally routable unicast, never because it is
missing from a list of bad ranges. The basis is the IANA IPv4 and IPv6
Special-Purpose Address Registries as read on **2026-11-04**; Swarmail pins its
own table and does not use the toolchain's `is_global`. `public` contacts only
such addresses. `loopback` also contacts `127.0.0.0/8` and `::1`, so that a
development mail catcher can check `localhost` links; the risk is that **mail
content can then cause GET requests to local ports** (headers only, no body
read, no credentials). Choose it deliberately.

- **IPv6** is contacted only inside `2000::/3`, and then not inside `2001::/23`
  (IETF protocol assignments, including Teredo `2001::/32`), `2001:db8::/32`
  (documentation), `2002::/16` (6to4) or `3fff::/20` (documentation). Every
  address outside `2000::/3` is refused without being named: `::`,
  IPv4-compatible and IPv4-translated forms, NAT64 (`64:ff9b::/96`,
  `64:ff9b:1::/48`), `100::/64`, `fc00::/7`, `fe80::/10`, the site-local
  `fec0::/10`, `ff00::/8` and every unassigned block.
- **IPv4** is contacted only outside the registry's non-global blocks: `0/8`,
  `10/8`, `100.64/10` (CGNAT), `169.254/16` (link-local, which includes the
  cloud metadata address), `172.16/12`, `192.0.0/24`, `192.0.2/24`,
  `192.88.99/24` (deprecated 6to4 relay), `192.168/16`, `198.18/15`,
  `198.51.100/24`, `203.0.113/24`, and `224/4` and `240/4` (multicast,
  reserved, broadcast).
- **IPv4-mapped IPv6** (`::ffff:a.b.c.d`) is unwrapped and judged as IPv4. Every
  other address that embeds IPv4 (NAT64, 6to4, Teredo, IPv4-compatible,
  IPv4-translated) is **refused and never decoded**. A host reachable only that
  way is reported as `refused` (not checked), never as safe.
- A URL with credentials (`user:pass@host`) is refused.
- **Residual:** if IANA allocates a special-purpose block *inside* `2000::/3`
  after the basis date, as it did with `3fff::/20` in 2024, that block is
  contacted until the table is edited. An allocation outside `2000::/3` is
  refused with no change.

**How a target is checked.** Swarmail resolves the host itself and refuses the
target unless **every** answer is permitted, then connects only to the first
validated address, so a name cannot change its address between the check and the
connect. The same applies to every redirect hop. It sends `GET` with
`User-Agent: swarmail-link-check`, no proxy, no cookies, no credentials, reads
the status and headers only, and drops the response without reading a body.

**Bounds** (constants, not knobs):

| bound | value |
|---|---|
| per-link budget, all hops together | 5 s |
| connect timeout per hop | 2 s |
| redirects followed | 5 (a sixth redirect answer is `redirect_cap`) |
| distinct targets per request | 32 (33 is `503 deliverability analysis failed` before any connection) |
| targets probed at once per request | 4 |
| probe requests at once per `Service` (one per server process) | 2 (a third is `503 deliverability analysis unavailable`) |

Worst case per request is ceil(32/4) x 5 s = 40 s with 8 open sockets across the
two requests (a probe dropped at its budget closes its socket a moment after
the next target has connected, so a fifth socket can briefly exist). A target's fragment is never sent, so links that differ only in
it are one target, fetched once. A probe never holds one of the four report job permits or a Store
guard, so SMTP, reads and cold default reports proceed while it waits. Dropping
the request cancels its probes and releases its permit.

**Resolver residual.** Names are resolved with the operating system's resolver
on tokio's blocking pool, and that call cannot be cancelled. When a target's
5 s budget drops a probe, its lookup keeps running until the OS resolver gives
up, so a hostile hostname plus repeated explicit requests can park blocking
threads. The opt-in mode, the per-`Service` caps and the OS resolver's own
deadline bound this. It costs latency only, never a `250`. A dedicated bounded
resolver would need its own decision.

| finding | severity | when |
|---|---|---|
| `link.redirect-chain.v1` | info | the link answered after one or more redirects; the hint names the count and the final origin (scheme, host, port), never a path, query or credentials |
| `link.reachability.v1` | warning | timeout; connection, DNS or TLS failure; HTTP status 400 or above (with the code); more than 5 redirects; a missing, invalid or non-HTTP redirect target |
| `link.reachability.refused.v1` | info | not checked: credentials in the URL, or an address the mode never contacts |

Each is located at every `link` location of its target. A `200` answer adds no
finding. The probed response is the cached deterministic report plus these
findings, canonicalized under the same 256-finding and 64 KiB limits (over the
limit is a failure, never a truncated report).

**Determinism.** The default report is byte-identical for the same bytes and
configuration, as before. The probed response is a pure function of the bytes,
the configuration and the network's answers at that moment, and it is never
cached or stored: two probes at different times can differ. Empty findings
mean "nothing found", not "the links are safe".

## Spam signals (`swarmail-spam/1`)

Six built-in, offline, deterministic rules read only the captured message: the
root Subject, the root From display name and the sender's `text/plain` bodies
(decision 0122, proposed). There is no external scorer, network, clock, locale or
configuration; the same stored bytes always give the same findings. Each rule
emits at most one finding per report, severity info or warning (error stays the
bulk checklist's requirement level), so a large message can never flood the
report. They arrived as `swarmail-spam/1`; with the link rules of proposed 0121
the report's `rule_set_id` is now `swarmail-html/4`, so the rule-set version
is recorded in every report and in the cache key. Reports carry no weight and no
total: the weights below are the proposal in decision 0122 and wait for the
operator to accept it; they are labelled "proposed in 0122 — not in the report"
because they are not emitted.

Subject means the first root Subject field in wire order, RFC 2047-decoded and
unfolded; From means the first mailbox of the first root From field, display name
decoded. A later duplicate field never substitutes for the first. Letters are
ASCII A–Z/a–z only and case comparison is ASCII-only: no Unicode case folding,
so a non-ASCII letter neither counts as a capital nor as lowercase. Body rules
read eligible text parts: a non-multipart part reached through multipart
containers only, whose type is `text/plain` or absent (RFC 2045 default; a
`multipart/digest` child defaults to `message/rfc822` and is not scanned), that is
not an attachment, does not carry a filename or name parameter and does not sit
under such a part. Embedded `message/*` bodies stay opaque and `text/html` is
never scanned (HTML is #259's surface; HTML-only mail already receives
`bulk.plain_text.v1`). The text is the transfer- and charset-decoded contents. A
body finding is located at the `mime_part` path of the first matching eligible
part in wire order (root `[]`, children by index).

### `spam.subject.all_caps.v1`

- **Matches:** a Subject with at least 8 ASCII letters, none lowercase.
- **Severity:** warning. **Weight:** 3 (proposed in 0122 — not in the report).
- **Location:** `header {name:"subject", occurrence:0}`.
- **Why:** an all-capital subject reads as shouting and is a classic spam signal;
  the 8-letter floor keeps short acronyms such as `RSVP` or `OTP: 123456` clear.
- **Fix hint:** `Write the Subject in sentence case; an all-capital subject reads as shouting and is a common spam signal.`
- **Passing:** `Subject: URGENT` (6 letters) and `Subject: WINTER Sale` (a lowercase letter).
- **Failing:** `Subject: WINTER SALE`.

### `spam.subject.punctuation.v1`

- **Matches:** a Subject containing `!!`, `??` or `$$`.
- **Severity:** info. **Weight:** 1 (proposed in 0122 — not in the report).
- **Location:** `header {name:"subject", occurrence:0}`.
- **Why:** repeated emphasis punctuation is a weak but common sales-spam marker; a
  single `!`, `?` or `$` is ordinary.
- **Fix hint:** `Remove repeated !, ? or $ characters from the Subject.`
- **Passing:** `Subject: Your receipt!`
- **Failing:** `Subject: Your receipt!!`

### `spam.subject.phrases.v1`

- **Matches:** a Subject containing a phrase from the list below.
- **Severity:** warning. **Weight:** 3 (proposed in 0122 — not in the report).
- **Location:** `header {name:"subject", occurrence:0}`.
- **Why:** these are high-pressure sales claims that filters have long weighted.
- **Fix hint:** `Remove high-pressure sales wording from the Subject; see the guide's phrase list.`
- **Passing:** `Subject: Your invoice is ready`
- **Failing:** `Subject: Act now to keep your account`

### `spam.from.display_name_address.v1`

- **Matches:** a From display name containing an address-shaped token
  (`[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+`) that is not, ASCII
  case-insensitively, the mailbox's own address. A group, an empty list or a
  mailbox without an address yields no finding.
- **Severity:** warning. **Weight:** 3 (proposed in 0122 — not in the report).
- **Location:** `header {name:"from", occurrence:0}`.
- **Why:** a different address in the display name is how mail clients are made to
  show a trusted-looking sender; the same address as the real one is harmless.
- **Fix hint:** `Do not put an email address in the From display name unless it is the sending address; a different address there looks like spoofing.`
- **Passing:** `From: "Billing" <billing@example.test>` and `From: "Billing@Example.test" <billing@example.test>`.
- **Failing:** `From: "ceo@bank.example" <billing@example.test>`

### `spam.body.phrases.v1`

- **Matches:** an eligible text part containing a phrase from the list below.
- **Severity:** warning. **Weight:** 3 (proposed in 0122 — not in the report).
- **Location:** `mime_part {path}` of the first matching eligible part.
- **Why:** the same claims as the Subject rule, in the body.
- **Fix hint:** `Remove high-pressure sales wording from the text/plain body; see the guide's phrase list.`
- **Passing:** `Your order has shipped.`
- **Failing:** `Your order has shipped. Act now.`

### `spam.body.zero_width.v1`

- **Matches:** an eligible text part with at least 3 zero-width characters
  (U+200B, U+200C, U+2060, U+FEFF). U+200D (ZWJ) is excluded because emoji
  sequences use it legitimately.
- **Severity:** info. **Weight:** 1 (proposed in 0122 — not in the report).
- **Location:** `mime_part {path}` of the first matching eligible part.
- **Why:** invisible characters inside words are used to hide them from filters; one
  or two are common in pasted text, so the floor is three.
- **Fix hint:** `Remove zero-width characters from the text/plain body; they are used to hide words from filters.`
- **Passing:** a body with two U+200B characters.
- **Failing:** a body with three U+200B characters.

### Phrase list

Fourteen fixed phrases: `act now`, `limited time offer`, `100% free`, `risk-free`,
`risk free`, `you have won`, `you are a winner`, `claim your prize`,
`no credit check`, `double your money`, `make money fast`, `once in a lifetime`,
`this is not spam`, `cash bonus`. A phrase matches ASCII case-insensitively, each
internal space matching one or more of space, tab, CR or LF (so a line-wrapped
phrase matches), with no ASCII letter, digit or `_` immediately before or after it
(`reactnow` and `act nowhere` do not match). Words transactional mail legitimately
uses, such as "urgent", "congratulations" or "verify your account", are
deliberately not listed.

### Spam signal bounds

| input | bound |
|---|---|
| raw message | 8,388,608 bytes, checked before any collector |
| root header block (Subject, From) | 65,536 bytes including the separator |
| MIME parts / depth | 256 including root / 32 (root 0) |
| aggregate decoded eligible text | 4,194,304 bytes, summed over every eligible part before any rule matches |
| spam findings | at most 6, one per rule |

Exceeding the text bound fails the whole report (REST 503 "deliverability
analysis failed", MCP isError), even if an earlier part would have matched: a
truncated scan is refused, never reported as clean. Work is linear: the phrase
matcher is one finite-automaton regex compiled once from the constant list, and
the zero-width count is one pass, so the spam collector inspects at most the
65,536 header bytes plus 4,194,304 text bytes per report, beyond the single
bounded MIME parse. Cost of the bound: a message with more than 4 MiB of decoded
`text/plain`, which earlier rule sets reported on, now fails analysis.

### Signals counted elsewhere

The spam rules do not re-detect what other collectors report. A reader assessing
spam risk should also count `bulk.plain_text.v1` (HTML-only mail),
`bulk.one_click.v1` (no one-click unsubscribe), `bulk.message_id.v1`,
`bulk.date.v1`, `html.image-only.v1` and `html.tracking-pixel.v1`. Decision 0122
proposes that, once weights ship, every finding counts by its severity (info 1,
warning 3, error 10), with `*.not_evaluated.*` findings and `demo.marker.v1` at
zero. Link and URL signals are #258's link rules and DNS policy belongs to #253;
the spam rules read no URL and resolve nothing.

### Known false negatives and positives

False negatives: HTML-only spam wording, phrases outside the fixed list, phrases
split by tags or inserted characters, non-ASCII look-alike letters, a partly
shouted subject with any lowercase letter, and spam with a clean Subject and body
but a bad link or sender (not read here). False positives: a legitimate subject in
capitals of 8 or more letters, a genuine "limited time offer" in a newsletter, and
a newsletter whose display name repeats a different mailbox, or a bare address followed by a comment holding another address (the parser reads the comment as the display name). An empty report does
not mean the message will reach the inbox.

### Rule-set version history

`swarmail-demo/1` (synthetic demo marker), `swarmail-bulk/1` (bulk-sender
checklist), `swarmail-html/2` and `swarmail-html/3` (HTML heuristics), then
`swarmail-spam/1` (these six rules; no existing rule changed meaning), then
`swarmail-html/4` (the five link rules of proposed 0121, integrated after
`swarmail-spam/1`; no existing rule changed meaning). A change to
any rule's meaning, hint, location or order requires a new id.
