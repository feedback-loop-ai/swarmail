# Implementation plan — coverage selftest

Implements [spec.md](spec.md); proofs are paired with [tasks.md](tasks.md).

## Technical context

The canonical gate is Bash with `set -euo pipefail`, jq, Cargo, an installed
coverage nightly and cargo-llvm-cov. Its standalone Rust scanner uses
syn/proc-macro2 over real source inventories and canonical expansions. There is
one measured application crate. No new dependency, production Rust line or
application test placement is required.

No plan template exists under `.specify/templates/`; use the requested
technical context, constitution check and project structure sections here,
with research, data model, contracts and quickstart inline.

## Constitution check

Preserve accepted 0020/0035/0050 and all frozen guarantees. This is evidence
strengthening, with no new semantic decision. Keep executable proof ownership
inside the pinned bundle and public shim bytes/checksums unchanged. Preserve
the selected engine/full lane under 0057, its protected reviews, official
DeepSeek Flash route and hard security hold. Commit unsigned and never push.

The candidate-local `.forge/bin/brokkr` must match
`bundles/self/tooling.json`; chief observed SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Recheck the manifest and digest before compile/dispatch. Do not borrow a
sibling binary or reconstruct journal receipts. The sequence's final pinned
check owns the design verdict.

## Project structure and files touched

| Path | Intended change |
|---|---|
| `bundles/self/scripts/coverage-gate.sh` | Only selftest-local helpers, diagnostic assertions and generated fixtures |
| `specs/coverage-selftest/spec.md` | Requirements, panel rulings and scope |
| `specs/coverage-selftest/plan.md` | Approach, proof contracts and risks |
| `specs/coverage-selftest/tasks.md` | Ordered implementation and verification tasks |

Scanner source, production gate functions/top-level verdict, `src/`, manifests,
locks, policies, routes, CI, public shims and accepted decisions are read-only
inputs to this repair. Fixture files and generated lockfiles live under the
existing unique `target/coverage/selftest.XXXXXX` root and are not committed.
No additional design documents or permanent fixture workspace are needed.

## Research and implementation approach

1. Confirm baseline ancestry and inspect the diff from `ff9bf8a`. Intake
   reproduced a status-only false pass with exit 127; the current helper has
   no diagnostic condition. Inventory every negative assertion, including
   existing naming calls and the separate #173 context/partial-output checks.
   Record an arm-to-reason mapping in local evidence to prevent omissions.
2. Convert all status-only calls to `expect_refuse_naming` or its existing
   `_without` variant, then delete the weak helper. Capture output/status once
   per command. Use literal comparisons and stable reason text, adding unique
   function/file discriminators where multiple faults coexist. Retain
   deterministic paired context proofs; replace escape-variable-only causes.
   Remove the redundant aggregate inner-attribute assertion; keep the
   isolated `inner-only`/inner-allow compiler proof. Do not snapshot complete
   rustc diagnostics or introduce a shared assertion API.
3. Add child-shell assertion controls around the real helper. Start a local
   failure count, invoke each controlled command, assert the expected delta,
   and relay only the control verdict to the parent. Suppress/capture
   deliberately failing control messages without swallowing unexpected
   failures. No recursion into the full selftest and no resetting of the real
   parent count. Explicitly observe status; conditional callers disable
   assumptions about shell errexit.
4. Add a small selftest-local function with a subshell body overriding `df`
   for exactly one `check_free_space` invocation. Emit a successful normal
   header/row with nonnumeric column four and assert the read-space refusal
   plus path. Keep stat-failure, numeric-success and short-space controls.
   No global PATH changes or production injection hook.
5. Generate a small derive-order red/green library pair through the existing
   fixture infrastructure. Use a real Eq derive and one excluded function
   after it; expand with coverage cfgs. Check the controlled dump's impl and
   function ordering/context, then require its unique function diagnostic.
   Green removes only the author exclusion. Separately inspect the already
   generated `green-exp.rs` for the Eq impl's minted attribute and
   `assert_fields_are_eq` association before the existing pass assertion.
   A small exact-shape check of this fixture is sufficient; do not introduce
   generalized Rust parsing or tolerate absent compiler output.
6. Generate two dependency-free binary-only crates with explicit bin names,
   `src/main.rs`, a bin-only include and an integration test that executes
   the actual binary. Red includes a harness-named fn-bearing file omitted
   by the report filter and a unique excluded function. Green uses an ordinary
   measured include with no author exclusion and executes it. Generate
   locks offline; collect real reports/builds/expansions with explicit bin
   selection and offline locked commands. Use existing `fx_cmd` fail-fast
   logging, unique targets and scanner `files --capture`; assert inventory
   membership before separate stage-1 and stage-2 checks. Ensure report
   filtering actually causes the intended red omission, and assert the
   reason plus filename. Binary-only crates prevent any library offender or
   rlib depfile from certifying this arm. Do not run the whole repo gate
   recursively for each fixture.
7. Run the strengthened selftest, inspect all four new proofs and controls,
   then the independent real exact coverage gate and full charter inventory.
   Check the diff's non-selftest prefix/suffix and scanner against baseline,
   including retained #173 arms. Record failures honestly; a discovered
   production defect returns through scope handling. No scanner repair,
   denominator change, lower floor or conditional skip is permitted here.

## Data model and contracts

No application schema, API or persistence changes. Selftest evidence consists
of command status + combined output, an expected literal refusal category,
optional function/path context and forbidden partial-output text. A negative
proof passes only when status is nonzero and its required text holds; compiler
setup success is a prerequisite, never an expected negative.

Generated artifacts are per-fixture manifests/locks, source files, reports,
captured dep-info, production file lists, canonical dumps and logs. Their
ownership and cleanup use the existing temporary-root lifecycle. Artifact
shape checks establish test premises; they do not become a production Rust
recognizer or a public diagnostic protocol. The actual gate's exact integer
coverage and build-derived inventories remain unchanged.

## Quickstart and validation

From the candidate root, verify `.forge/bin/brokkr` against the manifest.
Run `bash -n bundles/self/scripts/coverage-gate.sh`, then
`bash scripts/coverage-gate.sh selftest`. Inspect the named new arms and
control results, then run `bash scripts/coverage-gate.sh` separately.
Leave `CARGO_LLVM_COV_TARGET_DIR` unset for automatic fresh allocation; if
explicitly selected, supply a never-created leaf, never a cached/precreated
directory. Do not weaken the free-space preflight.

Execute the sole full gate inventory in
[house rules](../../docs/house-rules.md#gates--part-of-every-change), using
the candidate-local digest-matched Brokkr and locked Cargo commands. Also
execute `bundles/self/scripts/design-check.sh selftest`,
`bundles/self/scripts/delivery-proof-test.sh` and strict verification of
`bundles/self/scripts/shims.sha256`. Native-tls absence requires a failed
`cargo tree --locked -i native-tls` and its specific
`did not match any packages` diagnostic; any other failure is a failed proof.
Capture command, status, candidate/pin identity and evidence path in ignored
run evidence. CI-only checks remain those listed in the charter.

After the chief commit, run
`bash bundles/self/scripts/design-check.sh check .` as a structural preflight;
the final sequence check executes its pinned copy. HEAD must remain this
design commit until that check. Implementation and hermetic verify must rerun
their required proofs against the implemented candidate; draft/baseline gate
results do not certify future edits.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| An unrelated failure satisfies a negative test | Require reason + nonzero status; exercise unrelated exit 127 and text-plus-success controls; checked real builds precede assertions. |
| A different offender masks the intended branch | Isolate derive ordering and bin crates; use unique diagnostic names and independent stage checks; retain isolated inner-only proof. |
| Green derive stops containing the carve-out premise | Check associated Eq impl/minted method/attribute context before accepting scan success; changed compiler shape fails loudly and costs explicit fixture maintenance. |
| Diagnostic formatting changes cause brittle tests | Match stable literal categories and discriminators, not whole output, temporary roots or exit numbers. Wording changes require deliberate review. |
| Shell overrides or helper meta-failures leak | Use subshell-local overrides/counters and explicit observed control status; never reset the parent failures or depend on errexit. |
| Extra binary/derive builds cost time and disk | Reuse existing fixture runner and owned cleanup; dependency-free crates, installed toolchain and offline locks; accept bounded extra compiler work, no resource bypass. |
| Source changes escape the test-only scope | Byte-compare scanner and non-selftest gate regions to ff9bf8a, audit #173 controls, and return new production defects through scope handling. |
| A draft gets mistaken for completed delivery | Leave implementation tasks unchecked; distinguish structural validation, baseline gates, implemented selftest, actual coverage, hermetic verify/review and CI evidence. |
