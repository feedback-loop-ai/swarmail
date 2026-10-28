# Pin SMTP and POP3 session edges

Date: 2026-10-23. Status: integrated draft design for NEXT-188, issue #188,
epic #192. Run: `epic-192-next-188-integrated-ver-7bc38731`.
Retain the short feature slug `session-edge-proofs` and revise its existing
three artifacts in place.

Starting candidate: `930bf534cb18e05f4c85e28eed3473297d159743`, tree
`2ba9b8a5f1294adb6c41d8649ce60e70201525a1`. Local `origin/main` and
merge-base: `df8630f69a390fc6a35e1dfc8c54f2f40dd7e624`. The three
issue-188 commits above that base are `120ef3f`, `45d7a2f`, `930bf53`.
This is local identity, not a network-freshness claim. Later receipts must
bind their actual final candidate, including this design commit and any
permitted repairs; this starting identity is not their verification receipt.

## Feature and rationale

Pin six existing protocol contracts with real TCP SMTP/POP3/HTTP exchanges
and test fixtures. An error string alone cannot prove that an envelope
survives, a completed DATA transaction is consumed, authentication remains
unchanged, or deleted POP3 content stays unavailable. Each refusal therefore
has a same-session continuation and an observable state witness. Accepted
mail is queried by the exact ID returned by DATA, immediately and without
polling, so waiting for eventual visibility cannot mask a missing accepted record.

Triage selected full design and four reviewers because session state,
authentication precedence, persistence refusal and listener observation need
contract interpretation. Test-only scope does not reduce that review burden.
No production defect has been reproduced by this design phase.

## Scope and accepted authority

Implementation is confined to the four existing test/fixture files named in
[plan.md](plan.md). Any `src/pop3.rs` edit stays strictly inside its existing
`#[cfg(test)] mod tests`. No extra source/support paths or sibling harness
are permitted. Preserve the integrated base's owned admission, reap, backoff
and task-completion events. No production behavior, public interface, storage schema, dependency, policy, gate,
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
[0046](../../docs/decisions/0046-smtp-pop3-edges.md), plus
[0015 as corrected by 0045](../../docs/decisions/0045-delivery-repairs.md)
for committed POP3 prefixes and marked suffixes. Delivery authority is
[0050](../../docs/decisions/0050-delivery-gates.md),
[0056](../../docs/decisions/0056-signing-policy.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).
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

Read both run-local positions, `.brokkr/state/design/positions/simplicity.md`
and `.brokkr/state/design/positions/robustness.md`, and intake
`.brokkr/state/tasks/epic-192-next-188-integrated-ver-7bc38731.md`.
Both positions support the existing six proofs and full review. There is no
genuine disagreement on protocol semantics; the conflict is between this
commission and obsolete execution directions in the inherited design.
These rulings choose explicit alternatives rather than averaging them:

| Question | Ruling and cost |
|---|---|
| Reimplement or retain | Adopt simplicity's reuse of the existing tests and spec family, supported by robustness. Reject a duplicate suite or design family. Cost: audit inherited tests and obtain fresh execution evidence; source presence alone is insufficient. |
| Proof depth | Adopt robustness's causal, same-session transitions and immediate accepted-ID/raw/provenance witnesses, also supported by simplicity. Reject prefix-only, count-only, reconnect and eventually-visible substitutes. Cost: longer explicit exchanges; the cheaper alternative could conceal envelope loss or identity mutation. |
| Health observation | Adopt both positions' owner-local probe, subscribe-before-arm order, held retry readiness and exact parsed JSON. Reject direct flag mutation, API-side adapters, polling and serialized JSON member-order requirements. Cost: real HTTP wiring and deliberate fixture ownership; harmless key reorderings remain allowed. |
| Harness placement | Adopt both current positions and the explicit commission: retain the stable health proof inline and measured, with unconditional release/abort and success joins. Reject the inherited optional sibling-harness escape. Cost: a necessary cold failure arm that cannot meet scope and exact coverage requires re-triage, not relocation, a coverage exclusion or weaker cleanup. This does not amend 0020's general placement rule. |
| Fixture architecture | Adopt simplicity's existing bounded raw reader, concrete assertion helpers, two independent DATA 451 fixtures and real SQLite triggers. Reject generalized fault/state frameworks, new files, feature flags and dependencies. Cost: readable repetition and schema-coupled triggers. |
| Robustness scope | Adopt robustness's failure analysis as proof limits and structural review obligations. Reject new crash/webhook/TLS/burst paths absent re-triage. Cost: finite traces do not prove arbitrary schedules, exactly-once SMTP, durable webhooks or global resource bounds. |
| Candidate and delivery | Adopt both positions' current-candidate binding, full gates, pinned offline verify and four actual independent reviews. Reject stale ancestry prerequisites, old receipts and version-only binary checks. Root owns final integration/CI with no separate #177-first prerequisite. Cost: real provider/tool/cache failures leave obligations outstanding. |

### Clarification — 2026-10-23, retry2

Historical entry preserved below. It records the previous run, not the
current candidate or a current execution prerequisite. In this entry only,
"source SHA above" refers to `e7d1d8142c3e24b2a817822969ab0dae6619680a`;
its historical design parent/base were `150c84a`/`cc30ea2`. Its observations
and receipts cannot satisfy this integrated run.

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

### Clarification — 2026-10-23, integrated candidate

The chief independently resolved the starting HEAD/tree/local base recorded
above, inspected the clean seven-file branch diff (three specs and four
test/fixture files), and confirmed the 129 added POP3 lines are inside the
existing test module. The exact `.forge/bin/brokkr` digest matches the task
and `bundles/self/tooling.json`. Both current positions and intake identify
all six proofs as already implemented. Retain them; only a demonstrated
fixture defect permits a narrow repair within the listed files. The previous
stale-#177 run is historical context only. Current paths, scope and delivery
requirements here supersede its execution instructions. No test, house gate,
hermetic verification or independent-review pass is claimed by this entry.

## Acceptance Criteria

All six protocol criteria and the delivery criterion are mandatory. Fixed
SMTP replies below include CRLF on the wire; the PLAIN prompt must retain its space. Use exact single-line
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

### AC7 — Fresh integrated delivery evidence

Record final candidate commit/tree/base, clean scope audit, each command,
exit and nonzero executed counts for `smtp_edges`, `pop3`, `persistence` and
the library health test. Every gate in the house inventory must pass,
including exact zero missed lines, branches and functions and its selftest;
native-tls absence must be fail-closed. Actual pinned offline verify must
install nothing, establish binary/shim identity and execute the pinned proof
chain, including design-check selftest and delivery tamper-boundary proof.
Local preflights and earlier receipts do not replace that dispatch.

Require actual independent final-candidate reviews: Qwen Max correctness,
Claude Opus security, official DeepSeek Flash guarantees and DashScope
GLM-5.3 tests. No DeepSeek Pro, model/provider fallback or synthesized receipt.
A security hold stops delivery. After a repair, refresh affected evidence and
reviews with explicit final-candidate binding. Missing tools/cache/provider
results remain outstanding. Root retains integration and CI-owned checks in
their applicable contexts; this design is not a completion claim.

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
Root owns final integration/CI and reruns required proofs if integration
changes the verified candidate. No separate #177 landing prerequisite applies.
Author commits are unsigned; no push, merge, tag or registry action is authorized. Drafting is not implementation,
verification, operator acceptance, publication or closure of #192.
