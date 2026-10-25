# Issue #151 — local baseline run plan

Updated 2026-10-25 under the operator's 2026-10-24 #199 ruling and
[proposed 0076](../docs/decisions/0076-sequential-eval-trials.md).
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

Before measurement, run the standalone fmt, locked clippy and full test suite
in [README](README.md). All sixteen oracle cases must pass and all sixteen
null cases must fail with their declared idle class. The held-idle-victim
control must refuse a competing trial before agent launch; the victim fails
`gave_up`, with no accepted completion event, and a later oracle passes.

Build the root and eval binaries offline/locked as in README, then run this
local control matrix (the null failures intentionally make matrix exit 1):

```bash
E=evals/target/debug/swarmail-evals
cat > target/issue151-agents.json <<'EOF'
[{"label":"oracle","argv":[]},{"label":"null","argv":[]}]
EOF
$E matrix --tasks signup,otp --agents target/issue151-agents.json --trials 1 --concurrency 1 > target/issue151-controls.jsonl
$E report --input target/issue151-controls.jsonl
```

For a later operator-owned baseline, select explicit trusted agent wrappers
and fixed repetitions, keep concurrency 1, and preserve every row and report.
No retries or omitted harness errors: attempts count requested slots, including
unstarted refusals, and success estimates use pass+fail. Report also shows
passes/attempts. Schema-2 readers refuse absent, noninteger or non-1 concurrency
and schema-1 rows; catalogue/snapshot versions and scoring stay unchanged.
Every row remains `confinement: "unconfined"`. No paid agent or model result is
claimed by this implementation. Operator sealing, CI/network proofs and #201's
macOS eval execution are separate ownership, not locally proven here.
