# Tasks: the eval recipe's protected review, and every recipe compiled

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision: 0119 R4 (accepted (operator, 2026-11-06)).
Order matters: every test task lands, and is recorded red, before the
product change it proves.

## Phase 1: red first

- [x] T1 Rewrite `eval_recipe_defect` in `tests/delivery_recipes.rs` for spec §1:
  `protected_phase == "review"`; seats `["review","trial"]`; `review` is a gate
  on `score.sh` (600 s, no network, `binds: []`, results pass/fail/unscorable);
  phases `["trial","review","done","stop"]`; the five rules with `review`. Rename
  the existing mutants' `score` key to `review`, and add three: `protected_phase`
  removed, `review` as `class: work`, and `trial` renamed. Each is caught by its
  named pin.
  **Test:** `cargo test --locked --test delivery_recipes eval_recipe` is red on
  the current recipe. Record the output in implementation.md.
- [x] T2 Replace the seven-name loop in `every_recipe_stands_on_the_brokkr_folder_shelf`
  with a `shelf_defects(&Path)` helper over a directory listing. Every entry
  must be `README.md` or a real directory holding a regular `bundle.json`, and
  the seven known recipes must be present. Add `tempfile` arms: a stray
  directory, a symlinked recipe, an extra file.
  **Test:** the shelf test passes on the real shelf, and each arm names its
  defect (AC7).
- [x] T3 `BrokkrCiFixture`: build the fixture shelf from the real shelf's
  directory names plus `sentinel` and a `README.md`. The stand-in engine
  accepts any `.brokkr/recipes/<name>` and exits 94 otherwise. Rewrite
  `ci_admits_digest_before_executing_and_compiles_every_recipe` to compare
  compile lines as a set, and make the omission mutant a fixed list without
  `sentinel` that exits 0 and still fails the set comparison.
  **Test:** red on today's `ci.yml` because `sentinel` is never compiled
  (AC4). Record it.
- [x] T4 CI refusal arms in `tests/delivery_constitution.rs` (with `LC_ALL=C`):
  a warning on stdout or stderr and a compile failure on a non-last recipe
  and on the last one, with the status preserved and nothing compiled after
  it; an empty shelf; a `stray/` directory without `bundle.json`, refused by
  name, with no engine call for it.
  **Test:** these arms are red on the current CI body where it lacks the
  refusal (empty shelf, stray directory). Record it.
- [x] T5 `VerifySeatFixture`: a candidate shelf of `self`, `sentinel` and
  `README.md`; the stand-in engine appends one argv line per call, appends
  `brokkr` to the gate order, and honours `FAIL_RECIPE`/`COMPILE_MODE`;
  `LC_ALL=C`. Update the existing order assertion (two `brokkr` lines) and the
  argv assertion (`self`, then `sentinel`). Add arms: a warning on the first
  recipe → `fail`, the notes name `self`, no `sentinel` compile, no coverage
  gate; a compile failure; a stray directory; an empty shelf.
  **Test:** red on today's `verify-seat.sh` because `sentinel` is never
  compiled and there are no structural refusals (AC5). Record it.
- [x] T6 `tests/store_contract_docs.rs`: the gate-inventory pin requires
  `for r in .brokkr/recipes/*/; do brokkr compile --bundle` in place of the
  `self`-only line.
  **Test:** red on today's `docs/house-rules.md` (AC6). Record it.

## Phase 2: the product

- [x] T7 `.brokkr/recipes/eval/bundle.json` and `policy.json` per spec §1:
  rename the seat and the phase to `review`, add `protected_phase: "review"`,
  use the `REVIEW-*` ids, reword the reasons and the description (still
  `unconfined`), and add no `shippable_from`. Change only the header comment
  of `score.sh`.
  **Test:** T1 is green. `git diff` shows no other recipe's bundle or policy
  changed (AC2, AC3).
- [x] T8 `.github/workflows/ci.yml`: the spec §2 loop in the
  `brokkr bundle (pinned)` step, with admission untouched.
  **Test:** T3 and T4 are green (AC4).
- [x] T9 `.brokkr/recipes/self/scripts/verify-seat.sh`: the spec §2 loop, with
  the sweep check inside it, and the pass note changed to "every recipe
  bundle". Nothing else.
  **Test:** T5 is green.
  `verify_runs_the_pinned_gates_and_checks_the_public_shims` is still green,
  and `sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256`
  reports OK (AC3, AC5).
- [x] T10 `docs/house-rules.md` § Gates: the loop line and the reach
  sentence. `.brokkr/recipes/README.md`: the loop example and the eval
  section wording (`trial` → `review`, the scoring gate is the protected
  review).
  **Test:** T6 is green.
  `the_gate_inventory_has_one_home_and_everyone_points_at_it` and the
  `tests/delivery_constitution.rs` README pins are green (AC6).
- [x] T11 Evals: `journal.rs:699`'s inline test also rejects `seat: "review"`;
  the stand-in uses `seat review score.sh` (the `score=` log label is kept);
  the `evals/tests/recipe.rs:640` export seat becomes `"review"`; comments in
  `evals/src/main.rs` and the wording in `evals/README.md` change. Record
  `grep -rn '"score"' evals/src`.
  **Test:** `cd evals && cargo fmt --all -- --check`,
  `cargo clippy --all-targets --locked -- -D warnings`, and
  `SWARMAIL_BIN=<candidate> cargo test --locked` are green (AC8).

## Phase 3: records

- [x] T12 `specs/eval-brokkr-recipe/implementation.md`: the F4 row, issue 4's
  appended line, issues 1/3/5 reworded, and issue 6 verbatim from spec §3,
  all marked drafted, not filed. `specs/eval-brokkr-recipe/spec.md`: one
  pointer line at F4.
  **Test:** `grep -n 'score seat' specs/eval-brokkr-recipe/implementation.md`
  finds no draft that still says it, and issue 6 matches spec §3 byte for
  byte apart from the implementer's appended observation (AC10).
- [x] T13 Engine receipt (AC1). *Superseded 2026-11-02 by T16*, which
  makes the receipt a completion precondition. The earlier "not run, owners
  verify continuation or CI" record stays as history.
  **Test:** see T16.
- [x] T14 Every AC11 gate, `--locked`, with `CARGO_BUILD_JOBS=32` (≤
  nproc/4). Record a receipts table and the load applied in
  `specs/eval-recipe-review-gate/implementation.md`, and confirm that
  `Cargo.*`, `evals/Cargo.*`, `deny.toml`, `self/bundle.json`,
  `self/policy.json`, `tooling.json` and `shims.sha256` show no diff.
  **Test:** every gate exits 0 (coverage exact 100/100/100, selftest,
  publish-guard, seal-branch, deny). The diff check is empty (AC3, AC11,
  AC12).
- [x] T15 Commit unsigned in the repository's style. Never push, merge or
  accept 0119. *Revised 2026-11-02:* the implement result is `complete`
  only if every task is done **and AC1 is met** (T16). "Not run" is
  recorded honestly and the result is then not complete, naming the
  dispatcher's action. Under spec §1.1(b), the result is not complete.
  **Test:** `git log --format='%G? %s' -n3` shows `N`, and
  `git status --short` is clean apart from run-local state (AC12).

## Phase 4: answers to the review return (2026-11-02)

Each task answers a finding named in [plan.md](plan.md) (Revision table).
Red first applies wherever a pin exists to go red. T16 runs after T17–T21,
so that it compiles the final recipe tree, and T22 runs last.

- [x] T17 (F2) `.brokkr/recipes/README.md`: replace the `|| break` compile
  example with the house rules' line verbatim, in its subshell, chained:
  `(for r in .brokkr/recipes/*/; do brokkr compile --bundle "${r%/}" || exit 1; done) && brokkr recipes show codex-fast && brokkr run --recipe codex-fast --feature '…'`.
  No `|| break` remains in the file. Add a pin, in the README pins of
  `tests/delivery_constitution.rs` or in `tests/store_contract_docs.rs`,
  that requires the verbatim line in the README and refuses `|| break`.
  **Test:** the pin is red on today's README (record it), and green after.
- [x] T18 (F3) `docs/house-rules.md` § Gates prose: keep the line unchanged.
  Rewrite the reach paragraph so that it attributes each check: the line
  compiles every directory and stops at the first non-zero exit; CI's loop
  and the verify seat refuse an empty shelf and a directory without a
  regular `bundle.json` by name, and they refuse the policy-sweep warning
  per recipe; a hand run reads each recipe's output for `lets the run go on
  without reading`. Extend
  `store_contract_docs::the_gate_inventory_has_one_home_and_everyone_points_at_it`
  to require that clause. Keep every phrase the pin already requires.
  **Test:** the extended pin is red before the prose edit and green after.
  The line pin is unchanged and green.
- [x] T19 (F4) Counts and number history.
  `specs/eval-brokkr-recipe/implementation.md`: in the PR contract and in
  "What is the operator's" item 1, "five" becomes "six", and issue 6 (the
  protected review gate) is named in the held-PR precondition. Replace
  "The decision number 0119 is provisional (0115 is held by #221's branch)"
  with the true history: main took 0115 (#308) and 0116 (#313), and this
  record was renumbered 0116 → 0119 on 2026-11-02. Replace the gate-receipt
  line "Not run here, run by CI: the pinned-bundle compile (including
  `eval`)" with a pointer to AC1's receipt (CI corroborates).
  `docs/decisions/README.md`, 0119 row, **notes column only**: replace
  "so 0116 stayed free" with "main later took 0116 for #313's implicit TLS;
  renumbered 0116 → 0119 on 2026-11-02". The status column is unchanged.
  **Test:** `grep -n '0116 stayed free' docs/decisions/README.md` finds
  nothing. `git diff` of the index changes the 0119 row's notes column
  only. In `specs/eval-brokkr-recipe/implementation.md`, greps for
  `provisional` and for a "five" draft count find nothing in the PR
  contract or the operator list (AC9, AC10).
- [x] T20 (F5) Issue 6 in `specs/eval-brokkr-recipe/implementation.md`:
  "Swarmail now ships the same recipe" becomes "Swarmail's PR #316
  proposes the same recipe", as in spec §3. Add to the operator list that
  issue 6 is not filed while its bracketed slot is empty.
  **Test:** `grep -n 'now ships' specs/eval-brokkr-recipe/implementation.md`
  finds nothing, and issue 6 matches spec §3 apart from the filled slot
  (AC10).
- [x] T21 (tests lens, AC13) In
  `verify_seat_refuses_missing_or_mismatched_engine_identity_before_gates`,
  at the existing pass arm (`assert_eq!(fixture.read_result()["result"], "pass")`
  after "A matched identity reaches an actual pass verdict"), assert that
  the notes contain `every recipe bundle` and end with
  `brokkr <fixture.engine_digest()>`. Add a scratch mutant of the fixture's
  copy of `verify-seat.sh` that replaces `every recipe bundle` with
  `bundle`, and assert that the mutant's pass note fails the same check.
  Restore the script afterwards, as the digest mutant already does. Do not
  edit `verify-seat.sh`; its pin stays `dd893db5…`.
  **Test:** the test is green on the shipped script, and the mutant arm
  shows the pin catching the old note.
- [ ] T16 (F1, AC1) Engine receipt, as a completion precondition. Check
  that `.brokkr/state/bin/brokkr` exists, then show
  `sha256sum .brokkr/state/bin/brokkr` beside `tooling.json`'s
  `brokkr.binary_sha256`. Only an exact match counts; a host binary of
  unknown digest is never used. If it matches, run the spec §2 loop
  (structural refusals, compile, per-recipe sweep grep) over every
  directory under `.brokkr/recipes/`. Record the date, the exact command,
  the digest line, each recipe's verbatim output and exit status, and
  `git rev-parse HEAD:.brokkr/recipes/<name>` for each recipe at the
  compiled commit. Commit T17–T21 first, so that
  `git status --short .brokkr/recipes` is empty when the loop runs. Record
  them in the AC1 row of `specs/eval-recipe-review-gate/implementation.md`,
  as a new dated observation line under 0119 R4 (the earlier line stays),
  and in issue 6's bracketed slot. On a refusal, apply spec §1.1 and record
  the branch taken. If the artifact is absent or its digest differs,
  record "not run" with the reason. Do **not** report `complete`, and name
  the dispatcher's action: stage the artifact and re-enter implement, or
  continue at verify from a launch pinned to the candidate's bundle
  (`verify-seat.sh` `dd893db5…`). Do not move `eval` off the shelf for an
  absence.
  **Test:** the receipt itself. Every compile exits 0 with no sweep line,
  and the recorded tree ids equal `git rev-parse HEAD:.brokkr/recipes/<name>`
  at the final head. Without a receipt, the result is not `complete`.
- [ ] T22 (F1, AC11, AC12) *(2026-11-02 visit: every gate but the pinned-engine compile exits 0 and is recorded; the compile row is T16's absent receipt, so this task and T16 stay open.)* Re-run every AC11 gate `--locked` at
  `CARGO_BUILD_JOBS=32` and `RUST_TEST_THREADS=32` (≤ nproc/4). Record the
  receipts table and the load applied. The compile row is T16's receipt,
  never "not run" as a clean gate. Update the PR description draft per
  AC12: six drafts, issue 6 included; the AC1 receipt quoted with its
  route; CI as corroboration. Confirm that `Cargo.*`, `evals/Cargo.*`,
  `deny.toml`, `tooling.json`, `shims.sha256`, every `bundle.json` and
  `policy.json`, and `verify-seat.sh` show no diff in this phase.
  **Test:** every gate exits 0 (coverage exact 100/100/100, selftest,
  publish-guard, seal-branch, deny, shims OK), the diff check is empty,
  and `git log --format='%G? %s' -n3` shows `N`.
