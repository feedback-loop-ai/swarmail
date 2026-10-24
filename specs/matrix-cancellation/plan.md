# Residual repair plan — matrix cancellation

Implements [spec.md](spec.md), preserving existing proposed
[0062](../../docs/decisions/0062-matrix-cancellation.md). This chief commits
only the three design artifacts. Later implementation starts with falsifiers
and changes runtime code only for a reproduced recorded contract violation.

## Technical context

Rust 2024 standalone `evals/` workspace; existing Tokio signals/process/I/O,
libc, serde and tempfile. One sequential producer, one active trial, retained
waits and per-group cleanup owners/reapers. No dependency, feature, manifest
or lock change. Accepted 0055 requires separate evals verification; root exact
coverage does not measure evals. No plan template exists in this checkout;
this plan supplies the required dialect sections directly.

Baseline: `29a661590fdca79ab77b6ab5f2f872ed1a98bb4e`, including completed
`137e7e9`, merged main and accepted 0058. The chief commit creates a successor;
record actual revision/tree for every receipt. Current local engine digest is
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, matching
`bundles/self/tooling.json`. Resolve `.forge/bin/brokkr` absolutely and check
its hash before use. No install or pin edit; historical e525babf proof remains
historical. This engine-class repair requires five distinct 0058 results.

## Constitution check

Preserve 0001–0005, AUTH, 0055 lifecycle/schema/report contracts, 0020 test
placement/readiness and all house gates. Proposed 0062 is not accepted;
0045 §8 permits only compatible implementation. No new semantic ruling or
ADR is needed. A discovered conflict returns for the exact operator ruling;
frozen guarantees cannot weaken. Root source, gates, policy/recipes,
accepted decisions, dependencies, coverage exclusions and journals are outside
scope. Commit unsigned under 0056; never push, merge or close the issue.

## Research and evidence

Current intake and all four retained 3e202f8a member results delimit the
repair. Correctness/tests found missing falsifiers, not a demonstrated runtime
defect; security/guarantees were clean. Exact seq 714 aggregate binding is
unavailable in the box. Record this limit rather than inventing history.

Source inspection confirms both actual-Pending/bypass fences exist now.
`pending_matrix_signal` can be bypassed by tests whose hold consumes a signal;
new controls must leave it queued for the real check. `write_row` has one
sender and sequential acknowledgement; its Full branch needs a reachability
argument distinct from blocked stdout. `run_child` independently waits/reaps
and times post-exit EOF. `cleanup_once` creates absolute deadlines after its
existing hold, so repeats in that hold alone prove no deadline property.
Current guard Drop can merely log rescue/survivors, requiring checked finish.

Retain dated historical clarifications in spec.md. Supersede the inaccurate
`.forge/implement-194/t1-map.md` output-coverage claim in a new dated run-local
map, without rewriting that receipt. New T1–T10 track residual work, not a
claim that previous unchecked implementation tasks were all complete.

## Project structure and touched files

| Path | Authorized work |
| --- | --- |
| `specs/matrix-cancellation/{spec,plan,tasks}.md` | Exact chief commit scope; current scope, rulings and paired proof obligations. |
| `evals/src/main.rs` | Owning cfg(test) admission/pending/forward/output probes, fallback count, MatrixProbe checked completion; runtime repair only after failing falsifier. |
| `evals/src/runner.rs` | Owning cleanup-instance/deadline observations, error/expiry probes and checked Probe teardown; preserve actual-Pending/bypass tests. |
| `evals/tests/report.rs` | Ordinary PID-only CLI regression, byte prefix/exact readiness, captured EPIPE diagnostic, local MatrixGuard completion checks. |
| `evals/tests/runner.rs`, `evals/tests/common/mod.rs` | Reuse real lifecycle helpers and isolation; edit only as necessary for residual proof. |
| `evals/README.md` | Correct only an actual divergence in documented output/cleanup limits. |

Read TrialResult/refused_row and `evals/src/report.rs` as unchanged consumers.
Run `evals/tests/proof.rs`; no scoring/oracle rewrite. Existing 0062 and index
remain unchanged. Default to owning inline cfg(test); any sibling cold-path
harness needs a concrete 0020 reason, not denominator convenience. Binary
cfg(test) does not enable library cfg(test): test each private owner within
its own unit compilation and retain separate ordinary CLI regressions.

## Data model and contracts

No new persistent model or interface. Keep schema-1 TrialResult and immutable
queued bytes. Matrix retains sticky stop, ordered slot, aggregate and one
active child; runner retains registered identities, shared cleanup results and
direct reapers beyond aborted admission. Running → stopping → cleaning →
accounting → finished is descriptive, not a new framework or enum requirement.

Subscribe before owned spawn; join aborted admission before cleanup; retain
concurrently issued children. Forward TERM once and await the owner. Clean
groups concurrently under fixed five-second TERM plus five-second cessation/
reap bounds, never renewed by signals or multiplied serially. Each group can
have its own start timestamp; prove fixed per-owner deadlines and concurrent
progress, not mathematically identical start times. Linux observations prove
identity and live state; other Unix remains conservative. Unknown is failure.

Queue enqueue commits the row after reap/validation/pending-signal decision;
acknowledgement proves write/flush, not crash durability. Preserve prefix bytes,
truthful active harness_error with measured elapsed lifetime/observed statuses,
fresh unknown fallback if no usable row, and ordered unique zero-time refusal
suffix. Preserve normal 2/1/0 precedence and report formulas. Output retention
stays 64 KiB, output ack and post-child-exit EOF bounds five seconds. Complete
JSONL needs progressing stdout; process cleanup and output completion are
separate facts. No universal wall-time or escaped-command confinement claim.

## Implementation approach

1. Map each review finding to existing tests and missing controls, record
   revision/digest, and rerun both causal fences plus ordinary CLI regression.
   Retain passing proof; do not repeat predecessor readiness/ownership fixes.
2. Add local acknowledged barriers immediately before the real pending checks
   and around synchronous spawn issuance. Release with a pending signal to
   exercise the actual decision, not a pre-latched bool. Prove both issuance
   outcomes and unstarted suffix. Bracket attempted-child timing with events
   and inspect fresh fallback identity/unknown statuses; never guess a minimum
   sleep or substitute trial status for agent status.
3. Extend live cleanup controls with instance/deadline observations after
   budget creation and actual Pending. Send TERM, acknowledge, INT,
   acknowledge, TERM, acknowledge; standard signals may coalesce otherwise.
   Observe forwarding count, retained owner and fixed deadlines, concurrent
   groups, direct reaps and cessation. Keep existing bypass controls intact.
4. Exercise real stdout pressure with an open unread pipe and enough bounded
   rows to exceed its capacity. A local writer observation may acknowledge
   the actual blocked operation; timeout exit/diagnostic supplies the failure
   proof. Keep signal delivery observable and deadlines absolute. Separately
   retain a real descriptor holder after its parent exits, prove parent reap,
   EOF timeout/diagnostic and test-owned holder cleanup. Use existing executable
   override for actual spawn failure without invalidating Swarmail resolution.
   Capture EPIPE stderr, preserve validators and count fallback invocations.
5. Inventory every writer sender/caller/return. Sequential successful ack
   occurs after dequeue, so the next supported call should not find Full;
   failed calls must not create an overlapping producer. Document the complete
   argument. If useful, a narrow channel-state test covers Full's local signal
   branches but cannot claim a supported end-to-end race. Do not remove Full/
   Notify or enlarge queueing without a reproduced in-scope runtime defect.
6. Inject observation error and forced deadline expiry at the owning cfg(test)
   boundary with real processes retained. Require propagated harness failure
   and diagnostic, then safely rescue test-owned work. Do not weaken identity
   validation, alter global /proc or build a mock supervisor. Ordinary dev and
   release isolation controls must cover every added environment seam.
7. Add local checked finish results recording rescue, survivors and direct
   wait status. Positive tests assert no rescue and no unresolved ownership.
   Exercise forced failure inside a child probe whose parent requires nonzero
   outcome. Drop stays bounded emergency cleanup during unwind, without a
   second panic. Retain early identity snapshots, 30-second ordinary completion
   and 15/5-second failure phases; never race the product with a shorter kill
   deadline. Bypass negative controls explicitly separate expected product
   failure from successful rescue. No generic guard framework is required.
8. Tighten byte prefix/readiness assertions, remove tautology, ingest through
   real report and reconcile evidence only after commands execute. If a new
   control fails an existing contract, retain that failure and make only its
   local repair before repeating affected proof. Broader discoveries follow
   the bounded engine route, never an unrecorded scope expansion.

## Test map and quickstart

From `evals/`, reproduce these retained named tests before edits:

```bash
cargo test --locked --test report matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup
cargo test --locked matrix_completion_fence_is_observed_and_its_negative_control_exits_early
cargo test --locked runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults
```

AC4 uses new pending/admission/fallback/repeated-signal controls (T2/T3).
AC5 uses observation-error/expiry controls (T5). AC6/7 use spawn/fallback count,
blocked stdout, inherited EOF and EPIPE controls (T4). AC9 uses isolated forced
failure and checked-finish controls (T6). Bind final exact test names and
revision/command/exit in run-local evidence; labels here are requirements,
not claims those tests already exist. AC1/2/3 retain the three commands above
and real report assertions (T7).

For AC8 run full report and runner tests, then `cargo test --locked` (includes
proof.rs oracle/null/isolation), `cargo fmt --all -- --check` and
`cargo clippy --all-targets --locked -- -D warnings` from `evals/`. Exercise
ordinary dev/release control-isolation binaries separately as needed. Use
real subprocesses/protocols, isolated tempdirs/environments and atomic ready/
release acknowledgements; polling observes those facts, not guessed delays.

For AC10 run every [house gate](../../docs/house-rules.md#gates--part-of-every-change),
using the absolute digest-matched `.forge/bin/brokkr` for compilation. Verify
pinned shim integrity and fail-closed native-tls absence. Hermetic verification
uses pinned implementations, including design-check selftest and delivery
boundary proof, and installs nothing. Retain exact coverage plus selftest and
publish/keyless-seal proofs. CI retains chart/history/selftest, Docker,
network advisory, tag-rate and port-80 obligations; local success cannot claim
those. Five separate accepted 0058 results remain mandatory; medium historical
severity and no security residual do not justify a cheaper review route.

Chief validates links/diff, commits exactly three artifacts unsigned and
checks structural design against that HEAD, leaving it the final commit.
Chief gate receipts do not close future implementation tasks. Run-local logs
and mandatory drafted result remain uncommitted; only the sequence's final
pinned check rules the design attempt.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Historical evidence mistaken for current proof | Preserve dated receipts, name missing seq binding, log actual revision/digest. |
| Tests consume the signal before the production check | Hold without receiving it; assert actual pending-decision branch. |
| Signals coalesce or repeats precede deadline creation | Per-delivery acknowledgements and post-creation owner/deadline observations. |
| False cleanup proof or serial budgets | Preserve actual-Pending/bypass controls; prove fixed per-owner budgets and concurrent cleanup. |
| Full-channel fixture mistaken for real pressure | Separate reachability inventory from real unread-pipe/ack-bound test; retain runtime path. |
| Inherited pipe leaks a process | Retain descriptor-holder identity/handle before assertions, reap parent separately and check teardown. |
| Guard rescue hides product failure | Checked finish and isolated nonzero forced-failure proof; bypass rescue is explicitly expected. |
| Recycled identities or observer errors | Revalidate supported membership; unknown fails, empty groups retire, no process-name kills. |
| New test controls leak into ordinary builds | Owner-local cfg(test), installing tests and ordinary dev/release isolation checks. |
| Longer failure tests and platform limits | Accept real five-second bounds; Linux proof explicit, conservative other-Unix behavior unchanged. |
| Overbroad runtime cleanup or semantic change | Failing residual control before runtime edit; no new protocol/schema/dependency or ruling. |
| Output completeness overpromised | Preserve O(product) suffix and progressing-output condition, distinct from cleanup. |
