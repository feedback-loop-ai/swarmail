# Plan: Restore the fast implementer test contract

## Technical context

The repository is one Rust crate. `tests/delivery_recipes.rs` uses existing
`serde_json` helpers to inspect delivery bundles and offices. The fault is a
stale test oracle; the configured fast implementation route already matches
accepted 0057 and the adjacent regression. No application build behavior or
runtime protocol needs to change. There is no plan template in this checkout;
this plan supplies the technical context, constitution check, project structure,
research, data model, contracts and quickstart required by the spec-kit dialect.

## Constitution check

This repair implements existing authority in accepted 0057/0058 and makes no
semantic amendment, so it needs no proposed ADR or decision-index change.
Preserve frozen 0001–0003 and ingest extraction (0005), hermetic proof ownership
(0007/0050), exact coverage, dependency policy and the complete house gate
inventory. Leave fallback authority and the engine strategy unchanged. Commit
unsigned under 0056; do not push, merge or close an issue. The chief commit contains
exactly this directory's spec.md, plan.md and tasks.md and remains HEAD for the
sequence's final deterministic check. Run-local positions/results stay untracked.

## Research and evidence

The current intake and both positions identify the same final loop. Inspection
confirms all four cases in `bundles/codex-fast/bundle.json` select the fast office,
`agents/codex-fast-implementer.json` declares singleton Luna/xhigh, and
`adapters/codex.json` maps Luna to `gpt-6-luna`. The neighboring dedicated test
already checks this office, its implementer inheritance and adapter mapping.
The old loop instead requires Sol/high. These are source observations; Cargo
reproduction and candidate verification belong to implementation and verify.

The positions agree on the fix. Spec.md records explicit adoption, rejection and
cost rulings. No further provider research or live-provider probe is required
for this test-only repair. Structural proof cannot discharge inherited runtime
or engine evaluator obligations. Existing failures, including ones recorded in
prior history, must be re-observed against the candidate and reported without
expanding this patch or claiming historical evidence is a current gate pass.

## Project structure and touched files

| Path | Role |
| --- | --- |
| `specs/fast-implementer-contract/{spec,plan,tasks}.md` | Only files in the chief's design commit. |
| `tests/delivery_recipes.rs` | Only implementation edit: final loop in the named selector regression. |
| `bundles/codex-fast/bundle.json` | Read-only selector and reviewer contract input. |
| `agents/codex-fast-implementer.json`, `adapters/codex.json` | Read-only office and model-route inputs. |
| `docs/house-rules.md`, accepted 0057/0058 | Read-only authority. |
| `bundles/self/tooling.json`, pinned proof scripts | Read-only tooling identity and proof owners. |

## Implementation approach

1. Confirm the candidate and baseline diff; run the exact selector test before
   editing. Record its actual assertion failure. A build/setup failure is not
   reproduction of the Luna-versus-Sol mismatch.
2. In the existing four-strategy loop, assert
   `implement[class]["agent"] == "codex-fast-implementer"` before resolving that
   selected office. Retain the helper call. Change the exact model expectation to
   `json!(["luna"])` and the effort assertion to `overlay["efforts"]["luna"] ==
   "xhigh"`. Give all three assertions a `{class}` diagnostic. Retain surrounding
   assertions and the dedicated office test verbatim; add no helper abstraction.
3. Run the targeted post-edit test, dedicated Luna test and full recipe suite.
   Inspect the assertions for each AC1 rejection condition: wrong office fails
   before lookup, an extra model fails equality, and missing/wrong effort fails
   exact equality. No committed mutation harness or configuration edit is needed.
4. Review the diff for the bounded loop change and unchanged authority inputs.
   Execute all required gates through their established proof owners. Record
   actual failures separately; do not alter providers, recipes, pins, scripts,
   coverage or production code to make unrelated failures disappear.
5. Hand off the verified candidate to the existing fast engine review. Its five
   results, security hold, aggregation and no-fallback constraints remain owned
   by the engine. Do not manually select a smaller review or fabricate results.

## Data model and contracts

No schema, stored entity, wire API, migration or configuration format changes.
The existing contract is the mapping from each strategy to the named office,
from that office to exactly one model, and from Luna to xhigh. Independent
literal assertions preserve that mapping. The dedicated office test continues
to guard the inherited role and adapter identity. Review routing is an unchanged
accepted input, not a new contract introduced by this repair.

## Quickstart and validation

Before editing, run the first command and record the expected assertion failure.
After editing, run all three commands and require success:

```bash
cargo test --locked --test delivery_recipes fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers -- --exact
cargo test --locked --test delivery_recipes fast_implementation_uses_luna_at_xhigh_without_provider_fallback -- --exact
cargo test --locked --test delivery_recipes
```

Then execute the full [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
without maintaining a second divergent list here. Keep locked root/eval checks,
the unique executable root non-test Cargo artifact as `candidate_bin` (overwriting
any inherited `SWARMAIL_BIN`), pinned compilation, exact coverage and its selftest,
publish/seal proofs, license checks and native-TLS absence. Use the candidate-local
`.forge/bin/brokkr` only after matching the recorded digest; retain all inherited
verifier preflight checks. Verify installs nothing and works offline. CI-only
proofs remain CI obligations. Any missing tool or failing gate is outstanding,
not permission to weaken or replace the proof.

For the design commit, inspect exact committed paths and clean artifacts, then
preflight with `bash bundles/self/scripts/design-check.sh check .`. The sequence's
final check from the pinned bundle remains the authoritative design verdict.
The design phase leaves implementation tasks unchecked and claims no Cargo,
full-gate or review result.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Making the test green by weakening it or reverting PR #213 | Exact independent office/model/effort assertions in every strategy; unchanged inputs; bounded diff review. |
| Matching the right model through the wrong office | Add the named-office assertion; accept small overlap with the dedicated office regression. |
| Accidentally changing review/fallback authority or other Sol routes | Keep all surrounding tests and configuration unchanged; retain engine review and its five-result cost. |
| Treating structural success as runtime dispatch or whole-candidate proof | Separate targeted tests, hermetic gates, CI and protected review evidence; preserve every existing proof owner. |
| Unrelated gate failures tempt scope expansion | Record the exact current failure and return it to the workflow; this design does not authorize unrelated fixes or certify the candidate. |
| Future operator model changes stale the oracle again | Keep explicit literals and strategy diagnostics; accept deliberate test maintenance as regression protection. |
