# Implementation plan

## Technical context and research

Start with the supplied dispatcher observations and commit dd4f67a0. In `evals/tests/paid_baseline.rs`, `one_command_gates_on_controls_defers_stale_logins_and_resumes_by_slot` first kills the coordinator on the third codex-sol launch. Its second SIGKILL targets the trial worker during a held dashscope launch. In `evals/src/runner.rs`, spawn_extended places each service in its own process group. SIGKILL prevents the worker from reaching stop_servers or dropping its child handles. Releasing the CLI hold lets the stand-in exit. Running::finish observes only the coordinator. This explains the reported pair on inspection; require the isolated experiment below before recording the cause as verified.

## Panel ruling and project structure

Adopt simplicity's test-owned repair, existing launch barriers and two distinct death scenarios. Reject a production watchdog or a global test sweeper. Adopt robustness's private session for baseline commands that start trials and its independent end-of-invocation assertion. A single pre-kill descendant snapshot is cheaper but can miss an ordinary completion leak or a later child after reparenting. The extra cost is one session setup per tracked launch and a bounded number of process-table scans at lifecycle boundaries. Keep full-host inspection as ledger evidence. If isolation instead demonstrates a handled runner exit leaks, repair the existing Groups/Trial cleanup path in runner.rs and exercise that precise exit with real servers.

Use the launch and wait facilities in `evals/tests/common/paid.rs` for the real-trial invocations in paid_baseline.rs and `evals/tests/baseline.rs`. Keep the scope handle alive past Running::finish in the coordinator-death case. Extend the process helpers in `evals/tests/common/mod.rs` only where needed. Replace the paid test's private worker walk with the shared identity machinery. Dry runs and commands that only exercise argument parsing need no process census.

## Data model and lifecycle contracts

* Establish a fresh session in the child before exec. Record its leader identity. Use the existing ProcessIdentity fields, descendant discovery and survivor diagnostics. At each held launch verify the worker relationship, both expected executable/subcommand pairs and sibling role cwds under the same resolved scratch root. Pin identities across the kill and verify signal success. Keep the session discoverable through the surviving members even after the leader exits.
* On coordinator death, perform the original busy/refusal checks, release the stand-in and wait for normal worker completion. Check cessation before resuming. For ordinary completion, check immediately after the command has finished. At the worker kill, keep the no-closure and unconfirmed-refusal evidence, release the hold, then explicitly terminate the remaining owned family, including agent or adapter descendants. Use Linux pidfds through existing libc, verifying identity and session after opening and before signalling. Rescan for remaining session members. Wait on observable exit with one short monotonic deadline per cleanup, shared across members.
* Make the survivor assertion a separate observation, before confirmation or scratch removal. Capture its failure before best-effort bounded rescue on unwind. Install rescue ownership at launch so earlier assertions can fail safely. Wait on handles for direct children; terminal nonchildren have released their descriptors. Fail on inaccessible ownership evidence. Verify the exact abandoned scratch root from live role evidence, use artifacts::resolve_root before safe_root, and dispose of that root after cessation. Keep the run output and retained attempt leaf. The test supplies the existing confirmation argument only after these checks.

## Constitution check

This design applies accepted 0076 and 0137 within the standalone eval tests. It requires no new decision, dependency, public field or change to the frozen SMTP, store, await, eviction and extraction guarantees. Keep root coverage exact and follow the gate inventory in [the house rules](../../docs/house-rules.md#gates--part-of-every-change). Use offline oracle/null agents and local stand-ins. Keep locks, allowlists and held paths intact.

## Quickstart and evidence

Build the root candidate with Cargo's JSON artifact output and set SWARMAIL_BIN to the reported swarmail executable. From evals run `cargo test --locked --test baseline`, `cargo test --locked --test paid_baseline`, then the latter with the full function name and `-- --exact`. At each boundary record `ps -eo pid,ppid,pgid,sid,stat,args` and matching /proc cwd and start time. Attribute by launch ancestry/session and resolved trial path. Rescue verified survivors between runs. Time the identical filtered command before and after with a warm build. Repeat the census around `cargo test --locked --no-fail-fast` and record the mutation run separately.

## Risks

* Shared hosts can reuse PIDs or run another trial with the same command. Gate every signal on stable owned identity. A failed scan, missing expected live role or ambiguous scratch root fails the proof.
* The first coordinator exits before its worker by design. Check only after the held worker finishes. In the second case run the independent check even when explicit termination is bypassed, so rescue cannot erase a failing verdict.
* Avoid multiplying timeout costs or adding trials. Use held-launch events and process exits, return immediately on success, and measure the aggregate cost across affected tests.
* Place Linux cfg directly on each Linux-only item. Keep real-process and /proc tests annotated with the existing non-Linux ignore reason, and cfg-gate dependent references so macOS still compiles their bodies. Resolve symlinked temporary roots before validation.
