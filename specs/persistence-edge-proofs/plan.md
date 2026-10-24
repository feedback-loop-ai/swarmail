# Plan — persistence edge proofs

Implements [spec.md](spec.md), with ordered proof tasks in [tasks.md](tasks.md).

## Technical context

One Rust crate with bundled SQLite through locked rusqlite, standard Unix
filesystem APIs, and existing tempfile fixtures. The candidate's
`identity_of` follows metadata, tries exclusive creation on NotFound, then
reads metadata after AlreadyExists. A dangling symlink therefore refuses
without creating its target. `identity_guard` already validates the opened
handle. `restore_query` orders by `seq, rowid`; the ordinary seq index can
mask loss of rowid. The sixteen-thread probe already propagates worker
panics but discards three mutation Results.

No plan template exists in this candidate; `.specify/templates/` contains
only the spec template. This plan supplies the required technical context,
constitution check, project structure, research, data model, contracts and
quickstart within the three authorized artifacts.

## Constitution check

Preserve accepted 0001–0005, 0014, 0033, 0042 and 0047. No semantic amendment
or proposed ADR is required. Keep exact 100% line/branch/function coverage,
0035/0050 expansion and file-inventory checks, all pinned shims and all gates.
Under 0020, deterministic branch-stable tests stay inline and measured. Do
not migrate tests simply to avoid coverage; only a concrete cold failure
path can justify a declared cfg(test) sibling harness on design return.
There is no new seam, dependency, configuration, production abstraction or
schema migration. Existing single-writer scope and WAL restart-path limits
remain explicit.

## Project structure and ownership

| Path | Implementation work |
|---|---|
| `src/persist.rs` | Inline dangling-symlink and relative/symlinked-parent regressions; adversarial legacy fixture DDL; module and guard comments. Keep executable production code unchanged. |
| `src/store.rs` | Add expectations at the three discarded mutation Result sites in `run_boundary_round`; preserve the existing probe and joins. |
| `README.md` | Clarify existing persistence path behavior and stable-namespace scope. |
| `specs/persistence-edge-proofs/plan.md`, `tasks.md` | Later record actual test names, mutation receipts and gate/review evidence; do not mark unchecked work complete prematurely. |

`tests/binary.rs`, `tests/sqlite_locks.rs`, `tests/persistence.rs`,
`tests/store_contract_docs.rs` and `src/store/persist_tests.rs` are existing
verification neighbors, not an invitation to expand edits. Cargo files,
production schema/query, decisions, gates, bundles and other worktrees are
outside the implementation diff. Run-local logs and positions stay ignored.

## Research and chosen implementation

### 1. Owning guard explanation

Add the spec's three verbatim accepted-0047 sentences and specific no-seam
rationale to the guard's documentation, preserving the second validation
call. Explain that validation is of opened metadata, not pre/post inode
equality. A documentation/diff review proves this selected acceptance
alternative; do not claim it is an executed replacement-race proof.

### 2. Path fixtures using real Persist opens

Use fresh Unix temporary directories. For a dangling link, create its target
parent but not target, retain the original link target, then call
`Persist::open`. Destructure the error as Io and assert path, NotFound and
raw ENOENT (2 on the target Linux/macOS platforms, with an explanatory
platform scope; no new libc dependency). Check the entry via
`symlink_metadata` and `read_link`, and check target absence. Do not require
absence or deletion of the supplied-path sidecar.

For relative paths, create an isolated temporary directory beneath a
captured `current_dir` and strip that prefix to construct a path for which
`is_relative()` holds. Do not call `set_current_dir` or mutate environment.
For symlinked parents, create a local link to the real directory. Exercise
first creation through each spelling on a missing database separately from
contention; then open the real and alias spellings in both orders, inspect
typed InUse with the supplied rival PathBuf, drop the owner and reopen the
rival. Keep fixture handles alive through all opens and normal cleanup.

Add neighboring tests rather than folding these paths into a loop whose
comment asserts distinct sidecars: relative/parent aliases may share that
sidecar. Preserve the existing distinct-sidecar alias tests and the existing
isolated-root identity-guard proof. Update README and module docs together.

### 3. Adversarial legacy fixture and actual mutation

Within `a_legacy_duplicate_seq_restores_in_write_order`, precreate the
fixture-only index `emails_seq ON emails(seq, received_ms DESC)` before
`Persist::open`; its CREATE INDEX IF NOT EXISTS leaves the index in place.
Keep first/tied-a/tied-b rows, duplicate seq 2 and increasing received_ms.
The competing secondary order should expose tied-b before tied-a when the
actual full-column restore loses rowid. Label this an adversarial legal
fixture, not an index alleged to have shipped in an old release.

Run the existing named Rust test with --locked and check a nonzero test
count. Save the narrow source state; temporarily remove only rowid from the
real restore query, run the same unchanged test, and require failure of the
write-order assertion on reversed ties. A build failure, zero tests or
unrelated error is not a mutation receipt. Restore only those mutated bytes,
check the diff and rerun green. Store command, selected-test count, exit,
assertion output and mutation/restoration diff in run-local evidence; record
a concise receipt in these owning artifacts during implementation.

If the locked planner does not select the adversarial index as expected,
revise fixture DDL/data and repeat until the real mutation fails. Do not
change shipping SQL, force a narrowed query, accept a Python result, or
replace the behavior proof with a source-string assertion. Keep the
ordinary-schema full-column no-temp-sort plan test independent.

### 4. Boundary probe errors

Replace each discarded `delete`, `clear` and `clear_all` Result in
`run_boundary_round` with `.expect(...)`, explaining that the boundary probe
injects no storage failures, as the insert expectation already does. Do not
assert the successful bool/count value: another role can win. Existing
joined panics own failure propagation. Preserve every role, iteration,
barrier and final-state witness. No extra failure-injection mechanism or
error-aggregation abstraction is needed.

## Data model and contracts

There are no new persisted entities or public signatures. Fixture state is
a supplied path and its aliases, one owned Persist handle, and legacy rows
with deliberately conflicting seq/received_ms order. Public errors remain
InUse only for contention, Io with path/cause for filesystem validation and
Sqlite for SQL failures. Missing ordinary files can be created; dangling
symlink targets cannot. Identity guards never lock SQLite files or open and
close existing database descriptors. Sidecars remain inert after release;
alias refusal is not historical multi-writer repair or WAL relocation.

## Verification and quickstart

Run focused tests first and record the actual new path-test names. Verify
that every filter selects a nonzero number of tests:

```bash
cargo test --locked --lib persist::tests::
cargo test --locked --lib a_legacy_duplicate_seq_restores_in_write_order
cargo test --locked --lib the_restore_scan_reads_the_seq_index
cargo test --locked --lib sixteen_threads_cannot_leave_memory_and_disk_in_two_orders
cargo test --locked --test binary
cargo test --locked --test sqlite_locks
cargo test --locked --test persistence
cargo test --locked --test store_contract_docs
```

The legacy test additionally needs the red/restored-green sequence above.
Inspect the boundary probe to confirm all three Result sites use expectations
and the join remains. Review the exact citation and docs against the typed
path assertions. The broad binary/lock/persistence suites retain real process,
raw SQLite and real-server coverage without mocks.

Implementation/verify must then execute the single current
[house-rule gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
in full, including --locked, pinned brokkr compilation, exact coverage and
its selftest, publish-guard and seal proofs and license audit. Prove native-tls
absence fail-closed: dependency inspection itself must succeed; an arbitrary
Cargo error is not absence evidence. Resolve the pinned engine from this
candidate's tooling/recipe and record its digest; never substitute a
version-equal binary or weaken the offline verify boundary. Verify executes
the pinned scripts against the candidate after checking dispatch shims.
CI-only proofs remain assigned by the charter; retain macOS locking proof
separately as well. Chart, real Docker, network advisory, tag-rate and
port-80 evidence retain their assigned scope. No local Linux result
substitutes for them or authorizes publication.

For this design phase, check exact committed paths, nonempty clean artifacts,
relative authority links and `bash bundles/self/scripts/design-check.sh check .`.
The sequence performs its authoritative check from its pinned bundle.
Design drafting does not discharge implementation, full gates or review.
The engine-selected route retains Luna xhigh implementation and all four
configured independent review lenses named in the spec, without fallback.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Citation alternative mistaken for a race proof | Preserve the exact authority, rationale and explicit no-new-mutation-proof limit; keep the post-open call. |
| Unexpected planner keeps the rowid mutant green | Require actual locked-Rust assertion failure; revise only fixture DDL/data. The unusual index is an intentional proof cost. |
| Negative mutation leaks into final code | Save and restore only the narrow mutation; inspect production diff and rerun green before final gates. |
| Relative fixtures disturb parallel tests | Allocate unique directories under captured cwd; no global cwd/environment change or sleep synchronization. |
| Shared sidecar misattributed to identity lock | New tests claim observable exclusion; retain separate distinct-sidecar and isolated-root identity evidence. |
| Dangling link mistaken for an absent ordinary path | Separate creation witnesses, typed Io/ENOENT assertion, unchanged symlink and absent-target checks; allow inert sidecars. |
| Zero-result mutation treated as failure | Expect only Result success; permit false/zero concurrent outcomes. |
| New assertions pressure exact coverage | Use branch-stable inline tests and existing idioms; no blanket harness moves, exemptions or floor changes. |
| Linux-only evidence overstates filesystem portability | State Linux/macOS targets and existing stable shared namespace scope; preserve assigned macOS/CI proof obligations. |
| Missing tools or failed gates obscure unfinished work | Record exact commands/failures and outstanding obligations; no inferred pass, alternate provider or policy bypass. |

## Implementation evidence — Linux

- Before source edits, the committed artifacts at a8802b3 passed
  bash bundles/self/scripts/design-check.sh check . using the candidate-local
  pinned Brokkr binary (SHA-256
  ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f).

- The post-open call to check_guard(&held.metadata()...) still precedes
  try_lock; the owning comment records the three exact accepted 0047 §1
  statements, distinguishes metadata validation from inode equality, and
  explicitly claims no new replacement-race proof. No seam was added.
- a_dangling_data_symlink_refuses_with_enoent_without_creating_target
  passed in the 42-test persist::tests run. It checks typed OpenError::Io,
  the supplied path, NotFound, raw ENOENT, unchanged symlink type/target,
  and absent target.
- relative_data_path_refuses_its_absolute_alias_in_both_orders and
  symlinked_parent_refuses_its_real_path_in_both_orders passed. Each creates
  a missing database through its alias, checks typed InUse with the rival's
  exact spelling in both orders, and reopens after owner drop. No cwd or
  environment mutation is used.
- a_legacy_duplicate_seq_restores_in_write_order and
  the_restore_scan_reads_the_seq_index passed. The unchanged restore-order
  assertion failed under the actual ORDER BY seq mutant on
  ["first", "tied-b", "tied-a"], then passed after exact source
  restoration. Full command, test count, exits, assertion output, and hashes
  are retained in
  .forge/evidence/epic-192-issue-187-rowid-mutation.md.
- sixteen_threads_cannot_leave_memory_and_disk_in_two_orders passed after
  all three mutation results changed to .expect(...); the 16 roles, joins,
  final nonempty witness and live/reopen comparisons remain intact.
- Focused suites passed: persistence unit family 42; restore plan 1;
  boundary probe 1; tests/binary.rs 32 passed (1 CI-owned port-80 test
  ignored); tests/sqlite_locks.rs 1 passed (1 child helper ignored by
  design); tests/persistence.rs 12; and tests/store_contract_docs.rs 19.
- Full house-rule gates and the fail-closed native-tls absence proof passed.
  Exact counts, command inventory, pinned Brokkr SHA-256, and CI-owned limits
  are recorded in .forge/evidence/epic-192-issue-187-implement-gates.md.
  CI-only and macOS receipts remain outstanding as the charter assigns.
- T10's policy-selected four-lens review has not run in the implementation
  phase; no review verdict or completion is claimed here. The engine's later review phase owns that dispatch.
