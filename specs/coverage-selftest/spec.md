# Coverage selftest: prove the intended refusal

Created: 2026-10-24
Status: Draft
Story: [Epic #192 issue #174](https://github.com/feedback-loop-ai/swarmail/issues/174)
Baseline: `ff9bf8aba26fabaeea89876d5257b58198a12601` (reviewed issue #173 candidate)

## Feature and rationale

Strengthen the canonical coverage gate's selftest so each negative arm proves
the intended refusal, and four missing fixture premises are exercised. Today
`expect_refuse` accepts an unrelated exit 127; a green expansion can pass without
containing the compiler-generated exclusion it claims to test; library-only
fixtures cannot establish binary harvesting or scanning. These gaps can falsely
certify a broken verifier even when its exit-status assertions all pass.

The maintainer running `bash scripts/coverage-gate.sh selftest` must receive
success only when the intended diagnostics, fixture premises and green controls
all hold. Run actual coverage separately: a green selftest is not an exact
coverage verdict for the application.

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

## Panel synthesis and rulings

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

## Design handoff

This commit contains only this spec, [plan.md](plan.md) and [tasks.md](tasks.md).
The panel positions and result receipt remain ignored run-local evidence.
All implementation tasks stay unchecked until executed and proved. Structural
draft validation does not claim issue #174 is implemented or SHIP-COMPLETE.
