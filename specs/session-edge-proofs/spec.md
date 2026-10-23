# Pin SMTP and POP3 session edges

Date: 2026-10-23. Status: draft design for NEXT-188, issue #188, epic #192.
Base: `cc30ea2`, the review-clean #177 integration candidate. Repository:
Swarmail, one Rust crate. This design does not close epic #192.

## Feature and rationale

Pin six existing protocol contracts with real TCP SMTP/POP3/HTTP exchanges
and test fixtures. An error string alone cannot prove that an envelope
survives, a completed DATA transaction is consumed, authentication remains
unchanged, or deleted POP3 content stays unavailable. Each refusal therefore
has a same-session continuation and an observable state witness. Accepted
mail is queried by the exact ID returned by DATA, immediately and without
polling, so the tests cannot conceal an acceptance-before-storage gap.

Triage selected full design and four reviewers because session state,
authentication precedence, persistence refusal and listener observation need
contract interpretation. Test-only scope does not reduce that review burden.
No production defect has been reproduced by this design phase.

## Scope and accepted authority

Implementation is confined to tests and test-only fixtures. Preserve #177's
owned admission, reap, backoff and task-completion events. No production
behavior, public interface, storage schema, dependency, policy, gate,
OpenAPI, metrics, MCP or unrelated product-documentation change is authorized.

This is additional proof of accepted semantics, not a semantic amendment;
no new decision is declared. Authorities are accepted
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0029](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md),
[0044](../../docs/decisions/0044-compat-validation.md) and
[0047](../../docs/decisions/0047-storage-rpc.md), with
[0027's controlling clarification](../../docs/decisions/0027-living-listeners-bounded-sessions.md),
[0028](../../docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md),
[0034](../../docs/decisions/0034-data-terminator-boundary.md) and
[0046](../../docs/decisions/0046-smtp-pop3-edges.md).
Later acceptance markers control historical proposal wording. Accept equals
stored, subscribe-before-check, oldest-only pruning and extraction at ingest
remain unchanged. Stored `envelope_from`, rather than header From, proves
SMTP envelope provenance.

If a real-server test demonstrates an existing production regression,
retain its concrete failing transcript and return to triage before repair;
retain full-panel review. A conflicting contract change additionally needs
a proposed decision and the operator's ruling before contradicting accepted
authority. A weakening of a frozen guarantee is refused.

## Panel synthesis and rulings

Read both run-local positions, `simplicity.md` and `robustness.md`, and the
intake `epic-192-residual-lane-next-188--4fa4a704.md`. Their six substantive
expectations agree; do not manufacture a semantic disagreement. Resolve
the implementation choices as follows:

| Question | Ruling and cost |
|---|---|
| Proof depth | Adopt robustness's state-transition and immediate accepted-ID witnesses, also explicitly supported by simplicity. Reject string-only, count-only and new-connection substitutes. Cost: longer real exchanges and explicit envelope/raw/inventory assertions. |
| Health-test owner | Adopt simplicity's POP3-owner placement and existing task-scoped held-backoff probe. Decline robustness's optional API-side adapter: it widens seam visibility without improving this proof. Cost of owner placement: a small local HTTP test helper/wiring may be needed. |
| Health body | Adopt robustness's exact JSON value. Do not create a JSON member-order contract from simplicity's serialized-body suggestion. Require HTTP 503 and exactly the two specified members; different semantic JSON, extra keys or the wrong listener fail. Cost: harmless serialization order changes are not detected. |
| Harness placement | Adopt inline measured stable assertions from both. A concrete failure-only cleanup/diagnostic arm permits only the narrow declared health harness; a four-file budget cannot override 0020. Cost: document the actual cold path and keep its production execution covered. Seam access alone is insufficient. |
| Fixture architecture | Adopt simplicity's existing clients, two explicit 451 scenarios and real SQLite guards. Reject a general fault framework, scenario enum, public seam or feature flag. Cost: small, readable duplication. |
| Broader robustness audit | Retain robustness's failure analysis as limits, not new crash/webhook/TLS/burst work. Preserve existing proofs. Cost: these six traces make no new durability, availability, arbitrary scheduling or exhaustive protocol guarantee. |

## Acceptance Criteria

All six criteria are mandatory. Fixed SMTP replies below include CRLF on
the wire; the PLAIN prompt must retain its space. Use exact single-line
assertions for the named decisions, not prefix/substring matching.

### AC1 — Recipient-free DATA preserves the open envelope

On one SMTP connection issue MAIL, then DATA without RCPT. Require
`503 5.5.1 Error: need RCPT command`. Without intervening MAIL, greeting or
RSET, add RCPT and complete DATA successfully. Parse exactly the
`250 2.0.0 OK: stored as <id>` response. Immediately fetch that ID and raw
bytes over HTTP (200), and assert the original envelope sender, exact
recipient vector, inbox, payload and singleton inbox ID set. Use distinct
header and envelope senders so a header-only assertion cannot pass.

### AC2 — Real POP3 degradation reaches HTTP health

Run the actual POP3 listener and HTTP router sharing the POP3 degraded flag;
keep SMTP's flag false and shutdown inactive. Establish a real POP3 greeting
and NOOP and healthy HTTP `/healthz` 200 with status `ok`. Arm one accept
error on that POP3 owner's probe, wake its accept with a real connection,
and observe its backoff pending while retry readiness remains held. A real
HTTP GET must return 503 and exactly the JSON value
`{"status":"degraded","degraded":["pop3"]}`. Manually setting a degraded
flag, direct handler calls, SMTP substitution, or another listener's event
cannot satisfy this criterion. Release the hold and stop/join owned tasks;
cleanup also releases/terminates them on assertion failure. No new recovery
proof is required; existing owned-poll recovery coverage remains intact.

### AC3 — Both actual DATA 451 causes consume the transaction

Two independent scenarios must reach the DATA terminator after exact
`354 End data with <CR><LF>.<CR><LF>`: acknowledged DATA-only probability-100
chaos produces `451 4.3.0 chaos`; a real SQLite insert-refusing trigger
produces `451 4.3.0 storage commit failed`. Keep payloads below the size cap.

After each 451, on that same socket, RCPT and DATA each return
`503 5.5.1 Error: need MAIL command`, then NOOP returns `250 2.0.0 OK`.
Before recovery, HTTP lists the isolated target inbox as exactly empty;
no refused mail is queryable. If an existing fixture retains a sentinel,
assert its exact unchanged ID/content instead of an empty set. Remove the
actual fault, start a fresh envelope on the same connection, and accept
distinct content with fresh sender and recipients. Immediately fetch the
returned accepted ID/raw over HTTP and assert exact inbox, envelope sender,
recipient vector, payload and ID membership (singleton unless a declared
sentinel exists). Refused recipients/content must not leak into recovery.
MAIL-stage chaos, 552, synthetic commit failures and reconnects do not count.

### AC4 — PLAIN prompt cancellation returns to the command loop

On an isolated initially unauthenticated SMTP session send AUTH PLAIN with
no initial response. Require literal `334 \r\n`, send `*`, require
`501 5.5.2 Credentials refused`, then NOOP's exact `250 2.0.0 OK`.
Deliver on the same connection without AUTH; immediately query its accepted
ID/raw and exact envelope in `default`. After that transaction, valid PLAIN
AUTH on the same connection returns `235 2.7.0 Accepted`; deliver distinct
mail and immediately verify its accepted ID/raw in the named inbox. Exact
ID sets in both inboxes prove unchanged identity on cancellation and the
remaining successful-auth allowance. Bounded operations fail a hang; no
elapsed-time correctness ceiling is permitted.

### AC5 — Valid AUTH during a transaction preserves state

Encode known well-formed PLAIN bytes (exactly three NUL-separated fields,
with a valid distinct authcid) using the existing base64 encoder. During an
open MAIL transaction send that credential; require
`503 5.5.1 Error: MAIL transaction in progress` immediately, with no prompt.
Finish the original transaction on that socket and immediately query its
accepted ID/raw, original `envelope_from`, exact recipients and unchanged
routing. Compare actual inbox inventory before and after refused AUTH,
before delivery, to prove no candidate-inbox creation; its mail remains
absent after delivery. Keep malformed-base64 refusal coverage separate;
`AG90aGVyYm94A===` is malformed and cannot establish valid-credential
precedence or any accepted identity. No additional successful-AUTH leg is
needed here because AC4 independently proves retry eligibility.

### AC6 — TOP refuses marked and genuinely gone slots without stale bytes

Extend real POP3 selective-delete-trigger flows. Before commit, TOP on a
DELE-marked slot returns exactly `-ERR message already deleted`. Allow an
earlier numbered deletion to commit while a later one refuses; QUIT returns
exactly `-ERR storage commit failed`. TOP on the committed-gone prefix
returns exactly `-ERR no such message`, both before and after RSET. After
RSET the previously refused suffix is fetchable at its original number with
exact original content. HTTP witnesses prefix absence and suffix survival;
retain existing persistence witnesses. Follow every new TOP refusal with
NOOP's exact `+OK` on the same stream to expose leaked headers/body/dot
terminators. Preserve the existing external-clear TOP proof. Out-of-range
numbers and pending DELE marks cannot substitute for a committed-gone slot.

## Proof discipline and delivery boundary

Use real servers, isolated stores/inboxes, ephemeral ports, scoped SQLite
triggers and owned handshakes. No mocks, sleep synchronization, retry-until-
lucky loops, broad ignores, coverage exclusions, weakened assertions or new
dependencies. Use raw bytes after SMTP transparency and the fixture's exact
CRLF addition. Acceptance witnesses do not poll or await eventual visibility.
Tests use sufficient/unlimited capacity so unrelated pruning cannot erase a
witness. Successful cleanup stops/joins servers; failure cleanup owns its
held gates, connections, trigger and tasks.

All gates in the sole [house inventory](../../docs/house-rules.md#gates--part-of-every-change)
remain required, with exact zero misses in lines/branches/functions, coverage
selftest, publish/seal proofs and fail-closed native-tls absence. Hermetic
verify remains offline, installs nothing and runs pinned proof code with
shim integrity checks. CI-only obligations remain explicitly CI-owned.

Full4 means correctness Qwen Max, security Opus, guarantees official
DeepSeek Flash, tests DashScope GLM-5.3; never DeepSeek Pro or fallback.
Root owns #177 publication before final integration/CI and reruns proofs on
the integrated candidate. Author commits are unsigned; no push, merge,
tag or registry action is authorized. Drafting is not implementation,
verification, operator acceptance, publication or closure of #192.
