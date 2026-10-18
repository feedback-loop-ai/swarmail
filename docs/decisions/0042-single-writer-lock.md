# 0042 — Single-writer startup errors and the limit of restart recovery

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-18
- **Supplements:** accepted [0033](0033-one-server-per-data-file.md) §§2–4,
  whose std sidecar mechanism, refusal, test and contract sentence stand.
- **Amends:** 0033 §1's claim that restart corrects multi-server accounting
  and cap divergence; records the public open-error type change.
- **Spec:** [single-writer-lock](../../specs/single-writer-lock/spec.md).

## Context

Issue #85 reproduced two live servers on one file accepting five mails while
persisting four rows under a cap of two and an inserted counter of three.
Each server's absolute counters and prune plan came from its partial view.
Reloading those rows and counters cannot recover the missing event history.
0033 is accepted but its mechanism is absent at this design's base; its index
claims otherwise. Implementation metadata must become truthful with the
refusal/test/documentation commit, not through a documentation-only claim.

The two public open functions currently return rusqlite errors. Sidecar
contention and filesystem failure are not SQLite errors. Both panel positions
agree on 0033's mechanism but disagree on preserving those signatures.

## Ruling

1. Implement 0033 §§2–4 literally: append `.lock` to the full OS path, open
   without truncation, attempt one exclusive std `File::try_lock` before
   opening SQLite, and retain its File in `Persist` after the connection
   state field. Ordinary destruction closes the connection before releasing
   ownership. Never unlink, retry, block on lock acquisition or silently
   proceed without a lock.
2. `Persist::open` and `Store::open` return a public `persist::OpenError`
   distinguishing `InUse(PathBuf)`, `Io { path, source }` and
   `Sqlite(rusqlite::Error)`. Only `TryLockError::WouldBlock` means InUse.
   Its Display is exactly `the data file is already in use by another
   server: <path>`. Other failures retain their cause and path context and
   refuse startup. Store restore errors convert to Sqlite; mutation methods
   and `Persist::load` retain their existing rusqlite return types.
3. Existing startup handling carries the named refusal to nonzero exit
   before readiness. `run_on` may prefix it with existing data-file context.
   Unsupported locking is an I/O refusal, not a degraded operating mode.
   A supervisor overlapping a draining predecessor may see repeated refusals
   until the final owner drops; retry belongs to the supervisor.
4. **Restart reloads persisted state; it does not reconcile independent
   writers' event history.** Duplicate sequences, overwritten counters and
   accumulated over-cap rows may survive restart. This repair prevents new
   cooperating same-path owners from overlapping; it adds no historical
   data repair or restore-time pruning. 0033 §1's contrary claim is superseded
   by this correction if accepted, and the implementation records a dated
   addendum rather than silently replacing accepted history.
5. Ownership follows `Persist` lifetime. A library client's surviving
   `Arc<Store>` can retain the lock after listeners stop. Process exit releases
   handles automatically; a leftover unlocked sidecar is inert and reusable.
   Raw SQLite connections remain possible, preserving 0033's proof family.
   Stable common pathnames and a cooperative, unmodified sidecar namespace
   are the scope; alias identity, hostile replacement, old binaries and
   distributed leases are not promised.

## Alternatives and costs

Simplicity's rusqlite error wrapping loses to a named startup type because
callers should distinguish admission, I/O and SQL failures without parsing a
string. The chosen type costs source compatibility for callers explicitly
expecting rusqlite results; it is not a zero-cost signature change. Keep the
change confined to startup and preserve the underlying SQL cause.

SQLite EXCLUSIVE loses because it blocks the existing raw-connection
corruption, inspection and stalled-commit proofs. Cross-process reconciliation,
lock stealing and retry machinery lose because they expand the state machine
instead of closing admission. Costs accepted: one persistent inert sidecar,
directory access, failure on unsupported locking filesystems, and explicit
overlap refusals. No new dependency, migration or weakened guarantee.

## Proof and delivery

Inline deterministic tests prove exact typed refusal, lock-before-database,
correct suffix/nontruncation, drop and failed-open release, error causes and
raw-connection compatibility. Real serve processes prove nonzero refusal,
continued first-owner service and graceful/kill reuse without sleep or
sidecar deletion. The missing-parent test covers file open failure, not a
failed lock syscall; test both lock-error conversion cases explicitly.

Preserve synchronous accept-after-store, subscribe-before-check, oldest-only
pruning, ingest extraction, WAL/FULL, mutation ordering and the exact 5000 ms
busy-default tripwire. All six house gates remain required. README/module
sentence, behavior and tests land together; implementation metadata and the
decision index name the actual state and retain this record as proposed.

## Pointer — 2026-10-18 (proposed 0047)

[0047](0047-storage-rpc.md) §1, also **proposed**, updates §§1/5 here (issue #129): the
lock is held on the data file's identity as well as on the sidecar — after the macOS
fix-up of the same day, an identity sidecar named by owner, device and inode, never a
lock on the database itself — so §5's "stable common
pathnames" limit no longer covers symlink and hard-link aliases between current servers.
The residuals that remain — old sidecar-only binaries through other aliases, writers that
take no lock, live namespace replacement, single-file bind mounts, remote filesystems and
restarting a crashed owner through another hard link's name — are named in 0047, the
README and the persistence module.
