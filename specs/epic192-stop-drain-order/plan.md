# Plan — lifecycle ordering proofs

Implements [spec.md](spec.md); ordered proof obligations are in [tasks.md](tasks.md). This design commit contains only these three documents. Implementation and executed mutation receipts follow the engine's deterministic design check.

## Technical context and research

Single Rust crate; existing Tokio watch/oneshot channels and JoinSets, axum HTTP and real TCP suffice. No dependency, schema, CLI, timer or metric change. Inspection confirms correct current stop order and eager Unix signal polls; the defects are weak tests and overstated prose. Source locations and all six dispositions are recorded in the spec.

The repository has `.specify/templates/spec-template.md` but no plan-template.md. This plan supplies the requested technical context, constitution check, structure, research, data model, contracts and quickstart in these three authorized artifacts rather than adding template or ancillary files.

## Constitution check

0001–0005, 0023/0024 mutation ordering and refusal, 0027 runtime fences, 0029/0041 tracked drain and ledger limitations, 0042 writer lifetime, and 0050 proof/coverage requirements remain unchanged. Tests obey 0020: branch-stable private tests inline by default; only concrete cold failure paths justify a declared harness. No production code is hidden under a harness name. Both seam installation and absence must execute under the exact gate. Accepted bodies and all gates/shims/bundle definitions remain untouched. No ADR is required for preserving these semantics; only reserved proposed 0062 is available if a later design revision actually requires one.

## Project structure and ownership

| File | Planned change |
|---|---|
| `src/lib.rs` | Inline real-stop/tracker tests using existing private fields; qualify tracked-task comments. No new stop hook. |
| `src/api.rs` | Private test-only per-serve deadline witness at the actual deadline arm; directly supporting real-wire deadline and graceful-control tests. |
| `src/main.rs` | Small private production-used orchestration of eager Unix registration and startup, deterministic pending-probe tests. Preserve watcher and exit behavior. |
| `src/smtp.rs` | Drain guarantee comment only. |
| `tests/lifecycle.rs` | Correct held-lock/gauge/pause prose; replace relocated stalled-body implementation with its owner-test reference after proving parity. Retain other lifecycle/fan-out/refusal tests. |
| `tests/binary.rs` | Only serve/signal/drain-witness comments and directly necessary tests; preserve real child signal evidence and F's bridge/stdout/stdio sections byte-for-byte. |
| `src/shutdown_tests.rs`, `src/lifecycle_tests.rs`, or `src/main/*_tests.rs` | Conditional alternatives only for documented cold failure paths; declare `cfg(test)` from the owner. API-related cold scenarios may use the authorized root lifecycle harness with crate-private seam access. No automatic migration of existing inline modules. |

Store, persistence, webhook, POP3, test-common helpers, Cargo manifests/lock, decision index, recipes, gates and immutable baseline are context only. Evidence remains ignored/run-local. No sibling worktree or Git store is edited.

## Data model and contracts

No persisted model or public API changes. Test state consists of per-test entered/release oneshots, a subscribed drained receiver, a per-invocation deadline receipt, and two local first-poll records. Held tracker tasks model ownership completion, not protocol/storage stages. Signal probes substitute readiness inputs only; HTTP/SMTP/process tests still use real protocols and child processes. The new private startup helper is an implementation detail, with no public generic signal service.

## Implementation approach

### 1. Stop ordering through existing private trackers

Start a fresh `run_on` server on loopback ephemeral ports. In an inline lib test insert a held test-owned task into `server.tasks`; it acknowledges entry and awaits a oneshot release. Install an analogous held task in `server.dispatcher`, subscribe to `server.drained`, and retain shutdown observation if needed. Await entered acknowledgements before beginning the proof.

Pin `server.stop()` and explicitly poll it with `poll_fn` (return Ready from the poll wrapper after asserting the inner poll is Pending). With serve release withheld, the stop future has sent shutdown and cannot finish its serve joins; assert drained false. Release the serve task and continue driving the same future together with the drained receiver until drained is true. The dispatcher hold must still make stop Pending. Release it and await stop to completion. The actual dispatcher is joined by this same method. Never wait only on a drained receiver while leaving the stop future unpolled.

This checks the real method and its owned sets without a production hook or artificial post-stop mail. Existing fan-out and storage tests remain the public behavior evidence. Mutate only the real drained send above the serve loop; failure must be the pre-release drained assertion, not a compile failure or unrelated watchdog. Restore exact source bytes and rerun.

### 2. Deadline receipt and real stalled body

Emit a private `cfg(test)` one-shot receipt only inside `_ = drain =>`, preserving `Ok(())` and normal behavior. Bind it to one serve invocation. Use the existing owner-local consumed-hook precedent if needed: install in the tested task on a fresh current-thread runtime and consume at serve entry before any await, carrying the sender locally afterward. Do not leave a process-global last-event flag. Normal tests exercise the absent-hook path; stalled and graceful controls exercise installed paths. Clear any unconsumed installation during cleanup.

Relocate the existing stalled-body scenario to the owner-visible test target, retaining real bound TCP and server code. Read the full `100 Continue` head, not an assumed single TCP read. Keep the incomplete body socket alive while triggering the same shutdown watch read by `api::serve`; drive the actual serve future to completion and inspect its private receipt. Exactly one emission is enforced by the one-shot sender. Separately run normal graceful completion and establish no deadline receipt after its sender is dropped. Drop the stalled socket during cleanup; do not assert stop already closed it.

Prefer inline owner tests. If robust framing/timeout diagnostics need a failure-only branch, document it and use the allowed `src/lifecycle_tests.rs` harness instead; only the required private seam may become `pub(crate)`. Preserve the real ten-second fence and use generous watchdogs only to bound hung proofs. Remove the duplicate integration scenario only after the owner scenario covers its actual wire and liveness obligations; keep a source reference at the old section. No shared-helper hardening campaign.

Negative controls: remove the deadline-arm send and require the receipt assertion to fail; separately bypass the deadline arm with the retained socket and require the watchdog to fail. Save mutation diffs, commands/exits and restored-green output independently.

### 3. Registration before startup, through main's ordering owner

Extract the minimum private orchestration main actually calls, spanning eager registration and invocation of the startup continuation. Use an existing poll primitive and a small private adapter/closure for the two Unix sources. The production continuation owns watcher handoff/spawn and `run_on` invocation in the correct order; constructing an unpolled future must not count as invoking startup. Keep the registered streams alive for the existing first/second-signal watcher. Preserve startup failure handling, logs and non-Unix behavior.

The deterministic unit test supplies pending probes with local first-poll records and a continuation that inspects both records immediately on invocation before any yield. Background polling is withheld so the scheduler cannot repair a missing eager poll. Test the shared owner, not a standalone utility main could invoke too late. No actual process signals or shared global signal state enter this test. The existing child tests cover OS integration separately.

Mutate the shared path three ways, one at a time: remove SIGINT's eager poll; remove SIGTERM's; move registration after startup. Each unchanged unit test must reach and fail its ordering assertion. Restore and rerun green after each. If a concrete cold-path diagnostic requires a new main harness, follow 0020 and the ownership table.

### 4. Prose tied to observed stages

Update all cited docstrings and adjacent comments together. A labeled HTTP in-flight row means router entry and an outstanding guard; it says nothing about parsing completion, blocking worker execution, store mutex position or passed stop admission. The held SQLite lock independently prevents successful commits. SMTP's guard can drop when a session aborts while detached blocking work remains unresolved. An incomplete DATA transaction has no insert; an offloaded mutation may refuse at stop or finish atomically if admitted; retry may duplicate a committed row. Qualify lib's all-process-task claims to owned task sets and preserve detached-work residuals. Do not turn comment cleanup into changed client assertions or stricter timing policy.

## Verification and quickstart

Planned test names (implementation must record the final actual names and avoid zero-test filters):

```bash
cargo test --locked --lib stop_joins_serve_before_drained_and_dispatcher_before_return
cargo test --locked --lib stalled_body_selects_drain_deadline
cargo test --locked --lib graceful_http_completion_emits_no_deadline
cargo test --locked --bin swarmail signals_are_polled_before_startup
cargo test --locked --test lifecycle
cargo test --locked --test binary
```

For each mutation, run the relevant targeted test on clean code, record the narrow diff and digest, run the real mutated code and retain the expected assertion or specifically named watchdog failure, restore only the mutated bytes, verify restoration, and rerun green. A build error, unrelated failure, zero selected tests or source-order grep is not a red receipt. No negative mutation is committed.

The single gate inventory is [house rules](../../docs/house-rules.md#gates--part-of-every-change): fmt; locked all-targets clippy; locked workspace tests; pinned brokkr compile; exact coverage and selftest; publish-guard and seal proofs; license audit. Also execute the configured pinned design-check selftest, delivery tamper-boundary proof, recipe structural tests and fail-closed native-tls absence. The latter requires successful dependency-tree inspection plus the expected native-tls no-match refusal, never an arbitrary Cargo error.

Use the launcher engine `/home/valentin/source/feedback-loop-ai/swarmail/.forge/epic192/start-now/eight-lanes/baseline/.forge/bin/brokkr` and immutable recipe `/home/valentin/source/feedback-loop-ai/swarmail/.forge/epic192/start-now/eight-lanes/baseline/bundles/codex-fast`; follow its resolved pinned proof chain, recording executable paths/digests. Do not substitute a version-equal engine or execute candidate-controlled verification dispatch. Private build output is this candidate's `target/`, with 16 build jobs. Log commands, exit results, test counts, paths and mutation/restoration receipts in run-local evidence. Missing tools/evidence remain explicit obligations rather than passes.

CI-only chart, Docker, advisory audit, tag release-rate and real port-80 evidence is separate. This phase prepares a design commit for the engine's final deterministic check; it does not assert implementation, gates, reviewer verdicts or story completion. Later handoff lists all six dispositions, commits, files, tests, negative proofs, preserved rulings, any proposals, residuals and cross-lane requests. Full route retains all four fixed independent reviewers and hard security hold. No nested run or provider substitution.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| A test checks a copied sequence or an unpolled future | Drive actual stop/startup owners; mandatory moved-order mutations must fail relevant assertions. |
| Held tracker task mistaken for a pending commit | State the ownership-only scope; retain real protocol/storage/fan-out tests. Avoid an unnecessary production completion hook. |
| Wrong serve invocation consumes deadline witness | Consume at entry in the tested task before awaiting, fresh current-thread runtime, local ownership and cleanup; test graceful absence too. |
| Fragmented HTTP header makes proof flaky | Read a bounded complete interim head; use a documented cold-path harness when needed, not single-read assumptions or global helpers. |
| Coverage pressure causes exclusions or unnecessary migration | Inline default; name any concrete cold path; cover installed/absent hook and new startup logic; preserve exact denominator and all gate refusals. |
| Added ten-second test duplicates existing cost | Relocate one decisive scenario and remove its duplicate after parity; keep timer unchanged. |
| Stronger tests silently strengthen shutdown contract | Preserve detached connections/blocking tails, retained Store locks, existing fences and observed-only webhook ledger; watchdogs bound tests, not product timing. |
| Signal seam bypasses production path or sends global signals | Main and tests call the same ordering owner; per-test probes and no watcher progress until startup assertion; retain real child process tests. |
| Concurrent lane edits or helper creep | Restrict binary sections, report minimal indispensable helper needs before broadening; keep baseline and other worktrees read-only. |
| Verification unavailable or unrelated gates fail | Record exact commands and failures for parent; leave obligations outstanding, never relax gates or claim story completion. |
