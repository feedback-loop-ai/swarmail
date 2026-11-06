# The eval recipe's protected review, and every recipe compiled (#148 rework)

Decision: 0119 — [the eval recipe on brokkr](../../docs/decisions/0119-eval-brokkr-recipe.md),
section **R4 (2026-11-02)**, status accepted (operator, 2026-11-06). This design adds R4 and
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
- **Intake framing:** `specs/eval-brokkr-recipe/intake-framing.md`
  (committed in 70f19de).
- **Revised:** 2026-11-02, after the fourth review return (decision 0102's
  2026-10-31 addendum, `spec_defect: true`). AC1 and AC11 accepted "not run"
  for the engine compile, which narrowed the task's "run every house gate …
  including compiling every recipe". The revision makes the pinned-engine
  receipt a completion precondition and corrects the ruling text, the README
  loop, the draft count, the number history and issue 6's wording. See
  [Revision after the review return](#revision-after-the-review-return-2026-11-02);
  [plan.md](plan.md) maps each finding to its change.

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
| Who proves AC1 | "Verify catches it" (after this change). | The implement seat runs the pinned artifact. | **Neither alone.** Neither the design box nor (today) the candidate holds `.brokkr/state/bin/brokkr`. This run's verify seat executes the pinned copy of `verify-seat.sh` taken when the run started, which compiles `self` only. AC1 therefore names its receipt and its owners (see AC1). No seat claims it without the output. *Revised 2026-11-02: a missing receipt blocks completion; see the revision below.* |

## Revision after the review return (2026-11-02)

The review panel (correctness, guarantees and independent, all `residual`,
max medium; security and tests clean) found the recipe shape, the loops and
the fixtures sound. That part is **not reopened**. What it found defective
is a criterion. AC1 and AC11 let "not run" satisfy the gate inventory, and
AC1 named CI as an owner of an in-run receipt, although no seat pushes. The
verify passes at 53ec6b4 launched the pinned engine (`brokkr 29dc3309…`)
but carried the **old** pass note (`… candidate binary, bundle, exact …`).
So they ran the `verify-seat.sh` pinned when the run started, which
compiles `self` only. They say nothing about `eval`. The run pins its
bundle at launch, so no tree edit can change what this run's verify
executes.

Both positions agree that the receipt is required and that the shape stays.
They differ in four places, and this design rules on each:

| Question | Simplicity | Robustness | Ruling, and what the losing side costs |
|---|---|---|---|
| What does a missing receipt block? | `complete`: the run is not reported complete and the PR is not offered without it. No new verify fact or rule. | A verify `pass`: verify returns `residual`/`fail` naming AC1 unrun. | **Simplicity's mechanism, robustness's strength.** "Not run" never satisfies AC1 or AC11, and it blocks the implement seat's `complete`, the run's completion and the PR's offer. No typed verify fact or policy rule is added. Such a fact would edit `self/policy.json` and `verify-seat.sh` (the held, protected constitution), and it still could not take effect in this run, whose pin is old. Under the new script it adds nothing: `verify-seat.sh` (`dd893db5…`) already fails on any recipe's compile refusal or sweep line, so a pass from that script **is** a receipt. Cost to robustness: in a run pinned to the old script, a verify `pass` stays silent on `eval`. The reading rule (AC1) and the pinned pass note (AC13) let every reader tell the two apart. |
| How strong is the house rules' hand line? | Leave it as a compile loop. Narrow Ruling 4's text to what the line does, and add one prose clause. | Make it as strong as CI and the seat (empty shelf, missing or symlinked `bundle.json` by name, per-recipe sweep grep) on one long line. Keep Ruling 4's wording. | **Simplicity.** The line already stops at the first non-zero compile. Putting by-name refusals and a sweep grep with status preservation on one shell line would make a third copy of `verify-seat.sh`'s logic, and it would be the only copy with no executable fixture: a text pin cannot run it. That is the drift #148 is fixing. The checks that decide a merge are CI's and the verify seat's, and both keep every refusal. Narrowing Ruling 4's sentence brings an overstated text into line with code that never did more. No executed check is removed, so no gate is weakened. Cost to robustness: a person who runs the line by hand can miss the sweep warning, because the engine exits 0 on it. The prose tells them to read each recipe's output for it, and verify and CI refuse it. |
| Refuse a symlinked recipe directory inside the loops? | No (accepted residual). | Yes: `[ -L "$recipe" ]` in every surface, plus fixture arms. | **Simplicity; the round-3 ruling stands.** The shelf test (AC7) refuses a symlinked recipe directory, and it runs inside `cargo test --workspace --locked`. In the verify seat that step runs *before* the compile loop, and in CI it is the required `test` jobs. So both gates already refuse the symlink. A loop predicate would change `verify-seat.sh` again (a new pin, a held surface) and add nothing a gate does not already do. Cost to robustness: the CI bundle job, run on its own, would compile a symlinked directory through its target. Compile parses JSON and executes nothing. |
| Scope of an exec-judged protected review | Silent. | One sentence: the reading applies to a non-delivery trial recipe only. | **Robustness.** One sentence in Ruling 1 keeps "name the gate `review`" from becoming a way to satisfy layer 1 while removing a delivery recipe's reviewers. Cost to simplicity: one sentence of ruling text, with no code or test. |

They agree on the rest, and this design adopts it: the two in-run receipt
owners in preference order, with CI as the merge-time backstop; that absence
is not refusal (Ruling 3(b) applies only to an observed refusal); the
receipt's evidence (the union of both lists, including git tree ids); the
README's `exit 1`; the count of six drafts and the true number history; and
issue 6 saying "proposes" until the output is attached. Simplicity's one
local guard is adopted too: the pass note's text gets an assertion (AC13),
because the reading rule now depends on it. Left as recorded: the
historical task row T39, which is a record and not a draft (AC10's grep
targets drafts).

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

#### 1.2 Scope of an exec-judged protected review (2026-11-02)

An exec gate counts as the protected review only for a **non-delivery**
trial recipe whose terminal outcomes are data, where nothing ships and
nothing merges. Every delivery recipe keeps its model review panel. Naming
an exec phase `review` is not a way to meet layer 1 while removing a
delivery recipe's reviewers.

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
  *Revised 2026-11-02:* the line itself stays as written; it only compiles,
  and it stops at the first non-zero exit. The prose must say that, and say
  which surface does what. CI's loop and the verify seat refuse an empty
  shelf and a directory without a regular `bundle.json` **by name**, and
  they refuse the policy-sweep warning per recipe. A person running the line
  by hand reads each recipe's output for `lets the run go on without
  reading`, because the engine exits 0 on it. The prose must not claim that
  the line performs a check that only CI and the verify seat perform.
- **`.brokkr/recipes/README.md`**. In the recipe guide's compile examples,
  use the house rules' line **verbatim** (`… || exit 1; done)`), in its
  subshell, and chain the next commands on it with `&&`. `break` is
  forbidden there, because it leaves the loop with status 0 and lets
  `recipes show` and `run` proceed after a refused compile. In the eval
  section, say `trial` → `review` and explain that the scoring gate is the
  protected review.

### 3. Brokkr issue drafts (`specs/eval-brokkr-recipe/implementation.md`)

Drafted, not filed. There are **six** drafts (issues 1 to 6). Filing them,
or attaching each to the brokkr#766 sub-issue that covers it, and recording
the numbers is a merge precondition owned by the operator (earlier AC12).
The dispatcher files them before merge. No seat files anything. The held-PR
contract and the operator list in `specs/eval-brokkr-recipe/implementation.md`
say "six" and name issue 6 (the protected review gate) explicitly. Issue 6
is not filed while its bracketed slot is empty.

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
> non-removable (extension model, layer 1)`. Swarmail's PR #316 proposes the
> same recipe with its exec scoring gate named `review` (`class: gate`, exec
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

*Revised 2026-11-02:* "now ships" became "PR #316 proposes", because no
engine had accepted the shape when the draft was written. The slot is
filled from AC1's receipt. If the implement seat holds the receipt, it
fills the slot. If the receipt is a candidate-pinned verify note, the
dispatcher quotes that note when filing.

## Acceptance Criteria

- **AC1 The eval recipe compiles under the pinned engine (revised
  2026-11-02).** The §2 loop runs over every directory under
  `.brokkr/recipes/` with the artifact whose sha256 equals
  `.brokkr/recipes/self/tooling.json`'s `brokkr.binary_sha256`. Every
  compile, `eval`'s included, exits 0, and no recipe's output contains
  `lets the run go on without reading`.
  **"Not run" never satisfies AC1.** It is recorded honestly and it blocks
  `complete`. The implement seat does not report `complete`, the run is not
  reported complete, and the PR is not offered for merge until a receipt
  exists. A refusal is not an absence: an observed refusal applies §1.1,
  and an absent receipt **never** moves `eval` off the shelf.
  **In-run receipt owners, in preference order:**
  1. **The implement seat, with a staged binary.** The dispatcher stages
     the digest-matched artifact at `.brokkr/state/bin/brokkr` before the
     implement visit. The seat shows `sha256sum .brokkr/state/bin/brokkr`
     beside the recorded value, runs the loop, and records: the date; the
     exact command; the digest line and the recorded value; each recipe's
     verbatim output and exit status; and the git tree id of each compiled
     recipe directory (`git rev-parse <commit>:.brokkr/recipes/<name>` at
     the commit it compiled, with a clean recipe tree). This is preferred,
     because the receipt lands in committed text **before** verify, and
     verify then gates the same head.
  2. **A verify seat entered from a launch whose pinned self bundle carries
     `verify-seat.sh` sha256 `dd893db5…`** (or a later pin that keeps the
     loop). Its pass note reads `every recipe bundle` and ends with
     `brokkr 29dc3309…`. The script fails on any compile refusal or sweep
     line, so that note **is** the receipt, and it stays in the journal. It
     is not committed back into the tree, because a commit after verify
     changes the head the verify and the review are bound to. The
     dispatcher quotes it in the PR body and in issue 6 when filing.
  **CI's `brokkr bundle (pinned)` is the merge-time backstop and
  corroboration.** It is never the in-run receipt: it proves the pushed PR
  head, and no seat pushes.
  **Reading rule:** a verify pass note that says `bundle,` rather than
  `every recipe bundle,` compiled `self` only and says nothing about any
  other recipe.
  **Binding:** a receipt is void once any recipe directory's tree id
  differs from the recorded one. Commits that touch only documents outside
  `.brokkr/recipes/` do not void it.
  **If neither owner can be arranged,** the implement result is not
  `complete`. It names the dispatcher's action: stage the artifact and
  re-enter implement, or continue at verify from a launch pinned to the
  candidate's bundle.
  **Proof:** route 1 writes the receipt to
  `specs/eval-recipe-review-gate/implementation.md` (AC1 row), to 0119 R4
  as a dated observation line, and to issue 6's slot. Route 2 is the
  journal's verify result, quoted by the dispatcher.
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
  longer requires the `self`-only line. *Revised 2026-11-02:* the prose
  also says that the line only compiles, that the by-name refusals and the
  per-recipe sweep check belong to CI and the verify seat, and that a hand
  run reads each recipe's output for the warning. The same pin requires
  that clause. `.brokkr/recipes/README.md` uses the line verbatim with
  `|| exit 1`, and a pin refuses `|| break` in it. **Proof:** those pins,
  each red before its doc change.
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
  `proposed`. *Revised 2026-11-02:* the R4 revision in the design commit
  adds Ruling 1's non-delivery scope and Ruling 3's receipt rule, brings
  Ruling 4's text into line with the code, and makes the header's number
  history true. In the `docs/decisions/README.md` 0119 row, the status
  column stays `proposed — awaiting the operator`, and only the notes
  column is corrected: main took 0115 (#308), then 0116 (#313, implicit
  TLS), and this branch's decision was renumbered 0116 → 0119 on
  2026-11-02. Main's own 0116 row is not this branch's to edit. (This
  loosens the earlier "row unchanged", because the row's notes state a
  falsehood; plan.md gives the evidence.) **Proof:** `git diff` of the
  index touches the 0119 row's notes column only, and
  `grep -n '0116 stayed free' docs/decisions/README.md` finds nothing.
- **AC10 The brokkr drafts are current.** `implementation.md` carries §3:
  the F4 row, issue 4's appended line, the reworded issues 1/3/5 and issue
  6 verbatim, all marked drafted, not filed. *Revised 2026-11-02:* issue 6
  says "PR #316 proposes" and not "now ships". The PR contract and the
  operator list say **six** drafts and name issue 6. The operator list
  says that issue 6 is not filed while its slot is empty. The text "0119
  is provisional (0115 is held by #221's branch)" is replaced by the true
  number history. The gate-receipt line "Not run here, run by CI: the
  pinned-bundle compile (including `eval`)" no longer names CI as the
  owner of the pinned compile; it points at AC1's receipt. **Proof:** the
  file, and greps showing that no draft says "the score seat", that
  `now ships` and `provisional` find nothing in the drafts or the PR
  contract, and that no draft count says "five".
- **AC11 Every house gate is clean, all `--locked`.**
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo test --workspace --locked`
  - `cargo build --locked --bin swarmail`
  - the evals fmt, clippy and test against that binary
  - the §2 compile loop with the pinned artifact, per AC1. *Revised
    2026-11-02:* "not run" is not a clean gate. It is recorded and it
    blocks `complete` (AC1).
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
  operator by name. It says that the **six** brokkr drafts, issue 6
  included, await filing. It quotes AC1's receipt and names its route
  (staged binary, or the candidate-pinned verify note). It says that the
  verify seat's broader compile takes effect for runs whose pinned bundle
  includes it, and that CI's pinned-bundle job corroborates the receipt.
- **AC13 The verify pass note is pinned (added 2026-11-02).** AC1's reading
  rule depends on the pass note's text. The pass arm of
  `verify_seat_refuses_missing_or_mismatched_engine_identity_before_gates`
  (`tests/delivery_constitution.rs`, the `result == "pass"` assertion after
  "A matched identity reaches an actual pass verdict") also asserts that
  the notes contain `every recipe bundle` and end with
  `brokkr <launched digest>`. A scratch mutant of the fixture's copy of
  `verify-seat.sh` that turns `every recipe bundle` back into `bundle`
  fails that assertion, in the same scratch-mutant style the test already
  uses for the digest comparison. **Proof:** the test, green on the
  shipped script, with the mutant arm caught. No production code
  changes.

## Out of scope, and residuals named for the operator

- A live engine **run** of the recipe (oracle and null) remains the
  operator's first engine run (0119 §7). A compile proves the shape, not the
  run-time behaviour of an exec-only protected review.
- Whether the engine applies further review rules at run time (issue 6,
  question 2) is unknown until that run.
- A symlinked recipe merged past review would be compiled through its
  target by CI's bundle job taken alone. AC7's shelf test refuses it, and
  that test runs inside `cargo test --workspace --locked`: in the verify
  seat before the compile loop, and in CI's required `test` jobs.
- In a run pinned to the pre-change `verify-seat.sh`, a verify pass says
  nothing about `eval`. AC1's reading rule and AC13 make that visible, and
  AC1 makes the receipt a completion precondition.
- A hand run of the house rules' line does not refuse the sweep warning.
  The prose says so (AC6), and CI and the verify seat refuse it.
- A hung compile in CI is bounded only by the job's default timeout.
