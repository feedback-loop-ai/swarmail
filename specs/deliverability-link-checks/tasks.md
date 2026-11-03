# Deliverability link checks — tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision:
[0121](../../docs/decisions/0121-deliverability-link-checks.md) (proposed).

Each task names the test that proves it. Test names are binding
suggestions: the implementer may rename a test but must keep its
assertion. Inline tests are `#[cfg(test)]` in the named file. Wire tests
live in `tests/deliverability.rs`. No probe test names a non-loopback host
except through the resolver seam.

## Slice 1 — static link rules (compatible under 0045 §8)

- [ ] **T1.1** Widen `src/extract.rs` visibility to `pub(crate)` for
  `walk`, `Regions`, the `AnchorAttrs` fields, the stats types in the trait
  signature, `space`, `LABEL_BREAKS` and the label-cap constant. Make no
  behaviour change.
  Proof: the existing `extract.rs` and ingest tests pass unmodified;
  `link_ingest_is_byte_identical` (inline in `links.rs`) compares
  `email.links`, `link_metadata` and codes for every link fixture against
  values written in the test (AC11).
- [ ] **T1.2** In `html.rs`, factor eligible-part enumeration into
  `eligible(raw)`, returning `path`, `body` and
  `exact = part.contents() == body.as_bytes()`. `collect_with` keeps its
  budgets and order.
  Proof: every existing `html.rs` test passes unchanged, and
  `eligible_exact_tracks_charset_identity` covers UTF-8, us-ascii, base64,
  QP and Latin-1 with an 8-bit byte (AC2).
- [ ] **T1.3** Add `links.rs`: the `Anchors: Regions` consumer, giving
  `Occurrence {location, href, label}`, with the offset at the first
  non-ASCII-whitespace source byte of the href value and `mime_part` when
  `!exact`.
  Proof: `link_offsets_index_transfer_decoded_source` (quoted, unquoted,
  entity-bearing, whitespace-led, two parts, base64, QP) and
  `non_identity_charset_falls_back_to_mime_part` (AC2).
- [ ] **T1.4** Make labels follow #154 exactly: decode once, collapse,
  break on block boundaries, cap, visible only, and close at the next `<a>`
  or `</a>`.
  Proof: `anchor_labels_match_ingest_anchor_text` compares the
  `Anchors` labels with ingest `anchor_text` for the same http(s) bodies.
  `hidden_anchors_yield_no_occurrence` covers script, style and suppressed
  containers (AC1).
- [ ] **T1.5** Rules `link.url.invalid.v1` and `link.scheme.unsafe.v1`.
  Proof: `invalid_url_pair`. It fires on empty, relative, `//host`,
  `{{x}}` and an invalid host, and passes on `#top` and an absolute https
  URL. `unsafe_scheme_pair` fires on `javascript:`, entity-encoded
  `&#106;avascript:`, tab-split `java&#9;script:`, `JAVASCRIPT:`,
  `vbscript:`, `data:` and `file:`, and passes on `https:`, `mailto:`,
  `tel:` and `cid:` (AC1).
- [ ] **T1.6** Rule `link.scheme.http.v1`, with the loopback/`localhost`
  exemption.
  Proof: `http_scheme_pair`. It fires on `http://example.com/` and passes
  on `https://example.com/`, `http://localhost:3000/x`,
  `http://app.localhost/`, `http://127.0.0.1/`, `http://127.9.9.9/` and
  `http://[::1]/` (AC1).
- [ ] **T1.7** Add `SHORTENERS` and `SHORTENERS_VERSION`, and rule
  `link.shortener.v1`.
  Proof: `shortener_pair`. It fires on `https://bit.ly/x` and
  `http://t.co./x`, and passes on `notbit.ly`, `bit.ly.evil.test`,
  `www.bit.ly` and `example.com`. `shortener_list_is_canonical_and_pinned`
  checks the list is sorted, unique, lowercase and has no trailing dot, and
  pins its digest to `swarmail-shorteners/1` and `RULE_SET_ID` (AC9).
- [ ] **T1.8** Add the host-token parser and rule
  `link.anchor-host-mismatch.v1`.
  Proof: `host_token_table`, which covers scheme strip, path cut,
  punctuation trim, `@` and non-ASCII skip, `v1.2`, `3.14`, `e.g.` and
  `Node.js`. `anchor_mismatch_pair` fires on `paypal.com` →
  `paypal.com.evil.test`, on `example.com` → `https://1.2.3.4/`, and on
  `https://trusted.test@evil.test` with label `trusted.test`. It passes on
  `example.com` → `www.example.com` and `click.example.com`, on
  `www.example.com` → `example.com`, on an empty label, and on a label with
  no token (AC1).
- [ ] **T1.9** Call `links::collect` from `collect_with` for each eligible
  part. Bump `RULE_SET_ID` and `html-rules.json` to `swarmail-html/4`, and
  move every `/3` literal (`html.rs` snapshot fixture, `mcp.rs`,
  `openapi.json`, `llms.txt`, `tests/deliverability.rs`,
  `tests/bulk_sender.rs`, the guide).
  Proof: `reviewed_version_snapshot_and_every_rule_pair`, updated;
  `rg 'swarmail-html/3' src tests docs/guide` returns nothing (AC10);
  `link_findings_respect_report_limits` gives 257 link findings → `Failed`
  and over 64 KiB → `Failed` (AC8).
- [ ] **T1.10** Determinism of the default report with link findings.
  Proof: `link_report_is_deterministic_cold_warm_and_across_services`, and
  `report_provider_snapshot_is_deterministic` extended with a link fixture
  (AC7).
- [ ] **T1.11** Wire test of the static rules.
  Proof: `link_rules_over_smtp_and_rest` SMTP-delivers one HTML mail
  containing every failing case. The REST report equals findings written
  independently in the test, with exact offsets. The MCP report and the
  message page show the same findings, escaped (AC1, AC2).
- [ ] **T1.12** Docs: `docs/guide/deliverability.md` "Link rules" covers
  the rule table, the http ruling, the token rules, the shortener list and
  its version, the locations and the false-positive notes.
  Proof: review reads the guide against spec § Static rules (AC13).

## Slice 2 — opt-in reachability (held for 0121's acceptance)

- [ ] **T4.1** Add `reach::Mode` and the CLI flag `--link-reachability`
  (env `SWARMAIL_LINK_REACHABILITY`, default `off`), carry it as
  `Config::link_reachability` in every `Config` literal, and add
  `Store::with_link_reachability`, used by `lib.rs`.
  Proof: `link_reachability_flag_parses_and_defaults_off` (a `config.rs`
  inline test covering the default, the three values, the env, and a
  refusal of an unknown value) (AC3).
- [ ] **T4.2** Add `reach::permitted(IpAddr, Mode)`.
  Proof: `address_policy_table` covers every spec range at its first and
  last address in `public` and `loopback`, IPv4-mapped IPv6, `::`, `::1`,
  `fe80::1`, `fc00::1`, `ff02::1`, `2001:db8::1`, `169.254.169.254`,
  `100.64.0.1`, `255.255.255.255`, and two public addresses that pass
  (AC5).
- [ ] **T4.3** Implement the hop loop. It refuses userinfo, resolves
  through the resolver seam or `lookup_host`, requires all addresses to be
  permitted, and pins to the first address. The client uses `no_proxy`,
  `Policy::none`, a 2 s connect timeout and the fixed `User-Agent`, sends
  GET, reads status and headers only, follows up to 5 redirects, and runs
  under a 5 s per-link budget.
  Proof (inline, local `TcpListener` servers):
  - `probe_ok_and_status`: 200 → `Ok`; 404 → `Status(404)`;
  - `probe_redirects_within_and_beyond_cap`: 2 hops then 200 →
    `redirects: 2` and the final origin; an endless 302 → `RedirectCap`
    after exactly 6 requests;
  - `probe_bad_redirects`: no `Location`, an unparsable one, and `ftp:` →
    `BadRedirect`;
  - `probe_timeout_on_silent_server`: the server holds the connection until
    the test ends → `Timeout`;
  - `probe_connect_on_closed_port`: a closed port → `Connect`;
  - `probe_never_reads_body`: headers with `Content-Length: 104857600` and
    no body → `Ok`;
  - `probe_refuses_userinfo`: refused with zero accepts;
  - `probe_resolver_seam_pins_mixes_and_fails`: the pinned name reaches the
    listener with `Host` equal to the name; `[127.0.0.1, 10.0.0.1]` →
    `Refused` with zero accepts; a resolver error → `Connect`;
  - `probe_revalidates_each_hop`: a loopback hop redirecting to
    `http://10.0.0.1/` → `Refused` after exactly one accept;
  - `probe_public_mode_refuses_loopback`: zero accepts.

  These cover AC4 and AC5.
- [ ] **T4.4** Add `Prober::observe`, with the 32-distinct-target cap
  before any connection, 4 links at a time and `Semaphore(2)` across
  requests (`try_acquire`, giving `Unavailable`). Cancellation drops the
  probes and the permit.
  Proof:
  - `observe_caps_targets_before_connecting`: 33 targets → `Failed` with
    zero accepts;
  - `observe_runs_four_at_a_time`: one silent listener, five targets → a
    maximum of exactly 4 open connections, 5 accepts and five `Timeout`s;
  - `observe_refuses_third_request`: two held → the third is
    `Unavailable`;
  - `observe_cancellation_releases_permit`: drop a held request → a new
    request is admitted.

  These cover AC6.
- [ ] **T5.1** Add the pure `links::compose(report, occurrences,
  observation)`, with the outcome → finding table and literal hints
  (`redirect-chain` carries the count and the origin only; `status`
  carries the code), and every target occurrence located. It goes through
  the one canonicalizer.
  Proof:
  - `compose_is_pure_and_order_independent`: two observations built in
    different orders give byte-identical JSON;
  - `compose_finding_table`: every `End`, each with and without redirects,
    matches literal findings, and no path, query or userinfo appears in any
    hint;
  - `compose_respects_limits`: more than 256 findings or more than 64 KiB
    → `Failed`.

  These cover AC4, AC7 and AC8.
- [ ] **T5.2** Add `Service::probed`. It gets the cached default report
  first, then runs the occurrence scan on `Store::offload` under a job
  permit and releases the permit before observing, then composes. It never
  writes the slot.
  Proof: `probed_reuses_slot_and_never_caches`. A default request after a
  probed one is a `ready_only` hit with the unchanged report, and a second
  probed request probes again (the accept count rises). Slot inputs are
  unchanged by construction: `Inputs` gains no field (AC7).
- [ ] **T6.1** API: add `DeliverabilityQuery { reachability }`. The order
  is 404, then `Off` → 400 `link reachability is not enabled`, then
  `report_probed`. `false` or absent gives the default path; a bad value or
  an unknown key gives the existing 400.
  Proof: `deliverability_query_contract`, inline in `api.rs` or on the wire
  (AC3).
- [ ] **T7.1** Wire test: default off and lazy.
  Proof: `link_reachability_is_off_and_lazy`. A counting loopback listener
  sees zero accepts across SMTP `250`, the message read, the default
  report, the MCP report, the message page, assert and await (default and
  `loopback` servers), and across `?reachability=true` on a default server
  (400 with the literal). The default report has no `link.reachability*` or
  `link.redirect-chain.v1` finding. Then `?reachability=true` on the
  `loopback` server gives ≥ 1 accept (AC3).
- [ ] **T7.2** Wire tests: opt-in outcomes, policy and bounds.
  Proof:
  - `link_reachability_outcomes_on_loopback`: 200, 404, two redirects, an
    endless 302 and a silent server, with literal findings at exact
    offsets;
  - `link_reachability_public_mode_refuses_loopback`: `refused`, zero
    accepts;
  - `link_reachability_bounds_on_the_wire`: 33 targets → 503 `failed`; a
    third concurrent request → 503 `unavailable`; while two are held, a
    fresh SMTP `250`, its read and a cold default report all complete.

  These cover AC4, AC5 and AC6.
- [ ] **T7.3** Offline guard.
  Proof: `probe_tests_use_loopback_only`, an inline test that scans the
  probe test sources (`include_str!` of `reach.rs` and
  `tests/deliverability.rs`) for `http://` and `https://` literals in
  probe-enabled cases and refuses any host other than `127.0.0.1`, `[::1]`,
  `localhost` or a name registered in the resolver seam. Review also checks
  this by eye (Risks: internet). Where a scan is impractical, the
  implementer records the manual check in the implementation record
  instead.
- [ ] **T7.4** Docs:
  - `docs/guide/deliverability.md` "Opt-in reachability" covers the modes,
    the two-step enablement, every bound with its value, the outcome table,
    the determinism statement and the `loopback` risk;
  - `docs/guide/network-posture.md` covers the outbound probe;
  - `docs/guide/rest-api.md`, `openapi.json` and `llms.txt` cover the
    `reachability` parameter and its 400.

  Proof: the existing OpenAPI and `llms.txt` consistency tests, if present,
  extended; otherwise review against spec § Opt-in reachability (AC13).

## Slice 3 — records and gates

- [ ] **T8.1** Add the `docs/decisions/README.md` index row for 0121, with
  status "proposed — awaiting the operator" and this scope. Change no
  other decision's status.
  Proof: `git diff main -- docs/decisions/` shows only the new file and
  the one added row (AC13).
- [ ] **T8.2** Check that no dependency changed.
  Proof: `git diff main -- Cargo.toml Cargo.lock evals/Cargo.lock
  deny.toml` is empty, and `cargo deny check licenses` passes (AC12).
- [ ] **T9.1** Run every house gate `--locked` on the final tree:
  - fmt, clippy and test;
  - `cargo build --locked --bin swarmail`;
  - evals fmt, clippy and test with `SWARMAIL_BIN` from Cargo's artifact
    record;
  - the brokkr compile loop over `.brokkr/recipes/*/`, reading each output
    for `lets the run go on without reading`;
  - `scripts/coverage-gate.sh` (exactly 100%) and `selftest`;
  - `publish-guard-test.sh`, `seal-branch-test.sh` and
    `cargo deny check licenses`.

  Proof: the gate transcript in the implementation record. It names the
  CI-owned proofs (macOS, RustSec, Docker, Helm, port-80) as not claimed. A
  load proof, if run, is one named, timed command bounded to
  `--test-threads=$(( $(nproc) / 4 ))` and ≤ 10 minutes, with the load
  applied reported (AC14).
- [ ] **T9.2** Commit unsigned, in slice order (slice 1, slice 2, slice 3),
  in the repo's style. Never push, merge or accept a decision. The result
  states that the reachability slice awaits the operator's acceptance of
  0121.
  Proof: `git log --format='%G? %s' main..HEAD` shows `N` for every commit
  and the slice order.
