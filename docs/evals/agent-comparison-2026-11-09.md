# Agent-suite comparison, 2026-11-09: Swarmail, Mailpit, MailHog and MailSlurper

This is the paid comparison asked for by #163 under accepted
[0129](../decisions/0129-agent-suite-comparison.md). The operator confirmed
it on 2026-11-08 ("the plan for #163 is confirmed, go ahead") with 240
sessions and no GLM. It ran on ripper from 02:09 to 03:14 +02:00 on
2026-11-09, against main `2e5c289`, under product id
`agent-suite-comparison-2026-11-07` (`--authorized-product`).

**Headline: the suite does not separate the four mail servers.** 236 of 240
sessions passed:

- Swarmail 60/60;
- MailHog 60/60;
- Mailpit 59/60;
- MailSlurper 57/60.

All four failures are one model (codex-luna) on one task
(`digest-lookalike`). Swarmail against MailSlurper, the widest gap, is not
significant (Fisher exact, two-sided, p = 0.24). **Swarmail's sessions were
the slowest**: median 16.9 s, against 13.2 s on MailSlurper and 13.3 s on
MailHog. The cost of these sessions is unavailable (see [Cost](#cost)).

## What ran

| | |
|---|---|
| Targets | `swarmail-mcp` (Swarmail's native MCP), `mailpit-mcp`, `mailhog-mcp`, `mailslurper-mcp` (each competitor through its thin adapter behind the runner's bridge, 0129 §4–§5). One fresh instance per trial |
| Tasks | the three hard-tier tasks: `otp-backlog`, `recipient-crowd`, `digest-lookalike` |
| Models | claude-opus (`claude-opus-5-5`), claude-sonnet (`claude-sonnet-5-5`), codex-sol (`gpt-6.1-sol`), codex-luna (`gpt-6-luna`) |
| Trials | 5 per target × task × model, 240 slots, every slot scored (no harness error) |
| Order | one matrix per target, run in turn (Swarmail, Mailpit, MailHog, MailSlurper), so each target's slots are consecutive in time, and model or network drift across the hour falls between targets, not within them |
| Limits | concurrency 1 (0076); the bwrap sandbox with a private `/tmp` |
| Controls | the free oracle and null rows on all four targets on `2e5c289` before any paid slot: 120 rows in [`agent-controls-2026-11-08-main/`](agent-controls-2026-11-08-main/), with `comparison-check` passing every prerequisite. Main's CI on `2e5c289` is the receipt ([run 42527328769](https://github.com/feedback-loop-ai/swarmail/actions/runs/42527328769)) |
| Harness | `swarmail-evals` 0.1.0 built at `2e5c289`. The trial provenance records `source_dirty` true, because the uncommitted controls directory sat in the worktree; 0129 allows that, since it is an evidence path |
| CLIs | Claude Code 2.1.319, codex-cli 0.164.0. The wrapper profile label still reads `claude_2_1_310`; the binary that ran was 2.1.319 |

Raw data is beside this file:

- [`rows.jsonl`](agent-comparison-2026-11-09/rows.jsonl): all 240 schema-3 comparison rows as run;
- [`cite.json`](agent-comparison-2026-11-09/cite.json): `swarmail-evals comparison-cite --rows rows.jsonl`, which lists every (target, model, task) cell with its denominator.

The rows carry no argv, credential, transcript or private path.

## Success by target and model

Passes over attempts. Each cell is 15 trials (three tasks × five).

| Target | claude-opus | claude-sonnet | codex-sol | codex-luna | **Total** |
|---|---|---|---|---|---|
| Swarmail | 15/15 | 15/15 | 15/15 | 15/15 | **60/60** |
| Mailpit | 15/15 | 15/15 | 15/15 | **14/15** | **59/60** |
| MailHog | 15/15 | 15/15 | 15/15 | 15/15 | **60/60** |
| MailSlurper | 15/15 | 15/15 | 15/15 | **12/15** | **57/60** |

By task, over all targets and models: `otp-backlog` 80/80,
`recipient-crowd` 80/80, `digest-lookalike` 76/80.

## The four failures

All four are codex-luna on `digest-lookalike`. The agent exited 0 each time,
and the scorer found the task undone.

| Target | Attempt | Class | Wall time |
|---|---|---|---|
| Mailpit | 2 | `gave_up` | 18.3 s |
| MailSlurper | 2 | `gave_up` | 25.0 s |
| MailSlurper | 4 | `tool_misuse` | 23.2 s |
| MailSlurper | 5 | `gave_up` | 25.7 s |

On Swarmail and MailHog, codex-luna passed `digest-lookalike` 5/5. Its 2/5 on
MailSlurper against 5/5 on Swarmail is not significant at five trials
(Fisher exact, p = 0.17), so it is recorded as a loss for MailSlurper's
adapter in this run, not as a property of MailSlurper.

## Wall time

Wall time per trial, over each target's 60 trials:

| Target | Median | p95 | Max | Sum |
|---|---|---|---|---|
| MailSlurper | 13.2 s | 21.8 s | 25.7 s | 14.1 min |
| MailHog | 13.3 s | 22.3 s | 23.9 s | 14.3 min |
| Mailpit | 15.7 s | 23.3 s | 24.7 s | 15.9 min |
| Swarmail | 16.9 s | 23.8 s | 36.2 s | 16.5 min |

Swarmail is the slowest by median and has the longest single trial. This
run does not say why: the rows record no turn or tool-call counts, and no
usage. Swarmail offers its full native tool set of 18 tools. The adapters
offer 4 (Mailpit), 3 (MailHog) and 4 (MailSlurper), per the contracts in
[`agent-controls-2026-11-08-main/contracts/`](agent-controls-2026-11-08-main/contracts/).
A larger tool list is one candidate cause. Nothing here measures it.

## Cost

Unavailable, not zero (0091 §7, 0129 §10,
[`agent-comparison-cost.json`](agent-comparison-cost.json)). The sessions
ran on subscription logins with no per-call price, and the wrappers keep no
vendor output, so no usage was captured.

## What this means

- **The hard tier is saturated across mail servers, as it was across models**
  ([hard-2026-11-03.md](hard-2026-11-03.md), 74/75). Every model finishes
  every task on every server almost every time. This run cannot rank the
  servers on agent success, and it makes no parity or superiority claim
  either way.
- **Swarmail lost on wall time.** This is the one consistent difference in
  the run, and it went against Swarmail. It belongs next to any agent claim
  in the README.
- **A comparison that discriminates needs a harder or longer task**, where
  waiting, volume or lookalike mail stresses the server rather than the
  model. Throughput, wait latency and the cost to operate are measured
  separately (#349).

## Reproduce

From a checkout of `2e5c289`, with the three competitor executables at the
locked digests:

```
scripts/compare-mail.sh controls --output docs/evals/agent-controls-<date>-main \
  --product docs/evals/agent-comparison.json --bin mailpit=… --bin mailhog=… --bin mailslurper-source-build=…
scripts/compare-mail.sh measure --output OUT --config AGENTS --authorized-product agent-suite-comparison-2026-11-07 \
  --baseline docs/evals/baseline-2026-11-01.md --contracts docs/evals/agent-controls-<date>-main/contracts \
  --controls-dir docs/evals/agent-controls-<date>-main --receipts docs/evals/agent-controls-<date>-main/receipts.json \
  --cost docs/evals/agent-comparison-cost.json --identity docs/evals/agent-controls-<date>-main/identity.json \
  --product docs/evals/agent-comparison.json --bin …
```

`AGENTS` lists the four models' wrapper commands (labels, models and
wrapper profiles as in the table above). The paid `measure` bills the
agents' accounts.
