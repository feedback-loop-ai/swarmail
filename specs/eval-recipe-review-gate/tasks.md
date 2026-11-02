# Tasks: the eval recipe's protected review, and every recipe compiled

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision: 0116 R4 (proposed).
Order matters: every test task lands, and is recorded red, before the
product change it proves.

## Phase 1: red first

- [ ] T1 Rewrite `eval_recipe_defect` in `tests/delivery_recipes.rs` for spec §1:
  `protected_phase == "review"`; seats `["review","trial"]`; `review` is a gate
  on `score.sh` (600 s, no network, `binds: []`, results pass/fail/unscorable);
  phases `["trial","review","done","stop"]`; the five rules with `review`. Rename
  the existing mutants' `score` key to `review`, and add three: `protected_phase`
  removed, `review` as `class: work`, and `trial` renamed. Each is caught by its
  named pin.
  **Test:** `cargo test --locked --test delivery_recipes eval_recipe` is red on
  the current recipe. Record the output in implementation.md.
- [ ] T2 Replace the seven-name loop in `every_recipe_stands_on_the_brokkr_folder_shelf`
  with a `shelf_defects(&Path)` helper over a directory listing. Every entry
  must be `README.md` or a real directory holding a regular `bundle.json`, and
  the seven known recipes must be present. Add `tempfile` arms: a stray
  directory, a symlinked recipe, an extra file.
  **Test:** the shelf test passes on the real shelf, and each arm names its
  defect (AC7).
- [ ] T3 `BrokkrCiFixture`: build the fixture shelf from the real shelf's
  directory names plus `sentinel` and a `README.md`. The stand-in engine
  accepts any `.brokkr/recipes/<name>` and exits 94 otherwise. Rewrite
  `ci_admits_digest_before_executing_and_compiles_every_recipe` to compare
  compile lines as a set, and make the omission mutant a fixed list without
  `sentinel` that exits 0 and still fails the set comparison.
  **Test:** red on today's `ci.yml` because `sentinel` is never compiled
  (AC4). Record it.
- [ ] T4 CI refusal arms in `tests/delivery_constitution.rs` (with `LC_ALL=C`):
  a warning on stdout or stderr and a compile failure on a non-last recipe
  and on the last one, with the status preserved and nothing compiled after
  it; an empty shelf; a `stray/` directory without `bundle.json`, refused by
  name, with no engine call for it.
  **Test:** these arms are red on the current CI body where it lacks the
  refusal (empty shelf, stray directory). Record it.
- [ ] T5 `VerifySeatFixture`: a candidate shelf of `self`, `sentinel` and
  `README.md`; the stand-in engine appends one argv line per call, appends
  `brokkr` to the gate order, and honours `FAIL_RECIPE`/`COMPILE_MODE`;
  `LC_ALL=C`. Update the existing order assertion (two `brokkr` lines) and the
  argv assertion (`self`, then `sentinel`). Add arms: a warning on the first
  recipe → `fail`, the notes name `self`, no `sentinel` compile, no coverage
  gate; a compile failure; a stray directory; an empty shelf.
  **Test:** red on today's `verify-seat.sh` because `sentinel` is never
  compiled and there are no structural refusals (AC5). Record it.
- [ ] T6 `tests/store_contract_docs.rs`: the gate-inventory pin requires
  `for r in .brokkr/recipes/*/; do brokkr compile --bundle` in place of the
  `self`-only line.
  **Test:** red on today's `docs/house-rules.md` (AC6). Record it.

## Phase 2: the product

- [ ] T7 `.brokkr/recipes/eval/bundle.json` and `policy.json` per spec §1:
  rename the seat and the phase to `review`, add `protected_phase: "review"`,
  use the `REVIEW-*` ids, reword the reasons and the description (still
  `unconfined`), and add no `shippable_from`. Change only the header comment
  of `score.sh`.
  **Test:** T1 is green. `git diff` shows no other recipe's bundle or policy
  changed (AC2, AC3).
- [ ] T8 `.github/workflows/ci.yml`: the spec §2 loop in the
  `brokkr bundle (pinned)` step, with admission untouched.
  **Test:** T3 and T4 are green (AC4).
- [ ] T9 `.brokkr/recipes/self/scripts/verify-seat.sh`: the spec §2 loop, with
  the sweep check inside it, and the pass note changed to "every recipe
  bundle". Nothing else.
  **Test:** T5 is green.
  `verify_runs_the_pinned_gates_and_checks_the_public_shims` is still green,
  and `sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256`
  reports OK (AC3, AC5).
- [ ] T10 `docs/house-rules.md` § Gates: the loop line and the reach
  sentence. `.brokkr/recipes/README.md`: the loop example and the eval
  section wording (`trial` → `review`, the scoring gate is the protected
  review).
  **Test:** T6 is green.
  `the_gate_inventory_has_one_home_and_everyone_points_at_it` and the
  `tests/delivery_constitution.rs` README pins are green (AC6).
- [ ] T11 Evals: `journal.rs:699`'s inline test also rejects `seat: "review"`;
  the stand-in uses `seat review score.sh` (the `score=` log label is kept);
  the `evals/tests/recipe.rs:640` export seat becomes `"review"`; comments in
  `evals/src/main.rs` and the wording in `evals/README.md` change. Record
  `grep -rn '"score"' evals/src`.
  **Test:** `cd evals && cargo fmt --all -- --check`,
  `cargo clippy --all-targets --locked -- -D warnings`, and
  `SWARMAIL_BIN=<candidate> cargo test --locked` are green (AC8).

## Phase 3: records

- [ ] T12 `specs/eval-brokkr-recipe/implementation.md`: the F4 row, issue 4's
  appended line, issues 1/3/5 reworded, and issue 6 verbatim from spec §3,
  all marked drafted, not filed. `specs/eval-brokkr-recipe/spec.md`: one
  pointer line at F4.
  **Test:** `grep -n 'score seat' specs/eval-brokkr-recipe/implementation.md`
  finds no draft that still says it, and issue 6 matches spec §3 byte for
  byte apart from the implementer's appended observation (AC10).
- [ ] T13 Engine receipt (AC1): if `.brokkr/state/bin/brokkr` exists and
  `sha256sum` equals `tooling.json`'s `binary_sha256`, run the spec §2 loop
  with it and record the verbatim output in
  `specs/eval-recipe-review-gate/implementation.md` and as a dated line in
  0116 R4. On a refusal, apply spec §1.1 and record the branch taken. If the
  binary is absent, record "not run: no staged pinned binary" and name the
  owners (verify continuation, CI).
  **Test:** the receipt itself. An absent receipt is never reported as met.
- [ ] T14 Every AC11 gate, `--locked`, with `CARGO_BUILD_JOBS=32` (≤
  nproc/4). Record a receipts table and the load applied in
  `specs/eval-recipe-review-gate/implementation.md`, and confirm that
  `Cargo.*`, `evals/Cargo.*`, `deny.toml`, `self/bundle.json`,
  `self/policy.json`, `tooling.json` and `shims.sha256` show no diff.
  **Test:** every gate exits 0 (coverage exact 100/100/100, selftest,
  publish-guard, seal-branch, deny). The diff check is empty (AC3, AC11,
  AC12).
- [ ] T15 Commit unsigned in the repository's style. Never push, merge or
  accept 0116. The implement result is `complete` only if every task above
  is done and AC1 is met or honestly recorded as not run with its owner
  named. Under spec §1.1(b), the result is not complete.
  **Test:** `git log --format='%G? %s' -n3` shows `N`, and
  `git status --short` is clean apart from run-local state (AC12).
