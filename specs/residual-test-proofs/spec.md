# Issue #191: portable webhook refusal fixture

Status: Drafted design; implementation and verification remain outstanding.
Date: 2026-10-25.
Feature slug: `residual-test-proofs` (retain the owning design).
Run: `epic-192-issue-191-give-the-webh-4ea45493`.
Code scope baseline: `ac160ff4a74a975226d58dd447f5f107327694f6`.
Design-entry HEAD: `b7196b8`.

## Feature and rationale

Repair `tests/webhook.rs::refused_connection_never_blocks_the_smtp_path`
using the connected-client fixture already present in
`tests/binary.rs::sigterm_with_a_delivery_in_flight_prints_the_final_counts`.
The supplied PR #238 macOS evidence records that SIGTERM passes, while the
webhook test's bound, unconnected target produces TimedOut instead of the
required ConnectionRefused. A connected client endpoint preserves a held
address while supplying the evidenced portable setup.

The webhook test must continue proving that successful SMTP delivery is
queryable despite webhook failure and remains stored after observed retry
exhaustion. The repair changes only fixture construction, retention and
related comments/diagnostics. Preserve the HTTPS URL and all existing
PUT, SMTP, storage and retry observations. The passing binary test stays
byte-identical. Completed #191 helper, Dockerfile and snapshot fixes remain
preservation obligations, not new work.

## Evidence and authority

All five files in `.forge/epic192/issue191/SHA256SUMS`, including README.md,
operator-rulings.md and macos-ci-238.md, were verified from that directory.
An initial root-relative checksum invocation used the wrong directory;
the successful directory-local check establishes integrity. The operator's
2026-10-25 #191 ruling directs portability repair and assigns macOS proof to CI.
Both current `.brokkr/state/design/positions/{simplicity,robustness}.md` were
read. `.brokkr/state/tasks/` is absent; the committed
[current intake](intake-webhook-macos.md) and explicit task supply the framing.
No framing input remains unresolved. No external service, journal or sibling
worktree was consulted.

Preserve accepted synchronous accept/store (0001), subscribe-before-check
(0002), oldest-only pruning (0003), real isolated protocols (0004) and ingest
extraction (0005). Relevant authority:
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
for real-protocol test placement and observed synchronization,
[0041](../../docs/decisions/0041-webhook-boundary.md) for the unchanged
best-effort webhook boundary,
[0050](../../docs/decisions/0050-delivery-gates.md) for pinned proofs and
[0056](../../docs/decisions/0056-signing-policy.md) for unsigned commits.
No frozen guarantee, public contract or ruling changes; no new ADR is needed.
This seat accepts no decision and preserves candidate bundle routing.

## Panel synthesis and rulings

1. **Adopt both positions' connected pair.** Explicitly bind a reuse-disabled
   IPv4 client, connect it to a temporary loopback listener, accept its peer,
   drop the listener and target the client's local address. Retain both
   streams through the final storage assertion. Cost: a temporary listener,
   two retained descriptors and several duplicated setup lines.
2. **Adopt simplicity's local scope and robustness's lifetime discipline.**
   Keep ownership inline, with explicit final pair release and RAII on unwind.
   Reject shared helpers, keeper tasks, allocators, global locks,
   serialization, namespaces, dependencies and production seams. Cost of
   rejecting abstraction: the two short fixtures can drift and must be
   compared when maintained. Full port isolation remains outside this repair.
3. **Adopt robustness's exact oracle and both positions' macOS limitation.**
   Preserve bounded real connect and exact ConnectionRefused equality.
   Reject accepting arbitrary errors, longer deadlines, retry-until-pass,
   platform skips, sinks that accept connections, sleeps and new ignores.
   Cost: resource/setup/interference failures remain visible test failures.
   Holding both ends excludes reuse-disabled binders, but macOS reuse-enabled
   listeners, including parallel Tokio listeners, can still claim the endpoint.
   The probe observes setup only; it neither closes that race nor proves the
   failure stage of every subsequent webhook attempt.
4. **Reject any new contender assertion.** Both positions agree that the
   binary test's existing reuse-disabled check must remain there unchanged
   and must not be copied into this webhook test. A reuse-enabled exclusion
   assertion contradicts the supplied macOS limit. Cost: this test does not
   separately measure kernel bind policy; it retains the requested real
   refusal oracle and lifetime proof.
5. **Reconcile robustness's proof wording with the existing observation.**
   Preserve `wait_for_metric_from(..., exhausted_base, 1)` unchanged: its
   predicate is `value >= baseline + delta`. One delivered mail and one target
   motivate the expected exhaustion, but that call is not an exact-equality
   assertion. Do not add an assertion to satisfy a stronger reading of
   "exactly one new exhaustion." Preserve exact storage counts of one.
   Cost: no stronger counter or scheduling proof is claimed. Reject expanding
   robustness's crash, chaos, hostile-DATA and TLS analysis into new tasks;
   those existing contracts remain intact and outside this fixture's proof.
6. **Keep history and reset current tasks.** The positions have no substantive
   conflict on the implementation. Their shared recommendation wins over
   broader hardening; no averaged alternative is needed. This revision
   supersedes the previous active binary-only scope and webhook-byte-identity
   requirement. Dated history below remains evidence about earlier candidates,
   never proof of this repair. CI owns macOS execution.

## Acceptance Criteria

All criteria below are locally checkable on the implementation candidate.
This design establishes requirements, not their completed implementation.

- **AC1 — connected target and lifetime.** The named webhook test creates an
  ephemeral IPv4 loopback listener and an explicitly bound IPv4 TcpSocket
  with reuse disabled. Connect and accept each use `common::HELPER_DEADLINE`.
  Drop the setup listener, derive the target from the connected CLIENT's local
  address, and use that identical address in the refusal probe and HTTPS URL.
  Both connected streams remain untouched through the final storage assertion,
  then `drop((reservation, peer))`; neither is closed, half-closed or released
  early. Prove by source/diff inspection and the focused real-server test.
- **AC2 — honest refusal oracle.** Keep the bounded TcpStream connect probe
  and exact `std::io::ErrorKind::ConnectionRefused` equality. Connection
  success, expiry or any other error fails. Diagnostics describe a connected
  client endpoint. Add no contender assertion, sleep, probabilistic retry,
  platform branch, ignore, deadline extension or broader error acceptance.
  The fixture comment states the macOS reuse-enabled listener limit and
  setup-only proof boundary from ruling 3. Inspect the source and run the
  focused test with exactly one selected passing test.
- **AC3 — preserved observations.** Preserve real PUT success, the exhaustion
  baseline before SMTP, successful SMTP delivery, HTTP success and count
  exactly one before observing exhaustion, the existing delta-one wait, and
  HTTP success/count exactly one afterward. Preserve URL scheme, budgets,
  deadlines and existing ignore policy. The entire webhook and binary suites
  pass locally; the SIGTERM function and all of tests/binary.rs stay
  byte-identical to the scope baseline. No strict latency or each-retry
  failure-stage guarantee is introduced.
- **AC4 — bounded inventory.** Inspect #191's changed tests using current
  functions and available local history, including db0b517 and 35afa14:
  tests/webhook.rs, tests/binary.rs, tests/common/mod.rs and src/api.rs tests.
  Record no remaining unconnected reservation used as a refused target in
  that changed surface after repair. The SIGTERM contender is a bind probe,
  not a target; the older fixed 127.0.0.1:1 target is outside this correction.
  A text search alone is insufficient. Historical commit 98b54e51 is absent
  locally; do not claim its unavailable diff was reconstructed.
- **AC5 — scope and preservation.** Relative to ac160ff, implementation edits
  only the named webhook function's fixture, lifetime and directly related
  comments/diagnostics. Preserve Dockerfile, tests/common/mod.rs, src/api.rs,
  tests/binary.rs and all other code byte-for-byte. The owning three design
  artifacts may reconcile this scope; intake/verification records may be added
  as authorized. No dependency, lockfile, allowlist, accepted-decision, gate,
  bundle or recipe-routing change. Inspect the baseline diff and commit all
  scoped work unsigned before final verification.
- **AC6 — completed local proof.** Every local house gate exits zero on the
  committed repair, using the digest-verified candidate-local engine and
  pinned proof scripts. Exact coverage has zero missed lines, branches and
  functions AND a completed successful exit and passing selftest. Preserve
  command, platform, commit/tree, exit and output receipts; verify shims,
  design selftest, delivery proof and fail-closed native-tls absence as in
  plan.md. Design/intake receipts cannot certify the later code change.

## External proof ownership

CI owns execution of the repaired test on macOS. Historical SIGTERM success
supports the fixture choice but does not certify the future webhook repair.
CI also owns the charter's chart/history, Docker smoke, network RustSec,
privileged port-80 and release-only proofs. The operator owns live settings,
sealing and publication. No macOS, live GitHub, namespace or external-network
proof is added to this seat's acceptance criteria. No push, merge, tag,
publication, issue closure or decision acceptance is authorized.

## Historical clarifications

Every entry in this section describes a superseded phase and baseline.
References there to "active" or "current" apply only to that dated phase;
the requirements above and current plan/tasks exclusively govern this run.

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

## Dated clarification — 2026-10-25, webhook macOS repair

The current baseline is ac160ff and design entry b7196b8. PR #238 evidence
shows the corrected SIGTERM fixture passing and the separate webhook fixture
failing. Replace the prior binary-only scope and webhook preservation clause
with AC1–AC6 above. Preserve the binary test byte-for-byte; add no contender
assertion to webhook. Earlier checked tasks and receipts stay historical.
Implementation and fresh candidate-bound proof remain outstanding.

## Design handoff

Commit exactly this spec, [plan](plan.md) and [tasks](tasks.md), leaving that
commit last. Positions, result JSON and gate logs remain ignored. The pinned
sequence check owns advancement. The chief reports only `drafted` and runs
baseline gates without claiming repair completion.
