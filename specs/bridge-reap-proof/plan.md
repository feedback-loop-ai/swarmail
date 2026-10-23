# Implementation plan: bridge reap proof

Implements [spec.md](spec.md); ordered proof work is in [tasks.md](tasks.md).
The repository has no `.specify/templates/plan-template.md`; this plan
includes the required technical context, constitution check, project
structure, research, data model, contracts, quickstart and risks here.

## Technical context

Single Rust crate; existing Unix-specific integration test in the binary
harness. `tests/binary.rs` imports `tokio::process::Child`; Cargo.lock pins
Tokio 1.53.2. Existing libc supplies SIGKILL and ExitStatusExt supplies the
signal observation. No new dependency, helper abstraction or platform API.

## Constitution check

Accepted 0020 permits this integration harness's cold diagnostics and
requires observable synchronization. Preserve every frozen guarantee and
accepted 0001–0005 behavior. Accepted 0050 requires actual execution and
pinned proof ownership; 0056 requires unsigned commits. Intake already
selected the full engine route under 0057 because this is a lifecycle and
concurrency proof, irrespective of the small diff. No new semantic ADR.

## Project structure and files touched

| Path | Role |
| --- | --- |
| `tests/binary/helper_reliability.rs` | Only the existing bridge timeout regression's lifecycle evidence and local failure cleanup; temporary mutants are confined here. |
| `specs/bridge-reap-proof/{spec,plan,tasks}.md` | This design and directly necessary dated #179 evidence/clarifications. |
| `tests/binary.rs` | Read-only owner of `McpChild`, `finish_with_deadline`, `kill_and_reap`. |
| `Cargo.lock` and installed Tokio source | Read-only semantic evidence. |

No other source/note files are authorized. Keep raw execution receipts in
ignored run-local storage and summarize them in these notes if needed.

## Research and source evidence

Chief independently inspected installed Tokio 1.53.2:

- `src/process/mod.rs:1216–1226`: `id()` reads `FusedChild`; it returns
  None only for Done and does not poll/reap. Documentation explicitly
  names avoidance of reused-PID confusion.
- `src/process/mod.rs:1379–1394`: a successful `wait()` caches exit status
  by transitioning to Done.
- `src/process/mod.rs:1398–1427`: Unix `try_wait()` can reap and transition
  a live handle; the Done arm simply returns the stored status.
- `src/process/unix/mod.rs:164–165`: live `try_wait` delegates to the owned
  std child. Do not generalize Tokio's optional id to `std::process::Child`.

In `tests/binary.rs`, `kill_and_reap` requests kill and awaits bounded
`child.wait()`. `finish_with_deadline` awaits that cleanup on failure before
returning its diagnostic. Fixtures flush readiness, optionally close
stdout, then park indefinitely without descendants. The initial live
`try_wait` and Pending poll do not finish the owned handle. Only completed
helper polling can leave it Done before the new observation. OS signal
arrival alone cannot change the outer handle to Done.

Baseline SHA-256 (chief inspection, not execution proof):

| Input | SHA-256 |
| --- | --- |
| `tests/binary/helper_reliability.rs` | `2fdb0a3fd1cba70580cfd93b900cfeb121bfd704df9e836a2b3ff1844515cf69` |
| `tests/binary.rs` | `7da125cd5945bfc7f511c0c6fdaed52eb8baeb08497c5416ab29ede867752554` |
| `.forge/bin/brokkr` | `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c` |

## Data model and contracts

No public or persisted data changes. Local evidence consists of the finish
Result, a boolean completion snapshot, cached ExitStatus and diagnostic
text. The same Child owns the process throughout. The boolean records
whether the helper completed its obligation before the observer intervened;
it must never be recomputed after fallback cleanup to decide success.

Implementation order inside each existing mode iteration:

1. Preserve readiness, closed-mode EOF, initial live status and Pending
   operation choreography. Delete saved `pid`.
2. Keep `let result = bridge.finish_with_deadline(Duration::ZERO).await`
   without `unwrap_err` until lifecycle validation has occurred.
3. Immediately capture `let completed = bridge.child.id().is_none()`.
   If false, await `kill_and_reap(&mut bridge.child)` and then panic with
   AC4's named lifecycle text, mode and cleanup result. No status read may
   precede this capture. Explicitly preserve the failed fact.
4. On the true branch only, unwrap the expected error, retain both timeout
   and successful-cleanup diagnostic assertions, obtain cached status and
   require SIGKILL. Remove the obsolete later id check and raw waitpid
   block. Add the short explanatory comment beside the first observation.

The status check cannot reap on the passing path because the handle was
already Done. A live child and a killed-but-unobserved child both fail the
same completion check. This is proof of helper completion, not an assertion
that the missing-reap mutant necessarily reached a kernel zombie state.

## Negative controls and restoration

Save the implemented green file bytes and hash before each mutant. Replace
only the final finish invocation in this test, leaving its new observation
and cleanup intact. Supply an explicitly typed synthetic Err with diagnostic
strings that would otherwise satisfy existing checks; type it as the actual
finish Result if inference requires it. Synthetic text exists only in the
mutant and is labeled mutation-only.

| Mutant | Replacement action | Required failure |
| --- | --- | --- |
| `bridge_timeout_returns_without_reaping` | `start_kill` on this Child, no wait/try_wait, then synthetic result. | Completion is false; AC4 diagnostic after successful bounded cleanup. |
| `bridge_timeout_returns_with_live_child` | No kill or wait, then synthetic result. | Same lifecycle diagnostic and cleanup while fixture was left live. |

For each mutant run bridge-open and bridge-closed separately by temporarily
selecting one existing loop mode. A panic on the first loop iteration must
not count as proof of the second. Neither control changes helper definitions
or adds a permanent environment switch/mutation framework. Record four
nonzero test executions that reached the intended diagnostic, not compiler
failures or string mismatches. Record cleanup success and source restoration
hashes; restore the full two-mode loop and exact saved bytes after every
experiment. A final diff must contain no mutant, synthetic error or selector.

## Quickstart and verification

After implementation and exact restoration:

```bash
cargo test --locked --test binary helper_reliability::never_exiting_bridge_child_is_timed_out_killed_and_reaped -- --exact --nocapture
cargo test --locked --test binary helper_reliability
```

Require actual nonzero selected test counts; the deliberately ignored
fixture is launched by its parent, not separately counted as a passing
regression. Keep normal test concurrency; no PID-churn stress is necessary.

Then execute the entire authoritative
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
on this candidate: fmt, locked workspace clippy/tests, pinned compile,
exact coverage and selftest, publish-guard and keyless seal proofs, and
licenses. Assert `.forge/bin/brokkr` has the digest above and matches
`bundles/self/tooling.json` before using that exact binary for
`compile --bundle bundles/self`; version equality is insufficient.
Run fail-closed native-tls absence: Cargo tree command failure is failure,
not evidence of absence. Execute actual outer offline verify through the
pinned proof chain with checked shim digests, not a candidate-script-only
substitute. Preserve failed receipts as well as restored green evidence.
No prior candidate's verify receipt transfers.

The chief may preflight `bash bundles/self/scripts/design-check.sh check .`
after committing these three artifacts. The sequence's dispatched pinned
check owns the design verdict. Implementation, mutation proof, full gates,
outer verify and independent review remain subsequent delivery work.
CI-only checks in the charter remain root/CI obligations, not waived or
claimed here. Use the accepted four named reviewers; launch no alternate
route or nested delivery run. Root owns integration and issue closure.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Status observation repairs missing reap | Capture non-mutating id-none first; missing-reap mutant must fail before any test-side reap. |
| Reused PID names another child | Delete all saved-PID probing; use only the original owning handle. Accept dependence on Tokio's documented completion state. |
| Failure panics before cleanup | Retain Result, capture state, bounded same-handle cleanup, then named failure; keep existing kill-on-drop fallback. Cleanup errors remain visible failures. |
| Mutant red proves only a string mismatch | Lifecycle guard precedes diagnostic asserts; require named assertion and cleanup receipt for each mode and mutant. |
| Early panic hides the closed-stdout mode | Separate mode-specific negative executions; restore the ordinary two-mode test. |
| Temporary mutation leaks into final candidate | Save exact bytes/hash, restore after each control, compare hashes and diff before final green tests/gates. |
| Tokio upgrade changes the proof assumptions | Keep source/contract rationale in test note; recheck id/wait/try_wait semantics with any future Tokio change. No new dependency pin is needed here. |
| Scope or evidence expands beyond this residual | Diff against 86c851f, preserve reviewed helpers and unrelated tests byte-for-byte; label historical/design/implementation/verify evidence distinctly. |
