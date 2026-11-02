# The eval recipe on brokkr: interim opaque driver, journal join, retained tool-call records

Date: 2026-11-02. Status: design, **revision 2** (returned from review under
accepted 0102's 2026-10-31 spec-defect addendum; the first design commit
`8b9abc6` and the build it produced, `080db77`, stand in history and are
revised here, never amended). Run `issue-148-the-eval-recipe-on-bro-197e7def`.
Issue #148, epic #164, P1. Operator direction 2026-11-01: start now,
alongside a separate harder-tier run. Zero-Codex `claude-careful` lane
(accepted 0098 and 0099). Base: `main` at `f815d48`.

Decision: 0116 — [the eval recipe on brokkr](../../docs/decisions/0116-eval-brokkr-recipe.md),
status proposed. It records the operator's 2026-10-18 rulings (the harness
is built on brokkr; the interim opaque driver; every interim trial is
unconfined; brokkr#766 tracks the move to the broker), the compatible
reading of accepted 0075 this spec relies on, the value-bearing evidence
mode that waits for the operator, and (revision 2) the four environment and
output seams any accounting channel must cross. Only the operator accepts it.

**Number.** The task says 0115 is the highest in use and intake found no
0115 in this tree. Both are right: 0115 is held by #221's branch
(`repair/issue221-trial-isolation`, commit `cacd60e`, "Renumber the trial
isolation decision to 0115"), not yet on `main`. Drafting 0115 here would
collide at the merge of either branch, so this spec takes **0116**. If
`main` takes 0116 first, the record is renumbered at integration the way
0105 and 0108 were.

## Feature and rationale

The #151 baseline ([docs/evals/baseline-2026-11-01.md](../../docs/evals/baseline-2026-11-01.md))
measured 395/400 and left two things "Not measured": **tokens per success**
(the rows carry no token, turn or cost field; the wrappers discard raw CLI
output by design) and **transcripts or tool-call records** (raw CLI output
goes to `/dev/null` on every outcome and 0075 has no debug mode). Both were
planned to come from brokkr's journal "once the #148 recipe wraps the
runner, which it does not yet."

This change is that recipe, on the **interim path** the operator ruled on
2026-10-18. brokkr cannot yet lawfully give a seat Swarmail's MCP server
(the broker, brokkr decision 0065 slice two, is mid-build; URL-connection
MCP is outside it, brokkr 0077; no process may outlive its seat). Until
those land, an `eval` recipe's `trial` seat runs an opaque eval-trial
driver: `swarmail-evals` starts Swarmail and the fixture app as its own
children, writes the MCP config, runs the real `brokkr driver
claude|codex|dsh` as a child so brokkr's journal records tokens, turns and
tool rows for that seat, saves the app's state for a separate exec scoring
gate, then stops both servers. Nothing in that seat is confined: Codex sees
the operator's own MCP servers, dsh's web tools stay on. Every result
records the trial as `unconfined`.

**What this delivery measures, stated plainly (revision 2).** It lands the
measuring machinery: the recipe, the slot accounting, the typed journal
join, the structural evidence and the via-brokkr selectors. It does **not**
measure tokens per success, because the accounting channel between a nested
`brokkr driver` and the engine has never been observed and, as the code
stands, every candidate channel is severed by four seams this spec now
names (F1). Closing the #151 "Not measured" rows needs, in order: the
operator's first engine run (free, offline), the operator's first
authenticated via-brokkr smoke per family (which observes F1 and F2), a
follow-up change that forwards the observed channel through the four seams
under 0116 §6 (an exception to accepted 0075 §1 only the operator can
accept), and then the paid matrix. The PR says this in one sentence.

What already exists and is **reused, not rebuilt**: `swarmail-evals trial`
(fresh in-memory Swarmail and fresh fixture on port 0, owned Unix process
groups, deadline, TERM/KILL/reap/drain, freeze then one consistent snapshot
through the fixture's admin API, scoring, one schema-2 row, retention under
`--artifacts-dir`); `agent-claude|agent-codex|agent-dsh` (private HOME, the
MCP config with Swarmail HTTP plus the `eval_http` helper, cleared
environment, the eight EVAL inputs); `check` (re-scores a retained leaf,
exit 0 pass / 1 fail / 2 unscorable); `matrix` (one row per requested slot,
no retries, sequential admission under accepted 0076, refusal suffix, exit
2 > 1 > 0); `report` (Wilson interval, nearest-rank quantiles, failure
taxonomy). What is new: the recipe and its two exec scripts, one flag on
the wrappers, one worker mode on the matrix, a typed journal-facts input
to the report, and a bounded redacted tool-call record per trial.

## Panel synthesis: rulings (round 1, unchanged)

Both positions agree on: reuse the runner's lifecycle; the trial seat is an
exec driver running `swarmail-evals`; no sqlite and no new dependency; no
raw CLI stdout or stderr is retained by Swarmail; a missing journal fact is
a typed reason, never `0`; the gate reads the app's state and never the
agent's account; trials stay sequential (0076); anything brokkr lacks is
filed on brokkr, never worked around in Swarmail. Where they differed:

| Question | Ruling | What the losing side costs |
|---|---|---|
| A separate fsynced attempt ledger before any spawn (robustness A) | **Rejected as a mechanism, adopted as a property.** `matrix` already writes exactly one row per requested slot: a worker that dies without output yields a `harness_error` row, admission stops and every unstarted slot gets a zero-time refusal row (0076 §3). In brokkr mode the same accounting rules the `brokkr run` worker. The brokkr journal is the pre-spawn record of the run itself. | None for the matrix. A lone `brokkr run --recipe eval` started by hand that dies leaves its record only in brokkr's journal; the matrix is the measurement unit and the report reads rows, so that loss is stated, not patched. |
| Bind the child run id at spawn, never discover it (robustness B) | **Adopted through the key the matrix already assigns.** `trial_id` is assigned before the worker spawns, written into the slot's request file, echoed by the trial seat in its typed result `inputs.trial_id`, and is the only join key. A journal with two runs claiming one `trial_id` joins neither (`journal_ambiguous`). No timestamp or "newest run" matching anywhere. | Robustness wanted the run id inside the row before the agent runs; the row does not change (next ruling), so the run id is surfaced by the report, not the row. |
| Extend the v2 row with token and evidence fields under a new schema (robustness) vs rows unchanged (simplicity) | **Rows unchanged.** "Extend, not replace" is met at the report's inputs: `report --journal` joins a typed facts file to the rows by `trial_id`. The published 395/400 rows stay readable as they are; the strict readers of 0063 and 0076 are untouched; no v3, no migration, no 64 KiB pressure. Intake's AC1 wording ("each row carries the brokkr run id") is corrected: the row carries `trial_id`; the report carries the run id. | A row alone no longer tells you its tokens; you need the facts file beside it. The baseline publishes both. |
| Which selector a via-brokkr row declares | **New finite selectors** `claude_2_1_310_via_brokkr`, `codex_0_164_0_via_brokkr`, `dsh_0_1_5_rc_1_via_brokkr` (simplicity): the driver adds flags the family profile does not enumerate, so such a row must not claim the direct-launch surface. Same revision, basis `declared_configuration`, the family's restrictions, plus one residual naming the driver. | Three more enum variants and profile rows in `provenance.rs`; a binary before this change refuses the new rows, as the surface contract already says for historical rows. |
| The scoring gate | **A separate exec seat with three exit codes** (robustness D), implemented by the existing `check --artifacts-dir <leaf>`, which validates the bound evidence (metadata, result, state, diagnostics) and re-scores: 0 pass, 1 fail, 2 unscorable. **And the matrix cross-checks it** against the row (`score.json` must agree with `row.json`, else the slot is `harness_error` and admission stops). Simplicity's "replay equality is already proven" is true, but a gate whose verdict nobody reads is decoration. | About fifteen lines in the matrix and a second scorer run per slot, on a frozen snapshot that is already consistent (the fixture freezes, then snapshots through its admin API; there is no database copy and no WAL, so robustness's WAL concern does not arise). |
| Evidence: journal only (simplicity) vs a bounded, hashed, value-redacted side file, fail-closed (robustness) | **Both, in two steps.** Now: a per-trial `tool-calls.json` derived from the journal facts: ordinal, bounded tool name, outcome class, invalid-argument flag, argument and result byte lengths and SHA-256 digests. **No argument or result bytes.** It is harness-authored finite facts, compatible with accepted 0075 §4, bound by the `artifacts.rs` triple, 64 KiB cap, truncation typed, written through the exclusive-create primitives. Later, under proposed 0116 only: argument values of the two harness-owned tools (Swarmail MCP and `eval_http`), value-redacted and fail-closed to absence. That second step contradicts 0075 §4's "suppress raw bodies, URLs" and **is not implemented in this run** (0045 §8). | Robustness loses value-bearing evidence until the operator accepts 0116. Simplicity loses "nothing to write": one bounded file per trial. The baseline's open question (did the model act on a wrong message or give up?) is answered by tool names, counts and the retained `state.json`. |
| Admission inside the exec box (robustness 9) | **Accepted residual, named.** If brokkr's exec box gives the trial seat a private `/tmp`, the trial's advisory lock is blind to other cooperating runners on the host, which is exactly the posture the operator ran the #151 baseline under (bwrap with a private `/tmp`). The matrix stays sequential, so no two of its own trials overlap. No bypass flag and no parent-held reservation is added: both would weaken 0076 §1–2 for the shared-`/tmp` case. | On a host where the box shares `/tmp`, nothing is lost. On a host where it does not, the operator runs no other trial while a matrix runs, as for the baseline. Brokkr fact F6 records which it is. |
| Crash tests, join tests, unavailable tests, gate-failure tests, sentinel tests (robustness) | **Adopted**, with stand-ins: the matrix's worker stand-in plays `brokkr run` and runs the two real recipe scripts, so the recipe path is proven offline without the engine. | None. |
| Cert, cap, chaos, webhook failure modes (robustness table) | **Out of scope, stated.** The eval has no TLS (loopback HTTP), the harness never changes the server's cap or prunes, the eval runs no chaos, and the fixture has no webhook sink. Nothing here touches 0001–0005. | None. |
| Decision number 0115 (intake) vs 0116 (simplicity) | **0116**, for the collision reason above. | A renumber at integration if `main` takes 0116 first. |

## Revision 2: the review's findings and the rulings

The review panel (correctness, guarantees, independent, tests residual;
security clean) found one pattern: the design **asserted brokkr seams it had
never observed** (the driver argv F2, the accounting channel F1, the export
shape F3, the compile gate AC3, the hands F5) and authored every stand-in to
the assertion. Both round-2 positions name the same root cause and agree on
most corrections; the rulings below settle where they differ. Every change is
a revision of this spec, 0116, plan.md and tasks.md; the committed shape
(two seats, rows unchanged, report-side join, structural evidence, sequential
admission) is not re-opened.

| Finding | Ruling | What the losing side costs |
|---|---|---|
| **Prompt parsing is wrong for the real engine** (correctness 1 low, independent 2 medium). `trial.sh`/`score.sh` match only a line that *begins* `swarmail-eval-request:` and an unindented `/….json`; the engine renders the feature as `Feature: <text>` and the result path indented. Observed in this chief seat's own rendered prompt on 2026-11-02 (`Feature: Issue #148: …`; the result path line indented four spaces). | **Adopted as the real high.** Both scripts trim each line as `verify-seat.sh` and `design-check.sh` do and match the request token anywhere on the trimmed line. The stand-in renders the engine's observed shape. The proving test is written first and observed red against the committed scripts. | None. |
| **`--via-brokkr` runs an unobserved driver contract, fails open** (guarantees 3 high, independent 5 low, guarantees 7 low). | **Refuse unconditionally, at two layers, and cut the stand-in driver** (simplicity). The trial refuses before any launch when the agent argv carries `--via-brokkr` (`driver_contract_unknown`, exit 2, no row) and when a `*_via_brokkr` profile is declared without it (`via_brokkr_profile_mismatch`); the wrapper refuses `--via-brokkr` after `driver_invalid` validation and before any private read, with the same category. Argv assembly stays and is proven inline by inspecting the `Command`. `wrappers_launch/standin-driver.py` and its end-to-end arms go. Robustness's "keep the stand-in for assembly" loses: a real-versus-stand-in switch would be a production seam (0020 §4 in spirit), and the stand-in only ever proved our convention against itself. | No end-to-end via-brokkr trial until F2 is observed. The trial-level refusal matters more than the wrapper's: a wrapper that exits without acting scores as the task's idle class (`gave_up`), a fake model failure inside measurement data; refusing before launch makes it a typed harness refusal instead. |
| **The accounting channel is severed by construction** (independent 1 medium, correctness 2 low). `trial.sh` runs `env -i`; the runner builds the agent environment afresh; `through_driver` calls `env_clear`; the driver's stdout and stderr go to `/dev/null`. 0116 §6 said "the wrapper forwards exactly that variable", one site. | **Both positions agree; adopted.** F1 names the four seams. No forwarding is built now: forwarding touches accepted 0075 §1 at three environment sites and one output-routing site and waits for the observed channel and the operator's acceptance of 0116 §6. One test names the seams and proves a sentinel variable set in the matrix's environment reaches neither the trial nor the agent (the follow-up flips it to the observed name). AC16 and the READMEs stop saying this delivery closes the #151 rows. | Robustness's "sentinel survives all four seams" test cannot be written until the channel exists; it is the follow-up's. Tokens per success stays `null` with a typed reason in every via-brokkr group until then. |
| **F5: no bind for the operator's selection file; the score seat is not read-only** (guarantees 2 high, 4b medium, correctness 6, independent 6). | **Correct the facts; guess no bind** (simplicity, with robustness's preflight). F5 lists the whole paid read set (selection file, three credential files, three CLI executables and their install trees). None is bound: a `~` bind guessed now may break the operator's free offline oracle run and still would not reach the executables. The score seat has workspace hands read-write and writes exactly `<slot>/score.json`. Paid agents under `matrix --brokkr` are unsupported until the F2/F1/F5 follow-up and the PR says so; the direct `matrix` (the #151 path) stays the paid route. New: `matrix --brokkr` refuses an `--artifacts-dir` outside the current directory before any spawn (`brokkr_artifacts_unreachable`). | Robustness's "selection as a matrix input" is not taken: the matrix does not read the operator's private file; the wrapper reads it, inside the seat, and the seat cannot reach it today. The paid path waits. |
| **AC3/0116 §7 claim a verify-seat compile that does not run** (guarantees 4 medium, correctness 3 low, independent 4 low). | **Adopted.** AC3 and 0116 §7 claim only CI's pinned-bundle loop. `.brokkr/recipes/self/` is not edited to make the claim true (hold list; shims). The implement seat records the compile as **not run** without the binary. | No seat compiles `eval` before CI; the shape is pinned by `tests/delivery_recipes.rs`. |
| **A missing `tool_calls` is `[]`, a missing `deadline_killed` is `false`** (guarantees 5 medium). | **Adopted; one rule everywhere.** Facts v1 (unreleased) changes in place: `tool_calls` and `deadline_killed` are typed `{status, value|reason}` like tokens, turns and wall time. `accounted()`'s "non-empty tool list" heuristic goes. `tool_calls` and `deadline_killed` aggregates are `null` with `incomplete_join` when any scored joined trial lacks the fact. Evidence for a trial without tool rows is `absent: journal_has_no_accounting` and no file is written. Robustness's new reason `journal_has_no_tool_rows` is not added: the closed set stays closed and the reason is per fact. | A golden gains the tokens-present/tool-rows-absent case. |
| **`journal-facts.sh` dies instead of F3's soft fallback** (guarantees 8 low). | **Simplicity wins; the spec moves to the code.** The script exits 2 naming the failure class and writes no facts file. Robustness's `journal_unavailable` facts file is rejected: a facts file produced from an export nobody read would label "the journal has no accounting", which is false; `report` without `--journal` is already byte-identical and typed as "no journal"; and nothing is lost, because the journal is append-only and the script is re-run after `EXTRACT` is corrected. | The operator gets an exit code and a class, not a report full of `null`. |
| **A gate that never ruled is mislabeled `score_mismatch`** (tests 1 info; robustness R8). | **Adopted.** `score_missing` (absent, unreadable, malformed or not one line) is distinct from `score_mismatch` (present and disagreeing); the stand-in gains a `no-score` mode. | One more typed category and one more arm. |
| **The envelope and the score seat** (robustness R10, not in the review). | **Adopted with the premise corrected.** The matrix envelope is already the trial deadline plus 300 s, so a 1 195 s trial leaves 305 s, not 5 s, for a gate that normally takes seconds. In brokkr mode the envelope additionally carries the score seat's 600 s budget, so `trial_envelope_expired` there can only mean the engine or the gate hung past its own seat timeout. The arithmetic is a pure function with an inline test; no sleeping wire test. | A hung engine holds a slot up to 600 s longer. |
| **Filing is a draft** (guarantees 1 high, independent 3 low). | **Both positions agree; adopted.** The criterion stays "filed on brokkr and named in the PR". The implement seat reads brokkr#766 and its sub-issues when its hands allow; without network the PR body carries the issue texts verbatim and **filing, or attaching to an existing sub-issue, and recording the numbers is a precondition of merging, owned by the operator**. AC12 is operator-owned work outside the run's required work, so `complete` is reported honestly without claiming it. | The PR is held one more way; it is already held by name under 0056. |
| `trial_args`' `let _ = bin` (correctness 4, independent 6 info). | **No change.** `bin` is used under `#[cfg(test)]` to build the target; the `let _` silences the non-test build. The finding is inaccurate. | None. |
| Categories from the slot's `trial.stderr` never surface through the CLI (tests 2 info). | **Adopted, cheaply:** a stand-in mode appends a fixed 0076 category line to `<slot>/trial.stderr` and the test asserts it on the row's failure. | One mode, one assertion. |
| Cold refusal arms of the scripts (tests 3 info); no private HOME/TMPDIR for the brokkr-mode worker (independent 6). | **Named residuals, no change.** The scripts are outside the measured denominator; the agent's own environment is built by the runner and the wrapper identically in both modes. | None. |
| The transcript half of #151 (guarantees 6 medium). | **Text corrected.** Structural records land now; value-bearing records wait for the operator's acceptance of 0116 §4 (0045 §8). The README no longer says the operator's run closes the row. | None. |

## Brokkr contract: facts the implementer establishes, uses or files

brokkr is not reachable from the design seats, and the pinned binary
(`.brokkr/recipes/self/tooling.json`: 0.12.0, source `a9893b4` plus four
patches, staged at `.brokkr/state/bin/brokkr` by the recipe guide) was not
reachable from the implement seat in round 1 and may not be in round 2.
Every brokkr-dependent behaviour below is therefore a **contract
requirement** with (a) what to check against brokkr#766 and its sub-issues
(brokkr#757–#765, #467, #462, #430), (b) what ships regardless, and (c) what
is filed on feedback-loop-ai/brokkr and named in the PR if absent. Nothing is
worked around inside Swarmail; an **unobserved seam fails closed with a typed
reason** rather than running a guess.

| # | Requirement | Ships regardless | If absent or unobserved |
|---|---|---|---|
| F1 | A `brokkr driver <family>` run as a descendant of an exec seat records its tokens (input, output, cached), turns and tool rows against that seat in the journal, and says through which channel (stdout relay or a named environment variable). **Four seams** sever every candidate channel today: (1) `trial.sh` runs the trial under `env -i PATH LANG LC_ALL SWARMAIL_BIN`; (2) the runner builds the agent's environment afresh (`RuntimeDirs`/`CallerEnv`: PATH plus the EVAL inputs); (3) `wrappers::through_driver` calls `env_clear`; (4) `through_driver` routes the driver's stdout and stderr to `/dev/null` (0075 §1). | The seams named here, in 0116 §6 and in `evals/wrappers.md`; one test proving a sentinel variable in the matrix's environment reaches neither the trial nor the agent. No forwarding. | File (issue text in the PR). Forwarding the observed channel is a **follow-up change** that touches 0075 §1 at the named seams and lands only under 0116 §6 as accepted by the operator. Until then every via-brokkr group's tokens are `null` with a typed reason, and this delivery does not measure tokens per success. |
| F2 | The driver accepts the goal the wrapper already supplies (stdin bytes for Claude and Codex, one literal argument for dsh) and an explicit CLI executable, and documents the invocation form (`brokkr driver <family> -- …` or otherwise) and the flags it adds. | The wrapper's argv assembly for `DRIVER <family> -- <argv>`, proven inline byte for byte against the direct launch. **`--via-brokkr` refuses with `driver_contract_unknown`** at the trial (before launch, no row) and at the wrapper (after `driver_invalid` validation, before any private read). No stand-in driver. | File. The refusal lifts in the follow-up that implements the observed invocation; when it lifts, a via-brokkr argv under a direct profile and the reverse refuse (`via_brokkr_profile_mismatch`, live now for the second arm). |
| F3 | A per-run machine-readable query (`brokkr export` is documented in the recipe guide; the exact shape is to be observed) carries seat results with their `inputs`, token counts, turns, tool rows and the deadline-kill event. | The typed journal-facts v1 reader and golden tests; `evals/scripts/journal-facts.sh` with the one assumed export shape isolated in `EXTRACT`. | File. The script **exits 2 naming the failure class and writes no facts file** when `brokkr export` fails, the export is not JSON, the version is unreadable or the shape does not match; a shape that matches but carries no trial yields `journal_missing`. `report` without `--journal` is byte-identical to today's. |
| F4 | A two-seat recipe with its own `policy.json` and no intake, design, implement, verify, review or ship phase compiles, runs and finishes without a commit, with `class: work` on an exec driver. | `bundle.json` and `policy.json` as specified; `brokkr compile --bundle .brokkr/recipes/eval` in CI's pinned-bundle loop (the only place it runs before merge). | File. The recipe keeps the shape and the matrix's brokkr mode is proven with the stand-in worker only. |
| F5 | The trial seat's hands reach loopback, the candidate's `swarmail` and `swarmail-evals` executables, and the slot directory under the candidate's `.brokkr/state/eval/` read-write; the score seat has workspace hands read-write with no network and writes exactly `<slot>/score.json`. **A paid trial additionally reads** the operator's private selection file, the three credential files it names and the three CLI executables with their install trees, and needs the network; none of those paths is bound today. | `hands` as the recipe guide documents: `kind: workspace`, `binds: []`, `network: true` on `trial` and `false` on `score`. `matrix --brokkr` refuses `--artifacts-dir` outside the current directory before any spawn (`brokkr_artifacts_unreachable`). | **Paid agents under `matrix --brokkr` are unsupported** until the F2/F1/F5 follow-up declares the observed binds; the PR says so and the direct `matrix` stays the paid route. A paid agent tried anyway fails closed at slot 1 through the wrapper's existing typed refusals (`wrapper_config_*`, `wrapper_executable_unavailable`). File the first unreachable path. |
| F6 | Whether the exec box shares the host's `/tmp` with the trial seat. | Nothing changes either way. | Record the answer in `evals/README.md` (admission posture), not an issue. |
| F7 | A seat killed by the run deadline is distinguishable in the journal from a seat that returned. | `deadline_killed` is a typed fact (`present` true/false or `unavailable`). The trial deadline bound in brokkr mode makes brokkr's timer a backstop only. | File. The matrix already yields `harness_error`, never `gave_up`, for a worker that returns no row. |

## The design (normative)

### The recipe `.brokkr/recipes/eval/`

- `bundle.json`: name `eval`, `policy: policy.json`, no `extends`,
  description naming the interim path and `unconfined`. Two seats.
  - `trial`: `class: work`, driver `["{brokkr}","driver","exec","--","bash","./scripts/trial.sh","{prompt_file}"]`,
    results `["ran","harness_error"]`, `limits: {max_attempts: 1, timeout_seconds: 1800}`,
    hands `kind: workspace`, `binds: []`, `network: true` (F5).
  - `score`: `class: gate`, driver `["{brokkr}","driver","exec","--","bash","./scripts/score.sh","{prompt_file}"]`,
    results `["pass","fail","unscorable"]`, `limits: {max_attempts: 1, timeout_seconds: 600}`,
    hands `kind: workspace`, `binds: []`, `network: false`. The seat writes
    exactly `<slot>/score.json`; its hands are not read-only.
- `policy.json` (`forge.phase-machine/v2`): phases `trial, score, done, stop`;
  initial `trial`; terminal `done, stop`. Rules: `trial ran → score`;
  `trial harness_error → stop` (hard); `score pass → done`; `score fail →
  done` (a failed trial is data); `score unscorable → stop` (hard). No
  retry rule anywhere: a retried trial is a different experiment.
- **Prompt parsing (revision 2), both scripts.** The engine renders the
  feature text as `Feature: <text>` and the result path as an indented line
  of its own (observed in this run's chief prompt, 2026-11-02;
  `design-check.sh` documents the same). Each line is therefore trimmed of
  leading and trailing whitespace and `\r` exactly as `verify-seat.sh` and
  `design-check.sh` trim; the result path is the last trimmed line matching
  `/*.json`; the request is taken from the last trimmed line that contains
  the token `swarmail-eval-request: /` followed by a path ending in `.json`,
  by stripping everything through the token. Both the bare form and the
  `Feature: `-prefixed form parse. The request path is then validated as
  before (absolute, regular file, no symlink, ≤ 64 KiB, `jq`).
- `scripts/trial.sh`: refuses (result `harness_error`, `inputs: {}`) when
  the result path or request is missing, the request is not a regular file,
  is larger than 64 KiB, fails `jq`, or lacks `schema_version: 1`,
  `trial_id`, `slot`, `trials_root`, `evals_bin`, `swarmail_bin`, `args`
  (array of strings). Otherwise runs
  `env -i PATH=/usr/bin:/bin LANG=C LC_ALL=C SWARMAIL_BIN="$swarmail_bin" "$evals_bin" trial "${args[@]}"`
  (seam 1 of F1) with stdout to `<slot>/row.json` (exclusive create) and
  stderr to `<slot>/trial.stderr` (bounded to 64 KiB by `head -c`), writes
  the exit code to `<slot>/exit`, and reports
  `{"result":"ran","inputs":{"trial_id":"<id>"},"notes":"…"}` when the exit
  is 0, 1 or 2 and `row.json` is exactly one line; otherwise
  `harness_error`. The script never reads or prints the row's content.
- `scripts/score.sh`: same prompt parsing; runs
  `"$evals_bin" check --artifacts-dir "<trials_root>/<trial_id>"` (both
  from the request), writes `<slot>/score.json` as
  `{"schema_version":1,"trial_id":"…","outcome":"pass|fail|harness_error","exit":N}`
  **before** reporting, and reports `pass` (exit 0), `fail` (exit 1) or
  `unscorable` (anything else). A malformed request or an unusable slot
  refuses `unscorable` without writing `score.json`; the matrix then sees
  `score_missing`. `check --artifacts-dir` validates the bound leaf and
  re-scores the app's retained state; the agent's account is not an input
  and the gate reads nothing the agent could have written after the trial
  froze.
- Both scripts: `set -u`, `jq` renders every result (refusing with a fixed
  `harness_error`/`unscorable` text without it), results are written to a
  sibling temporary and renamed, and no secret, path or child byte is
  echoed into a result's `notes`.

### The wrapper flag `--via-brokkr DRIVER` (revision 2: fail closed)

`agent-claude|agent-codex|agent-dsh --model M --config P --via-brokkr /abs/driver`:

1. validates `DRIVER` as before: an absolute path to a regular executable,
   anything else refuses `driver_invalid`;
2. then **refuses `driver_contract_unknown`**, before the EVAL inputs are
   read, before the selection or any credential file is opened and before
   anything is spawned. The refusal is unconditional: there is no switch,
   digest, flag or environment variable that distinguishes a "real" driver
   from a test one (that would be a production seam).

`through_driver` (argv assembly `DRIVER <family> -- <program> <args…>`, the
direct launch's environment keys and values, working directory, stdout and
stderr routing) stays in the code and is proven by inline tests that inspect
the assembled `Command` (`get_program`, `get_args`, `get_envs`,
`get_current_dir`): the argv after `--` equals the direct launch's argv byte
for byte for all three families. It is dead at runtime until the follow-up
that implements the observed F2 invocation lifts the refusal, and the
`#[cfg(test)]`-only reach to it is permitted by 0020 §4 (a seam, not a
flag). The three `*_via_brokkr` selectors, their `ToolSurface` rows,
`--wrapper-profile` and `agents.json` acceptance stay as round 1 specified:
they label rows once the refusal lifts.

**Trial-level refusals, live now.** Before launching any agent, `trial`
inspects the public agent argv (the arguments after `--`):

- if it contains the literal `--via-brokkr`, the trial refuses
  `driver_contract_unknown` (exit 2, fixed category on stderr, no row); the
  matrix records that slot as `harness_error` and stops admission;
- if `--wrapper-profile` (or `agents.json`'s `wrapper_profile`) is a
  `*_via_brokkr` selector and the argv does not contain `--via-brokkr`, or
  the reverse once the first refusal lifts, the trial refuses
  `via_brokkr_profile_mismatch` the same way.

Why at the trial and not only at the wrapper: a wrapper that exits before
acting is scored as the task's idle class (`gave_up`) with its exit status
on the row, which would put a fake model failure into measurement data.

### `matrix --brokkr BIN`

With `--brokkr /abs/brokkr`, `--artifacts-dir ROOT` becomes required
(`brokkr_requires_artifacts`), the current directory must contain
`.brokkr/recipes/eval/bundle.json` (`brokkr_realm_invalid`), **`ROOT` must
resolve inside the current directory** (`brokkr_artifacts_unreachable`,
revision 2: the trial seat's `kind: workspace` hands reach nothing else, so
an outside root would burn a slot after the engine started), and the
deadline must not exceed 1 200 000 ms (`deadline_exceeds_seat`: the trial's
own deadline plus the runner's fixed startup and shutdown budgets and the
300 s slack stay under the `trial` seat's 1800 s). Every refusal is row-less
exit 2. For each slot, in the existing task → agent → repetition order and
under the existing admission, the matrix:

1. creates `ROOT/<matrix_id>/slots/<trial_id>/` (private dir) and writes
   `request.json` there: `{"schema_version":1,"trial_id":…,"slot":…,"trials_root":"ROOT/<matrix_id>/trials","evals_bin":<current_exe>,"swarmail_bin":<resolved SWARMAIL_BIN>,"args":[…]}`
   where `args` is exactly the argument list (after the `trial` subcommand)
   `trial_args` builds for both workers;
2. spawns `BIN run --recipe eval --feature "swarmail-eval-request: <abs request.json>"`
   in its own process group with the existing signal forwarding, bounded
   drain and five-second EOF bound (the worker's stdout is brokkr's, not the
   row; it is drained and discarded). **The envelope in brokkr mode is
   `trial deadline + 300 000 ms + 600 000 ms`** (the direct envelope plus the
   score seat's budget), a pure function with an inline test, so
   `trial_envelope_expired` here can only mean the engine or the gate hung
   past its own seat timeout;
3. after the worker exits, reads `row.json` (through `results::read`, 64 KiB
   bound), `exit` and `score.json` from the slot, and accepts the slot only
   when the row matches the planned cell and worker context exactly as
   today, `exit` pairs with the row's outcome exactly as `matching_trial_row`
   requires, and `score.json` is present, one line, well-formed and its
   `outcome` equals the row's outcome. A missing or invalid `row.json` or
   `exit` is `trial_row_missing`; a `score.json` that is absent, unreadable,
   malformed or not one line is **`score_missing`**; one that is present and
   disagrees is **`score_mismatch`**. Each yields the slot's `harness_error`
   row, stops admission and writes the refusal suffix (0076 §3). The slot's
   `trial.stderr` is scanned for the same fixed 0076 categories the direct
   worker's stderr is;
4. commits the row through the existing `MatrixEvidence` path (the trial
   inside the seat retained the leaf under `trials/<trial_id>`).

Cardinality, the committed prefix, the active row, the zero-time suffix,
the 2 > 1 > 0 exit precedence, `concurrency: 1` and sequential admission are
unchanged. The default worker (no `--brokkr`) is byte-for-byte today's.

### Journal facts v1 (revision 2: every fact typed) and `report --journal`

`evals/scripts/journal-facts.sh BROKKR DB OUT TRIAL_ID...` queries the
pinned brokkr (F3) and writes `OUT` in this closed shape, read strictly
(`deny_unknown_fields`, no defaults) by `evals/src/journal.rs`. Facts v1 was
never released, so the shape changes in place; there is no v2.

```json
{"schema_version": 1,
 "source": {"brokkr_version": "0.12.0", "binary_sha256": "…", "query": "brokkr export …"},
 "trials": [
   {"trial_id": "…", "run_id": "…", "seat": "trial",
    "tokens": {"status": "present", "value": {"input": 1200, "output": 300, "cached": 4000}},
    "turns": {"status": "present", "value": 7},
    "wall_time_ms": {"status": "present", "value": 18400},
    "deadline_killed": {"status": "present", "value": false},
    "tool_calls": {"status": "present", "value": [
      {"ordinal": 1, "tool": "mcp__swarmail__wait", "outcome": "ok",
       "argument_bytes": 61, "argument_sha256": "…", "result_bytes": 2048, "result_sha256": "…"}]}},
   {"trial_id": "…", "run_id": "…", "seat": "trial",
    "tokens": {"status": "present", "value": {"input": 900, "output": 120, "cached": 0}},
    "turns": {"status": "present", "value": 3},
    "wall_time_ms": {"status": "unavailable", "reason": "journal_has_no_accounting"},
    "deadline_killed": {"status": "unavailable", "reason": "journal_has_no_accounting"},
    "tool_calls": {"status": "unavailable", "reason": "journal_has_no_accounting"}}]}
```

`outcome` is one of `ok`, `error`, `invalid_argument`. A present empty
`tool_calls` array is a real fact ("zero calls"); an absent, null or
malformed one is `unavailable`. `deadline_killed` is `present` only for a
JSON boolean in the export. Reasons are the closed set `journal_missing` (no
run carries the trial id), `journal_ambiguous` (more than one does),
`journal_has_no_accounting` (the run exists, this fact does not),
`builtin_agent` (the report derives it for oracle and null rows, which
launch no driver). `accounted()` is "any fact present"; the non-empty-tool-
list heuristic is removed. A trial id present in the rows and absent from
the file is `journal_missing`; a facts entry whose trial id is in no row
refuses the input (`journal_unmatched_trial`); two entries with one trial id
refuse (`journal_duplicate_trial`).

`journal-facts.sh` fails **hard and typed**: `brokkr export` failing, a
non-JSON export, an unreadable version or an `EXTRACT` shape mismatch exit 2
with the class on stderr and write no `OUT`; `jq`, a SHA-256 tool, an
absolute `BROKKR`/`OUT` and a fresh `OUT` are preconditions. Only a matching
shape that carries no run for a requested trial yields `journal_missing`
(by absence from `OUT`).

`report --input ROWS --journal FACTS` adds to each group a `journal` object
and leaves every existing field byte-identical:

- `joined`, `scored`, `excluded_harness_errors`, `missing: {reason: count}`;
- `tokens`: for `input`, `output`, `cached` and `total` (input + output;
  cached beside, never added), the nearest-rank median, p95 and max over
  joined scored trials, plus `sum`;
- `tokens_per_success`: `sum of total tokens / passes`, or `null` with
  `tokens_per_success_reason` `no_passes` or `incomplete_join` when any
  scored trial in the group lacks token facts;
- `turns`, `seat_wall_time_ms`: median, p95, max, or `null`;
- `tool_calls`: `{total, invalid_argument, by_tool: {name: count}, trials}`,
  or **`null` with `tool_calls_reason: incomplete_join`** when any scored
  joined trial's `tool_calls` is unavailable (never a total that silently
  omits trials); `deadline_killed`: a count, or **`null` with
  `deadline_killed_reason: incomplete_join`** under the same rule;
- `runs: {trial_id: run_id}` for the accounted trials.

Without `--journal` the output is byte-identical to today's.

### Retained tool-call records: `report … --evidence-dir DIR`

With `--evidence-dir DIR` (requires `--journal`), the report writes
`DIR/<trial_id>/tool-calls.json` for every joined trial whose `tool_calls`
fact is present, through `artifacts::resolve_root`, `ensure_root`,
`create_private_dir` and `publish` (exclusive 0700 directory, exclusive 0600
file, no clobber): the trial's tool calls with the tool name bounded to 128
bytes of printable ASCII (anything else is replaced by its SHA-256), under a
64 KiB cap; an oversize record is truncated at a whole-call boundary with
`"truncated": true` and `"retained": N` beside the report's full total. **No
argument or result bytes are ever copied**; the record is harness-authored
finite facts (names, classes, lengths, digests), which is what accepted 0075
§4 permits. The report's `journal` object gains
`evidence: {trial_id: {path, length, sha256} | {absent: reason}}`. A trial
whose `tool_calls` fact is unavailable gets **no file** and
`absent: journal_has_no_accounting`; a publication failure records
`absent: evidence_write_failed` for that trial and the report still
completes. An empty file is never written as evidence. The value-bearing
mode (argument values of the Swarmail MCP and `eval_http` tools,
value-redacted and fail-closed) is specified in 0116 §4 and not implemented
here.

### Documentation

`.brokkr/recipes/README.md`'s `eval` section: what the two seats do, the
engine's prompt rendering the scripts parse, the request contract, the
matrix command, the facts script, the held-PR rule, and that paid agents
under `matrix --brokkr` wait for the F2/F1/F5 follow-up. `evals/README.md`:
the brokkr-mode command, the facts and evidence commands, the admission
posture (F6), and the corrected closure sentence: *this delivery lands the
machinery; tokens per success are measured only after the accounting
channel is observed and forwarded under 0116 §6; structural tool-call
records land now and value-bearing records wait for 0116 §4's acceptance.*
`evals/wrappers.md`: `--via-brokkr` (refuses `driver_contract_unknown`
until F2 is observed), the three selectors, the four F1 seams and the full
paid read set (F5). `docs/decisions/README.md` carries the proposed 0116
row. `docs/evals/baseline-2026-11-01.md` is history and does not change.

## Acceptance Criteria

Criteria marked **(operator)** are owned by the operator and lie outside the
run's required work; the implement seat reports them as such, never as met.
Every other criterion is proven offline, without the engine, by the named
test.

- **AC1 One command, one row per slot.** `swarmail-evals matrix --brokkr BIN
  --tasks T… --agents A --trials N --artifacts-dir ROOT` writes exactly
  `tasks × agents × trials` schema-2 rows, each matching its planned cell,
  with no row field added or removed; admission, order, suffix and exit
  precedence are today's. **Proof:** `evals/tests/report.rs` matrix test with
  the stand-in worker `tests/data/brokkr-standin.sh` (it parses `--feature`,
  reads the request, synthesizes each seat's prompt **in the engine's
  rendered shape** and runs the real `trial.sh` and `score.sh` in policy
  order): 2 tasks × {oracle, null} × 2 trials = 8 rows, 4 pass, 4 fail; two
  sequential oracle trials of one task both pass with distinct seeds.
- **AC2 The worker's death is a row, never a gap.** A stand-in worker that
  exits without `row.json`, that writes a row for another trial id, that
  writes `exit` disagreeing with the row, whose `score.json` disagrees with
  the row, **that never writes `score.json`**, or **that leaves a fixed 0076
  category line in `trial.stderr`**, yields that slot's `harness_error` row
  (`trial_row_missing`, `trial_context_mismatch`, `score_mismatch`,
  `score_missing`, or the category, on stderr and as the row's failure),
  stops admission, and writes zero-time refusal rows for every later slot;
  the exit is 2. A worker killed by SIGKILL mid-trial yields
  `harness_error`, never `gave_up`. **Proof:** the same test file, one case
  per arm.
- **AC3 The recipe is real and compiles where a binary exists.**
  `.brokkr/recipes/eval/bundle.json` and `policy.json` are as specified;
  `brokkr compile --bundle .brokkr/recipes/eval` is in CI's pinned-bundle
  loop, which is the only place it runs before merge (the verify seat
  compiles only `self`; `.brokkr/recipes/self/` is not edited);
  `tests/delivery_recipes.rs` pins: two seats, both exec drivers, `score` is
  `class: gate` with `network: false`, `trial` has `network: true`, both
  have `binds: []`, no seat has `max_attempts` above 1, the policy has
  exactly the five rules above and no self-loop, and `eval` is on the shelf
  list. **Proof:** that test with one-fault mutants; CI's loop output on the
  PR. The implement seat records the compile as **not run** without the
  binary.
- **AC4 The two scripts behave without the engine, on the engine's
  rendering.** `evals/tests/recipe.rs` runs `bash
  .brokkr/recipes/eval/scripts/trial.sh PROMPT` and `score.sh PROMPT`
  directly with prompts rendered as the engine renders them (`Feature:
  swarmail-eval-request: /abs/request.json`, the result path indented, lines
  ending `\r\n` in one case) and with the bare form: oracle request →
  `row.json` one pass row, `exit` 0, result `ran` with `inputs.trial_id`;
  null request → fail row, `exit` 1, `ran`; a prompt without the request
  token, a request over 64 KiB, a request missing `args`, a relative
  `evals_bin` → `harness_error` and no `row.json`; `score.sh` on the oracle
  leaf → `pass` and `score.json.outcome` `pass`; on the null leaf → `fail`;
  on a leaf with a truncated `state.json` → `unscorable` **with
  `score.json` written**; on a malformed request → `unscorable` with no
  `score.json`; no result `notes` contains a path under ROOT or any byte of
  the agent's stderr. The `Feature:`-prefixed case is written first and
  observed red against the round-1 scripts (recorded in
  `implementation.md`).
- **AC5 Controls through the recipe path.** Through `matrix --brokkr` with
  the stand-in worker over all 16 catalogue tasks: oracle 16/16 pass; null
  0/16 pass with only `gave_up` or `wait_timeout`. **Proof:**
  `evals/tests/proof.rs`'s brokkr-mode arm beside the existing direct arm.
- **AC6 The join is typed and cannot be dropped silently.** `report --input
  ROWS --journal FACTS` on the golden fixture (8 rows, 3 agents) yields
  hand-computed `tokens` quantiles, `sum`, `tokens_per_success`, `turns`,
  `tool_calls.by_tool`, `invalid_argument`, `deadline_killed`, `runs`; one
  group has a trial with `journal_missing` and shows `tokens_per_success:
  null` with `incomplete_join`; **one group has a trial with tokens present
  and `tool_calls` unavailable and shows `tool_calls: null` and
  `deadline_killed: null` with `incomplete_join`** while its token figures
  are numbers; the oracle group shows `builtin_agent` for every trial and no
  token figure; a facts file naming a trial id absent from the rows refuses
  with `journal_unmatched_trial`; duplicate trial ids refuse; a facts file
  with an untyped `tool_calls: []` or `deadline_killed: false` refuses
  (`journal_invalid`); no numeric field anywhere is `0` or `false` for an
  unavailable fact. Without `--journal` the report equals today's byte for
  byte. **Proof:** `evals/tests/report.rs` golden tests and inline tests in
  `journal.rs`, including `a_dropped_join_is_red`.
- **AC7 Evidence is bounded, structural and never carries bytes.** With
  `--evidence-dir`, a facts fixture whose tool rows' arguments and results
  are built from the fixture admin secret and a planted token (`sentinel-…`)
  yields `tool-calls.json` files none of which, nor the report, contains
  either byte sequence; every file is 0600 under a 0700 directory; a
  5 000-call trial is truncated at a call boundary with `truncated: true`
  and `retained` under 64 KiB; the report's `evidence` names each file with
  its length and SHA-256; a trial whose `tool_calls` fact is unavailable has
  **no file** and `absent: journal_has_no_accounting`; a pre-existing target
  records `absent: evidence_write_failed` without touching it. **Proof:**
  `evals/tests/report.rs`.
- **AC8 Via-brokkr fails closed; assembly is proven inline.**
  `agent-<family> --via-brokkr DRIVER` with `DRIVER` relative, missing or
  non-executable refuses `driver_invalid`; with a valid `DRIVER` it refuses
  `driver_contract_unknown` before any private file is read and before
  anything is spawned (the private selection and credential files in the
  test are unreadable sentinels that would refuse differently if opened).
  Inline tests in `wrappers.rs` prove `through_driver`'s `Command` for all
  three families: program `DRIVER`, argv `<family> -- <direct argv…>` byte
  for byte, the same environment keys and values, the same working
  directory. A `trial` whose agent argv contains `--via-brokkr` refuses
  `driver_contract_unknown` (exit 2, no row, no agent launched); a `trial`
  declaring a `*_via_brokkr` profile without `--via-brokkr` refuses
  `via_brokkr_profile_mismatch` the same way; under `matrix --brokkr` such a
  slot is `harness_error` and stops admission. A direct launch's argv is
  unchanged. `wrappers_launch/standin-driver.py` is removed. **Proof:**
  `evals/tests/wrappers.rs`, `evals/tests/wrappers_launch/mod.rs`, inline
  `wrappers.rs` and `runner.rs`/`main.rs` tests.
- **AC9 Deadlines nest.** `matrix --brokkr` refuses `--deadline-ms` above
  1 200 000 with `deadline_exceeds_seat` and no rows; the brokkr-mode
  envelope is `deadline + 300 000 + 600 000` ms (inline test on the pure
  function; the direct envelope is unchanged); the `trial` seat's
  `timeout_seconds` is 1800 and `score`'s is 600 in `bundle.json` (pinned by
  `tests/delivery_recipes.rs`).
- **AC10 Unconfined, everywhere.** Every row's `confinement` is
  `unconfined`; the report's note says so; the bundle description and 0116
  say so. **Proof:** assertion on recipe-path rows in AC1's test.
- **AC11 Sequential (accepted 0076).** No concurrency option changes;
  `--concurrency` other than 1 still refuses; the issue's "two concurrent
  trials" criterion is **not** implemented here and the PR says isolation is
  #221's and concurrency is ruled out by 0076. **Proof:** existing refusal
  tests pass unchanged.
- **AC12 Brokkr facts F1–F7 filed (operator).** The PR lists each fact with
  exactly one disposition: observed from the pinned binary (command and
  output), tracked under a named brokkr#766 sub-issue, or filed (issue
  number). The implement seat reads brokkr#766 and #757–#765, #467, #462,
  #430 when its hands allow; when they do not, the PR body carries the issue
  texts verbatim, says "drafted, not filed" in its first paragraph, and
  **filing them (or attaching them to the sub-issue that covers them) and
  recording the numbers is a precondition of merging, owned by the
  operator**. No Swarmail code works around an absent fact; the typed
  refusals and `null` reasons are the only degradation.
- **AC13 No new dependency; row and artifact schemas unchanged.**
  `Cargo.toml`, `Cargo.lock`, `evals/Cargo.toml`, `evals/Cargo.lock` and
  `deny.toml` are unchanged; `TrialResult`, `Metadata`, `MatrixPlan`,
  `MatrixIndex` have no changed field; `WrapperProfile` and its `ToolSurface`
  rows gain exactly the three `*_via_brokkr` entries.
- **AC14 Gates, all `--locked`, all clean**, as listed in the plan's
  quickstart. `brokkr compile --bundle .brokkr/recipes/eval` runs in CI;
  the implement seat reports it as **not run** when the binary is
  unreachable, never as passed.
- **AC15 Delivery paper.** Commits unsigned, nothing pushed, merged,
  published or accepted; 0116 stays proposed with its index row. The PR
  says, in its first paragraph: it touches `.brokkr/` and `.github/` and so
  merges only on the operator's approval by name (0056's 2026-11-01 hold
  list); which brokkr issues were filed, or that they are drafted and filing
  is a merge precondition (AC12); that this delivery does **not** measure
  tokens per success and why (F1's four seams; the forwarding follow-up
  under 0116 §6); that paid agents under `matrix --brokkr` are unsupported
  until the F2/F1/F5 follow-up; that the first `brokkr run --recipe eval`
  with the engine (oracle, offline, free) and the first authenticated
  via-brokkr smoke per family are the operator's and are the observations
  that pin F1, F2 and F5; that AC16 needs the paid run after the follow-up.
- **AC16 Paid measurement (operator; after the follow-up).** Real `claude`,
  `codex` and `dsh` trials through `matrix --brokkr` with the observed
  driver invocation and the forwarded accounting channel; the facts file
  from the real journal shows tokens, turns and tool calls present;
  `tokens_per_success` is a number for each paid group; `tool-calls.json`
  exists per trial. This closes the #151 "Not measured" rows. It cannot
  happen on this delivery's tree; it needs the F2/F1/F5 follow-up first.
- **AC17 The seams are named and proven (revision 2).** One test sets a
  sentinel environment variable in the matrix's environment and proves it
  reaches neither `swarmail-evals trial` under `trial.sh` nor the agent
  under the wrapper; `evals/wrappers.md`, 0116 §6 and this spec name the
  four seams identically. **Proof:** `evals/tests/recipe.rs` (seam 1
  through the real `trial.sh`) and the existing wrapper environment tests
  extended with the sentinel (seams 2–4).

## Residuals named for the operator

- **Tokens per success is not measured by this delivery.** F1's channel is
  severed at four named seams; forwarding is a follow-up under 0116 §6 that
  touches accepted 0075 §1 and waits for the observed channel and the
  operator's acceptance.
- **Via-brokkr trials refuse until F2 is observed.** The refusal is typed
  and pre-launch; no measurement data is polluted.
- **Paid agents under `matrix --brokkr` are unsupported** until the binds
  F5 lists are observed and declared; the direct `matrix` is the paid route.
- **Unconfined.** A hostile agent can write into its own trial's slot
  (`row.json`, `score.json`) before the trial reaps it; the row is
  cross-checked against the gate and the bound leaf, which narrows but does
  not close this. #221 and brokkr's broker own confinement.
- **Admission inside a private `/tmp`** (F6), as above.
- **Value-bearing evidence** waits for 0116 §4's acceptance; the transcript
  half of #151 is only structurally closed until then.
- **Cold refusal arms of the two scripts** (no `jq`, unreadable size,
  symlinked request) have no named test; the scripts are outside the
  measured denominator.
- **The brokkr-mode worker** gets no private HOME/TMPDIR/XDG of its own; the
  agent's environment is built identically in both modes by the runner and
  the wrapper.
