# Implementation plan: single-writer lock

Implements [spec.md](spec.md), accepted
[0033](../../docs/decisions/0033-one-server-per-data-file.md) §§2–4 and the
supplemental proposed [0042](../../docs/decisions/0042-single-writer-lock.md).
This is the design handoff, not evidence that the implementation has landed.

## Technical context

Single Rust 2024 crate, rusqlite 0.32 with bundled SQLite, Tokio and existing
`thiserror` 2. `Persist` currently owns only `Mutex<State>`; `State` owns the
connection. `Store::open` opens then loads the mirror. `run_on` opens/restores
before binding, and `main` already reports startup errors and exits 1.
`File::try_lock` requires Rust 1.89 or newer; verify the active and CI
toolchains rather than treating 0033's historical version claim as evidence.
No new dependency or manifest change is planned.

There is no `.specify/templates/plan-template.md` in this checkout; this plan
supplies its technical context, constitution check, structure, research,
data model, contracts and quickstart directly.

## Constitution check

0001/0014/0023 keep synchronous commit-before-visibility and awaited offload;
the lock is acquired once before SQLite open, outside the mutation hot path.
0002 subscription-before-scan, 0003 oldest-only eviction and 0005 extraction
at ingest remain unchanged. SQLite WAL, FULL and its 5000 ms busy default
are preserved. Raw connections remain usable under accepted 0033.

Tests obey 0020: deterministic tests stay inline and measured; public process
tests live in `tests/binary.rs`. A new harness is justified only by a concrete
cold failure path, never by seam access or uncovered production code. Keep
0035's exact line/branch/function floor, no coverage attributes, no feature
flags, no global mutable hooks, no sleep synchronization and no probabilistic
chaos. No dependency allowlist, bundle, gate or unrelated feature changes.
0042 remains proposed; accepted decision text is corrected by dated addendum.

## Project structure and files touched

| Path | Implementation responsibility |
|---|---|
| `src/persist.rs` | Sidecar admission, owned file after `state`, public startup error and private conversion, module contract sentence, inline proofs. |
| `src/store.rs` | Startup return type and conversion of restore errors only; existing mutation APIs unchanged. |
| `tests/binary.rs` | Two-server refusal and crash/graceful reuse using existing subprocess helpers and real HTTP/SMTP. |
| `tests/lifecycle.rs`, `src/store/persist_tests.rs` | Audit retained owners across reopen; change only fixtures that need explicit final-owner drop or sidecar-safe cleanup. |
| `README.md` | Exact contract sentence and operational crash/overlap note in the data-file section. |
| `docs/decisions/0033-one-server-per-data-file.md`, `docs/decisions/README.md` | Implementation metadata, dated historical correction, current index row and proposed 0042 index entry, in the implementation commit. |
| `specs/single-writer-lock/{spec,plan,tasks}.md`, `docs/decisions/0042-single-writer-lock.md` | Exactly four design artifacts; later evidence/clarifications belong with their owning requirements. |

`src/lib.rs` and `src/main.rs` should need no edits: existing formatting and
error propagation accept the new Display type. No new production module,
config, CLI flag, database migration or schema change.

## Research and resolved choices

Read both positions, the run-19 commission, issue #85, the historical task
framings, decision index and applicable rulings. Source confirms that SQLite
currently opens before any admission guard, schema startup drops an old
index and performs migration, and `run_on` opens the store before binding.
Therefore lock-first is a correctness requirement, not a style preference.

The spec's ruling table resolves the genuine error-type disagreement. Public
`persist` and `store` modules make the signature change observable outside
this crate; existing callers needing only Display/`?` do not prove downstream
source compatibility. Use one startup-specific type, not a general store
error hierarchy. The type also covers `load` failure propagated by
`Store::open`; `load` itself and mutation methods keep rusqlite results.

Inspection of `tests/lifecycle.rs` found reopen-after-stop at the mid-DATA
and mid-POP3 legs. Inspect any caller-owned `Arc<Store>` and cleanup lifetime
while implementing; do not assume server stop releases externally held Arcs.
The binary suite already has readiness parsing, child guards, signaling and
reaping, graceful restarts and hard-kill durability tests. Reuse them.

## Data model and open sequence

No persisted data changes. Add a `File` field, named `_lock`, after `state`
in `Persist`, so Rust drops the SQLite-owning state before the lock. Do not
clone the file, implement a custom Drop, explicitly unlock, or unlink it.

1. Build the sidecar path by copying `path.as_os_str()` to `OsString` and
   pushing `.lock`. Open read/write/create with `truncate(false)` explicitly.
2. Call `File::try_lock` once. Convert `WouldBlock` to `OpenError::InUse`
   carrying the data path. Convert other lock or open failures to `Io`
   carrying the sidecar path and original `io::Error`. Never fall back to
   unlocked operation, retry, or call the blocking lock method.
3. Only then run the existing connection, pragma, schema, migration and
   next-sequence code. Connection locals are created after the lock local,
   so failure unwinding drops the connection before the file. Construct
   `Persist` with connection state before the lock field on success.
4. `Store::open` returns `Result<Self, persist::OpenError>` and converts
   restore errors to `Sqlite`. A restore failure drops its local `Persist`.
   Existing `run_on`/main formatting supplies path context and exit 1.

`OpenError` has `InUse(PathBuf)`, `Io { path: PathBuf, source: io::Error }`
and `Sqlite(rusqlite::Error)` cases. Implement Display/Error and the needed
SQL conversion with the existing thiserror dependency or equivalent small
code. `InUse` has exactly 0033's Display text and no underlying cause;
the other variants retain their underlying errors. Avoid unrelated helpers.

## Contracts and proof design

Inline tests use the existing TempDir fixture owner. Retain the first
`Persist` through the refusal assertion, match the variant and exact string,
and reopen only after drop. Hold a raw SQLite connection in this test to pin
the intentional advisory boundary. Independently lock an absent database's
sidecar and prove refusal never creates the database. Concurrent opens of
`mail.db` and `mail.sqlite` detect extension replacement; sentinel bytes
detect truncation, and a retained file proves shutdown did not unlink it.
Fresh `Store`/public API assertions pin empty inboxes and zero counters.

Cover filesystem-open failure with a sidecar directory or nonexistent
parent under a unique TempDir, without permission-based fixtures that pass
under root. Cover SQL failure with the existing malformed database fixture;
after refusal, take its sidecar lock directly to prove cleanup. For lock
error mapping, factor the necessary conversion into one private function
taking the data/sidecar paths and `TryLockError`. Exercise both explicit
error values inline, checking Display and source. This tests pure error
classification, not a mock filesystem or a claim that an unsupported mount
was exercised. Real success and contention still run through the syscall.
A missing-parent fixture cannot execute the lock syscall's Error branch.

In `tests/binary.rs`, start A using `serve_with_data`/`spawn_serve`, observe
readiness and check empty inboxes/counters. Start B with its own ephemeral
ports and the identical data path; use a captured-output child with cleanup,
not `spawn_serve`, which requires readiness. Under the suite's existing
helper deadline, assert exit nonzero, stderr contains the exact refusal with
path and stdout contains no readiness record. Confirm A remains healthy,
deliver a mail by SMTP and query it. Kill and reap A, mark its guard reaped,
then start C without touching the sidecar, verify the retained mail, stop
and reap C gracefully, and start D on the same file. Clean up all children
on failures as well as success; do not drop a timeout future and leak B.
Drain pipes as existing helpers do. Deadlines detect failure; readiness and
process exit establish ordering. Reuse existing graceful tests where they
already prove that same sequence without deleting the sidecar.

Keep raw-connection corruption/inspection and `BEGIN IMMEDIATE` stall tests
intact. Preserve the 5000 ms pragma tripwire, including its can-fire test.
Capture a behavioral red for the new second-server regression against the
unchanged behavior, in an isolated permitted checkout or a reversible local
mutant; restore it before delivery. A compile failure is not behavioral red.

Documentation updates land only with code/tests. Copy 0033 §4's exact sentence
into README and module docs; add crash, no-manual-unlock and overlap notes.
Preserve 0033's acceptance history, annotate its status metadata with actual
implementation state and add a dated correction identifying the old
not-yet-implemented passage and §1 restart claim as historical/superseded.
Reference proposed 0042; update the index truthfully without self-acceptance.

## Quickstart and validation

Work on a feature branch and keep the implementation, required tests and
contract sentences in one implementation commit. Run targeted tests first:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
rustc --version
cargo test --workspace --locked persist
cargo test --workspace --locked --test binary --test lifecycle --test persistence
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
cargo deny check licenses
```

Use no TMPDIR override. Require exact nonzero 100/100/100. Inspect coverage
of each added error conversion, Display/source arm and seam if any. Missing
coverage requires a meaningful proof, not an exclusion or harness relocation.
Leave implementation tasks unchecked until their proving tests run.

For this design, validate four nonempty artifacts, the Acceptance Criteria
and Risks headings, ordered checkboxes, relative links and proposed 0042.
Stage exactly those four paths, use an imperative signed commit with a why
body, and never push. If `.git` is read-only, retain files and an ignored
patch, report that the required commit is blocked, and do not redirect Git
metadata or call the paths committed. Report unavailable gates honestly.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Lock released before connection close, or during open failure. | Field order and local creation order; drop/reopen and failed-open release tests. |
| External Arc retains ownership after stop. | Drop final fixture/client owner explicitly; library contract follows `Persist` lifetime, not listener lifetime. |
| Startup overlap fails repeatedly. | Immediate explicit refusal; supervisor controls retry. No promise of exactly one failed attempt. |
| Unsupported locking or unwritable sidecar directory. | Fail closed with original I/O cause and path. Such deployments cannot start. |
| Sidecar deleted/replaced or aliased data path. | Never unlink in production; document cooperative stable-path scope. No hostile-directory or alias-identity guarantee. |
| Public return-type change breaks downstream source. | Proposed 0042 names the cost; narrow change to two open methods and keep SQL source accessible. |
| Lock Error arm is assumed covered by an earlier open failure. | Directly test the shared conversion with both TryLockError cases and inspect exact coverage. |
| Existing metadata or old database is mistaken for repaired state. | Dated correction, honest implementation metadata and explicit no historical-data reconciliation. |
| Binary timeout leaks an owner and poisons later tests. | Per-test TempDir and child guards, captured pipes, kill/reap on all exit paths; no fixed ports or sleeps. |
| Toolchain or sandbox blocks verification/commit. | Record exact failure and pending work; preserve a patch without bypassing permissions or claiming delivery. |
