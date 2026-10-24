# Plan: recover the metric amendment handoff

Implements [spec.md](spec.md). Baseline: `faef809`. The checkout lacks
`.specify/templates/plan-template.md`; its required sections are supplied here.

## Technical context

One Rust crate, Markdown specifications and retained run-local JSON reviews.
`src/api.rs::count_in_flight` takes request/total guards before `next.run`;
`inbox_of_path` still returns the raw selected segment. `await_messages`
admits the decoded name before `park_await` increments active/cumulative
await accounting. `prom_label` escapes backslash, quote and LF, passing CR/NUL
literally. No runtime change is part of this plan. `tests/api.rs` already
owns the independent complete-scrape parser and real HTTP fixtures.

## Constitution check

Accepted 0065 supplies authority; recovery does not amend it. Preserve frozen
0001–0003 and ingest extraction 0005, 0043's accounting/escaping and 0047's
cap/overflow. [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
controls any future seam/harness placement; no source seam is added here.
[0050](../../docs/decisions/0050-delivery-gates.md) binds proof to execution and
design identity to the chief's HEAD. No dependency, exclusion, allowlist,
gate, recipe or policy changes. Unsigned commits under 0056; never push.
All house gates and independent security disposition remain required.

## Project structure and files touched

Chief commit: exactly `specs/metric-recovery/{spec,plan,tasks}.md`.
Downstream correction: these six existing files, plus dated evidence/task
updates in this recovery directory as needed; no other tracked files by default.

| File | Correction |
|---|---|
| `specs/http-plane-edges/spec.md` | Insert exactly the missing blank line. |
| `specs/record-metric-amendment/plan.md` | Append dated owning receipt, proof mapping and narrowly scoped historical-comparison clarification. |
| `specs/record-metric-amendment/tasks.md` | Reconcile checks to whole proofs; add dated current-route clarification without erasing historical mapping. |
| `specs/decoded-inbox-metrics/spec.md` | Dated AC1/AC5 refused-control exposure and future proof requirement. |
| `specs/decoded-inbox-metrics/plan.md` | Append deterministic observation/parser/cleanup design; retain both historical receipts verbatim. |
| `specs/decoded-inbox-metrics/tasks.md` | Pair the future refused-control proof with an unchecked task, preserving unchecked T2–T8. |

Accepted decisions/index, source and the HTTP-plane plan remain unchanged.
`tests/store_contract_docs.rs` and `tests/api.rs` are read-only context unless
an identified current-contract test defect requires a justified test-only
scope update and its proof; no such defect is established by this design.
Run-local positions, logs and assigned result JSON remain ignored.

## Research and resolved choices

Both positions, recovery intake, retained reviews, decision index and relevant
rulings establish a documentary residual, not a request to implement decoding.
The prior guarantees receipt is under the runtime design, while recording
T1–T7 remain unchecked. Prior gate success is attributed history. No reviewer
can complete a task whose whole proof is still absent.

Use the existing independent `parse_prometheus` for the future test. It accepts
literal CR/NUL in quoted label values and refuses raw LF/illegal escapes.
Require an independent valid CR/NUL fixture plus retained malformed controls;
no new parser or renderer-derived oracle. No live transient refused-control
proof is claimed by this design. A parser disagreement goes back to review;
it never authorizes changing accepted escaping.

## Data model and evidence ownership

No runtime or persistent model change. A recovery receipt maps each recording
task to its entire paired test, historical candidate or fresh candidate,
command/outcome and unresolved portion. Keep historic candidate identities
and failed receipts intact. Record fresh evidence against the actual tree;
mark independent review pending until the journal supplies its result.

Recording T4's historical reconstruction now permits exactly AC1's additional
blank line. Remove only known amendment blocks and that specific inserted
separator when comparing to the original `59a37f2` bodies. Do not normalize
all whitespace or impose raw equality that rejects the authorized repair.
Scope comparisons distinguish the old recording change from this recovery's
six-file correction and three new design artifacts.

## Contracts and proof design

For AC1, read `git show faef809:specs/http-plane-edges/spec.md`. Assert the bad
adjacency occurs exactly once, the good blank-line boundary is absent there,
and the corrected file equals that baseline with just the one insertion.
This is a meaningful negative control without mutating baseline history.

For AC2/AC4/AC5, save baseline excerpts of both decoded-plan receipts and
compare them byte-for-byte after appending clarification. Compare all accepted
decision files/index against `faef809`; compare the original five 0065 ruling
clauses against the recording task's `59a37f2` source when mapping that task's
proof. Check the complete allowed path inventory and relative links. Preserve
manual 0065-to-0043 scope review as distinct from passing inventory tests.

For the AC3 handoff, a fast refusal has no parked-await window. Future runtime
work must hold execution after request guards are acquired and before they
release, observe a real full HTTP scrape during that hold, then release and
join. Neither an incomplete request, unread response, client disconnect,
sleep nor repeated attempts to catch a fleeting row proves this interval.
If public mechanisms cannot hold it, resolve a test-only middleware-owner
seam under 0020: branch-stable tests stay inline; tests with cold failure
paths use the owner's cfg(test) harness with real router/HTTP. Public tests
must not require a crate-private seam. This recovery adds none of that source.

The future fixture needs an exclusive decoded CR/NUL name and encoded alias,
full-executable cumulative headroom or isolated process, exact named counts,
strict parsing, and no await registration before/during/after refusal. Prove
no cumulative increment; if an overflow case is separately exercised, account
for overflow explicitly. Overflow is never a substitute for the required
named request sample/headroom. Bound all requests, hold/readiness, scrape,
release and joins; own clients/server for cleanup on timeout or assertion
failure. Do not assert whole-process request totals or reset registries.
Existing admitted hostile labels and invalid-UTF-8 compatibility remain
separate proofs. Future runtime work must demonstrate a behavioral failure
with the old raw helper, then success with accepted decoding.

## Quickstart and validation

1. Capture baseline/historical proof fixtures; apply the six bounded repairs.
2. Run the focused boundary negative/positive check, historical-byte/link/path
   checks and `git diff --check`; run `cargo test --locked --test store_contract_docs`.
3. Run every current local gate in the authoritative
   [house-rule inventory](../../docs/house-rules.md#gates--part-of-every-change)
   with locked/offline tooling, exact zero-missed coverage, selftest, publish
   and seal proofs, pinned compile and fail-closed native-tls absence. Resolve
   the candidate-local `.forge/bin/brokkr` digest against tooling.json before
   compilation. Retain fresh logs and owning receipts; no installs or waived gates.
4. Submit through the engine-selected full review; preserve protected review,
   security hold and the exact task-authorized reviewer/fallback limits.
   Dispatch success alone is not review proof. CI retains its chart, Docker,
   advisory, tag release-rate and real port-80 responsibilities.

The chief can validate this design and run local gates, but the six repairs
and eventual candidate verification belong to downstream phases. Final design
validation must use HEAD touching exactly this one spec directory; no later
unrelated commit may replace the chief commit before the check.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Green old receipts become fresh claims | Name historical candidates, append dated owning evidence, and require whole paired proofs before checking tasks. |
| Future prescription mistaken for tested runtime | Keep runtime T2–T8 unchecked; explicitly disclose absent decoding/transient proof and independent security disposition. |
| Fleeting refusal escapes the test | Require deterministic guard hold, exact full-scrape parse, deadlines and cleanup; accept future seam/harness work if needed. |
| Shared registries hide registration or overflow | Exclusive names and executable-wide headroom audit; conditional process isolation, no resets/global totals. |
| Literal controls induce an accidental contract rewrite | Preserve renderer/admission/identity and raw-fallback collision; retain the security finding for independent assessment. |
| Blank-line fix breaks historical equality | Permit only the exact separator in comparison; retain every original byte and both receipts. |
| New decision consumes a reserved number | Cite accepted 0065 as authority only; byte-compare decisions/index and preserve all reservations. |
| Documentation test overstates semantic coverage | Keep the finite AMENDMENTS limitation explicit and directly review 0065-to-0043 scope/links. |
| Gate pass is mistaken for delivery completion | Fresh eventual-candidate gates and actual four-lens review remain required; CI-only proof ownership stays explicit. |

## Chief design validation — 2026-10-24

Fresh local execution on `faef809` plus this three-file design draft passed
`cargo test --locked --test store_contract_docs` (19/19), formatting, locked
all-target Clippy with warnings denied, locked workspace tests, pinned bundle
compile, exact coverage (100% lines/branches/functions, zero missed), coverage
selftest, publish-guard proof (51 mutation-red/restored-green receipts),
seal-branch proof (7 cases, 10 mutants), license check and fail-closed
native-tls absence. The dependency-tree command had to succeed before its
package inventory could establish absence. Cargo used `CARGO_NET_OFFLINE=true`.
The candidate-local `.forge/bin/brokkr` matched tooling.json's SHA-256
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`;
compile reported zero unruled valuations. Run-local commands, exit statuses
and logs are in `.brokkr/state/design/metric-recovery-checks/`.

The long workspace call exceeded its 300-second response timeout, but the
runner continued: the subsequently inspected results.json records all eleven
checks with exit zero, and their logs corroborate completion. No timeout was
counted as a passing command. Structure/link checks passed; the paragraph
assertion rejects the unchanged baseline and accepts the in-memory single-line
repair. Existing tracked files remain unchanged. These checks were followed
only by clarification that source remains read-only and this receipt; the
chief's final committed-artifact check follows the commit.

These are design-phase checks, not evidence that the six recovery edits or
future decoded/control-label behavior have been implemented. Downstream tasks
remain unchecked and require fresh eventual-candidate proof. Independent
review/security disposition and the CI-owned proofs remain outstanding.
