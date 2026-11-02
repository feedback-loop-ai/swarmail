# Plan: the eval recipe's protected review, and every recipe compiled

Spec: [spec.md](spec.md). Decision: [0116 R4](../../docs/decisions/0116-eval-brokkr-recipe.md)
(proposed).

## Technical context

- **Languages:** JSON (recipe), bash (CI step, verify seat), Rust tests
  (`tests/delivery_recipes.rs`, `tests/delivery_constitution.rs`,
  `tests/store_contract_docs.rs`, `evals/src/journal.rs` inline test,
  `evals/tests/recipe.rs`), Markdown.
- **Engine:** brokkr 0.12.0, pinned by `binary_sha256` in
  `.brokkr/recipes/self/tooling.json`. The design box has no engine, and the
  candidate has no `.brokkr/state/bin/brokkr` today.
- **Production code:** none. No `src/` change, so the coverage denominator
  is unchanged. The evals crate changes only in tests and comments.
- **Dependencies:** none added. No lockfile or `deny.toml` change.

## Constitution check

| Rule | Status |
|---|---|
| Frozen 0001–0003, ruling 0005 | Untouched: no `src/` change. |
| Protected review gate | Not weakened. `self`'s review and every extending recipe are unchanged. Eval gains a real protected review (its exec scoring gate). |
| Exec gates / pinned scripts (0050 §1) | `verify-seat.sh` changes only its compile block and pass note, which widens the gate. `shims.sha256` is unchanged, because verify-seat is not a shim. |
| 0045 §8 (code under a proposal) | Compatible: no accepted ruling is contradicted. 0116 §7's R2 clause is superseded inside the same proposed record. |
| 0056 hold | The PR touches `.brokkr/`, `.github/` and `docs/decisions/`, so it stays held for the operator by name. |
| 0020 test placement | Every proof sits in existing `tests/*.rs` fixtures and inline modules. No sleeps, no shared mutable state, fixtures under `tempfile`. |
| Load bound | Gate runs at most `nproc`/4 busy jobs (`CARGO_BUILD_JOBS=32` on this 128-core host) for at most 10 minutes per load proof. No load proof is needed. |

## Project structure (files touched)

```
.brokkr/recipes/eval/bundle.json          score -> review, protected_phase, description
.brokkr/recipes/eval/policy.json          phase review, REVIEW-* ids, reason texts
.brokkr/recipes/eval/scripts/score.sh     header comment only
.brokkr/recipes/self/scripts/verify-seat.sh  compile block -> loop; pass note
.brokkr/recipes/README.md                 compile loop example; eval section wording
.github/workflows/ci.yml                  brokkr bundle (pinned): fixed list -> loop
docs/house-rules.md                       § Gates line + one prose sentence
docs/decisions/0116-eval-brokkr-recipe.md R4 (design commit); dated receipt lines (implement)
tests/delivery_recipes.rs                 eval_recipe_defect + mutants; shelf test reads the dir
tests/delivery_constitution.rs            BrokkrCiFixture shelf + sentinel; VerifySeatFixture shelf + arms
tests/store_contract_docs.rs              gate-inventory pin -> the loop line
evals/src/journal.rs                      inline test: "review" also rejected (test only)
evals/src/main.rs                         doc comments naming the seat (comments only)
evals/tests/data/brokkr-standin.sh        seat review score.sh
evals/tests/recipe.rs                     synthetic export seat "review"
evals/README.md                           seat wording
specs/eval-brokkr-recipe/implementation.md  F4 row, issue 4 note, issues 1/3/5 wording, issue 6
specs/eval-brokkr-recipe/spec.md          one pointer line at F4: superseded by this spec
specs/eval-recipe-review-gate/implementation.md  receipts, AC table (new; implement)
```

## Approach

1. **Red first.** Change the tests before the product: `eval_recipe_defect`
   for the §1 shape, the CI and verify fixtures with the sentinel recipe,
   the shelf test, and the house-rules pin. Run them against the unchanged
   product, record the red output in implementation.md, then change the
   product.
2. **Recipe.** Edit the two JSON files by hand. Rename the key and keep
   every field value. The tests compare JSON values, so the key order of
   `seats` does not matter. Leave `score.sh` alone apart from its header
   comment.
3. **CI loop.** In the `run:` block, replace the `for recipe in …; do` line
   and its uses of `.brokkr/recipes/$recipe`. Bind
   `for dir in .brokkr/recipes/*/; do recipe="${dir%/}"; name="${recipe##*/}"`,
   then the two refusals (not a directory; no regular `bundle.json`), then
   the existing compile, cat, status and sweep body with `$recipe`, and the
   log at `$engine_dir/$name.log`. `set -euo pipefail` stays.
   `brokkr_ci_body` extracts the step by its markers. Keep those markers
   intact.
4. **CI fixture.** `BrokkrCiFixture::new` creates
   `.brokkr/recipes/<name>/bundle.json` for every directory name on the
   real shelf (read from disk, not listed) plus `sentinel`. It also creates
   `.brokkr/recipes/README.md`. The stand-in engine's `case` accepts
   `.brokkr/recipes/*` with no `/` after the name and exits 94 otherwise.
   `ci_admits_digest_before_executing_and_compiles_every_recipe` compares
   the marker's compile lines, sorted, with the fixture shelf's directories,
   sorted. The old `omitted` mutant becomes a body whose loop is mutated
   back to a fixed list without `sentinel`. It must exit 0 and still fail
   the set comparison: an exit code alone cannot catch an omission.
   `ci_refuses_warning_on_either_stream_and_preserves_compile_failure_status`
   drives `FAIL_RECIPE` over a recipe that is not last (`sentinel`, under
   `LC_ALL=C` set on the command) and over the last one. It asserts the
   marker ends with the failing recipe and contains nothing after it. New
   arms: remove every shelf directory (empty-shelf refusal), and create
   `.brokkr/recipes/stray/` with no bundle (refusal naming `stray`, engine
   not called for it).
5. **Verify seat.** In `verify-seat.sh`, replace lines 125–131 with the
   loop (spec §2). Use `fail_now` for the two structural refusals. Call
   `run "brokkr compile --bundle $recipe" "$launched_q compile --bundle $(printf '%q' "$recipe")"`,
   followed by the per-recipe sweep check. Change the pass note's `bundle`
   to `every recipe bundle`.
6. **Verify fixture.** `VerifySeatFixture::new` creates
   `candidate/.brokkr/recipes/{self,sentinel}/bundle.json` and a
   `README.md`. The stand-in engine appends (`>>`) one line per call to the
   argv marker (`$*`), appends `brokkr` to the gate order, and honours
   `FAIL_RECIPE` and `COMPILE_MODE` (`warning` printed to stdout, or
   `failure` with exit 23). The `base_command` sets `LC_ALL=C`. The
   existing order assertion becomes `… "cargo:test --locked", "brokkr",
   "brokkr", "coverage-gate.sh"`, and the argv assertion lists both
   compiles in C-locale order (`self`, then `sentinel`). New tests: a
   warning on `self` (the first) → `fail`, notes name `.brokkr/recipes/self`,
   no `sentinel` compile, no coverage gate; a compile failure on `sentinel`
   → `fail`; a `stray/` directory without a bundle → `fail` naming it; an
   empty shelf (no recipe directories) → `fail` naming the shelf.
7. **Shelf test.** Replace the seven-name loop in
   `every_recipe_stands_on_the_brokkr_folder_shelf` with a directory
   listing. Put the predicate in a helper, `shelf_defects(&Path) ->
   Vec<String>`, and call it on the real shelf (empty) and on a `tempfile`
   shelf for each of these: a stray directory without `bundle.json`, a
   symlinked recipe, and an extra non-README file. Each one names its
   defect. The seven known names must be present.
8. **House rules.** Replace line 62 with the spec §2 line and add the prose
   sentence. Update the `store_contract_docs.rs` pin to require
   `for r in .brokkr/recipes/*/; do brokkr compile --bundle`.
9. **Evals.** Make the one-line test addition in `journal.rs`, update the
   stand-in seat name and the `recipe.rs` export seat, and reword the
   comments. Then run `grep -rn '"score"' evals/src` and record the result.
10. **Docs and drafts.** Update the recipes README and evals README
    wording. Edit `implementation.md` per spec §3. Add a one-line pointer
    at the earlier spec's F4 row. Create the new implementation.md with the
    AC table and receipts.
11. **Engine receipt (AC1).** If `.brokkr/state/bin/brokkr` exists and its
    sha256 equals `binary_sha256`, run the §2 loop with it and record the
    output. A refusal triggers spec §1.1. If the binary is absent, record
    "not run: no staged pinned binary". The receipt then belongs to the
    verify continuation or to CI, and the PR body says so.
12. **Gates.** Run every gate in AC11 with `CARGO_BUILD_JOBS=32` and record
    the receipts.

## Research, data model, contracts, quickstart

- **Research:** the engine's layer-1 rule is known only from its refusal
  text, and brokkr's extension-model documentation was not reachable from
  the design seats. Issue 6 asks the open questions.
- **Data model:** unchanged. Rows, `score.json`, request, journal facts v1.
- **Contracts:** the recipe's seat and phase name (`review`) and its rule
  ids (`REVIEW-*`) are journal-visible. No reader depends on them: the facts
  reader keys on `trial`, and no evals code reads rule ids.
- **Quickstart:**
  `for r in .brokkr/recipes/*/; do brokkr compile --bundle "${r%/}" || { echo "REFUSED: $r"; break; }; done`
  is the operator's interactive form. The gate form in the house rules
  exits non-zero on the first refusal.

## Risks

| Risk | Mitigation |
|---|---|
| The pinned engine refuses the renamed shape for a second reason (for example a required `shippable_from`, delivery vocabulary or a panel). | Spec §1.1 fixes the order: (a) an additive field is added and pinned; (b) otherwise eval moves to `evals/recipe/`, the run reports not complete, and issue 6 is named. No ceremonial seat and no remapped result word. Thanks to §2, a second refusal now fails at verify, not in CI. |
| No seat in this run can execute the pinned engine, so AC1 stays unproven in-run. | AC1 names its receipt and its owners: a staged binary, a verify continuation entered from the candidate's bundle, or CI's pinned-bundle job. "Not run" is reported as such, never as met. The PR stays held by name. |
| This run's verify seat runs the pre-change pinned `verify-seat.sh`, so it compiles `self` only. | The fixture tests prove the new script's behaviour. The dispatcher can continue at verify from the candidate's bundle to execute it. Spec AC12 tells the PR to say this. |
| Glob order differs by locale and makes the fixture traces flaky. | Set comparisons for "every recipe compiled". `LC_ALL=C` on the fixture commands wherever order matters (fail-first arms). The product does not depend on order. |
| The fixture's stand-in engine accepts too much and hides a wrong `--bundle`. | The stand-in still exits 94 for anything not shaped `.brokkr/recipes/<name>`. The set comparison catches an extra or missing compile. |
| An empty shelf or a stray directory passes silently. | Explicit refusals (spec §2 steps 2 and 3), each proven by a fixture arm in CI and verify. The shelf test refuses at the source. |
| The sweep warning is checked only for the last recipe. | The check sits inside the loop. A verify arm warns on the first recipe while the second compiles clean, and must fail. |
| Renaming `score` breaks the evals facts reader or the report. | The reader validates `trial` only (`journal.rs:189`). The grep is recorded (AC8). The `score=` stand-in log label is kept, so `report.rs` is unchanged. The evals gates run. |
| The house-rules edit drifts from the scripts. | `store_contract_docs.rs` pins the line. CI and verify are pinned by executable fixtures. |
| Moving eval out (§1.1 b) silently breaks the evals preflight. | That branch includes moving `evals/src/main.rs:901` and `evals/tests/common/mod.rs:1165-1180` to the new path, and the evals gates run. |
| Host overload during gates (128 cores). | `CARGO_BUILD_JOBS=32`, test threads ≤ 32, report the load applied. |
