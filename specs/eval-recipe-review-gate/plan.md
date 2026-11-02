# Plan: the eval recipe's protected review, and every recipe compiled

Spec: [spec.md](spec.md). Decision: [0119 R4](../../docs/decisions/0119-eval-brokkr-recipe.md)
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
| 0045 §8 (code under a proposal) | Compatible: no accepted ruling is contradicted. 0119 §7's R2 clause is superseded inside the same proposed record. |
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
docs/decisions/0119-eval-brokkr-recipe.md R4 (design commit); dated receipt lines (implement)
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
docs/decisions/README.md                  0119 row notes column only (2026-11-02 revision)
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
11. **Engine receipt (AC1, revised 2026-11-02).** If `.brokkr/state/bin/brokkr`
    exists and its sha256 equals `binary_sha256`, run the §2 loop with it
    (every step, sweep check included) and record the receipt AC1
    describes. A refusal triggers spec §1.1. If the binary is absent,
    record "not run: no staged pinned binary". That **blocks `complete`**:
    the implement result names the dispatcher's action (stage the artifact
    and re-enter implement, or continue at verify from a launch pinned to
    the candidate's bundle, `verify-seat.sh` `dd893db5…`). CI is the
    merge-time backstop, not the owner.
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
- **Quickstart:** the house rules' line, verbatim:
  `(for r in .brokkr/recipes/*/; do brokkr compile --bundle "${r%/}" || exit 1; done)`.
  The `exit 1` runs inside the subshell, so it does not close an
  interactive shell, and the group's status is non-zero on the first
  refusal. Read each recipe's output for `lets the run go on without
  reading`. (*Revised 2026-11-02:* the earlier `break` form left the loop
  with status 0.)

## Revision after the review return (2026-11-02)

The fourth review returned `residual` with `spec_defect: true` (guarantees),
max severity medium, and no security residual. The shape, the loops and the
fixtures were found sound and are not reopened. Each finding the
implementer must act on gets its own task in [tasks.md](tasks.md) (Phase 4).

| Finding | Members, severity | Change, and the artifact that owns it | Task |
|---|---|---|---|
| F1: AC1 has no pinned-engine receipt, and AC1/AC11 let "not run" satisfy the gate inventory (the spec defect). The verify passes at 53ec6b4 carried the old pass note. | correctness, guarantees, independent; medium | spec AC1 rewritten as a completion precondition with two in-run owners, the receipt's evidence, tree-id binding and a reading rule. AC11 drops "or not run". AC12 has the PR quote the receipt. 0119 R4 Ruling 3 gets the dated receipt rule. Plan step 11 and the risk row follow. | T16, T22 |
| F2: `.brokkr/recipes/README.md` uses `|| break`, which exits 0 | correctness; low | spec §2 README bullet: the house rules' line verbatim, chained with `&&`; AC6 adds a pin that refuses `|| break` | T17 |
| F3: 0119 R4 Ruling 4 says the house rules' line refuses by name and checks the sweep warning, and it does neither | guarantees low; correctness info | 0119 Ruling 4 narrowed to the code (design commit). The spec §2 house-rules bullet and AC6 require a prose clause, pinned. The line is not extended (ruled in spec.md). | T18 |
| F4: "five" drafts, issue 6 not named in the held-PR contract, "0119 provisional (0115 held)", index row "0116 stayed free", header history | guarantees; low | spec §3 and AC10 (six, issue 6 named, number history). AC9 corrects the index notes column. The 0119 header is restored to the true history (design commit). | T19 |
| F5: issue 6 says "now ships" before any engine accepted the shape | independent; info | spec §3 issue 6 text: "PR #316 proposes"; the slot-filling rule; no filing with an empty slot | T20 |
| Tests lens: the pass note text is not asserted | tests; info | AC13, needed because AC1's reading rule now depends on that text | T21 |
| T39 historical row; symlinked recipe directory | correctness; info | No change. T39 is a record, not a draft. The symlink is refused by the shelf test, which runs inside `cargo test` in both CI and verify (ruled in spec.md) | none |

**One criterion loosened, with evidence.** AC9 said that the
`docs/decisions/README.md` row is unchanged. That row's notes say "0116
stayed free", and that is false: main's `docs/decisions/0116-*.md` is #313's
implicit-TLS decision, merged into this branch at 53ec6b4, and this branch
renamed its own record in `03d3dea`. A criterion that preserves a false
statement is wrong. AC9 therefore keeps the status column frozen
(`proposed — awaiting the operator`) and allows only the notes column to be
corrected. No other criterion is loosened. AC1 and AC11 are tightened.

**Why the 0119 header changes in the design commit.** In `03d3dea` the
mechanical renumber replaced "0116" with "0119" inside the Number
paragraph. That turned "the next free number across both trees is 0116"
into a claim about 0119, which was never true (compare
`git show 89f57c4:docs/decisions/0116-eval-brokkr-recipe.md`). The R4
revision restores the original reasoning and appends the renumbering. No
other dated entry changes, and the 2026-11-02 implement observation is
kept verbatim.

**What this run's own verify will and will not prove.** This run's pin
carries the pre-change `verify-seat.sh`. Its verify pass, however green,
says `bundle,` and is silent on `eval`, according to AC1's reading rule.
The receipt therefore has to come from route 1 (a staged artifact at
implement) or from a verify continuation launched from the candidate's
bundle. The design cannot arrange either. Both are dispatcher actions, and
the implement result must name them if neither is in place.

## Risks

| Risk | Mitigation |
|---|---|
| The pinned engine refuses the renamed shape for a second reason (for example a required `shippable_from`, delivery vocabulary or a panel). | Spec §1.1 fixes the order: (a) an additive field is added and pinned; (b) otherwise eval moves to `evals/recipe/`, the run reports not complete, and issue 6 is named. No ceremonial seat and no remapped result word. Thanks to §2, a second refusal now fails at verify, not in CI. |
| No seat in this run can execute the pinned engine, so AC1 stays unproven in-run. | *Revised 2026-11-02:* "not run" blocks `complete`. AC1 names two in-run owners (a staged binary at implement, or a verify continuation from a candidate-pinned launch), and CI only corroborates. The PR stays held by name and is not offered without the receipt. |
| This run's verify seat runs the pre-change pinned `verify-seat.sh`, so it compiles `self` only. | The fixture tests prove the new script's behaviour. The dispatcher can continue at verify from the candidate's bundle to execute it. Spec AC12 tells the PR to say this. |
| Glob order differs by locale and makes the fixture traces flaky. | Set comparisons for "every recipe compiled". `LC_ALL=C` on the fixture commands wherever order matters (fail-first arms). The product does not depend on order. |
| The fixture's stand-in engine accepts too much and hides a wrong `--bundle`. | The stand-in still exits 94 for anything not shaped `.brokkr/recipes/<name>`. The set comparison catches an extra or missing compile. |
| An empty shelf or a stray directory passes silently. | Explicit refusals (spec §2 steps 2 and 3), each proven by a fixture arm in CI and verify. The shelf test refuses at the source. |
| The sweep warning is checked only for the last recipe. | The check sits inside the loop. A verify arm warns on the first recipe while the second compiles clean, and must fail. |
| Renaming `score` breaks the evals facts reader or the report. | The reader validates `trial` only (`journal.rs:189`). The grep is recorded (AC8). The `score=` stand-in log label is kept, so `report.rs` is unchanged. The evals gates run. |
| The house-rules edit drifts from the scripts. | `store_contract_docs.rs` pins the line. CI and verify are pinned by executable fixtures. |
| Moving eval out (§1.1 b) silently breaks the evals preflight. | That branch includes moving `evals/src/main.rs:901` and `evals/tests/common/mod.rs:1165-1180` to the new path, and the evals gates run. |
| Host overload during gates (128 cores). | `CARGO_BUILD_JOBS=32`, test threads ≤ 32, report the load applied. |
| *(2026-11-02)* The run stalls at implement because no artifact is staged. | That is the honest state: the task forbids reporting `complete` while required work remains. The implement result names both dispatcher actions. Absence never triggers Ruling 3(b), so a sound shape is not thrown away on a hypothesis. |
| *(2026-11-02)* The route-1 receipt is produced by a model seat with a shell, and the seat could misreport it. | The receipt has to be verbatim and checkable: digest beside the recorded value, the exact command, per-recipe output and exit, and tree ids that a review seat can check by git ids without running anything. CI's required pinned job re-proves the same bytes before merge. |
| *(2026-11-02)* The receipt goes stale after a later recipe edit. | Tree-id binding: any change to a recipe directory's tree id voids the receipt, and a fresh compile is owed. Doc-only commits outside `.brokkr/recipes/` do not void it. |
| *(2026-11-02)* A host binary of unknown digest (for example `~/.local/bin/brokkr`) is used and reported as a receipt. | Only `.brokkr/state/bin/brokkr`, with its `sha256sum` shown equal to `binary_sha256`, counts. Any other binary's output is not a receipt and is not recorded as one. |
| *(2026-11-02)* An edit to the verify pass note silently breaks the reading rule. | AC13 pins the note text and the trailing digest, and a scratch mutant shows the pin fires. |
| *(2026-11-02)* The narrowed Ruling 4 is read as weakening a gate. | No executed check changes: CI's and the verify seat's refusals stay byte for byte. Only an overstated sentence about the hand line is corrected, and the hand line's limit is now written down beside it. |
| *(2026-11-02)* The exec-review reading becomes a precedent for removing delivery reviewers. | Spec §1.2 and 0119 Ruling 1 scope the reading to non-delivery trial recipes. |
