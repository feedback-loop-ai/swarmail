# Baseline, 2026-11-01: the DashScope GLM cell, run 2026-11-16

The sixth model of the #151 paid baseline. The
[2026-11-01 baseline](baseline-2026-11-01.md) ran five of the six approved
declarations and deferred `dashscope/glm-5.3` because its quota was exhausted.
The operator approved running it on 2026-11-16 ("we have some quota"). It ran
on ripper from 16:39 to 17:18 +02:00 on 2026-11-16, with **the same
executables as the 2026-11-01 run**: harness and target built from main
`a4f552d` (#300), the same clean checkout.

**Headline: 80 of 80 paid trials passed.** Every one of the 16 tasks passed
5/5. No row failed and no row is a harness error. With this cell the
baseline's matrix is complete: six models × 16 tasks × 5 trials, **475 of 480
passed**, and the five failures are still the 2026-11-01 run's five codex-luna
`gave_up` trials. The suite stays saturated at this difficulty. See
[What this changes](#what-this-changes).

## What ran

| | |
|---|---|
| Tasks | all 16 of the v2 catalogue, catalogue sha256 `6068b687…`, the same bytes as the 2026-11-01 run (every row's per-task `definition_sha256` equals that run's) |
| Model | `dashscope/glm-5.3`, label `glm-dashscope`, wrapper profile `dsh_0_1_5_rc_1`: the approved declaration in [`evals/agents-baseline.json`](../../evals/agents-baseline.json), unchanged (`public_argv: false`) |
| Trials | 5 per task, 80 paid slots, one fresh fixture seed per trial (80 distinct seeds, none shared with the 2026-11-01 rows) |
| Controls | none fresh; see [Deviations](#deviations-from-the-run-plan) |
| Limits | concurrency 1; per-trial agent deadline 600000 ms, as the run plan fixes |
| Harness | `swarmail-evals` 0.1.0 at `a4f552d`, clean tree, executable sha256 `87d9728e…`, on every row |
| Target | `swarmail` built from the same checkout, executable sha256 `ad538afa…`, on every row |
| CLI | dsh, version not recorded; see [Deviations](#deviations-from-the-run-plan) |

Both digests are the 2026-11-01 run's, on all 80 rows, and the binaries in
the run checkout still hash to them. Rows record schema 2, scorer
`business-score-v2`, the same resolved target settings as the 2026-11-01 rows
and `confinement: unconfined`.

The raw rows and the generated report are kept beside this file:
[`paid.jsonl`](baseline-2026-11-01-glm-dashscope/paid.jsonl) (80 rows,
byte-identical to the run's output, sha256 `23ded079…`) and
[`report.json`](baseline-2026-11-01-glm-dashscope/report.json)
(`swarmail-evals report --input paid.jsonl`). The report was generated with
the run's own harness binary (`87d9728e…`, `a4f552d`). The harness built from
main `ee31cf1` produces byte-identical output, and it also reproduces the
2026-11-01 [`report.json`](baseline-2026-11-01/report.json) byte for byte.
The rows carry no argv (`command` is `unavailable: private_argv`), no
credential and no private path.

The cell has its own directory instead of rows appended to
[`baseline-2026-11-01/`](baseline-2026-11-01/). That page published
`paid.jsonl` as 400 rows and `report.json` as the report over them, and it
is append-only. Every other run in this directory also keeps its data beside
its own page. The run's agent declaration file is not copied: its argv names
the operator's private wrapper configuration. A one-trial signup probe ran at
16:39:06, before the cell. It passed, and it is not part of the cell or of
these rows.

## Success by task

Passes over attempts. Each cell is five trials. The other five columns are
the [2026-11-01 page's](baseline-2026-11-01.md#success-by-task-and-by-model)
published figures, which its committed report reproduces.

| Task | glm-dashscope | claude-opus | claude-sonnet | codex-sol | codex-luna | glm-spark |
|---|---|---|---|---|---|---|
| confirm-decoys | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| digest-forty | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| entity-url | 5/5 | 5/5 | 5/5 | 5/5 | **4/5** | 5/5 |
| i18n-subject | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| inbox-isolation | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| invite-time | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| magic-login | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| otp | 5/5 | 5/5 | 5/5 | 5/5 | **4/5** | 5/5 |
| reset-login | 5/5 | 5/5 | 5/5 | 5/5 | **4/5** | 5/5 |
| reset-reuse | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| sibling-addresses | 5/5 | 5/5 | 5/5 | 5/5 | **4/5** | 5/5 |
| signup | 5/5 | 5/5 | 5/5 | 5/5 | **4/5** | 5/5 |
| slow-delivery | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| smtp-retry | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| stale-otp | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| support-reply | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| **Total** | **80/80** | **80/80** | **80/80** | **80/80** | **75/80** | **80/80** |

The six totals sum to 475/480. The report's 95% interval for each 5/5 cell is
[0.57, 1.0], so, as before, no single cell separates two models and the
totals are the comparable unit.

**Against the other models.** These are two-sided Fisher exact tests over the
totals, chosen after seeing the data (post hoc):

- glm-dashscope 80/80 against claude-opus, claude-sonnet, codex-sol or
  glm-spark, each 80/80: p = 1.0. Nothing separates them.
- glm-dashscope 80/80 against codex-luna 75/80: p = 0.059. That does not
  separate at the 0.05 level.
- The five models without a failure, pooled (400/400), against codex-luna
  (75/80): p = 0.0001. That is a statement about codex-luna, chosen because
  it is the model that failed. It is not a ranking of the other five.

**GLM now has two models.** #151 asks for two models per family. On
2026-11-01 GLM had one (glm-spark). With this cell, both GLM declarations sit
at 80/80 on this tier.

Wall time per trial (the row's `wall_time_ms`), over each model's 80 trials.
The method is the one the 2026-11-01 page's figures follow: the median is the
midpoint of the 40th and 41st values, and p95 is nearest-rank. Applied to the
2026-11-01 rows, it reproduces all fifteen of that page's figures. (The
report's own nearest-rank median for glm-dashscope would be 26.6 s.) This is
not latency under a fixed load, and glm-dashscope ran 15 days after the
others, against a hosted provider:

| Model | Median | p95 | Max |
|---|---|---|---|
| claude-sonnet | 8.4 s | 14.6 s | 23.0 s |
| claude-opus | 11.9 s | 20.3 s | 32.0 s |
| codex-luna | 18.8 s | 29.8 s | 79.6 s |
| codex-sol | 21.9 s | 27.7 s | 29.2 s |
| **glm-dashscope** | **26.7 s** | **47.0 s** | **72.4 s** |
| glm-spark | 62.0 s | 154.8 s | 215.6 s |

No trial came near the 600 s deadline. Every agent exit was status 0 with no
signal. (`inbox-isolation` rows have a null `agent_exit_status` and three
`agent_exits`, one per identity, as in the 2026-11-01 rows.)

## Tokens per success

**Not measured**, for the reason the
[2026-11-01 page](baseline-2026-11-01.md#tokens-per-success) gives: the rows
carry no token, turn or cost field, and the wrappers discard raw CLI output by
design ([wrappers.md](../../evals/wrappers.md)). Nothing here estimates it.

## Failure taxonomy

| Class | Count |
|---|---|
| every class (`gave_up`, `wait_timeout`, `wrong_message`, `wrong_link`, `stale_code`, `tool_misuse`, `budget`, `harness_error`) | 0 |

There are no failures to classify and so no failure evidence to point at.
The per-trial evidence (`state.json`, `result.json`, `metadata.json` and
`diagnostics.log` for each of the 80 trials) is kept in the run's output tree
on the operator's host, under one directory per matrix id, not in this
repository.

Across the whole six-model matrix the taxonomy is unchanged from 2026-11-01:
5 `gave_up`, all codex-luna, one trial each on five tasks, and nothing in any
other class.

**Transcripts: not available**, as on 2026-11-01: the wrappers send raw CLI
output to `/dev/null` on every outcome. With no failure, no transcript was
wanted for this cell anyway.

## Deviations from the run plan

- **Run 15 days after the rest of the matrix.** The bytes are the same
  (harness, target and task definitions, verified by digest on every row), but
  the hosted model behind `dashscope/glm-5.3` is not attested by any row and
  may have changed between 2026-11-01 and 2026-11-16.
- **No fresh controls.** The 2026-11-01 run gated its paid slots on an
  oracle/null matrix. This run's driver (the 2026-11-01 driver with the model
  loop cut to `glm-dashscope`) had no control step. The 2026-11-01 controls
  ran on the same executables (all 32 control rows record `87d9728e…` and
  `ad538afa…`): 16/16 oracle pass, 16/16 null fail. They show the scorer can
  pass and fail on these binaries, but they are not a gate run before this
  cell.
- **CLI version not recorded.** The 2026-11-01 page lists dsh 0.2.1-rc.3 for
  that run. The rows record the wrapper profile, not the CLI binary, and the
  wrapper's configuration that names the executable is a private operator
  file this record does not read. The dsh on the host's `PATH` on 2026-11-17
  reports 0.2.2-rc.1, installed 2026-11-11. So the dsh this cell ran is
  unattested and may differ from glm-spark's.
- **`matrix`, under `bwrap`, task-major**, as on 2026-11-01: one
  `swarmail-evals matrix --tasks T --trials 5` per task, 16 matrices, in
  catalogue order, under the same private-root sandbox. The DashScope route
  needs no login refresh, so the driver never waited. Its log's first line
  still says "16 tasks x 5 models x 5 trials", a leftover of the five-model
  driver it was derived from. Only `glm-dashscope` ran, and the rows prove
  it: 80 rows, all with that label.
- **Ordering against #152–#160**, as on 2026-11-01: `a4f552d` already carries
  most of the epic, so this measures the improved surface.

## Not measured

- Tokens per success, for every model.
- Transcripts for any class.
- The epic's starting point (pre-#152 main).
- GLM on the hard and expert tiers. This cell is the 2026-11-01 suite's
  (16 v2 tasks on `a4f552d`). It is not a GLM cell of the
  [2026-11-07 hard and expert run](expert-2026-11-07.md): neither glm-spark
  nor glm-dashscope has run those eight tasks, so this cell says nothing
  about #325's second model at 10% or more.

## What this changes

**#151's baseline matrix is complete.** All six approved declarations ran all
16 tasks five times on main `a4f552d`, 480 slots, and 475 passed. This record
does not edit #151 or #164. The issue's other open items are not closed by
this cell: tokens per success and transcripts stay unmeasurable with today's
wrappers. On the epic's re-prioritization, the operator ruled on 2026-11-01 to
build a harder tier first (#309), and
[`expert-2026-11-07.md`](expert-2026-11-07.md) proposes the re-prioritization
from that tier.

**The reading of the 2026-11-01 page holds.** A sixth model at 80/80 adds no
failure, so the taxonomy at this difficulty is still five early exits by one
model. It still cannot rank work by the classes #152–#160 target. Five of six
models never fail here. That is more evidence that the tier is saturated, and
no evidence about the harder tiers.
