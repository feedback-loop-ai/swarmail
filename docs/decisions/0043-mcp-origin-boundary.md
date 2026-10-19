# 0043 — Origin validation on exposed HTTP listeners

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance is the operator's.
- **Date:** 2026-10-18
- **Amends:** accepted [0030](0030-loopback-plane-bounded-results.md) §2:
  off-loopback Origin validation and default-port Host admission.
- **Spec:** [HTTP plane edges](../../specs/http-plane-edges/spec.md).
- **Issues:** #88, #89, #92. Number 0042 is occupied and remains unchanged.

## Context

The published container/chart binds off loopback. Under 0030 §2 both header
checks are inert there, so a browser can submit a text/plain MCP POST and
arm chaos without preflight. The String body is deliberate: dispatch owns
JSON-RPC parsing and its error envelope (0029 §2). A new JSON media-type
requirement could close the browser probe but would also reject bare curl -d.

Separately the loopback Host parser treats an omitted port as refusal even
on port 80. Fixing that must not turn malformed supplied values into defaults
or silently change the Origin policy. Metrics have an unbalanced total guard
and Debug-encoded labels; the scrape's own request makes issue #88's literal
0/1 acceptance numbers inconsistent with the all-requests counting contract.

## Ruling

1. Keep the whole-plane guard and its ordering. On loopback retain Host-first
   then Origin, existing refusal envelopes and accepted spellings. A missing
   Host port means 80; invalid supplied ports and malformed bracket suffixes
   refuse. Missing Host refuses. The loopback Origin's existing explicit-port
   requirement, scheme/path behavior and messages stay unchanged. In particular
   portless Origin still refuses at bound policy 80; #92 repairs Host only.
2. Off loopback the Host allowlist remains inactive. No-Origin requests pass
   unchanged. For a present Origin require exactly one valid Host and Origin;
   compare their authorities before dispatch on every route and method.
   Only http/https serialized Origins are valid: refuse null, lists, duplicate
   headers, invalid header text, missing/empty authority, userinfo, URL path,
   query, fragment, malformed brackets and invalid/empty/overflow ports.
   Compare ASCII host text case-insensitively and handle bracketed IPv6
   without DNS lookup or alias equivalence. Explicit ports are decimal u16.
3. Normalize missing ports on BOTH authorities using the Origin scheme
   (http 80, https 443), then require equal ports. Host has no scheme; the
   backend socket must not impose http's default on an HTTPS proxy request.
   Thus https://mail.example and Host mail.example or mail.example:443 pass;
   either direction between an omitted https port and explicit 8025 refuses.
   No Forwarded or X-Forwarded-* header participates. This validates authority,
   not an independently known external scheme.
4. Off-loopback refusal is ApiError 403 with the message
   `cross-origin requests must match the Host authority`, before any handler
   effect. Leave MCP's String body, media-type acceptance and dispatch envelopes
   intact. Do not add CORS. OPTIONS /mcp admitted by the guard remains 405
   without Access-Control-*; a hostile Origin is refused first with 403.
5. Restore metric ownership by incrementing inside TotalGuard's constructor
   and decrementing on Drop only. Count every request, including scrapes and
   guard refusals, through response production. An isolated quiet scrape reads
   1, a scrape plus one parked request reads 2, and after completion reads 1.
   Observer-excluded counts are 0/1/0. No route exception or persistent counter.
6. Encode registry label values at rendering with exactly the Prometheus
   backslash, double-quote and LF escapes. Other characters pass literally.
   Do not narrow the inbox domain. Preserve metric names, cap, `_uncounted`,
   family lifetimes and existing raw-path versus decoded-inbox keys.

## Alternatives and costs

Adopt robustness's always-active Origin leg off loopback; reject simplicity's
415 design. The latter solves the exact browser probe cheaply but changes
existing non-browser media-type acceptance. The chosen rule costs parsing
cases and refuses browser traffic behind Host-rewriting proxies. Preserve
the original external Host authority at the proxy, including nondefault port.
The suspected additional body-less POST routes are absent: chaos/webhook clear
are DELETE. That disproved premise is not the reason for this ruling.

Reject robustness's compare-ports-only-if-both-explicit rule: it admits a page
at a default port driving a custom port on the same hostname. Scheme-default
normalization preserves normal TLS termination without that wildcard. The
cost is refusal when a proxy strips a nondefault external port. No full
scheme authentication is possible from Host alone, and this work adds none.

Adopt simplicity's Host-only default correction over robustness's loopback
Origin broadening. Cost: port-80 reads with no Origin work but browser writes
carrying a portless loopback Origin retain the pre-existing refusal. State
that limitation rather than claiming full default-port browser compatibility.

Both positions' local metrics repair wins over saturation, resets, scrape
exclusion and cross-plane inbox validation. Cost: the issue's literal raw
0/1 acceptance is amended, and exact total tests require a child process.
Use an independent test-only emitted-subset parser for whole-scrape acceptance;
an escape-only scan is insufficient evidence and a production parser is not
needed. No new dependencies or allowlist changes are justified.

## Consequences and proof

Off-loopback still trusts network peers. A direct client can omit or forge
Origin; DNS rebinding is not prevented without the loopback Host allowlist.
No authentication is introduced. Document SSH/VS Code/devcontainer/Codespaces
port-forward refusals under loopback, the explicit off-loopback bind remedy
and its exposure, and Host-preserving TLS proxy requirements in README and
the applicable llms.txt posture text. Accepted 0030 is not rewritten here.

Real HTTP on a listener actually bound to 0.0.0.0:0 must refuse the evil
text/plain chaos probe without effect, accept matching authorities and retain
no-Origin client compatibility. Real router HTTP tests with Some(80) over an
ephemeral socket prove the default-port policy honestly without claiming a
port-80 bind. Existing loopback regressions remain unchanged. A fresh serve
child proves raw gauge 1/2/1 and no drift; a hostile decoded inbox label must
round-trip through an independent parser over a full real scrape.

Keep frozen store/wait/prune/extraction guarantees and all six gates, including
exact coverage. Unit lifecycle tests use private atomics; wire tests use
fresh servers and observed readiness, no sleeps or shared resets. The spec's
AC1–AC12 and paired task proofs are the implementation acceptance contract.
Index this record as proposed with the implementation, never self-accept it.

## Pointer — 2026-10-18 (proposed 0047)

[0047](0047-storage-rpc.md) §2, also **proposed**, updates §6 here (issue #130): the
awaits-registry overflow label is no longer `_uncounted`, a legal inbox name, but a single
line feed, which the shared inbox rule refuses; it renders as `inbox="\n"`. Metric names
and the 4,096-name cap are unchanged, and the cumulative total is summed from the rows of
the same scrape.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit. The operator accepted
this record and the later record below on 2026-10-19; the text above, the
0047 pointer included, stays as written, and that pointer's "proposed" is
historical. Each marker uses the later record's own verb and scope. The
amendment this record makes to 0030 §2 now carries a dated marker in
[0030](0030-loopback-plane-bounded-results.md).

- *(Updated by [0047](0047-storage-rpc.md) (accepted 2026-10-19): §6's
  overflow label, as the pointer above describes — a single line feed,
  rendered `inbox="\n"`, instead of the legal name `_uncounted`.)*
