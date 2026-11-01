# Issue #151 — local baseline run plan

Updated 2026-10-25 under the operator's 2026-10-24 #199 ruling and
[accepted 0076](../docs/decisions/0076-sequential-eval-trials.md).
This is the local replacement plan, not a remote issue edit or measurement.

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

The 2026-10-26 approved product is recorded in [intake](issue151-intake.md#authority-and-evidence).
It is every current v2 catalogue task, two exact models per family, five paid
trials per cell and concurrency one. The six credential-free declarations are
[agents-baseline.json](agents-baseline.json). Currently there are 16 tasks,
32 free controls (one oracle and one null per task), and 480 requested paid
slots (16 × 6 × 5). Inbox-isolation has three simultaneous agents within its
slot; slots are not agent session counts.

## Current readiness and operator command

The launching wrappers use the versioned evidence and the 2026-10-26 ruling.
[wrappers.md](wrappers.md) defines private selection/auth schemas and the
configured asymmetry. First authenticated smoke for each of the six approved
models and live HTTP MCP, especially dsh, remains operator-owned and outstanding.
Delivery runs only offline stand-ins. Do not start the full baseline before
that smoke; baseline publication and reprioritization remain separate work.

Build offline and locked from the repository root:

```bash
cargo build --offline --locked --bin swarmail
cargo build --offline --locked --manifest-path evals/Cargo.toml
export SWARMAIL_BIN="$PWD/target/debug/swarmail"
# Adjust both artifact paths if CARGO_TARGET_DIR is selected.
E="$PWD/evals/target/debug/swarmail-evals"
$E baseline --config /private/wrappers.json --output /tmp/baseline-new --dry-run
# Only the operator executes without --dry-run, after the six-model authenticated smoke:
$E baseline --config /private/wrappers.json --output /tmp/baseline-new
```

The dry-run prints the actual ordered tasks and six models, control/paid
repetitions and cardinalities, fixed deadline and concurrency, output layout,
ordered validation/measurement/report steps, report argv and wrapper evidence
boundaries. It returns before opening the config, creating output, preparing
Swarmail, launching trials/wrappers or contacting services. The config path is
an absolute private locator, not printed in the plan. The closed selected-file schemas are documented in wrappers.md. Public output
paths must contain no secrets. Set up private CLI authentication through those
selected files, never credentials in model argv.

Per-trial agent deadline: **600000 ms (10 minutes)**, explicitly used for both
control and paid products. This operational allowance gives real models time
for CLI startup and several MCP/HTTP turns beyond the fixture's 60000 ms
default, within its existing one-hour maximum. It is not measured latency.
480 sequential paid slots have 80 hours of nominal agent budgets, excluding
provisioning/cleanup; wall time can be longer. Isolation's agents share the
slot deadline. The existing runner enforces it and retains TERM/KILL/reap/drain
ownership; no shorter competing parent kill timer is added.

## Admission, retention and statuses

Execution exclusively creates the requested output leaf at 0700 under an
existing non-symlink parent. Existing or partial outputs refuse unchanged.
controls.jsonl and paid.jsonl are streamed as rows commit, at 0600. Keep the
whole output tree, including incomplete bytes and absent completion markers.
No automatic retry, overwrite, row filtering, resume, archive conversion or
fabricated suffix is performed by the coordinator.

1. Run the fresh free oracle/null matrix over every catalogue task, in
   task → agent → repetition order, retaining controls.jsonl and controls/.
2. Require strict complete v2 rows bound to this invocation's prelaunch plan,
   unique cell/trial identities, catalogue/task definitions, declared builtins,
   harness/target identity and effective limits. Every oracle must pass with no
   class; every null must fail with that task's idle_class. Expected matrix
   exit is 1, since null intentionally fails. Exit alone never admits paid work.
   Also verify retained matrix/trial bindings against the streamed rows.
3. Refuse any missing, duplicate, extra, malformed, foreign, wrong-class or
   infrastructure row, mismatching limit, unexpected exit or interruption.
   Retain controls but never launch paid work on refusal. After valid controls,
   validate the credential-free private selection before admitting paid slots;
   this opens no credential file.
4. The command's paid branch uses the
   fixed six declarations, five repetitions, same target, concurrency and
   deadline. Preserve every emitted paid row, including harness errors and
   the matrix owner's unstarted refusal suffix, in paid.jsonl and paid/.
5. Strictly parse paid rows and validate complete accounting without requiring
   model success. Generate report.json with the existing report logic even
   after paid matrix exit 1/2 or incomplete accounting if every row parses.
   Truncated/malformed streams remain untouched and refuse reporting.

Output layout: controls.jsonl, controls/<matrix-id>/ (prelaunch matrix-plan.json,
trials/<trial-id>/ and final matrix.json); paid.jsonl, paid/<matrix-id>/ with the
same retention structure; report.json. Files are synthetic fixture evidence,
not provider logs. The command discards raw child output through the current
runner. Complete artifacts are detectable by their bound metadata/index; this
is not fsync/power-loss durability or historical authentication.

Exit 0 means complete all-pass paid results and a valid report. Exit 1 means
complete measured business failures with a valid report. Exit 2 means an
operational/admission/retention/interruption/accounting/report failure and wins
over report success. The current missing-contract refusal is exit 2 and is not
a model measurement. Reports count every requested row as an attempt, score
success over pass+fail, and also show passes/attempts; infrastructure is explicit.
No failure or infrastructure row is removed from the denominator. An incomplete
diagnostic report is not a completed baseline. Retain every row and report.

Before any operator-owned measurement run standalone fmt, locked clippy and
full tests from [README](README.md), including `--test wrappers`, `--test baseline`
and `--test proof`. All sixteen oracle cases must pass and all sixteen null
cases must fail with their declared idle class. The held-idle-victim control
must refuse a competing trial before agent launch; its victim fails gave_up,
without an accepted completion event, and a later oracle passes.

Every row stays `confinement: "unconfined"`. Current schemas, scoring and report
arithmetic remain unchanged. #243 owns tracked authority repairs outside evals;
the known broken authority link in specs/agent-eval-harness/spec.md (and the
same link in accepted 0055) remains outside this change's permitted scope and
must not be hidden by weakening the boundary test.

The operator owns CLI/provider interoperability, paid execution, the actual
baseline publication with token/transcript/taxonomy evidence, epic
reprioritization, sealing, live settings and decision acceptance. CI owns the
house charter's network/chart/Docker/port-80/macOS root proofs; #201 owns macOS
eval execution. #152/#153 PRs stay unmerged until the baseline exists;
#154–#160 await reprioritization. No paid/networked agent is run by delivery.

## Trial isolation — prepared, not activated (#221, proposed 0115)

Date: 2026-11-01 (the boundary is bubblewrap; the 2026-10-31 raw-namespace
worker is deleted). **Trials remain sequential.** Accepted 0076 still admits one
trial per uid and refuses `--concurrency` other than 1; nothing here changes a
row, a refusal or a guarantee, and no row claims isolation (`confinement`
stays `unconfined`). Same-uid separation by fresh `0700` HOME directories is
hygiene, not a boundary.

A hidden `trial --isolated` flag (not in `--help`) runs the whole trial —
runner, Swarmail, fixture and every agent — inside one bubblewrap sandbox
(user, PID, network and IPC namespaces; a private `/tmp` of 512 MiB, HOME and
`/proc`; a cleared environment; descriptors above 2 closed), or refuses by
name before any server or agent exists. There is no fallback to an unisolated
run. The controller (the `swarmail-evals` you started) stays in the host
namespaces, holds the host admission and invokes `/usr/bin/bwrap` with a fixed
argv; the same binary runs the unchanged trial arm inside.

**Host prerequisite, not a dependency.** bubblewrap (LGPL-2.0-or-later) is
executed as a separate process and never linked, vendored or added to a
lockfile. An isolated trial needs Linux; `/usr/bin/bwrap` of at least 0.11.1
(an absolute path with no lookup and no override; Ubuntu 24.04's packaged
0.9.0 is refused until a proof lowers the floor); and a host policy that lets
bwrap create the namespaces (Ubuntu's `bwrap-userns-restrict` AppArmor
profile, or `kernel.apparmor_restrict_unprivileged_userns` lowered). macOS
refuses by name, since bwrap is Linux-only.

The trial refuses, as a harness-error row and exit 2, with:
`concurrency_unsupported` (first), `isolation_requires_swarmail_bin`,
`isolation_artifacts_unsupported`, `isolation_wrapper_unsupported` (provider
egress is an open operator question in 0115, so no real driver runs
isolated), `isolation_expose_invalid`, `isolation_source_invalid`; then, from
the controller, the closed set `boundary_unavailable:platform`,
`boundary_unavailable:bwrap_absent`, `boundary_unavailable:bwrap_version` and
`boundary_unavailable:bwrap_probe` (the exact production argv with
`/usr/bin/true` did not exit 0 within 10 s; bwrap's own words, such as
`ENOSPC`, go to stderr and never into the row). After a launch, a runner that
produced no row is `isolated_trial_no_row`, and an interrupt before any runner
existed is `isolated_trial_interrupted`. The network is a private loopback;
provider egress does not exist inside, and `--share-net` is rejected.
`--isolated-expose <absolute regular file>` binds one more read-only file, for
a test executable.

Delivery seats cannot create namespaces (`/usr/bin/bwrap` is present there and
refuses with `ENOSPC`), so the live path is proven only by the ignored suite,
on a host whose bwrap can create them. This is the one documented command; it
is deterministic (null and oracle built-ins, `/usr/bin/sleep`, scripted
probes), free, and bounded (every isolated trial carries a deadline of at most
30 s):

```bash
cargo build --locked --bin swarmail
cd evals && SWARMAIL_BIN="$PWD/../target/debug/swarmail" \
  cargo test --locked --test trial_isolation -- --ignored --nocapture --test-threads=1
```

Where the boundary cannot be built each of those tests prints
`SKIP trial-isolation: boundary_unavailable:<step>` and fails; it never passes
without the boundary, and the gate inventory never passes `--ignored`. That
command alone proves the hostile same-uid agent in trial B cannot read A's
prompt, reach A's MCP or fixture, or make A's idle agent pass (AC-L1), the
topology from inside (AC-L2) and the lifetime bounds (AC-L3). CI proves only
the always-on tests and, on macOS, the `platform` refusal; whether a hosted
ubuntu runner has a usable bubblewrap is unobserved. Provider egress (AC-L4)
and the operator's ruling on 0115 remain outstanding
(specs/trial-isolation/spec.md).
