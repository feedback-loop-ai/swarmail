# 0119 — The eval recipe on brokkr: interim opaque driver, journal accounting, retained tool-call records

- **Status:** proposed
- **Date:** 2026-11-02 (revised the same day after the first review return,
  decision 0102's 2026-10-31 addendum; the revision is marked **R2** below;
  revised again after the second return, which found the gate's agreement
  rule wrong, marked **R3**; revised a third time the same day after CI's
  pinned-bundle job refused the recipe, marked **R4**; R4 revised the same
  day after the fourth review return, its revision marked
  *R4 revision, 2026-11-02*)
- **Scope:** Epic #164 issue #148 (P1), delivered from main `f815d48` on the
  zero-Codex `claude-careful` recipe (0098/0099, rework per 0102).
- **Specification:** [the eval recipe on brokkr](../../specs/eval-brokkr-recipe/spec.md).
- **Records:** the operator's rulings of 2026-10-18 on #148, already cited
  by accepted [0055](0055-agent-eval-harness.md) (context) and left for
  "the next run's recipe/driver".
- **Supplements:** accepted 0055 §§6–10 and accepted
  [0076](0076-sequential-eval-trials.md) compatibly; accepted
  [0075](0075-eval-privacy.md) compatibly in §3 and by **amendment** in §4
  and §6, both of which wait for the operator.
- **Preserves:** 0001–0005; accepted 0020, 0045 §8, 0050, 0056 and its
  2026-11-01 addendum, 0063, 0071, 0098, 0099, 0100, 0102.
- **Number:** 0119. Drafted as provisional 0116: 0115 was held by #221's
  branch (`repair/issue221-trial-isolation`, commit `cacd60e`), intake saw
  no 0115 on `main`, and the task called 0115 the highest in use, so the
  next free number across both trees was 0116. Kept at the merge of main
  `5645bd9`, where `main` took 0115 for #221's trial isolation (#308). On
  2026-11-02, `main` took 0116 for #313's implicit TLS, so this record was
  renumbered 0116 → 0119 (commit `03d3dea`). That commit's mechanical
  rename also turned this paragraph's original "0116" into "0119"; the
  paragraph was restored to the true history by the R4 revision.

## Context

The #151 baseline (395/400, 2026-11-01) could not measure tokens per
success and retained no transcript or tool-call record: the wrappers
discard raw CLI output by design (0075) and the rows carry no accounting
field. Both were deferred to the #148 recipe. Issue #148's acceptance
criteria ask for one command over a task × model × trials matrix, a report
with a confidence interval and token, turn and time distributions, a success
check proven able to fail, and trials that cannot see each other.

## The 2026-10-18 rulings, recorded

1. **The harness is built on brokkr.** Each trial is, eventually, a brokkr
   run of an `eval` recipe with one agent seat and one exec scoring gate.
   brokkr's journal owns tokens, turns and cost per seat; `brokkr compare`
   and analytics give the per-model view. The agent harnesses are the real
   CLIs (Claude Code, Codex, dsh), never API mocks.
2. **Interim path.** brokkr cannot yet lawfully give a seat Swarmail's MCP
   server (the MCP broker, brokkr 0065 slice two, is mid-build; URL-connection
   MCP is outside it, brokkr 0077; no process may outlive its seat). Until
   those land, the `trial` seat runs an opaque eval-trial driver that takes
   free ports, starts Swarmail and the fixture app as its own children,
   writes the MCP config, runs the real `brokkr driver claude|codex|dsh` as a
   child so the journal keeps tokens, turns and tool rows, saves the app's
   state for the scoring gate, then stops both servers.
3. **Unconfined.** Nothing in that seat is confined: Codex sees the
   operator's own MCP servers; dsh's web tools stay on. Every result records
   the trial as `unconfined`.
4. **Gaps are brokkr's.** Any brokkr capability the recipe needs and brokkr
   lacks is filed on feedback-loop-ai/brokkr and named in the PR, never
   worked around inside Swarmail. The move to the broker is tracked in
   brokkr#766 (sub-issues brokkr#757–#765, plus brokkr#467, #462 and #430).

## Proposed ruling

1. **Shape.** `.brokkr/recipes/eval/` holds two exec seats: `trial` (runs
   `swarmail-evals trial` with the matrix's request; result `ran` carrying
   `inputs.trial_id`, or `harness_error`) and `score` (runs
   `swarmail-evals check --artifacts-dir` on the retained leaf; `pass`,
   `fail` or `unscorable`). No retries. A failed trial is data (`done`); an
   unscorable one or a trial that produced no row stops. **R3:** the score
   seat's result is the trial's class whenever the business replay agrees
   with the row under §2's agreement table (`pass` for a pass row, `fail`
   for a business or budget failure), and `unscorable` for a
   `harness_error` row or a contradiction, so the run's terminal (`done` or
   `stop`) and the row's outcome never disagree for a row the runner can
   produce. The policy's five rules and transitions do not change. The recipe reuses
   the runner's lifecycle, admission, scoring and retention unchanged. Both
   seats have workspace hands; the score seat has no network and writes
   exactly `<slot>/score.json` (**R2**: its hands are not read-only). Both
   seat scripts parse the engine's rendered prompt (`Feature: <text>`, an
   indented result path) by trimming each line and matching the request
   token anywhere on it (**R2**). **R4:** the scoring gate is the recipe's
   protected review. The `score` seat and phase are renamed `review`, and
   the bundle declares `protected_phase: "review"`. The seat's class,
   driver, script (`score.sh`), hands, limits and results are unchanged,
   and so are the five rules apart from their ids (`REVIEW-*`). See the R4
   section below.
2. **One command, one row per slot.** `swarmail-evals matrix --brokkr BIN`
   runs each slot as one `brokkr run --recipe eval`, reads the slot's row,
   exit and gate verdict, and keeps 0076's accounting: a worker that returns
   no row, a row for another cell, an exit that disagrees with the row, a
   gate that never ruled (`score_missing`, **R2**) or a gate whose business
   replay contradicts the row under the agreement table (`score_mismatch`,
   **R3**) is that slot's `harness_error`, stops admission and writes the
   refusal suffix.

   **R3, the agreement table.** The gate replays only the business check
   over the retained state (`check --artifacts-dir`); the row applies the
   whole precedence of accepted 0055 §2 and 0071 §2 (infrastructure, then
   the deadline, then the business check). The two are equal only for
   business-decided rows, so agreement is defined against the row's own
   precedence, never as raw equality. The row alone shows which leg decided
   it, because a task's business failure class can never be `budget` or
   `harness_error` and the runner yields `fail/budget` only from the
   deadline leg:

   | Row | Decided by | Agreeing business replay |
   |---|---|---|
   | `pass` | business | `pass` |
   | `fail`, any class but `budget` | business | `fail` |
   | `fail/budget` | the deadline, over a snapshot the runner validated | `pass` or `fail`, never `unscorable` |
   | `harness_error` | infrastructure | any ruled replay |

   A budget or infrastructure row whose state happens to pass is committed
   as the row says; the gate never promotes it, and the matrix never
   rewrites it. The gate's business verdict is recorded under its own name
   (`business`) and never as the trial's outcome. The slot root must lie inside
   the workspace the seat's hands reach, checked before any spawn (**R2**).
   The brokkr-mode envelope covers the trial deadline, the direct envelope's
   slack and the score seat's budget (**R2**). Trials stay sequential (0076);
   isolation remains #221's; the issue's two-concurrent-trials criterion is
   not met by this recipe and is not claimed.
3. **Accounting joins by `trial_id`, never by discovery; every fact is
   typed.** Rows do not change. `report --journal FACTS` joins a typed
   journal-facts file (produced from the pinned brokkr's export by
   `evals/scripts/journal-facts.sh`) to the rows by the `trial_id` the matrix
   assigned before spawn and the trial seat echoed in its result. Each of
   the five facts (tokens, turns, wall time, deadline-killed, tool calls) is
   present with a value or unavailable with a reason from the closed set
   (`journal_missing`, `journal_ambiguous`, `journal_has_no_accounting`,
   `builtin_agent`); a missing fact is never `0`, `false` or `[]` (**R2**: an
   empty tool list is a present fact, an absent one is unavailable). Tokens
   per success, the tool-call aggregate and the deadline-killed count are
   each `null` with a reason whenever any scored trial in a group lacks the
   fact or (for tokens per success) no trial passed. A facts script that
   cannot read the export exits with a typed failure and writes no facts
   file; the report without a facts file is unchanged (**R2**). This is a
   compatible reading of 0075 §4 (harness-authored finite facts) and of
   0055 §10 (report formulas unchanged, new figures added beside them).
4. **Retained tool-call records.** Per joined trial whose tool-call fact is
   present, the report may write a bounded `tool-calls.json`: ordinal,
   bounded tool name, outcome class, invalid-argument flag, argument and
   result byte lengths and SHA-256 digests; never argument or result bytes;
   never an empty file standing for an unavailable fact (**R2**). This is
   compatible with 0075 §4 and lands with the recipe. **By amendment,
   awaiting the operator:** a value-bearing mode that also retains the
   *argument values* of the two harness-owned tools (Swarmail's MCP tools and
   `eval_http`), value-redacted against the fixture admin secret and
   registered sentinels, fail-closed to absence with a typed reason when
   redaction cannot be proven, bounded and hash-bound as above, and never
   scored. 0075 §4 suppresses raw bodies and URLs at every publication sink
   and names transcripts an accepted loss; this clause reverses that for the
   two tools whose values are, by construction, the fixture's synthetic data.
   It is not implemented until accepted (0045 §8).
5. **Via-brokkr surfaces, refused until observed.** Rows for agents launched
   through the wrapper's `--via-brokkr DRIVER` declare the finite selectors
   `claude_2_1_310_via_brokkr`, `codex_0_164_0_via_brokkr` and
   `dsh_0_1_5_rc_1_via_brokkr`: the family's declared configuration plus
   one residual naming the driver's own flags. A via-brokkr row never
   claims the direct-launch surface, and a via-brokkr profile is never
   declared for a direct launch (`via_brokkr_profile_mismatch`). **R2:**
   until the real `brokkr driver <family>` invocation is observed and
   recorded here by dated addendum, `--via-brokkr` refuses with
   `driver_contract_unknown` at the trial (before any launch, no row) and at
   the wrapper (before any private read); the argv assembly stays in the code
   under test reach only. No stand-in driver ships: a guessed invocation that
   runs is the workaround ruling 4 forbids, and a real-versus-stand-in switch
   would be a production seam.
6. **The accounting channel, if brokkr needs one, crosses four seams.**
   **R2** (replaces "the wrapper forwards exactly that variable"): every
   candidate channel between a nested driver and the engine is severed today
   by (1) `trial.sh`'s `env -i`, (2) the runner's freshly built agent
   environment, (3) the wrapper's `env_clear` on the driver launch, and (4)
   the wrapper's `/dev/null` routing of the driver's stdout and stderr. When
   the channel is observed: for one named environment variable, exactly that
   name is added at seams 1–3 and nothing else; for a stdout relay, seam 4 is
   opened for the driver only. Either is an exception to 0075 §1 and lands
   in a follow-up change only after this clause is accepted with the observed
   name or relay recorded by dated addendum. Until then every via-brokkr
   group's accounting is unavailable with a typed reason, and the delivery
   that introduces the recipe does not measure tokens per success.
7. **Gates, holds and what waits.** The recipe compiles in CI's pinned-bundle
   loop beside the six existing recipes; the verify seat compiles only
   `self` and is not changed for this (**R2**). **R4 supersedes that
   clause:** CI, the verify seat and the house rules' gate inventory each
   compile every directory under `.brokkr/recipes/`, found by glob, with
   no fixed list of names. The PR touches `.brokkr/` and
   `.github/` and merges only on the operator's approval by name (0056's
   2026-11-01 addendum), and only after the brokkr capability gaps are filed
   on feedback-loop-ai/brokkr or attached to brokkr#766's sub-issues and
   named in the PR (ruling 4; **R2**: a draft is not a filing, and filing is
   the operator's act when the seats have no network). The first engine run
   of the recipe (oracle, offline), the first authenticated via-brokkr smoke
   per family (which observes F1, F2 and the paid binds F5) and the paid
   matrix are the operator's; paid agents under `matrix --brokkr` are
   unsupported until that smoke and the follow-up (**R2**). Delivery proves
   the recipe with the oracle and null agents and the stand-in worker only.

## Synthesis, rejected alternatives and costs

Both design positions agreed on reusing the runner, an exec trial seat, no
sqlite or new dependency, typed unavailability and the filing rule. Adopted
from simplicity: the two-shim recipe, one wrapper flag, one matrix mode,
rows unchanged, a report-side join, the via-brokkr selectors, and the
reading that the matrix's accounting is already the attempt ledger. Adopted
from robustness: a separate three-exit-code gate the matrix cross-checks, a
spawn-time join key, fail-closed structural evidence bound by the artifact
triple, and the crash, join, unavailable, gate-failure and sentinel tests.
Rejected: a separate fsynced attempt ledger (the matrix already writes one
row per slot), a schema bump (re-opens 0063/0071/0076 readers and the
published baseline), parsing CLI stream-json in Swarmail (a workaround for
brokkr's accounting), a parent-held or bypassed admission inside the exec
box (weakens 0076 §1–2 where `/tmp` is shared), and any Swarmail-side
transcript capture.

**R2 (after the review return).** The review found the design asserting
seams it had never observed and the stand-ins authored to the assertions.
Adopted from both round-2 positions: the engine's prompt rendering, typed
tool-call and deadline-killed facts, the four seams, the corrected compile
and hands claims, filing as an operator-owned precondition. Ruled for
simplicity: the via-brokkr refusal is unconditional and the stand-in driver
goes (no production switch); `journal-facts.sh` dies typed rather than
writing a facts file about an export nobody read; no bind is guessed. Ruled
for robustness: `score_missing` is distinct from `score_mismatch`; the
brokkr-mode envelope carries the score seat's budget; the slot root is
preflighted; the via-brokkr profile is cross-checked at the trial. Rejected:
a `journal_unavailable` reason and a soft facts file; a paid bind guessed
from `verify-seat`'s `~/.rustup` example; an end-to-end via-brokkr arm kept
alive by a real-versus-stand-in switch.

Costs accepted: a second scorer run per slot; one bounded evidence file per
trial; **tokens per success is not measured until the channel is observed
and forwarded under §6**; via-brokkr trials refuse until §5's addendum; paid
agents under `matrix --brokkr` wait for the same smoke; admission is blind
to other host runners if the exec box has a private `/tmp` (the baseline's
own posture); value-bearing evidence waits for §4's acceptance.

**R3 (after the second review return).** All four reviewing members found
§2's "the gate agrees with the row" defined as equality between a
business-only replay and a total verdict, so a legitimate `fail/budget`
trial over a passing state, or a `harness_error` row over a valid one, was
rewritten as `score_mismatch` and stopped the matrix. Ruled: the agreement
table in §2 (both positions; robustness's "budget never unscorable" clause
adopted). Ruled for robustness against simplicity: the score seat reports
the trial's class, reading the harness-authored row's outcome and failure
class only to choose its result, so the journal never records `done` for a
`harness_error` row and `brokkr compare` never counts a budget failure as a
pass; simplicity's "policy unchanged, the seat names only the state" would
have left both. Ruled for simplicity against robustness: no derived
`outcome` in `score.json` (copied from the row, it checks nothing), no new
policy rule for a result outside the seat's vocabulary, no full-precedence
replay in the gate (the deadline leg's only retained input is the row's
own claim). Cost accepted: the agreement table is mirrored in the score
script, outside any coverage denominator, and proven per cell by the
recipe tests and the stand-in arms.

## R4 (2026-11-02): the protected review, and every recipe compiled

**What happened.** CI's `brokkr bundle (pinned)` job refused PR #316 with
the pinned engine (brokkr 0.12.0, sha256 `29dc3309…`):

```
error: bundle: policy has no 'review' phase; the protected review gate is
non-removable (extension model, layer 1)
```

This falsifies the earlier specification's fact F4, that a two-seat recipe
with its own policy and no `review` phase compiles. The local gates and the
verify seat were green only because they compiled `self` alone (§7 R2). CI
compiled a fixed list of names. A green verify seat therefore did not mean
that every recipe compiled.

**Ruling 1: the scoring gate is the recipe's protected review (both
positions).** The phase and the gate seat `score` are renamed `review`. The
bundle declares `protected_phase: "review"`, as `self`, the only other
recipe that owns its policy, already does. No other key or seat is added.
The seat stays `class: gate`, runs the exec driver on `./scripts/score.sh`
with workspace hands, no network and no binds, has one attempt and 600 s,
and keeps the results `pass`, `fail` and `unscorable`. The phases are
`trial, review, done, stop`. The five rules keep their results, transitions
and severities; their ids become `REVIEW-PASS`, `REVIEW-FAIL` and
`REVIEW-UNSCORABLE`. `trial` keeps its name, because it is the only seat
name the journal-facts reader accepts.

This is a true description, not a relabel. The protected review exists so
that nothing finishes without an independent judgement the working seat
cannot write. In this recipe that judgement is the scoring gate. A shell
re-applies the task's check to the retained state and never reads the
agent's account. The R3 agreement table, which makes the gate report the
trial's class, is unchanged. Replacing the results with the delivery words
`clean`/`residual` would reopen the R3 defect, in which a budget failure is
counted as a pass, so they are not used.

*R4 revision, 2026-11-02: scope.* This reading holds only for a
**non-delivery** trial recipe whose terminal outcomes are data: nothing
ships and nothing merges. Every delivery recipe keeps its model review
panel (0013, 0098, 0099). Naming an exec phase `review` is no way to meet
layer 1 while removing a delivery recipe's reviewers.

**Ruling 2: no second, ceremonial review.** A deterministic `review` seat
after `score` was considered and rejected. Every check it could own is
already owned. The row, exit and trial-id pairing, `score_missing`,
`score_mismatch` and the slot root belong to the matrix's slot reader. The
strict row read belongs to `score.sh`. A second seat would carry the
protected gate's name only to satisfy the engine, and a protected gate that
cannot fail independently is the rubber stamp the house refuses.

**Ruling 3: if the engine refuses this shape, follow this order.** The
design seats had no engine, so Ruling 1 is a compile hypothesis. A receipt
from the pinned artifact decides it.
(a) If the refusal names an additive field that changes no result word,
seat or transition (for example `shippable_from`), add the field and record
it here by dated line.
(b) If it requires delivery result words, a review panel or a model seat,
no lawful shape exists under this engine. Move the recipe out of
`.brokkr/recipes/` so that the compile gates stay green. Record here that
the brokkr-mode matrix is not delivered. Report the run as not complete,
and name brokkr issue 6.
No result word is remapped, and no ceremonial seat is added, to make the
compile pass.

*R4 revision, 2026-11-02: the receipt is a precondition.* "Not run" never
satisfies this ruling's receipt. It blocks the implement seat's `complete`,
the run's completion and the PR's offer for merge. An absent receipt is not
a refusal, and it never moves the recipe off the shelf: (b) applies only to
an observed refusal. The receipt has two in-run owners, in this order.
First, the implement seat, using a digest-matched artifact staged at
`.brokkr/state/bin/brokkr`. It records the date, the command, the digest
beside the recorded value, each recipe's verbatim output and exit status,
and each recipe directory's git tree id, here by dated line. Second, a
verify seat entered from a launch whose pinned self bundle carries the
looping `verify-seat.sh` (`dd893db5…` or later). Its pass note,
`every recipe bundle … brokkr 29dc3309…`, is the receipt, and it stays in
the journal. CI's `brokkr bundle (pinned)` corroborates the receipt at
merge time and is never the in-run receipt. A verify pass note that says
`bundle,` rather than `every recipe bundle,` compiled `self` alone. A
receipt is void once a recipe directory's tree id changes.

**Ruling 4: one gate inventory compiles every recipe.** The fixed list
evicted `eval` from the local gates. CI's loop, the verify seat's compile
and the house rules' gate line each iterate every directory under
`.brokkr/recipes/` by glob, and each one stops at the first non-zero
compile. CI's loop and the verify seat also refuse an empty shelf, and they
refuse a directory without a regular `bundle.json` by name, so a stray
directory fails the gate instead of being skipped. They check the
policy-sweep warning for every recipe, not only for the last one. The house
rules' line only compiles. The by-name refusals and the sweep check belong
to CI and the verify seat, and the prose beside the line says so (*R4
revision, 2026-11-02*: the earlier sentence said the line did all of this,
which overstated the line). Executable fixtures with a sentinel recipe
prove CI's and the verify seat's behaviour.
The shelf test lists the directory instead of naming the recipes. §7 R2's
"the verify seat is not changed" is superseded. The verify seat's script is
a pinned bundle script (0050 §1). The change widens a gate and weakens
nothing.

**Ruling 5: the brokkr issue drafts stay current.** Issue 4 is corrected:
its premise is false. Issue 6, the review-gate finding, is drafted. Issues
1, 3 and 5 name the gate by its role. Filing remains the operator's merge
precondition (§7, earlier AC12). A draft is not a filing.

**Costs accepted.** The rule ids `SCORE-*` become `REVIEW-*`. No trial has
run under the recipe, so no journal row carries the old ids. Moving the
recipe out under 3(b) would remove the one-command brokkr path until
brokkr answers issue 6. A fail-first loop reports one refused recipe per
run. This run's own verify seat runs the script pinned when the run
started, so the first compile of `eval` comes from a staged pinned binary,
a verify continuation entered from the candidate's bundle, or CI. Each such
receipt is recorded here by dated line. (*R4 revision, 2026-11-02:* CI
corroborates and is not an in-run owner; see Ruling 3. A run may now stop
at implement and wait for the dispatcher to stage the artifact. That is the
honest cost of requiring the receipt.)

**Observation, 2026-11-02 (implement seat).** The renamed shape is built
(`review` seat and phase, `protected_phase: "review"`, `REVIEW-*` rule ids)
and pinned by `eval_recipe_defect`. Ruling 3's engine receipt is **not run**:
the seat held no staged pinned binary (`.brokkr/state/bin/brokkr` absent,
`binary_sha256` `29dc3309…` not on the host) and no network. Ruling 1 is
therefore still a compile hypothesis. The receipt belongs to a verify
continuation entered from the candidate's bundle, whose compile now visits
every recipe directory, or to CI's `brokkr bundle (pinned)` job. A refusal
there applies Ruling 3 and is recorded here by dated line.

Specification: [eval recipe review gate](../../specs/eval-recipe-review-gate/spec.md).

## Delivery and acceptance ownership

The chief commits this record with the spec, plan and tasks (and its
revision, on top, never amended). Implementation adds the index row and the
dated F1–F7 observations or drafted issue texts. Only the operator accepts
this proposal, records the §5 invocation and the §6 channel by dated
addendum, files or attaches the brokkr issues, and approves the held PR by
name; the engine alone advances delivery. Commit unsigned; never push or
merge.
