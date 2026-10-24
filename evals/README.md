# swarmail-evals — the agent eval harness

Epic #164, Phase 0 run A: the fixture app (#149), the v1 task suite with
oracle solutions (#150) and the agent-agnostic trial runner that #148's
brokkr recipe will wrap. The design and its rulings are proposed decision
[0055](../docs/decisions/0055-agent-eval-harness.md); the acceptance
authority is the [spec](../specs/agent-eval-harness/spec.md).

The harness is proven able to fail before it measures anything: every task
passes with the scripted `oracle` and fails with the do-nothing `null`
(`tests/proof.rs`).

## Layout and gates

`evals/` is a **standalone crate**: its own `[workspace]` and `Cargo.lock`,
like `tools/coverage-fn-scan`. The root package, its lock, its tests and its
exact coverage gate neither build nor measure it, and the published `.crate`
does not include it. Every package in `evals/Cargo.lock` is already in the
root lock at the same version and source (`tests/boundary.rs` checks it).

Until epic #120's batch 5 lands the CI and verify-seat wiring, **evals is
proven by its own suite, run by the implement seat and reproduced by the
reviewers**. From `evals/`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked                  # everything; the proof alone:
cargo test --locked --test proof     # oracle 16/16 pass, null 16/16 fail, isolation ×2
```

The tests locate the Swarmail under test through `SWARMAIL_BIN`; when it is
unset they build the root `swarmail` binary once per test binary
(`cargo build --offline --locked --bin swarmail` in the repository root,
reading the executable path from Cargo's artifact report, so a custom
`CARGO_TARGET_DIR` is honoured).

## Commands

```bash
cargo build --offline --locked --bin swarmail
cargo build --offline --locked --manifest-path evals/Cargo.toml
export SWARMAIL_BIN="$PWD/target/debug/swarmail"   # adjust for CARGO_TARGET_DIR
E=evals/target/debug/swarmail-evals

$E tasks                                            # the v1 catalogue, schema 1
$E trial --task signup --agent oracle               # one JSON line; exit 0
$E trial --task signup --agent null                 # fail/gave_up; exit 1
$E trial --task signup --agent mine -- /abs/agent --mode email
$E trial --task otp --agent oracle --artifacts-dir /tmp/evals
$E check --task otp --state /tmp/evals/<trial_id>/state.json

cat > agents.json <<'EOF'
[{"label": "oracle", "argv": []},
 {"label": "null", "argv": []},
 {"label": "mine", "argv": ["/abs/agent", "--mode", "email"]}]
EOF
$E matrix --tasks signup,otp,stale-otp --agents agents.json --trials 5 > runs.jsonl
$E report --input runs.jsonl                        # or --input - for stdin
```

`trial` exits 0 pass, 1 fail, 2 harness error. `matrix` runs the product
sequentially, one row per requested slot with no retries, continues after
ordinary harness errors and exits 2 if any occurred, else 1 if any failed,
else 0. Once its inputs are valid, every `(task, agent, trial)` element gets
a row: if the Swarmail binary cannot be resolved, each element is a
`harness_error` row (no trial child is launched) and the exit is 2. Invalid
inputs (an unknown task, zero trials, an unreadable agents file) are refused
with exit 2 and no rows.

On matrix SIGTERM or SIGINT, the first observed signal stops admission. The
matrix forwards one SIGTERM to its active trial PID and waits for that trial
to reap its children and observe its owned process groups stopped. Trial
cleanup sends TERM for up to five seconds, then KILL and observes cessation
for up to five more seconds; matrix has no shorter competing kill timer.
Matrix exits 2 for either signal. It keeps committed rows, writes one
`harness_error` row for active work with any usable trial ID and observed
agent statuses, then writes zero-time refusal rows for every unstarted slot
in task → agent → repetition order. `report` counts these as requested
attempts; that denominator may include slots whose agents never ran. A
signal during the fallback offline Cargo build cancels and reaps that build
before the refusal suffix is written.

The matrix accepts exactly one matching schema-v1 result from a trial and
checks its exit code. It drains child output while retaining at most 64 KiB;
invalid, extra or oversized output stops admission with exit 2. Broken or
non-progressing stdout also exits 2 after cleanup; unavailable output cannot
promise complete JSONL. SIGKILL, host failure, daemonizing or hostile
commands, escaped processes and uninterruptible kernel tasks remain outside
the lifecycle guarantee. `check` exits 0 pass, 1 fail, 2 for unscorable
state. Only machine output goes to stdout; diagnostics go to stderr.

## The external agent contract

An agent is a label plus a literal argv after `--` (never a shell string);
`oracle` and `null` are reserved built-ins run through the same child
boundary. Each agent runs in its own process group and working directory
with the caller's environment plus:

| Variable | Value |
|---|---|
| `EVAL_TASK_ID` | the task id |
| `EVAL_APP_URL` | the fixture's public base URL |
| `EVAL_MCP_URL` | Swarmail's MCP endpoint, ending in `/mcp` (MCP `2025-03-26`) |
| `EVAL_IDENTITY` | the synthetic identity (an email address) |
| `EVAL_INBOX` | the agent's Swarmail inbox |
| `EVAL_GOAL` | the rendered prompt — byte-identical to the prompt file |
| `EVAL_PROMPT_FILE` | `prompt.md` in the agent's working directory |
| `EVAL_DEADLINE_MS` | the agent deadline (default 60000) |

No admin URL, admin secret or snapshot path is ever exported. Every request
to the app must carry `X-Eval-Identity: <identity>`; `GET /` on the app
lists its operations:

| Route | Does |
|---|---|
| `GET /a/{token}` | follow a link from an email |
| `POST /otp/verify {code}` | submit a 2FA code |
| `POST /otp/resend {}` | mail a new code; earlier codes stop working |
| `POST /reset/complete {token, password}` | single-use password reset |
| `POST /login {password}` | sign in |
| `GET /invite/{token}/event.ics` | the invitation's calendar file (identical to the attachment) |
| `POST /invite/accept {token, uid, start}` | accept, confirming the ICS UID and UTC start |
| `POST /support/reply {ticket, challenge, reply}` | answer a support ticket |
| `POST /deliveries/release {}` | release mail the app is holding (slow-delivery) |

The fixture's admin plane (a second loopback listener, bearer secret known
only to the runner) provisions identities, runs the scenario barriers,
freezes the app and serves the redacted snapshot and per-flow state
(`GET /admin/state/{flow}`). No route sets a success fact; no state response
carries a token, code, password, challenge or calendar capability.

## The result row

One JSON line per trial: `schema_version` (1), `trial_id`, `task`, `agent`,
`outcome` (`pass|fail|harness_error`), `failure_class` (null only on pass),
`wall_time_ms` (provisioning through cleanup, excluding the binary build),
`agent_exit_status` (the single agent's numeric code, else null — always
null for the three-agent isolation task), `agent_exits` (identity, code,
signal per agent) and `confinement: "unconfined"`. Exit statuses never decide
a goal: an agent that completes the task and then crashes passes with its
real status; `null` exits zero and fails.

## Tasks and the failure taxonomy

`swarmail-evals tasks` prints the catalogue: 15 tasks in three tiers plus
`inbox-isolation` (three agents on one Swarmail, one AUTH inbox each). Each
declares the `failure_class` it provokes and the `idle_class` (`gave_up` or
`wait_timeout`) a do-nothing agent earns. The class set is closed:
`wrong_message`, `stale_code`, `wrong_link`, `wait_timeout`, `gave_up`,
`tool_misuse`, `budget`, `harness_error`.

Scoring precedence: infrastructure failure → `harness_error`; an agent alive
at its deadline → `fail/budget`; a satisfied app-state predicate → pass
(isolation also forbids any foreign-token presentation); otherwise the first
refused app action names the class; otherwise the task's idle class. The
scorer reads only the frozen app snapshot — never a mailbox, transcript or
claim.

Infrastructure failure includes the snapshot itself: any fixture delivery
(decoys included) whose *final* status is `failed` or `parked` — at
preparation or later, say on a release — makes the trial `harness_error`
ahead of the deadline and the business check. A delivery retried from 451
to 250 is `delivered` and scores normally; `held` (unreleased) mail is an
ordinary business outcome. `check` applies the same snapshot rule, so a
retained state with a failed or parked delivery replays as `harness_error`
(exit 2). Replay reproduces only what the snapshot records: process
failures and deadlines are not in it.

## Honest limits

- **Unconfined.** Process-group ownership is lifecycle cleanup, not a
  sandbox; a child that daemonizes or leaves its group escapes. Admin
  separation stops accidents, not a hostile local process. Unix only.
- **slow-delivery** prescribes an initial MCP wait before the release; the
  app-state score proves the final confirmation, not that an arbitrary agent
  performed that first wait.
- **smtp-retry** proves the agent copes with mail that was refused (a real
  451 under 100% DATA chaos) and retried before the agent started — not
  recovery during a live outage.
- **invite-time** serves the ICS through the app's token-authorized
  download; it does not measure MCP attachment retrieval (#159).
- **support-reply** replies through the app's HTTP form with the mailed
  challenge; it does not measure Swarmail inbound reply transport (#160).
- No tokens, turns or cost here: those come from brokkr's journal when the
  #148 recipe wraps this runner. No model was run in this batch.
