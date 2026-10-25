# Ordered tasks: macOS SIGTERM contender correction

These continuation implementation tasks are complete with the fresh local
proof recorded below. Earlier completed work and Linux receipts remain
historical; their reuse-enabled assertion is superseded. Only the current
continuation receipt certifies this correction locally; no macOS execution is
claimed. Gate logs and panel positions stay ignored; CI owns the macOS run.

- [x] T1 — Confirm authority and scope (AC1–AC3, AC5–AC7). Proof: verify packet SHA256SUMS and engine digest; record HEAD/platform/status; compare the candidate to `94b8ae1`. Protect earlier Dockerfile/helper/webhook/snapshot work and all code outside the named test. The T6 workspace suite executes existing regressions.
- [x] T2 — Preserve the connected fixture and exact refusal (AC4/AC7). Proof: source diff shows unchanged setup, bounded connect/accept, same client address and bounded exact `ConnectionRefused` probe. The focused command in plan.md selects exactly one passing test. Both streams remain local through all assertions with unchanged final drop.
- [x] T3 — Remove the reuse-enabled contender path and correct its claims (AC4). Proof: the focused real-server test requires one explicitly reuse-disabled IPv4 contender bind to fail with exactly `AddrInUse`, never listening, before final pair release. Source/text comparison confirms no dual-bind loop, singleton loop, new platform conditional or universal exclusion claim. Spec ruling 2/AC4, plan step 5 and source comments agree: retained lifetime and reuse-disabled rejection do not exclude reuse-enabled macOS binders/listeners (including parallel Tokio listeners), do not fully close that reuse race, and the setup refusal probe is only a point observation.
- [x] T4 — Preserve shutdown and diagnostic observations (AC4). Proof: the same focused test observes the stalled request and exhaustion before SIGTERM, successful EOF/reap and exit, one final message with 0/0/0/1/1 totals, one substantive cause warning and absence of all three fixture secrets. Source diff confirms unchanged formatting assertions, URLs, budgets, deadlines and no new ignore, sleep, retry or relaxed error.
- [x] T5 — Exercise the full binary suite and audit scope (AC1–AC7). Proof: `cargo test --locked --test binary` exits zero and the focused run selected exactly one pass; diff from `94b8ae1` limits code edits to the contender check and directly related comments. Protected files and all other binary regions remain byte-identical. Historical prose is explicitly distinguished from active requirements.
- [x] T6 — Execute every local house proof on the corrected candidate (AC8). Proof: all charter gates exit zero, digest-verified candidate-local engine compile has no sweep warning, exact coverage reports zero missed lines/branches/functions with completed exit, and its selftest plus publish/seal proofs and licenses pass. Verify shims, pinned design selftest/delivery proof and strict native-tls absence per plan.md. Save command/platform/candidate/output/exit receipts; design or earlier Linux runs cannot close this task.
- [x] T7 — Commit and hand off the scoped repair (AC7/AC8). Proof: unsigned implementation commit, clean intended worktree and current receipts for T1–T6; final verify binds to that committed candidate. Preserve recipe routing, independent review and security hold. Record CI's macOS and external proof ownership without claiming a pass. No push, merge, tag, publication, issue closure or decision acceptance.

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
