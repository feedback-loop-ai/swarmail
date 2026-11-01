# The eval recipe on brokkr: interim opaque driver, journal join, retained tool-call records

Date: 2026-11-02. Status: design draft (chief's synthesis of the design
panel, run `issue-148-the-eval-recipe-on-bro-197e7def`). Issue #148, epic
#164, P1. Operator direction 2026-11-01: start now, alongside a separate
harder-tier run. Zero-Codex `claude-careful` lane (accepted 0098 and 0099;
review residuals rework under accepted 0102 and its 2026-10-31 spec-defect
addendum). Base: `main` at `f815d48`.

Decision: 0116 — [the eval recipe on brokkr](../../docs/decisions/0116-eval-brokkr-recipe.md),
status proposed. It records the operator's 2026-10-18 rulings (the harness
is built on brokkr; the interim opaque driver; every interim trial is
unconfined; brokkr#766 tracks the move to the broker), the compatible
reading of accepted 0075 this spec relies on, and the value-bearing evidence
mode that waits for the operator. Only the operator accepts it.

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

## Panel synthesis: rulings

Both positions agree on: reuse the runner's lifecycle; the trial seat is an
exec driver running `swarmail-evals`; no sqlite and no new dependency; no
raw CLI stdout or stderr is retained by Swarmail; a missing journal fact is
a typed reason, never `0`; the gate reads the app's state and never the
agent's account; trials stay sequential (0076); anything brokkr lacks is
filed on brokkr, never worked around in Swarmail. Where they differ:

| Question | Ruling | What the losing side costs |
|---|---|---|
| A separate fsynced attempt ledger before any spawn (robustness A) | **Rejected as a mechanism, adopted as a property.** `matrix` already writes exactly one row per requested slot: a worker that dies without output yields a `harness_error` row, admission stops and every unstarted slot gets a zero-time refusal row (0076 §3). In brokkr mode the same accounting rules the `brokkr run` worker. The brokkr journal is the pre-spawn record of the run itself. | None for the matrix. A lone `brokkr run --recipe eval` started by hand that dies leaves its record only in brokkr's journal; the matrix is the measurement unit and the report reads rows, so that loss is stated, not patched. |
| Bind the child run id at spawn, never discover it (robustness B) | **Adopted through the key the matrix already assigns.** `trial_id` is assigned before the worker spawns, written into the slot's request file, echoed by the trial seat in its typed result `inputs.trial_id`, and is the only join key. A journal with two runs claiming one `trial_id` joins neither (`journal_ambiguous`). No timestamp or "newest run" matching anywhere. | Robustness wanted the run id inside the row before the agent runs; the row does not change (next ruling), so the run id is surfaced by the report, not the row. |
| Extend the v2 row with token and evidence fields under a new schema (robustness) vs rows unchanged (simplicity) | **Rows unchanged.** "Extend, not replace" is met at the report's inputs: `report --journal` joins a typed facts file to the rows by `trial_id`. The published 395/400 rows stay readable as they are; the strict readers of 0063 and 0076 are untouched; no v3, no migration, no 64 KiB pressure. Intake's AC1 wording ("each row carries the brokkr run id") is corrected: the row carries `trial_id`; the report carries the run id. | A row alone no longer tells you its tokens; you need the facts file beside it. The baseline publishes both. |
| Which selector a via-brokkr row declares | **New finite selectors** `claude_2_1_310_via_brokkr`, `codex_0_164_0_via_brokkr`, `dsh_0_1_5_rc_1_via_brokkr` (simplicity): the driver adds flags the family profile does not enumerate, so such a row must not claim the direct-launch surface. Same revision, basis `declared_configuration`, the family's restrictions, plus one residual naming the driver. | Three more enum variants and profile rows in `provenance.rs`; a binary before this change refuses the new rows, as the surface contract already says for historical rows. |
| The scoring gate | **A separate exec seat with three exit codes** (robustness D), implemented by the existing `check --artifacts-dir <leaf>`, which validates the bound evidence (metadata, result, state, diagnostics) and re-scores: 0 pass, 1 fail, 2 unscorable. **And the matrix cross-checks it** against the row (`score.json` must agree with `row.json`, else the slot is `harness_error` with `score_mismatch` and admission stops). Simplicity's "replay equality is already proven" is true, but a gate whose verdict nobody reads is decoration. | About fifteen lines in the matrix and a second scorer run per slot, on a frozen snapshot that is already consistent (the fixture freezes, then snapshots through its admin API; there is no database copy and no WAL, so robustness's WAL concern does not arise). |
| Evidence: journal only (simplicity) vs a bounded, hashed, value-redacted side file, fail-closed (robustness) | **Both, in two steps.** Now: a per-trial `tool-calls.json` derived from the journal facts: ordinal, bounded tool name, outcome class, invalid-argument flag, argument and result byte lengths and SHA-256 digests. **No argument or result bytes.** It is harness-authored finite facts, compatible with accepted 0075 §4, bound by the `artifacts.rs` triple, 64 KiB cap, truncation typed, written through the exclusive-create primitives. Later, under proposed 0116 only: argument values of the two harness-owned tools (Swarmail MCP and `eval_http`), value-redacted and fail-closed to absence. That second step contradicts 0075 §4's "suppress raw bodies, URLs" and **is not implemented in this run** (0045 §8). | Robustness loses value-bearing evidence until the operator accepts 0116. Simplicity loses "nothing to write": one bounded file per trial. The baseline's open question (did the model act on a wrong message or give up?) is answered by tool names, counts and the retained `state.json`. |
| Admission inside the exec box (robustness 9) | **Accepted residual, named.** If brokkr's exec box gives the trial seat a private `/tmp`, the trial's advisory lock is blind to other cooperating runners on the host, which is exactly the posture the operator ran the #151 baseline under (bwrap with a private `/tmp`). The matrix stays sequential, so no two of its own trials overlap. No bypass flag and no parent-held reservation is added: both would weaken 0076 §1–2 for the shared-`/tmp` case. | On a host where the box shares `/tmp`, nothing is lost. On a host where it does not, the operator runs no other trial while a matrix runs, as for the baseline. Brokkr fact F6 records which it is. |
| Crash tests, join tests, unavailable tests, gate-failure tests, sentinel tests (robustness) | **Adopted**, with stand-ins: the matrix's worker stand-in plays `brokkr run` and runs the two real recipe scripts, so the recipe path is proven offline without the engine. | None. |
| Cert, cap, chaos, webhook failure modes (robustness table) | **Out of scope, stated.** The eval has no TLS (loopback HTTP), the harness never changes the server's cap or prunes, the eval runs no chaos, and the fixture has no webhook sink. Nothing here touches 0001–0005. | None. |
| Decision number 0115 (intake) vs 0116 (simplicity) | **0116**, for the collision reason above. | A renumber at integration if `main` takes 0116 first. |

## Brokkr contract: facts the implementer establishes, uses or files

brokkr is not reachable from the design seats, and the pinned binary
(`.brokkr/recipes/self/tooling.json`: 0.12.0, source `a9893b4` plus four
patches, staged at `.brokkr/state/bin/brokkr` by the recipe guide) may not be
reachable from the implement seat either. Every brokkr-dependent behaviour
below is therefore stated as a **contract requirement** with (a) what to
check against brokkr#766 and its sub-issues (brokkr#757–#765, #467, #462,
#430), (b) what ships regardless, and (c) what is filed on
feedback-loop-ai/brokkr and named in the PR if absent. Nothing is worked
around inside Swarmail.

| # | Requirement | Ships regardless | If absent |
|---|---|---|---|
| F1 | A `brokkr driver <family>` run as a descendant of an exec seat records its tokens (input, output, cached), turns and tool rows against that seat in the journal, and says through which channel (stdout relay or a named environment variable). | The `--via-brokkr DRIVER` wrapper flag, proven with the stand-in driver. | File. Via-brokkr rows join as `journal_has_no_accounting`; tokens per success waits on brokkr; the PR names the issue. If the channel is one environment variable, the wrapper forwards exactly that name and 0116 records the exception to 0075 §1. |
| F2 | The driver accepts the goal the wrapper already supplies (stdin bytes for Claude and Codex, one literal argument for dsh) and an explicit CLI executable, and documents the flags it adds. | The wrapper's argv assembly for `DRIVER <family> -- <argv>`. | File. The wrapper refuses `--via-brokkr` with the real driver by a typed category (`driver_contract_unknown`); the stand-in driver still proves the path. |
| F3 | A per-run machine-readable query (`brokkr export` is documented in the recipe guide; the exact shape is to be observed) carries seat results with their `inputs`, token counts, turns, tool rows and the deadline-kill event. | The typed journal-facts v1 reader and golden tests; `evals/scripts/journal-facts.sh` written against the observed export. | File. The script emits every trial as `journal_has_no_accounting` with the query named in `source`. |
| F4 | A two-seat recipe with its own `policy.json` and no intake, design, implement, verify, review or ship phase compiles, runs and finishes without a commit, with `class: work` on an exec driver. | `bundle.json` and `policy.json` as specified; `brokkr compile --bundle .brokkr/recipes/eval` in CI's loop and the verify list. | File. The recipe keeps the shape and the matrix's brokkr mode is proven with the stand-in worker only. |
| F5 | The trial seat's hands reach loopback, the candidate's `swarmail` and `swarmail-evals` executables, the slot directory under the candidate's `.brokkr/state/eval/` read-write, the operator's private selection file read-only (paid runs only), and the network for paid runs; the score seat reaches the slot read-only with no network. | The bundle's `hands` declaration as the recipe guide documents binds. | File the first unreachable path. |
| F6 | Whether the exec box shares the host's `/tmp` with the trial seat. | Nothing changes either way. | Record the answer in `evals/README.md` (admission posture), not an issue. |
| F7 | A seat killed by the run deadline is distinguishable in the journal from a seat that returned. | The trial deadline bound in brokkr mode (below) makes brokkr's timer a backstop only. | File. The matrix already yields `harness_error`, never `gave_up`, for a worker that returns no row. |

## The design (normative)

### The recipe `.brokkr/recipes/eval/`

- `bundle.json`: name `eval`, `policy: policy.json`, no `extends`,
  description naming the interim path and `unconfined`. Two seats.
  - `trial`: `class: work`, driver `["{brokkr}","driver","exec","--","bash","./scripts/trial.sh","{prompt_file}"]`,
    results `["ran","harness_error"]`, `limits: {max_attempts: 1, timeout_seconds: 1800}`,
    hands as F5 with `network: true`.
  - `score`: `class: gate`, driver `["{brokkr}","driver","exec","--","bash","./scripts/score.sh","{prompt_file}"]`,
    results `["pass","fail","unscorable"]`, `limits: {max_attempts: 1, timeout_seconds: 600}`,
    hands with `network: false`.
- `policy.json` (`forge.phase-machine/v2`): phases `trial, score, done, stop`;
  initial `trial`; terminal `done, stop`. Rules: `trial ran → score`;
  `trial harness_error → stop` (hard); `score pass → done`; `score fail →
  done` (a failed trial is data); `score unscorable → stop` (hard). No
  retry rule anywhere: a retried trial is a different experiment.
- `scripts/trial.sh`: reads the prompt file line by line exactly as
  `verify-seat.sh` and `design-check.sh` do, taking the last absolute
  `/….json` line as the result path and the last
  `swarmail-eval-request: /abs/path.json` line as the request. Refuses
  (result `harness_error`, `inputs: {}`) when either is missing, the request
  is not a regular file, is larger than 64 KiB, fails `jq`, or lacks
  `schema_version: 1`, `trial_id`, `slot`, `evals_bin`, `swarmail_bin`,
  `args` (array of strings). Otherwise runs
  `SWARMAIL_BIN="$swarmail_bin" "$evals_bin" trial "${args[@]}"` with stdout
  to `<slot>/row.json` (exclusive create) and stderr to `<slot>/trial.stderr`
  (bounded to 64 KiB by `head -c`), writes the exit code to `<slot>/exit`,
  and reports `{"result":"ran","inputs":{"trial_id":"<id>"},"notes":"…"}`
  when the exit is 0, 1 or 2 and `row.json` is exactly one line; otherwise
  `harness_error`. The script never reads or prints the row's content.
- `scripts/score.sh`: same prompt parsing; runs
  `"$evals_bin" check --artifacts-dir "<trials_root>/<trial_id>"` (both
  from the request), writes `<slot>/score.json` as
  `{"schema_version":1,"trial_id":"…","outcome":"pass|fail|harness_error","exit":N}`
  and reports `pass` (exit 0), `fail` (exit 1) or `unscorable` (anything
  else). `check --artifacts-dir` validates the bound leaf and re-scores the
  app's retained state; the agent's account is not an input and the gate
  reads nothing the agent could have written after the trial froze.
- Both scripts: `set -u`, `jq` renders every result (refusing with a fixed
  `harness_error`/`unscorable` text without it), results are written to a
  sibling temporary and renamed, and no secret, path or child byte is
  echoed into a result's `notes`.

### The wrapper flag `--via-brokkr DRIVER`

`agent-claude|agent-codex|agent-dsh --model M --config P --via-brokkr /abs/driver`
builds exactly the argv, MCP config, private HOME and cleared environment it
builds today, then execs `DRIVER <family> -- <argv>` instead of the selected
executable. `DRIVER` must be an absolute path to a regular executable;
anything else refuses before launch (`driver_invalid`). The goal reaches the
driver exactly as it reaches the CLI today. Rows for such launches declare
the matching `*_via_brokkr` selector; `--wrapper-profile` and
`agents.json`'s `wrapper_profile` accept the three new names; model
validation per family is unchanged. Nothing about the direct launch changes.

### `matrix --brokkr BIN`

With `--brokkr /abs/brokkr`, `--artifacts-dir ROOT` becomes required
(`brokkr_requires_artifacts`), the current directory must contain
`.brokkr/recipes/eval/bundle.json` (`brokkr_realm_invalid`), and the
deadline must not exceed 1 200 000 ms (`deadline_exceeds_seat`, so the
trial's own deadline plus the runner's fixed startup and shutdown budgets
stay under the seat's 1800 s). For each slot, in the existing task → agent →
repetition order and under the existing admission, the matrix:

1. creates `ROOT/<matrix_id>/slots/<trial_id>/` (private dir) and writes
   `request.json` there: `{"schema_version":1,"trial_id":…,"slot":…,"trials_root":"ROOT/<matrix_id>/trials","evals_bin":<current_exe>,"swarmail_bin":<resolved SWARMAIL_BIN>,"args":[…]}`
   where `args` is exactly the argument list `run_child_assigned` builds
   today for `swarmail-evals trial` (task, agent, deadline, seed, fixture
   time, worker context, `--artifacts-dir ROOT/<matrix_id>/trials`, model,
   config id, public argv, wrapper profile, `--` and the agent argv);
2. spawns `BIN run --recipe eval --feature "swarmail-eval-request: <abs request.json>"`
   in its own process group with the existing deadline envelope, signal
   forwarding, bounded drain and five-second EOF bound (the worker's stdout
   is brokkr's, not the row; it is drained and discarded);
3. after the worker exits, reads `row.json` (through `results::read`, 64 KiB
   bound), `exit` and `score.json` from the slot, and accepts the slot only
   when the row matches the planned cell and worker context exactly as
   today, `exit` pairs with the row's outcome exactly as `matching_trial_row`
   requires, and `score.json.outcome` equals the row's outcome; a missing or
   invalid `row.json` or `exit` is `trial_row_missing`, a disagreeing
   `score.json` is `score_mismatch`, and each yields the slot's
   `harness_error` row, stops admission and writes the refusal suffix (0076 §3);
4. commits the row through the existing `MatrixEvidence` path (the trial
   inside the seat retained the leaf under `trials/<trial_id>`).

Cardinality, the committed prefix, the active row, the zero-time suffix,
the 2 > 1 > 0 exit precedence, `concurrency: 1` and sequential admission are
unchanged. The default worker (no `--brokkr`) is byte-for-byte today's.

### Journal facts v1 and `report --journal`

`evals/scripts/journal-facts.sh BROKKR DB OUT TRIAL_ID...` queries the
pinned brokkr (F3) and writes `OUT` in this closed shape, read strictly
(`deny_unknown_fields`) by the new module `evals/src/journal.rs`:

```json
{"schema_version": 1,
 "source": {"brokkr_version": "0.12.0", "binary_sha256": "…", "query": "brokkr export …"},
 "trials": [
   {"trial_id": "…", "run_id": "…", "seat": "trial",
    "tokens": {"status": "present", "value": {"input": 1200, "output": 300, "cached": 4000}},
    "turns": {"status": "present", "value": 7},
    "wall_time_ms": {"status": "present", "value": 18400},
    "deadline_killed": false,
    "tool_calls": [
      {"ordinal": 1, "tool": "mcp__swarmail__wait", "outcome": "ok",
       "argument_bytes": 61, "argument_sha256": "…", "result_bytes": 2048, "result_sha256": "…"}]},
   {"trial_id": "…", "run_id": "…", "seat": "trial",
    "tokens": {"status": "unavailable", "reason": "journal_has_no_accounting"},
    "turns": {"status": "unavailable", "reason": "journal_has_no_accounting"},
    "wall_time_ms": {"status": "unavailable", "reason": "journal_has_no_accounting"},
    "deadline_killed": false, "tool_calls": []}]}
```

`outcome` is one of `ok`, `error`, `invalid_argument`. Reasons are the closed
set `journal_missing` (no run carries the trial id), `journal_ambiguous`
(more than one does), `journal_has_no_accounting` (the run exists, the fact
does not), `builtin_agent` (the report derives it for oracle and null rows,
which launch no driver). A trial id present in the rows and absent from the
file is `journal_missing`; a facts entry whose trial id is in no row refuses
the input (`journal_unmatched_trial`), so a facts file from another matrix
cannot be joined by mistake. Two entries with one trial id refuse
(`journal_duplicate_trial`).

`report --input ROWS --journal FACTS` adds to each group a `journal` object
and leaves every existing field byte-identical:

- `joined`, `missing: {reason: count}`;
- `tokens`: for `input`, `output`, `cached` and `total`, the nearest-rank
  median, p95 and max (the existing `nearest_rank`) over joined
  non-harness-error trials, plus `sum`;
- `tokens_per_success`: `sum of total tokens over joined trials / passes`,
  or `null` with `reason` `no_passes` or `incomplete_join` when any trial in
  the group lacks token facts, so a partial join can never bias it;
- `turns`: median, p95, max; `tool_calls`: `by_tool: {name: count}`,
  `invalid_argument: count`, `total`; `deadline_killed: count`;
- `runs: {trial_id: run_id}` for the joined trials.

Without `--journal` the output is byte-identical to today's.

### Retained tool-call records: `report … --evidence-dir DIR`

With `--evidence-dir DIR` (requires `--journal`), the report writes
`DIR/<trial_id>/tool-calls.json` for every joined trial through
`artifacts::resolve_root`, `ensure_root`, `create_private_dir` and `publish`
(exclusive 0700 directory, exclusive 0600 file, no clobber): the trial's
`tool_calls` array as in the facts file, with the tool name bounded to 128
bytes of printable ASCII (anything else is replaced by its SHA-256), under
a 64 KiB cap; an oversize record is truncated at a whole-call boundary with
`"truncated": true`. **No argument or result bytes are ever copied**; the
record is harness-authored finite facts (names, classes, lengths, digests),
which is what accepted 0075 §4 permits. The report's `journal` object gains
`evidence: {trial_id: {path, length, sha256} | {absent: reason}}`. Writing
nothing is never silent: a publication failure records
`absent: evidence_write_failed` for that trial and the report still
completes. The value-bearing mode (argument values of the Swarmail MCP and
`eval_http` tools, value-redacted and fail-closed) is specified in 0116 and
not implemented here.

### Documentation

`.brokkr/recipes/README.md` gains an `eval` section (what the two seats do,
the request contract, the matrix command, the facts script, the held-PR
rule); `evals/README.md` gains the brokkr-mode command, the facts and
evidence commands, the admission posture (F6) and the sentence that the
#151 "Not measured" items are closed by the operator's run of this recipe;
`evals/wrappers.md` documents `--via-brokkr` and the three selectors;
`docs/decisions/README.md` gains the proposed 0116 row (implementation task,
not the chief's commit). `docs/evals/baseline-2026-11-01.md` is history and
does not change.

## Acceptance Criteria

- **AC1 One command, one row per slot.** `swarmail-evals matrix --brokkr BIN
  --tasks T… --agents A --trials N --artifacts-dir ROOT` writes exactly
  `tasks × agents × trials` schema-2 rows, each matching its planned cell,
  with no row field added or removed; admission, order, suffix and exit
  precedence are today's. **Proof:** `evals/tests/report.rs` matrix test with
  a stand-in worker program standing in for `brokkr run` (it parses
  `--feature`, reads the request, runs the real `trial.sh` and `score.sh`
  with synthesized prompt files in policy order): 2 tasks × {oracle, null} ×
  2 trials = 8 rows, 4 pass, 4 fail; two sequential oracle trials of one
  task both pass with distinct seeds.
- **AC2 The worker's death is a row, never a gap.** A stand-in worker that
  exits without `row.json`, that writes a row for another trial id, that
  writes `exit` disagreeing with the row, or whose `score.json` disagrees
  with the row, yields that slot's `harness_error` row (`trial_row_missing`,
  `trial_context_mismatch`, or `score_mismatch` on stderr), stops admission,
  and writes zero-time refusal rows for every later slot; the exit is 2. A
  worker killed by SIGKILL mid-trial yields `harness_error`, never
  `gave_up`. **Proof:** the same test file, one case per arm.
- **AC3 The recipe is real and compiles.** `.brokkr/recipes/eval/bundle.json`
  and `policy.json` are as specified; `brokkr compile --bundle
  .brokkr/recipes/eval` is in CI's pinned-bundle loop and in the verify list;
  `tests/delivery_recipes.rs` pins: two seats, both exec drivers, `score` is
  `class: gate` with `network: false`, no seat has `max_attempts` above 1,
  the policy has exactly the five rules above and no self-loop, and `eval`
  is on the shelf list. **Proof:** that test; CI's loop output.
- **AC4 The two scripts behave without the engine.** `evals/tests/recipe.rs`
  runs `bash .brokkr/recipes/eval/scripts/trial.sh PROMPT` and `score.sh
  PROMPT` directly: oracle request → `row.json` one pass row, `exit` 0,
  result `ran` with `inputs.trial_id`; null request → fail row, `exit` 1,
  `ran`; a prompt without the request line, a request over 64 KiB, a request
  missing `args`, a relative `evals_bin` → `harness_error` and no `row.json`;
  `score.sh` on the oracle leaf → `pass` and `score.json.outcome` `pass`; on
  the null leaf → `fail`; on a leaf with a truncated `state.json` →
  `unscorable`; no result `notes` contains a path under ROOT or any byte of
  the agent's stderr.
- **AC5 Controls through the recipe path.** Through `matrix --brokkr` with
  the stand-in worker over all 16 catalogue tasks: oracle 16/16 pass; null
  0/16 pass with only `gave_up` or `wait_timeout`. **Proof:** `evals/tests/proof.rs`
  gains the brokkr-mode arm beside the existing direct arm.
- **AC6 The join is typed and cannot be dropped silently.** `report --input
  ROWS --journal FACTS` on the golden fixture (8 rows, 3 agents) yields
  hand-computed `tokens` quantiles, `sum`, `tokens_per_success`, `turns`,
  `tool_calls.by_tool`, `invalid_argument`, `runs`; one group has a trial
  with `journal_missing` and shows `tokens_per_success: null, reason:
  incomplete_join`; the oracle group shows `builtin_agent` for every trial
  and no token figure; a facts file naming a trial id absent from the rows
  refuses with `journal_unmatched_trial`; duplicate trial ids refuse; no
  numeric field anywhere is `0` for an unavailable fact. Without
  `--journal` the report equals today's byte for byte. **Proof:**
  `evals/tests/report.rs` golden tests and inline tests in `journal.rs`.
- **AC7 Evidence is bounded, structural and never carries bytes.** With
  `--evidence-dir`, a facts fixture whose tool rows' arguments and results
  are built from the fixture admin secret and a planted token (`sentinel-…`)
  yields `tool-calls.json` files none of which, nor the report, contains
  either byte sequence; every file is 0600 under a 0700 directory; a
  5 000-call trial is truncated at a call boundary with `truncated: true`
  under 64 KiB; the report's `evidence` names each file with its length and
  SHA-256, and a pre-existing target records `absent: evidence_write_failed`
  without touching it. **Proof:** `evals/tests/report.rs`.
- **AC8 Via-brokkr launches.** `agent-<family> --via-brokkr DRIVER` execs
  `DRIVER <family> -- <argv>` where `<argv>` equals the direct launch's
  argv byte for byte, with the same environment keys and MCP config;
  `DRIVER` relative, missing or non-executable refuses before launch; a
  trial of `signup` with the stand-in driver that execs the built-in oracle
  passes and its row declares `claude_2_1_310_via_brokkr`; the same with a
  do-nothing stand-in fails `gave_up`; a direct launch's argv is unchanged.
  **Proof:** `evals/tests/wrappers.rs` with `wrappers_launch/standin-driver.py`.
- **AC9 Deadlines nest.** `matrix --brokkr` refuses `--deadline-ms` above
  1 200 000 with `deadline_exceeds_seat` and no rows; the `trial` seat's
  `timeout_seconds` is 1800 and `score`'s is 600 in `bundle.json`
  (pinned by `tests/delivery_recipes.rs`).
- **AC10 Unconfined, everywhere.** Every row's `confinement` is
  `unconfined`; the report's note says so; the bundle description and 0116
  say so. **Proof:** assertion on recipe-path rows in AC1's test.
- **AC11 Sequential (accepted 0076).** No concurrency option changes;
  `--concurrency` other than 1 still refuses; the issue's "two concurrent
  trials" criterion is **not** implemented here and the PR says isolation is
  #221's and concurrency is ruled out by 0076. **Proof:** existing refusal
  tests pass unchanged.
- **AC12 Brokkr facts F1–F7.** The PR lists each fact with: observed from
  the pinned binary, or already tracked under brokkr#766 (which sub-issue),
  or filed (issue number). No Swarmail code works around an absent one; the
  typed `journal_has_no_accounting` path is the only degradation.
- **AC13 No new dependency; row and artifact schemas unchanged.**
  `Cargo.toml`, `Cargo.lock`, `evals/Cargo.toml`, `evals/Cargo.lock` and
  `deny.toml` are unchanged; `TrialResult`, `Metadata`, `MatrixPlan`,
  `MatrixIndex` have no changed field; `WrapperProfile` and its `ToolSurface`
  rows gain exactly the three `*_via_brokkr` entries.
- **AC14 Gates, all `--locked`, all clean**, as listed in the plan's
  quickstart, including `brokkr compile --bundle .brokkr/recipes/eval` where
  the binary is present (the verify seat and CI; the implement seat reports
  it as not run if the binary is unreachable, never as passed).
- **AC15 Delivery paper.** Commits unsigned, nothing pushed, merged,
  published or accepted; 0116 stays proposed with its index row added. The
  PR says: it touches `.brokkr/` and `.github/` and so merges only on the
  operator's approval by name (0056's 2026-11-01 hold list); which brokkr
  issues it filed; that the first `brokkr run --recipe eval` with the engine
  (oracle, offline, free) and the first authenticated via-brokkr smoke per
  family are the operator's; that AC16 needs the paid run.
- **AC16 Operator's paid run only; not in delivery.** Real `claude`, `codex`
  and `dsh` trials through `matrix --brokkr` with the real driver; the facts
  file from the real journal shows tokens and turns; `tokens_per_success` is
  a number for each paid group; `tool-calls.json` exists per trial. This is
  what closes the #151 "Not measured" rows.

## Residuals named for the operator

- **Unconfined.** A hostile agent can write into its own trial's slot
  (`row.json`, `score.json`) before the trial reaps it; the row is
  cross-checked against the gate and the bound leaf, which narrows but does
  not close this. #221 and brokkr's broker own confinement.
- **Admission inside a private `/tmp`** (F6), as above.
- **Value-bearing evidence** waits for 0116's acceptance.
- **Tokens per success may stay unmeasured** if F1 or F3 is absent; it is
  then filed, typed and visible, never estimated.
