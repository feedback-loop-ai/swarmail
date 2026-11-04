# Deliverability link checks — specification

Date: 2026-11-03. Epic #251, issue #258. Run
`issue-258-deliverability-link-ch-d9779f5d`, from `main` `8732445`.

Revised 2026-11-04 after review of head `33efa4c` returned the design (run
`issue-258-deliverability-link-ch-4052800f`, decision 0102's return,
`REVIEW-RESIDUAL-SPEC-SECURITY-REDESIGN`). The defect was this spec's
address policy, which ended "Everything else is permitted". It is now a
default-deny allow-list. [plan.md § Revision
2026-11-04](plan.md#revision-2026-11-04--answering-the-review) maps every
finding to the text that answers it. Git history keeps the spec the code
was first built to (`a1977dc`).

Decision: 0121 — [Per-link deliverability findings and an opt-in, bounded reachability probe](../../docs/decisions/0121-deliverability-link-checks.md) (proposed; only the operator accepts it).

Authorities: accepted [0085](../../docs/decisions/0085-deliverability-report.md)
and its [contract](../deliverability-report-contract/spec.md#contract);
accepted [0106](../../docs/decisions/0106-html-client-compatibility.md) and
[0107](../../docs/decisions/0107-html-review-repair.md); 0094 §1 (#154's
anchor labels); the frozen guarantees 0001–0003 and 0005; 0020 (test
placement); 0045 §8 (code under a proposed decision).

## What and why

The deliverability report (0085) has a `link` location variant that no
real rule emits yet. Senders and agents testing mail want to know whether
the links in a message are dangerous, weak or broken before it goes out.
This story adds per-link findings:

- **Static rules, in the default report:** unsafe schemes, invalid
  (non-absolute) links, plain `http`, public URL shorteners, and anchor text
  that names a different host than the target.
- **Opt-in reachability:** whether each `http`/`https` target answers, and
  where its redirect chain ends. This runs only when the operator enabled it
  at startup **and** a caller asks for it on one REST request. It never runs
  on SMTP, in predicates, in MCP or on the message page. It is bounded and
  refuses non-public addresses unless the operator chose `loopback`.

Empty link findings mean "no findings from this rule set", never "the
links are safe".

## Framing notes

**Authority actually read.** `.brokkr/state/tasks/` is absent from this
worktree, and epic #251 is not in the tree. This seat has no network, so
the epic's exit criteria were **not read at source**, and this spec does
not claim to have verified them. The design instead rests on:

- the task brief's transcription of the operator direction of 2026-11-03;
- the operator's #251 ruling of 2026-10-26 as accepted 0085 records it in
  its `Authority` line: analyze stored bytes lazily, cache the results,
  never analyze on SMTP, no external scorer, and no new dependency for this
  kind of story;
- accepted 0085, 0106 and 0107, and the intake result (14 criteria).

The design meets every point of that recorded ruling. Before 0121 is
presented for acceptance, an operator or a seat that can read #251 should
check its exit criteria against this spec.

This design rules on six intake criteria; [plan.md § Panel
adjudication](plan.md#panel-adjudication) records each ruling and its cost:

- AC4: the probe has its own cap and does not use the job pool.
- AC6: probe results are not cached.
- AC7: no cache-key change.
- AC9: a constant list, not JSON.
- AC11: a production `loopback` mode instead of a `cfg(test)` seam.
- AC12: no harness is needed unless a cold line appears.

## Contract

### Eligible links

For each eligible HTML part, the links are every **visible `<a>` start tag
that has an `href`**, in source order. Eligible parts are the ones the 0106
walk already selects: outer-message `text/html` leaves outside attachment
subtrees and attached messages, under the same raw, aggregate, part, depth
and nesting budgets. "Visible" and the label follow #154 exactly, through
the same `extract::walk`:

- the label is the decoded text runs between this `<a>` and the next `<a>`
  or `</a>`;
- ASCII whitespace collapses to one space;
- the 0094 §1 block boundaries add one space;
- the label has no cap of its own. #154 has none either: ingest's own test
  pins a 3000-character label. Labels are disjoint text runs, so they are
  bounded only by 0106's 4 MiB aggregate body budget.

Only the first `href` of a tag counts. `<img src>`, `<area>`, CSS `url()`
and text/plain URLs are out of scope.

The **href value** is the source value with HTML character references
decoded once (`decode_refs`) and ASCII whitespace trimmed. It is then parsed
with `reqwest::Url` (WHATWG), which lowercases the scheme and host, removes
tab, CR and LF inside the value, and punycodes IDN hosts.

### Location

Each finding is located at `link {part_path, byte_offset}`. `byte_offset`
is the zero-based offset, in the part's transfer-decoded payload, of the
first source byte of the href value that is not ASCII whitespace. The
offset is taken before charset or entity decoding.

This offset is valid only when the part's character-decoded text is a
**prefix of its transfer-decoded payload, and the rest of the payload is
ASCII whitespace** (`is_exact`). Under that test, every offset in the
decoded text indexes the same byte in the payload. A trailing newline that
mail-parser trims does not move an offset. If the test fails, every link
finding in that part uses `mime_part {path}`, 0085 §1's fallback. A failed
transfer decode gives an empty payload, which is never a prefix of a
non-empty body, so it also falls back. Swarmail never emits a guessed
offset.

### Static rules (default report)

Rules are evaluated per link, in this order. One link may carry several
findings, and the 0085 canonicalizer orders and deduplicates them.

| rule id | sev | fires when | fix hint (literal) |
|---|---|---|---|
| `link.url.invalid.v1` | warning | the href value is empty, or `Url::parse` fails (relative path, `//host`, `{{placeholder}}`, invalid host); a value starting with `#` never fires | `Use an absolute https URL; mail has no base URL for relative links.` |
| `link.scheme.unsafe.v1` | error | parsed scheme ∈ {`javascript`, `vbscript`, `data`, `file`} | `Replace the script, data or file link with an https link.` |
| `link.scheme.http.v1` | warning | scheme `http`, and the host is not the domain `localhost` or a subdomain of it, nor an IPv4 address in `127.0.0.0/8`, nor IPv6 `::1` | `Serve the link over https.` |
| `link.shortener.v1` | warning | scheme `http`/`https`, and the domain host with one trailing dot removed equals an entry of `SHORTENERS` | `Link to the destination directly instead of through a URL shortener.` |
| `link.anchor-host-mismatch.v1` | warning | scheme `http`/`https`, the label has ≥ 1 host-like token, and no token T matches the target host H (below) | `Make the visible link text name the host the link goes to.` |

**Host-like tokens.** Split the label on ASCII whitespace. Apply these
steps to each token, in this order. The 2026-11-04 revision moved the
leading trim before the scheme strip, and moved the `www.` strip before
qualification:

1. Skip the token if it contains `@` or any non-ASCII character.
2. Lowercase it.
3. Trim leading `(` `[` `<` `"` `'`.
4. Remove a leading `http://` or `https://`.
5. Cut at the first `/`, `?`, `#` or `:`.
6. Trim trailing `.` `,` `;` `:` `!` `?` `)` `]` `>` `"` `'`.
7. Remove one leading `www.`.

The result qualifies if it has at least two dot-separated labels, every
label is non-empty `[a-z0-9-]`, and the last label is alphabetic with at
least two characters. So `v1.2`, `3.14`, `e.g.` and `www.com` (which is
`com` after step 7) do not qualify. `(https://paypal.com)`,
`<https://paypal.com/>` and `"https://paypal.com/login"` all give
`paypal.com`.

**Matching.** Take H as the target host, lowercased, with one trailing dot
removed and then one leading `www.` removed. IP-literal hosts are compared
as written. T matches when H == T or H ends with `.`+T. So `paypal.com` →
`paypal.com.evil.test` fires, and `example.com` → `www.example.com` or
`click.example.com` passes.

**Shortener list.** `SHORTENERS` is a sorted, lowercase, unique Rust
constant. `SHORTENERS_VERSION = "swarmail-shorteners/1"`, basis date
2026-11-03. It is Swarmail-authored and not fetched. Initial entries:

> `bit.ly`, `bl.ink`, `buff.ly`, `cutt.ly`, `goo.gl`, `is.gd`, `lnkd.in`,
> `ow.ly`, `rb.gy`, `rebrand.ly`, `s.id`, `shorturl.at`, `t.co`, `t.ly`,
> `tiny.cc`, `tinyurl.com`, `v.gd`

Matching is exact. `notbit.ly`, `bit.ly.evil.test` and `www.bit.ly` do not
match.

**Rule set.** `RULE_SET_ID` and `html-rules.json`'s `rule_set_id` become
`swarmail-html/4`. Every literal of `/3` in code, fixtures, OpenAPI,
`llms.txt` and the guide moves with it. Siblings #253 and #260 bump the
same id; the integrator takes the next free version at merge.

### Opt-in reachability

**Enablement.**

- CLI `--link-reachability <off|public|loopback>`, env
  `SWARMAIL_LINK_REACHABILITY`, default `off`. It is carried as
  `Config::link_reachability` and set on the Store at startup.
- REST `GET /api/v1/messages/{id}/deliverability?reachability=true`.
  `reachability=false` or an absent parameter returns the default report.
  Any other value, or an unknown parameter, is the existing 400 query
  refusal.
- On an `off` server, `reachability=true` returns
  `400 {"error":"link reachability is not enabled"}` and opens no
  connection.
- A missing message stays 404, checked before the mode.
- MCP, the message page, `assert`, `await` and MCP wait are unchanged and
  never probe.

**Targets.** The probe takes the distinct parsed `http`/`https` URLs of the
message's eligible links. The **target key** is `Url::as_str` after the
fragment is cleared. A fragment is never sent, so `x#a` and `x#b` are one
target, fetched once. Each occurrence finds its outcome under the same
key. More than **32** distinct keys is `AnalysisError::Failed` (503, the
existing fixed message) before any connection.

**Per-target procedure.** All hops share a **5 s** budget, after which the
outcome is `timeout`. For each hop:

1. If the URL has userinfo, the outcome is `refused`.
2. Resolve the host: an IP literal is used directly; a name goes through
   the resolver (`tokio::net::lookup_host`, with a `cfg(test)` resolver
   seam). A resolver error or an empty answer is `connect`.
3. If **any** resolved address is not permitted by the mode, the outcome is
   `refused`.
4. Otherwise connect only to the first address. The client is pinned to it
   (`resolve_to_addrs`) and built with `no_proxy`, `redirect::Policy::none`,
   a 2 s connect timeout, no cookie store and `User-Agent: swarmail-link-check`.
5. Send `GET` and read the status and headers only. The body is never read.
6. Classify the answer:
   - a send error (connect, TLS or protocol) → `connect`;
   - 301, 302, 303, 307 or 308 with a `Location` that joins to an
     `http`/`https` URL → the next hop, unless 5 redirects were already
     followed (→ `redirect_cap`);
   - such a status with no `Location`, an unparsable one, or a non-HTTP
     one → `bad_redirect`;
   - status ≥ 400 → `status(code)`;
   - anything else → `ok`.

**Address policy (default-deny, revised 2026-11-04).** `permitted(ip, mode)`
asks whether the address is globally routable unicast. It never asks
whether the address is on a list of bad ranges. The basis is the IANA IPv4
and IPv6 Special-Purpose Address Registries as read on 2026-11-04. The
code comment and the guide record that basis date. The rules, in order:

1. `off` permits nothing.
2. IPv4-mapped IPv6 (`::ffff:0:0/96`) is unwrapped and judged as IPv4. No
   other form that embeds IPv4 is decoded.
3. Loopback (`127.0.0.0/8`, `::1`) is permitted only in `loopback` mode.
4. **IPv4** is permitted only outside every entry of the IPv4 registry
   that is not globally reachable: `0/8`, `10/8`, `100.64/10`,
   `169.254/16`, `172.16/12`, `192.0.0/24`, `192.0.2/24`, `192.88.99/24`
   (deprecated 6to4 relay anycast), `192.168/16`, `198.18/15`,
   `198.51.100/24`, `203.0.113/24` and `224/3` (`224/4` multicast and
   `240/4` reserved, including broadcast). IPv4 global unicast is the
   whole space minus that registry, so for IPv4 this complement is the
   allow-list. `192.31.196/24`, `192.52.193/24` and `192.175.48/24` are
   globally reachable anycast, so they stay permitted.
5. **IPv6** is permitted only inside **`2000::/3`**, and then only outside
   four blocks: `2001::/23` (IETF protocol assignments, including Teredo
   `2001::/32`), `2001:db8::/32` (documentation), `2002::/16` (6to4) and
   `3fff::/20` (documentation, RFC 9637). Every address outside
   `2000::/3` is refused without being named. That covers `::`, `::/96`
   (IPv4-compatible), `::ffff:0:0:0/96` (IPv4-translated), `64:ff9b::/96`
   and `64:ff9b:1::/48` (NAT64), `100::/64`, `5f00::/16`, `fc00::/7`,
   `fe80::/10`, `fec0::/10`, `ff00::/8` and every unassigned block.
6. An address that embeds IPv4 (NAT64, 6to4, Teredo, IPv4-compatible or
   IPv4-translated) is **refused, never decoded**.

`std`'s `is_global` is not used: its table moves with the toolchain, and
this policy must be pinned by Swarmail. No mode has an "everything else is
permitted" clause.

**Residual.** If IANA later allocates a special-purpose block *inside*
`2000::/3`, as it did with `3fff::/20` in 2024, that block is permitted
until the table is edited. Any allocation outside `2000::/3` is refused
with no change.

**Concurrency.** Each request probes at most **4** targets at once. At most
**2** probe requests run per `Service`, and a production process has one
`Service`. A third concurrent request is `AnalysisError::Unavailable`
(503). The cap belongs to the `Service` and is not a process-global
static, so parallel tests do not contend for it. Probes run on the async runtime and never hold one of 0085's four job
permits or a Store guard. The one occurrence scan before probing runs on
`Store::offload` under a job permit, which is released before any
connection. Dropping the request cancels its probes and releases its permit.

**Resolver residual (accepted and documented).** `tokio::net::lookup_host`
runs `getaddrinfo` on tokio's blocking pool, and that call cannot be
cancelled. When the 5 s budget drops a probe, its resolver thread keeps
running until the OS resolver gives up. A hostile hostname plus repeated
explicit requests can therefore park blocking threads. The operator-enabled
mode, the 2 requests × 4 links per `Service` and the OS resolver's own
deadline bound that. The cost is latency only. It never delays a `250`,
because `Store::insert` is synchronous and the probe holds no job permit.
0121 §6 and `network-posture.md` record the residual. A dedicated bounded
resolver would need a separate decision.

**Findings.** Each outcome is reported at every location of its target:

| rule id | sev | when | fix hint |
|---|---|---|---|
| `link.redirect-chain.v1` | info | `ok` or `status` after ≥ 1 redirect | `This link redirects N time(s) before reaching <origin>; link to the destination directly.` Here `<origin>` is the final URL's scheme, host and port only. |
| `link.reachability.v1` | warning | `timeout` | `The link did not finish answering within 5 seconds during the opt-in reachability check.` |
| `link.reachability.v1` | warning | `connect` | `The link's server could not be reached (DNS, connection or TLS failure) during the opt-in reachability check.` |
| `link.reachability.v1` | warning | `status(code)` | `The link answered HTTP <code> during the opt-in reachability check; fix or replace the target.` |
| `link.reachability.v1` | warning | `redirect_cap` | `The link redirected more than 5 times during the opt-in reachability check; link to the destination directly.` |
| `link.reachability.v1` | warning | `bad_redirect` | `The link redirected to a missing, invalid or non-HTTP location during the opt-in reachability check.` |
| `link.reachability.refused.v1` | info | `refused` | `Not checked: the link uses credentials or resolves to an address the reachability check never contacts in this mode.` |

**Composition.** The probed response is `{rule_set_id, findings}`. Its
findings are the cached deterministic report's findings plus the
observation findings, passed through the one canonicalizer under
`MAX_FINDINGS` and `MAX_REPORT_BYTES`. Exceeding either limit is `Failed`.
The deterministic part comes from the existing cache slot, and the slot's
inputs are unchanged. The composed response is never cached, and an
observation never enters a slot.

## Acceptance Criteria

Every criterion is proven by a test named in [tasks.md](tasks.md). Wire
tests run real SMTP and REST against an in-process server. Reachability
tests contact only local `127.0.0.1`/`::1` listeners, and no test resolves a
public name.

- **AC1 — Rule table.** Each of the eight rule ids above emits exactly its
  severity, literal hint and location. Each has at least one failing
  fixture (fires) and one passing fixture (does not fire). Pinned edge
  fixtures:
  - entity-encoded and tab-split `javascript:`;
  - uppercase scheme;
  - `#top` (no finding);
  - `mailto:` and `tel:` (no finding);
  - `http://localhost:3000/x` and `http://127.0.0.1/` (no http finding);
  - `notbit.ly` and `bit.ly.evil.test` (no shortener finding);
  - `https://trusted.test@evil.test` (host `evil.test`);
  - `paypal.com` → `paypal.com.evil.test` (fires);
  - `(https://paypal.com)`, `<https://paypal.com/>` and
    `"https://paypal.com/login"` → `paypal.com.evil.test` (fires);
  - `www.com` → `https://evil.com/` (no token, so no finding);
  - `example.com` → `www.example.com` (passes);
  - `Version 1.2` (no token);
  - an `@` token (skipped);
  - hidden anchors inside suppressed containers (no finding).
- **AC2 — Locations.** For a UTF-8 or us-ascii part, `byte_offset` equals
  the index of the href's first non-whitespace source byte in the
  transfer-decoded payload, for quoted, unquoted, entity-bearing and
  whitespace-led values, under base64 and quoted-printable encoding, and
  when the payload carries trailing ASCII whitespace after the decoded
  text. The same link yields `mime_part {path}` in two cases:
  - the decoded text is not a prefix of the payload that is followed only
    by ASCII whitespace (for example `charset=iso-8859-1` with an 8-bit
    byte);
  - the transfer decode fails.
- **AC3 — Default off and lazy.** All of the following run against a
  counting loopback listener named by the mail's links, and it records
  **zero** accepted connections:
  - SMTP acceptance (`250`), the message read, the default report, the MCP
    report, the message page, `assert` and `await` with
    `no_findings_at_or_above`. This holds both on a default server and on
    a `loopback`-mode server.
  - `?reachability=true` on an `off` server, which returns 400 with the
    literal error.

  The default report contains no `link.redirect-chain.v1` or
  `link.reachability*` finding.
- **AC4 — Opt-in outcomes**, in `loopback` mode against local servers.
  Assertions are on outcome classes and literal findings, never on wall
  time:
  - 200 gives no reachability finding;
  - 404 gives `status` with `HTTP 404`;
  - two redirects then 200 gives `redirect-chain` with `2 time(s)` and the
    final `http://127.0.0.1:<port>` origin;
  - an endless 302 gives `redirect_cap` after exactly 6 requests;
  - a 302 without `Location` gives `bad_redirect`;
  - a 302 to `ftp:` gives `bad_redirect`;
  - an accept-and-never-answer server gives `timeout`;
  - a closed port gives `connect`;
  - a server that sends headers with a large `Content-Length` and no body
    gives `ok` (the body is not read).
- **AC5 — Address policy (default-deny).**
  - *A pure table test* covers both modes and pins `off` as refusing
    everything. Every row is a literal written in the test, never computed:
    - **refused:** every IPv4 block of rule 4, at its first and last
      address;
    - **refused:** the IPv6 addresses `::`, `::a00:1` (IPv4-compatible),
      `::ffff:0:a00:1` (IPv4-translated), `64:ff9b::a00:1`,
      `64:ff9b:1::a00:1`, `100::1`,
      `1fff:ffff:ffff:ffff:ffff:ffff:ffff:ffff` (the last address below
      `2000::/3`), `2001::1`, `2001:0:4136:e378::1` (Teredo), `2001:2::1`,
      `2001:1ff:ffff:ffff:ffff:ffff:ffff:ffff` (the last address of
      `2001::/23`), `2001:db8::1`, `2002:a00:1::1`, `3fff::1`,
      `3fff:fff:ffff:ffff:ffff:ffff:ffff:ffff` (the last address of
      `3fff::/20`), `4000::`, `5f00::1`, `fc00::1`, `fe80::1`, `fec0::1`
      and `ff02::1`;
    - **permitted** in `public` and `loopback`: `2000::1`, `2001:200::1`,
      `2001:db7:ffff::1`, `2001:db9::1`, `2003::1`, `3ffe::1`,
      `3fff:1000::1`, `2606:4700:4700::1111`, `8.8.8.8`, `::ffff:8.8.8.8`,
      `192.31.196.1` and `192.88.98.255`;
    - **mode rows:** `127.0.0.1`, `127.255.255.255`, `::1` and
      `::ffff:127.0.0.1` are permitted only in `loopback`.

    No address that the old table pinned as permitted (`fec0::1`,
    `fbff::1`, `64:ff9a::1`, `64:ff9c::1`) is still permitted.
  - *Resolver seam, embedded-IPv4 answer:* in `loopback` mode, a hostname
    that the seam resolves to `[<listener 127.0.0.1>, 64:ff9b:1::a00:1]`
    gives `refused`, and the listener records zero accepts. The same holds
    for `[<listener>, 2002:a00:1::1]`. The permitted local address comes
    first on purpose: a broken check would dial only the local listener,
    never an outside address.
  - *Redirect into a special-purpose literal:* in `loopback` mode, a
    loopback hop that redirects to `http://[fec0::1]/` gives `refused`,
    after exactly one accepted connection.
  - *Wire, `public` mode:* a `127.0.0.1` link gives `refused` and the
    listener records zero accepts.
  - *Wire, both modes:* a `user:pass@127.0.0.1` link gives `refused`.
  - *Resolver seam, mixed answer:* in `loopback` mode, a hostname that the
    seam resolves to `127.0.0.1` and `10.0.0.1` gives `refused`, and the
    listener records zero accepts.
  - *Resolver seam, pinning:* in `loopback` mode, a hostname that the seam
    resolves to the listener's address connects to that listener and sends
    the hostname as `Host`.
  - *Resolver seam, failure:* a resolver error gives `connect`.
  - *Redirect re-validation:* in `loopback` mode, a loopback hop that
    redirects to `http://10.0.0.1/` gives `refused`, after exactly one
    accepted connection. No connection to `10.0.0.1` is attempted.
- **AC6 — Bounds.**
  - 33 distinct targets give 503 `deliverability analysis failed` with zero
    accepts.
  - Per-request concurrency is exactly 4, proven by **counting
    connections, not timing them**. The test server holds every connection
    open without answering, and keeps an open-connection gauge that rises
    on accept, falls when the peer closes, and records its high-water mark.
    Five distinct targets on that server give a high-water mark of
    exactly 4, 5 accepts in total and five `timeout` findings. No
    assertion compares timestamps or elapsed time.
  - Links to `<url>#a` and `<url>#b` in one message are one target and
    give one accept.
  - A third concurrent probe request gets 503
    `deliverability analysis unavailable` while two are held.
  - While two probe requests are held on hanging targets, a fresh SMTP
    `250`, its message read and a cold default report all complete. After
    the cold report returns, the test asserts that **both held probe
    requests are still pending** (`!is_finished()`). A probe that held a
    Store guard or a job permit would turn that test red. The unit
    assertion `jobs.available_permits() == JOBS` stays.
  - The guide documents every constant.
- **AC7 — Determinism and cache.**
  - The default report is byte-identical cold, warm and across two
    `Service`s.
  - The composer is pure: the same deterministic report, occurrences and
    frozen observation give byte-identical output twice, whatever order the
    observation was built in.
  - A probed request reuses, and does not replace, the deterministic slot:
    a default request after it is a ready-only cache hit.
  - Slot inputs gain no member.
- **AC8 — Limits.** Static or composed findings beyond 256, or a report
  beyond 64 KiB, give `Failed`, never a truncated report.
- **AC9 — Shortener list.**
  - The list is sorted, lowercase, unique and has no trailing dots.
  - A snapshot test pins its content digest to `SHORTENERS_VERSION` and
    `RULE_SET_ID`.
  - One listed and one unlisted host are each tested.
- **AC10 — Versioning.** `RULE_SET_ID` and `html-rules.json` are
  `swarmail-html/4`, and the catalogue snapshot fixture and every `/3`
  literal move with them.
- **AC11 — Ingest unchanged.**
  - `email.links`, `link_metadata`, codes and persisted rows are
    byte-identical before and after this change for the link fixtures.
  - The existing ingest/extract tests pass unmodified, apart from the
    visibility-only change in `extract.rs`.
  - The production files `src/smtp.rs`, `src/model.rs` and
    `src/store/persist.rs` carry no diff.
  - A test harness may gain the one-line `Config::link_reachability`
    literal, as `src/store/persist_tests.rs` does, and nothing else.
- **AC12 — Dependencies.** `git diff main -- Cargo.toml Cargo.lock
  evals/Cargo.lock deny.toml` is empty.
- **AC13 — Records.**
  - 0121 is present with `- **Status:** proposed` and an index row in
    `docs/decisions/README.md`.
  - No other decision's status changes.
  - `docs/guide/deliverability.md` documents:
    - the rules and the http ruling;
    - the revised token steps;
    - the shortener list;
    - both modes;
    - the default-deny address policy, with its basis date and the
      `2000::/3` residual;
    - every bound, stated per `Service`;
    - the loopback-mode risk.
  - The guide, `llms.txt` and 0121 state the location test as a prefix
    followed only by trailing ASCII whitespace. None of them calls it
    "byte-identical".
  - OpenAPI and `llms.txt` describe the `reachability` parameter and the
    400.
  - These surfaces each name the per-link rules (`link.*`) beside the
    demo, bulk-sender and HTML rules:
    - the OpenAPI deliverability route and `DeliverabilityReport`
      descriptions;
    - the modern MCP report tool description;
    - `README.md`;
    - `docs/guide/README.md`.
  - `docs/guide/network-posture.md` names the opt-in outbound probe, the
    default-deny policy and the resolver residual.
- **AC14 — Gates.** Every gate in `docs/house-rules.md § Gates` passes
  `--locked`:
  - the evals crate against the candidate binary;
  - the brokkr compile loop, with each recipe's output read for the
    policy-sweep warning;
  - the coverage gate at exactly 100% and its selftest;
  - the publish-guard and seal proofs;
  - `cargo deny check licenses`.

  CI-only proofs (macOS, RustSec, Docker, Helm, port-80) are named as
  CI-owned and not claimed. A load proof, if any, is bounded to ≤ nproc/4
  workers and ≤ 10 minutes, as a named, timed command, with the load
  reported.

- **AC15 — The disposition reaches the merge.** The slice-2 hold is not on
  0056's mechanical hold list, so it is written where the merging
  assistant reads it:
  - `implementation.md` § Disposition says that the static slice may be
    approved by name under 0045 §8, and that the reachability slice merges
    only after the operator accepts 0121;
  - the result handed to ship says the same and asks for it in the PR
    body;
  - 0121 stays `proposed`.

## Non-goals

These are out of scope for this story:

- text/plain URLs;
- `<img>`, `<area>` and CSS URLs;
- IDN or homograph detection;
- public-suffix logic;
- reachability over MCP or the UI;
- caching probe results;
- a per-deployment allowlist;
- a `reachability` predicate;
- any change to SMTP, ingest, persistence or the wait loop.
