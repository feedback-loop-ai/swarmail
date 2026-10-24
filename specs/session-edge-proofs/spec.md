# Pin SMTP and POP3 session edges

Date: 2026-10-24. Status: residual draft design for NEXT-188, issue #188,
epic #192. Feature slug: `session-edge-proofs` (retain the existing family).
Residual input candidate: `11134f1`; actual reviewed Swarmail base: `45d68df`;
accepted engine-pin commit: `238372d`. Each execution must resolve its own
current HEAD/tree after subsequent documentation commits.

Historical ancestry only: `cc30ea2` was the review-clean #177 integration
candidate for the earlier design `150c84a` and implementation
`e7d1d8142c3e24b2a817822969ab0dae6619680a`. These identifiers explain
inherited work; they are not the current reviewed base or execution binding.
Swarmail remains one Rust crate. This design does not close epic #192.

## Feature and rationale

Pin six existing protocol contracts with real TCP SMTP/POP3/HTTP exchanges
and test fixtures. An error string alone cannot prove that an envelope
survives, a completed DATA transaction is consumed, authentication remains
unchanged, or deleted POP3 content stays unavailable. Each refusal therefore
has a same-session continuation and an observable state witness. Accepted
mail is queried by the exact ID returned by DATA, immediately and without
polling, so the tests cannot conceal an acceptance-before-storage gap.

The 2026-10-24 residual corrects the review record: the quickstart borrowed a
sibling executable and superseded digest, ancestry was not labeled historical,
and a tracked source description could be mistaken for a committed build
receipt. Correct these bindings together so isolated execution proves its
actual candidate and engine. See [tracked provenance](plan.md#engine-provenance).

Triage classified this residual as `engine` because executable gate guidance
and delivery provenance are affected. Under accepted 0057, four independent
reviews remain the smallest permitted route despite the small Markdown diff.

## Scope and accepted authority

This residual edits only `spec.md`, `plan.md` and `tasks.md` in this directory.
Keep AC1–AC6 and their existing witnesses unchanged. The inherited test-only
implementation is a preservation reference, not authorization for fixture
repair in this residual. Preserve #177's owned admission, reap, backoff and
task-completion events. No source, test, public interface, schema, dependency,
engine pin, tooling patch, policy, gate, provider mapping, accepted decision
or unrelated documentation change is authorized. No rebuild, receipt copying
or new provenance machinery is required.

Delivery authorities are accepted
[0045](../../docs/decisions/0045-delivery-repairs.md),
[0050](../../docs/decisions/0050-delivery-gates.md),
[0056](../../docs/decisions/0056-signing-policy.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).

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

The chief read both `.brokkr/state/design/positions/simplicity.md` and
`.brokkr/state/design/positions/robustness.md`, and intake
`.brokkr/state/tasks/epic-192-issue-188-residual-foll-84aa1533.md`.
Both positions support the same bounded correction; there is no genuine
conflict between them to average. The following rulings choose explicitly
among their supported and rejected alternatives.

| Question | Ruling and cost |
|---|---|
| Artifact scope | Adopt simplicity's correction of the existing three-document family, supported by robustness. Reject a second spec family, standalone provenance file or new validator. Cost: history and current instructions coexist and need explicit labels. |
| Binding depth | Adopt robustness's joint correction of candidate identity, executable identity and evidence provenance, also supported by simplicity. Reject a pathname/hash-only patch. Cost: each execution repeats identity checks; the cheaper alternative leaves misleading ancestry and receipt claims. |
| Provenance ownership | Adopt one plan section with tooling.json owning patch order/checksums; name the three patches for navigation as robustness recommends. Reject a second manifest and new rebuild/attestation machinery. Cost: the recorded source composite is not independently reproduced here, and the ignored receipt is not durable tracked evidence. |
| Historical record | Adopt both positions' explicit historical/superseded labels and preserve the dated retry2 observation. Reject global SHA replacement or deletion of history. Cost: some repetition remains, but old observations cannot masquerade as current receipts. |
| Runtime and tests | Adopt both positions' preservation of all six criteria and witnesses. Robustness's crash, webhook, pruning, load, chaos, hostile-byte and certificate analysis sets limits, not new implementation. Reject duplicate tests, fixture repair and broader hardening in this residual. Cost: finite existing traces retain their original coverage limits. |
| Review and gates | Retain the recorded engine route and four configured independent lenses. Reject a one-reviewer shortcut, author self-review, provider fallback and old receipts as fresh proof. Cost: full gates and provider prerequisites exceed the effort of the text correction, as accepted 0057 requires. |

### Clarification — 2026-10-23, retry2 (historical)

This dated observation describes the earlier retry2 candidate only.
The source already contains all six implementations. The chief independently
resolved workspace Git HEAD to the exact source SHA above, found a clean
starting worktree, and inspected its four-file diff (405 additions, 30
deletions); `src/pop3.rs` additions are inside its existing cfg(test) module.
That historical source's parent is `150c84a`, whose parent is `cc30ea2`. The earlier
panel/intake alternate-object visibility failure remains historical evidence;
it did not recur in this chief workspace and is not a current observed block.
Every later verification environment must still resolve and bind its own
candidate; this observation is not a hermetic verification receipt.

The retry2 chief also checked the tracked `recipes -> bundles` symlink (mode
120000), and the candidate `.forge/bin/brokkr` digest
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`
against the then-current tooling.json; it reported 0.12.0. This historical
preflight and e525 digest are superseded by pin commit `238372d` and the
active ab935558 digest in the plan; they prove nothing about this residual. No prior completed gate or four-review
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

## Residual record acceptance

These additional record checks preserve, rather than replace, AC1–AC6:

- R1: The active quickstart runs from the candidate root, hashes its own
  `.forge/bin/brokkr` against unchanged tooling.json and the full active
  `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`,
  resolves that executable absolutely for boxed checks, and compiles with it.
  No active sibling path, PATH fallback or e525 instruction remains.
- R2: Spec and tasks identify `45d68df` as reviewed base and `238372d` as
  pin commit, with `11134f1` as residual input, and label cc30ea2, 150c84a
  and e7d1d81 ancestry historical. Every retained e525 reference is historical
  and superseded. Current execution binds its own HEAD/tree and ancestry.
- R3: The tracked plan names the engine base, ordered patches, recorded source
  composite, compiler and binary digest consistently with tooling.json. It
  states that `.forge/epic192/engine-main-entry/build-receipt.json` is
  gitignored run-local evidence, not tracked or committed, and attributes
  historical build claims without asserting a fresh rebuild or receipt inspection.
- R4: Diff review against `11134f1` finds only these three documents changed,
  with AC1–AC6 and T2–T7 witnesses semantically intact and every protected
  implementation, pin, gate, policy and accepted ruling unchanged.
- R5: Targeted suites, all house gates and protected independent reviews are
  recorded against the actual execution candidate before delivery. Missing
  evidence stays outstanding; inspection, hashing or compile success alone
  cannot satisfy verification or review.

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
