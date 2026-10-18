# HTTP plane edges

Date: 2026-10-18. Status: design draft; implementation and operator acceptance
remain pending. Scope: review issues #88, #89 and #92, run 20.

## Feature and rationale

Make HTTP readiness metrics truthful and parseable, refuse cross-site browser
requests on explicitly exposed listeners, and admit an omitted Host port when
the loopback listener uses HTTP port 80. Keep existing non-browser clients
working and explain forwarding and proxy requirements.

The total currently decrements without incrementing, so completed requests
wrap it below zero. Rust Debug formatting emits invalid Prometheus escapes
for inbox labels. Off-loopback listeners currently disable both header checks,
allowing a visited page to invoke MCP with a simple text/plain POST. Finally,
the loopback Host check rejects the legitimate omission of default port 80.

This changes a public HTTP contract and amends accepted decision 0030 §2.
Proposed [0043 — MCP origin boundary](../../docs/decisions/0043-mcp-origin-boundary.md)
owns that amendment, the port-default policy and the metric observation
clarification. Only the operator accepts it. No frozen guarantee is weakened.

## Evidence and explicit panel rulings

Read both run-local positions, the commission and all three issue bodies.
The files in `.brokkr/state/tasks/` frame historical run-12 work; there is no
run-20 intake file there. The current framing is `.brokkr/state/run20-feature.txt`
and `.brokkr/state/epic70/{88,89,92}.md`. These ignored sources remain evidence,
not deliverables. The decisions below make this specification self-contained.

| Conflict or proposal | Ruling and cost |
|---|---|
| Both: constructor-owned total and renderer-owned escaping. | Adopt. One increment paired with Drop, three Prometheus escapes, no metric reset or domain restriction. All inbox names admitted today remain admitted. |
| Issue #88's scrape values 0/1 versus counting every request. | Adopt both panel corrections: raw scrape is 1 at rest and 2 with one parked request; subtracting the observer gives 0/1. No route exception. Cost: the issue's literal acceptance wording is corrected explicitly. |
| Exact total tested in a parallel in-process server. | Adopt robustness's isolated child process. Per-inbox uniqueness cannot isolate a process-global static. Cost: one real-binary fixture and its cleanup. |
| Label parser evidence. | Adopt simplicity's independent strict reader for the emitted Prometheus subset, with decoded-label recovery and whole-scrape parsing. Reject robustness's escape-only scan as sufficient evidence: it cannot validate sample syntax. Cost: a small test-only parser with invalid fixtures, no new dependency. |
| Simplicity: off-loopback /mcp media-type requirement with 415. Robustness: whole-plane Origin check with 403. | Adopt the Origin check; do not stack a media-type gate. This preserves text bodies and bare curl -d while enforcing the same header boundary on all routes. Cost: more authority cases and refusal behind proxies that rewrite Host. The 415 alternative would preserve those proxies but reject existing curl bodies and depend on preflight refusal for this defense. |
| Robustness's suspected body-less POSTs. | Reject that premise after inspection: clear_chaos and clear_webhooks are DELETE. The router's POSTs are JSON-extracted seed and String-bodied MCP; UI/compat add no POST. The Origin ruling rests on compatibility and explicit header validation, not an invented REST vulnerability. |
| Robustness: compare ports only if both explicit. | Reject the wildcard. Off loopback normalize both omitted ports using the Origin's http/https default and require equality. Thus https://mail.example and Host mail.example work behind TLS termination, but https://mail.example and Host mail.example:8025 refuse. Cost: deployments stripping a nondefault external port must preserve the original Host authority. |
| Robustness: also default loopback Origin ports by scheme. Simplicity: default only Host. | Adopt simplicity. #92 changes only missing Host ports to 80; existing loopback Origin validation and explicit-port requirement stay intact. Cost: portless browser Origin on a loopback port-80 write remains refused, a pre-existing limitation outside #92's Host repair. |
| Shared framework, new config, dependencies, proxy-header trust. | Reject. Local helpers in api.rs suffice. Off-loopback remains unauthenticated and exposed to direct peers and DNS rebinding; this work does not claim to authenticate Host or Origin. |

## Requirements and boundary contract

The total counts entry through response production, including /metrics and
guard refusals. RAII releases ownership on return, cancellation and unwind;
streaming response-body lifetime is not added. All four registry families use
the same label rendering: escape backslash, double quote and LF as `\\`,
`\"`, `\n`; preserve other characters, including literal tab, CR and Unicode.
Keep the awaits cap, `_uncounted` fold, metric names and row lifetimes.

Loopback preserves Host-first then Origin refusal and existing error bodies.
Only a genuinely absent Host port now means 80. An empty, invalid, overflowing
port or malformed bracket suffix never becomes omission. No change is made
to the loopback Origin helper's existing scheme/path handling or explicit-port
requirement. Missing Host still refuses. Existing loopback spellings remain.

Off loopback, with no Origin, the guard passes as before; it introduces no
Host allowlist or media-type requirement. With Origin, require one valid Host
and one valid Origin. Refuse duplicates, invalid header text, opaque `null`,
lists, userinfo, paths, queries, fragments, empty hosts, malformed brackets or
ports, and schemes other than http/https. A serialized Origin is scheme plus
authority only. Hosts compare ASCII case-insensitively; bracketed IPv6 is
handled as an authority, without DNS resolution or alias expansion. No trailing
dot, IDNA or alternative IPv6-spelling equivalence is promised.

Normalize an omitted Origin port to 80 for http or 443 for https. Normalize an
omitted Host port using that same scheme, not the backend socket's plaintext
scheme. Explicit ports must be decimal values in u16 range. Require equal
host text and equal resulting ports. This compares authorities, not a proven
external scheme: Host carries none. Do not trust Forwarded/X-Forwarded-*.
On mismatch or invalidity return 403 before dispatch with the existing
ApiError envelope and message `cross-origin requests must match the Host authority`.
Apply this on every route and method. Guard-admitted OPTIONS /mcp still returns
405 without Access-Control-* headers; hostile-Origin OPTIONS returns 403.

README and llms.txt must state the off-loopback trust, Host-preserving proxy
requirement, and the remaining absence of authentication/rebinding defense.
README must explain that ssh -L, VS Code/devcontainer and Codespaces forwarding
can preserve an external port different from the loopback bind, causing 403.
The explicit remedy is e.g. `SWARMAIL_HTTP_LISTEN=0.0.0.0:8025`, with network
exposure owned by the operator and Origin checks still active. Document both
the portless Host correction and the remaining loopback Origin limitation.

## Acceptance Criteria

1. **AC1 — total ownership:** deterministic isolated-atomic tests observe
   0 → 1 → 0 for normal drop, cancellation and unwind; the production middleware
   constructs the guard through its incrementing constructor. No global reset,
   saturation or /metrics exemption is introduced.
2. **AC2 — observed total:** over real HTTP to a fresh serve child, sequential
   quiet scrapes each report raw total 1; a single acknowledged parked await
   yields 2; after satisfying and joining it, the total returns to 1. At least
   three further sequential scrapes remain 1. The labeled readiness row proves
   parking; no sleep establishes it. Observer-excluded values are 0/1/0.
3. **AC3 — valid labels:** park a real wait on a percent-encoded inbox whose
   decoded name includes tab, backslash, quote and LF. An independent strict
   Prometheus-subset reader accepts the whole scrape and recovers that exact
   decoded label from the awaits family. Unit byte fixtures additionally pin
   CR and Unicode and all four families' common renderer. Parser fixtures
   reject Rust-style `\t`/`\u{...}`, raw LF in a quoted label, unterminated
   quotes and malformed sample/value syntax.
4. **AC4 — reproduced drive refused:** bind via run_on to 0.0.0.0:0 and connect
   over 127.0.0.1 at that port. The issue's evil.example text/plain MCP
   swarmail_set_chaos probe returns 403 with the specified refusal and leaves
   the previously read chaos configuration unchanged. Use probability 100.
5. **AC5 — non-browser compatibility:** on that bind, no-Origin valid MCP
   requests succeed with application/json, text/plain, form-urlencoded and
   absent Content-Type. SDK-shaped calls, existing batch/notification/parse
   error envelopes and real stdio bridge tests remain green. Malformed JSON
   admitted by the guard still reaches dispatch and returns its parse error.
6. **AC6 — matching authorities:** real HTTP accepts same host/port, hostname
   case variation, bracketed IPv6, explicit scheme defaults and omitted-port
   http/https pairs. Specifically https://mail.example + Host mail.example
   passes, as does https://mail.example + Host mail.example:443.
7. **AC7 — mismatches and invalidity:** real HTTP refuses evil host, unequal
   explicit ports, https://mail.example + Host mail.example:8025 and
   https://mail.example:8025 + Host mail.example. It refuses null, malformed
   Origin, missing Host with Origin, invalid header text and duplicate Origin
   or Host, where accepted by the HTTP parser; a parser-level 400 is acceptable
   only for messages it cannot deliver to middleware. Private validator tests
   pin rejection of every malformed authority class in the contract.
8. **AC8 — whole plane and preflight:** hostile Origins refuse representative
   REST reads/writes, MCP and UI before effects; GET chaos confirms unchanged
   state. Guard-admitted OPTIONS /mcp returns 405, hostile Origin returns 403,
   and neither response contains Access-Control-* headers.
9. **AC9 — loopback preservation:** existing tests
   same_origin_loopback_spellings_pass_the_guard,
   cross_origin_and_malformed_spellings_are_refused_before_the_router and
   a_rebound_host_refuses_even_without_an_origin pass unchanged. Add a
   text/plain MCP success, Host-before-Origin refusal precedence, and explicit
   proof that portless Origin remains refused even with bound policy 80.
10. **AC10 — Host default:** real HTTP through the real router with
    AppState.http_origin_guard = Some(80) admits portless localhost, 127.0.0.1
    and [::1]. The same headers refuse under Some(8025); explicit matching
    ports pass, explicit wrong/empty/nondecimal/overflow ports, absent Host,
    nonloopback host and malformed bracket suffixes refuse. Use an ephemeral
    socket and disclose that this proves the port-80 policy without binding 80.
    Keep startup's derivation from the actual listener covered.
11. **AC11 — documented costs:** README and llms.txt match the chosen rule;
    README names all forwarding cases, the off-loopback remedy, Host-preserving
    TLS proxies, the loopback portless-Origin limitation and the scrape's
    observer contribution. Neither document calls the off-loopback guard inert.
12. **AC12 — proof and delivery:** each child issue has a behavioral failing
    baseline and passing repair (both gauge and label defects for #88); compile
    errors do not count. All six house gates pass, coverage stays exact
    100/100/100 without TMPDIR override or exclusions, and no dependency is
    added. Proposed 0043 is indexed during implementation, never self-accepted.

## Scope boundaries

Keep synchronous accept-after-store, subscribe-before-scan, oldest-only
pruning and ingest extraction. Keep the lock sidecar, webhook plane, stdio
bridge, extraction fixes, 5000 ms SQLite busy bound and staged compat kind.
Do not normalize raw-path versus decoded-inbox metric keys in this change.
No authentication, CORS policy, proxy allowlist, port-forward bypass, new
production module or metric framework. This design seat writes exactly four
artifacts and does not implement, push, merge or close issues.
