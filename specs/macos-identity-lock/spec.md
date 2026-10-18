# macOS-safe identity ownership (#129)

Status: design handoff; implementation and macOS verification remain
outstanding. Branch: `epic120-storage-rpc`, extending `8819e62` and `794e919`.
Decision: 0047 — [§1](../../docs/decisions/0047-storage-rpc.md), amended in place
and still **proposed**. No new decision number: 0048 is free, but the commission
specifically requires correcting the existing proposal. Acceptance belongs to
the operator.

## Feature and rationale

Replace the application lock on the SQLite database with a separate guard
whose name follows file identity. Preserve refusal of a second server through
any ordinary spelling of the same file, including symlinks and hard links in
different directories, in either acquisition order. Retain the original
`<supplied-path>.lock` for same-path compatibility with previous binaries.

At `794e919`, `Persist::open` calls `acquire(path, path)` before SQLite opens.
That takes an exclusive flock on the database. Linux local-filesystem flock
and SQLite's POSIX record locks are independent; Darwin/BSD locking interfaces
interoperate, so the application's flock conflicts with its own SQLite lock
requests. This explains the reported macOS `DatabaseBusy` failures while Linux
passes. Application guards must target neither the database nor its `-wal`,
`-shm` or `-journal` files. SQLite retains its own normal locks.

The supported rendezvous is
`/tmp/swarmail-db-locks-<database-owner-uid>/<device-hex>-<inode-hex>.lock`.
The root is fixed, independent of TMPDIR, HOME, cwd, supplied name and parent
directory. All ordinary hard links share uid/device/inode, even across
directories; symlinks resolve to the same metadata. The owner directory is
private and verified before use. Failure to establish ownership refuses
startup; there is no alternate directory or unlocked fallback.

## Panel rulings

Both current run-local position files were read in full. The task directory
contains only older coverage-run framings; the matching intake is
`.brokkr/state/epic120/fixup-intake-framing.md`. Cached #120, #129, #130, #132
and #133, including all text in their evidence/comment sections, set scope.

| Question | Ruling and cost of the losing position |
|---|---|
| One or two guards? | Adopt both panels: retain the original pathname sidecar and add an identity sidecar. Two lifetime descriptors and inert files are the compatibility cost. |
| Bare shared file or private directory? | Adopt robustness's verified private directory keyed by the database owner, not the caller. Reject simplicity's bare file in world-writable `/tmp`: a planted symlink could redirect a lock back onto SQLite. This costs directory creation, metadata checks, refusal paths and exact-coverage tests. |
| Open every database to read identity? | Adopt robustness's metadata-only lookup for an existing file. Reject simplicity's extra open/close: closing any descriptor for that inode can release this process's POSIX locks during a refused in-process alias attempt. For a missing file, use exclusive creation and fstat; on an AlreadyExists race read metadata instead. This strengthens robustness's nonexclusive-create suggestion, which can still open a newly existing file. Cost: an explicit creation-race branch and stable-namespace assumption. |
| Keep connection-first destruction? | Keep it. Reject robustness's suggestion that ordering is merely tidiness: SQLite must finish closing/checkpointing before either ownership guard is released. Removing the database descriptor removes its special POSIX-close hazard, not the ownership lifetime requirement. |
| Temporary cleanup heartbeat? | Adopt simplicity's startup-only scope and robustness's documented-residual fallback. No timer or ignored-error refresh: it cannot prevent arbitrary unlink/replacement. Cost: operators must exclude the guard directory from age cleanup for the owner's entire lifetime; otherwise alias exclusion can be lost silently. |
| Canonicalization, hard-link refusal, SQLite EXCLUSIVE or data-file byte locks? | Reject all. Canonicalization cannot join hard links; rejecting nlink > 1 rejects a legitimate first owner; EXCLUSIVE breaks raw-writer proofs; byte locks still target SQLite and have process-wide close hazards. |
| Dependencies and platform scope? | Adopt simplicity: std-only local helpers, no production module, dependency, feature flag, migration or public knob. Linux/macOS are the delivery targets. Any unsupported target must refuse persistent startup rather than fall back to database locking or pathname-only exclusion. |

## Contract and limits

Ownership precedes SQLite schema/migration, store restore and listener bind,
and lasts until the final `Persist` closes. Only contention returns
`InUse(<supplied data path>)` with the existing exact message. Filesystem,
metadata, unsafe-namespace and unsupported-operation failures return existing
path/cause-bearing `Io`; SQL failures remain `Sqlite`. Acquisition is one
nonblocking attempt per guard, with no stealing, lock retry or unlink.

The alias guarantee covers cooperating current servers under the database
owner's credentials, in one shared, stable local data/guard namespace with
working advisory locks. Cross-user access may fail closed as `Io`, not InUse;
the uid key is the file owner's so different callers never choose independent
guards. Cross-container/private-tmp namespaces can split the rendezvous and
are not covered. The existing same-path sidecar still helps wherever shared.
Older binaries through different aliases, noncooperating writers, live file
replacement/chown, hostile same-user namespace mutation, unproven single-file
bind mounts and remote filesystems remain outside the proven scope.

Never remove guard files during an owner's lifetime. They remain inert after
drop/crash and may be cleared only while no server uses that namespace.
Age-based `/tmp` cleanup must exclude the directory; merely keeping a file
open is not a promise that a cleaner preserves it. Preserve the configured
database pathname across crash restarts: identity exclusion does not relocate
a hard link's WAL or recover mail lost by earlier concurrent writers.

## Acceptance Criteria

1. **No self-lock:** a Linux regression named
   `server_guards_do_not_lock_sqlite_files` opens a live persisted database,
   commits a row, and successfully takes short-lived exclusive `File::try_lock`
   probes on the database and its existing WAL/SHM companions. Assert the
   database/WAL/SHM cases were actually exercised; probe `-journal` if present,
   without manufacturing a journal. The database assertion fails with
   `794e919`'s guard and passes after repair. Do not run this probe on Darwin,
   where SQLite's legitimate record locks would invalidate the inference.
2. **SQLite coexistence:** `identity_lock_preserves_raw_sqlite_and_busy_bound`
   still proves raw `BEGIN IMMEDIATE`, write and commit beside the owner,
   followed by the owner's successful commit and exact 5000 ms busy default.
   Existing fresh-file/restore/lifecycle tests pass. Linux alone does not
   establish Darwin success: CI `test (macos-latest)` is the final proof.
3. **Aliases:** existing inline and real-process alias tests cover symlink,
   same-directory hard link and a hard link in a second directory on the same
   filesystem, both acquisition orders. Retain same-path, relative and
   symlinked-parent cases. A first owner with hard links already present is
   admitted; a contender is refused by supplied-path InUse before schema,
   restore or bind. Distinct files remain independently usable. Give rival
   child processes different TMPDIR values and prove exclusion still holds.
4. **Lifecycle and integrity:** keep first-owner health, acknowledged mail,
   cap/counter fidelity, graceful release, SIGKILL/reap reuse, final-owner
   lifetime and failed-open release proofs. Retarget simulated identity
   holders to the sidecar. A refused in-process alias must not release a
   live SQLite write transaction's locks; prove it with a separate-process
   raw contender and a deterministic transaction/child handshake.
5. **Safe startup:** test metadata failure, missing-file exclusive creation,
   a competing creator's AlreadyExists path, and no descriptor open on the
   existing-file identity path. Creation is nontruncating, no wider than
   0644 subject to umask; existing bytes/modes stay unchanged. Test directory
   creation/reuse, wrong owner/mode/type, symlink refusal and missing root;
   reject nonregular, symlinked or multiply-linked identity guards before
   locking. Every refusal preserves its path/cause and releases earlier guards.
   No new measured production branch may lack a deterministic test.
6. **Docs and proposal:** implementation updates the persistence module docs,
   README, decision index and superseded storage-rpc lock instructions to
   match 0047 §1. State the cross-directory argument, permissions, cleanup
   prerequisite, cross-user/namespace limits and Darwin/Linux proof distinction.
   Preserve 0047's proposed status and §§2–4 unchanged. Accepted rulings are
   never rewritten.
7. **Preservation:** #130/#132/#133 behavior and tests remain unchanged;
   synchronous FULL commit before acceptance, subscribe before scan,
   oldest-only pruning, ingest extraction, awaits cap, exact-count pins,
   webhook/Origin/envelope-sender/bridge protections, chart and publish guards
   remain intact. No Cargo, dependency allowlist, chart or bundle change.
8. **Evidence and delivery:** implementation records #129 alias reds against
   `2a325c9` separately from the macOS-fix Linux probe red against `794e919`.
   All final house gates and exact coverage pass, with pinned brokkr compile
   owned by verify and macOS proof owned by CI. Signed additive commits only
   on `epic120-storage-rpc`; no history rewrite, push, PR, merge or issue close.
   This design commit contains exactly this spec, plan, tasks and amended 0047.

## Handoff precedence

This design and amended 0047 §1 supersede only the database-lock mechanism,
its proof assumptions and obsolete branch-creation instructions in
`specs/storage-rpc/`. The older spec's #130/#132/#133 requirements stand.
Module/README claims are changed with implementation, not prematurely in this
design-only commit. The chief's artifact commit is required by this phase;
intake's earlier “one commit” estimate does not authorize squashing or skipping
the separate committed design. A drafted result certifies artifacts, not a
completed runtime repair or operator acceptance.
