# Deliverability reports

[← README](../../README.md) · [Guide index](README.md)

GET /api/v1/messages/{id}/deliverability takes only the optional query key `reachability` (see Opt-in reachability below) and returns
{rule_set_id, findings:[{rule_id,severity,location,fix_hint,weight}], dns, arc:{state,sets,failing_instance,reason}, total}. Severity is
info < warning < error. Locations are tagged header {name,occurrence:null|u64},
mime_part {path:[u64]} or link {part_path:[u64],byte_offset:u64}; link locators
are transfer-decoded source-byte coordinates, not URLs. Reports are sorted by
rule, typed location, severity and hint, with exact duplicates removed.
The bulk-sender checklist adds seven source-citing rules (first versioned swarmail-bulk/1, now part of swarmail-html/9):
one_click, message_id and date are errors; plain_text is warning-level presence
advice. DMARC policy presence, From alignment and passing aligned DKIM-or-SPF
are independent checks. They read six facts that the report evaluator computes
from the stored bytes, the SMTP envelope observed at ingest, DKIM's verdicts and
the declared DNS policy (SPF and DMARC, below). A fact that cannot be determined
is Unknown, and its entry is an error-level *.not_evaluated.v1 finding that
explicitly says "Not evaluated:". Sender-written Authentication-Results and
DKIM-Signature never supply evidence. A message qualifies at a threshold only when
every finding is below it, so a wait or an exact zero assertion matches mail whose
SPF, DKIM and DMARC evidence passes and whose checklist is clean at that
threshold; exact zero assertions count qualifying messages when every required
analysis succeeds; an analysis failure aborts the assertion. Waits still require a
qualifying message.

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
provider certification. The rule record is decision 0089.

`swarmail-html/9` retains the synthetic demo.marker.v1: the first root
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

DNS policy (decision 0120, accepted). Report checks that need DNS answers (DKIM keys, SPF,
DMARC; #254, #255, #256) read a declared, offline TXT map, never the network.
Every report carries a required `dns` member
{source:"default"|"server"|"inbox", policy:{txt:[{name,values}],tempfail:[name],spf_client_ip?},
lookups:[{name,type:"txt",outcome:"records"|"no_records"|"temp_fail",values}]}.
`policy` is the complete canonical effective policy; `lookups` lists each name an
evaluation resolved, once, sorted. DKIM verification (decision 0125, accepted, below) records
every key, ATPS and report name it asks for that lies inside decision 0120's name grammar (an out-of-grammar name fails closed and is not recorded); a message with no `DKIM-Signature`
records none, and SPF and DMARC (decision 0126, below) record the TXT names they ask for
(the record, its `include:` and `redirect=` targets, an `exp=` name, and the `_dmarc` names of the tree walk). `temp_fail` is never a pass and never clean: a report with a
`temp_fail` lookup qualifies at no no_findings_at_or_above threshold, so an assert
does not match it and a wait rides to its deadline. `no_records` is a definite
negative. The same message under the same configuration gives byte-identical
report JSON; no path, time or generation appears (the two stated exceptions are the
DKIM wall-clock residual, `dkim.expired_since_receipt.v1`, whose full radius is
described below, and SPF's 20-second monotonic lookup timer, below).
Declare a server map with `serve --dns-policy FILE` (env SWARMAIL_DNS_POLICY): a
JSON file of at most 8192 bytes, `{"txt":[{"name":"s1._domainkey.example.com",
"values":["v=DKIM1; k=rsa; p=..."]}],"tempfail":["_dmarc.example.org"]}`; `{}` is
an empty map and source "server". The declaration also takes an optional
`spf_client_ip`, an IPv4 or IPv6 address literal (stored canonical, present in
`dns.policy` only when declared) that SPF evaluates instead of the connecting
address the SMTP session saw; see SPF and DMARC below. A bad file stops startup before any listener
binds; there is no fallback to an empty map. Declare an inbox map with
PUT /api/v1/inboxes/{inbox}/dns (same body; 200 echoes the object
{"inbox","source","policy"}; 400 for an invalid declaration or an inbox label over
512 bytes, 413 for a body over 8192 bytes, 503 when 4096 inboxes already hold one)
and clear it with DELETE /api/v1/inboxes/{inbox}/dns (idempotent; it echoes the
fallback, the server map else the default). An inbox map replaces the server map;
there is no merge. A declaration is bounded twice, on its raw bytes and on its
canonical form (what a report embeds), at most 8192 bytes each, so a report's
`dns.policy` can always be declared back verbatim. An input that fits on the wire
but whose canonical form does not, for example one near the bound that omits
`tempfail`, is refused with 400
`canonical declaration is larger than 8192 bytes`. It also holds at most 256 names
and values in all (each txt owner, each value and each tempfail name counts one);
one more is refused with `declaration has more than 256 names and values`. A
declaration is kept in memory under its inbox label, so the label may be at most
512 bytes: `inbox label is longer than 512 bytes`. Names are ASCII labels of
[A-Za-z0-9_-], folded to lowercase with one trailing dot stripped, at most 253
bytes; a name matches only itself (no wildcard, no IDNA). Values are exact
printable ASCII, never trimmed. Duplicates, an empty values list and a name under
both txt and tempfail are refused, never repaired. Inbox declarations are
process-local and are not persisted; each request freezes one policy snapshot,
and a change wakes no waiter. Real DNS is configured but refused:
`--dns-resolver real` (env SWARMAIL_DNS_RESOLVER) exits with "real DNS is not
supported; declare records with --dns-policy or PUT /api/v1/inboxes/{inbox}/dns";
`declared` is the default.

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
(2025-03-26) suffix; modern descriptors state the full current scope and the structured output (decision 0105).

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
#253 shipped no verification: DKIM verification arrives with accepted 0125 on
`mail-auth`, offline against the declared keys; SPF and DMARC arrive with decision
0126 on the same crate (below), and ARC is #256; no external spam scorer or service
ships here. The cache key is the selected Weak mail identity (which carries the
row's immutable SMTP envelope observation), the frozen policy snapshot (including
`spf_client_ip`) and the rule set; every authentication fact is a function of
those, so two requests with equal keys get equal facts.

## For agents

Decision 0131 (issue #261). An agent that sends mail through an application and
wants to know whether the mail is deliverable reads the report as data, fixes the
mail and sends it again. This section is the contract for that loop; the rules
themselves are documented below and are not repeated here.

**The tool.** `swarmail_get_deliverability_report` takes one string argument,
`id`, the id of a stored message: the id in the SMTP reply
`250 2.0.0 OK: stored as {id}`, or the `id` of a mail a wait or search returned.
It is the same report as `GET /api/v1/messages/{id}/deliverability`.

**The output.** On MCP 2025-06-18 and 2025-11-25 the report is
`structuredContent`, under a closed `outputSchema`, and the same JSON is also in
one text block. On 2025-03-26 (and when no version is sent) the report is the
text block alone, with no `outputSchema` and no `structuredContent`. Parse the
text block on any revision; use `structuredContent` when it is there. A missing
message and a failed or unavailable analysis are `isError: true` with no
structured content.

**The loop.**

1. Read the report of the message you sent.
2. Fix each finding from its `rule_id`, its `location` (a header name and
   occurrence, a MIME part path, or a link's source-byte offset in its part) and
   its `fix_hint`.
3. Send a **new** message. A stored message is never edited: extraction and
   analysis read the bytes ingested (decision 0005), so a repair is a new
   delivery with a new id.
4. Read the new message's report, not the old one.

**Judge content by rule family, not by threshold.** Under the default DNS policy
no authentication record is declared, so the `bulk.*`, `spf.*`, `dkim.*` and
`dmarc.*` findings stay on every message however clean its content is, and
`no_findings_at_or_above` cannot say "the content is clean". To judge the mail an
agent controls, filter `findings` by the prefix of `rule_id`: `link.`, `html.`
and `spam.` are the content families. Authentication and the bulk-sender
checklist are properties of DNS and of the sending infrastructure; declare a DNS
policy (above) to exercise them.

**Forwarded mail: read the `arc` member.** Every report carries `arc`, the ARC
chain (RFC 8617) state of the message: `none` (no ARC field), `pass`, `fail` or
`not_evaluated`, with the number of `sets`, the `failing_instance` that broke the
chain and the `reason`, the id of the one `arc.*` finding. A message whose chain
validates and a message with no chain both have no `arc.*` finding, so assert
that `arc.state` is `pass` and do not infer it from empty findings. The `arc.*`
findings are authentication findings like `dkim.*`, not a content family, and
`arc=pass` does not say the sender is authentic. Declare each sealer's key in the
DNS policy to exercise it.

**What a report does not say.**

- A report that could not be read (an `isError` result, a not-found message, a
  503 or "unavailable") is **not** a clean report. Retry it or treat it as
  unknown; never read it as "no findings".
- Empty findings do not imply deliverability. They mean no rule fired.
- The report is bounded: at most 256 findings and 65536 compact JSON bytes. A
  report over either bound is an error, never a truncated list.
- `rule_set_id` changes whenever the report's bytes change. Ignore members you do
  not know, match on `rule_id` and never on a whole report body.

A trimmed excerpt (one finding of three; the other members elided):

```json
{
  "rule_set_id": "…",
  "findings": [
    {
      "rule_id": "link.scheme.http.v1",
      "severity": "warning",
      "location": {"kind": "link", "part_path": [], "byte_offset": 131},
      "fix_hint": "Serve the link over https."
    }
  ],
  "dns": {"source": "default", "…": "…"}
}
```

**Fixtures and the eval.** `tests/fixtures/deliverability-agent/` ships a failing
newsletter and its repaired twin (`newsletter-failing.eml` carries
`link.scheme.http.v1`, `html.img.alt-missing.v1` and `spam.subject.all_caps.v1`;
`newsletter-fixed.eml` carries none of the three families), proven over real SMTP
by rule id. The `newsletter-findings` task in `evals/` measures this loop: an
agent reads the report, edits a newsletter template and re-sends until a
re-sent newsletter's report has no `link.`, `html.` or `spam.` finding.

## Link rules

Decision 0121 (accepted; issue #258) added five per-link rules to the default
report. They arrived in rule set `swarmail-html/4` and are part of the current
rule set (`swarmail-html/9`). They read the same eligible outer
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

## DKIM verification (`swarmail-html/9`)

Decision 0125 (accepted; issue #254) verifies every `DKIM-Signature` field of a
stored message and reports each way a signature fails to verify as a named
finding, so a test can assert "my signer works" by the absence of `dkim.*`
findings and each specific breakage by its rule id. It runs lazily, inside the
report evaluator, on the stored bytes after the SMTP `250`; nothing runs at
ingest or on the accept path. A signature that verifies emits nothing, and so
does a message with no signature: there is no `dkim.pass`, `dkim.missing` or
`dkim.valid` id.

**What is verified.** `rsa-sha256` (keys of 1,024 to 8,192 bits, RFC 8301) and
`ed25519-sha256` (RFC 8463); `simple` and `relaxed` canonicalization for headers
and for the body, in any pair; the body hash and the header hash; `l=`; `x=`.
`rsa-sha1` is unsupported. The cryptography, parsing and canonicalization are
the `mail-auth` crate's (the operator's ruling of 2026-10-26 on epic #251); what
decides a verdict around it is Swarmail's.

**Keys come from the declared DNS map only.** Declare the key like any other
record (see the DNS policy above):

```
curl -X PUT localhost:8025/api/v1/inboxes/qa/dns -d '{"txt":[
  {"name":"sel._domainkey.example.com","values":["v=DKIM1; k=ed25519; p=<base64>"]}]}'
```

Every name inside 0120's grammar that the verifier asks for is answered from that
map and recorded in the report's `dns.lookups`: the key name `<s>._domainkey.<d>`, the RFC 6541 ATPS name
when a verified signature carries `atps=` for a From domain, and the RFC 6651
`_report._domainkey.<d>` name when a failing signature has `r=y` (asked, answered
and recorded like any name, but never typed as a report record: see the clock
residual). A name outside
the policy's grammar is never looked up. The crate compiles in a DNS resolver (it
cannot be built without one) and it is unreachable: every name is answered by the
declared map, and the one authenticator has no name servers. Real DNS is not
supported.

**Time is the receipt time.** A signature is expired when its `x=` is at or
before the second the message was received (`received_ms` floored to seconds,
clamped at zero). The receipt time never changes for a stored message, so a
cached report stays correct. `t=` in the future is not enforced; an `x=` at or
before `t=` is malformed (RFC 6376 section 3.5), and a literal `x=0` with no `t=` is an
expiry in 1970 (`dkim.signature_expired.v1`), not an absent tag.

**The clock residual (its full radius).** One read of the host clock remains,
inside the `mail-auth` verifier, which compares `x=` with it before it checks
anything else. It applies only to a signature with an `x=` that is after the
receipt second and, when `t=` is present, after `t=` too. Once the host clock reaches that `x=`, the crate
stops at the expiry, so *every* verdict the later rules could have given that
signature becomes `dkim.expired_since_receipt.v1`: a pass (no finding), the
`dkim.body_length_partial.v1` warning, and every error from
`dkim.body_hash_mismatch.v1` to `dkim.signature_mismatch.v1` alike. The key and
ATPS lookups for that signature also disappear from `dns.lookups`. So a clean or
warning-only report can begin to disqualify, and a report recomputed after cache
eviction can differ from the cached one: the first of the two stated exceptions to
byte-identical reports (the second is SPF's lookup timer, below). The move is one-way unless the host clock is set back.
The crate's RFC 6651 report sampling also reads the clock, but Swarmail never
hands it a report record, so that read is never reached. A test that awaits a
clean report on a signature whose `x=` is near the host clock can start to fail
without any new mail: keep a fixture's `x=` far from the wall clock, or derive it
from the receipt.
Malformed, unsupported, `from_unsigned` and `signature_expired` verdicts, every
message-level finding and every unsigned message are clock-free. The operator
accepted this read on 2026-11-06 as an amendment of decision 0085 section 8
(0125 section 8).

**`l=`.** The crate counts `l=` in raw body bytes, in its signer and in its
verifier, and hashes the first `l=` raw bytes (all of them when it is longer than
the body). A literal `l=0` cannot be told from no `l=` and is unsupported; an `l=`
that is not a number is malformed. A signature that verifies with an `l=` equal to
the raw body length is clean. One that verifies with an `l=` that leaves canonical
content out is a warning, because text can be appended after it unsigned (RFC 6376
section 8.2); an `l=` longer than both the raw and the canonical body is an error.
RFC 6376 section 3.4.5 counts canonicalized octets where the crate counts raw
ones: see the interop gap under known limits.

**One body, one field list.** DKIM must hash the body every other collector and
the message page see, and `occurrence k` must be the same field in both. Before
anything is verified, two things must hold: the verifier's header/body boundary
equals Swarmail's, and the verifier lists as many `DKIM-Signature` fields as
Swarmail's root header list does. If either fails, the message gets one
`dkim.header_boundary_ambiguous.v1` and nothing else: no verification, no lookup,
no other `dkim.*` finding. The verifier ends the headers at a line without a
colon (and at a whitespace-only first line) where Swarmail skips the line and
goes on, so the two would hash different bodies; and it reads a name with a space
before the colon (`DKIM-Signature :`) as a signature where Swarmail's scanner
names no field, so the two would count different fields. A whitespace-only line
among the fields is a continuation for both and is no difference. A message with
no `DKIM-Signature` never gets this finding. The finding is on occurrence 0, or
on `null` when the message has no root DKIM-Signature field (a lone
`DKIM-Signature :` is no field to Swarmail). A passing check proves the body and
the field count are shared, and nothing more.

**Bounds.** At most 16 `DKIM-Signature` fields per message and at most 128 names
in any `h=`. Past either bound the message gets one `dkim.limits_exceeded.v1`
finding on the first field past it, nothing else is judged and nothing is looked
up, so a report carries at most 16 DKIM findings (or one message-level finding).
The work is bounded too: a first parse hashes the body at most once for each
distinct body canonicalization and hash algorithm among signatures without `l=`
(at most four hashes over at most 8 MiB) before a bound or the boundary check can
refuse the message; within the bounds, at most 16 verifications ask for at most
48 names (a key, an ATPS and a `_report._domainkey.<d>` name each).

**Severity.** Every outcome that leaves a signature unverified is an error; the
only warning is a verified signature with a partial `l=`. A `temp_fail` lookup
(a key declared in `tempfail`) gives `dkim.not_evaluated.v1` and no report with a
`temp_fail` lookup ever qualifies. The report of a signed message that passes also
needs its SPF and DMARC evidence and the rest of the checklist to be clean before it
qualifies at a threshold; assert on DKIM rule ids
(`GET /api/v1/messages/{id}/deliverability`) to judge a signer alone.

**Known limits.** The crate does not check a key's bytes before using it, so a
`p=` that is base64 but not a public key fails verification and reads as
`dkim.signature_mismatch.v1`, not `dkim.key_invalid.v1`. A name with an `_atps`
label and no `_domainkey` one is read as an ATPS record and fails closed as
`dkim.atps_unauthorized.v1`. **`l=`
interop gap:** the crate counts `l=` in raw bytes, RFC 6376 section 3.4.5 in
canonicalized ones, so a conformant signer whose raw and canonical `l=` prefixes
differ fails as `dkim.body_hash_mismatch.v1`. That is fail-closed and not detected:
do not read a `body_hash_mismatch` on a signature with `l=` as evidence of
tampering (DMARC reads the same verdict and does not either). The wall-clock residual has the radius stated
above. The compiled-in resolver is unreachable, not absent. Each finding is on
`{"kind":"header","name":"dkim-signature","occurrence":k}`, `k` counting the
message's root DKIM-Signature fields in header order from zero (the boundary
check above guarantees the verifier lists the same fields).

### `dkim.malformed.v1`

**Severity:** error. A duplicate tag, a missing required tag (`v=`, `a=`, `d=`, `s=`, `b=`, `bh=`, `h=`), a value that is not base64, a `t=`, `x=` or `l=` that is not a plain decimal number fitting 64 bits (the crate would read it as absent), or an `x=` at or before `t=`.

**Hint:** Fix the DKIM-Signature tag list: each tag once; v=, a=, d=, s=, b=, bh= and h= present; valid base64; t=, x= and l= plain decimal numbers; and x= later than t= (RFC 6376 sections 3.2 and 3.5).

**Passing:** a well-formed signature.

**Failing:** `bh=` missing; `d=` twice; no `v=`; no `a=` (the crate would default to rsa-sha256); `t=abc`, `x=abc` or `l=abc`; `x=99999999999999999999`; `x=` equal to `t=`; `x=0` with a `t=`.

### `dkim.unsupported.v1`

**Severity:** error. `v=` other than 1, an unknown `a=` or `c=`, `a=rsa-sha1` (RFC 8301), or a literal `l=0` (the crate cannot tell it from no `l=`).

**Hint:** Use v=1, a=rsa-sha256 or ed25519-sha256, c= simple or relaxed, and no l=0 (RFC 6376 section 3.5, RFC 8301 section 3.1, RFC 8463).

**Passing:** `a=ed25519-sha256`.

**Failing:** `a=rsa-sha1`; `l=0`; `v=2`.

### `dkim.header_boundary_ambiguous.v1`

**Severity:** error. The verifier's header/body boundary differs from Swarmail's, so the body it would hash is not the body the rest of the report reads (a line without a colon in the header block, or a whitespace-only first line), or the verifier lists a different number of `DKIM-Signature` fields than Swarmail's root list (a field name with a space before the colon). Only a message with a `DKIM-Signature` gets it; the finding is on occurrence 0 (`null` when the message has no root DKIM-Signature field, as with a lone `DKIM-Signature :`) and nothing else is judged or looked up.

**Hint:** A line in the header block that is not a header field (no colon, or only whitespace at the very top), or a DKIM-Signature field name that not every parser accepts (such as a space before the colon), ends or splits the headers differently at different receivers; remove it, fold it into the previous field, or write the name as DKIM-Signature: (RFC 5322 sections 2.2 and 2.2.3).

**Passing:** a signed message whose boundaries agree; a whitespace-only line among the header fields.

**Failing:** `stray line` between the signature field and the blank line; the same line before the signature; a whitespace-only first line; a `DKIM-Signature :` field (space before the colon) beside a valid signature.

### `dkim.from_unsigned.v1`

**Severity:** error. `h=` does not name `from`, case-insensitively. The crate does not check this.

**Hint:** Sign the From header: list from in h= (RFC 6376 section 5.4).

**Passing:** `h=from:subject`.

**Failing:** `h=subject`.

### `dkim.limits_exceeded.v1`

**Severity:** error. More than 16 `DKIM-Signature` fields, or more than 128 names in one `h=`, or a `DKIM-Signature` with `l=` beside so many `ARC-Message-Signature` fields with `l=` that the two kinds together number more than 16 (the one re-parse an `l=` signature needs would hash them all; decision 0133 section 6). The finding sits on the first field past a bound (the first `DKIM-Signature` with `l=` for the last), nothing else is judged and nothing is looked up.

**Hint:** Send at most 16 DKIM-Signature fields, each with at most 128 header names in h=, and no more than 16 fields with an l= tag in all, ARC-Message-Signature fields included.

**Passing:** 16 valid fields; 128 `h=` names; one `l=` signature beside 15 ARC `l=` fields.

**Failing:** 17 fields; 129 `h=` names; one `l=` signature beside 16 ARC `l=` fields.

### `dkim.signature_expired.v1`

**Severity:** error. `x=` is at or before the second the message was received, or is a literal `x=0` with no `t=`.

**Hint:** Sign again with an x= expiration later than the time the message was received (RFC 6376 section 3.5).

**Passing:** `x=` one second after receipt.

**Failing:** `x=` equal to the receipt second; `x=0` with no `t=`.

### `dkim.expired_since_receipt.v1`

**Severity:** error. `x=` is after receipt, but the crate's own wall clock has passed it. It replaces every other verdict for that signature and hides its key lookups (the clock residual, above).

**Hint:** The signature's x= has passed on this machine's clock since the message was received; sign with a later x= (RFC 6376 section 3.5).

**Passing:** `x=` a day after the wall clock.

**Failing:** `t=1000; x=2000`, received at 1,500.

### `dkim.selector_invalid.v1`

**Severity:** error. The name the verifier asks for, `<s>._domainkey.<d>.`, is not a valid DNS name under the policy's grammar (a `d=` that itself ends in a dot included); it is never looked up.

**Hint:** Use an s= and d= that form a valid DNS name, <s>._domainkey.<d> (RFC 6376 section 3.6.2.1).

**Passing:** `s=sel`.

**Failing:** `s=bad*sel`; `d=example.com.`.

### `dkim.key_missing.v1`

**Severity:** error. No record is declared at `<s>._domainkey.<d>`.

**Hint:** Declare a TXT record at <s>._domainkey.<d> in the DNS policy; none is declared (RFC 6376 section 3.6.2.1).

**Passing:** the key is declared.

**Failing:** the key is not declared.

### `dkim.key_invalid.v1`

**Severity:** error. The record is not a usable key record: not `v=DKIM1`, `p=` missing or not base64, an unsupported `k=`, or a key type that does not match `a=` (an ed25519 key for `rsa-sha256`). A `p=` that is base64 but no key is `dkim.signature_mismatch.v1`.

**Hint:** Publish a v=DKIM1 key record with a valid p= and a k= that matches a= (RFC 6376 section 3.6.1, RFC 8463).

**Passing:** a valid `p=`.

**Failing:** `p=!!!!`; an ed25519 key for `a=rsa-sha256`.

### `dkim.key_revoked.v1`

**Severity:** error. `p=` is empty, which revokes the key.

**Hint:** The key record has an empty p=, which revokes the key; publish a non-empty p= (RFC 6376 section 3.6.1).

**Passing:** a valid `p=`.

**Failing:** `v=DKIM1; p=`.

### `dkim.not_evaluated.v1`

**Severity:** error. The key name (or the ATPS name) is declared in `tempfail`. A report with a `temp_fail` lookup never qualifies.

**Hint:** The key lookup failed temporarily, so the signature was not evaluated; fix the DNS answer and retry (RFC 6376 section 6.1.2).

**Passing:** the key is declared.

**Failing:** the key name is in `tempfail`.

### `dkim.body_hash_mismatch.v1`

**Severity:** error. The canonicalized body does not hash to `bh=`.

**Hint:** The body differs from what was signed; sign the final body with the same c= (RFC 6376 section 3.7).

**Passing:** the intact body.

**Failing:** one body byte altered.

### `dkim.signature_mismatch.v1`

**Severity:** error. The signed headers do not verify against `b=` and the declared key: a signed header changed, another key signed, or `p=` is base64 that is not a public key (the crate does not check key bytes, and its ed25519 backend words every failed check as a library error).

**Hint:** A signed header changed after signing, b= was made with another key, or p= is not a usable public key; sign the final headers with the declared key (RFC 6376 section 3.7).

**Passing:** intact headers.

**Failing:** a signed `Subject` altered (both algorithms); `p=` of base64 non-key bytes.

### `dkim.auid_mismatch.v1`

**Severity:** error. `i=` is not at `d=` or a subdomain of it, or the key has `t=s` and `i=` is a subdomain.

**Hint:** Make the i= domain equal to d= or a subdomain of it, and equal to d= when the key has t=s (RFC 6376 sections 3.5 and 3.6.1).

**Passing:** `i=@d`.

**Failing:** `i=@other.example`.

### `dkim.atps_unauthorized.v1`

**Severity:** error. `atps=` names the From domain and the RFC 6541 record `<d>._atps.<atps>` is not declared, is declared with no values, or is declared with a value that is not a valid ATPS record.

**Hint:** Declare the RFC 6541 ATPS record for the signing domain, or remove atps= from the signature (RFC 6541 section 4).

**Passing:** the ATPS record is declared.

**Failing:** the ATPS record is not declared; it is declared with a junk value.

### `dkim.body_length_exceeds.v1`

**Severity:** error. `l=` is longer than both the raw body and its canonical form. The crate hashes the whole body and passes it.

**Hint:** Remove l= or set it to the body length; l= is longer than the body (RFC 6376 section 3.4.5).

**Passing:** `l=` equal to the body length.

**Failing:** `l=` ten bytes longer.

### `dkim.body_length_partial.v1`

**Severity:** warning. The signature verified, but `l=` covers only a prefix of the body that leaves canonical content out (RFC 6376 section 8.2).

**Hint:** Remove l=: it signs only a prefix of the body, so text can be appended unsigned (RFC 6376 section 8.2).

**Passing:** `l=` equal to the body length.

**Failing:** `l=` short and text appended after it.

## SPF and DMARC (`swarmail-html/9`)

Decision 0126 (issue #255) evaluates SPF for the SMTP envelope and
DMARC for the root `From`, with relaxed and strict alignment of SPF and of the
DKIM signatures above, and reports each outcome the way Authentication-Results
(RFC 8601) names it. It runs lazily, inside the report evaluator, on the stored
bytes after the SMTP `250`; nothing runs at ingest or on the accept path. A
passing result emits nothing: there is no `spf.pass` or `dmarc.pass` id. The
six facts the bulk checklist reads (`policy_present`, `valid_from`, `dkim_pass`,
`dkim_aligned`, `spf_pass`, `spf_aligned`) are now outputs of this evaluation,
so `bulk.dmarc_presence`, `bulk.from_alignment` and `bulk.authenticated_sender`
carry real evidence. The crate is `mail-auth` 0.14.0, as for DKIM; DNS comes
only from the declared policy.

**The observed envelope.** SPF needs the address a message arrived from and the
name the client gave, and Swarmail did not store either. The SMTP session now
records the TCP peer (an IPv4-mapped IPv6 address is written as the IPv4 it
maps) and the argument of the last `HELO`/`EHLO` (trimmed; an empty argument or
no greeting is none; `STARTTLS` clears it) in the stored row, before the one
synchronous insert, so they ride the same commit as the `250`. They are never
read from a header: a forged `Received:`, `X-Forwarded-For:` or
`Authentication-Results:` changes nothing. REST-seeded mail and any row stored
before the columns existed hold none and are never backfilled. Neither fact
appears in any REST, MCP, webhook or feed payload.

**Which SPF identity.** One `check_host` per message: the MAIL FROM domain, or the
HELO name when MAIL FROM is the null reverse path (RFC 7208 section 2.4). A
sender with no `@` uses the HELO name as its domain. SPF is not evaluated, and
the message gets the information finding `spf.not_evaluated.v1`, when there is
no observed peer, no known envelope sender, or a null sender with no HELO name.

**The address SPF sees.** The socket's, so a test over loopback declares
`ip4:127.0.0.1` (and `ip6:::1`) in its record. Or declare `spf_client_ip` in the
policy (the server file, or `PUT /api/v1/inboxes/{inbox}/dns`): an IPv4 or IPv6
literal, stored in canonical form, echoed in the report's `dns.policy` only when
declared, and applied to every peer. Anything else is a 400, or a startup
refusal for the server file. It never makes a message with no observed peer
evaluable.

**Record selection is Swarmail's.** Of the values declared at a name, only those
beginning `v=spf1` followed by a space or the end count (`v=spf10` is not a
record); none is "no record" and more than one is a permanent error (RFC 7208
section 4.5). Every term of the record must name a mechanism or be a modifier
`name=value`; the crate would skip an unknown bare word and swallow the term
after it, so `inlcude:x.example` is a permanent error here. The same rule picks
the `v=DMARC1` value at a `_dmarc` name, with two differences: the tag follows
RFC 7489 section 6.4 (spaces or tabs may surround the `=`, and `DMARC1` is
case-exact), and two `v=DMARC1` values at one name are no record at that name,
not an error (RFC 7489 section 6.6.3), so the walk goes on.

**Record types: TXT only.** The declared policy is a TXT map, so SPF's `a`, `mx`,
`ptr` and `exists` mechanisms and the `%{p}` macro, which need A, AAAA, MX or PTR
answers, are not evaluated: the evaluation is marked and its verdict is
`spf.unsupported_mechanism.v1`, whatever the crate decided, with `spf_pass`
Unknown. That is "not evaluated", never a failure. A term after the first match
is never evaluated (RFC 7208 section 4.6.2), so `ip4:127.0.0.1 a -all` passes
for 127.0.0.1. The `exp=` name is looked up and recorded, but no explanation is
ever expanded.

**Limits (RFC 7208 section 4.6.4).** Ten DNS-querying terms are admitted and the
eleventh is `spf.permerror.v1`; with the policy above only `include:` and
`redirect=` can draw on the count without being unsupported. Void lookups need no
counter of their own: an `include:` or `redirect=` target with no record is
already a permanent error at its first void answer, and every other term that
could be void is unsupported before a void answer can decide anything. One
evaluation looks up at most twelve names (the record, ten terms, one `exp=`
target). A DMARC tree walk asks at most eight names.

**DMARC.** From validity is Swarmail's first: exactly one root `From` field, which
the crate parses where Swarmail's own scan ends the headers, and whose addresses
share one domain (else `dmarc.from_invalid.v1`, or `dmarc.not_evaluated.v1` when
the two parsers disagree). The crate's RFC 9989 tree walk then asks for
`_dmarc.<name>` from the Author Domain upward (at most eight names; a `psd=y` or
`psd=n` record stops the walk). **The organizational domain, and the public
suffix, are determined offline by the declared records alone**: a declared
`psd=y` record marks a public suffix domain and the organizational domain is one
label below it; otherwise it is the shortest name with a record. No public
suffix list is fetched, checked in or compiled in, so a test that wants
`example.co.uk` under a public suffix declares `_dmarc.co.uk` with `psd=y`. The
applied policy is the record's `p=` for the Author Domain's own record, `sp=` for
a record found above it, and `np=` when the Author Domain does not exist, which
Swarmail answers from the declared names (it exists when it, or any name below it,
is declared as a TXT owner or in `tempfail`; RFC 8020); `t=y` steps it down once.
A failure carries the applied policy in its id (`dmarc.fail.none.v1`,
`dmarc.fail.quarantine.v1`, `dmarc.fail.reject.v1`). `pct=` is not a tag of RFC
9989: it is ignored and no message is sampled. Alignment is `adkim` and `aspf`,
relaxed by default, from the crate. DKIM aligns only through signatures Swarmail's
own DKIM verdicts accept (a pass or the `l=` warning; a deferred key lookup stays
Unknown), so a signature that does not verify, or that the crate verifies but
DKIM refuses (an unsigned `From`), never aligns. SPF aligns by its identity's
domain whatever SPF's own result: an aligned MAIL FROM whose record rejects the
sender is aligned and not authenticated, which is exactly the difference between
`bulk.from_alignment` and `bulk.authenticated_sender`.

**The facts, and the checklist.** `valid_from` is True for one From with one
domain, False for none, several or two domains, Unknown when the parsers
disagree. `policy_present` is True when a record applies, False when none does or
a record is invalid, Unknown for a temporary failure. `dkim_pass` is True when a
signature passed, False when there is none or all failed, Unknown when one is
deferred or DKIM stopped at its bound. `spf_pass` is True for a pass and False for
fail, softfail, neutral, none and permerror; Unknown for a temporary error, an
unsupported term, or no evaluation. The two alignment facts are Unknown unless a
policy record applies. The DMARC result is `valid_from` and ((`dkim_pass` and
`dkim_aligned`) or (`spf_pass` and `spf_aligned`)); the three checklist entries
read the same facts, so a failing DMARC message carries both its `dmarc.*` id
and its `bulk.*` entry.

**Authentication-Results correspondence.** `spf=pass`: no `spf.*` finding.
`spf=fail|softfail|neutral|none|temperror|permerror`: `spf.<token>.v1`. No `spf=`
clause (SPF not run): `spf.not_evaluated.v1` or `spf.unsupported_mechanism.v1`.
`dkim=pass`: no `dkim.*` error for that field; any other `dkim=`: the ids above.
`dmarc=pass`: no `dmarc.*` finding. `dmarc=fail`: `dmarc.fail.<policy>.v1`.
`dmarc=none`: `dmarc.none.v1`. `dmarc=temperror`: `dmarc.temperror.v1`.
`dmarc=permerror`: `dmarc.permerror.v1` or `dmarc.from_invalid.v1`. No `dmarc=`
clause: `dmarc.not_evaluated.v1`.

**Known limits and the clock.** SPF's evaluation starts a monotonic timer inside
the crate and refuses a DNS-querying term once 20 seconds have passed on it. That
is the one reachable ambient read on this path (decision 0126 section 8, an
amendment of the report's no-ambient-time rule, accepted on 2026-11-07):
only if the process is descheduled for 20 seconds inside a single evaluation does
the verdict become `spf.permerror.v1` (fail-closed, never a pass), so a clean
report cannot be made to pass by it. The crate's report sampling (`ra=`) reads the
wall clock and is unreachable: every SPF record has `ra=` removed before the crate
sees it; `%{t}` is unreachable too. DMARC reads no clock. The compiled-in resolver
is unreachable, not absent.

Every SPF finding is on `{"kind":"header","name":"return-path","occurrence":null}`
(the envelope, never a stored header); every DMARC finding is on the root `From`
field, `null` when absent, `1` when duplicated, else `0`.

### `spf.fail.v1`

**Severity:** error. The connecting address matches no term before a `-all`, or a term with a `-` qualifier matches it.

**Hint:** The connecting address is not authorized by the SPF record of the domain of the envelope sender, which ends in -all; add the address to the record or send from an authorized host (RFC 7208 section 2.6.4).

**Passing:** `v=spf1 ip4:127.0.0.1 -all` for a message sent from 127.0.0.1.

**Failing:** `v=spf1 ip4:192.0.2.1 -all` for the same message.

### `spf.softfail.v1`

**Severity:** error. The first match, or the `all` fall-through, has the `~` qualifier.

**Hint:** The connecting address is probably not authorized by the SPF record of the domain of the envelope sender (~all); add the address to the record or send from an authorized host (RFC 7208 section 2.6.5).

**Passing:** `v=spf1 ip4:127.0.0.1 ~all` for 127.0.0.1.

**Failing:** `v=spf1 ip4:192.0.2.1 ~all`.

### `spf.neutral.v1`

**Severity:** error. The first match has the `?` qualifier, or no term matches and the record has no `all` or `redirect=` (RFC 7208 section 4.7).

**Hint:** No mechanism of the SPF record of the domain of the envelope sender matched the connecting address, or the record says ?all; add the address to the record (RFC 7208 section 2.6.2).

**Passing:** `v=spf1 ip4:127.0.0.1 ?all` for 127.0.0.1: the term matches, and the match is a pass.

**Failing:** `v=spf1 ip4:192.0.2.1`; `v=spf1 ip4:192.0.2.1 ?all`; `v=spf1`.

### `spf.none.v1`

**Severity:** error. The identity's domain declares no `v=spf1` record, or is not a valid domain name (an empty or malformed MAIL FROM domain).

**Hint:** Declare a v=spf1 TXT record for the domain of the envelope sender in the DNS policy; none is declared, or the domain is not a valid name (RFC 7208 section 2.6.1).

**Passing:** A declared `v=spf1` record at the domain.

**Failing:** No record at the domain; `v=spf10 ...` (not an SPF record); a sender with no domain and no HELO name.

### `spf.temperror.v1`

**Severity:** error. A name SPF needed (the record, or an `include:` or `redirect=` target) is declared in `tempfail`. `spf_pass` is Unknown, never a failure, and a report with a `temp_fail` lookup never qualifies.

**Hint:** A DNS lookup the SPF check needed failed temporarily, so the check was not evaluated; fix the DNS answer and retry (RFC 7208 section 2.6.6).

**Passing:** A declared record.

**Failing:** `tempfail: ["example.com"]`.

### `spf.permerror.v1`

**Severity:** error. Two `v=spf1` values at one name, a syntax error, a term that names no mechanism or modifier, an `include:` or `redirect=` target with no record, or the eleventh DNS-querying term (RFC 7208 sections 4.5, 4.6 and 4.6.4), or the lookup timer residual below.

**Hint:** Fix the SPF record: one v=spf1 record per name, valid syntax, declared include: and redirect= targets, and at most 10 DNS-querying terms (RFC 7208 sections 4.5, 4.6.4 and 2.6.7).

**Passing:** One record, valid terms, at most ten `include`/`redirect` terms, every target declared.

**Failing:** `v=spf1 inlcude:x.example -all`; two `v=spf1` values; `include:gone.example` with nothing declared; eleven `include:` terms.

### `spf.unsupported_mechanism.v1`

**Severity:** error. The evaluation reached `a`, `mx`, `ptr`, `exists` or a `%{p}` macro. Swarmail's declared policy holds TXT records only, so it answered those queries with a fixed negative and did not look: this is not a failure. `spf_pass` is Unknown, so the `bulk.*` entries are `not_evaluated` and DMARC is `dmarc.not_evaluated.v1` unless DKIM passed aligned.

**Hint:** The SPF record reaches a, mx, ptr, exists or a %{p} macro, which need address records the DNS policy cannot declare; Swarmail did not evaluate it, so this is not a failure. Put an ip4: or ip6: term before it, or declare spf_client_ip and an ip4:/ip6: record (RFC 7208 section 5).

**Passing:** `v=spf1 ip4:127.0.0.1 a -all` for 127.0.0.1 (the match comes first; `a` is never evaluated).

**Failing:** `v=spf1 a -all`; `v=spf1 mx -all`; `v=spf1 a ip4:127.0.0.1 -all` (`a` comes first).

### `spf.not_evaluated.v1`

**Severity:** info. SPF was not run: the message has no observed SMTP connection address (REST-seeded mail, or a row stored before the columns existed), no known envelope sender, or a null sender with no HELO name. Info severity: it disqualifies no threshold.

**Hint:** SPF was not evaluated: the message has no observed SMTP connection address, no known envelope sender, or a null sender without a HELO name. Send it over SMTP with a HELO and a MAIL FROM (RFC 7208 section 2.2).

**Passing:** A message sent over SMTP with a MAIL FROM and a connecting address.

**Failing:** A REST-seeded message; a `MAIL FROM:<>` session with no HELO.

### `dmarc.from_invalid.v1`

**Severity:** error. Not exactly one root `From` field (none, or several), or the field's addresses do not share one domain. Located at occurrence `null` (absent), `1` (duplicated) or `0`. `Email.from` is never read: it falls back to the envelope sender, which would make a From-less message look valid.

**Hint:** Send exactly one From header field whose addresses share one domain (RFC 5322 section 3.6.2, RFC 9989 section 4.4).

**Passing:** One `From:` field; two addresses of one domain are one domain.

**Failing:** No `From`; two `From` fields; `From: a@example.com, b@example.org`.

### `dmarc.none.v1`

**Severity:** error. A valid From, and no applicable DMARC record along the tree walk from the Author Domain up: nothing declared, a value that is not a `v=DMARC1` record, a record with neither `p=` nor `rua=`, two `v=DMARC1` values at one name (RFC 7489 section 6.6.3 applies no policy for multiple records, so the walk goes on to the organizational domain), or a single-label Author Domain.

**Hint:** Declare exactly one v=DMARC1 record with a p= policy at the _dmarc name of the From domain or of its organizational domain in the DNS policy; none applies, and two records at one name apply none (RFC 9989 section 4.10).

**Passing:** A declared `v=DMARC1; p=...` record at `_dmarc.<domain>` or at its organizational domain.

**Failing:** Nothing declared; `v=DMARC1; fo=1`; two `v=DMARC1` values at one name and nothing above them.

### `dmarc.temperror.v1`

**Severity:** error. A `_dmarc` name on the walk is declared in `tempfail` (the Author Domain's own, or a parent's, even when a record was found below it). `policy_present` is Unknown and the report never qualifies.

**Hint:** A DNS lookup the DMARC policy discovery needed failed temporarily, so the message was not evaluated; fix the DNS answer and retry (RFC 9989 section 4.10).

**Passing:** Every walked name answers.

**Failing:** `tempfail: ["_dmarc.example.com"]`.

### `dmarc.permerror.v1`

**Severity:** error. A walked name holds one `v=DMARC1` value that does not parse (RFC 9989 section 4.10). Two values are not this rule: they are no record at that name (`dmarc.none.v1`).

**Hint:** Fix the DMARC policy record so that its tags are valid (RFC 9989 section 4.10).

**Passing:** One valid record per name.

**Failing:** `v=DMARC1; p=bogus`.

### `dmarc.fail.none.v1`

**Severity:** error. A record applies, neither SPF nor DKIM produced an aligned pass, and the applied policy is `none`.

**Hint:** Neither SPF nor DKIM passed with a domain aligned to the From domain; the applied policy is none. Align a passing DKIM d= or SPF MAIL FROM domain with the From domain (RFC 9989 section 4.10).

**Passing:** SPF or DKIM passes aligned.

**Failing:** SPF fails and nothing is signed, with `p=none`.

### `dmarc.fail.quarantine.v1`

**Severity:** error. As above, with applied policy `quarantine` (the record's `p=`, `sp=` or `np=`, stepped down by `t=y`).

**Hint:** Neither SPF nor DKIM passed with a domain aligned to the From domain; the applied policy is quarantine. Align a passing DKIM d= or SPF MAIL FROM domain with the From domain (RFC 9989 section 4.10).

**Passing:** SPF or DKIM passes aligned.

**Failing:** SPF fails and nothing is signed, with `p=quarantine`; `p=reject; t=y`.

### `dmarc.fail.reject.v1`

**Severity:** error. As above, with applied policy `reject`.

**Hint:** Neither SPF nor DKIM passed with a domain aligned to the From domain; the applied policy is reject. Align a passing DKIM d= or SPF MAIL FROM domain with the From domain (RFC 9989 section 4.10).

**Passing:** SPF or DKIM passes aligned.

**Failing:** SPF fails and nothing is signed, with `p=reject`.

### `dmarc.not_evaluated.v1`

**Severity:** error. A From that the crate parses where Swarmail's own header scan ends the headers differently (a stray colon-less line), or a record applies and the result is Unknown because SPF or DKIM could not be evaluated (not evaluated, unsupported terms, a deferred key lookup) and nothing passed aligned.

**Hint:** DMARC was not evaluated: the From header cannot be judged the way the Swarmail header scan reads it, or SPF or DKIM could not be evaluated and no aligned pass was found (RFC 9989 section 4.10).

**Passing:** A From both parsers read the same way.

**Failing:** `From: a@example.com` followed by a colon-less line; an SPF record `v=spf1 a -all` with no signature.

## ARC (`swarmail-html/9`)

Decision 0133 (accepted 2026-11-16; issue #256, the last story of epic #251) validates the
ARC chain (RFC 8617) of a stored message and reports it. A forwarder that changes
a message breaks its DKIM signature; ARC lets each forwarder record what it saw
and seal that record, so the next receiver can check the chain of custody. A test
that sends mail through a forwarder or a mailing list into Swarmail can now say
whether the chain it produced validates, and which instance broke it. It runs
lazily, inside the report evaluator, on the stored bytes after the SMTP `250`;
nothing runs at ingest or on the accept path.

**The `arc` member.** Every report carries one more required member, after `dns`
and before `total` (which stays the last key):

```json
"arc": {"state": "pass", "sets": 2, "failing_instance": null, "reason": null}
```

`state` is `none`, `pass`, `fail` or `not_evaluated`. The first three are the
states RFC 8617 section 5.2 gives a chain, and the token `Authentication-Results`
carries. `not_evaluated` is Swarmail's: it declined to judge (a declared
temporary DNS failure, an ambiguous header boundary, a bound, or an expiry only
the wall clock could judge), and calling that `fail` would report a chain nobody
saw broken. `sets` is the largest number of fields any one ARC family holds.
`failing_instance` is the `i=` that broke the chain, only with `fail`. `reason` is
the rule id of the report's one `arc.*` finding. A pass and a message with no ARC
field both emit no finding, so only this member tells them apart: **a test that
asserts "my forwarder's chain validates" should read `arc.state == "pass"`**, not
the absence of `arc.*` findings, which a message with no chain also has.

**What is verified.** The three field families, `ARC-Seal`,
`ARC-Message-Signature` and `ARC-Authentication-Results`, with their instances
`i=` from 1 to 50; the structure (complete sets, instances 1, 2, 3 and so on,
`cv=none` on the first seal and `cv=pass` on every later one); the body hash and
signature of the newest `ARC-Message-Signature`; and every seal, newest first.
`rsa-sha256` and `ed25519-sha256`, as for DKIM. The cryptography, parsing and
canonicalization are the `mail-auth` crate's; what decides a verdict around it is
Swarmail's. The contents of an `ARC-Authentication-Results` field are never read,
and **no fact, no DMARC result and no other finding changes with the chain**:
`arc=pass` says the chain validated, not that the sender is authentic.

**Keys come from the declared DNS map only.** Declare each sealer's key like any
other record, at `<s>._domainkey.<d>` of that hop's `s=` and `d=` (see the DNS
policy above). Every name asked inside the DNS policy's grammar is recorded in
`dns.lookups`; a name outside it is not asked and answers "no record". A chain asks
at most 51 times: once for the newest signature, once per seal.

**Evaluation order.** The first step that applies decides, and a report holds at
most one `arc.*` finding: (0) no ARC field in the root list or the verifier's:
`none`; (1) one body and one field list, on every path (mail with no
`DKIM-Signature` too); (2) at most 128 `h=` names; (3) each field in wire order,
for what the verifier would read silently as a default; (4) the three families
hold the same number of fields; (5) no two fields of one family share an
instance; (6) expiry at the receipt second; (7) the verifier, which checks the
instance sequence, `cv=`, the body hash and every signature and seal.

**Which seal failed.** The verifier asks for the newest signature's key first and
then each seal's key from the newest down, and stops at the first failure. The
number of keys it asked for names the field, exactly, for any chain length. The
instance named is the **newest** seal that does not verify: each seal covers every
older set, so bytes altered in an older set after a newer hop sealed are reported
at the newest seal that covers them, which is where RFC 8617 validation stops
too. An older seal is named when the newer ones verify: a hop that sealed over an
already broken seal, or a declared key for that seal's selector that is not the
one it was sealed with. A finding's `occurrence` is the field's place among the
fields of its name in the message, never its instance, so a chain written
oldest-first names the right field.

**Time is the receipt time.** Swarmail reads no clock. The newest
`ARC-Message-Signature`'s `x=` at or before the receipt second, or at or before
its own `t=`, is `arc.signature_expired.v1`. Any other `x=` on that field is
`arc.not_evaluated.v1`: **a chain whose newest signature carries an `x=` after
the receipt second is not verified**, and reports an error, never a clean pass.
The verifier reads the wall clock once and compares it with that `x=` only; the
chain is handed to it only when that field has no `x=`, so the value it read is
never compared. The operator accepted this reading on 2026-11-16 (decision 0133 section 5). An `x=`
on an older signature is nobody's expiry. A chain whose newest signature has an
`x=` is decided at that step even if its instances or `cv=` values are also wrong.

**Bound and `l=`.** ARC always reads the strict parse of the message, so its
verdict does not depend on an unrelated `DKIM-Signature` that carries `l=`. DKIM's
non-strict re-parse for an `l=` signature would also hash every
`ARC-Message-Signature` with `l=`, so the `l=` fields of both kinds share one bound
of 16: a message with a DKIM `l=` field and more than 15 ARC `l=` fields gets
`dkim.limits_exceeded.v1` at its first DKIM `l=` field and no other DKIM verdict.

**Authentication-Results correspondence.** `arc=none`: `state` is `none`.
`arc=pass`: `state` is `pass`. `arc=fail`: `state` is `fail`, and `reason` and the
one `arc.*` finding say why. No `arc=` clause: `state` is `not_evaluated`.

**Assert and await.** Every `arc.*` finding is an error, so `no_findings_at_or_above`
at `error` does not count a message whose chain failed or was not evaluated, and
a passing chain or no chain adds nothing. A rule's weight is 10, and 0 for
`arc.not_evaluated.v1`, as for every rule (Weight and total, below).

### `arc.header_boundary_ambiguous.v1`

**Severity:** error. The state is `not_evaluated`. The verifier and Swarmail's header scan disagree about where the headers end or which fields are ARC fields (a colon-less line, a whitespace-only first line, a name with a space before the colon, or a name that begins with `ARC-Seal`), so they would not judge the same fields. Located at the first root ARC field in wire order, or `arc-seal` with occurrence `null` when the root list holds none.

**Hint:** A line in the header block that is not a header field (no colon, or only whitespace at the very top), or an ARC field name that not every parser accepts (such as a space before the colon), ends or splits the headers differently at different receivers, so the chain was not evaluated; remove it, fold it into the previous field, or write the name as ARC-Seal:, ARC-Message-Signature: or ARC-Authentication-Results: (RFC 5322 sections 2.2 and 2.2.3, RFC 8617 section 4.1).

**Passing:** a valid chain with the header block in order.

**Failing:** a colon-less line before or after the ARC fields; a whitespace-only first line; `ARC-Seal :` beside a valid chain.

### `arc.limits_exceeded.v1`

**Severity:** error. The state is `not_evaluated`. An `ARC-Message-Signature` lists more than 128 names in `h=`; located at the first one in wire order.

**Hint:** Sign at most 128 header names in the h= tag of each ARC-Message-Signature; the chain was not evaluated (RFC 8617 section 4.1.2).

**Passing:** an `h=` of 128 names.

**Failing:** an `h=` of 129 names.

### `arc.malformed.v1`

**Severity:** error. The state is `fail`. A field the verifier rejects, or that it would read through a silent default: a repeated tag, no `a=`, a `t=` or `x=` that is not a plain number, an `i=` that is missing, not 1 to 50 or not the instance the verifier read (it keeps the low 32 bits of a longer one), an `ARC-Authentication-Results` whose `i=` is not first, a seal with `h=` or no `cv=`. `failing_instance` is the field's own `i=` when it is 1 to 50.

**Hint:** Fix the ARC field: each tag once; a= present; t= and x= plain decimal numbers; an i= instance from 1 to 50, written first in an ARC-Authentication-Results; valid base64, cv=, d=, s= and b= as RFC 8617 requires, and no h= in an ARC-Seal (RFC 8617 sections 4.1.1 to 4.1.3).

**Passing:** the sealer's own output.

**Failing:** `i=0`; `i=51`; `i=4294967297`; no `a=`; `x=abc`.

### `arc.unsupported.v1`

**Severity:** error. The state is `fail`. An `ARC-Message-Signature` with an `l=` tag, whatever its value (a body length limit lets text be appended unsigned), or `a=rsa-sha1` on a signature or a seal.

**Hint:** Use a= rsa-sha256 or ed25519-sha256 and no l= tag in an ARC-Message-Signature: a body length limit lets text be appended unsigned (RFC 8617 section 4.1.2, RFC 8301 section 3.1, RFC 6376 section 8.2).

**Passing:** `a=rsa-sha256` or `ed25519-sha256` and no `l=`.

**Failing:** `l=5`; `l=0`; `a=rsa-sha1`.

### `arc.set_incomplete.v1`

**Severity:** error. The state is `fail`. The three families hold different numbers of fields. It includes ARC fields with no `ARC-Message-Signature` at all, which is a broken chain and never `none`. Located at the family with the fewest fields (ties in the order results, signature, seal) with occurrence `null`; `failing_instance` is the lowest instance the families disagree on.

**Hint:** Every instance needs exactly one ARC-Seal, one ARC-Message-Signature and one ARC-Authentication-Results; the three families hold different numbers of fields (RFC 8617 section 4.2).

**Passing:** a seal, a signature and a results field for every instance.

**Failing:** no `ARC-Authentication-Results` for instance 2; ARC fields and no `ARC-Message-Signature`.

### `arc.signature_expired.v1`

**Severity:** error. The state is `fail`. The newest `ARC-Message-Signature` has an `x=` at or before the receipt second, or at or before its own `t=` (a literal `x=0` included). Located at that field.

**Hint:** The newest ARC-Message-Signature has expired: its x= is at or before the time the message was received, or at or before its t=; seal again with a later x= or none (RFC 8617 section 4.1.2, RFC 6376 section 3.5).

**Passing:** no `x=`.

**Failing:** `x=1000` for a message received at second 1000.

### `arc.instance_invalid.v1`

**Severity:** error. The state is `fail`. Two fields of one family share an instance (located at the second field in wire order that carries the lowest duplicated instance, of the first family that duplicates it), or the instances are not 1, 2, 3 and so on in the verifier's sorted order (located at the first of seal, signature and results whose `i=` is not its position).

**Hint:** The i= instances must be distinct and run 1, 2, 3 and so on, up to 50, in all three families (RFC 8617 sections 4.2.1 and 5.2).

**Passing:** sets 1, 2 and 3.

**Failing:** sets 1 and 3; sets 2 and 3; two sets that both say `i=1`; 51 sets.

### `arc.cv_invalid.v1`

**Severity:** error. The state is `fail`. A seal's `cv=` is not `none` at instance 1 or not `pass` later. Located at the first seal, by instance, that is wrong.

**Hint:** The ARC-Seal of instance 1 needs cv=none and every later seal needs cv=pass; a seal with cv=fail or any other value ends the chain (RFC 8617 sections 4.1.3 and 5.2).

**Passing:** `cv=none` at instance 1 and `cv=pass` after it.

**Failing:** `cv=pass` at instance 1; `cv=none` or `cv=fail` at instance 2.

### `arc.body_hash_mismatch.v1`

**Severity:** error. The state is `fail`. The body is not the one the newest `ARC-Message-Signature` signed: a hop changed it after sealing. Located at that field.

**Hint:** The body differs from what the newest ARC-Message-Signature signed, so a hop changed it after sealing; seal after the last change, with the same c= (RFC 8617 section 5.2, RFC 6376 section 3.7).

**Passing:** the body as sealed.

**Failing:** a body edited after the last hop sealed.

### `arc.key_missing.v1`

**Severity:** error. The state is `fail`. No record is declared at the `<s>._domainkey.<d>` name of the field whose key was asked for, or the name is outside the DNS grammar. Under decision 0120 the declared map is the world, so an undeclared key is a definite absence, as for DKIM. Located at that field.

**Hint:** Declare a TXT record at the selector._domainkey.domain name of the field named in the finding in the DNS policy; none is declared, or the name is not a valid DNS name (RFC 8617 section 5.2, RFC 6376 section 3.6.2.1).

**Passing:** every sealer's key is declared.

**Failing:** a key not declared for a forwarder.

### `arc.key_invalid.v1`

**Severity:** error. The state is `fail`. A record is declared and is not a usable key: not a key record, revoked (an empty `p=`), or of the other type than the field's `a=`. Located at the field whose key was asked for.

**Hint:** Publish a v=DKIM1 key record with a valid, non-empty p= and a k= that matches the a= of the field named in the finding (RFC 6376 section 3.6.1, RFC 8463).

**Passing:** a `v=DKIM1` key record of the right type.

**Failing:** `v=spf1 -all`; `p=` empty; an RSA key for an ed25519 seal.

### `arc.signature_mismatch.v1`

**Severity:** error. The state is `fail`. A signature or seal does not verify against its declared key: a wrong key of the right type, a sealed header or older set that changed after sealing, key bytes that are no key. Located at the newest field that fails (see Which seal failed).

**Hint:** The field named in the finding is the newest signature or seal that does not verify against its declared key: a sealed header or an older ARC set changed after sealing, b= was made with another key, or p= is not the right public key (RFC 8617 section 5.2).

**Passing:** every seal verifies against its declared key.

**Failing:** another key declared for a hop; the oldest seal's `b=` altered after the second hop sealed.

### `arc.not_evaluated.v1`

**Severity:** error, weight 0. The state is `not_evaluated`. A key lookup failed temporarily (a declared `tempfail`), located at the field whose key was asked for; or the newest `ARC-Message-Signature` has an `x=` after the receipt second, located at that field. A declared temporary failure also keeps the report from qualifying at any threshold (decision 0120).

**Hint:** The chain was not evaluated: a key lookup failed temporarily, or the newest ARC-Message-Signature carries an x= later than the receipt time, which Swarmail does not verify; fix the DNS answer or seal without x=, and retry (RFC 8617 section 5.2).

**Passing:** a key answered, and no `x=` on the newest signature.

**Failing:** a declared `tempfail` on a sealer's key; `x=` later than the receipt second.

**Known limits.** The newest-failing-seal rule above; a chain whose newest
signature carries an `x=` after the receipt second is reported `not_evaluated`
and not verified; real sealers' use of `x=` on that field was not measured here.
The verifier checks signatures only: an `ARC-Authentication-Results` field is
sealed text written by an intermediary, and nothing trusts it. Trusted-sealer
lists, sealing and real DNS are not part of this change.

## Spam signals (`swarmail-spam/1`)

Six built-in, offline, deterministic rules read only the captured message: the
root Subject, the root From display name and the sender's `text/plain` bodies
(decision 0122, accepted). There is no external scorer, network, clock, locale or
configuration; the same stored bytes always give the same findings. Each rule
emits at most one finding per report, severity info or warning (error stays the
bulk checklist's requirement level), so a large message can never flood the
report. They arrived as `swarmail-spam/1`; with the link rules of decision 0121,
the `dns` member of decision 0120, the DKIM rules of decision 0125, the SPF and
DMARC rules of decision 0126, the weight and total of decision 0122 §2 and the
ARC chain of decision 0133 the report's `rule_set_id` is now
`swarmail-html/9`, so the rule-set version
is recorded in every report and in the cache key. Every finding carries a
`weight` and the report a `total` (see Weight and total below); the weights
named in each rule are the weights the report emits.

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
- **Severity:** warning. **Weight:** 3.
- **Location:** `header {name:"subject", occurrence:0}`.
- **Why:** an all-capital subject reads as shouting and is a classic spam signal;
  the 8-letter floor keeps short acronyms such as `RSVP` or `OTP: 123456` clear.
- **Fix hint:** `Write the Subject in sentence case; an all-capital subject reads as shouting and is a common spam signal.`
- **Passing:** `Subject: URGENT` (6 letters) and `Subject: WINTER Sale` (a lowercase letter).
- **Failing:** `Subject: WINTER SALE`.

### `spam.subject.punctuation.v1`

- **Matches:** a Subject containing `!!`, `??` or `$$`.
- **Severity:** info. **Weight:** 1.
- **Location:** `header {name:"subject", occurrence:0}`.
- **Why:** repeated emphasis punctuation is a weak but common sales-spam marker; a
  single `!`, `?` or `$` is ordinary.
- **Fix hint:** `Remove repeated !, ? or $ characters from the Subject.`
- **Passing:** `Subject: Your receipt!`
- **Failing:** `Subject: Your receipt!!`

### `spam.subject.phrases.v1`

- **Matches:** a Subject containing a phrase from the list below.
- **Severity:** warning. **Weight:** 3.
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
- **Severity:** warning. **Weight:** 3.
- **Location:** `header {name:"from", occurrence:0}`.
- **Why:** a different address in the display name is how mail clients are made to
  show a trusted-looking sender; the same address as the real one is harmless.
- **Fix hint:** `Do not put an email address in the From display name unless it is the sending address; a different address there looks like spoofing.`
- **Passing:** `From: "Billing" <billing@example.test>` and `From: "Billing@Example.test" <billing@example.test>`.
- **Failing:** `From: "ceo@bank.example" <billing@example.test>`

### `spam.body.phrases.v1`

- **Matches:** an eligible text part containing a phrase from the list below.
- **Severity:** warning. **Weight:** 3.
- **Location:** `mime_part {path}` of the first matching eligible part.
- **Why:** the same claims as the Subject rule, in the body.
- **Fix hint:** `Remove high-pressure sales wording from the text/plain body; see the guide's phrase list.`
- **Passing:** `Your order has shipped.`
- **Failing:** `Your order has shipped. Act now.`

### `spam.body.zero_width.v1`

- **Matches:** an eligible text part with at least 3 zero-width characters
  (U+200B, U+200C, U+2060, U+FEFF). U+200D (ZWJ) is excluded because emoji
  sequences use it legitimately.
- **Severity:** info. **Weight:** 1.
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
`bulk.date.v1`, `html.image-only.v1` and `html.tracking-pixel.v1`. Every one of
them is already counted: the report's `total` weighs every finding by its
severity (info 1, warning 3, error 10), with `*.not_evaluated.*` findings and
`demo.marker.v1` at zero (Weight and total, below). Link and URL signals are #258's
link rules and DNS policy belongs to #253; the spam rules read no URL and resolve
nothing.

### Weight and total

Decision 0122 §2, accepted on 2026-11-06 and emitted from `swarmail-html/8`,
gives every finding a `weight` and the report a `total`, so no reader has to
invent a weighting and no edge (REST, MCP, the UI) computes a score. Both are
non-negative integers, and each is the **last** key of its object: a finding is
`rule_id`, `location`, `severity`, `fix_hint`, `weight`, and the report is
`rule_set_id`, `findings`, `dns`, `arc`, `total`. MCP serializes the same report with
sorted keys, where `weight` and `total` also sort last and the new `arc`
sorts first, so no key that was already served moves.

| severity | weight |
|---|---|
| info | 1 |
| warning | 3 |
| error | 10 |

The weight depends on the severity alone, for every rule family: the demo
marker, the bulk checklist, the HTML heuristics, the link rules, the spam
signals, DKIM, SPF and DMARC. There is no per-id table. Two exceptions weigh 0:
any rule id containing `.not_evaluated.` (a trusted fact that could not be
determined is not evidence about the sender, and a fresh deployment must not
score every message) and `demo.marker.v1` (synthetic).

The `total` is the sum of the findings' weights after the findings are sorted
and exact duplicates are removed, so a duplicate counts once. It is at most
2,560 (256 findings of weight 10). The evaluator computes it inside the byte
bound, so a report served from the cache and a freshly evaluated one are
byte-identical, and a sum that would overflow fails the report and never wraps.

**Predicates ignore weight.** `no_findings_at_or_above` on `assert`, `await`
and `swarmail_wait_for_email` still decides on severity alone, plus the
`temp_fail` rule of decision 0120. A report whose `total` is 0 is not "clean"
(a sole `bulk.*.not_evaluated.v1` error weighs 0 and still refuses at `error`),
and a large `total` is not "dirty" (256 info findings total 256 and still
qualify at `warning`).

**Cost.** The two keys are part of the report's 65,536-byte bound: about 11 to
12 bytes per finding and up to 14 for the total. A message whose report was just
under the bound before `swarmail-html/8` can now fail analysis, which is the
bound doing its job (it counts what is served).

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
`swarmail-spam/1`; no existing rule changed meaning), then
`swarmail-html/5` (the required `dns` report member of proposed 0120,
integrated after `swarmail-html/4`; no rule changed meaning), then
`swarmail-html/6` (the eighteen DKIM rules of proposed 0125, integrated after
`swarmail-html/5`; no existing rule changed meaning), then
`swarmail-html/7` (the sixteen SPF and DMARC rules of proposed 0126, integrated
after `swarmail-html/6`; the three authentication entries of the bulk checklist
now read evaluated facts, so their findings changed), then
`swarmail-html/8` (the per-finding `weight` and the report `total` of decision
0122 §2, integrated after `swarmail-html/7`; no rule changed meaning, and every
report gained the two keys), then
`swarmail-html/9` (the thirteen ARC rules and the required `arc` report member of
proposed 0133, integrated after `swarmail-html/8`; no existing rule changed
meaning, and every report gained the member). A change to
any rule's meaning, hint, location or order requires a new id.
