# Decoded inbox metric identity

Epic #192 / issue #202. Design draft, 2026-10-24.

Decision: 0065 — [Decoded inbox metric identity](../../docs/decisions/0065-decoded-inbox-metrics.md), accepted (operator, 2026-10-24).

The following design-status language records the position before that
acceptance. The operator supplied the chat ruling “accept that” on 2026-10-24;
this record does not claim runtime implementation or test completion.

## Feature and rationale

For an inbox-scoped HTTP request, `swarmail_http_requests_in_flight` must
use the same decoded inbox identity as `swarmail_awaits_active` and
`swarmail_awaits_registered_total`. Today `/inboxes/issue202%20space/await`
counts the HTTP request under `issue202%20space` while the await families
count `issue202 space`. Unicode names have the same mismatch. A caller
observing one inbox must not need different label selectors for these rows.

### Draft-time authority and supersession request (historical)

Triage's `engine` classification stands: accepted
[0043 §6](../../docs/decisions/0043-mcp-origin-boundary.md) explicitly
preserves the raw-path/decoded-inbox distinction. This is a narrow semantic
amendment, although the implementation is small. Under accepted
[0045 §8](../../docs/decisions/0045-delivery-repairs.md), conflicting runtime
implementation must wait for the operator to accept 0062. This draft does
not accept it, and a passing design check supplies no acceptance.

Upon acceptance, this design supersedes only the raw-path key preservation
in `specs/http-plane-edges/spec.md` scope boundaries and its plan's AC3
proof. Their historical text and accepted 0043 remain intact in this design
commit; later documentation must explicitly mark the scoped supersession.
Until acceptance, 0043 remains authoritative.

**Acceptance recorded — 2026-10-24:** The operator accepted this ruling as
0065. This satisfies the draft-time prerequisite above; dated supersession
pointers now appear in 0043 §6 and the HTTP-plane spec and plan. Runtime
behavior and its regression proofs remain outstanding.

## Requirements

Select the inbox segment with the existing `inbox_of_path` route recognition
and empty-component handling, then percent-decode that segment exactly once
with the existing decoder. Strict UTF-8 success yields the verbatim decoded
string. Failure yields the entire original segment, without partial decoding
or replacement characters. This applies to every inbox-scoped route handled
by the middleware, including refused requests; it is not an await exception.

`+` remains literal; `%252F` becomes `%2F`; `%2F` becomes slash data inside
the already selected segment. Malformed percent escapes retain the existing
library's literal treatment. Do not trim, normalize Unicode, fold case,
decode the whole path, or repeatedly decode. Valid wire aliases coalesce
under the decoded HTTP label, intentionally changing encoded-label selectors.

Axum's existing `Path<String>` extractor continues to refuse invalid UTF-8
before await registration, with its current status and response body. Raw
fallback is accounting only, not admission. It can collide with a valid
literal-percent inbox: refused `%FF` and valid `%25FF` both account under
`%FF`. Preserve this compatibility limit; do not invent a new namespace.

Keep middleware ordering, immutable guard keys, increment/decrement ownership,
all-request totals including scrapes/refusals, and completion/cancellation/
unwind lifetimes unchanged. Preserve Prometheus escaping, metric families,
MCP identity, the 4096-name cumulative cap and LF overflow label from
[0047 §2](../../docs/decisions/0047-storage-rpc.md). Named registration rows
are guaranteed only within that cap. Preserve verbatim admitted names and
blank/control refusal under [0046](../../docs/decisions/0046-smtp-pop3-edges.md).

## Panel synthesis and rulings

Adopt simplicity's local helper change, existing dependency and existing
`tests/api.rs` parser. Adopt robustness's strict whole-segment fallback,
explicit alias-collision limitation, stable parked-window proof and cleanup.
The panels agree on runtime behavior; there is no genuine competing decoding
rule to average. Robustness supplies necessary proof constraints, not grounds
for a new identity layer.

Rule for the existing-file proof over unconditional process isolation: use
exclusive names and prove cap headroom in the API test executable. Escalate
to an isolated server process only if headroom cannot be established. The
cost is an explicit fixture audit; the alternative always pays process
startup and supervision complexity. Never replace a required named row with
an overflow assertion or serialize/reset global registries.

Reject lossy/repeated/whole-path decoding, render-only rewriting, dual raw
and decoded rows, and moving accounting into handlers: these merge wrong
identities, duplicate samples or lose refused-request ownership. Accept the
selector migration cost instead of retaining the mismatch. Reject broader
storage, SMTP, webhook, TLS and chaos changes: robustness's failure analysis
supplies preservation constraints, not proposed runtime scope. This change
cannot claim to repair their documented failure limits.

Both panels request a decision index entry. Defer that entry and historical
spec pointers to the subsequent documentation handoff because this seat is
explicitly limited to three spec files plus the proposed decision. No
uncommitted index edit is left behind; the cost is temporary discovery via
this spec/ADR until that tracked handoff lands.

## Acceptance Criteria

1. **AC1 — Exact segment identity.** Inline helper fixtures prove unchanged
   ASCII/missing/unrelated path behavior and exact outputs for space, Unicode,
   padding, literal `+`, `%252F`, `%2F`, literal `%`, incomplete/nonhex escapes,
   `%FF`, and mixed valid escapes plus invalid UTF-8. The latter returns the
   entire raw segment. Route recognition and empty-component behavior remain
   unchanged. Both strict UTF-8 branches execute under exact coverage.
2. **AC2 — Three equal rows while parked.** A real HTTP test parks two awaits
   concurrently through encoded segments for exclusive inbox names containing
   a space and a multibyte character. Empty inboxes remain empty until the
   assertion. Under bounded readiness observation, one fully parsed scrape
   must contain exactly one sample per decoded name in each of the three
   metric families above, all with value 1 for unused names. Both encoded
   aliases are absent from all three families. No global-total assertion,
   whitespace-split parser, sleep-as-readiness or registry reset is allowed.
   Restoring the original helper must fail this behavioral assertion.
3. **AC3 — Completion and retention.** Seed one mail per inbox through its
   encoded route, join both successful await responses and assert `matched`
   equals 1. A subsequent parsed scrape has neither live gauge row for either
   name and retains registration value 1. All operations have deadlines;
   pending clients and the real server have failure-safe cleanup. Client
   cancellation alone is not evidence that a server handler has finished.
4. **AC4 — Invalid UTF-8 compatibility.** Before changing production, record
   and pin the current real-HTTP extractor refusal status/body for an encoded
   invalid UTF-8 inbox using raw/text response handling. After the change it
   is identical; await-active and cumulative registration rows for the raw
   name and a lossy replacement-character alias remain absent (exclusive
   names). The helper fixture proves raw fallback; a post-refusal scrape is
   not evidence of the transient HTTP label. No request is admitted by fallback.
5. **AC5 — Existing guarantees.** Existing hostile-label strict scrape,
   ASCII readiness, total ownership, cap/overflow, blank/control refusal and
   guard lifecycle tests continue to pass. Preserve synchronous accepted
   storage, subscribe-before-scan, oldest-only pruning and ingest extraction.
   No new dependency, allowlist widening, runtime configuration or metric
   family is introduced.
6. **AC6 — Authority and delivery.** Operator acceptance of 0065 is recorded
   (2026-10-24); runtime implementation still requires targeted tests and
   red-before/green-after evidence, remaining selector documentation, and the
   entire [house-rules gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
   clean, with exact coverage and its selftest. CI-only proofs remain CI's
   responsibility. This acceptance record is not a claim of implementation
   success.
