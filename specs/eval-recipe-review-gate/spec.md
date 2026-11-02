# The eval recipe's protected review, and every recipe compiled (#148 rework)

Decision: 0119 — [the eval recipe on brokkr](../../docs/decisions/0119-eval-brokkr-recipe.md),
section **R4 (2026-11-02)**, status `proposed`. This design adds R4 and
changes no other ruling. Code that is compatible with R4 may land while
the decision is proposed (accepted 0045 §8).

- **Issue:** #148, PR #316. The operator approved #148 by name on 2026-11-02.
  The PR stays held under 0056's 2026-11-01 addendum because it touches
  `.brokkr/` and `.github/`.
- **Branch:** `repair/issue148-eval-recipe`. Current main is merged in
  (37b16e6, ae7cb83).
- **Recipe:** `claude-careful` (accepted 0098, 0099). Review residuals are
  reworked under accepted 0102 and its 2026-10-31 addendum.
- **Supersedes:** fact F4 and the `score` naming in the earlier design,
  [eval-brokkr-recipe](../eval-brokkr-recipe/spec.md). Everything else in
  that design stands. That includes the agreement table (R3), the journal
  join, the evidence rules, the via-brokkr refusal and AC12 (filing is the
  operator's job).
- **Intake framing:** `specs/eval-brokkr-recipe/intake-framing.md` (run-local,
  uncommitted).

## Feature and rationale

CI's required check `brokkr bundle (pinned)` compiles every recipe on the
shelf. It refused the eval recipe with the pinned engine (brokkr 0.12.0,
sha256 `29dc3309…`):

```
REFUSAL: brokkr compile --bundle .brokkr/recipes/eval failed (exit 1)
error: bundle: policy has no 'review' phase; the protected review gate is
non-removable (extension model, layer 1)
```

That refusal exposed two defects.

1. **The recipe's shape.** In the engine's extension model, layer 1 makes
   the `review` phase mandatory for every bundle, including one that owns its
   policy and declares no `protected_phase`. The earlier design asserted the
   opposite (F4) without observing it.
2. **The gate gap.** The verify seat
   (`.brokkr/recipes/self/scripts/verify-seat.sh:125`) and the house rules'
   gate inventory (`docs/house-rules.md:62`) compile `self` only. CI
   (`.github/workflows/ci.yml:399`) compiles a fixed list of seven names.
   Three test fixtures repeat fixed lists: the CI fixture's stand-in engine
   (`tests/delivery_constitution.rs:1445`), its expected trace (`:1652`) and
   the shelf test (`tests/delivery_recipes.rs:74`). A fixed list behaves like
   a cap that silently drops what it does not name. It dropped `eval` from
   the local gates, so "verify passed" never meant "the recipe compiles", and
   it will drop the next recipe too.

This change gives the eval recipe a shape whose review is real. It makes
every gate that compiles recipes compile all of them, and it keeps the
brokkr issue drafts true. No `src/` line changes. Frozen guarantees
0001–0003 and ruling 0005 are not touched. No dependency is added.

## Panel synthesis: rulings

Both positions read the same tree, and neither could run the engine. They
agree on the core: rename the scoring gate to `review`, keep `trial`, keep
`score.sh`, no second scorer, glob every surface, keep the sweep-warning
check inside the loop. Where they disagree, this design rules as follows.

| Question | Simplicity | Robustness | Ruling, and what the losing side costs |
|---|---|---|---|
| Declare `protected_phase: "review"` on eval's bundle? | No. The engine's default already requires `review`, and tests treat the key as self-owned. | Yes. | **Robustness.** `self` is the only bundle proven to compile, and it owns its policy and declares the key. Copying that proven shape removes one unknown from AC1. The "self-owned" pins in `tests/delivery_recipes.rs` cover *extending* recipes, which must not redeclare the key. Eval extends nothing, so the pins do not apply to it. Cost to simplicity: one key, and one test mutant to pin it. |
| Fallback if the engine refuses the renamed shape | Do not add a second review seat. Record the refusal and move eval out (AC11). | R4-B first: a second deterministic `review` gate with a real check (slot self-consistency), and only then R4-C (move out). | **Simplicity, with one addition.** Every check R4-B names is already owned. Row, exit and trial-id pairing, `score_missing`/`score_mismatch` and the slot root belong to the matrix's slot reader. The strict row read belongs to `score.sh`. A second seat would therefore be a courier carrying the protected gate's name, and the house refuses gates that cannot fail on their own. **Added:** if the refusal names only an additive field that changes no result word, seat or transition (for example `shippable_from`), add the field and record it. Cost to robustness: if the engine requires delivery words or a panel, the brokkr-mode matrix leaves the shelf instead of keeping a second seat. |
| Loop reach: `*/` or `*/bundle.json` | `*/`, so a directory without a bundle fails the compile instead of being skipped. | `*/bundle.json`, plus checks for an empty shelf, the name and symlinks. | **Reconciled.** Iterate `*/`, the reach simplicity argued for, and refuse a directory without a regular `bundle.json` *by name* before the compile. This is the explicit check robustness wanted, and it does not depend on how the engine treats a missing file. Refuse an empty shelf (the unexpanded pattern is not a directory). Reject the name regex and the runtime symlink refusal: the shelf is reviewed repository content, already held by name under 0056, and the shelf test (AC7) refuses symlinked recipes at the source. Cost to robustness: a symlinked recipe added *and* merged past review would compile its target. |
| Per-recipe `timeout 300` in CI | Not raised. | Yes. | **Rejected.** Compiling is a local parse of repository JSON, and every other step in the job has no per-command timeout. It is outside this change's scope. Cost: a hung compile holds the job until GitHub's default job timeout. |
| How the gap is proven | A text pin in `tests/delivery_recipes.rs`. A run of the verify seat is "impossible" because it would recurse into `cargo test`. | Executable CI fixture with a sentinel recipe; text pin for verify-seat. | **Robustness, extended.** The CI fixture (`BrokkrCiFixture`) already runs the extracted CI body against a stand-in engine. `VerifySeatFixture` (`tests/delivery_constitution.rs:840`) already runs the real `verify-seat.sh` against stand-in `cargo` and `brokkr`, so it does not recurse. Both gain a sentinel recipe and go red on today's tree. The house-rules line is text, so it gets a text pin. The shelf test reads the directory. Cost to simplicity: fixture edits in an existing file. No new test file is needed, so simplicity's objection to AC5's "new file" holds. |
| Evals changes | None in `src`. Optionally the stand-in's seat name. | None in the facts reader. Add `review` as an invalid facts seat in the `journal.rs:699` test. | **Both.** No `evals/src` production line changes. The facts reader accepts only `trial` (`journal.rs:189`), and `journal.rs:699` is a rejection test. The inline test gains `"review"` as a second rejected seat. The stand-in (`seat score score.sh`) and the synthetic export at `evals/tests/recipe.rs:640` use `review`, matching what the engine renders. Comments that name "the score seat" are reworded. The `score=` log label stays, so `evals/tests/report.rs` keeps its assertions. |
| Who proves AC1 | "Verify catches it" (after this change). | The implement seat runs the pinned artifact. | **Neither alone.** Neither the design box nor (today) the candidate holds `.brokkr/state/bin/brokkr`. This run's verify seat executes the pinned copy of `verify-seat.sh` taken when the run started, which compiles `self` only. AC1 therefore names its receipt and its owners (see AC1). No seat claims it without the output. |

## The design (normative)

### 1. The eval recipe (`.brokkr/recipes/eval/`)

`bundle.json`:

- `name: eval`, `policy: policy.json`, no `extends`, and
  **`protected_phase: "review"`**.
- `description`: as today, except that "one exec scoring gate" becomes "one
  exec scoring gate, which is the recipe's protected review". It still says
  `unconfined`.
- Seats: exactly `trial` and `review`.
  - `trial` is unchanged in every field: `class: work`, exec driver on
    `./scripts/trial.sh`, results `["ran","harness_error"]`,
    `max_attempts: 1`, 1800 s, `kind: workspace`, `binds: []`,
    `network: true`.
  - `review` is the former `score` seat, renamed, with every field
    unchanged: `class: gate`, driver
    `["{brokkr}","driver","exec","--","bash","./scripts/score.sh","{prompt_file}"]`,
    results `["pass","fail","unscorable"]`, `max_attempts: 1`, 600 s,
    `kind: workspace`, `binds: []`, `network: false`. It has no `agent`,
    `panel`, `aggregate` or `inputs`.

`policy.json` (`forge.phase-machine/v2`):

- phases `["trial","review","done","stop"]`, initial `trial`, terminal
  `["done","stop"]`. There is no `shippable_from` (nothing ships), unless
  §1.1(a) applies.
- Rules, in this order, with results, transitions and severities as today:
  `TRIAL-RAN` ran → `review`; `TRIAL-HARNESS-ERROR` harness_error → `stop`
  (hard); `REVIEW-PASS` pass → `done`; `REVIEW-FAIL` fail → `done`;
  `REVIEW-UNSCORABLE` unscorable → `stop` (hard). The reason texts and the
  policy description say "review (scoring) seat" where they said "score
  seat". The R3 agreement table is unchanged in meaning. There is no retry
  rule, no `when` and no self-loop.

`scripts/score.sh` keeps its name, its behaviour and `<slot>/score.json`.
Only its header comment changes, to say that it is the `review` gate. The
scripts do not read the phase or seat name: they parse `Feature:` and the
indented result path.

#### 1.1 If the pinned engine refuses this shape

Apply the first branch that fits, and record the engine's exact output in
0119 R4 by a dated line:

- (a) The refusal names an **additive** field that changes no result word,
  seat, seat count or transition (for example `shippable_from`). Add it,
  pin it in `eval_recipe_defect`, and compile again.
- (b) Anything else, such as delivery result words
  (`clean`/`residual`/`security-hold`), a review panel or a model seat.
  **No lawful shape exists under this engine.** Move the recipe to
  `evals/recipe/` (outside `.brokkr/recipes/`). Point `evals/src/main.rs`'s
  realm preflight and `evals/tests/common/mod.rs` at it. Record in 0119 R4
  that the brokkr-mode matrix is not delivered. Report the implement result
  as **not complete**, naming brokkr issue 6. The gates of §2 still pass on
  the six remaining recipes.

It is forbidden to remap `fail` or `unscorable` into delivery words, or to
add a second gate whose only purpose is satisfying the engine.

### 2. One gate inventory compiles every recipe

The same loop semantics apply in every surface:

1. Iterate `for dir in .brokkr/recipes/*/` (every directory; `README.md` is
   not matched). Set `recipe="${dir%/}"`.
2. If `$recipe` is not a directory, refuse: "no recipe directory under
   `.brokkr/recipes/`". Only the unexpanded pattern of an empty shelf
   reaches this.
3. If `$recipe/bundle.json` is not a regular file, refuse by name:
   "`$recipe` has no bundle.json". A stray directory is a refusal, never a
   skip.
4. Compile `$recipe` with the admitted pinned engine. A non-zero exit
   refuses, naming the recipe and preserving the status.
5. Inside the loop, for **this** recipe's output, refuse a line containing
   `lets the run go on without reading`, naming the recipe.
6. Keep no fixed list of names anywhere in the loop.

Surfaces:

- **`.github/workflows/ci.yml`** (the `brokkr bundle (pinned)` job). Replace
  `for recipe in self … eval; do` with the loop above. Download, digest and
  version admission are unchanged, and so is the per-recipe log under
  `$engine_dir`.
- **`.brokkr/recipes/self/scripts/verify-seat.sh`**. Replace the single
  `run "brokkr compile --bundle .brokkr/recipes/self" …` and the sweep check
  after it with the loop. Refusals use `fail_now` (steps 2 and 3) and `run`
  (step 4). The sweep check (step 5) stays inside the loop, because `run`
  overwrites `$output` on every call. The final pass note says "every recipe
  bundle" instead of "bundle". Nothing else in the script changes. The
  `evals cargo test` → `brokkr compile` → coverage order stays.
- **`docs/house-rules.md` § Gates**. Replace the `brokkr compile --bundle
  .brokkr/recipes/self` line with
  `(for r in .brokkr/recipes/*/; do brokkr compile --bundle "${r%/}" || exit 1; done)   # every recipe directory, as CI and the verify seat; the brokkr pinned in .brokkr/recipes/self/tooling.json`.
  Add one sentence to the prose after the block: the compile covers every
  directory under `.brokkr/recipes/`; a directory without `bundle.json` is a
  refusal; CI and the verify seat also refuse the policy-sweep warning per
  recipe. `CONTRIBUTING.md` already points at the section and keeps no copy.
- **`.brokkr/recipes/README.md`**. In the recipe guide's compile examples,
  use the same loop instead of the four `compile` lines. In the eval
  section, say `trial` → `review` and explain that the scoring gate is the
  protected review.

### 3. Brokkr issue drafts (`specs/eval-brokkr-recipe/implementation.md`)

Drafted, not filed. Filing them, or attaching each to the brokkr#766
sub-issue that covers it, and recording the numbers is a merge
precondition owned by the operator (earlier AC12). The dispatcher files
them before merge. No seat files anything.

- **F4 row**: "falsified by the pinned engine (0.12.0 refuses a policy
  without a `review` phase); see issue 6 and 0119 R4".
- **Issue 4**: keep the text. Append one line: "Superseded: brokkr 0.12.0
  refuses this shape (see issue 6); the recipe now uses phases `trial,
  review, done, stop` with the exec scoring gate as its protected review."
- **Issues 1, 3, 5**: where they say "the score seat", say "the review
  (scoring) gate seat", and name `trial` as the only seat the journal-facts
  reader accepts.
- **Issue 6, new, verbatim:**

> **A non-delivery trial recipe and the protected review gate (0.12.0).**
> With brokkr 0.12.0 (binary sha256 `29dc3309925a2556b9713b7a9b5fb8089b8e4610e86215fc89554e75634e7cad`,
> source `a9893b4` plus four pinned patches), `brokkr compile --bundle
> .brokkr/recipes/eval` refused a standalone `forge.phase-machine/v2` recipe
> with phases `trial, score, done, stop` and no `protected_phase`:
> `error: bundle: policy has no 'review' phase; the protected review gate is
> non-removable (extension model, layer 1)`. Swarmail now ships the same
> recipe with its exec scoring gate named `review` (`class: gate`, exec
> driver, results `pass`/`fail`/`unscorable`, one attempt, no agent, no
> panel) and `protected_phase: "review"`. Questions: (1) Is that phase a
> lawful protected review for a non-delivery recipe, and is the layer-1
> requirement (a `review` phase even without `protected_phase`) documented?
> (2) Does the engine require of a protected review any result vocabulary,
> rule facts, `shippable_from` or panel aggregate, and does it apply them at
> compile or at run time? (3) If only a review panel is lawful, may a
> trial-only recipe declare itself exempt from layer 1, or what is the
> minimal panel shape that needs no paid model call? Until this is
> answered, Swarmail ships the shape the engine accepts or keeps `eval` off
> the compiled shelf and names this issue in the PR.
> [Implementer: append the observed compile output of the renamed shape,
> or the refusal that triggered 0119 R4 Ruling 3(b), with its date.]

## Acceptance Criteria

- **AC1 The eval recipe compiles under the pinned engine.**
  `brokkr compile --bundle .brokkr/recipes/eval`, run with the artifact
  whose sha256 equals `.brokkr/recipes/self/tooling.json`'s
  `binary_sha256`, exits 0 and prints no `lets the run go on without
  reading` line. Every other recipe directory compiles the same way.
  **Proof:** the command, the digest check and the verbatim output,
  recorded in `specs/eval-recipe-review-gate/implementation.md` and in 0119
  R4 by dated line. The implement seat produces it if the digest-matched
  binary is staged at `.brokkr/state/bin/brokkr`. Otherwise the receipt
  comes from the verify seat of a continuation entered from the candidate's
  bundle, or from CI's `brokkr bundle (pinned)` job. An absent receipt is
  reported as "not run", never as met. A refusal applies §1.1.
- **AC2 The recipe's shape is pinned.** `eval_recipe_defect`
  (`tests/delivery_recipes.rs`) accepts exactly the §1 shape. That means
  `protected_phase == "review"`; seats `["review","trial"]`; `review` is a
  gate on `score.sh` with the R3 results, 600 s, no network and no binds;
  `trial` is unchanged; phases `["trial","review","done","stop"]`; and the
  five rules with `review` replacing `score`. It refuses each existing
  one-fault mutant (renamed to `review`) and three new ones, each caught by
  the pin that names it: `protected_phase` removed, the `review` seat as
  `class: work`, and `trial` renamed. **Proof:** the test is red on the
  current tree, which still has `score`, and green after the change.
- **AC3 Nothing else is weakened.** `git diff` from the pre-change HEAD
  shows no change to any `bundle.json` or `policy.json` outside
  `.brokkr/recipes/eval/`. In `.brokkr/recipes/self/scripts/verify-seat.sh`,
  only the compile block and the final pass note change. `shims.sha256`,
  `tooling.json`, the coverage floor and every other gate are unchanged.
  **Proof:** the diff, quoted in implementation.md.
- **AC4 CI compiles every recipe directory.** The CI loop follows §2. The
  extracted CI body, run by `BrokkrCiFixture` with a fixture shelf of the
  repository's recipe names plus `sentinel`, compiles exactly the shelf's
  directories (compared as a set) and nothing else. It refuses, each by
  name and before any later recipe compiles: a compile failure (status
  preserved); a sweep warning on stdout or stderr for a recipe that is not
  last; a directory without `bundle.json`; and an empty shelf. The stand-in
  engine accepts any `.brokkr/recipes/<name>` and still refuses a
  `--bundle` outside the shelf (exit 94). **Proof:**
  `tests/delivery_constitution.rs` CI tests. The sentinel assertion is red
  on today's fixed list.
- **AC5 The verify seat compiles every recipe directory.**
  `VerifySeatFixture` runs the real `verify-seat.sh` with a candidate shelf
  of `self`, `sentinel` and `README.md`, under `LC_ALL=C`. The stand-in
  engine records one argv line per call. The seat compiles both
  directories, in glob order, between `evals cargo test` and the coverage
  gate. The seat fails, with its notes naming the recipe and no later gate
  run, for each of these: a sweep warning on the *first* recipe while the
  second compiles clean (proving the check sits inside the loop); a
  compile failure; a directory without `bundle.json`; and an empty shelf.
  **Proof:** the fixture tests. The sentinel assertion is red on today's
  `self`-only compile.
- **AC6 The house rules name the loop.** The § Gates block carries the §2
  line, and the prose states the reach. The text pin in
  `tests/store_contract_docs.rs::the_gate_inventory_has_one_home_and_everyone_points_at_it`
  requires `for r in .brokkr/recipes/*/; do brokkr compile --bundle` and no
  longer requires the `self`-only line. **Proof:** that test, red before
  the doc change.
- **AC7 The shelf has no fixed list.**
  `every_recipe_stands_on_the_brokkr_folder_shelf` reads
  `.brokkr/recipes/`. Every entry must be either `README.md` or a real
  directory (not a symlink) holding a regular `bundle.json`. The seven
  known recipes must be present. **Proof:** the test, plus a scratch-tree
  arm or equivalent showing that a stray directory, a symlinked recipe and
  an extra file each refuse.
- **AC8 Evals unchanged in production.** `evals/src` changes only in the
  `journal.rs` inline test (which also rejects `seat: "review"`) and in
  comments. The stand-in's `seat review score.sh` and the synthetic export
  in `evals/tests/recipe.rs` use `review`. `grep -rn '"score"' evals/src`
  finds no production reader. **Proof:** the grep output, and
  `cd evals && SWARMAIL_BIN=<candidate> cargo test --locked` green.
- **AC9 0119 R4.** 0119 carries the dated R4 section (this design's
  commit). The implement seat adds only dated observation lines: the AC1
  receipt and, if needed, the §1.1 branch taken. The status stays
  `proposed`. The `docs/decisions/README.md` row is unchanged.
  **Proof:** `git diff` and the design check.
- **AC10 The brokkr drafts are current.** `implementation.md` carries §3:
  the F4 row, issue 4's appended line, the reworded issues 1/3/5 and issue
  6 verbatim, all marked drafted, not filed. **Proof:** the file, and a
  grep showing no draft still says "the score seat".
- **AC11 Every house gate is clean, all `--locked`.**
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo test --workspace --locked`
  - `cargo build --locked --bin swarmail`
  - the evals fmt, clippy and test against that binary
  - the §2 compile loop with the pinned artifact, or "not run" with the
    reason, per AC1
  - `bash scripts/coverage-gate.sh` (exact 100/100/100) and its `selftest`
  - `bash scripts/publish-guard-test.sh`
  - `bash scripts/seal-branch-test.sh`
  - `cargo deny check licenses`
  - `sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256`

  Builds and tests run with at most `nproc`/4 busy jobs
  (`CARGO_BUILD_JOBS`, test threads). The load applied is reported.
  **Proof:** the receipts table in implementation.md.
- **AC12 Delivery paper.** Commits are unsigned. Nothing is pushed, merged,
  published or accepted. No paid trial runs. `Cargo.toml`, `Cargo.lock`,
  `evals/Cargo.*` and `deny.toml` are unchanged. The PR stays held for the
  operator by name. It says that the brokkr drafts await filing, and that
  the verify seat's broader compile takes effect for runs whose pinned
  bundle includes it.

## Out of scope, and residuals named for the operator

- A live engine **run** of the recipe (oracle and null) remains the
  operator's first engine run (0119 §7). A compile proves the shape, not the
  run-time behaviour of an exec-only protected review.
- Whether the engine applies further review rules at run time (issue 6,
  question 2) is unknown until that run.
- A symlinked recipe merged past review would be compiled through its
  target. AC7 refuses it at the source.
- A hung compile in CI is bounded only by the job's default timeout.
