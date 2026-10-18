# Implementation plan — macOS identity lock

Implements [spec.md](spec.md) and proposed
[0047 §1](../../docs/decisions/0047-storage-rpc.md). No runtime change is part
of this design commit. No plan template is present; the required spec-kit
sections are supplied below.

## Technical context

Single Rust crate, rusqlite's bundled SQLite, std File locks and Unix metadata.
`Persist` currently owns its mutex-protected connection, a data-file flock and
a pathname sidecar. Replace the data-file guard with an identity sidecar;
retain connection-first field destruction and the existing OpenError API.
No dependency, schema, configuration or asynchronous runtime change.

## Constitution check

Preserve 0001/0002/0003/0005 and accepted 0033's original sidecar disciplines.
Amend proposed 0047 in place per commission, without accepting it or editing
accepted text. Keep WAL/FULL, the busy bound, every protected feature and exact
100/100/100 coverage. Inline deterministic tests are the 0020 default; a
concrete cold failure path alone justifies a declared sibling test harness.
Public protocol proofs use real processes. No sleep synchronization, global
environment mutation, production feature seams or coverage exclusions.

## Research and resolved questions

The defective call is `acquire(path, path)` at `794e919`. Linux local flock
success does not establish compatibility with Darwin's interoperating advisory
locks. The Linux direct-flock probe is the baseline-red proof; raw SQLite
transactions and macOS CI are separate evidence. Do not claim testing on a
macOS host in this environment.

Both panels require identity plus a common directory to catch cross-directory
hard links. Choose fixed `/tmp` (the chart already provides it), with a private
database-owner directory. Do not consult temp_dir(), HOME or a public root
override. Test-only root arguments are local to each test, never global.
Root injection may exercise helpers, but real binary tests must prove the
actual fixed-root behavior and different-TMPDIR contention.

Read existing-file metadata without an open descriptor. POSIX close semantics
make an extra open/drop unsafe during an in-process failed alias attempt.
For a missing file use `create_new`, not `create(true)` after a failed stat:
another creator may have started SQLite in between. An AlreadyExists result
goes back to metadata once, without opening that file or retrying a lock.
Replacement during startup remains outside the stable-namespace contract.

## Project structure and files

| File | Work in implementation |
|---|---|
| `src/persist.rs` | Metadata/create helper, verified guard directory, identity path/acquisition, field comments/module contract and inline tests. Keep mutation/schema/restore code unchanged. |
| `src/persist/identity_tests.rs` | Only if the transaction/child or creation-race proof requires a concrete cold failure path; declare under cfg(test), keep production seams cfg(test)-only. |
| `tests/binary.rs` | Extend existing real alias proof with a cross-directory hard link and different child TMPDIR; preserve real health/mail/cap/restart checks. |
| `tests/common/mod.rs` | Only necessary deterministic real-child coordination; reuse existing bounded cleanup. |
| `README.md`, `docs/decisions/README.md` | Mechanism, scope, cleanup prerequisite, current proposed index synopsis. |
| `specs/storage-rpc/{spec,plan,tasks}.md` | Replace obsolete data-file flock directions and branch-create workflow; retain sibling issues and historical receipts, mark this design as the controlling fix-up. |
| `docs/decisions/0042-single-writer-lock.md` | Clarify its proposed 0047 pointer if it implies a data-file lock; do not rewrite accepted 0033. |

The chief stages only `specs/macos-identity-lock/{spec,plan,tasks}.md` and
`docs/decisions/0047-storage-rpc.md`. The remaining files above belong to the
subsequent implementation commit. No new production file is needed.

## Implementation approach

1. Confirm the existing branch and preserve its two shipped commits. Acquire
   the exact original supplied-path `.lock` first, using its existing
   nontruncating, nonblocking, never-unlink behavior.
2. Obtain database metadata with `std::fs::metadata` (follow database aliases).
   Require a regular file; preserve a path-bearing Io refusal for a directory.
   For NotFound, attempt read/write exclusive creation at mode 0644 subject
   to umask, read the created handle's metadata, and close it before SQLite.
   On AlreadyExists read metadata once; other failures refuse. A test-only
   hook between missing lookup and creation deterministically exercises this
   race. Do not open an existing database solely to identify it.
3. Use Unix uid/dev/ino from that metadata. Format numeric uid and unambiguous
   hex device/inode components under the fixed directory in the spec. No hash
   or path canonicalization. Other platforms must supply a real identity
   mechanism or return path-bearing Unsupported Io before SQLite; never
   restore the forbidden flock. This commission adds no platform promise.
4. Create the owner directory at mode 0700, tolerate AlreadyExists only after
   checking with symlink_metadata that it is a real directory, belongs to the
   database owner and has private owner-access permissions (0700). Refuse an
   unsafe existing directory; do not chmod, replace or delete it. A creation
   race must converge to the same verified directory.
5. Create the identity guard exclusively, mode 0600, or validate and open an
   existing guard nontruncating. Before opening an existing entry require
   symlink_metadata to report a regular, singly-linked file owned by the
   database owner with no group/other access. This prevents a FIFO open from
   hanging and refuses symlink/hard-link redirection. Validate opened metadata
   before try_lock as well. Handle AlreadyExists at creation by the same
   existing-entry validation, without lock retries. The private stable parent
   is the trust boundary; hostile same-user replacement is not promised.
6. Take one exclusive File::try_lock on that guard, mapping WouldBlock to the
   supplied database path's InUse and other failures to the guard's Io.
   Hold both sidecar files after connection state. Only now run unchanged
   Connection::open, WAL/FULL, migration and restore. Drop releases guards on
   errors, stop and process death. Never delete or refresh a held guard.
7. Correct obsolete docs and tests with the implementation, including the
   assertion that SQLite “never sees” flock and the database-descriptor
   lifetime warning. Keep the separate rule that connection close precedes
   ownership release. Preserve all sibling issue behavior and tests.

## Data model and contracts

No database format or public type changes. `_identity: File` now holds a
sidecar. Inert files accumulate per distinct device/inode, and startup gains
a writable, stable `/tmp` prerequisite. Metadata and namespace errors use
Io with the actual failing path/source. Only real contention is InUse.
Fresh failed startup may leave an empty data file and inert guards; no schema
or mail is written without both guards. Cross-user refusal is allowed to be
Io; caller identity must never change the rendezvous key.

## Proof design

AC1's Linux-only flock probe must run after a real write creates WAL/SHM,
then release all probe descriptors before any subsequent SQLite work. A raw
write witness alone is green even on the faulty Linux implementation and
must never be described as its red proof. Retarget the existing simulated
identity holder without weakening before-schema and failed-open assertions.

For AC4, hold a real owner's SQLite write transaction, attempt an in-process
alias open and observe InUse, then have a separate process attempt a raw
write with zero busy timeout. It must observe SQLITE_BUSY until the owner
commits, then succeed. Coordinate transaction and child via pipes/channels;
use a real deadline only as a failure bound. This detects accidental POSIX
lock release that same-process probes cannot establish. Keep private-seam
coordination in a cfg(test) harness only if 0020's cold-path rule applies.

Directory/guard tests use owned TempDir roots passed to local helpers, never
global TMPDIR changes. Real process tests use production `/tmp`, which leaves
inert guard files by design; do not unlink active files or claim zero litter.
Use metadata-validation helper inputs for a deterministic wrong-owner branch
if host privileges cannot construct one, paired with real symlink/mode/type
filesystem refusals. Every test seam must have an installer. No privileged
chown, network filesystem, fake SQLite or wall-clock validity assertion.

Collect AC1 red against `794e919` behavior in an isolated control and green
against the repair. Preserve #129's existing `2a325c9` alias-red evidence;
that baseline lacks the new self-lock bug. Keep control mutations out of the
final tree and identify the exact tests/baselines in the implementation commit.

## Quickstart and validation

For iteration run `cargo fmt --all` before each check, then targeted
`cargo test --workspace --locked persist::` and
`cargo test --workspace --locked --test binary` as appropriate. Final gates
run in this order, writing fmt before each:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo fmt --all
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all
cargo test --workspace --locked
cargo fmt --all
bash scripts/coverage-gate.sh
cargo fmt --all
bash scripts/coverage-gate.sh selftest
cargo fmt --all
bash scripts/publish-guard-test.sh
cargo fmt --all
cargo deny check licenses
```

Coverage remains exact missed == 0 for lines, branches and functions, without
a TMPDIR override. Run the existing fail-closed native-tls absence proof.
Verify owns `brokkr compile --bundle bundles/self` from the tooling-pinned
binary; an expected sandbox refusal is reported, not bypassed or installed
around. CI retains chart/advisory/release-rate/port-80 obligations, and
`test (macos-latest)` is required after the operator's assistant pushes.
No delivery or Darwin-green claim before those receipts exist.

If cargo is refused, immediately stop, leave changes uncommitted and save
`.brokkr/state/epic120-storage-rpc.patch`. If configured signing fails, stop
without an unsigned commit. Otherwise append signed commits, retain the
branch with a clean tracked tree, and do not push or act on GitHub.

## Risks

| Risk | Mitigation and accepted limit |
|---|---|
| Darwin lock-table interaction | Application locks target two sidecars only; Linux baseline-red probe plus raw writer and macOS CI. |
| Extra database close drops process record locks | Stat existing file; exclusive create when missing; deterministic creation race and separate-process lock witness. |
| Cross-directory links evade exclusion | Identity-only filename in fixed shared root; both-order tests across directories and differing TMPDIR. |
| Shared `/tmp` entry redirects a lock | Verified private owner directory and regular/singly-linked owned guard checks before locking; fail closed. Same-user hostile namespace mutation remains outside scope. |
| Cleaner unlinks held guard, allowing split ownership | Document mandatory cleanup exclusion and no live deletion. No heartbeat claims; kernel lock lifetime alone cannot protect pathname existence. |
| Private mount namespaces select different sidecars | Document shared-namespace prerequisite; no cross-pod/private-tmp claim. Original sidecar retained wherever shared. |
| Cross-user writable database is rejected | Database-owner key is shared; inaccessible/foreign directory is Io, never a separate per-caller guard. No cross-user service promise. |
| Guard release precedes checkpoint/close | Keep state before both guard fields and test drop/failure/kill reuse. |
| Hard-link crash restart misses WAL | Preserve configured pathname across crashes; only transfer spelling after graceful close in tests. No historical recovery claim. |
| New refusal paths threaten exact coverage | Deterministic filesystem fixtures and minimal cfg(test) seams; no floor change or unmeasured production helper. |
| Proposed design mistaken for shipped fix | Explicit unchecked tasks, proposed ADR, docs with code, and separate artifact/runtime/macOS receipts. |
