# Deliverability link checks — plan

Spec: [spec.md](spec.md). Decision: [0121](../../docs/decisions/0121-deliverability-link-checks.md)
(proposed). Base: `main` `8732445`. Revised 2026-11-04 on top of `33efa4c`
after review returned the design; see [§ Revision
2026-11-04](#revision-2026-11-04--answering-the-review). The structure and
data model below describe the shipped shape at `33efa4c` plus the
revision, so they no longer describe the pre-implementation plan of
`a1977dc`.

## Technical context

- **Language and crate:** Rust, single crate. Evaluator: `src/deliverability.rs`,
  `src/deliverability/html.rs`. The anchor walk is `src/extract.rs`
  (`walk`, `Regions`, `AnchorAttrs`, `decode_refs`, `LABEL_BREAKS`, `space`).
- **Existing dependencies used:** `mail-parser` (MIME), `reqwest` 0.12 with
  `rustls-tls` (`Url`, `Client`, `redirect::Policy`, `resolve_to_addrs`),
  `tokio` (`net::lookup_host`, `time::timeout`, `sync::Semaphore`, `task::JoinSet`),
  `serde_json`, `clap`. **No new crate and no new feature.**
- **Performance bounds.** The static rules add one linear `walk` per
  eligible part, already bounded by 0106's 8 MiB raw, 4 MiB aggregate, 256
  parts and depth 32, and one byte comparison per part. The probe's bound is
  arithmetic: ≤ 32 targets, 4 at a time, 5 s each, so ≤ 40 s and 8 sockets
  per `Service` (2 requests; one `Service` per production process).
- **Testing.** Inline `#[cfg(test)]` modules in the new files, with real
  local TCP servers written on `tokio::net::TcpListener`. The static rule
  wire test is in `tests/deliverability.rs`. The probe wire tests are in
  `tests/link_reachability.rs`, so they stay clear of the sibling stories
  that edit `tests/deliverability.rs`. Both use `common::start_with`.

## Constitution check (house rules)

| Rule | How this plan holds it |
|---|---|
| 0001 accept == stored | Nothing on `smtp.rs`, `Store::insert`, seed, restore or fan-out. AC3 proves zero probe connections across `250`. |
| 0002 subscribe before check | The wait/assert loop and `Store::qualify` are untouched. Predicates call only the default report. AC3 runs `await` on a loopback-mode server and gets zero accepts. |
| 0003 oldest-only pruning | No retention, row or cap change. The probe holds only its `Arc<Email>` snapshot. |
| 0005 extraction at ingest | Ingest output is byte-identical (AC11). `extract.rs` gains visibility only. |
| 0085 contract | Same schema, canonicalizer, limits and slot key. The `link` location is now emitted. The rule-set id moves. Probe output enters through the §8 shape: a frozen observation outside the evaluator. |
| 0045 §8 | The static slice is compatible. The reachability slice is separate commits, held for 0121's acceptance (decision §7). |
| 0020 placement | Inline tests by default. No `_tests.rs` harness is planned, because every branch is reachable by a deterministic local fixture. A harness is added only if a concrete cold line appears, and the implementer records why. Seams are `cfg(test)`-only and each one is installed by a test. |
| Coverage 100% exact, no `coverage(off)` | Every new branch has a named test in tasks.md. |
| Dependencies | No change to `Cargo.toml`, the lockfiles or `deny.toml` (AC12). |
| Tests never touch the internet | Static fixtures that name public hosts run with the probe off, and AC3 proves off opens nothing. Probe tests use `127.0.0.1`/`::1` and the resolver seam only. |

## Project structure (files touched)

- **new** `src/deliverability/links.rs`
  - `Anchors: Regions` yields `Occurrence { location, href: String,
    label: String }` to a callback; occurrences are not retained;
  - `scan(part, f)`, and the five static rules through
    `collect(part, out)`;
  - `SHORTENERS` and `SHORTENERS_VERSION`;
  - the host-token parser `token()`, whose step order the revision fixes
    (R3);
  - `targets(raw)`, the distinct target keys failing at the 33rd, and
    `observed(raw, observation)`, the findings for targets with outcomes
    failing at the 257th. Both use one `key(url)` that clears the fragment
    (R5);
  - the pure `compose(report, observed_findings) -> Result<Report>`;
  - inline tests.
- **new** `src/deliverability/reach.rs`
  - `Mode { Off, Public, Loopback }` (clap `ValueEnum`, serde lowercase);
  - `permitted(IpAddr, Mode)`, which the revision rewrites as default-deny
    (R1);
  - `Outcome { redirects: u8, final_origin: String, end: End }`, with
    `End { Ok, Status(u16), Timeout, Connect, RedirectCap, BadRedirect,
    Refused }`;
  - `Prober { mode, requests: Semaphore(2) }`, with
    `async fn observe(&self, targets) -> Result<Observation, AnalysisError>`,
    where `Observation = BTreeMap<String, Outcome>`;
  - a `#[cfg(test)]` resolver seam;
  - inline tests against local servers.
- `src/deliverability/html.rs`
  - `collect_with` calls `links::collect` per eligible part, after the
    catalogue scan;
  - eligible-part enumeration is factored into
    `fn eligible(raw) -> Result<Vec<Part>>`, so the probe's occurrence scan
    reuses the same walk (one MIME walk owner);
  - `Part` carries `path`, `body` and `exact`. `exact` is `is_exact`: the
    character-decoded text is a prefix of `html::payload` (the transfer
    decode through mail-parser's own decoders), and the rest of the
    payload is ASCII whitespace. `part.contents()` cannot serve, because
    for an HTML part it returns the charset-decoded text.
- `src/deliverability.rs`
  - `mod links; mod reach;`;
  - `RULE_SET_ID = "swarmail-html/4"`;
  - `Service` owns `prober: reach::Prober` and a new
    `pub(crate) async fn probed(&self, email) -> Result<Report, AnalysisError>`.
    It gets the default report through `with_inputs` (cached), takes an
    occurrence scan on `Store::offload` under a job permit, then
    `prober.observe`, then `links::compose`.
- `src/deliverability/html-rules.json`: `rule_set_id` → `swarmail-html/4`.
  The snapshot fixture in `html.rs` moves too.
- `src/extract.rs`: **visibility only** (`pub(crate)` on `walk`, `Regions`,
  `AnchorAttrs` fields, `TagScanStats`/`RefScanStats` as the trait needs,
  `space`, `LABEL_BREAKS`, the label-cap constant). There is no behaviour
  change.
- `src/store.rs`
  - `Store::report_probed(email)` delegates to `reports.probed`;
  - `Store::link_reachability()` reads the mode;
  - a builder `with_link_reachability(mode)`, used before the Store is
    shared.
- `src/config.rs`: `ServeArgs::link_reachability` (`--link-reachability`,
  env `SWARMAIL_LINK_REACHABILITY`, default `off`) and
  `Config::link_reachability`. Every `Config` struct literal names it:
  `src/lib.rs`, `tests/common`, `tests/persistence.rs`, `tests/stdio.rs`,
  `tests/listen.rs`, `src/lifecycle_tests.rs`,
  `src/compat/page_work_tests.rs`, `src/store/persist_tests.rs`,
  `src/webhook/ca_construction_tests.rs` and
  `src/deliverability/concurrency_tests.rs`. Literals only; no behaviour
  changes there.
- `src/lib.rs`: pass the mode to the Store at startup.
- `src/api.rs`: `get_deliverability` parses `DeliverabilityQuery {
  reachability: Option<bool> }` through the existing `parse_query`. It
  returns 404 first, then 400 `link reachability is not enabled` when the
  mode is `Off`, then `report_probed`.
- `src/mcp.rs`, `src/static/openapi.json`, `src/static/llms.txt`: the
  `swarmail-html/4` literals, plus the `reachability` parameter and its 400
  in OpenAPI and `llms.txt`. The MCP tool schema is unchanged.
- `tests/deliverability.rs`: the static wire test (AC1, AC2).
  `tests/link_reachability.rs`: the probe wire tests (AC3–AC6), listed in
  `docs/guide/testing.md`. `tests/bulk_sender.rs` and
  `tests/config_cli.rs` get the `/4` literal and the flag test.
- Docs: `docs/guide/deliverability.md` gets "Link rules" and "Opt-in
  reachability" sections. `docs/guide/network-posture.md` gets the
  outbound probe and the `loopback` risk. `docs/guide/rest-api.md` gets the
  parameter. `docs/decisions/README.md` gets the 0121 index row.

These are **not touched:** the production files `src/smtp.rs`,
`src/model.rs`, `src/store/persist.rs` and `src/webhook.rs`, plus
`Cargo.toml`, `Cargo.lock`, `evals/`, `deny.toml`, `.github/`, `scripts/`
and `.brokkr/`. Test harnesses such as `src/store/persist_tests.rs` gain
only the one-line `Config` literal.

**Revision touches (R1–R12 below):** `src/deliverability/reach.rs`
(policy, table, seam rows, connection gauge, guard taxonomy),
`src/deliverability/links.rs` (`token()` order, target key, test rename),
`tests/link_reachability.rs` (pending assertion, fragment case),
`src/static/openapi.json`, `src/mcp.rs`, `src/static/llms.txt`,
`README.md`, `docs/guide/README.md`, `docs/guide/deliverability.md`,
`docs/guide/network-posture.md`, and
`specs/deliverability-link-checks/implementation.md`.

## Research

- **The stored `LinkMetadata` cannot serve.** `URL_RE` stores only http(s)
  occurrences, so `javascript:` and relative hrefs are absent. Rows carry
  no part path or offset (verified in `src/extract.rs` and `src/model.rs` at
  `8732445`). The report therefore re-walks stored bytes lazily, as 0106
  does.
- **`extract::walk` hands consumers the source span of an `a` tag's first
  `href`** (`AnchorAttrs.href`), visible-only element events and
  unsuppressed text runs. `LinkRegions` builds the 0094 label from exactly
  these. A second `Regions` consumer reuses the tokenizer and the label
  semantics without forking them, so parity holds by construction.
- **`walk` offsets index `text_contents()`.** That is the transfer-decoded
  payload only when charset decoding is the identity, so the per-part
  `exact` comparison decides between `link` and `mime_part`.
- **`html.rs::Scan` fails on incomplete constructs before links run**, so
  the walk sees only bodies the catalogue accepted. A body that fails stays
  a failed report, as today.
- **`reqwest::Url`** implements the WHATWG parse: tab and newline
  stripping, scheme and host lowercasing, IDNA. `ClientBuilder` offers
  `no_proxy`, `redirect(Policy::none())`, `connect_timeout` and
  `resolve_to_addrs` without new features. `Response` is dropped unread
  after the status and headers.
- **`tokio::net::lookup_host`** is in the existing `net` feature and runs
  `getaddrinfo` on tokio's blocking threads, not under 0085's job permits.

## Data model

```text
Occurrence { location: Location, href: String, label: String }
Mode       = Off | Public | Loopback
End        = Ok | Status(u16) | Timeout | Connect | RedirectCap | BadRedirect | Refused
Outcome    { redirects: u8, final_origin: String, end: End }   // no durations, no clocks
Observation = BTreeMap<String /* key(url): Url::as_str, fragment cleared */, Outcome>
```

No new persisted state, cache member or `Email` field.

## Contracts

- REST `GET /api/v1/messages/{id}/deliverability[?reachability=true|false]`:
  - 200: the report schema, unchanged;
  - 404: unchanged;
  - 400: `link reachability is not enabled`, or the existing query refusal;
  - 503: the existing `deliverability analysis failed` /
    `deliverability analysis unavailable`.
- The CLI flag, the env variable and the `Config` field are as in the spec.
- MCP, UI, assert and await keep their current contracts.

## Quickstart (for the guide)

```bash
swarmail serve --link-reachability loopback
curl 'http://127.0.0.1:8025/api/v1/messages/<id>/deliverability?reachability=true'
```

## Revision 2026-11-04 — answering the review

Review of `33efa4c` returned the design (`spec_defect: true`, one HIGH
security residual). Each finding is answered in the artifact that owns it.
The ids in the last column match the tasks in [tasks.md § Slice
R](tasks.md#slice-r--revision-2026-11-04-answering-the-review).

| finding (lens) | change | owner | task |
|---|---|---|---|
| **HIGH: address policy is a block list.** `fec0::/10`, `64:ff9b:1::/48`, `2001::/32`, `2002::/16`, `::/96` and `::ffff:0:0:0/96` were reachable in `public` mode, and the table pinned `fec0::1` as permitted (security, guarantees). This is the spec defect. | R1: default-deny. IPv6 is allowed only inside `2000::/3` minus four blocks. IPv4 is the complement of the registry, plus `192.88.99/24`. Embedded IPv4 is refused, never decoded. The table is rewritten to literal rows, with wire rows that fail safe. | spec § Address policy, AC5; 0121 §6 | R1, R2, R3 |
| LOW: 0121 §3 says "byte-identical", but `is_exact` accepts a prefix plus trailing ASCII whitespace (guarantees) | R4: the **text** is amended to match the code, and the code does not change. Offsets stay valid, which every lens agreed. | spec § Location, AC2; 0121 §3; guide, `llms.txt` | R4 |
| LOW: the label is "capped at #154's cap", but no such cap exists (guarantees, correctness) | The criterion was wrong. It now reads: no cap, bounded by the 4 MiB body. | spec § Eligible links | — (text only) |
| LOW: AC11 says `persist*.rs` has no diff, but `persist_tests.rs` gains a literal (guarantees) | The criterion was wrong. It is now scoped to production files. A test-harness literal is allowed. | spec AC11 | — (text only) |
| LOW/INFO: OpenAPI, MCP, README and the guide README omit the link rules (guarantees) | R6: each surface names the `link.*` rules. | spec AC13 | R6 |
| INFO: the epic #251 authority was not recorded (guarantees, robustness) | The spec's Framing notes record what was read, and say the epic was not read at source. | spec § Framing notes | — |
| INFO: plan and tasks are stale (guarantees) | This plan and tasks.md now describe the shipped shape. | plan, tasks | R11 |
| LOW: `(https://paypal.com)` yields no token, and `www.com` matches every `.com` (correctness, independent; also robustness R3) | R5a: the leading trim comes before the scheme strip, and `www.` is stripped before qualification. | spec § Host-like tokens, AC1 | R5 |
| LOW: the 4-at-once cap is proven by timestamps (correctness, independent) | R7: an open-connection high-water gauge on the test server. No timing assertion. | spec AC6 | R7 |
| LOW: the held-probe wire test cannot go red (independent; correctness INFO) | R8: assert both probes are `!is_finished()` after the cold report. | spec AC6 | R8 |
| INFO: the `…_across_services` test overclaims (tests, correctness) | R9: rename it to what it runs. Cross-`Service` and warm-cache determinism stay proven by `deliverability.rs` and `tests/deliverability.rs`. | tasks T1.10 | R9 |
| INFO: the offline guard's taxonomy omits refused private literals (tests) | R10: the doc comment names the category, and now `fec0::1` as well. | tasks T7.3 | R10 |
| INFO: the fragment is part of the target key (independent) | R5b: the key clears the fragment, so one resource is fetched once. | spec § Targets, AC6 | R5 |
| INFO: the slice-2 hold is not on 0056's list (independent, security) | AC15: `implementation.md` and the result for ship carry the hold. | spec AC15 | R11 |
| INFO: "per process" (robustness R10) | Text: per `Service`, one per production process. | spec § Concurrency; 0121 §6 | R6 |
| residual raised by robustness: uncancellable `lookup_host` | Documented, not fixed. | spec § Concurrency; 0121 §6; network-posture | R6 |
| INFO: client per hop (correctness); post-probe `try_acquire` 503 (independent) | **No change.** See the rejected alternatives in 0121 and the adjudication below. | — | — |

**No criterion was loosened to make review pass.** The two corrected
criteria, the label cap and the AC11 scope, were wrong as written. The
correctness and guarantees lenses both showed this with evidence: #154 has
no cap, and the AC11 diff is a mechanical test-harness literal. AC5 and AC6
are **stricter** than before. The rule-set id stays `swarmail-html/4`,
because `/4` has never shipped. The `token()` reorder is a change inside an
unreleased version, and the snapshot test still ties the shortener digest
to it.

### Panel adjudication — second round (2026-11-04)

Both positions passed and agree on the shape of the fix: a default-deny
allow-list of `2000::/3` minus exclusions, with embedded IPv4 refused and
not decoded. They conflict on these points:

| question | simplicity | robustness | ruling and what the losing side pays |
|---|---|---|---|
| IPv4 table | unchanged | add `192.31.196/24`, `192.52.193/24`, `192.88.99/24`, `192.175.48/24` | **Add only `192.88.99/24`**, the deprecated 6to4 relay, so that it matches the `2002::/16` refusal. The other three are globally reachable anycast, so refusing them buys no SSRF safety and breaks the "globally routable" sentence. *Robustness pays:* those three anycast /24s are probed in `public` mode. *Simplicity pays:* one table row. |
| `www.` strip before qualification | not raised; swap only | yes | **Adopt.** One reordered line closes the `www.com` wildcard (correctness INFO 7) in a rule that exists to catch phishing. *Simplicity pays:* one extra fixture. |
| Fragment in the target key | keep: harmless and pinned | clear it | **Clear it.** A mail author can otherwise make the probe fetch one resource 32 times, so this is a small amplification lever. The fix is one `set_fragment(None)`. *Simplicity pays:* the target-count fixtures are re-pinned. |
| Wire proofs | one seam row | each refused literal on the wire, in `loopback` mode | **The table carries range membership, and the wire carries the path.** That means seam rows whose permitted listener comes first, plus one redirect into `[fec0::1]`. Wire-dialling every special range would, if the policy were broken, send real packets toward 6to4 relays and Teredo servers, and tests must never touch the internet. *Robustness pays:* no per-range wire row. The existing literal-refusal wire test (`127.0.0.1` in `public`) already proves that literals reach `permitted()`. |
| Resolver-thread residual | not raised | document it | **Document it** in the spec, in 0121 §6 and in network-posture. A bounded resolver needs a separate decision. |
| `std::net::Ipv6Addr::is_global` | unstable, so not used | never use it, even when stable | **Never use it** (robustness). Swarmail pins its own policy. |
| Client reuse, post-probe `try_acquire`, a public-suffix list | reject | reject or accept the cost | **No change.** Both positions agree, and both are bounded. |

The first-round adjudication follows unchanged. It records why the probe
exists, why `loopback` is a production mode, and why the probe is not
cached.

## Panel adjudication

First round, 2026-11-03. The robustness and simplicity positions both
passed. This synthesis rules on each conflict:

| Question | Simplicity | Robustness | Ruling and cost |
|---|---|---|---|
| Build reachability now? | No; record in 0121 and defer | Yes, as a frozen observation | **Build it** (robustness). The operator asked for it in this story, and deferring it would report completion with required work left. Simplicity's cost claims rest on a probe inside `evaluate` holding the pool, which this design does not do. *Cost to simplicity:* about 400 more lines at 100% coverage, security-seat scrutiny, and the PR held until 0121 is accepted (or the static slice approved by name). |
| Contradicts 0085/0106? | Yes | No (0085 §8 seam) | The letter is compatible: the evaluator stays network-free, and 0085 §8 anticipates observations. A broad reading is the operator's call, so the reachability slice is held (0121 §7). Separate commits keep the static slice severable. |
| Link source | Reuse `extract::walk` | Extend the `html.rs` lexer | **`extract::walk`** (simplicity). It gives #154 parity by construction and stays outside the 0107 `Scan` work census. *Cost to robustness:* none of substance; its parity-fixture ask is kept (T1.4). |
| Enable as config bool in the key | — | Rejected | **Rejected** by both. A startup mode plus a per-request ask, with no cache-key change. |
| Probe execution | Sequential, inside the 4-permit worker | Separate owner and semaphore | **Separate async owner** (robustness). Holding job permits across network waits starves every report. |
| Cache probe results (intake AC6) | — | Snapshot id in the key | **Not cached.** A per-probe key makes every slot single-use and serves stale "unreachable" answers. *Cost:* repeated probes repeat network work, bounded by the semaphore. Determinism is claimed for the pure composer only. |
| Loopback targets | `cfg(test)` seam | `cfg(test)` seam, no knob | **Production `loopback` mode.** A dev catcher must check `localhost` links, and wire tests then need no seam. Private, link-local and metadata addresses stay refused in every mode. *Cost to robustness:* an opted-in operator lets mail cause GETs to local ports; the guide documents this. |
| Plain http | Warning, exempt loopback | Warning | **Warning, exempt loopback/`localhost`** (simplicity). Otherwise nearly every dev mail warns. |
| Invalid URL rule | `link.scheme.invalid` | `link.url.invalid`, with an 8 KiB cap and control-character checks | **`link.url.invalid.v1`**, decided by `Url::parse` failure or emptiness. No length cap, because input is already bounded. |
| Shortener severity | warning | info | **warning.** Public shorteners are a common spam-filter signal. |
| Shortener storage (intake AC9) | Rust const, versioned by the rule-set id | JSON with its own version | **Const plus its own version plus a digest snapshot test.** "Versioned" holds without a loader. |
| Anchor comparison | Equal or subdomain | Full host after `www.` | **Equal or subdomain after `www.`** (simplicity). Full host flags ESP click subdomains everywhere. *Cost:* labels naming a shared suffix pass. |
| Redirect disclosure | Hop count | Hop list in a side result | **Hop count and final origin in the hint.** The finding schema is closed (0085/0105), and a side result would be a contract change. |
| Per-message link cap (static) | Cut | 32 per request plus occurrence caps | **Static: none** (0085's 256 findings fail explicitly). **Probe: 32 distinct targets.** |
| Probe concurrency (intake AC4) | The existing pool | Semaphore(2) | **2 requests × 4 links**, on the async runtime. The intake's "reuse the 4-job pool" is replaced for the permit-starvation reason above. |
| Cold-path harness (intake AC12) | Not needed | Allowed | **Not planned.** Timeout, cap and refusal are deterministic local outcomes, not cold paths (0020 rule 2 applies only to bug-only lines). |

## Delivery slices and commit order

1. **Static slice** (T1.1–T1.12): `extract.rs` visibility, `links.rs`, the
   `html.rs` hook, the rule-set bump, static wire tests and docs. This is
   compatible under 0045 §8.
2. **Reachability slice** (T4.1–T7.4): `reach.rs`, config, API, compose, wire
   tests and docs. It is held for 0121's acceptance.
3. **Records and gates** (T8.1–T9.2): the index row and the full gate run.
4. **Revision, 2026-11-04** (R1–R12): a later implement run lands it on
   top of `33efa4c`, in two commits:
   - the static-slice corrections (R4–R6, R9 and the static docs), which
     stay compatible under 0045 §8;
   - the reachability corrections (R1–R3, R7, R8, R10), which stay held.

   The records and gates close it (R11, R12). Earlier commits are not
   rewritten.

Each slice leaves every gate green on its own, so the operator can approve
slice 1 by name.

## Risks

| Risk | Mitigation |
|---|---|
| **SSRF through mail-chosen URLs.** The probe could be aimed at metadata, private hosts, a DNS rebind, or a redirect into private space. Review of `33efa4c` found that the block list let six special-purpose IPv6 ranges through (HIGH). | Off by default, with two explicit opt-ins. The policy is **default-deny**: IPv6 only inside `2000::/3` minus four blocks, and IPv4 only outside the registry's non-global entries. Embedded IPv4 is refused, never decoded. Swarmail resolves names itself; every answer must be permitted; it pins to the validated address and re-validates every hop. Userinfo is refused. No proxy, cookies, credentials or body read. The table test is written as literal rows, including every address review named. Resolver-seam rows put the permitted local listener first, so a broken check would dial only that listener. A redirect into `[fec0::1]` is refused (AC5, R1–R3). |
| **A future special-purpose block inside `2000::/3`** is permitted until the table is edited. | Recorded as a residual in the spec, 0121 and the guide, with the basis date. Any allocation outside `2000::/3` is refused with no change. Re-read the registry when 0121 is accepted. |
| **Refusing transition-only hosts.** A public site reachable only over 6to4, Teredo or NAT64 is reported as `refused`. | The outcome is an info finding saying the link was not checked, never a claim that it is safe. Accepted in 0121's alternatives. |
| **Uncancellable `getaddrinfo`.** Dropped probes leave blocking-pool threads running until the OS resolver gives up. | The opt-in mode, the per-`Service` caps and the OS resolver deadline bound it. It costs latency, never a `250`, because no job permit or Store guard is held. Documented in the spec, 0121 §6 and network-posture. |
| **The token reorder changes rule results inside `swarmail-html/4`.** | `/4` has never shipped, so no cached slot or released client sees two meanings. `host_token_table` and `anchor_mismatch_pair` pin the new results (R5). |
| **The slice-2 hold is lost at merge.** 0056's mechanical hold list does not cover it. | AC15 writes the hold into `implementation.md` and into the result handed to ship, so that it reaches the PR body. |
| **`loopback` mode lets mail cause GETs to local services.** | It is an explicit, documented operator choice in the guide and network-posture. It sends GET only, with no body read, no credentials and fixed headers. Private and link-local addresses stay refused. |
| **Resource exhaustion from a hostile mail or many callers.** | 32 targets, 4 per request, 2 requests per process, and a 5 s per-link budget across all hops give a worst case of 40 s and 8 sockets. Over the cap is an explicit 503. The probe never holds job permits or Store guards (AC6). |
| **`resolve_to_addrs` does not pin as expected** (for example, the URL port is ignored). | The pinning test (AC5) asserts the listener is reached and the `Host` header carries the name. The implementer adjusts the call until it passes. A test failure here blocks the slice. |
| **Offsets wrong for non-identity charsets or mail-parser normalisation.** | The per-part `exact` comparison falls back to `mime_part` (AC2), with fixtures for base64, QP, UTF-8, Latin-1 and entity or whitespace-led values. |
| **Label or anchor semantics drift from #154.** | The same `walk` and helpers are reused, not copied. A parity test compares the `Anchors` labels with ingest's stored `anchor_text` for the same body (T1.4). |
| **False positives** (`Node.js` in a label, `README.md`, shared suffixes). | Warnings only, with documented token rules and fixed hints that never claim phishing. Tests pin the documented edges. |
| **A hostile mail with more than 256 link findings fails its report.** | This is the 0085 rule: an explicit failure is better than a comfortable subset. The guide documents it, and AC8 proves it. |
| **Timeout tests slow down or flake.** | Servers hold the connection until the test ends, so the timeout is a real deadline and not a sleep. Assertions are on classes, not wall time. The 4-at-once proof counts open connections with a high-water gauge, not timestamps (R7). Two wire tests pay the 5 s budget (~10 s for concurrency), and the inline tests use the same production constant. A shorter `cfg(test)` budget is allowed only as a seam that a test installs. |
| **Rule-set id collision with #253/#260.** | The integrator takes the next free id at merge, and the snapshot fixtures record it. This slice touches only its own rules, files and tests. |
| **The PR is held because 0121 is proposed.** | The static and reachability slices are separate commits, and the operator may approve the static slice by name. The result says plainly that the reachability slice awaits acceptance. |
| **A test accidentally touches the internet.** | Probe tests use loopback literals and the resolver seam. Static fixtures with public names run with the mode `off`, which AC3 proves opens nothing. Review greps the probe tests for non-loopback hosts (T7.3). |
| **The `Config` literal churn breaks a sibling at merge.** | The field defaults to `Off` everywhere, and each literal change is a single line. Merge conflicts are mechanical. |
