# Agent eval harness — Phase 0, run A

- Date: 2026-10-19; branch: `epic164-harness`.
- Scope: epic #164, #148's runner seam, fixture #149, task suite #150.
- Status: implemented under `evals/` (run A); the proof is `evals/tests/proof.rs`.
- Decision: [0055 — agent eval harness](../../docs/decisions/0055-agent-eval-harness.md), **accepted** (operator, 2026-10-19); amended for named traps by accepted [0071](../../docs/decisions/0071-eval-trap-scoring.md).

## Active amendment — 2026-10-25, accepted 0071

Catalogue, snapshot and trial-result versions are now **2**, with identical
field sets, task IDs/tiers and closed classes. `check` refuses v1 snapshots and
`report` refuses v1/mixed rows before aggregation, diagnostically. Historical
v1 replay requires the recorded old source/binary; never relabel, migrate or
pool experiments. Dated earlier receipts remain historical.

The exact policy in [eval-trap-scoring/spec.md](../eval-trap-scoring/spec.md)
applies to confirm-decoys, sibling-addresses, i18n-subject, stale-otp,
magic-login, entity-url and digest-forty. Rejected exact action/class/flow
records require a provisioned Agent; sibling's known Decoy foreign-token
refusal counts with any/null flow. Match the entire ordered frozen trial
before the goal branch. Structurally valid mismatches remain nonmatches.
The affected goals disclose disqualification even after successful recovery.
Application facts/HTTP remain truthful. Expiry alone and non-target tasks
retain recovery; required reset replay is positive evidence. Existing
validation, infrastructure/deadline precedence and isolation fallback stay.
Completed external try-all controls with zero raw exits, DATA 250, true goals
and actual check replay supplement oracle/null/isolation proofs.

This amendment changes scoring authority under accepted 0071 only. #194/#196
cancellation/privacy assertions remain intact with current v2 rows. #201 owns
macOS eval CI evidence, the operator owns live settings and sealing, and #221
owns namespaces; no receipt for those is claimed here.

## Why and scope

Swarmail's agent claims need a falsifiable measurement. Before measuring a
model, prove that the same runner and deterministic checks award every
scripted oracle success and every do-nothing agent failure. The subject is
today's shipped Swarmail, through real SMTP ingest and its current MCP HTTP
surface. Neither a simulated mailbox nor an agent's claim proves completion.

Deliver one standalone, unpublished Rust crate under `evals/`, with one
executable `swarmail-evals`: `fixture`, `tasks`, `agent`, `trial`, `matrix`,
`check`, and `report` subcommands. `fixture` is the small HTTP application,
not a separate package. Its own `[workspace]` and offline-generated lockfile
follow `tools/coverage-fn-scan`; every registry package name/version/source
in its dependency closure must already occur in root `Cargo.lock`.

No root manifest, lockfile, `src/`, `tests/`, `scripts/`, `bundles/`, or
`.github/` changes. The root package include list already excludes `evals/`.
Do not change shipped behavior, frozen guarantees, dependency policy or
coverage. A discovered Swarmail defect requires a reproduction in run notes
and an outstanding proof, never a workaround or source edit.

The brokkr recipe, interim brokkr driver, MCP configuration generation,
model runs, tokens, turns, costs and publication are subsequent work.
Browser automation, new Swarmail attachment/reply APIs, persistence, TLS,
webhooks and pruning benchmarks are outside this run.

## User scenarios

1. A maintainer runs `trial --task signup --agent oracle`: a fresh app sends
   mail to a fresh Swarmail; the child oracle reads MCP and confirms the
   account over HTTP; the runner scores app state and emits one pass row.
2. Replacing oracle with null leaves that account unconfirmed and emits
   `fail/gave_up`, even though null exits zero. A wrong-token agent also
   fails. This is the prerequisite to trusting any later model measurement.
3. An operator supplies a program and literal argv after `--`. It receives
   the same prompt and endpoints as oracle; matrix and report require no
   adapter-specific knowledge. No model execution is required in run A.

## Fixture requirements

The app has a public loopback HTTP listener and a separate loopback admin
listener, both bound to port zero. Admin routes require a per-trial bearer
secret supplied only to the fixture and runner. All state responses are
redacted: outcome facts, delivery receipts and rejection categories, never
usable tokens, OTPs, passwords, support challenges or calendar capabilities.
Admin separation prevents accidental access, not hostile same-user access;
every trial is explicitly unconfined. No endpoint directly sets success.

The runner provisions synthetic identities and inbox mappings on the admin
plane before launching agents. Public requests identify the synthetic
identity in the `X-Eval-Identity` header; secret presentations must also
match that identity and flow.
Mail uses AUTH PLAIN with the **inbox as username** (0004); recipient
addresses alone do not select an inbox. Mail is never seeded.

| Flow | Required behavior and outcome facts |
|---|---|
| Sign-up | An unpredictable, identity-bound confirmation link; `confirmed` only after valid presentation. |
| 2FA | Distinct OTP generations, expiry, resend immediately invalidating the preceding code; `otp_verified` and `logged_in` only for a current, unexpired code. |
| Password reset | An identity-bound single-use link sets an agent-chosen password; subsequent login validates it; replay cannot change it. Record reset use, successful post-reset login and rejected replay separately. |
| Magic login | Emailed capability establishes `logged_in` for the intended identity. |
| Invite | Real MIME ICS attachment plus an invitation link. The linked app page offers a token-authorized download of the identical ICS bytes; acceptance requires the correct event UID and UTC start time. Record accepted invite and time match. |
| Support | Email carries a per-ticket challenge. Public HTTP reply submission validates ticket, identity, challenge and nonempty reply, then records `reply_received`. This is fixture-local HTTP reply, not Swarmail inbound reply transport. |
| Digest | Exactly 40 distinct actionable links, one matching the goal's described item; decoys cannot set `digest_selected`. |
| Marketing | HTML-only body with entity-encoded HTTP URLs, including encoded query separators; only the intended decoded target sets `marketing_completed`. |

SMTP handles multiline replies, CRLF, DATA dot transparency, bounded I/O
and final acceptance. MIME includes encoded Unicode subjects, bounded lines
and a base64 ICS attachment. Record each attempt and reply; only final DATA
250 records delivery. Never retry an acknowledged message or an ambiguous
post-DATA disconnect. Retry only explicit transient refusals within a bound;
unexpected permanent failure or exhausted fixture delivery is a harness error.

OTP expiry exists in runtime behavior. Test expiry with a fixture-owned clock
seam and generated times, not sleeps or fixed calendar dates. Staleness in
the task suite uses resend/supersession. Invite dates are generated per trial.

The transient task is prepared by the runner: arm existing MCP DATA chaos
at 100%, trigger mail, observe the app's 451 receipt and parked retry, clear
chaos, release retry, observe DATA 250, then start the agent. The fixture
performs the retry; the runner only controls its scenario barrier. The app
knows SMTP, not MCP. This proves handling of retried delivery, not agent
recovery during a live outage. Chaos and shared-server isolation never mix.

The slow task uses a public, documented delivery release: mail is queued but
held; the goal instructs the agent to perform an initial bounded MCP wait,
release delivery over app HTTP, then wait again and confirm. The oracle
proves the first wait timed out before releasing mail. This causal barrier
wins over a timer that may fire before a delayed agent starts. The app-only
score proves final completion, not that an arbitrary agent performed the
first wait; this limitation is part of the task description.

## Task format and current v2 catalogue

A typed fixed catalogue, serialized by `tasks` as schema version 2, is the
task format. No arbitrary executable checks or task-loading language.
Each entry contains `id`, `tier` (`straight|traps|multi_step`),
`failure_class` (the targeted failure), `idle_class`, `goal`, `goal_template`,
`scenario`, `check` (a typed predicate), `agent_count`, and `deadline_ms`.
Unknown enum values, duplicate IDs, unknown template placeholders, invalid
deadlines and chaos with multiple agents refuse validation.

Each template includes `{app_url}`, `{mcp_url}`, `{identity}`, `{inbox}` and
`{goal}`. The common instructions state the available HTTP operations,
assigned inbox, MCP URL including `/mcp`, task goal, and prohibition on
reading harness/admin files or other identities. These are instructions,
not confinement. The rendered template is identical in the prompt file and
environment. Every catalogue entry has a scripted oracle solution.

| ID | Tier | Targeted failure | Idle class | Goal and typed success predicate |
|---|---|---|---|---|
| `signup` | straight | gave_up | gave_up | Confirm this identity; `confirmed`. |
| `otp` | straight | gave_up | gave_up | Complete 2FA; current OTP verified and logged in. |
| `magic-login` | straight | wrong_link | gave_up | Follow the magic login; logged in. |
| `stale-otp` | traps | stale_code | gave_up | Use the resent code beside the old one; current generation verified. |
| `confirm-decoys` | traps | wrong_link | gave_up | Choose Confirm beside Unsubscribe and Report; confirmed. |
| `sibling-addresses` | traps | wrong_message | gave_up | Confirm the requested sibling address; requested identity confirmed. Both look-alike messages are in the assigned inbox. |
| `entity-url` | traps | wrong_link | gave_up | Complete the marketing action using the encoded URL; marketing completed. |
| `i18n-subject` | traps | wrong_message | gave_up | Find the described Unicode-subject mail among a decoy; intended confirmation complete. |
| `slow-delivery` | traps | wait_timeout | wait_timeout | Initial wait, release, wait again, confirm; released mail accepted and account confirmed. |
| `smtp-retry` | traps | wait_timeout | wait_timeout | Confirm after transient delivery refusal; recorded 451, retry DATA 250 and confirmation. |
| `digest-forty` | traps | wrong_link | gave_up | Select the requested item among 40 links; intended item selected. |
| `reset-login` | multi_step | tool_misuse | gave_up | Reset password, then log in with it; reset used and post-reset login. |
| `reset-reuse` | multi_step | tool_misuse | gave_up | Reset, demonstrate that reuse is refused, then log in with the first new password; exactly one reset, rejected replay, login. |
| `invite-time` | multi_step | wrong_link | gave_up | Accept invite and confirm the downloaded ICS event/time; accepted and UID/time matched. |
| `support-reply` | multi_step | tool_misuse | gave_up | Reply through the fixture's HTTP form using the mailed challenge; validated reply received. |
| `inbox-isolation` | multi_step | wrong_message | gave_up | Three concurrent children each confirm their own identity; all three confirmed and no foreign-identity token presentations. |

These are 15 ordinary tasks plus one isolation task. Inside an isolation
trial there is one Swarmail/app and three distinct AUTH inboxes. Trials run
sequentially; distinct ports and identities do not isolate concurrent same-uid
processes. See the current issue-#148 AC4 replacement below.

## Scoring and failure taxonomy

The closed enum is exactly `wrong_message`, `stale_code`, `wrong_link`,
`wait_timeout`, `gave_up`, `tool_misuse`, `budget`, `harness_error`.
The scorer is a pure function of a validated versioned app-state snapshot
and the catalogue predicate. It reads no mailbox, transcript, agent account
or claimed result. Token provenance is retained privately by the app, which
emits categorized events with monotonically increasing sequence numbers.

Outcome precedence is normative:

1. Infrastructure failure (startup, server death, malformed/missing state,
   fixture delivery failure, runner transport failure, spawn or cleanup
   failure) produces `harness_error/harness_error`.
2. An agent still alive at its deadline produces `fail/budget`, even if it
   previously reached the business goal. The runner terminates its group.
3. The earliest matching named trap anywhere in the frozen trial produces fail
   with its existing class, whether before or after goal completion (0071).
4. Otherwise a satisfied app predicate produces pass, independent of the
   child's exit code. Isolation additionally forbids any foreign presentation.
5. On unsatisfied state, the first relevant rejected action by app event
   sequence determines the class: another identity's valid token is
   `wrong_message`; a superseded/expired OTP is `stale_code`; a known decoy
   link or incorrect invite selection/time is `wrong_link`; malformed or
   unknown credentials, invalid flow arguments and reset misuse are
   `tool_misuse`. Expected replay refusal in `reset-reuse` is evidence for
   its predicate and excluded from failure events.
6. If there is no such action, use the task's declared `idle_class`, either
   `gave_up` or `wait_timeout`. These are declared idle outcomes, not inferred
   mental states or proof that a client called MCP wait.

Transport failures observed only inside an external agent cannot be
reliably classified without telemetry. Its exit status is retained, while
app state determines failure; do not invent transcript classification.
Runner/setup and built-in oracle protocol failures are retained as harness
diagnostics; the proof must refuse an oracle transport error as a success.

## Trial, external-command and result contracts

`trial --task ID --agent LABEL [--deadline-ms N] [--artifacts-dir DIR]
[-- PROGRAM ARG...]` runs one attempt. `oracle` and `null` are reserved
labels, executed as child subcommands through the same spawn/environment/
deadline boundary as external commands. Other labels require literal argv;
never interpolate a shell command. The v1 process lifecycle targets Unix.

Resolve `SWARMAIL_BIN` to an executable built from this checkout; otherwise
build the root binary once with `cargo build --offline --locked --bin
swarmail` before provisioning. Matrix resolves once. Proof tests resolve
once and pass the absolute path to each trial, avoiding recursive builds
and shared target writes per trial. Build failures never skip the proof.

Start fresh in-memory Swarmail with all listeners at `127.0.0.1:0`, no TLS
or data file and explicit cap 100000. Use the closed runtime map in proposed 0075
for that child, pin startup logging, parse announced actual addresses, then
check HTTP health and SMTP greeting. Fixture readiness also requires a real
HTTP response. Startup has a separate bounded deadline; port probing then
rebinding and sleep-as-readiness are forbidden.

Each agent receives `EVAL_TASK_ID`, `EVAL_APP_URL`, `EVAL_MCP_URL`,
`EVAL_IDENTITY`, `EVAL_INBOX`, `EVAL_GOAL`, `EVAL_PROMPT_FILE`, and
`EVAL_DEADLINE_MS`. Public work directory and prompt are per identity; no
admin URL/secret or private snapshot path is exported to agents. External
credentials/configuration remain the caller's responsibility. HTTP and MCP
clients use explicit I/O bounds; MCP initialize negotiates `2025-03-26`,
IDs are correlated, tool JSON is decoded from text content, and errors are
not mistaken for data. Use actual loopback Host and no Origin (0043).
Oracle reads mail only via MCP search/get/wait, never REST, raw-store access,
seeds or private fixture state. It has no attachment API to invent.

The runner monitors server exit while agents run. It owns process groups,
captures and continuously drains child output with bounded retention, and
terminates, escalates and reaps children on every handled ending. After
agent termination, freeze/snapshot fixture state, stop servers, score and
emit. SIGINT/SIGTERM trigger cleanup and a harness-error result. Catchable
panics follow the same guarded cleanup path; SIGKILL, abort, machine loss
and an unwritable result stream cannot promise a line. Commands must not
daemonize or leave their process groups; unconfined cleanup is not a sandbox.

Exactly one stdout JSON line per attempted trial, diagnostics on stderr:
`schema_version:2`, required integer `concurrency:1`, `trial_id`, `task`, `agent`, `outcome`
(`pass|fail|harness_error`), `failure_class` (null only on pass),
`wall_time_ms` (monotonic, provisioning through cleanup, excluding binary
build), `agent_exit_status` (integer or null), `agent_exits` (ordered records
of identity, numeric code or null, signal or null), `confinement:"unconfined"`.
For ordinary trials the singular status is that child's numeric code; it is
null when no numeric exit exists. For isolation it is null, and all three
actual statuses are in `agent_exits`; never synthesize a successful command
exit. Failed spawns have null code and signal. All built-ins also carry the
unconfined label; built-in does not mean sandboxed.

Trial exit codes are 0 pass, 1 fail, 2 harness error. Invalid trial inputs
produce one harness-error row using supplied labels (null if absent);
help/version are not trials. A requested artifact directory retains a unique
trial subdirectory with `state.json`, `result.json`, redacted diagnostics
and metadata; default temporary files are removed after the result is made.
Snapshots contain no solution secrets and are written atomically by the
runner, permitting the later exec scoring gate to reproduce the check.
`check --task ID --state PATH` applies the identical pure predicate and
prints its verdict; malformed/mismatched snapshots refuse as harness error.

`matrix --tasks ID[,ID...] --agents PATH --trials N` takes explicit task
IDs, an agent JSON file and a positive trial count. That file is an array
of objects `{"label":"oracle","argv":[]}` or
`{"label":"custom","argv":["/absolute/agent","--mode","email"]}`;
reserved built-ins require empty argv, external agents require nonempty
argv, and duplicate labels/unknown fields refuse. It runs their Cartesian product,
sequentially by default, emitting one trial row each without retries or
discarding failures. It continues after a trial harness error and exits 2
if any occurred, else 1 if any fail, else 0. It owns the only stdout writer.

`report --input PATH` (or `--input -` for stdin) validates JSONL and groups
by task/agent. Print attempts, passes,
fails, harness errors and harness-error rate; success is passes divided by
passes plus fails, with a 95% Wilson interval (`z=1.959963984540054`). Also
print passes/attempts so infrastructure loss remains visible. Median and
p95 use nearest rank, index `ceil(p*n)-1`, over non-harness-error wall times;
print sample sizes and failure-class counts, including harness_error.
Empty denominators/inputs produce null estimates, never NaN or invented
100%. Malformed rows refuse; no arbitrary error-rate threshold is added.

## Acceptance Criteria

- **AC1 — Boundary:** Root manifests, source, tests, scripts, bundle and CI
  have zero diff; package listing excludes evals. Standalone metadata and
  full lock-closure comparison prove separation and offline dependencies.
- **AC2 — Fixture:** Real HTTP/SMTP tests exercise all eight flows, DATA 250
  receipts, per-flow read-only state and secret redaction. Wrong identity,
  token, stale/expired OTP, decoy, wrong invite time and consumed reset
  cannot satisfy their predicates; reads cannot change state.
- **AC3 — Catalogue:** Exactly the 16 rows above validate and serialize
  with unique IDs, three tiers, strict enum/template validation, declared
  idle classes and deterministic app-state checks.
- **AC4 — Positive control:** `evals/tests/proof.rs` runs every task through
  the actual CLI with oracle; all 16 pass through real HTTP/SMTP/MCP.
- **AC5 — Negative controls:** The same test target runs every task with
  null; all 16 fail with the row's idle class and zero goal completion.
  Focused wrong-action children provoke wrong_message, stale_code,
  wrong_link and tool_misuse; a nonterminating child proves budget.
- **AC6 — Sequential admission and within-trial inboxes:** A held idle
  victim fails after a competing trial refuses before agent launch. A later
  oracle passes. Each inbox-isolation trial retains three AUTH inboxes on
  one server, and its foreign-token control fails.
- **AC7 — Trap fidelity:** Wire proofs establish both OTP generations,
  Confirm/Unsubscribe/Report, sibling messages, entity URLs, Unicode
  subject, initial wait timeout before release, observed 451 then 250,
  40 digest links, identical attached/downloaded ICS and HTTP reply challenge.
- **AC8 — Scoring:** Pure snapshot tests prove precedence, idle fallback,
  expected reset replay handling, strict snapshot validation and rejection
  of fabricated success flags; exits alone neither pass nor fail a goal.
- **AC9 — Lifecycle and seam:** Real external child tests prove exact argv,
  env/prompt agreement, readiness, spawn failure, early server exit,
  deadline, interruption, catchable panic and descendant cleanup. Every
  handled attempt emits exactly one schema-valid row with truthful statuses.
- **AC10 — Batch/report:** Matrix cardinality, no stdout contamination,
  continued accounting after errors, exit aggregation, Wilson calculations,
  quantiles, malformed input and empty/all-error reports have tests.
- **AC11 — Honest limits:** README and decision retain the ICS/reply/slow
  task limits, Unix/unconfined posture, deferred driver and deferred gate
  wiring. No real model claim is made from oracle/null controls.
- **AC12 — Delivery:** Evals' own fmt/clippy/tests pass, run by implement
  and reproduced by reviewers until CI/verify wiring follows #120 batch 5.
  Root gates remain unchanged and green, including exact coverage,
  selftest, publish guard and license/native-tls checks. Pinned bundle
  compilation belongs to verify. Signed commits only; no push/PR/merge.

## Synthesis rulings and costs

Adopt simplicity's one crate/executable, typed catalogue, in-memory state,
minimal MIME/SMTP and sequential matrix. Reject a second fixture binary,
task DSL, worker pool and third public `stall` agent: test-only external
children prove failure paths. Cost: fixed v1 tasks and Unix lifecycle;
extension requires code and review.

Adopt robustness's ephemeral binds, separate admin listener, pure snapshots,
process groups, child-path oracle/null, pre-agent chaos and provenance-based
failure classification. Reject simplicity's assignment of a targeted trap
class to an idle agent. Cost: extra state/event types and tests, two listeners
and separate target/idle declarations; the benefit is that counts say what
was observed rather than attributing nonexistent actions.

Reject robustness's permanently unavailable support stub and attachment-only
assumptions. Adopt simplicity's HTTP challenge reply and token-authorized
ICS download. Cost: no claim about MCP attachment retrieval (#159) or
Swarmail reply transport (#160); those future tasks must be separately named
and cannot be compared as if these tasks measured them.

Reject timer-only slow-task ordering and impossible promises of output after
SIGKILL. Use the explicit release protocol and bounded handled-error
contract above. Reject robustness's `builtin` confinement label and arbitrary
report error threshold. Cost: the first wait is prescribed but not app-score
enforced; every row discloses unconfined execution and infrastructure counts.


## 2026-10-24 clarification — decision 0075 (renumbered from 0062) (proposed)

This dated clarification supersedes the original caller-environment and raw-
diagnostic wording above where it conflicts. Every harness-created child starts
with `env_clear()`. The common map is fixed `PATH=/usr/bin:/bin`, `LANG=C`,
`LC_ALL=C`, a fresh private `HOME` and `TMPDIR`, and fresh XDG config/cache/data/
state directories under a 0700 `/tmp` owner. Swarmail adds only fixed `RUST_LOG`;
the fixture alone receives its generated admin secret; each agent receives
exactly the eight `EVAL_*` trial inputs already listed. Matrix trial workers add
only resolved `SWARMAIL_BIN`. Caller variables and test controls are not forwarded.
Resolve programs at the original cwd/PATH boundary; keep remaining argv literal.
Ambient credentials/runtime config migrate to deliberate caller-owned wrappers.

The runner concurrently drains stdout and stderr and retains zero arbitrary
child bytes. Diagnostics, panic/Clap errors, Cargo failures, paths, argv,
response bodies and OS error text use safe categories and numeric facts only.
Service announcement lines cap at 8192 bytes and publish only one typed
loopback tuple. Cargo records cap at 1 MiB; matrix output retains the stricter #194 total 64 KiB bound; overflow is drained
then fails safely. Matrix stderr is discarded. `diagnostics.log` is capped at
64 KiB; metadata records `swarmail_bin: null`. These rules do not imply process
confinement or redact explicit task/agent labels, argv, caller-selected config
paths, or files a child deliberately writes. Results remain `unconfined`.

Automatic Swarmail preparation also clears the environment, resolves Cargo,
rustc and rustdoc from the original PATH, and sets only the named toolchain and
cache selectors in decision 0075 (renumbered from 0062). Preparation remains offline and locked.

## 2026-10-24 integration onto matrix cancellation

Proposed [0075](../../docs/decisions/0075-eval-privacy.md), renumbered from
0062, and [the port contract](../eval-privacy-port/spec.md) govern the current
closed environments and safe publication. Preserve #194's owned waiters,
pre-spawn signal handlers, concurrent five-second TERM grace plus five-second
post-KILL observation, final cleanup fence and complete matrix accounting.
Private runtime owners survive abort, reaping and drainage. Agents receive
generated PWD equal to actual cwd. Matrix row capture remains 64 KiB. Artifact
publication is exclusive, initially private, descriptor-normalized before
payload and atomic per file with no clobbering. Existing entries remain intact.
Historical receipts above are not proof of this port. Five fresh 0058 careful
reviews and every Linux preparation/toolchain/custom-target, eval fmt/locked
clippy/full-test, artifact, mutation and gate proof remain obligations. Under
the 2026-10-24 operator ruling, row **macOS evidence for `evals/`**, in
[operator-rulings.md](../../.forge/epic192/issue199/operator-rulings.md), `evals/`
is outside CI on every OS until #201. #201's CI gating owns macOS eval
preparation/toolchain, clippy/tests and selected artifact-primitive execution;
those receipts do not block #196 (port spec AC13/AC23). This records ownership,
not macOS execution or existing CI proof.

## 2026-10-25 — local replacement for issue #148 AC4 (#199)

The harness's AC4 above remains the oracle positive control. The issue's
“trials can't see each other” claim is replaced under the verified operator
ruling, recorded by [proposed 0076](../../docs/decisions/0076-sequential-eval-trials.md):

The baseline runs sequentially with a fresh HOME per trial (and per agent).
Every result row records `concurrency: 1`, the supported trial execution limit,
including refused and unstarted rows; it is neither an executed-agent count nor
an observation of host activity. Multiple agents inside one inbox-isolation
trial still run together.

Concurrent trials are not isolated: a same-uid agent can read a concurrent trial's prompt from the shared TMPDIR, use its MCP URL, app URL, identity and inbox context, and complete that trial's task, so the victim's do-nothing agent passes.
Private TMPDIRs under fixed `/tmp` remain readable by the same uid. Namespace
isolation belongs to #221. Admission is an advisory, nonblocking reservation
for cooperating current runners with the same effective uid and shared `/tmp`,
independent of caller HOME, TMPDIR, cwd, checkout and artifacts. It is not
confinement across hosts, uids, mount namespaces, old binaries or hostile
lock bypass. A contender refuses with `concurrent_trials_refused`; unsafe
admission refuses with `trial_admission_unavailable`. Trial and matrix accept
`--concurrency 1` (the default); other parsed integers refuse as
`concurrency_unsupported`, never silently downgrade. A parsed trial emits one
harness-error row and exits 2; an invalid matrix emits no rows and exits 2.

Admission stays held through owned cleanup, reaping, drains and private-state
disposal, including a dropped library waiter on a surviving runtime. Unproved
cleanup stops matrix admission and accounts for its unstarted suffix. Forced
runner death (SIGKILL/abort), runtime destruction or host failure may release
the lock without stopping children: these runs are invalid baseline evidence.
The operator must confirm cleanup before resuming. Lock release is not proof
of cessation. CI-only house proofs remain CI-owned; #201 owns macOS eval proof.

Every row and retained result uses schema 2 and required integer concurrency 1;
old-version or missing/invalid metadata refuses in matrix and report. No
catalogue/snapshot version or scoring change follows. The baseline procedure
is the local [#151 run plan](../../evals/run-plan.md). Historical overlap proof
is not current isolation evidence.
