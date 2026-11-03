# 0121 — Per-link deliverability findings and an opt-in, bounded reachability probe

- **Status:** proposed
- **Date:** 2026-11-03
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
   0085 §1 only when the part's character-decoded text is byte-identical to
   its transfer-decoded payload, so the collector checks this per part. When
   the check fails, every link finding of that part uses `mime_part {path}`,
   0085's own fallback. Swarmail never fabricates an offset.
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
     `public` permits only globally routable unicast addresses. `loopback`
     also permits `127.0.0.0/8` and `::1`, for local development servers.
     Private, link-local (which includes `169.254.169.254`), CGNAT, ULA,
     unspecified, broadcast, multicast, documentation and reserved ranges
     are refused in every mode. Swarmail unwraps IPv4-mapped IPv6 before the
     check. A target with userinfo is refused.
   - *Bounds are constants, not knobs.* Connect timeout is 2 s. The per-link
     budget is 5 s, covering every hop. The redirect cap is 5, followed
     manually. At most 32 distinct targets are probed per request; more is
     an explicit failure before any connection. Each request probes at most
     4 links at once, and at most 2 probe requests run per process; a busy
     process answers `Unavailable`. The response body is never read. The
     probe uses no proxy, cookies or credentials, and a fixed `User-Agent`.
     Worst case per request: ⌈32/4⌉ × 5 s = 40 s and 8 open sockets. No
     probe ever holds one of 0085's four job permits or a Store guard.
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
