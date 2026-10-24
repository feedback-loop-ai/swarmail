# Gate evals against the candidate build

- Status: draft for implementation; 2026-10-24.
- Scope: epic #192, issue #201; fixture provenance follow-up for #149.
- Baseline: `dc1822a45e4d2ae8326b44baf22818187a8320e9`.
- Intake: run `epic-192-issue-201-on-current-or-cbc7d3d2`, classified `engine`.

## Feature and rationale

CI and hermetic verify must exercise the standalone eval crate against the
Swarmail executable produced by that candidate's root test build. Today root
green excludes evals, and stale documentation describes the already accepted
harness as proposed. The runner also retains an unbounded server-stdout queue
after readiness. Complete the deferred gate wiring, prove its failures are
visible, reconcile the documentation, and remove that accumulating queue.

The users are contributors relying on CI and operators relying on pinned
verification. An eval-only format, lint, boundary or behavioral failure must
prevent a green result. A successful eval must identify the candidate it
exercised. A verbose child must neither exhaust a readiness queue nor block
on an unread pipe.

## Authority and scope

[0055](../../docs/decisions/0055-agent-eval-harness.md) is accepted by the
operator on 2026-10-19. Its conditional gate deferral ends with the already
landed batch-5 proof ownership in accepted
[0050](../../docs/decisions/0050-delivery-gates.md). This design implements
that follow-up and repairs internal buffering; it adds no public task,
result schema, protocol, frozen guarantee or amendment to an accepted ruling.
No new decision is declared or accepted. If implementation requires a
contradiction of an accepted ruling, return the design instead of silently
expanding this authorization.

Preserve synchronous accept/store, subscribe-before-check, oldest-only
pruning, ingest extraction, AUTH inbox isolation and the current MCP Origin
boundary. Evals remains an unpublished standalone workspace, outside root
coverage and packaging, with dependencies drawn from root-locked package
versions and sources. No root production code, dependency, license allowance,
new workflow route or model measurement is requested. Offline verify installs
nothing. Protected review, security hold, engine identity and shim integrity
remain mandatory under [0050](../../docs/decisions/0050-delivery-gates.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).

## Panel synthesis and explicit rulings

1. **Adopt simplicity's existing owners and robustness's executed proof.**
   Extend CI quality and Linux/macOS test jobs and the canonical pinned
   verifier. Keep explicit command wiring there; reject a new gate platform,
   matrix or dispatch shim. Cost: modest duplication and eval runtime on both
   platforms. Structural command checks alone lose: actual eval-only
   failures must propagate through pinned dispatch, costing isolated proof
   runs and maintained negative controls.
2. **Choose one root test invocation with Cargo artifact capture.** Select
   the root package's ordinary `swarmail` binary, never its unit-test harness,
   from that successful invocation. Robustness's separate `--no-run` pass is
   unnecessary here; losing it requires preserving mixed artifact/test output,
   Cargo status and diagnostics correctly. Reject guessed paths, inherited
   `SWARMAIL_BIN`, PATH lookup and per-case fallback builds in gates. Missing
   provenance refuses even if an apparently usable stale binary exists.
3. **Fix stdout now, rather than track the known queue as the delivery.**
   Adopt both panels' one-shot parsed readiness and log-only agent mode.
   Choose robustness's bounded whole-line framing over simplicity's wish to
   leave chunk framing alone: an oversized continuation must not masquerade
   as a new announcement. Cost: a small discard-until-newline state machine
   and boundary tests. Reject awaited bounded sends (pipe stalls) and lossy
   try-send queues (lost readiness). Old diagnostics beyond the log tail are
   intentionally lost. This does not promise confinement or a total process
   memory bound.
4. **Separate server shape assertions from client compatibility.** Adopt
   robustness's exact real-server response assertions and simplicity's
   refusal to invent a stricter general MCP client. Test existing client
   refusals through private decoding seams where needed; preserve tolerance
   for harmless extra fields/blocks. Cost: no support for structured-only
   results or a newer protocol, and no general MCP conformance claim.
5. **Preserve historical authority and correct current guidance.** Both
   panels agree that 0055 is already accepted. Update current spec/README
   claims and contradictory dependent instructions, preserving dated history.
   A dated factual addendum may explain the completed gate deferral; never
   rewrite the accepted body. For #149, document the canonical source and
   subcommand, and track unavailable external wording explicitly. Cost:
   the external issue correction remains an identified handoff, not a
   fabricated edit or a claim that the external issue was corrected.

The triage diagnosis is refined: agent launch drops the line receiver; the
persistent accumulation is in server `Proc`s retained after `announce`.
Startup can also outrun consumption. A chatty-agent test alone cannot prove
this defect fixed.

## Acceptance Criteria

- **AC1 — Mandatory execution.** CI quality runs eval fmt and locked,
  all-target clippy with warnings denied; both existing OS test jobs run the
  full locked eval suite after successful root tests. Pinned verify runs the
  same checks offline. No path-based skip or allowed failure; publication
  prerequisites continue to require these jobs. Job deadlines are finite.
  An eval-only defect fails the job/seat.
- **AC2 — Binary provenance.** Gates resolve exactly one distinct executable
  from successful root-test Cargo artifact messages, matching the canonical
  root package, binary target `swarmail`, non-test profile and non-null
  executable. The resulting path is absolute and executable. Missing,
  ambiguous, wrong-package or test-harness-only artifacts refuse. A failed
  root test/build cannot be masked by capture or parsing. A custom target
  directory containing spaces succeeds; a decoy inherited `SWARMAIL_BIN`
  is overwritten, and eval tests do not invoke fallback builds.
- **AC3 — Lock and workspace boundary.** The full eval suite executes the
  existing unpublished/standalone/package exclusion and no-native-tls/openssl
  tests. Independent foreign name, version and source mutations fail the
  same subset predicate; unchanged inputs pass. Root manifest, lock,
  packaging allowlist, license allowlist and coverage denominator are intact.
  Package-tuple subset is not feature-graph equivalence.
- **AC4 — MCP boundary.** Real Swarmail initialize and tool responses prove
  protocol `2025-03-26`, correlated ids, one `type: text` block containing
  JSON, `isError: false` on successful tools, and no `structuredContent`.
  Wrong id/version, JSON-RPC error, missing result/text, non-JSON text and
  missing/non-false `isError` refuse through the existing client decoding
  path. Structured-only data cannot bypass missing text. Harmless additional
  fields/blocks retain existing client tolerance; acceptance still traverses
  real SMTP/HTTP/MCP with no fake server replacing it.
- **AC5 — Harness truth.** All 16 oracle cases pass; all 16 null cases fail
  with their declared idle classes; two overlapping isolation trials pass.
  Existing scoring/report, failed-delivery and lifecycle controls pass.
  Infrastructure failures remain harness errors, never ordinary agent
  failures. Trials retain owned loopback port-zero servers and AUTH inboxes.
- **AC6 — Finite readiness retention.** No unbounded stdout-line channel
  remains in the runner. Each server can publish at most one parsed address
  pair; agents allocate no readiness forwarding queue. The drainer continues
  after delivery, timeout or receiver drop and never awaits a consumer.
  The existing 64-KiB shared per-process log tail, finite read buffer and an
  at-most-8192-byte startup line bound remain explicit. An oversized line is
  discarded through its newline; its suffix cannot become an announcement.
  This bounds retained output data, not child memory or allocator overhead.
- **AC7 — Readiness and cleanup proof.** Deterministic controls cover noise
  before readiness, one valid announcement amid noise, sustained server output
  after readiness while `Proc` stays alive, a chatty agent, exact-bound and
  oversized lines, a valid-looking oversized suffix, EOF/read error before
  readiness, duplicate announcements and receiver drop. Assertions prove
  bounded retained data and continued pipe progress. Existing startup/health/
  SMTP probes, deadlines, server-death reporting and terminate/kill/reap of
  children and descendants remain effective. Use markers/barriers and real
  deadlines, never sleep as synchronization or RSS timing as a bound proof.
- **AC8 — Executed gate refusals.** Isolated eval format, clippy and test
  defects each fail actual pinned verify dispatch at the relevant gate;
  restored controls pass it and a clean candidate passes the dispatch.
  Exercise binary selection refusals and space-containing target paths.
  CI-equivalent commands fail on the same defects. Keep shim-tamper and
  recorded-engine identity controls; candidate helper substitution cannot
  choose the verification algorithm. Preserve evidence with exit/result and
  decisive diagnostics, not only command-string assertions.
- **AC9 — Documentation and fixture provenance.** Both older eval specs and
  `evals/README.md` cite accepted 0055 (operator, 2026-10-19), current gate
  wiring and candidate-binary prerequisite. Reconcile dependent plan/tasks
  and the stale eval manifest comment without erasing dated history. Keep
  the deferred brokkr driver, unconfined execution, HTTP ICS download and
  HTTP support-reply limitations. Document #149's fixture as
  `evals/src/fixture.rs`, invoked with `swarmail-evals fixture`. Locate any
  erroneous repository path before replacing it. If external #149 text
  remains unavailable, record that precise outstanding correction, canonical
  replacement and operator follow-up; do not claim an external mutation.
- **AC10 — Inventory and full validation.** The single house-rule gate
  inventory adds eval fmt/clippy/tests and the test-build binary prerequisite.
  Retain every current gate, exact coverage/selftest, publish/seal proofs,
  license/native-TLS checks, pinned proofs and CI-only distinctions. Run the
  eval suite and complete inventory for implementation; record real failures
  or environmental blockers as outstanding. Design drafting and structural
  validation are not implementation success or a gate-pass receipt.
- **AC11 — Delivery isolation.** Changes stay in this checkout; concurrent
  worktrees are neither reset nor silently imported. Reconcile any later
  overlap deliberately and rerun affected proofs. Commit unsigned per
  [0056](../../docs/decisions/0056-signing-policy.md); never push, merge, close
  issues or accept decisions. This chief commit contains exactly this
  directory's spec, plan and tasks, and remains HEAD for design validation.

## Honest limits

Eval success proves these task paths, not TLS, persistence/restart durability,
pruning, webhook reliability, hostile-input CPU bounds or thousand-session
capacity. Root and CI-owned proofs retain those distinct responsibilities.
No queue-residual admission is assumed: if the selected fix cannot be proved,
return with its exact outstanding evidence and proposed follow-up for review;
tracking alone does not satisfy AC6–AC7 or authorize a clean completion claim.
