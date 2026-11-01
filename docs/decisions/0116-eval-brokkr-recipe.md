# 0116 — The eval recipe on brokkr: interim opaque driver, journal accounting, retained tool-call records

- **Status:** proposed
- **Date:** 2026-11-02
- **Scope:** Epic #164 issue #148 (P1), delivered from main `f815d48` on the
  zero-Codex `claude-careful` recipe (0098/0099, rework per 0102).
- **Specification:** [the eval recipe on brokkr](../../specs/eval-brokkr-recipe/spec.md).
- **Records:** the operator's rulings of 2026-10-18 on #148, already cited
  by accepted [0055](0055-agent-eval-harness.md) (context) and left for
  "the next run's recipe/driver".
- **Supplements:** accepted 0055 §§6–10 and accepted
  [0076](0076-sequential-eval-trials.md) compatibly; accepted
  [0075](0075-eval-privacy.md) compatibly in §3 and by **amendment** in §4,
  which waits for the operator.
- **Preserves:** 0001–0005; accepted 0020, 0045 §8, 0050, 0056 and its
  2026-11-01 addendum, 0063, 0071, 0098, 0099, 0100, 0102.
- **Number:** provisional 0116. 0115 is held by #221's branch
  (`repair/issue221-trial-isolation`, commit `cacd60e`); intake saw no 0115
  on `main` and the task called 0115 the highest in use. Both are true, so
  the next free number across both trees is 0116. Renumber at integration
  if `main` takes it first, as 0105 and 0108 were.

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
   unscorable one or a trial that produced no row stops. The recipe reuses
   the runner's lifecycle, admission, scoring and retention unchanged.
2. **One command, one row per slot.** `swarmail-evals matrix --brokkr BIN`
   runs each slot as one `brokkr run --recipe eval`, reads the slot's row,
   exit and gate verdict, and keeps 0076's accounting: a worker that returns
   no row, a row for another cell, an exit that disagrees with the row, or a
   gate that disagrees with the row is that slot's `harness_error`, stops
   admission and writes the refusal suffix. Trials stay sequential (0076);
   isolation remains #221's; the issue's two-concurrent-trials criterion is
   not met by this recipe and is not claimed.
3. **Accounting joins by `trial_id`, never by discovery.** Rows do not
   change. `report --journal FACTS` joins a typed journal-facts file
   (produced from the pinned brokkr's export by `evals/scripts/journal-facts.sh`)
   to the rows by the `trial_id` the matrix assigned before spawn and the
   trial seat echoed in its result. A missing, ambiguous or accounting-free
   fact is a typed reason (`journal_missing`, `journal_ambiguous`,
   `journal_has_no_accounting`, `builtin_agent`), never `0`; tokens per
   success is `null` with a reason whenever any trial in a group lacks
   facts or no trial passed. This is a compatible reading of 0075 §4
   (harness-authored finite facts) and of 0055 §10 (report formulas
   unchanged, new figures added beside them).
4. **Retained tool-call records.** Per joined trial the report may write a
   bounded `tool-calls.json`: ordinal, bounded tool name, outcome class,
   invalid-argument flag, argument and result byte lengths and SHA-256
   digests; never argument or result bytes. This is compatible with 0075 §4
   and lands with the recipe. **By amendment, awaiting the operator:** a
   value-bearing mode that also retains the *argument values* of the two
   harness-owned tools (Swarmail's MCP tools and `eval_http`), value-redacted
   against the fixture admin secret and registered sentinels, fail-closed to
   absence with a typed reason when redaction cannot be proven, bounded and
   hash-bound as above, and never scored. 0075 §4 suppresses raw bodies and
   URLs at every publication sink and names transcripts an accepted loss;
   this clause reverses that for the two tools whose values are, by
   construction, the fixture's synthetic data. It is not implemented until
   accepted (0045 §8).
5. **Via-brokkr surfaces.** Rows for agents launched through the wrapper's
   `--via-brokkr DRIVER` declare the finite selectors
   `claude_2_1_310_via_brokkr`, `codex_0_164_0_via_brokkr` and
   `dsh_0_1_5_rc_1_via_brokkr`: the family's declared configuration plus
   one residual naming the driver's own flags. A via-brokkr row never
   claims the direct-launch surface.
6. **Environment exception, if brokkr needs one.** If the driver's
   accounting channel is one named environment variable, the wrapper
   forwards exactly that variable to the driver and nothing else; 0075 §1's
   cleared environment is otherwise unchanged. The variable's name is
   recorded here by dated addendum when observed.
7. **Gates and holds.** The recipe compiles in CI's pinned-bundle loop and
   the verify seat beside the six existing recipes. The PR touches
   `.brokkr/` and `.github/` and merges only on the operator's approval by
   name (0056's 2026-11-01 addendum). The first engine run of the recipe
   (oracle, offline), the first authenticated via-brokkr smoke per family and
   the paid matrix are the operator's; delivery proves the recipe with the
   oracle and null agents, the stand-in worker and the stand-in driver only.

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

Costs accepted: a second scorer run per slot; one bounded evidence file per
trial; tokens per success may remain unmeasured, typed and filed, if brokkr's
driver does not account for a nested run or its export carries no
accounting; admission is blind to other host runners if the exec box has a
private `/tmp` (the baseline's own posture); value-bearing evidence waits for
this record's acceptance.

## Delivery and acceptance ownership

The chief commits this record with the spec, plan and tasks. Implementation
adds the index row and the dated F1–F7 observations. Only the operator
accepts this proposal; the engine alone advances delivery. Commit unsigned;
never push or merge.
