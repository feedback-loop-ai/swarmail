# Issue #191: portable SIGTERM delivery proof

Status: Drafted design; implementation and delivery acceptance remain separate.
Date: 2026-10-25.
Feature slug: `residual-test-proofs` (retain the existing owning design).
Scope: Epic #192 issue #191, macOS repair of finding 23.4 only.
Pre-intake code baseline: `90eb2a0333a751576c94d82474e4410040d68d43`.
Design-entry HEAD: `fb4c6ead0d3ddb28d0cd2b58f43c4c1aa4171b6e`.

## Feature and rationale

Repair the failed-connect fixture in
`tests/binary.rs::sigterm_with_a_delivery_in_flight_prints_the_final_counts`.
The held, bound but unconnected socket introduced for #191 times out on macOS,
where Linux refuses the dial. Releasing the port would restore the parallel-test
reuse race. A held established connection's exclusively bound client endpoint
provides the proposed non-listening target while retaining ownership. A real
connect probe must still demonstrate prompt refusal before configuring the
shipped dispatcher against that exact address.

The test must continue proving one exhausted delivery and one in-flight delivery
aborted by SIGTERM, with a successful exit, exact final totals and credential-safe
transport diagnostics. It remains a real binary/SMTP/HTTP test. The feature
changes only its socket fixture, not webhook behavior or retry policy.

## Evidence and authority

The issue README, operator-rulings.md and macos-ci-failure.md in
`.forge/epic192/issue191/` were verified against that directory's SHA256SUMS.
The operator's 2026-10-25 ruling authorizes the portability repair and assigns
macOS proof to CI. Both current run-local panel positions were read under
`.brokkr/state/design/positions/`. The requested `.brokkr/state/tasks/` is absent;
the committed [macOS intake](intake-macos.md) and current task provide the
framing. Earlier intake and proof receipts are history, not repaired-code proof.
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

1. **Adopt both positions' established-connection fixture.** Explicitly bind a
   reuse-disabled IPv4 client on loopback port zero, connect it to a temporary
   loopback listener, accept its peer and retain both streams through the last
   assertion. The failed target is the client's local address, never its peer
   or listener address. Cost: two idle descriptors plus a short-lived listener
   and bounded setup replace one unconnected descriptor.
2. **Rule for robustness on ownership checks.** A competing IPv4 socket must
   fail to bind the target with `AddrInUse` for both `SO_REUSEADDR=false` and
   `true`, after final ledger assertions and before releasing the pair. This
   rejects simplicity's narrower reuse-disabled-only check: ordinary listeners
   may enable reuse. Cost: two short bind probes and a small inline loop. No
   listen is attempted if a bind unexpectedly succeeds. Lexical retention,
   source review and explicit final drop establish uninterrupted lifetime;
   endpoint checks alone cannot observe every intermediate instant.
3. **Rule for simplicity on fixture structure and lifetime.** Keep setup and
   assertions inline at this single call site; drop the temporary listener
   after accept. Reject a reusable abstraction, shared allocator, keeper task
   or production seam. Robustness permits retaining the listener, but it owns
   a different address and adds no protection to the failed target. Cost of
   declining that option: reviewers must distinguish the setup listener from
   the two retained streams; the comments and local-address selection do so.
   Do not repeat ownership probes at multiple phases without a new reason.
4. **Adopt both positions' exact connect-refusal oracle.** Retain the bounded
   real `TcpStream::connect` probe and exact `ConnectionRefused` assertion.
   Reject merely accepting `TimedOut`, enlarging deadlines or changing retry
   budgets: four five-second attempts plus backoff can exhaust the stalled
   delivery before SIGTERM and change the ledger experiment. Reject bind/drop,
   accept/close, TLS rejection, backlog saturation, fixed or remote ports,
   firewall/namespace changes, OS bypasses and probabilistic retries. Their
   smaller code or apparent flexibility sacrifices ownership, failure stage
   or independence from host configuration.
5. **Reconcile scope and proof limits.** Preserve all reviewed helper, comment,
   snapshot and distinct webhook changes. Reuse this spec directory and replace
   the obsolete AC4 and 23.5-only edit prohibition. Preserve dated provenance
   below. Panel Linux socket experiments support feasibility but are not Rust
   acceptance receipts. The actual macOS behavior remains a CI obligation;
   failure there requires another fixture repair, never weakened assertions.

## Acceptance Criteria

These are local implementation criteria. This design commit defines them; it
cannot mark the future repair proved. AC numbers retain the owning design's
mapping to the already reviewed #191 findings.

- **AC1 — preserve 23.1.** Dockerfile remains byte-identical to the pre-intake
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
  URL. Before releasing the pair, fresh sockets with reuse disabled and enabled
  each fail to bind that address with exactly `AddrInUse`.

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
  remains byte-identical to the pre-intake baseline. Its held target and stored
  mail/exhaustion observations are existing proof, not a substitute for AC4.
- **AC6 — preserve 23.5.** `src/api.rs` remains byte-identical to the pre-intake
  baseline, retaining the measured runtime acquisition observer and snapshot
  discriminator. The workspace suite executes it. No new mutation experiment
  or snapshot refactor is authorized by this portability repair.
- **AC7 — scope.** Implementation changes are confined to the named function's
  socket setup, ownership checks, explanatory comments and final release in
  `tests/binary.rs`. Other code in that file and all shipping code, Cargo files,
  dependencies, allowlist, decisions, gates and bundle routing stay unchanged
  from the pre-intake baseline. Only the three owning design artifacts are
  reconciled. Inspect the exact diff; no new support module or production seam.
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

## Design handoff

Commit exactly this spec, [plan](plan.md) and [tasks](tasks.md), leaving the
design commit last. Positions, result JSON and gate logs remain ignored.
The sequence's pinned deterministic check owns advancement. The chief reports
only `drafted`; design-baseline gates do not establish repair completion.
