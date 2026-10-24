# Coverage selftest: prove refusals and concurrent reservations

Created: 2026-10-24
Status: Draft
Story: [Epic #192 issue #174](https://github.com/feedback-loop-ai/swarmail/issues/174)
Preservation baseline: `ff9bf8aba26fabaeea89876d5257b58198a12601` (reviewed issue #173 candidate)
Follow-up baseline: `16841aa4f503ad8eb83e36208ff5fd125fe25b95`
Updated: 2026-10-24 — recovery after two stopped implementation attempts
Recovery baseline: `310b9695fa03cec63bd06bb1b01e1697a5f5d984`

## Feature and rationale

Resolve the concurrent explicit-target reservation residual recorded after the
initial #174 implementation. The retained verifier receipt reports two resolver
successes (`0 0 ` instead of `0 1 `) and no named loser refusal. A selftest that
cannot establish exclusive reservation cannot certify isolated instrumented
builds. One contender must reserve the exact fresh leaf, the loser must receive
the existing named refusal, and both must preserve caller ownership.

The initial #174 work strengthened refusal diagnostics, malformed free-space
output, real derive premises and independent binary fixtures. AC1–AC9 remain
binding preservation requirements; the follow-up adds AC10–AC15. Existing code
is not evidence of successful verification. Selftest success requires every
intended diagnostic, premise and green control. Actual application coverage
remains a separate verdict.

This is test/selftest work inside `selftest()` in
`bundles/self/scripts/coverage-gate.sh`. Preserve every reviewed #173 change,
the scanner, production gate checks, diagnostics and exact verdict. No
application, dependency, shim, checksum, toolchain, policy or recipe change is
authorized. Issue #175, derive-provenance analysis, generic accounting and
other coverage residuals remain outside scope. If a new proof reveals a
production defect, preserve the evidence and return through the established
scope process; do not weaken the assertion or silently repair production.

## Authority and semantic assessment

Accepted [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0035](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md) and
[0050](../../docs/decisions/0050-delivery-gates.md) already establish exact
coverage, real compiler inventories and pinned proof ownership. This design
adds evidence for those contracts, not a new refusal policy; no proposed
decision is required. Their derive-provenance and uninstantiated-generic limits
remain explicit limits, not permission to exclude production code.

Synchronous acceptance, subscribe-before-check, oldest-only pruning and
extraction-at-ingest remain unchanged. Coverage does not replace the existing
real-protocol, concurrency or persistence proofs.
[0057](../../docs/decisions/0057-codex-delivery-recipes.md) governs the already
selected `engine` lane. Gate work retains the full design and independent
four-provider review, including official DeepSeek Flash, never Pro or fallback.
The chief does not choose the next phase or certify delivery.

## Initial panel synthesis — 2026-10-24

The run-local simplicity and robustness positions agree on scope and on
diagnostic-specific assertions. Adopt simplicity's single canonical owner,
existing assertion helpers, generated temporary fixtures and scoped overrides.
Adopt robustness's causal proof requirements: infrastructure failure cannot
stand in for scanner refusal, and the derive/binary premises must be observed.

Rule for a generated **binary-only red/green pair** using the existing fixture
runner, rather than simplicity's preferred addition of binaries to the shared
multi-offender library fixtures. Exercise stage 1 and stage 2 independently.
This costs another pair of tiny manifests and compiler invocations, but avoids
library artifacts masking executable dep-info selection or bin exclusions.
The losing shared-fixture approach would be cheaper, but needs extra
disambiguation and leaves more ways to certify the wrong compilation unit.

For derive ordering, use a small dedicated real-compiler red/green pair with
one author exclusion after a legitimate derive. Reuse the existing green
stage-2 dump for the minted-attribute presence proof. This reconciles isolation
with reuse: no extra compilation solely to inspect the existing green dump.
The cost of isolated ordering is bounded compilation time; an aggregate
fixture's unrelated exclusion must never satisfy this proof.

Reject status-only assertions, broad diagnostic matches on multi-offender
fixtures, handwritten compiler dumps, a general shell Rust parser, permanent
fixture crates, a new assertion framework and production injection flags.
Retain the isolated inner-only fixture and its green twin; remove the redundant
aggregate inner-attribute assertion instead of claiming that any exclusion in
that aggregate proves inner-attribute handling. Robustness's runtime examples
explain why proof quality matters; they do not authorize new runtime work.

## Follow-up panel synthesis and rulings — 2026-10-24

Read both run-local positions (`simplicity.md` and `robustness.md`) and intake
for `epic-192-issue-174-follow-up-res-8891f7c7`. Retained verifier receipt
`.forge/results/0f6f9e1f-944f-4bb1-8c0e-6edcba5a4638.json` records failure,
not its cause. The resolver uses one plain leaf `mkdir`, assigns ownership 0,
and cleanup removes only owned defaults. The fixture has nested subshells,
FIFO releases and child status files, but searches both logs together. Earlier
command overrides are explicitly unset. Neither a resolver defect nor an
EXIT-trap defect has been established by execution in this design seat.

**Adopt simplicity's boundary and investigation order.** Keep the canonical
selftest and atomic leaf reservation; reproduce through actual functions.
Update this spec instead of adding competing requirements. Reject speculative
locks, tokens, leases, backoff, stale-target reclamation, path normalization
and automatic explicit cleanup. A passing reduced race is a control, not an
explanation of the full-context failure. Unreplicated causality stays unresolved.

**Adopt robustness's attribution and finite failure handling.** Rule against
simplicity's suggestion to require a bound only after observing a hang: child
death before FIFO readiness is a credible failure of this proof. Readiness,
release and completion must be bounded and checked. Resolver status and worker
completion are separate obligations. The accepted cost is a small local
coordination protocol and failure controls; the losing minimal approach risks
an indefinitely blocked gate and ambiguous child exits.

**Reconcile readiness with causality.** Both children must acknowledge the
start boundary before either is released; each checks its release before
calling the resolver. This proves concurrent opportunity, not identical kernel
scheduling instants. Either winner is valid. Readiness alone cannot explain two
successful atomic reservations. Reject a generalized subprocess framework,
extra permanent test executable, exhaustive scheduler matrix and production
injection API. Keep sound existing coordination and only the local state needed
for the proof. The cost of rejecting broader machinery is coupling this small
protocol to two contenders and the existing stable refusal category.

**Keep AC8 until evidence warrants scope return.** The authorized repair is
inside selftest. If actual resolver execution proves a production defect,
preserve its reproduction and use the established scope-return process to
revise AC8, plan and tasks before production edits. Do not silently make an
exception. Restoring the existing contract does not itself need a new decision;
a semantic contract/ruling change requires a proposed decision, and conflict
with accepted authority waits for operator acceptance. No semantic change is
designed here. The accepted cost is a bounded design return if that hypothesis
is proved, rather than speculative production authority.

Robustness's runtime examples explain why trustworthy evidence matters; they
authorize no SMTP, store, webhook, TLS or extraction work. Keep intake's
`engine` classification under 0057 §§6–7, all gates and four independent
reviews. Patch size cannot justify a lane downgrade.

## Recovery synthesis and clarification — 2026-10-24

This synthesis reads both current panel positions and intake for
`epic-192-issue-174-recovery-afte-190349d3`. Retain the single feature slug
`coverage-selftest`, the dated initial/follow-up rulings above, AC1–AC15 and
unchecked T1–T8. The current panel agrees on the mandatory bound; the earlier
follow-up's losing suggestion to defer it is historical, not a position the
current simplicity seat still holds. No new semantic ruling is proposed.

**Evidence is a discrepancy, not attribution.** The stopped runs were
`epic-192-issue-174-strengthen-th-63080acf` and
`epic-192-issue-174-follow-up-res-8891f7c7`. The first implementation receipt
`.forge/results/d6005095-9b0d-4259-93bf-5b9e314e30d8.json` reports inability
to reproduce; the second, `9cb9ec21-b2ea-4fc6-a1a7-7244ca7007b9.json` in the
same directory, reports reproduction with causality unresolved. Their stops
are not completed tasks or permission to reset the engine's return bounds.
The retained `repro-canonical.output` and `repro-canonical.trace` under
`.forge/epic192/issue174-followup/` show the two-success failure. The adjacent
`direct-race.txt` records {1,0}, successful worker waits and the loser's own
refusal. Its shell options differ from the canonical context. Xtrace records
two leaf commands and success assignments, not two proven successful syscalls
against an unchanged namespace. This chief inspected those receipts; it did
not reproduce or attribute the fault anew.

**Adopt simplicity's single owner and simplification order.** First remove an
unnecessary path if T1 proves it responsible, then simplify lifetime under one
fixture owner, restore the proven behavior, and only then add a necessary local
guard. Preserve sound existing code. Reject speculative command bypasses,
trap changes, locks, retries and readiness rewrites as causal repairs. The
cost is that patch selection can remain stopped even when a plausible rewrite
passes; buying a green fixture would leave the reported failure unexplained.

**Adopt robustness's complete lifecycle proof within that small boundary.**
One local two-child controller must bound setup through joins, including
blocking opens/writes and any nested resolver or watchdog. Check malformed
records and tokens through the actual coordination/assertion path as well as
child death and wrong-log attribution. Rule against minimizing away these
controls: their maintenance cost is accepted because unchecked infrastructure
can impersonate a reservation result. Conversely, reject a general process
framework, permanent helper executable and exhaustive scheduler/platform
matrix. The accepted cost of that rejection is a specialized two-worker shell
protocol with a documented finite timeout that may fail on an overloaded host.
Timeout never earns a pass; no sleep establishes readiness or success.

**Reconcile proof strength with investigation scope.** Keep focused shell
observations first. If they cannot distinguish command selection/result,
status masking, effective path identity or intervening removal, T1 needs
process-attributed filesystem observations from the failing execution using
available tools. A passing instrumented rerun cannot supply missing evidence
for another run; instrumentation's scheduling effect remains a stated limit.
Neither identical CPU scheduling instants nor a syscall-tracing dependency is
required. Unavailable or inconclusive evidence leaves T1 unresolved.

**Retain caller ownership and the scope return.** Observe winner metadata;
never manufacture ownership 0. Neither worker deletes the leaf. The caller's
sentinel survives real cleanup and retained-path retry. AC8 remains binding:
a proved production resolver defect requires evidence-backed scope return and
reconciled spec/plan/tasks before production edits. Broader reservation or
runtime redesign loses; its extra recovery/portability surface has no causal
justification. Accept a bounded design return instead. #175 remains excluded,
the exact 100/100/100 floor stands, and the existing engine lane retains all
gates and its four protected reviewers. Runtime failure examples in the panel
are preservation arguments, not additional implementation tasks.

## Acceptance Criteria

- **AC1 — Every refusal has a cause.** Remove `expect_refuse` and every call.
  Every negative arm requires both nonzero status and a stable literal fragment
  identifying its intended refusal category, with the relevant function/file
  where needed. Audit existing naming helpers too: a path, escape-variable
  name, injected stderr or generic `coverage` alone cannot identify the cause.
  Preserve diagnostic-context assertions paired with a cause assertion over
  the same captured outcome or an equivalent deterministic invocation. Keep
  forbidden partial-output assertions. Isolated inner-only scanning remains
  red with its clean twin green.
- **AC2 — Assertion controls cannot pass vacuously.** Exercise the actual naming
  helper in child-local state: intended diagnostic plus nonzero passes;
  unrelated exit 127 fails; intended diagnostic plus zero fails. Assert the
  child failure counts/status and keep deliberate failures out of the parent
  result. Preserve the real parent's failure count.
- **AC3 — Malformed successful free-space output.** A subshell-local `df`
  returns zero with a normal two-line `df -Pk` shape and a nonnumeric available
  field (column four). The unchanged preflight refuses with
  `could not read the free space of` and the fixture path. The arm does not
  pass on stat failure, insufficient-space refusal or arithmetic error.
  Existing numeric success, short-space and named stat-failure controls pass;
  the override cannot escape into later commands.
- **AC4 — Derived scope ends before the offender.** A real dependency-free
  crate derives `PartialEq, Eq` before a uniquely named excluded function.
  Its canonical expansion demonstrably contains a compiler-generated
  `automatically_derived` impl before that function, outside the impl body.
  Stage 2 must refuse with `on fn <unique-name>`. The same derive and function
  without the author exclusion pass. No forged derive marker or other red
  offender may substitute.
- **AC5 — The green derive proof has its premise.** Before accepting the
  existing green stage-2 scan, assert its real expansion contains
  `coverage(off)` on `assert_fields_are_eq` within the generated Eq
  `automatically_derived` impl. Match associated local context, not unrelated
  tokens anywhere in the dump. This fixture contains no author exclusion.
  A missing/empty dump, compiler failure or missing minted attribute fails the
  selftest with useful fixture/log context; scanner success alone cannot pass.
- **AC6 — Binary evidence is independent.** Generate binary-only red/green
  crates and use actual executable builds, dep-info, coverage reports and
  explicit `--bin <name>` expansions. Assert the harvested set includes the
  bin root and its fn-bearing bin-only included file. Stage 1 separately
  refuses a red harness-named included file absent from the measured report,
  naming that file and the missing-production-coverage reason. Stage 2 refuses
  a uniquely named bin-only excluded function. The green binary executes its
  ordinary included function, has report records for the relevant files and
  passes both checks. No lib dump, fabricated dep-info or build failure is
  accepted as a bin refusal.
- **AC7 — Isolated, real fixture execution.** Generate added fixture locks
  offline and use `--offline --locked` for added builds, reports and
  expansions with the selected installed coverage toolchain/cfgs. Use owned
  unique temporary roots, fixture-specific targets and existing checked build
  helpers. Setup/compile failure fails the selftest; overrides, deliberate
  helper failures and cleanup cannot contaminate subsequent arms or runs.
- **AC8 — Preserve the candidate and verifier contract.** Diff against
  `ff9bf8a` shows implementation edits confined to selftest helpers/fixtures.
  Scanner and production gate logic remain byte-identical. Retain #173's fresh
  target reservations, failed-inventory/partial-output, clean-boundary and
  raw-identifier proofs. Actual coverage remains fail-closed, with positive
  counts and integer covered == count for lines, branches and functions
  (literal 100/100/100), no denominator shrink or exclusion authority.
- **AC9 — Validation and delivery evidence.** Execute the complete inventory in
  [house rules](../../docs/house-rules.md#gates--part-of-every-change), plus the
  pinned verifier's design selftest, tamper-boundary and shim-integrity proofs,
  and fail-closed native-tls absence. Run actual coverage and selftest
  separately. Record command/status and candidate identity; blocked or failed
  checks remain outstanding. CI-only obligations stay CI-owned. Preserve
  protected review and security hold; never push, merge, tag or publish.

- **AC10 — Causal evidence before repair.** Run the actual baseline resolver
  and fixture in its accumulated selftest context. Retain candidate, command,
  Bash version/options, relevant command resolution/overrides, exact requested
  paths, working directory, child identities, relevant traps/descriptors,
  per-child outcomes and leaf lifetime. Compare a controlled race through
  the same resolver outside that context. Reproduce and attribute the recorded
  two-success/no-refusal fault before choosing a patch. Static inspection,
  successful retries and a passing approximation cannot close the residual;
  unreplicated causality must remain explicitly unresolved. When shell evidence
  is inconclusive, obtain process-attributed filesystem evidence from the
  failing execution; a passing instrumented run cannot explain a different
  failure. Distinguish actual command results from recorded resolver outcomes.
- **AC11 — One attributable winner and loser.** Two live children share one
  exact initially absent leaf under a unique disk-backed fixture root. Both
  acknowledge readiness before release and consume checked release tokens.
  Parent observes and reaps both. Complete fresh records show exactly one
  resolver status 0 and one status 1. The winner reports its actual exact path
  and `BUILD_DIR_OWNED=0`; the status-1 child's own output contains
  `fresh, unused leaf` and that path. Merged logs, arbitrary winner identity,
  distinct leaves or serialized substitute calls cannot satisfy the proof.
- **AC12 — Infrastructure failure fails finitely.** Explicitly check setup,
  descriptor opens, readiness/release I/O, output writes and worker waits,
  including under conditional Bash execution. Missing/malformed records, wrong
  tokens, unrelated exits and worker death fail the proof. A finite deadline
  or watchdog covers setup through joins, every blocking coordination
  operation and nested resolver processes; expiry is failure,
  never successful synchronization. Stop and reap fixture children and finish
  any watchdog before root cleanup; teardown cannot hide an unbounded final wait or signal
  unrelated processes. Controls for death before readiness, failure after
  release with missing completion, missing/malformed records, wrong release tokens,
  unrelated nonzero status and refusal text only in the wrong child's log
  must fail through the actual local proof without leaked children or
  descriptor damage, including under conditional Bash execution.
  No sleep-as-synchronization or retries-until-green.
- **AC13 — Caller-owned lifetime.** Neither child deletes the reserved leaf.
  After both outcomes it remains present. A sentinel in the winning tree
  survives actual `cleanup_build_dir` with observed ownership 0 and retry
  refusal naming category/path. Leaf/sentinel remain unchanged. Only the outer
  fixture owner removes its root after joins and assertions. Retain existing
  empty/dirty-directory, file, symlink/dangling-symlink, trailing-slash, spaces,
  parent/leaf failure and owned-default cleanup controls.
- **AC14 — Causal red/green regression.** A focused control through actual
  canonical functions fails when the demonstrated fault is restored and passes
  after repair. Also run the canonical full selftest in normal context.
  Controls use the actual local assertion/coordination code, with child-local
  overrides/counters that cannot reset the real parent's failure count.
  Bounded repetition may supplement isolation evidence; it cannot replace
  this control or AC12's infrastructure controls.
- **AC15 — Preservation and delivery.** Compare the follow-up diff to
  `16841aa` and preserved scanner/non-selftest bytes to `ff9bf8a` under AC8.
  Retain AC1–AC9, including offline locked real compiler fixtures, failed
  inventory/partial-output, raw identifiers and clean boundaries. All AC9
  final-candidate gates must pass, including separate exact application
  coverage with positive counts and zero missed lines, branches and functions.
  Actual engine review receipts cover the implemented candidate: Qwen Max
  correctness, Claude Opus security, official DeepSeek Flash guarantees and
  DashScope GLM 5.3 tests, never Pro or fallback. Keep security hold and
  CI-owned obligations. Drafting is not repair or verification completion.

## Design handoff

This commit contains only this spec, [plan.md](plan.md) and [tasks.md](tasks.md).
The panel positions and result receipt remain ignored run-local evidence.
All implementation tasks stay unchecked until executed and proved. Structural
draft validation does not claim issue #174 is implemented or SHIP-COMPLETE.
