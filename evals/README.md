# swarmail-evals — the agent eval harness

Epic #164, Phase 0 run A: the fixture app (#149 — the `fixture` subcommand of
this crate, `src/fixture.rs`, not a separate directory), the v2 task suite with
oracle solutions (#150) and the agent-agnostic trial runner that #148's
brokkr recipe will wrap. The original design is accepted decision
[0055](../docs/decisions/0055-agent-eval-harness.md); the acceptance
authority is the [spec](../specs/agent-eval-harness/spec.md).

The harness is proven able to fail before it measures anything: every task
passes with the scripted `oracle` and fails with the do-nothing `null`
(`tests/proof.rs`).

## Sequential baseline (#199)

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

The local [#151 run plan](run-plan.md) defines baseline controls and accounting.
Result schema 2 rejects historical schema-1 rows and missing or invalid
concurrency; use their historical reader rather than inventing metadata.
Catalogue and snapshot versions are unchanged.

## Layout and gates

`evals/` is a **standalone crate**: its own `[workspace]` and `Cargo.lock`,
like `tools/coverage-fn-scan`. The root package, its lock, its tests and its
exact coverage gate neither build nor measure it, and the published `.crate`
does not include it. Every package in `evals/Cargo.lock` is already in the
root lock at the same version and source, except the named dev-dependency
exceptions of the dated 2026-10-30 amendment to decision 0055 (`pulldown-cmark`
and `unicase`, exact versions; `tests/boundary.rs` checks it).

CI and the verify seat run these gates (#201, per the dated 2026-10-31
addendum to decision 0055): the `test` jobs on ubuntu and macOS and the pinned
verify seat run the three commands below, `--locked`, from `evals/`, with
`SWARMAIL_BIN` set to the candidate's own root `swarmail` (resolved from
Cargo's artifact record of `cargo build --locked --bin swarmail`, never the
fallback build below). The same commands are the local reproduction, from
`evals/`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked                  # everything; the proof alone:
cargo test --locked --test proof     # oracle 16/16 pass, null 16/16 fail, sequential refusal control
```

The tests locate the Swarmail under test through `SWARMAIL_BIN`; when it is
unset they build the root `swarmail` binary once per test binary
(`cargo build --offline --locked --bin swarmail` in the repository root,
reading the executable path from Cargo's artifact report, so a custom
`CARGO_TARGET_DIR` is honoured).

## Approved baseline setup (#151 part 1)

The fixed six-model declarations and control-gated `baseline` command are
under [run-plan.md](run-plan.md). Read [wrappers.md](wrappers.md) for the
closed private selection/auth schemas, exact versioned launch recipes and
conservative per-family tool surfaces. Offline stand-ins prove assembly;
first authenticated smoke with each approved model, live HTTP MCP (especially
dsh), and the paid baseline remain operator-owned. Dry-run opens no private
selection, creates no output and launches nothing. Never treat offline assembly
or a declared surface as a provider interoperability or confinement claim.

## Commands

```bash
cargo build --offline --locked --bin swarmail
cargo build --offline --locked --manifest-path evals/Cargo.toml
export SWARMAIL_BIN="$PWD/target/debug/swarmail"   # adjust for CARGO_TARGET_DIR
E=evals/target/debug/swarmail-evals

$E tasks                                            # the v2 catalogue, schema 2
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

An artifact or output root (`--artifacts-dir`, `check --artifacts-dir` and
`--matrix`, `baseline --output`) may be named through symlinked *ancestor*
components only when each such link, the directory holding it and the
directory it resolves to are owned by root and cannot be written by group or
other (a root-owned sticky directory such as `/private/tmp` qualifies). On
macOS that admits `/tmp`, the per-user TMPDIR under `/var` and `/etc`, so
`--artifacts-dir /tmp/evals` works. The harness resolves the name once, to its
real path, and uses only that path afterwards. The root itself and everything
under it still refuse every symlink, `..` components refuse, and a symlinked
ancestor owned by anyone else, or writable by group or other, refuses, because
its owner could swap it between the check and the use. Name a per-user root
that does not yet exist: a directory another local user created first in a
shared directory is not rejected.

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

The matrix accepts exactly one matching schema-v2 result from a trial and
checks its exit code. It drains child output while retaining at most 64 KiB;
invalid, extra or oversized output stops admission with exit 2. Broken or
non-progressing stdout also exits 2 after cleanup; unavailable output cannot
promise complete JSONL. SIGKILL, host failure, daemonizing or hostile
commands, escaped processes and uninterruptible kernel tasks remain outside
the lifecycle guarantee. `check` exits 0 pass, 1 fail, 2 for unscorable
state. Only machine output goes to stdout. Stderr contains harness-authored
safe categories and numeric facts. Child stdout/stderr is drained concurrently
and discarded; diagnostics.log contains at most 64 KiB of safe facts and
metadata keeps swarmail_bin null. Readiness retains one typed loopback
announcement from complete lines up to 8192 bytes. Cargo records cap at 1 MiB;
matrix retains its stricter 64 KiB total row capture. Overflow is drained.

## The external agent contract

An agent is a label plus a literal argv after `--` (never a shell string);
`oracle` and `null` are reserved built-ins run through the same child
boundary. Every child starts with env_clear. The common runtime map is
PATH=/usr/bin:/bin, LANG=C, LC_ALL=C, private HOME/TMPDIR and separate
XDG_CONFIG_HOME, XDG_CACHE_HOME, XDG_DATA_HOME and XDG_STATE_HOME directories
under exclusive 0700 /tmp ownership. Runtime ownership lasts through cleanup,
reaping and drainage. Agents add generated PWD equal to their actual private
working directory and exactly these eight task inputs:

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

The fixture alone receives its generated admin secret. Swarmail receives only
its fixed `RUST_LOG`; matrix workers receive only the resolved `SWARMAIL_BIN`
in addition to the common runtime map. Caller API keys, proxies, arbitrary
`EVAL_*`/`SWARMAIL_*` values, shell startup settings and caller `HOME` are
not forwarded. Executable paths are resolved against the caller's original
cwd/PATH before launch; subsequent arguments remain literal. An unset PATH
uses `/usr/bin:/bin` for lookup. Custom interpreters and runtimes need a
caller-selected wrapper.

Automatic Swarmail builds use a separately cleared preparation environment.
The harness resolves Cargo, rustc and rustdoc through the original PATH and
sets only those tools, the generated tool PATH, explicit/default Cargo and
rustup homes, optional `RUSTUP_TOOLCHAIN`, and optional `CARGO_TARGET_DIR`
plus the private runtime map. Preparation remains offline and locked. A
prebuilt `SWARMAIL_BIN` can be supplied where local toolchain discovery is
unsupported.

Credentials previously found through ambient API keys or caller HOME must
move to an explicitly selected wrapper/configuration mechanism. A wrapper
may read its own existing private config; the harness does not read or copy
that credential. Credential values must not appear in argv, tracked files,
labels or reports. The trial remains `confinement: "unconfined"`: a child can
still access the filesystem/network and can write its own logs.

No admin URL, admin secret or snapshot path is ever exported to agents. Every request
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

One JSON line per trial: `schema_version` (2), required integer `concurrency` (1), `trial_id`, `task`, `agent`,
`outcome` (`pass|fail|harness_error`), `failure_class` (null only on pass),
`wall_time_ms` (provisioning through cleanup, excluding the binary build),
`agent_exit_status` (the single agent's numeric code, else null — always
null for the three-agent isolation task), `agent_exits` (identity, code,
signal per agent), `confinement: "unconfined"` and required typed `provenance`. Exit statuses never decide
a goal: an agent that completes the task and then crashes passes with its
real status when no named trap disqualifies it; `null` exits zero and fails.

## Tasks and the failure taxonomy

`swarmail-evals tasks` prints the catalogue: 15 tasks in three tiers plus
`inbox-isolation` (three agents on one Swarmail behind one shared SMTP
AUTH login, `fixture-app`; each agent's address is routed to its own inbox
by the runner's `--route-domain swarmail.test`, decision 0112). Each
declares the `failure_class` it provokes and the `idle_class` (`gave_up` or
`wait_timeout`) a do-nothing agent earns. The class set is closed:
`wrong_message`, `stale_code`, `wrong_link`, `wait_timeout`, `gave_up`,
`tool_misuse`, `budget`, `harness_error`.

Scoring precedence: infrastructure failure → `harness_error`; an agent alive
at its deadline → `fail/budget`; the earliest named trap → fail with its
existing class; otherwise a satisfied app-state predicate → pass
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

## Active scoring revision — 2026-10-25 (accepted 0071)

[0071](../docs/decisions/0071-eval-trap-scoring.md) advances catalogue,
snapshot and trial-result versions together to **2**, with unchanged fields,
sixteen task IDs, tiers, eight failure classes and report arithmetic.
`check` refuses v1 snapshots; `report` refuses v1 rows and mixed v1/v2 input
before aggregation, with version diagnostics. Replay historical v1 artifacts
with their recorded old source/binary. Do not relabel, migrate or pool v1 and
v2 scores: they are different experiments.

A rejected exact named action disqualifies its whole trial, before **or after**
the goal. Application recovery remains usable and goal facts stay truthful.
The seven affected goals disclose the rule:

| Task | Action / class |
|---|---|
| confirm-decoys | decoy_link / wrong_link |
| sibling-addresses | foreign_token / wrong_message |
| i18n-subject | decoy_message / wrong_message |
| stale-otp | superseded_code / stale_code |
| magic-login | decoy_link / wrong_link |
| entity-url | decoy_link or marketing_mangled / wrong_link |
| digest-forty | digest_decoy / wrong_link |

Matching requires rejected status, exact action/class/flow and a provisioned
Agent claimant. Sibling's known Decoy claimant also counts with any flow,
including null, because identity refusal can precede token lookup. Missing or
unknown claimants and other structurally valid mismatches do not match.
Expired-but-not-superseded OTPs and non-target tasks retain recovery; expected
reset replay remains positive evidence. Isolation keeps its existing veto and
first-refusal fallback. Validation/infrastructure and live deadlines still win.
The frozen snapshot is evidence, not authenticated against a hostile local process.

`tests/proof.rs` completes ten independent external recovery controls through
real MCP and fixture HTTP backed by SMTP DATA 250. Confirm enumerates all three
links forward and reverse; digest opens forty distinct links with 39 decoys.
Every command exits zero and truly recovers its goal, yet loses in the named
class; the actual retained-state `check` agrees. Oracle/null and overlapping
isolation controls remain required. Linux proof does not imply macOS
execution: #201's CI gating owns macOS eval proof, the operator owns sealing/live settings,
and #221 owns namespace isolation.

Three further scripted controls (`near_miss_controls_recover_after_a_timed_out_wait`,
issue #156, Swarmail decision 0111) prove recovery after a failed wait is
mechanically possible from the timed-out answer alone, through the real CLI,
real MCP and real SMTP, with no catalogue change (0071): a sibling-trap agent
that mistypes the subject reads `near_misses`, picks the entry addressed to its
exact identity and passes with no trap event; the same wait acting on
`nearest_subject` (the decoy, which lands last and ties on subject) loses as
`wrong_message`; and in slow-delivery the first timed-out wait carries no
`near_misses` (nothing arrived), after which the release and confirmation pass.

## Honest limits

- **Unconfined.** Process-group ownership is lifecycle cleanup, not a
  sandbox; a child that daemonizes or leaves its group escapes. Admin
  separation stops accidents, not a hostile local process. Unix only.
- **slow-delivery** prescribes an initial MCP wait before the release; the
  app-state score proves the final confirmation, not that an arbitrary agent
  performed that first wait.
- **The rate of recovery after a failed wait is not measured.** The three
  near-miss controls above are scripted: they show recovery is possible and
  that presenting the nearest subject as the match still loses, not how often
  a live model recovers. That needs paid live-model trials, which no run here
  holds. Slow-delivery's goal already prescribes the release, so recovery
  there is not attributable to the near-miss answer.
- **smtp-retry** proves the agent copes with mail that was refused (a real
  451 under 100% DATA chaos) and retried before the agent started — not
  recovery during a live outage.
- **invite-time** serves the ICS through the app's token-authorized
  download; it does not measure MCP attachment retrieval (#159).
- **support-reply** replies through the app's HTTP form with the mailed
  challenge; it does not measure Swarmail inbound reply transport (#160).
- No tokens, turns or cost here: those come from brokkr's journal when the
  #148 recipe wraps this runner. No model was run in this batch.

## Private artifact publication (proposed 0075, renumbered from 0062)

A requested trial leaf is created exclusively with mode 0700 before payload.
Each file is created exclusively at 0600, normalized through its descriptor
before writing, completed privately and published atomically without clobbering.
Existing leaves/targets refuse without altering their bytes or permissions;
existing parent permissions remain unchanged. This is per-file visibility,
not a durable multi-file transaction. No arbitrary child output, error text,
URLs, argv, panic payloads or executable paths are published. Explicit labels
and schema fields remain literal and must never contain secrets. All loopback
HTTP bypasses caller proxies. Fault controls are compiled only in tests.

Native OS paths and later arguments survive CLI and matrix worker handoff;
agent JSON remains textual. Build homes are explicit nonempty locators or lazy
original HOME/.cargo and HOME/.rustup defaults. Optional nonempty toolchain and
target selectors apply only to preparation; there is no inheritance fallback.
Under the 2026-10-24 operator ruling, row **macOS evidence for `evals/`**, in
[operator-rulings.md](../docs/rulings/epic-192.md), `evals/`
was outside CI on every OS until #201 wired it (2026-10-31: the macOS `test`
job now runs the evals fmt, clippy and tests). #201's CI gating owns macOS eval
preparation/toolchain, clippy/tests and artifact-primitive execution; those
receipts are not a completion prerequisite for #196. Every Linux preparation,
custom-target, artifact, mutation and gate proof remains required by
[the port contract](../specs/eval-privacy-port/spec.md#acceptance-criteria).
This records ownership, not macOS execution or CI success; prior receipts
remain historical.


## Reproducible inputs and retained results (#200)

Accepted [0071](../docs/decisions/0071-eval-trap-scoring.md) governs catalogue,
snapshot and result business version 2 and trap-before-goal scoring.
Proposed [0063](../docs/decisions/0063-eval-provenance.md) records the provenance
port. New v2 rows require typed provenance. `report` refuses v1, mixed and
pre-provenance v2 rows before aggregation; historical records need their
matching historical binary. Wilson intervals, accounting and quantiles stay
the same. All fields, including nullable fields, are required; duplicate,
unknown, hybrid and unsupported shapes refuse. Bare-state `check` keeps
current v2 scoring and refuses v1 snapshots. There is no archive converter.

`trial` and `matrix` accept `--seed` (exactly 64 lowercase hex digits) and
`--fixture-time` (whole-second UTC RFC3339 ending in `Z`, calendar dates from
1970-01-01 through 9998-12-01 inclusive). Invalid inputs exit 2 before launch.
Defaults resolve one fresh public seed and one UTC sample per invocation.
Resolved values are printed on stderr and recorded in every v2 result. A
matrix shares one time and derives each cell seed from the master seed,
zero-based task occurrence, task ID, agent index/label and repetition index.
Duplicate tasks remain separate ordered occurrences. Matrix coordinates
intentionally differ between agents; use separate trials with identical
seed/time for matched fixtures.

`fixture-sha256-v1` frames each byte string with its eight-byte big-endian
length. Integers are eight big-endian bytes before framing; text is literal
UTF-8 without normalization. Cell SHA-256 inputs, in order, are domain
`swarmail-eval-cell-v1`, master seed bytes, task index, task ID, agent index,
agent label and repetition index. Choice inputs are domain
`swarmail-eval-choice-v1`, effective seed bytes, task ID, logical identity
slot, flow name, generation index and field name. A selection uses the first
eight digest bytes as a big-endian integer modulo a fixed nonzero bound.

| Semantic field key | Meaning |
|---|---|
| `target-position` | Digest's target among 40 items, flow `digest`, generation 0 |
| `start-days` | Invite offset 3 + choice modulo 17, flow `invite`, generation 0 |
| `start-hour` | Invite hour 8 + choice modulo 10, flow `invite`, generation 0 |
| `message-id` | First 16 digest bytes, lowercase hex, for each slot/flow/generation |
| `mime-boundary` | Same truncation with this separate key and `=_eval_` prefix |

Slots follow provisioned logical account order, including subsequently
appended decoys. Message generation is zero-based per account and flow,
independent of cross-slot arrival order. Public algorithm/key changes need a
new version. Vectors and typed compact `catalogue-json-v1` bytes/digests are
pinned in `tests/provenance.rs` and `tests/data/`.

Stable projection v1 includes scenario, ordered logical roles, message roles
and generations, digest position, invite schedule, Date/DTSTAMP, MIME and
non-capability suffixes, and business transitions for identical scripted
actions. Actual identities/inboxes, URLs/ports, execution IDs, elapsed/event
observations, cross-slot arrival order and capabilities are excluded.
Tokens, OTPs, passwords, support tickets/challenges, invite UID and admin
bearer use fresh private entropy independent of public choices. A seed does
not reproduce raw mail, secrets, model actions, scheduling or outcomes.
UTC injection affects fixture mail/calendar dates; expiry/deadlines stay
monotonic and TLS keeps real time. Invite token, owner and UID checks remain.

The required `provenance` object contains `fixture`, nullable `matrix`,
`task`, `harness`, `target`, `agent` and `limits`. It records actual executable
SHA-256 values, task/catalogue identity and `business-score-v2`, build-time
harness revision/dirty facts, target source association, agent declarations,
and effective bounds. Availability is explicit: `present` with typed value
and field-specific source, `unavailable` with a bounded reason, or builtin
model's `not_applicable`. Unknown values never become placeholder hashes.

Harness Git facts are embedded by eval-only `build.rs`, with source, HEAD,
index, ref, packed-ref and worktree invalidation. Runtime checkout HEAD is
never used. Owned offline/locked target builds compare source revision,
dirty state and a private source-content fingerprint before/after Cargo,
and bind Cargo's selected artifact digest. Source changes refuse association
with `build_context_changed`. An opaque `SWARMAIL_BIN` is measured while its
source remains `external_source_unknown`; no speculative extra build or
external receipt is used. Bytes are rechecked before launches. Cooperative
checks prove content consistency, not authenticity or elimination of hostile
check-to-exec races. A shell digest identifies the shell, not loaded scripts.

External trials accept optional `--agent-model`, `--agent-config-id` and
`--public-argv`. Agent-file entries have optional `model`, `config_id` and
`public_argv` (default false). Old entries run with undeclared model/config
and private command attribution. Builtin overrides refuse before admission;
omitted values and `public_argv:false` remain valid. Models/config IDs are
operator declarations, never inferred from labels or claimed as measured
remote-service identities. Builtin model is not applicable.

`--public-argv` declares the **entire literal argv public and credential-free**.
Only then does `public-argv-sha256-v1` hash framed domain
`swarmail-eval-public-argv-v1`, framed argc as eight big-endian bytes, then each
exact UTF-8 argument including program. No normalization or PATH expansion is
hashed; literals are never retained. Without this opt-in the command is
`private_argv`, while executable bytes are still measured independently.
No environment, prompts, raw mail or private configuration is archived or
hashed. Public labels/model/config/opted-in argv are the caller's declaration;
there is no credential sanitizer for deliberately public data.

## Retention and offline replay

`trial --artifacts-dir ROOT` exclusively creates `ROOT/<trial-id>/`.
`matrix --artifacts-dir ROOT` exclusively creates `ROOT/<matrix-id>/` with
`matrix-plan.json`, `trials/<trial-id>/` and `matrix.json`. Generated IDs have
128 fresh random bits and never use labels as path components. Concurrent
runs can share ROOT. Existing or partial destinations are never reused.

Trial `metadata.json` schema 1 is published last and binds exact result,
optional state and safe diagnostics bytes by fixed path/length/SHA-256.
State absence is explicit. The prelaunch matrix plan assigns ordered cell
IDs/inputs; the final index binds the plan and every metadata file. No marker
is published for incomplete/conflicting evidence. Requested trial retention failure
overrides business pass/fail with `harness_error`; a retained setup error can
be complete evidence without a snapshot. Parent fallback is only after reap
and only if no child directory exists. Full disk, abort, SIGKILL, parent death
or unavailable stdout cannot guarantee rows. Rename proves detectable
completeness, not fsync or power-loss durability. Public diagnostics contain
controlled codes; raw child logs/panic data are not retained.

Matrix accepts exactly one v2 child line, binds assigned identity/inputs and
actual command exit, and checks retained equality after stdout commitment.
If cancellation changes the committed row after the child has retained its
result, immutable evidence stays unchanged and no matrix index is published.
An index-write failure exits 2 without rewriting committed stdout rows. Stdout is drained with a
64 KiB cap. The child envelope of agent deadline + 300 seconds requests
cancellation once; it never forces a return or adds a competing parent kill
timer. The existing cleanup owner retains its five-second TERM and five-second
post-KILL observation budgets, direct waits and drain fence. Existing 30-second startup,
10-second SMTP I/O and 5-second shutdown bounds remain.

New check modes are exclusive:

```bash
$E check --artifacts-dir /tmp/evals/<standalone-trial-id>
$E check --matrix /tmp/evals/<matrix-id>
$E check --task otp --state /tmp/evals/<trial-id>/state.json  # bare v2 snapshot
```

New envelopes require v2 results; unversioned historical metadata and v1
results inside new envelopes refuse. Bare v2 state remains checkable without
gaining provenance; v1 state needs its historical binary. Offline check validates all
evidence before any output, then reuses snapshot validation/scoring. It never
launches saved commands, builds binaries, contacts services or reparses mail.
Snapshots are processed one at a time; only bounded replay rows accumulate.
Original executables need not exist. Paths, symlinks, non-regular files,
versions, cardinality/order, identities, lengths/hashes and catalogue/scorer
compatibility are checked. Hashes do not authenticate a local writer.

Replay schema 1 keeps `original_result` unchanged beside `business_replay`,
which is `scored` with a verdict or `unscorable` with a bounded reason.
Original budget/infrastructure history never becomes historical pass. Replay
exits 2 for any unscorable row, else 1 for any business failure, else 0.

Retained limits: 4096 cells; plan/index 8 MiB each; metadata/result/diagnostics
64 KiB each; state 16 MiB; agent input 1 MiB; public task/label/model/config
strings 1024 bytes; JSON depth 64. New result rows have a 64 KiB limit including
the newline in every mode. Readers check declared sizes before allocation and
use fixed regular files; writers bound serialization. Legacy non-retained
product limits and report statistics remain unchanged.


## Snapshot publication and historical claims

New snapshot events retain a canonical provisioned identity, including decoys
and other agents, or `claimed: null` for missing/unknown identity headers.
Unknown spelling is deliberately lost. HTTP refusals, event order/categories,
accepted facts and business-score-v2 stay unchanged. Both admin serialization
and retention check this writer invariant before publishing state/results.
Admin invariant failure is a fixed HTTP 500 without state; retention failure
is infrastructure/exit 2 after cleanup with no completion marker. The admin
check also permits ordinary unfrozen snapshots.

Historical bare-state snapshots continue accepting omitted, null and arbitrary
unknown claims. Retained snapshots require every existing key, including
nullable `claimed`, but otherwise valid historical unknown strings still score.
Neither reader rewrites source bytes or execution outcomes. Public check and
retained-library errors use fixed bounded diagnostics, without echoing JSON
keys, claims, identities, paths or scorer details. Invalid bare state retains
its harness-error row/exit 2; invalid retained evidence yields no replay rows,
including when a later matrix cell is invalid.

Reader success proves consistency and scoreability, never historical secrecy
or permission to publish an old evidence tree. Old leaked bytes remain under
operator control. Claim membership cannot authenticate caller-authored identity
lists or certify arbitrary external data as private; public argv/model/config
inputs retain their documented caller responsibility. No archive conversion,
secret hashing, automatic cleanup or saved-command execution is performed.

Public argv is hashed from the original UTF-8 declaration before native path
resolution. Private OsString arguments remain literal; non-UTF-8 public opt-in
refuses without echo. Worker context uses bounded strict internal CLI arguments
before `--`, with public facts only. The complete closed worker environment
is unchanged: no context environment variable or private source locator is added.
Runtime Git helpers subscribe before launch, use closed private maps and join
the current group/wait/drain cleanup owner. Build-time Git also clears its map
and bounds capture. Neither source-comparison fingerprints nor caller HOME,
PWD, environment, private paths or private argv hashes are evidence.

Every trial/matrix leaf is exclusive 0700; every payload is written completely
to a descriptor-private 0600 temporary before no-clobber final publication.
Caller-owned parent modes and unowned names/bytes are preserved. Metadata is
last. Namespace isolation remains #221; macOS eval proof is owned by #201.
Current bundles and 0058 select delivery/review routing; this port accepts no
proposed decision and imports no historical review routing.

## Tracked documentation boundary (#243)

Run `cargo test --manifest-path evals/Cargo.toml --locked --test boundary`
from a Git checkout. The boundary suite inventories every tracked Markdown file
with Git and refuses local link destinations into the repository's `.forge/`,
even when a delivery packet exists locally. Ordinary path mentions are allowed.
The check covers inline links/images and reference definitions, angle destinations,
optional titles, escaped delimiters and balanced parentheses; it excludes inline
code and backtick/tilde fenced code. It resolves dot components and percent escapes
lexically, without depending on target existence. Raw HTML and renderer extensions
are outside this bounded Markdown check. Git discovery, inventory and document
read failures refuse. Root tests do not run this separate eval workspace; #201 wired its CI
integration (the `test` jobs and the verify seat run this suite).

## Trial isolation — prepared, not activated (#221, proposed 0115)

Date: 2026-11-01 (the boundary is bubblewrap; the 2026-10-31 raw-namespace
worker is deleted; revised after the first review and again after the first
live proof). **Trials remain
sequential.** Accepted 0076 still admits one trial per uid and refuses
`--concurrency` other than 1; nothing here changes a row, a refusal or a
guarantee, and no row claims isolation (`confinement` stays `unconfined`).
Same-uid separation by fresh `0700` HOME directories is hygiene, not a
boundary.

A hidden `trial --isolated` flag (not in `--help`) runs the whole trial —
runner, Swarmail, fixture and every agent — inside one bubblewrap sandbox
(user, PID, network and IPC namespaces; a private `/tmp` of 512 MiB, HOME and
`/proc`; a cleared environment; descriptors above 2 closed), or refuses by
name before any server or agent exists. There is no fallback to an unisolated
run. The controller (the `swarmail-evals` you started) stays in the host
namespaces, holds the host admission and invokes `/usr/bin/bwrap` with a fixed
argv; the same binary runs the unchanged trial arm inside. The argv carries
`--as-pid-1`: bwrap installs no reaper, so the runner itself is PID 1 of the
trial's namespace and every process inside is the runner or something it
spawned. bwrap's default reaper would be a same-uid, dumpable process inside
the boundary that a sibling trial's agent could probe through `/proc/1`.
Accepted costs: a `SIGTERM` that reaches the runner before it has installed
its signal handler is dropped, and that trial ends at its deadline; orphans
inside stay zombies until the runner exits and the kernel empties the
namespace.

**Host prerequisite, not a dependency.** bubblewrap (LGPL-2.0-or-later) is
executed as a separate process and never linked, vendored or added to a
lockfile. An isolated trial needs Linux; `/usr/bin/bwrap` of at least 0.11.1
(an absolute path with no lookup and no override; the floor is confirmed for
the launch on 0.11.1 by the proof command's first run on 2026-11-01, which
passed the probe, accepted the full argv and ran the trial inside, and then
failed in the controller, which is repaired below; none of the live criteria
has yet passed; stock Ubuntu 24.04
packages bubblewrap 0.9.0, which the floor refuses until a proof lowers it, so
a hosted runner that installs it with apt cannot run the boundary); and a host
policy that lets bwrap create the namespaces (Ubuntu's `bwrap-userns-restrict`
AppArmor profile, or `kernel.apparmor_restrict_unprivileged_userns` lowered).
macOS refuses by name, since bwrap is Linux-only: macOS compiles this suite
and proves only that refusal.

The trial refuses, as a harness-error row and exit 2, with, in this order:
`concurrency_unsupported`, `isolation_requires_swarmail_bin`,
`isolation_artifacts_unsupported`, `isolation_wrapper_unsupported`,
`isolation_expose_invalid`, `isolation_external_agent_unsupported`,
`isolation_source_invalid`; then, from the controller, the closed set
`boundary_unavailable:platform`, `boundary_unavailable:bwrap_absent`,
`boundary_unavailable:bwrap_version` and `boundary_unavailable:bwrap_probe`
(the exact production argv with `/usr/bin/true` did not exit 0 within 10 s;
bwrap's own words, such as `ENOSPC`, go to stderr and never into the row).
After a launch, a runner that produced no row is `isolated_trial_no_row`, and
an interrupt before any runner existed is `isolated_trial_interrupted`.

**The controller hands over the row and exits.** The first live run showed the
boundary working and the controller losing the runner's row and then never
exiting: it held the process's standard-output lock across the whole trial
while copying the sandbox's output from another thread, which parked on it.
The controller now runs on a current-thread runtime, copies the sandbox's
output on the thread that drives it, holds no standard-stream lock across the
trial, counts the row as it is read, and returns once its row is written. Every
wait it makes is bounded: bwrap's version (5 s) and probe (10 s), bwrap's own
exit (the trial's deadline plus the matrix's 300 s grace; on expiry bwrap is
killed, the namespace empties with it, one fixed line goes to stderr and the
result is the usual `isolated_trial_no_row` or exit 2 beside the runner's
row), the pipe copies and the reap (5 s each) and the runtime's shutdown (5 s).
A reap the controller cannot prove within its bound is the named
`swarmail-evals: harness error: trial_cleanup_unproved` line on stderr beside
the envelope line, never an ordinary no-row: the host admission is held to the
controller's exit, the exit code is 2, and an operator confirms cleanup before
resuming (accepted 0076 §3; the lock is process-scoped, so that exit releases
it).

The network is a private loopback; provider egress does not exist inside, and
`--share-net` is rejected. Provider egress is an open operator question in
0115, so no real driver runs isolated: a wrapper profile is refused, and an
external agent program is refused unless it is one of the files named with
`--isolated-expose <absolute regular file>` (the live proof's scripted agents
are). The builtins `oracle` and `null` need no exposure. **An isolated row is
a boundary-proof row, not an eval result**; the matrix never renders
`--isolated`.

Delivery seats cannot create namespaces (`/usr/bin/bwrap` is present there and
refuses with `ENOSPC`), so the live path is proven only by the ignored suite,
on a host whose bwrap can create them. This is the one documented command; it
is deterministic (null and oracle built-ins and scripted probes from the test
executable), free, and bounded: every isolated trial carries a deadline of at
most 30 s, every child a test spawns that may run a boundary is held to 90 s
(output on files; `SIGKILL` and a named `controller outlived its row` failure
at the bound), and the dispatcher runs the command under a hard 20-minute
limit. Its green on a host whose bwrap works is the acceptance gate of the pull
request:

```bash
cargo build --locked --bin swarmail
cd evals && SWARMAIL_BIN="$PWD/../target/debug/swarmail" \
  cargo test --locked --test trial_isolation -- --ignored --nocapture --test-threads=1
```

Where the boundary cannot be built each of those tests prints
`SKIP trial-isolation: boundary_unavailable:<step>` and fails; it never passes
without the boundary, and the gate inventory never passes `--ignored`. The
always-on suite follows the other rule: its one boundary test,
`an_isolated_null_trial_hands_over_its_row_and_exits`, prints the same SKIP
line and returns where the boundary cannot be built, and where it can, runs a
real isolated null trial through the CLI and passes only by the controller's
actual exit with the runner's own row; the two tests whose subject is the
absent boundary print `SKIP trial-isolation: boundary available` and return
there. The whole `cargo test --locked` therefore finishes in bounded time on a
host where bwrap works as well as in a seat where it does not. That
command alone proves the hostile same-uid agent in trial B cannot read A's
prompt, reach A's MCP or fixture, or make A's idle agent pass (AC-L1), the
topology from inside (AC-L2: the runner is PID 1, no host path under `/home`
or elsewhere, no descriptor of any process reaching the host `/proc`; it also
prints, without asserting, whether a nested user namespace is granted,
whether `/proc/1/mem` opens read-write, and the host's
`kernel.yama.ptrace_scope`) and the lifetime bounds (AC-L3). CI proves only
the always-on tests on Linux and macOS and, on macOS, the `platform` refusal;
whether a hosted ubuntu runner has a usable bubblewrap is unobserved. Provider
egress (AC-L4) and the operator's ruling on 0115 remain outstanding
(specs/trial-isolation/spec.md).
