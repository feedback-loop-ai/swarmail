# 0121 — Per-link deliverability findings and an opt-in, bounded reachability probe

- **Status:** accepted (operator, 2026-11-06 — "every decision"; see the
  dated acceptance note at the end)
- **Date:** 2026-11-03
- **Revised:** 2026-11-04, still proposed. Review of `33efa4c` returned the
  design (run `issue-258-deliverability-link-ch-4052800f`). §6's address
  policy is now default-deny; §3's location test states what the code
  checks; §6's caps are per `Service`, and it records the resolver
  residual. See the [plan's revision
  table](../../specs/deliverability-link-checks/plan.md#revision-2026-11-04--answering-the-review).
- **Scope:** Epic #251, issue #258 (operator direction 2026-11-03: start the
  remaining deliverability stories). Run
  `issue-258-deliverability-link-ch-d9779f5d`, from `main` `8732445`. The
  dispatcher assigned the number. It is not taken from the index.
- **Specification:** [deliverability link checks](../../specs/deliverability-link-checks/spec.md).
- **Builds on:** accepted [0085](0085-deliverability-report.md) (the report
  contract, its cache, its limits and its §8 observation seam), accepted
  [0106](0106-html-client-compatibility.md) / [0107](0107-html-review-repair.md)
  (the HTML part walk and its budgets), and #154's anchor-label rules
  (decision [0094](0094-focused-mail-waits.md) §1).
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) (no link work on
  SMTP, in `Store::insert`, seed, restore or fan-out),
  [0002](0002-awaits-subscribe-before-checking.md) (predicates never probe;
  the wait loop is untouched), [0003](0003-oldest-only-pruning.md) (no
  retention change), [0005](0005-extraction-at-ingest.md) (ingest
  extraction and stored rows are byte-identical; the report re-reads stored
  bytes lazily, as 0106 does), and the 0085 schema `{rule_set_id, findings}`
  with its finding fields, ordering and limits.
- **Amends:** nothing in the static half. On the opt-in probe, see §7.
  If the operator reads 0085's rejected "policy networking" alternative or
  0106 §1's "no runtime network access" as covering an operator-enabled
  probe outside the evaluator, accepting this record narrows those
  sentences to the deterministic evaluator, which stays network-free.

## Decision proposed

1. **Static link rules run in the default report.** A pure collector,
   `src/deliverability/links.rs`, reads the same eligible outer-message
   `text/html` parts as the 0106 walk. It reuses #154's lexical walk and
   label rules from `src/extract.rs`; it adds no new parser and no regex
   extractor. For every visible `<a>` start tag with an `href` it emits
   findings from this closed table:

   | rule id | severity | fires when |
   |---|---|---|
   | `link.scheme.unsafe.v1` | error | the parsed scheme is `javascript`, `vbscript`, `data` or `file` |
   | `link.url.invalid.v1` | warning | the decoded `href` is empty, or is not an absolute URL (relative path, `//host`, an unrendered `{{placeholder}}`, an invalid host); fragment-only `#…` is exempt |
   | `link.scheme.http.v1` | warning | the scheme is `http` and the host is not `localhost`, `*.localhost`, `127.0.0.0/8` or `::1` |
   | `link.shortener.v1` | warning | an `http`/`https` host, with one trailing dot removed, equals an entry of the built-in shortener list |
   | `link.anchor-host-mismatch.v1` | warning | the anchor label names at least one host-like token, and the `http`/`https` target host neither equals one of them nor is a subdomain of one, after one leading `www.` is stripped from each side |

   Other schemes (`https`, `mailto`, `tel`, `cid`, `sms`, `ftp`, app
   schemes) produce no scheme finding.
2. **Plain `http` is a warning, never an error** (the "decide and document"
   item). Swarmail does not upgrade, rewrite or fetch the link in the default
   report. Loopback and `localhost` targets are exempt, because Swarmail is a
   development mail catcher and `http://localhost:3000/confirm` is its normal
   case. There is no allowlist and no knob.
3. **Locations are source bytes, or the containing part.** A finding's
   location is `link {part_path, byte_offset}`, where `byte_offset` is the
   first non-ASCII-whitespace source byte of the `href` value. That matches
   0085 §1 only when the part's character-decoded text is a prefix of its
   transfer-decoded payload and the rest of the payload is ASCII
   whitespace, so the collector checks this per part. Under that test every
   decoded offset indexes the same payload byte. When the check fails,
   including a failed transfer decode, every link finding of that part uses
   `mime_part {path}`, 0085's own fallback. Swarmail never fabricates an
   offset.
4. **The shortener list is built in, versioned and exact.** It is a sorted
   Rust constant with version `swarmail-shorteners/1` and basis date
   2026-11-03. It is Swarmail-authored and was not fetched. A snapshot test
   ties the list's content to that version and to `RULE_SET_ID`, so any list
   edit must move both. Matching is exact on the host. There is no suffix or
   substring match and no network lookup.
5. **One rule-set version change.** `RULE_SET_ID` and `html-rules.json` move
   `swarmail-html/3` → `swarmail-html/4` under 0085 §1, so cached
   `/3` slots miss. Siblings #253 and #260 also bump the id. The integrator
   takes the next free version at merge, and the snapshot fixtures record it.
6. **Opt-in HTTP reachability is a separate, bounded observation.** It is
   never part of the cached report and never runs inside `evaluate`:
   - *Enablement takes two explicit steps.* First, the operator starts the
     server with `--link-reachability <off|public|loopback>`
     (`SWARMAIL_LINK_REACHABILITY`, default `off`). Second, a caller asks
     with `GET /api/v1/messages/{id}/deliverability?reachability=true`. On an
     `off` server that query is a `400` that opens no connection. MCP, the
     message page, assert and await never probe.
   - *The address policy is checked on every hop.* Swarmail resolves the
     host itself and refuses a target if **any** resolved address is not
     permitted. It pins the connection to the first validated address.
     The policy is **default-deny**: an address is contacted only if the
     policy proves it globally routable unicast. The basis is the IANA IPv4
     and IPv6 Special-Purpose Address Registries as read on 2026-11-04.
     IPv6 is permitted only inside `2000::/3`, and not inside `2001::/23`,
     `2001:db8::/32`, `2002::/16` or `3fff::/20`. Everything outside
     `2000::/3` is refused without being named. IPv4 is permitted only
     outside every non-globally-reachable registry entry, and the spec
     lists those blocks. IPv4-mapped IPv6 is unwrapped and judged as IPv4.
     Every other form that embeds IPv4 (NAT64, 6to4, Teredo,
     IPv4-compatible, IPv4-translated) is refused and never decoded.
     `loopback` mode additionally permits `127.0.0.0/8` and `::1`, for
     local development servers. A target with userinfo is refused.
     `std`'s `is_global` is not used, because its table moves with the
     toolchain. Residual: a future special-purpose block inside `2000::/3`
     is permitted until the table is edited.
   - *Bounds are constants, not knobs.* Connect timeout is 2 s. The per-link
     budget is 5 s, covering every hop. The redirect cap is 5, followed
     manually. At most 32 distinct targets are probed per request; more is
     an explicit failure before any connection. Each request probes at most
     4 links at once, and at most 2 probe requests run per `Service` (one
     per production process); a busy `Service` answers `Unavailable`. The response body is never read. The
     probe uses no proxy, cookies or credentials, and a fixed `User-Agent`.
     Worst case per request: ⌈32/4⌉ × 5 s = 40 s and 8 open sockets. No
     probe ever holds one of 0085's four job permits or a Store guard.
     Targets are keyed with the fragment cleared, so one resource is
     fetched once.
   - *Resolver residual, accepted.* `tokio::net::lookup_host` runs on the
     blocking pool and cannot be cancelled. A probe dropped at its budget
     leaves the OS lookup running until the OS gives up. Repeated explicit
     requests for hostile names can therefore park blocking threads. The
     opt-in mode, the per-`Service` caps and the OS resolver deadline bound
     this. It costs latency, never a `250`. A dedicated bounded resolver
     would need its own decision.
   - *Outcomes become findings through the one canonicalizer.*
     `link.redirect-chain.v1` (info) discloses the redirect count and the
     final origin (scheme, host and port; never a path, query or userinfo).
     `link.reachability.v1` (warning) reports one fixed outcome class:
     timeout; connection, DNS or TLS failure; HTTP status ≥ 400 (with the
     code); redirect cap exceeded; or an invalid or non-HTTP redirect.
     `link.reachability.refused.v1` (info) reports a target the address
     policy did not contact. The probed response composes the cached
     deterministic report with these findings under 0085's 256-finding and
     64 KiB limits. Exceeding a limit is a failure, never truncation.
   - *Determinism is stated honestly.* The default report is byte-identical
     for the same bytes and configuration, as before. The probed response is
     a pure function of the bytes, the configuration and the frozen
     observation, and it is not cached. Two probes at different times may
     differ. That is the point of asking.
7. **Relation to accepted rulings.** The evaluator stays free of network,
   clock and resolver access, exactly as 0085's specification and 0106 §1
   require. The probe enters through 0085 §8's seam: an explicit, frozen
   observation input outside the evaluator. Because a broad reading of
   0085's "policy networking" rejection could cover it, the reachability
   slice merges only after the operator accepts this record (0045 §8, as
   [0118](0118-mailslurper-shim.md)'s PR was held). The static slice is
   compatible implementation either way. The two slices are separate
   commits, so the operator may approve the static slice by name.

## Alternatives ruled and costs

- **Defer reachability to a later run** (simplicity position): rejected. The
  operator asked for it in this story. Deferring leaves required work undone
  while claiming completion. Its premise, that the probe must run inside
  `evaluate` and hold the job pool, is avoided by §6. Cost accepted: more
  security surface for review, and the whole PR waits for acceptance unless
  the operator approves the static slice by name.
- **Read ingest's stored `LinkMetadata`:** rejected. It stores only http(s),
  and it has no offsets or part paths, so unsafe schemes could never fire and
  0085 §1 locations are impossible. Storing offsets would change ingest
  (0005/0094) and the persisted format.
- **A reachability boolean in `AnalysisConfig` with `reqwest` inside
  `evaluate`:** rejected. It makes the cached report non-deterministic,
  holds the four blocking permits across network waits, and lets every
  predicate and page view probe.
- **Cache probe results in the slot** (intake AC6): rejected. A cached
  "unreachable" would answer the next explicit check with stale data, and a
  per-observation key would make every slot single-use. The deterministic
  part stays cached.
- **A `cfg(test)`-only loopback seam with no loopback mode** (robustness
  position): rejected for a production `loopback` mode. A development catcher
  must be able to check `localhost` links. The mode also lets wire tests run
  through public code, with no seam. Cost: an operator who chooses
  `loopback` lets mail content cause GETs to local ports (no body read, no
  credentials). The guide says so.
- **A JSON shortener catalogue with its own loader** (intake AC9): rejected
  for a constant plus version plus snapshot test. It has no parse or
  validation paths, and the version cannot drift from the content.
- **Full-host anchor comparison** (robustness): rejected for
  equal-or-subdomain after `www.` stripping. Without a public-suffix list
  (no new dependency), full-host comparison flags `example.com` →
  `click.example.com`, the most common legitimate pattern. Cost: a label
  that names a shared suffix (`github.io`) lets any of its subdomains pass.
- **Per-message link caps in the default report, an 8 KiB URL cap, and
  `<img>`/`<area>`/CSS/text-plain URLs:** rejected for this story. Input is
  already bounded by 0106's limits and output by 0085's. Text/plain URLs are
  the first follow-up.
- **Extend the IPv6 block list with the ranges review found** (the
  2026-11-04 return): rejected for default-deny. The block list missed six
  ranges in six entries, and every future allocation would be a bypass by
  default. An allow-list fails closed. Cost: a host whose only addresses
  are 6to4, Teredo or NAT64 gets `refused`. That is a visible "not
  checked", never a false "safe".
- **Decode embedded IPv4 (6to4, Teredo, NAT64) and judge it:** rejected.
  It adds four decoders and their boundary fixtures to the exact coverage
  gate, only to permit rare transition-only hosts.
- **Block the globally reachable anycast /24s inside the IPv4 registry**
  (`192.31.196/24`, `192.52.193/24`, `192.175.48/24`): rejected. They are
  global unicast, and refusing them is no SSRF gain. `192.88.99/24`, the
  deprecated 6to4 relay, *is* refused, to match `2002::/16`.
- **One `reqwest::Client` per request instead of per hop:** rejected. Each
  hop is pinned to its own validated address through `resolve_to_addrs`.
  Sharing a client would need a custom resolver to keep that pin. The cost
  is bounded (at most 32 × 6 builds) and accepted.
- **IDN and homograph label comparison:** out of scope. Non-ASCII label
  tokens are skipped, and the guide documents this gap.

## Dependency, proof and disposition

No new crate. `Cargo.toml`, both lockfiles and `deny.toml` are unchanged.
`reqwest` (rustls, existing features) and `tokio` (`net`, `time`, `sync`)
suffice. Every rule has a passing and a failing fixture. Reachability is
proven only against local servers on `127.0.0.1` (and `::1` where the host
has it). No test resolves a public name or touches the internet. The paired
tasks and the gate inventory are in the [specification](../../specs/deliverability-link-checks/spec.md)
and its [tasks](../../specs/deliverability-link-checks/tasks.md). This
record stays proposed until the operator rules. No seat accepts it.

## Integration note — 2026-11-05 (merge of `main` after #260)

`main` took `swarmail-spam/1` for #260's spam signals (proposed
[0122](0122-deterministic-spam-rules.md)) while this branch held
`swarmail-html/4`. Under §5 and 0122 §1.4 the branch integrating second takes
a fresh id. `swarmail-html/4` has never shipped on `main` or in a release, so
it is that fresh id: the merged report carries the spam signals and the five
link rules under `swarmail-html/4`. No rule id, severity, hint or location of
either story changes. This note changes no status; the record stays proposed.

## Integration note — 2026-11-05 (proposed 0125, issue #254)

Section 7's "free of network, clock and resolver access" holds for the link
probe and the static link rules. Under proposed
[0125](0125-offline-dkim-verification.md) section 8 the report evaluator as a
whole now holds one reachable crate-internal clock read: `mail-auth`'s `x=`
comparison (operator hold 2). The crate's RFC 6651 report-sampling clock read
is unreachable, because Swarmail never hands the crate a report record. A
hickory resolver is compiled in and unreachable (hold 3). This note changes no
status: the record stays proposed, and section 7's text is not edited.

## Dated acceptance note — 2026-11-06

The operator ruled on 2026-11-06, in chat to the dispatcher, "every decision": every decision on `main` that was still proposed is accepted in full. That covers 0119, 0120, 0121, 0122, 0123, 0124 and the rest of 0125, whose three holds the operator had accepted earlier the same day. This record was proposed when written; the text above is
unchanged.

- **The merge preceded the acceptance.** §7 held the reachability slice
  for the operator's acceptance, but #328 merged both slices on
  2026-11-05, a day before this ruling: the merge pipeline detected only
  dependency holds, not a 0045 §8 hold. This acceptance legitimises that
  merge after the fact and lifts the 0045 §8 hold on the reachability
  slice. The static slice was compatible work either way.
- **It narrows two accepted sentences to the deterministic evaluator**, as
  the header and §7 frame it: accepted
  [0085](0085-deliverability-report.md)'s rejected "policy networking"
  alternative, and accepted
  [0106](0106-html-client-compatibility.md) §1's "no runtime network
  access". The evaluator stays network-free; the operator-enabled probe is
  an observation outside it, entered through 0085 §8's seam. 0085 and 0106
  carry dated pointers to this note.
- The 2026-11-05 integration note on 0125 reads with accepted 0125 §8: the
  evaluator's one reachable clock read is `mail-auth`'s `x=` comparison.
