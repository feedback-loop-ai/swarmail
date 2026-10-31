# 0050 — Delivery verdicts bind to executed proofs

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance is the operator's.
- **Date:** 2026-10-19
- **Refines:** proposed [0035](0035-coverage-gate-refuses-by-effect.md) and
  [0045](0045-delivery-repairs.md) §§1–3.
- **Implements:** accepted [0007](0007-hermetic-gates-single-model.md) and
  [0009](0009-agent-council-hybrid.md); preserves
  [0020](0020-concurrency-tests-and-the-coverage-gate.md) and
  [0027](0027-living-listeners-bounded-sessions.md), including dated clarifications.
- **Design:** [spec](../../specs/delivery-gates/spec.md),
  [plan](../../specs/delivery-gates/plan.md),
  [tasks](../../specs/delivery-gates/tasks.md).
- **Issues:** #123, #124, #125, #140, #143 under epic #120.

## Context

The tree after #119 still executes mutable proof scripts, admits unvalidated
drafts, and admits residuals with unknown safety facts. Compiler-based
coverage checks omit configuration-dependent code and local production
packages; several apparent regression proofs cannot fail. This record makes
the added refusal policies reviewable instead of calling them incidental
implementation details. It does not accept any earlier proposal.

## Proposed ruling

1. **Proof ownership is transitive.** One canonical executable proof chain
   resides under the dispatched pinned resources: gates, selftests, scanner
   source/manifests/lock, validator and prerequisites. Fixed public shims are
   checked against pinned expectations but never select verify's algorithm.
   Candidate sources and release inputs remain candidate inputs. A changed
   shim refuses even if coverage is otherwise green. Preserve that refusal
   while collecting canonical coverage diagnostics for the tampered-shim,
   uncovered-line proof; do not introduce a candidate-controlled bypass.
   The launched engine must be bound to the pinned tooling identity; version
   equality alone does not establish the recorded source revision.
2. **Unaccounted configurations and local production packages refuse.**
   Root identity is canonical Cargo manifest identity, not metadata order.
   Compare measured, no-coverage and release-profile production expansions;
   a difference or failed/empty expansion refuses. Scan the test expansion
   too. Until harvesting supports more, refuse extra application workspace
   packages and unharvested local production dependency closure. The separate
   scanner workspace is not an application dependency. This intentionally
   trades future multi-package flexibility for an enforceable single-crate
   denominator, without weakening the exact coverage floor.
3. **The validator's identity is chief's HEAD commit.** Exactly one changed
   spec directory, committed matching artifacts, required structure and
   pinned prerequisites are necessary for `designed`. Explicit new semantic
   declarations require proposed decision records; authority references may
   cite accepted decisions. An intervening unrelated commit requires a new
   chief commit rather than selection by mtime or branch-wide history.
   Only the final deterministic exec result advances design. A phase-level
   `drafted` is an explicit hard stop; first check failure retries once and
   second failure stops, including an engine-required separate check phase.
4. **Unknown residual facts have a defined destination.** The residual allow
   rule requires explicit false security and valid severity at most medium.
   Specific stops come first where the engine admits them, then the allow
   rule, then one final unconditional hard stop that catches security
   residuals and absent, null or malformed facts alike. No coercion,
   defaults or missing-fact inference. This strengthens the existing
   safe-admission intent without changing clean-review routes, the
   `security-hold` stop or the protected review boundary.

   *Amended in place 2026-10-19 by operator ruling in chat; still proposed.*
   The recorded verify engine refuses to compile a separate security stop
   ahead of the allow followed by an unknown-only fallback: with every fact
   present the earlier rules already decide, so the fallback fires on no
   valuation and is refused as dead. The committed order is therefore
   `REVIEW-RESIDUAL-ABOVE-MEDIUM` (stop, hard) → `REVIEW-RESIDUAL-OK`
   (`has_security_residual: false` and
   `max_residual_severity_at_most: "medium"`, ship) →
   `REVIEW-RESIDUAL-UNSAFE` (no `when`, stop, hard). First match decides: a
   valid above-medium severity stops at ABOVE-MEDIUM even when security is
   absent, null or wrong-typed; every other unsafe or unknown row reaches
   UNSAFE, whose single ID covers both causes — the journal's typed inputs
   tell them apart. The unchanged structural pin is
   `tests/delivery_constitution.rs::residual_admission_requires_explicit_safe_facts_and_unknowns_stop`;
   it does not replace executing the evaluator truth table (§5).
5. **Execution is the proof.** Actual verify dispatch, actual design dispatch
   and the pinned policy evaluator are mandatory. Structural inspection and
   compiler success are useful checks but cannot replace those proofs. A
   missing evaluator CLI may be bridged by a harness linked to the recorded
   engine core, not by a locally reimplemented policy. Blocked execution
   remains outstanding and is never reported as passed.
6. **Test strengthening does not redefine runtime contracts.** The mandatory
   503 proof establishes handler entry during drain; it does not promise a
   response on every connection closed by graceful shutdown. Keep the 10 s
   server fence and no new process-wall validity assertions. Test independent
   existing Config idle overrides, not invented CLI flags. Replace racy
   selected ports with child-bound port 0 plus held occupied-port controls;
   do not claim an unsupported descriptor handoff. Allocation measurements
   observe actual RETR/sizing/post_json work, and Drop cleanup covers unwinding,
   not external kill. Decision-index tests compare statuses, not perpetual
   proposed strings; this never grants the test author acceptance authority.

## Alternatives and consequences

General recursive package harvesting is deferred; future local production
packages must extend the proof or be refused. Extra compiler passes cost
time and can expose legitimate configuration differences; the remedy is an
explicit reviewed rule, never stripping inconvenient semantics. These checks
do not certify optimized codegen equivalence or solve 0035's derive-provenance
and uninstantiated-generic limits.

Branch-range spec discovery is rejected because it can certify an earlier
design. Strict HEAD identity costs a recommit after unrelated changes.
Unmatched policy results are rejected in favor of explicit stops, costing
re-entry when facts are malformed. The validator remains structural; panel
and chief still own the design's substance.

Compiler/cache/configuration attestation, socket activation, new dependencies,
and broad documentation or chart-release work are outside this ruling.
0001–0005, the five-second busy bound, awaits cap, single-writer lock,
envelope sender, Origin guard and prior batch behavior remain intact.
No accepted decision text is amended by editing its historical body.

## Proof obligations

The spec's AC1–AC13 and paired task tests are normative. Especially decisive:
the actual verify refusal of uncovered code despite an `exit 0` worktree
gate; missing-risks design refusal despite an old valid spec; the actual
residual truth table and warning-free pinned compile; six real-build coverage
bypass refusals; and each #143 broken-tree mutation failing before restoration.
The operator alone accepts this record; a design draft or green test run is
not that acceptance.

## Dated known limit — 2026-10-24 (proposed 0072)

The existing release-expansion comparator's `check_expansions_equivalent`
allowance admits an equal-indentation `if true {` to `if false {` change
without establishing that the branch came from a debug assertion. An ordinary
`cfg!(debug_assertions)` branch can fit the same textual shape. The retained
observation is explicitly textual, not compiler-expansion or real-build proof:
[0072](0072-delivery-proof-residuals.md) and the
[recovery receipt index](../../specs/delivery-proof-residuals/receipts.md).
This records a known limit only. It grants no exception, repairs no false
acceptance, and does not weaken §2's requirement that unaccounted release
differences refuse. The comparator and its function bytes remain unchanged.

## Dated addendum — 2026-10-30 (0102, accepted 2026-10-30)

§4's residual rows are amended by [0102](0102-review-residuals-rework.md) on
operator direction: a residual above low, or any security residual, returns
to implement up to five times and then parks for the operator; only
non-security residuals at or below low ship as tracked debt. The security
hold and the final unconditional hard stop for unknown facts stand. The body
above is unchanged as the record of what was accepted.

## Dated addendum — 2026-10-31 (0102 addendum, operator direction)

On the operator's direction of 2026-10-31 ("send spec defects back to
design"), [0102](0102-review-residuals-rework.md)'s dated addendum of that
day amends §4's review table again, and this time its clean-review routes
too: a spec defect with every fact present and well-formed, and no fixes
applied, returns to design, sharing 0102's five returns, and parks at the
sixth review; beside a security residual it returns to design as well and
never ships. A spec defect with absent, null or malformed facts, or beside
applied fixes, still reaches the final unconditional hard stop, and the
security hold stands. The body above is unchanged as the record of what was
accepted.
