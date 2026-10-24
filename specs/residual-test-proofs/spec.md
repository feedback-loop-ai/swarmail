# Issue #191 residual test proofs

Status: Drafted design; implementation and delivery acceptance remain separate.
Date: 2026-10-25.
Feature slug: `residual-test-proofs` (reuse the existing owning design).
Scope: Epic #192 issue #191 finding 23.5; preserve reviewed findings 23.1–23.4.
Code baseline: `68b4ef8602d754541512dea3793aedaa18fe0261`.
Design-entry HEAD: `9db0ecc` (adds the resolved intake record only).
Historical attribution comparison: `c3d124d` (AC1 only).

## Feature and rationale

Replace the copied-source snapshot assertion with a deterministic behavioural
proof of the existing single-acquisition scrape. Accepted 0047 §2 requires
summing and rendering a cumulative registry under one held lock. The current
`cumulative_rows` already does that. Its inline test checks correct idle output,
then compares the function's source with a copied literal. A compiling split-lock
scrape can produce the same idle output; the literal only proves source drift.
The careful review therefore identified a framing defect, not a runtime bug.

Repair ruling 3, AC6, AC7 and the dependent plan/tasks together. Implementation
must remove the literal and observe actual registry acquisitions during the real
scrape, then execute a green/red/green split-lock experiment. Reuse this design
directory; no second competing spec. Findings 23.1–23.4 and the distinct webhook
proof are already reviewed clean on the code baseline. Their acceptance criteria
below remain preservation obligations, not authorization to reopen their code.

## Evidence and authority

Read both run-local positions, `.brokkr/state/design/positions/simplicity.md`
and `robustness.md`, the packet README, and
`.forge/epic192/issue191/review-6cf3ced1.md`. The requested
`.brokkr/state/tasks/` and packet `operator-rulings.md` are absent. The current
operator request and committed [resolved intake](intake-23.5.md) supply consistent
framing and expressly authorize the test-only seam. Historical receipts are not
current execution. No GitHub, CI, journal or sibling worktree is consulted.

Preserve synchronous accept/store, subscribe-before-check, oldest-only pruning,
real isolated protocols and ingest extraction (accepted 0001–0005).
[0047 §2](../../docs/decisions/0047-storage-rpc.md) already requires the locked
snapshot, including LF overflow. Follow
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
for installed, measured inline test seams,
[0050](../../docs/decisions/0050-delivery-gates.md) for executed pinned proofs,
and [0056](../../docs/decisions/0056-signing-policy.md) for unsigned commits.
[0041](../../docs/decisions/0041-webhook-boundary.md),
[0048](../../docs/decisions/0048-rest-webhook-validation.md) and
[0043](../../docs/decisions/0043-mcp-origin-boundary.md) remain authorities for
the preserved webhook and Host/Origin fixtures. Preserve current routing under
[0057](../../docs/decisions/0057-codex-delivery-recipes.md) and
[0058](../../docs/decisions/0058-codex-review-routing.md), independent review and
hard security hold. Proposed 0072 supplies no exemption from accepted proof rules.
These links are authorities, not new decision declarations. Strengthening an
existing proof changes no frozen guarantee, public contract or ruling; no ADR
is needed and no decision is accepted by this work.

## Panel synthesis and rulings

1. **Adopt both positions' bounded repair.** Preserve 23.1–23.4 and the
   existing refusal fixtures. Reject the old binary-gap narrative: `68b4ef8`
   already repaired it. Only the snapshot test, necessary owner seam and these
   three design documents may change. Cost: unrelated weaknesses remain outside
   this repair and cannot be represented as newly proved.
2. **Rule for simplicity's scrape-local observer over robustness's registry-wide
   wrapper.** Shadow the borrowed registry at scrape entry in test builds with
   a private forwarding facade; consume a next-call thread-local observer there.
   Keep the existing Registry alias, constructors and non-test executable
   statements byte-identical. Robustness's wrapper would automatically cover
   more registry consumers, including future helper paths, but requires broader
   test infrastructure and constructor edits. We decline that scope. The cost
   of the narrower choice is an explicit review obligation: every acquisition
   made by this scrape, directly or through a future callee, must remain on the
   observed access path. No unobserved dereference escape or direct bypass is
   permitted in the scrape or the negative control. This is a local proof, not
   universal lock tracing.
3. **Adopt both positions' execution oracle and robustness's proof discipline.**
   The observer delegates to the actual mutex and returns its real guard with
   unchanged lifetime and poisoning semantics. Count each real acquisition;
   do not count function entry, a manually placed one-shot notification or a
   separately modelled scrape. Assert exactly one (zero also fails) and exact
   fixture output. Reject the previous complete-function literal, source/token
   parsing, probabilistic writers, sleeps and writer handshakes that can deadlock
   the correct lock. A compiling sum/unlock/relock/render mutant must fail the
   runtime count with two versus one, with test and observer unchanged. Cost:
   a little measured test-only forwarding code replaces the smaller but invalid
   source pin; both installed and absent observer paths need coverage.
4. **Reconcile robustness's isolation with simplicity's local ownership.**
   Seed a private registry before installing the observer. A next-call
   thread-local slot transfers ownership to one synchronous scrape; use a local
   counter and consume the slot at entry so later assertion failure cannot leak
   installation. No process-global counter, cross-thread writer, atomics,
   generic tracing framework, feature flag or new harness is justified.
   Cost: this mechanism is specific to the current synchronous scrape; future
   asynchronous dispatch would require revisiting the observation boundary.
5. **Adopt robustness's contract limits.** Preserve storage/crash/refusal,
   pruning, DATA, extraction, TLS and best-effort webhook semantics unchanged.
   This proof adds no durable outbox, cross-family atomicity, crash-recovery,
   exactly-once delivery, scheduler or latency guarantee. Counting locks rejects
   the required split snapshot; it does not certify arbitrary redundant map
   copies under one guard. Exact output and inspection of the shared renderer
   remain complementary evidence. Linux is local; macOS and other CI-owned
   checks remain CI-owned without fabricated passes.

## Acceptance Criteria

- **AC1 — 23.1, attribution.** Dockerfile cites AC52.2 and is byte-identical
  to `c3d124d`, including the builder digest. Inspection proves this comment
  correction; no image refresh or new spelling test is needed.
- **AC2 — 23.2, helper ownership.** Raw-text prose directly documents
  `http_send_text`; deadline prose documents `HELPER_DEADLINE`; extra-header
  and Origin prose documents `http_send_text_with_headers`. Exactly one
  `send_with_host` definition has one truthful required/verbatim Host
  description. Existing signatures, request bytes and deadlines are unchanged;
  real helper consumers pass.
- **AC3 — 23.3, dead wrapper.** Exact-identifier source/import/re-export search,
  including relevant hidden/non-Rust sources and excluding VCS/build/run
  evidence, finds no executable `wait_for_metric` declaration or caller.
  Documentary mentions are allowed. Preserve `metric_now`,
  `wait_for_metric_from`, `wait_for_metric_row` and their live callers.
- **AC4 — 23.4, binary ownership.** The named binary final-counts test holds a
  Tokio IPv4 socket bound to `127.0.0.1:0`, reuse disabled, without listening.
  A separate real connect bounded by `HELPER_DEADLINE` must return exactly
  `ConnectionRefused`; success, timeout or another error fails. Hold the
  reservation through configuration, SMTP, exhaustion, SIGTERM, EOF/reap and
  every final assertion; drop explicitly afterward, with RAII on unwind.
  Preserve the stalled sink's completed-request synchronization, exactly one
  final event, delivered/dropped/lagged zero, exhausted one, aborted one,
  transport-cause checks and absence of credentials/secret. A fresh child
  justifies exhaustion baseline zero. Execute the named test on Linux;
  macOS execution is CI-owned, not pending local work.
- **AC5 — preserve the distinct webhook proof.** Its existing non-listening
  reservation, bounded exact refusal probe, pre-SMTP exhaustion baseline,
  stored-count one / one new exhaustion / stored-count one observations,
  and final drop remain. Execute its named test locally. This is supporting
  evidence and cannot substitute for AC4; its macOS leg is CI-owned.
- **AC6 — 23.5, behavioural snapshot discriminator.** Keep
  `api::tests::cumulative_scrape_uses_one_registry_snapshot` inline and measured.
  Remove its `include_str!`, extraction anchors, copied function and source
  equality assertion. Seed the existing private registry through the real
  registration function and preserve exact rendered bytes: total six, LF
  overflow two, `a` two, `b` one, `c` one. Execute actual `cumulative_rows` with
  an installed acquisition observer after setup and assert exactly one observed
  real registry acquisition plus the exact output. Every access in that scrape
  uses the observer, including any callee access; no unobserved alias or Deref
  escape is exposed. Observer code delegates real locking and returns the real
  guard, without changing poison handling, lock lifetime or renderer behavior.
  Also execute the observer-absent path. A zero count fails; an entry notification,
  output-only assertion or separately implemented scrape does not satisfy this
  criterion. Support is private, entirely `#[cfg(test)]` in `src/api.rs`, and
  installed/executed by the test, with every measured arm covered under 0020.

  Use the identical exact test command in plan.md for green/red/green, with
  exactly one selected test on every invocation. Save the repaired file bytes
  and arrange unconditional restoration before mutation. Temporarily change
  only the shared scrape access pattern to sum under one guard, drop it, acquire
  again and render with the saved total. Keep test and observer unchanged and
  both acquisitions observed. Require successful compilation and failure at the
  runtime acquisition assertion (two versus one); the idle output may still
  match, but that alone is insufficient. Compiler errors, zero tests, source
  mismatches, timeouts or unrelated panics do not prove discrimination. Record
  the precise mutant diff, candidate identity, platform, command, exit, selected
  test count and diagnostic. Restore exact saved bytes even on failure, verify
  byte equality and rerun the same test green. Never stage or commit the mutant.
- **AC7 — scope and truthful handoff.** Reconcile this spec, plan and tasks to
  the current 23.5 repair, retaining dated provenance as history. Against
  `68b4ef8`, all shipping executable statements remain byte-identical; changes
  outside the inline test module are solely `#[cfg(test)]` owner seam support,
  absent from non-test compilation. Registry aliases/constructors and renderer
  stay unchanged. Dockerfile, `tests/common/mod.rs`, `tests/binary.rs`,
  `tests/webhook.rs`, Cargo files, dependencies, allowlist, decisions, gates,
  policy and routing remain unchanged. No coverage exclusions, harness move,
  feature flags, sleeps, probabilistic retries or shared uncontrolled state.
  Inspect the scoped diff and non-test expansion; do not claim executable
  binary hash identity merely from source equality. Historical passes and
  design-baseline gates cannot satisfy implementation acceptance.
- **AC8 — executed local verification.** Affected real-server consumers and
  every gate in [the sole house inventory](../../docs/house-rules.md#gates--part-of-every-change)
  pass on the restored final candidate, including locked builds, digest-matched
  `.forge/bin/brokkr`, exact zero-missed coverage with successful exit, coverage
  selftest, publish/seal proofs, licenses and fail-closed native-tls absence.
  The engine SHA-256 must equal
  `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`
  and its tooling manifest identity before invocation. Historical receipts and
  printed summaries followed by timeout do not pass. CI owns macOS and the
  house inventory's CI-only proofs; no local CI success is claimed. Commit
  scoped work unsigned. Never push, merge, tag, publish, close the issue or
  accept a decision.

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

## Design handoff

Commit exactly this spec, [plan](plan.md) and [tasks](tasks.md), keeping the
design commit last. Positions, gate logs and result JSON remain ignored. The
sequence's pinned deterministic check owns its verdict. The chief reports only
`drafted`; committed design and baseline gates do not establish repair completion.
