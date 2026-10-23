# Pin SMTP and POP3 session edges

Date: 2026-10-23. Status: retry2 draft design for NEXT-188, issue #188, epic #192.
Base: `cc30ea2`, the review-clean #177 integration candidate. Repository:
Swarmail, one Rust crate. Reuse the implementation already committed at
`e7d1d8142c3e24b2a817822969ab0dae6619680a`, following design `150c84a`.
This design does not close epic #192.

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

Read both current run-local positions, `.brokkr/state/design/positions/simplicity.md`
and `robustness.md`, and intake
`.brokkr/state/tasks/epic-192-residual-lane-next-188--0658a130.md`.
The current panel agrees on all six protocol expectations and on retaining
full4. There is no genuine semantic conflict to average or invent. Replace
the prior design's attributed API-adapter and serialized-JSON disagreements:
neither current position advocates them. Rule on the alternatives explicitly:

| Question | Ruling and cost |
|---|---|
| Reimplement or reuse | Adopt simplicity's reuse of the existing candidate and spec family, supported by robustness. Reject duplicate tests, new source files and a second design family. Cost: independently audit inherited implementation and obtain fresh receipts rather than treating prior work as verified. |
| Proof depth | Adopt robustness's same-session state transitions and immediate accepted-ID/raw/provenance witnesses, also supported by simplicity. Reject prefix-only, count-only and reconnect substitutes. Cost: longer exchanges and distinct fixtures; the cheaper alternative could conceal envelope loss or changed identity. |
| Health ownership and JSON | Adopt both positions' POP3-local probe, one owner-observed failure and held retry readiness, with exact parsed JSON equality. Reject a new API-side adapter, manually toggled flag, polling and JSON member-order contract. Cost: local HTTP wiring and no detection of harmless object-key reorderings. |
| Harness placement | Keep the existing stable health proof inline and measured, as both positions advise: its unconditional Drop release/abort and explicit success joins need no cold-arm migration. If a concrete failure-only arm becomes necessary, 0020 wins over a strict four-file budget; move only that proof/helper into a declared sibling harness. Cost: every inline line remains an exact-coverage obligation; a justified migration requires explicit review. |
| Fixture architecture | Adopt simplicity's bounded raw reader, local SMTP assertion helpers, two explicit 451 scenarios and real SQLite triggers. Reject a general state enum, scenario framework, public seam or feature flag. Cost: small readable duplication and schema-coupled fixtures. |
| Robustness boundary | Adopt robustness's failure analysis as explicit limits, not new crash/webhook/TLS/burst implementation. Reject widening this lane without a reproduced regression and re-triage. Cost: these finite traces do not establish arbitrary scheduling, exactly-once SMTP, durable webhook delivery or exhaustive transport behavior. |
| Review and provenance | Adopt both positions' full four independent reviews and source-bound pinned verification. Reject reduced review based on test-only size and reject version-only engine identity. Cost: provider/cache/tool prerequisites can stop delivery. |

### Clarification — 2026-10-23, retry2

The source already contains all six implementations. The chief independently
resolved workspace Git HEAD to the exact source SHA above, found a clean
starting worktree, and inspected its four-file diff (405 additions, 30
deletions); `src/pop3.rs` additions are inside its existing cfg(test) module.
The source's parent is `150c84a`, whose parent is `cc30ea2`. The earlier
panel/intake alternate-object visibility failure remains historical evidence;
it did not recur in this chief workspace and is not a current observed block.
Every later verification environment must still resolve and bind its own
candidate; this observation is not a hermetic verification receipt.

The chief also checked the tracked `recipes -> bundles` symlink (mode
120000), and the candidate `.forge/bin/brokkr` digest
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`
against tooling.json; it reports 0.12.0. No prior completed gate or four-review
receipt was found in the available run-local evidence. Tasks below therefore
require retention, validation and fresh delivery proof, not duplicate coding.
Design preflights do not substitute for the pinned verify/check seats.

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
keep SMTP's flag false and shutdown inactive. Establish a real POP3 greeting,
USER/PASS and NOOP and healthy HTTP `/healthz` 200 with status `ok`.
Subscribe to the backoff event before arming one accept error on that POP3 owner's probe, wake its accept with a real connection,
and observe its backoff pending while retry readiness remains held. A real
HTTP GET must return `HTTP/1.1 503 Service Unavailable` and exactly the JSON value
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
no refused mail is queryable. The existing fixtures use no sentinel. Remove the
actual fault, start a fresh envelope on the same connection, and accept
distinct content with fresh sender and recipients. Immediately fetch the
returned accepted ID/raw over HTTP and assert exact inbox, envelope sender,
recipient vector, payload and singleton ID membership. Refused
recipients/content must not leak into recovery.
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

Full4 means correctness Qwen Max, security Claude Opus, guarantees official
DeepSeek Flash, tests DashScope GLM-5.3; never DeepSeek Pro or fallback.
Root owns #177 publication before final integration/CI and reruns proofs on
the integrated candidate. Author commits are unsigned; no push, merge,
tag or registry action is authorized. Drafting is not implementation,
verification, operator acceptance, publication or closure of #192.
