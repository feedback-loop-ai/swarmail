# Trusted baseline wrappers

Date: 2026-10-26. Status: draft design. Epic #164, issue #151 part 1.

## What and why

Give the operator one reproducible way to run the approved baseline: three
explicit trusted agent wrappers, six fixed model declarations, and one command
that proves the free controls before admitting paid trials and then reports
all paid results. This delivery prepares the experiment; it does not perform
or publish the baseline, close #151, or reprioritize the epic.

The checksum-verified issue151 packet records no prior run outcomes. Its
2026-10-26 operator ruling fixes every v2 catalogue task, the following models,
five paid repetitions and concurrency one. The current catalogue has 16 tasks:
32 control rows and 480 paid result slots. Inbox-isolation runs three agents
inside one slot, so slots are not session counts.

| Wrapper subcommand | Literal model | Public label |
|---|---|---|
| agent-claude | claude-opus-5-5 | claude-opus |
| agent-claude | claude-sonnet-5-5 | claude-sonnet |
| agent-codex | gpt-6.1-sol | codex-sol |
| agent-codex | gpt-6-luna | codex-luna |
| agent-dsh | dashscope/glm-5.3 | glm-dashscope |
| agent-dsh | spark-glm/GLM-5.3-Flash-EXL3 | glm-spark |

Implementation stays under `evals/`, including its documentation and tests.
This chief commit contains only this spec, plan and tasks. These instantiate
the existing external-agent and sequential-matrix contracts rather than amend
them; no new decision is declared. Authorities are accepted
[0055](../../docs/decisions/0055-agent-eval-harness.md), accepted
[0071](../../docs/decisions/0071-eval-trap-scoring.md), the operator direction
recorded in [intake](../../evals/issue151-intake.md), and the current
[external agent contract](../../evals/README.md#the-external-agent-contract).
Existing provenance, cleanup, privacy and admission behavior is described by
proposed [0063](../../docs/decisions/0063-eval-provenance.md),
[0073](../../docs/decisions/0073-matrix-cancellation.md),
[0075](../../docs/decisions/0075-eval-privacy.md) and
[0076](../../docs/decisions/0076-sequential-eval-trials.md); none is accepted here.
No scoring/schema/report-formula, frozen guarantee, server API, dependency,
root manifest, gate, bundle or delivery-route change is authorized.

## Panel rulings and explicit costs

Both positions support the existing executable, matrix, strict reader and
cleanup owners. Adopt simplicity's three subcommands sharing implementation
instead of three scripts or runtimes. Reject a new supervisor, provider SDK,
agent implementation, scheduler, retry/resume system or credential broker.
Cost: three CLI-specific configurations still need maintained evidence, and
sequential execution can be lengthy.

Adopt robustness's exact control-set/provenance checks, bounded app helper,
private configuration and output omission. Reject exit-code-only admission,
prompt-only identity restrictions and replacing known secret strings in raw
logs. Cost: more boundary tests and less diagnostic detail. This complements,
rather than contradicts, simplicity's small shared helper. The panel provides
no disagreement that licenses relaxing these requirements.

For the concrete report tension, choose simplicity's required paid report only;
retain the control rows and matrix artifacts without a second aggregate report.
The losing robustness suggestion costs control-summary convenience, not evidence.
Choose robustness's paid accounting validation as well as strict parsing;
otherwise a report can silently describe an incomplete product. Preserve the
existing matrix status distinction: baseline exit 0 means all paid rows pass,
1 means complete measured failures with a valid report, and 2 means an
operational failure. A report never erases exit 2.

Adopt both panels' 600000 ms per-trial agent deadline. Ten minutes allows CLI
startup and multiple tool turns beyond the fixture's 60-second default; this
is a policy allowance, not measured latency. It is below the one-hour maximum.
480 sequential slots have 80 hours of nominal agent budgets, excluding setup
and cleanup. Isolation's concurrent agents share the slot deadline. Keep the
runner's monotonic enforcement and cleanup budgets; reject a competing wrapper
kill timer. Cost: slow failures remain expensive and wall time is not capped
at exactly 80 hours.

Reject claims that private HOME alone excludes all CLI configuration, or that
fake executables prove real CLI interoperability. Missing local contracts below
remain prerequisites, not permission to ship unsupported wrappers. Cost: this
design can be drafted before those inputs exist, but mandatory wrapper usability
cannot be called implemented or verified until they are resolved.

## Wrapper and command requirements

Wrappers accept only `--model MODEL --config ABSOLUTE_PRIVATE_FILE`; no arbitrary
CLI passthrough or effort tuning. Each accepts only its two listed models.
The selected wrapper, not the harness/coordinator, reads its private config.
Credentials reach only the selected CLI through supported private files or a
narrow child-only environment. No provider credential reaches the HTTP helper.
The private locator is not a secret value or public configuration identity.

Keep the eight EVAL inputs, cleared environment, private HOME/TMPDIR/XDG roots
and literal argv. Deliver EVAL_GOAL byte-for-byte, including newlines and shell
metacharacters. Do not append tool instructions to it; use tool descriptions
or an evidenced separate instruction field. Config must register Swarmail at
EVAL_MCP_URL and one app tool, `eval_http`, bound to EVAL_APP_URL. No inherited
MCP configuration may silently add tools. This configured surface is not a
namespace or hostile-process confinement claim; rows remain unconfined.

`eval_http` exposes GET/POST, an app-relative path and optional JSON body.
It supplies X-Eval-Identity itself, including on GET /. No caller URL, headers,
admin capability, shell shortcut or answer-solving operation is accepted.
Bound requests/responses, disable redirects and proxies, and restrict the final
origin to the supplied public app endpoint. Forward fixture business responses
without changing their meaning or removing decoys. Keep provider TLS validation.

The baseline command has fixed tasks/models/repetitions/concurrency/deadline,
with only output location, private config locator and dry-run selection exposed.
Use fresh controls from this invocation, never an imported successful file.
Keep controls and paid JSONL separate. Stream every emitted row to exclusive
private files; preserve incomplete bytes on failure without repairing them.
Report all strictly parseable paid rows, including incomplete diagnostic sets,
but return 2 if full accounting, retention or matrix execution failed. Never
retry cells, filter failures, overwrite evidence or invent measurements.

## Acceptance Criteria

These are implementation proofs, not claims that this design has implemented them.

- AC1: The three wrapper subcommands and `evals/agents-baseline.json` parse with
  exactly the six table entries, matching literal argv and explicit model
  provenance. Models outside each wrapper's pair, missing/invalid EVAL inputs,
  invalid config and unsupported CLI setups refuse before provider execution.
  Probe assertions cover literal goal/model bytes and the complete environment.
- AC2: Each wrapper configures its evidenced CLI with the given MCP endpoint
  and app helper under a private runtime; no caller keys, HOME, proxy, plugins
  or unrelated MCP configuration are inherited. Tests inspect generated configs
  and selected auth fields, including refusal paths. A fixed CLI contract must
  substantiate every generated flag/config field; probes alone do not do so.
- AC3: Synthetic credential canaries stay absent from argv, tracked examples,
  labels/model/config IDs, public diagnostics, JSONL, reports and retained
  artifacts on success, invalid setup, noisy child failure and interruption.
  Private files are 0600 in 0700 runtime directories before payload; unrelated
  existing files remain unchanged. Raw CLI output is drained and discarded.
- AC4: A real loopback MCP client sees exactly one helper tool. A real loopback
  HTTP receiver observes the fixed identity on every GET/POST, including GET /.
  Invalid methods, URL escapes, identity/header overrides, redirects and size
  overruns refuse; a second listener observes no redirected request. Requests
  are bounded by the remaining invocation budget. No public network is needed.
- AC5: The plan and every trial record specify 600000 ms and concurrency 1;
  wrappers receive that deadline and remain in the trial-owned process group.
  Controlled child/cleanup handshakes prove expiry/interruption drains, waits
  and disposes private state through the existing owner. Preserve the existing
  deadline scoring, TERM/KILL budgets and cancellation envelope.
- AC6: Controls cover every catalogue task exactly once with oracle and null,
  in catalogue -> agent -> repetition order. Admission requires strict v2 rows,
  expected matrix/cell identities, unique complete coordinates, matching task,
  catalogue, agent and limits provenance, oracle pass/no class, null fail with
  the task's idle_class, and aggregate exit 1. Missing, duplicate, extra,
  malformed, foreign, wrong-class/outcome/limit rows, infra errors, signals and
  any other exit forbid paid launch. Each refusal has a zero-launch proof.
- AC7: Only admitted controls permit the exact six-model, five-repetition paid
  product. Retain all emitted bytes and existing per-trial/matrix artifacts,
  including harness errors and zero-time refused suffixes. Validate complete
  coordinates/declarations without requiring model success. A valid report is
  generated after paid exit 0/1/2 when rows parse; malformed/truncated input is
  retained and fails safely. Exit 2 wins over report success; valid measured
  failures exit 1; all-pass exits 0. Controls refusal never launches paid work.
- AC8: Dry-run prints the actual ordered tasks/models/repetitions, current
  32/480 counts, concurrency, deadline, output layout and report operation.
  It returns before credential reads, output creation, Cargo/CLI/trial launches
  or service contact. Test probes observe zero launches and no private-config
  read; dry-run and execution derive from the same plan.
- AC9: Existing real SMTP/MCP/fixture HTTP controls, trap scoring, provenance,
  retention, row accounting and sequential admission keep their proofs. The
  independent eval suite and all local house gates are run without weakening
  assertions. Current known documentation failure is recorded below, not waived.
- AC10: Eval documentation gives build/setup prerequisites, exact supported
  CLI contracts, command/dry-run forms, retention/status interpretation and
  deadline rationale. Paid execution/publication are operator-owned. New links
  never target run-local files. No paid/networked agent is used for delivery.

## Known prerequisites and ownership

The local adapters provide only partial CLI evidence. Claude's recorded 2.1.287
shape supports strict MCP configuration and native-tool removal, and refuses
managed MCP configuration. Codex's recorded 0.160.0 private CODEX_HOME still
loads project/system/managed MCP sources; it has no established general tool
allowlist. dsh 0.1.5-rc.1 is explicitly marked MCP/tool permissions unsupported.
Its two approved routes accept no effort override. The referenced adapter
measurement file and controller interface captures are absent; no harness CLI
executable is available in the workspace tool environment.

Missing precisely: versioned noninteractive goal/model and remote-MCP plus
stdio-helper registration contracts for dsh; an evidenced Codex exclusion
mechanism or exhaustive pre-provider refusal check for ambient sources; and
selective authentication/config layouts for all three CLI versions. Native-tool
selection details must support the intended app-tool surface without claiming
host confinement. Record available facts in eval documentation, refuse unknown
setups, and name unresolved inputs instead of inventing flags. An always-refusing
dsh stub does not satisfy AC1/AC2. Independent command/helper work may proceed.

The full eval suite currently fails `boundary::sequential_contracts_and_decision_links_are_honest`
on README's absent issue199 operator-rulings link. The same absent authority is
also linked outside evals; fixing only README may expose the next failure.
#243 owns the tracked authority home and external link repairs. Replace active
eval links with truthful tracked references available here, preserve assertions,
and report any remaining external-scope failure explicitly.

CI owns chart/Docker/RustSec/network/port-80 and macOS root receipts; #201 owns
macOS eval execution, #221 namespace isolation. The operator owns live provider
interoperability, paid execution, token/transcript evidence, baseline publication,
epic reprioritization, sealing and decision acceptance. #152/#153 stay unmerged
until the baseline exists; #154-#160 wait for its reprioritization. These are
ownership facts, not extra locally unprovable acceptance criteria.
