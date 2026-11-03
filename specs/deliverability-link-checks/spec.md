# Deliverability link checks — specification

Date: 2026-11-03. Epic #251, issue #258. Run
`issue-258-deliverability-link-ch-d9779f5d`, from `main` `8732445`.

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

`.brokkr/state/tasks/` is absent in this worktree. The framing is the task
brief, the intake result (14 criteria) and accepted 0085, 0106 and 0107.
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
- the label is capped at #154's character cap.

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

This offset is only valid when the part's character-decoded text equals its
transfer-decoded bytes byte for byte. If they differ, every link finding in
that part uses `mime_part {path}`, 0085 §1's fallback. Swarmail never
emits a guessed offset.

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

**Host-like tokens.** Split the label on ASCII whitespace. For each token:

1. Skip the token if it contains `@` or any non-ASCII character.
2. Remove a leading `http://` or `https://`, compared case-insensitively.
3. Cut at the first `/`, `?`, `#` or `:`.
4. Trim leading `(` `[` `<` `"` `'` and trailing `.` `,` `;` `:` `!` `?` `)` `]` `>` `"` `'`.
5. Lowercase the token.

The token qualifies if it has at least two dot-separated labels, every label
is non-empty `[a-z0-9-]`, and the last label is alphabetic with at least two
characters. So `v1.2`, `3.14` and `e.g.` do not qualify.

**Matching.** Take H as the target host, lowercased, with one trailing dot
removed. IP-literal hosts are compared as written. Strip one leading `www.`
from H and from T. T matches when H == T or H ends with `.`+T. So
`paypal.com` → `paypal.com.evil.test` fires, and `example.com` →
`www.example.com` or `click.example.com` passes.

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
message's eligible links, by `Url::as_str`. More than **32** distinct
targets is `AnalysisError::Failed` (503, the existing fixed message) before
any connection.

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

**Address policy.** IPv4-mapped IPv6 is unwrapped first.

- *Never permitted:* `0.0.0.0/8`, `10/8`, `100.64/10`, `169.254/16`,
  `172.16/12`, `192.0.0/24`, `192.0.2/24`, `192.168/16`, `198.18/15`,
  `198.51.100/24`, `203.0.113/24`, `224/4`, `240/4` (which includes
  broadcast), `::`, `fc00::/7`, `fe80::/10`, `ff00::/8`, `2001:db8::/32`.
- *Loopback* (`127/8`, `::1`): permitted only in `loopback` mode.
- Everything else is permitted in both `public` and `loopback`.

**Concurrency.** Each request probes at most **4** targets at once. At most
**2** probe requests run per process; a third is `AnalysisError::Unavailable`
(503). Probes run on the async runtime and never hold one of 0085's four job
permits or a Store guard. The one occurrence scan before probing runs on
`Store::offload` under a job permit, which is released before any
connection. Dropping the request cancels its probes and releases its permit.

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
  - `example.com` → `www.example.com` (passes);
  - `Version 1.2` (no token);
  - an `@` token (skipped);
  - hidden anchors inside suppressed containers (no finding).
- **AC2 — Locations.** For a UTF-8 or us-ascii part, `byte_offset` equals
  the index of the href's first non-whitespace source byte in the
  transfer-decoded payload, for quoted, unquoted, entity-bearing and
  whitespace-led values, under base64 and quoted-printable encoding. For a
  part whose decoded text differs from its bytes (for example
  `charset=iso-8859-1` with an 8-bit byte), the same link yields
  `mime_part {path}`.
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
- **AC5 — Address policy.**
  - *A pure table test* covers every listed range, each in both modes,
    including mapped IPv6 and the boundary addresses.
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
  - Per-request concurrency is exactly 4. One listener holds every
    connection open without answering. Five distinct targets on it give a
    maximum of exactly 4 simultaneously open connections, 5 accepts in
    total, and five `timeout` findings.
  - A third concurrent probe request gets 503
    `deliverability analysis unavailable` while two are held.
  - While two probe requests are held on hanging targets, a fresh SMTP
    `250`, its message read and a cold default report all complete. That
    proves the probe holds no job permit and no Store guard.
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
  - `src/smtp.rs`, `src/model.rs` and `src/store/persist*.rs` carry no diff.
- **AC12 — Dependencies.** `git diff main -- Cargo.toml Cargo.lock
  evals/Cargo.lock deny.toml` is empty.
- **AC13 — Records.**
  - 0121 is present with `- **Status:** proposed` and an index row in
    `docs/decisions/README.md`.
  - No other decision's status changes.
  - `docs/guide/deliverability.md` documents the rules, the http ruling,
    the shortener list, both modes, every bound and the loopback-mode risk.
  - OpenAPI and `llms.txt` describe the `reachability` parameter and the
    400.
  - `docs/guide/network-posture.md` names the opt-in outbound probe.
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
