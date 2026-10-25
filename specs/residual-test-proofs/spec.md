# Issue #191: portable SIGTERM delivery proof

Status: Drafted design; implementation and delivery acceptance remain separate.
Date: 2026-10-25.
Feature slug: `residual-test-proofs` (retain the existing owning design).
Scope: Epic #192 issue #191, continuation correcting the contender assertion and its claims only.
Continuation code baseline: `94b8ae15f0b3515d5860266f0988a2a4b75f15b0`.
Design-entry HEAD: `f6a83e6` (continuation intake and gate receipts).

## Feature and rationale

Correct the Linux-only contender assertion in
`tests/binary.rs::sigterm_with_a_delivery_in_flight_prints_the_final_counts`.
The earlier connected-client repair retains a sound exact refusal probe, but
requires a reuse-enabled contender bind to fail. The supplied independent
review explains why BSD/XNU allows that bind against an established endpoint:
the assertion would fail macOS CI even though the connect probe is sound.
Remove that unnecessary contender path, keep the reuse-disabled bind check,
and correct the fixture's overstated exclusion claim.

Retaining both streams preserves the established endpoint's lifetime. The final
bind check demonstrates exclusion of a reuse-disabled contender at that instant.
It does **not** establish exclusion of reuse-enabled binders or listeners on
macOS, including parallel Tokio listeners. The macOS port-reuse race is not
proved fully closed. The exact pre-delivery refusal probe observes setup-time
behavior; it cannot prevent a later reuse-enabled listener from claiming the
address. Existing ledger/cause assertions do not prove every retry fails at
connect or detect every such interference.

The test continues proving one exhausted delivery and one in-flight delivery
aborted by SIGTERM, with successful exit, exact final totals and credential-safe
transport diagnostics. Retain its connected-client target, exact bounded refusal
probe and real binary/SMTP/HTTP experiment. Only the final contender assertion
and directly related comments change in code.

## Evidence and authority

The issue README, operator-rulings.md, macos-ci-failure.md and full
review-78a81fc6.md in
`.forge/epic192/issue191/` were verified against that directory's SHA256SUMS.
The operator's 2026-10-25 ruling authorizes the portability repair and assigns
macOS proof to CI. Both current run-local panel positions were read under
`.brokkr/state/design/positions/`. The requested `.brokkr/state/tasks/` is absent;
the committed [continuation intake](intake-macos-continuation.md) and current
task provide the framing. The prior review's independent findings 1 and 2
identify the incorrect bind contract and remaining macOS reuse risk. Earlier
intake and proof receipts are history, not repaired-code proof.
No GitHub, CI, journal, sibling worktree or external source was consulted.

Preserve accepted 0001–0005: synchronous accept/store, subscribe-before-check,
oldest-only pruning, real isolated protocols and ingest extraction.
[0018](../../docs/decisions/0018-shutdown-observed-waits-and-explicit-bounds.md)
and [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
own observed events, bounded waits and real-protocol test placement.
[0041 §§5–6](../../docs/decisions/0041-webhook-boundary.md) and
[0048 §6](../../docs/decisions/0048-rest-webhook-validation.md) own the unchanged
shutdown ledger, causes and redaction. Follow
[0050](../../docs/decisions/0050-delivery-gates.md) for pinned executed proofs and
[0056](../../docs/decisions/0056-signing-policy.md) for unsigned commits.
Preserve current 0057/0058 bundle routing, independent review and security hold.
These are authority references. No frozen guarantee, public contract or ruling
changes; no semantic ADR is needed and this seat accepts no decision.

## Panel synthesis and rulings

1. **Adopt both current positions: remove the reuse-enabled path.** Replace
   the `[false, true]` loop with one inline IPv4 contender, explicitly set
   `SO_REUSEADDR=false`, and require `bind(refused)` to fail with exactly
   `AddrInUse`. Never listen on it. Preserve explicit final pair release.
   This supersedes the earlier synthesis's ruling for two failed binds; that
   robustness argument extrapolated Linux behavior to macOS and loses.
   Cost of rejecting it: no reuse-enabled exclusion claim, including on Linux;
   the remaining macOS reuse-enabled collision risk is disclosed, not solved.
2. **Reconcile the ownership contract with the evidence.** Both current
   positions agree on the limited claim: uninterrupted lexical retention and
   explicit final drop establish lifetime; the final check observes one
   reuse-disabled bind rejection. No universal port exclusivity or complete
   macOS race closure is promised. The earlier ruling 2 demanding both failed
   binds is replaced by this limit and ruling 1's single check. Update AC4,
   plan step 5, T3 and directly conflicting active prose/comments together.
   A passing macOS CI run proves that execution, not every concurrent schedule.
3. **Rule for deletion over platform-specific expectations.** The intake
   permits Linux failure/macOS success for a reuse-enabled bind. Reject that
   alternative: it adds conditional kernel-policy checks without protecting
   the shutdown experiment. Cost: the test no longer demonstrates platform
   reuse differences. The current positions have no substantive conflict on
   the chosen repair; no compromise assertion is needed.
4. **Adopt simplicity's inline scope and robustness's proof limits.** Keep
   the reuse-disabled connected client, accepted peer, dropped setup listener,
   existing bounded setup and exact `ConnectionRefused` probe unchanged. Reject
   allocators, shared locks, keeper tasks, namespaces, dependencies, production
   seams or serialization: they widen the authorized repair. Cost: the known
   macOS reuse-enabled interference risk remains for separately scoped work
   if CI exposes it. Keep exact totals, causes and redaction; do not accept
   timeout, relax errors, extend budgets, add retries, sleeps or ignores.
5. **Preserve history without certifying the repair.** Retain this feature
   slug, earlier dated clarifications and Linux receipts as historical. Reset
   current implementation/proof tasks. This design corrects a test-spec defect,
   not an accepted runtime ruling, and creates no ADR. CI owns actual macOS
   execution; local receipts cannot discharge that handoff.

## Acceptance Criteria

These are local implementation criteria. This design commit defines them; it
cannot mark the future repair proved. AC numbers retain the owning design's
mapping to the already reviewed #191 findings.

- **AC1 — preserve 23.1.** Dockerfile remains byte-identical to the continuation
  baseline, retaining AC52.2 attribution and its reviewed digest.
- **AC2 — preserve 23.2.** `tests/common/mod.rs` remains byte-identical: helper
  prose stays on its owner and `send_with_host` retains its one required,
  verbatim Host description and implementation.
- **AC3 — preserve 23.3.** The removed `wait_for_metric` wrapper is not restored;
  the existing live metric helpers and consumers remain unchanged. Baseline
  comparison and the workspace suite prove preservation, without new cleanup.
- **AC4 — portable fixture and unchanged ledger.** The named binary test owns
  a reuse-disabled, explicitly bound IPv4 client connected to an accepted
  loopback peer. Both streams remain open from setup through webhook
  configuration, SMTP, retry exhaustion, SIGTERM, EOF/reap and all assertions,
  with an explicit final drop and RAII on unwind. Neither stream is read,
  written, shut down or half-closed. Setup connect/accept and the real refusal
  probe use the existing `HELPER_DEADLINE`. The probe to the client's local
  address must return exactly `ConnectionRefused`; success, expiry or another
  error fails. The same address is used in the unchanged plain HTTP webhook
  URL. Before releasing the pair, one fresh IPv4 socket explicitly using
  `SO_REUSEADDR=false` fails to bind that address with exactly `AddrInUse`;
  it never listens. Remove the reuse-enabled contender path entirely, without
  a singleton loop or new platform conditional.

  The active spec, plan, tasks and related source comments must state the
  macOS reuse-enabled listener limitation above. A source/text comparison
  must find no active requirement that both bind modes fail or that this
  fixture fully closes the macOS reuse race. Historical dual-bind receipts
  are explicitly superseded and do not prove the corrected candidate.

  Preserve the stalled sink's completed-request observation before SIGTERM,
  the fresh-child exhaustion baseline zero and observed exhaustion one, clean
  process exit and exactly one final `webhook dispatcher stopped` message.
  Its delivered/dropped/lagged totals remain zero, exhausted one and aborted
  one. Exactly one exhaustion warning has a substantive transport cause beyond
  its display prefix, and stdout contains none of `hunter2`, `hunter3` or
  `marker-secret`. Preserve existing message-position/format assertions.
  The focused Linux test selects exactly one passing test. No sleeps as
  synchronization, new ignore, broad error match or timeout-as-success.
- **AC5 — preserve the distinct webhook regression.** `tests/webhook.rs`
  remains byte-identical to the continuation baseline. Its held target and stored
  mail/exhaustion observations are existing proof, not a substitute for AC4.
- **AC6 — preserve 23.5.** `src/api.rs` remains byte-identical to the continuation
  baseline, retaining the measured runtime acquisition observer and snapshot
  discriminator. The workspace suite executes it. No new mutation experiment
  or snapshot refactor is authorized by this portability repair.
- **AC7 — scope.** Implementation changes are confined to the named function's
  contender check and directly related explanatory comments relative to
  `94b8ae1`. Setup, bounded refusal probe and final pair release are preserved in
  `tests/binary.rs`. Other code in that file and all shipping code, Cargo files,
  dependencies, allowlist, decisions, gates and bundle routing stay unchanged
  from the continuation baseline. The chief reconciles only these three owning
  design artifacts; the committed continuation intake remains framing, not a
  new edit. Inspect the exact diff; no new support module or production seam.
- **AC8 — completed local proofs and honest handoff.** The focused test, full
  binary suite and every local gate in the
  [sole house inventory](../../docs/house-rules.md#gates--part-of-every-change)
  pass on the repair, using digest-verified `.forge/bin/brokkr`, locked builds,
  exact zero-missed coverage with exit zero, its selftest, publish/seal proofs,
  licenses and fail-closed native-tls absence. Record candidate identity,
  commands, platform, exits and selected-test count. Commit scoped work unsigned
  before verify/review; historical and design-baseline receipts cannot close
  implementation tasks. No push, merge, tag, publication, issue closure or
  decision acceptance.

## External proof ownership

CI owns the existing macOS binary test execution, which must exercise the repair
without adding an ignore. A Linux pass is not a macOS pass. The operator retains
live hosting/settings and sealing authority; CI retains chart/history, Docker,
RustSec, privileged port-80 and release-only proofs named in the house inventory.
These are explicit handoffs, not unprovable local acceptance criteria. No durable
outbox, crash recovery, exactly-once webhook, host-privilege isolation or
unbounded-load latency guarantee is introduced.

## Dated clarification — 2026-10-24, earlier synthesis (historical)

The original `0298c31` design framed four residual repairs prospectively at
`34eb99f`. Implementation `db0b517` is now the inspected baseline, and this
run explicitly includes the already-correct Dockerfile criterion. This revision
supersedes stale prospective wording without asserting any historical task or
execution succeeded. The original design remains available in Git history;
prior framing is untouched. All implementation/proof tasks remain unchecked
until their actual receipts exist.

## Dated clarification — 2026-10-24, rollup-a synthesis (historical)

The preceding entry is retained provenance, not the active baseline or scope.
The active candidate is `ffca01b`, with draft changes relative to `c3d124d`.
Source disproves both the stale unused-wrapper gap and the earlier assertion
that every repair is present: the binary port is still unowned. This revision
supersedes the zero-edit restriction, webhook-only refusal criterion and local
macOS requirement. Preserve valid helper/webhook/snapshot changes and implement
AC4. Gates run during design describe the unrepaired candidate only; final
implementation, mutation and verification receipts remain required.

## Dated clarification — 2026-10-25, behavioural proof re-entry

The two entries above describe earlier candidates only. The active code
baseline is now `68b4ef8`, reviewed by the full 0058 panel in run
`epic-192-issue-191-verify-and-re-6cf3ced1`. Findings 23.1–23.4 are preservation
constraints. The current instruction supersedes the literal discriminator and
old seam prohibition, and explicitly permits a measured `cfg(test)` owner seam.
Ruling 3 and AC6 now require a runtime split-lock discriminator; AC7 permits only
that test support beyond the inline test. No implementation or mutation proof
is claimed by this design. The resolved intake remains historical framing.

## Dated clarification — 2026-10-25, macOS portability re-entry

The entries above are historical and retain their original candidate names.
The active baseline is now `90eb2a0`; findings 23.1–23.3 and 23.5 are reviewed
preservation obligations. The operator's macOS ruling supersedes the old ban
on editing this binary fixture and AC4's unconnected-bound-socket requirement.
The current three artifacts govern this repair; earlier mutation tasks and
receipts neither reopen 23.5 nor certify the new fixture. The operator-rulings
file is now present and checksum-verified. The historical tasks directory
remains absent; current committed intake supplies the framing.

## Dated clarification — 2026-10-25, reuse-enabled bind correction

All preceding dated entries and the prior implementation receipt in tasks.md
are historical. The active continuation baseline is `94b8ae1`; design entry is
`f6a83e6`. The full supplied review supersedes the earlier synthesis's ruling 2,
AC4's dual-bind clause, plan step 5 and T3. Their Linux success did not establish
macOS exclusion. Keep the connected-client target and exact refusal probe;
remove the reuse-enabled contender and explicitly retain the macOS race limit.
All current implementation tasks require fresh proof after the correction.
This design commit does not change the test or claim macOS execution.

## Design handoff

Commit exactly this spec, [plan](plan.md) and [tasks](tasks.md), leaving the
design commit last. Positions, result JSON and gate logs remain ignored.
The sequence's pinned deterministic check owns advancement. The chief reports
only `drafted`; design-baseline gates do not establish repair completion.
