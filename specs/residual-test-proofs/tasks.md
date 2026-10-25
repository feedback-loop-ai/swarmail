# Ordered tasks: portable webhook refusal fixture

All current tasks are implementation/verification work and remain unchecked.
The chief changes design artifacts only. Historical receipts below describe
prior candidates; they cannot close these tasks or certify macOS execution.

- [ ] T1 — Confirm authority and baseline (AC4–AC6). Proof: packet SHA256SUMS and pinned engine digest pass; record HEAD/platform/status and compare to ac160ff. Inspect current functions plus available db0b517/35afa14 history; record the bounded reservation inventory and unavailable 98b54e51 historical object without external lookup.
- [ ] T2 — Replace the webhook fixture with the retained connected pair (AC1). Proof: source/diff inspection shows an ephemeral listener, explicitly bound reuse-disabled IPv4 client, bounded connect/accept, dropped listener and client local address shared by probe/URL. Both streams remain untouched until explicit final pair release. The focused real-server test in plan.md selects exactly one passing test.
- [ ] T3 — Preserve the refusal oracle and correct fixture claims (AC2). Proof: the same focused test requires exact ConnectionRefused under the existing deadline. Source inspection finds connected-endpoint diagnostics, the macOS reuse-enabled listener/setup-only limitation, no new contender assertion, broader errors, sleeps, ignores, retries or platform branches.
- [ ] T4 — Preserve SMTP/storage/retry behavior (AC3). Proof: the focused test retains PUT success, the baseline before SMTP, SMTP success, HTTP/count one before observation, unchanged delta-one wait and HTTP/count one afterward. Diff confirms HTTPS scheme, budgets, deadlines and observations are unchanged; do not reinterpret the helper's >= predicate as exact exhaustion equality.
- [ ] T5 — Audit complete suites and scope (AC3–AC5). Proof: locked full webhook and binary suites exit zero with existing ignore policy. Inspect baseline diff and functions: only the named webhook fixture/lifetime/comments/diagnostics change; binary, common helpers, API and Dockerfile are byte-identical. Record zero remaining unconnected refused reservations in #191's changed test surface, distinguishing the SIGTERM bind contender and unrelated fixed-port test.
- [ ] T6 — Commit and execute all local proofs (AC5–AC6). Proof: scoped unsigned implementation commit followed by every house gate in plan.md with completed zero exits, exact zero-missed coverage and selftest, publish/seal proofs, licenses, engine/shim integrity, design selftest, delivery proof and strict native-tls absence. Save commands/platform/commit/tree/output/exits; reject policy-sweep warnings and partial timeout summaries.
- [ ] T7 — Hand off the committed candidate (AC5–AC6). Proof: clean intended worktree and fresh receipts for T1–T6 bind to the committed code; any later code change reruns affected proof. Preserve recipe routing, independent review and security hold. Record CI-owned macOS/external gates and operator-owned settings/sealing/publication without claiming those executions. No push, merge, tag, publication, issue closure or decision acceptance.

## Historical task receipts

Everything below until the new webhook clarification is preserved provenance.
References to current tasks or completion apply only to those earlier phases;
T1–T7 above exclusively govern this repair.

## Dated clarification — 2026-10-25, earlier 23.5 proof (historical)

Before portability re-entry, the owning tasks recorded completion of the
behavioural snapshot repair at `90eb2a0`: the identical focused scrape test
passed, rejected a compiling split-lock mutant at two real acquisitions versus
one, and passed after exact restoration. Its recorded coverage was
13,871/13,871 lines, 782/782 branches and 1,512/1,512 functions, with local
gates and native-tls absence passed. Receipts were recorded under
`.forge/epic192/issue191/implementation-235/`. This preserves the earlier
completion record, not a new audit of those receipts or proof of the macOS
fixture. The original checked task list remains in Git history. It is not
reopened; T1–T7 above govern only the new portability repair.

## Implementation proof — 2026-10-25 (historical, candidate 94b8ae1)

The implementation commit containing this record changes only the named
SIGTERM test and this checklist. On Linux x86_64, based on design HEAD
`f5b29d4`, the focused locked test selected exactly one passing test; the full
binary suite passed 40 tests with its two existing ignores. The fixture keeps
both established streams until the end, requires exact `ConnectionRefused`,
and rejects competing binds with reuse disabled and enabled. Existing webhook
configuration, SMTP, observed exhaustion, signal/EOF/reap, ledger and redaction
assertions are unchanged. Protected files and other binary regions match
`90eb2a0`; no production, dependency, decision, gate or routing change occurred.

All house commands in plan.md exited zero on the final Rust code: fmt, locked
clippy and workspace tests, digest-verified `.forge/bin/brokkr` compile (76
valuations, zero unruled), exact coverage and its selftest, publish/seal proofs
and licenses. Shim integrity, pinned design selftest, delivery tamper proof and
the strict native-tls absence diagnostic also passed. Coverage evidence is
`target/coverage/run.IB3UuU`: 14,030/14,030 lines, 790/790 branches and
1,531/1,531 functions, with zero missed in each dimension. The workspace RPC timed out while the command continued; completed
per-command exit records, rather than that RPC outcome, establish the verdict.

Candidate identity, commands, timestamps, zero exits, scope comparisons and
raw logs are retained under the ignored directory
`.forge/epic192/issue191/implementation-macos-gates/`. Both candidate.json and
receipt.json identify the tested `tests/binary.rs` SHA-256 as
`eafb705d1b15b51cd57f955d77089c2b78af6dd9fe5fc9debba5a44cd4ad296a`.
This checklist and the fixture are committed unsigned together under 0056.
CI owns the actual macOS run and the other external proofs in the house
inventory; the operator owns sealing and publication. Those executions and
independent verify/review remain handoffs, not claimed local results.

## Dated clarification — 2026-10-25, continuation reset

The historical implementation receipt above describes Linux candidate `94b8ae1`
and its now-rejected dual-bind assertion only. The current panel and chief
remove the reuse-enabled assertion and its stronger ownership claim; current
T1–T7 require fresh evidence. No production behavior or accepted decision is
changed. The chief commits only spec.md, plan.md and tasks.md; its own local
gate receipts exercise the still-unrepaired test and cannot close these tasks.

## Continuation implementation proof — 2026-10-25

Committed code candidate `7266cf992eaee849a5e72a2127be23b4349c3147`
(tree `d311d45113dda9783f26bd303a84aa1ed6994bf0`) passed all local proofs on
Linux x86_64 with `CARGO_NET_OFFLINE=true`. This completion-record commit
changes only this checklist; the tested binary source stays SHA-256
`934627ad03b7376c7f0b795713530e87d3ed304579dc1fe9988e2d8686d8eb84`.

The focused locked invocation selected exactly one passing test; the full
binary suite passed 40 tests with the two existing ignores. The contender
path is now one IPv4 socket explicitly setting reuse false and requiring
exactly `AddrInUse`. The connected pair, bounded setup/refusal probe, webhook
URL, SMTP, observations, SIGTERM, EOF/reap, ledger/cause/redaction assertions
and final pair release are unchanged. Scope comparison against `94b8ae1`
confirms protected files and every other binary region are byte-identical.
The existing corrected spec ruling 2/AC4 and plan step 5 agree with the code:
retention and reuse-disabled rejection do not exclude reuse-enabled macOS
binders/listeners, including parallel Tokio listeners. Setup refusal is a
point observation; full macOS reuse-race closure is not established.

Every house gate exited zero on that committed candidate: format, locked
clippy and workspace tests, digest-verified candidate-local brokkr compile,
exact coverage and its selftest, publish/seal proofs and permissive-license
check. The pinned compile evaluated 76 valuations with zero unruled and no
policy-sweep warning. Shim and packet checksums, pinned design selftest,
delivery tamper-boundary proof and strict native-tls absence also passed.
Coverage completed with 14,030/14,030 lines, 790/790 branches and
1,531/1,531 functions, zero missed in each dimension; its report is
`target/coverage/run.gwP1xN`. As prescribed in plan.md and 0050, proof scripts
ran from `bundles/self/scripts/` after verifying public shim digests.

Fresh command arguments, timestamps, platform, candidate identity, raw logs,
completed zero exits and scope proof are retained under ignored
`.forge/epic192/issue191/implementation-contender-gates/` in `receipt.json`,
`candidate.json`, `scope.json` and adjacent logs. The workspace RPC timed out
before returning the batch; the subsequently inspected persisted receipts
and final done marker establish completed exits, not the RPC timeout.

Both implementation and this proof record are committed unsigned under 0056.
Independent verify/review are subsequent phase handoffs. Actual macOS
execution and the other charter-owned external proofs remain with CI;
live settings, sealing and publication remain with the operator. No macOS
pass, push, merge, tag, publication, issue closure or decision acceptance is
claimed. Production, dependencies, gates and recipe routing remain unchanged.

## Dated clarification — 2026-10-25, webhook repair reset

PR #238 macOS evidence supersedes the webhook-preservation obligation from
the binary continuation. The new tasks repair only the separate webhook
fixture and preserve the corrected SIGTERM test. Prior green Linux gates and
binary receipts do not certify the new implementation. The chief's design
commit leaves every current implementation task unchecked.
