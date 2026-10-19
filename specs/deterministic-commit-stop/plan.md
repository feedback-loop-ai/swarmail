# Implementation plan — deterministic commit at stop

Implements [spec.md](spec.md); scope is the one #143 re-drive-5 repair.

## Technical context

Single Rust crate, Tokio server/session tasks and blocking mutation pool,
rusqlite persistence, real loopback SMTP, existing per-store pre-mirror gate.
The library's `#[cfg(test)]` persistence harness can access that gate;
external integration crates cannot. Existing std/Tokio synchronization and
tempfile suffice. No dependency or production configuration changes.
There is no plan template in `.specify/templates/`; this plan supplies the
house-required sections directly.

## Constitution check

Retain the one synchronous plan → commit → apply boundary, 0024's one
shutdown admission check and 0023's offload of the whole mutation. Moving a
test-only invocation leaves production expansions unchanged. 0020 permits
the existing owner harness for private synchronization and cold diagnostics;
the moved test still exercises the actual server and protocol. Do not import
integration helpers by `#[path]`, move unrelated tests, or add exclusions.
No semantic change is proposed and no decision text needs amendment.

## Research resolved by inspection

`Store::record` owns the stopping check and mirror write. The pre-mirror gate
currently runs only in `clear` and `clear_all`, before `record`; relocate those
invocations to the common owner after admission. Existing panic and ordering
installers in `src/store/persist_tests.rs` must keep working. SMTP offloads
parsing before calling `insert_async`, so the delivery-in-flight metric cannot
certify entry to the commit. `insert` broadcasts only after applying memory
and counters; subscribe before the test delivery to observe completion.

The busy timeout is rusqlite's connection default, pinned by tests in
`src/persist.rs`, not an existing configurable call to thread through Config.
Its default and deliberate-override tripwire remain unchanged. A controlled
pause before SQLite tests the lifecycle of the admitted whole mutation; it
does not claim to test SQLite's busy-handler implementation.

## Project structure and touched files

| Path | Change |
|---|---|
| `src/store.rs` | Consolidate the existing cfg(test) gate invocation into `record`, after admission; correct the seam comment. |
| `src/store/persist_tests.rs` | Own the moved named regression, local bounded wire driver and panic-safe entered/release handshake; reuse existing fixture helpers. |
| `tests/lifecycle.rs` | Remove only the migrated test and any imports made unused by that removal. Keep remaining lifecycle tests and budgets. |
| `src/smtp.rs` | Only correct the existing comment's pointer to the migrated regression if necessary; no runtime change. |
| `.brokkr/state/` | Ignored mutant/load/gate transcripts, commands and candidate identities. |

Chief commits only `specs/deterministic-commit-stop/{spec,plan,tasks}.md`.
Existing delivery-gates artifacts and all dated entries remain historical
authority for the other batch work. No tracked stress-runner or new harness
file is needed; minor adjustments to these three documents may record actual
implementation outcomes without widening scope.

## Data model and contracts

No production data model, schema, wire, metric or Config change. Test state
has three monotonic stages: admitted-and-held, stopped-and-held, released.
Use a per-instance async entered signal plus a blocking release primitive
(for example a oneshot to the test and a mutex/condition-variable predicate
for the blocking closure). A predicate loop handles spurious wakes; a Drop
guard sets the predicate and notifies. A release before gate entry must also
work. The closure must not re-enter Store or hold its gate-storage mutex.

The gate's release is test-owned and has no self-expiring busy interval.
Deadlines diagnose lack of progress, not whether a five-second opportunity
was caught. Keep the current stop budget and existing observation bounds;
do not widen shared helpers. Start the final SMTP-reply observation after
stop rather than spend a helper deadline while intentionally holding DATA.
An early reply remains buffered and must fail the no-250 assertion.

## Implementation sequence

1. Move the invocation with its cfg guard, remove old duplicate invocations,
   and audit all existing gate installers. Confirm the production path still
   performs the same check and mirror call with no new recheck after release.
2. Start `crate::run_on` with fresh persistence, loopback port 0 listeners,
   no webhook targets, and a dedicated inbox. Clone the store before stop
   consumes the running-server handle. Install the gate and its release guard
   before starting delivery; subscribe before sending the mail.
3. Drive the SMTP handshake through DATA using local bounded harness helpers.
   Wait asynchronously for gate entry. Keep it held while stopping the server,
   reading the final SMTP outcome, checking refused listeners, and inspecting
   memory/SQLite/counters/no notification. Prefer reads that do not take the
   mutation lock; never wait on the blocking release from an async worker.
4. Release, await the existing notification, and compare the exact stored mail
   and counter deltas between memory and SQLite. Use a separate read-only
   connection while the live store holds its writer-identity lock; reopening
   another Store would intentionally refuse. Keep the temp directory until
   the blocking operation finishes and all SQLite handles can close.
5. Execute AC6's four independent mutants in disposable candidates or with
   exact reversible patches. Make the discard mutant actually prevent the
   post-gate write: merely dropping a spawn_blocking JoinHandle does not
   cancel its closure. For the early-apply mutant expose a real memory/counter
   change before the gate, without introducing an unrelated lock deadlock.
   Match named diagnostics, preserve patch and output, restore, then rerun.
6. Run AC7's 20-pass load sample and full instrumented coverage under load.
   Discover allowed CPUs (including affinity restrictions), start one tracked
   busy worker per available CPU, and keep them active through execution.
   Record affinity if used, PIDs, all statuses, and trap cleanup on every exit.
   Use existing shell/host tooling; install nothing. Do not serialize the
   suite or silence failed runs. A finite deadline cannot guarantee progress
   on an indefinitely starved host; the claimed determinism is the causal
   ordering plus the measured load evidence, not unlimited scheduling fairness.
7. Run the complete final-tree gate inventory, then append signed commits with
   proof-to-issue mapping in their bodies. Leave unrun/failed obligations named
   and unchecked. Verify owns the pinned compile if this sandbox refuses it.

## Validation and quickstart

Focused implementation commands (the full module name must match the moved
test; inspect `--list` so a zero-test filter cannot pass as evidence):

```bash
cargo fmt --all
cargo test --workspace --locked --lib store::persist_tests -- --list
cargo test --workspace --locked --lib store::persist_tests::an_in_flight_commit_neither_holds_stop_nor_half_lands -- --exact
cargo test --workspace --locked --lib store::persist_tests
cargo test --workspace --locked --test lifecycle
```

The loaded sample repeats the exact-test command at least 20 times with all
workers active, aborts the sample on a failure, and records all results. It
does not change tests' internal deadlines. Then execute the full coverage
gate with those workers still active, including normal suite concurrency.

Final inventory, with formatting written before checks:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
cargo deny check licenses
brokkr compile --bundle bundles/self
```

No TMPDIR override. Verify uses canonical pinned scripts and the recorded
binary; use its existing fail-closed native-tls proof too. The loaded coverage
run may satisfy that inventory entry on the same restored final source tree;
it must report zero missed lines, branches and functions. No test or gate
receipt is claimed by this design. If cargo itself is refused, follow the
operator's patch-and-stop instruction; never manufacture a pass. If signing
fails, leave the intended changes uncommitted and report the blocker.

## Risks

- **Gate entry before admission would preserve the parse/admission race.**
  Put it after the only stopping refusal, under the mutation lock; inspect
  the source and prove the mandatory post-stop completion tail.
- **Moving a common seam changes existing tests' reach.** Keep one invocation
  owner, exercise all current installers and rerun queued-stop/refusal pins.
  Test-only code remains measured through the owning unit's execution.
- **A failed assertion can strand a blocking task at runtime teardown.**
  Construct the release guard before delivery, use a non-lossy release
  predicate, preserve cleanup on unwind, and verify each red process exits.
- **Wire helpers can introduce another timeout race.** Do not start an
  autonomous short DATA-reply timer while the gate is deliberately held;
  bound the actual step observations and distinguish protocol EOF/reset from
  harness failures. Stop uses the unchanged fence-derived budget.
- **The new proof does not exercise SQLite busy backoff.** Preserve the real
  lock/refusal regressions and 5000-ms pragma/default-override pins; do not
  claim this test replaces their dependency-specific proof.
- **CPU stress can mask a resource leak or monopolize the host.** Track only
  this run's worker PIDs, trap their cleanup, record the load configuration,
  and retain instrumented outputs. Do not kill unrelated processes.
- **Mutation evidence can be vacuous.** Require the named test to execute
  exactly once and fail with the intended diagnostic, not a build error,
  unrelated panic or external watchdog. Restore before every green gate.
