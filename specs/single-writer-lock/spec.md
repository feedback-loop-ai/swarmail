# Single-writer lock

Run 19, issue #85. Design handoff; implementation and its verification remain
owed. Implements accepted [0033 §§2–4](../../docs/decisions/0033-one-server-per-data-file.md)
as written. Proposed [0042](../../docs/decisions/0042-single-writer-lock.md)
records the public startup error type and corrects 0033 §1's restart claim;
only the operator accepts that supplemental decision.

## Feature and rationale

Exactly one cooperating server may own a persistent data-file path at a time.
Acquire an exclusive, nonblocking std `File::try_lock` on `<data-file>.lock`
at `Persist::open`, before SQLite is opened and before any listener binds.
Keep the handle with `Persist` until its connection closes. Contention refuses
startup with `the data file is already in use by another server: <path>`
and a nonzero process exit. A stopped or killed owner's file is immediately
reusable after that owner releases its handle; no manual unlock exists.

Today two servers independently allocate sequences, overwrite absolute
counters and prune partial views. Issue #85 observed five accepted mails,
four stored rows under cap two, and inserted_total three after restart.
SQLite serialization does not serialize these in-memory plans. Restart loads
the damaged accounting; it cannot reconstruct missing counter history or
promise cap repair. Refuse the second owner instead of reconciling writers.

The framing sources are `.brokkr/state/run19-feature.txt` and the verbatim
issue at `.brokkr/state/epic70/85.md`. The three files under
`.brokkr/state/tasks/` frame historical run-12 work, not this feature.
Both current panel positions were read; they are ignored run-local evidence.

## Panel rulings

| Question | Ruling and reason | Cost of the losing choice / cost accepted |
|---|---|---|
| Scope and mechanism | Adopt both positions: one std advisory sidecar owned by `Persist`; no new dependency, lock service, recovery protocol, retry, or SQLite EXCLUSIVE mode. | EXCLUSIVE would break the live raw-connection proof family; the chosen mechanism leaves one inert sidecar and requires a directory where it can be opened/created. |
| Error representation | Adopt robustness's typed `OpenError`, rejecting simplicity's encoding into a borrowed rusqlite error variant. Restrict the change to startup; mutation errors remain rusqlite errors. | Preserving signatures would conceal a filesystem/admission failure as a SQLite failure. The typed choice costs a public return-type change and conversions; robustness's claim of zero caller cost is rejected for downstream callers. |
| Admission ordering and lifetime | Adopt both positions' lock-first ordering, OS-string suffix append and connection-before-lock field destruction. | Locking after schema work lets a refused process mutate the database. A lock local to `open` protects no running server. |
| Other I/O failures | Adopt robustness's distinction: only `WouldBlock` is `InUse`; all other failures are loud and retain their I/O cause. | No startup on a filesystem lacking supported advisory locking. Reporting every failure as contention would misdiagnose it; ignoring one would restore the bug. |
| Proof size | Adopt simplicity's existing inline and binary suites, with robustness's explicit error classification, lock-first and lifetime assertions. No new harness is required by this design. | A two-open test alone misses process exit/crash release. More tests cost maintenance but each pins a different failure. |
| Error-arm coverage | Reject robustness's claim that a missing directory exercises `TryLockError::Error`: it fails before the syscall. Exercise the pure conversion with an explicit error value, plus real open failures and real contention. | Requires a small tested conversion function; moving a test to a harness would not make unexecuted production code covered. |
| Restart policy | Adopt immediate refusal, leaving retry to the supervisor. | A replacement started before its predecessor releases ownership can fail on every overlapping attempt, not necessarily exactly once as robustness suggests. No arbitrary delay is added to startup. |
| Decision history | Adopt simplicity's dated addendum discipline; update implementation metadata only with the implementation. | Historical wording stays readable but must be explicitly identified as superseded evidence, never repeated as a current recovery promise. |

## Requirements and boundaries

The sidecar suffix is appended to the full `OsString` path (for example,
`mail.db` becomes `mail.db.lock`), without replacing its extension or requiring
UTF-8. Open/create without truncation. Contents, PID and age convey no
ownership; an existing unlocked sidecar is reusable. Production never deletes
it, including during shutdown. Failure at any later startup stage releases
ownership through ordinary destruction. In-process `Arc<Store>` owners retain
the lock until the final owner drops, even if listeners have already stopped.

`Persist::open` and `Store::open` expose `persist::OpenError` with named
`InUse`, `Io`, and `Sqlite` cases. The `InUse` display is exactly 0033's
sentence including the data path; I/O errors identify the failing sidecar
and preserve their cause. Existing `run_on` startup context may prefix that
sentence and repeat the path. SQL open/restore failures remain distinguishable.
No change to the runtime mutation result types or wire failure contracts.

Protection is advisory and assumes cooperating binaries, a stable common
data-file pathname and an unmodified sidecar directory. Raw rusqlite clients
remain possible by design. Arbitrary hard-link/symlink aliases, hostile
replacement, old noncooperating binaries and distributed lease guarantees
are outside this repair. It does not repair already-divergent databases.
Memory-only servers acquire no sidecar. Existing WAL/FULL settings, the
dependency's 5000 ms busy timeout, mutation ordering, watcher ordering,
oldest-only pruning and ingest extraction remain unchanged.

## Acceptance Criteria

- **AC1 — Inline admission:** retain one `Persist`, open the identical path
  again and assert the `InUse` variant and exact diagnostic; drop the owner,
  reopen successfully with the same sidecar still present. Hold a raw
  rusqlite connection alongside the first owner successfully.
- **AC2 — Lock before database access:** hold the sidecar lock directly while
  the database does not exist. `Persist::open` refuses with `InUse` and the
  database remains absent. Release the holder and the normal open succeeds.
- **AC3 — Path and sidecar semantics:** `mail.db` and `mail.sqlite` can be
  owned concurrently, creating distinct literal-suffix sidecars. A preexisting
  unlocked sidecar with sentinel contents is reused without truncation;
  dropping `Persist` leaves it present. A fresh store reports no inboxes,
  no mails and zero inserted/dropped counts despite the sidecar.
- **AC4 — Honest failures and cleanup:** an unusable sidecar path fails as
  `Io`, not `InUse`; explicit `WouldBlock` and `Error(io)` conversion tests
  preserve their distinct variants, paths and underlying causes. A malformed
  database fails as `Sqlite`; after that failed open the sidecar is immediately
  lockable again. Display and source paths added by the implementation execute
  under the exact coverage gate.
- **AC5 — Real processes:** after real server A announces readiness on
  ephemeral ports, real server B with the same data path exits nonzero under
  the existing helper deadline, emits the named refusal and announces no
  readiness. A still answers health and can accept/query a mail. Killing and
  reaping A permits server C to start on the unchanged path immediately,
  retaining the accepted mail. Graceful stop/reap also permits reuse. No
  sleep, sidecar removal, startup retry or cooldown may make these tests pass.
- **AC6 — Existing guarantees:** persistence, lifecycle, burst and rate tests
  pass, including raw-connection corruption/mirror/stalled-commit tests,
  acknowledged-mail hard-kill durability and the exact `busy_timeout = 5000`
  pragma tripwire. In-process owners must be dropped before reopen; the
  lock must never be released early just to preserve a fixture.
- **AC7 — Documentation lands with behavior:** README and the persistence
  module doc contain 0033 §4's exact contract sentence. README explains
  crash/graceful reuse, inert sidecars, no manual unlock and overlap refusal.
  In the implementation commit, 0033's status metadata and index describe
  the actual implementation and a dated addendum corrects its historical
  restart-reconciliation claim by referencing proposed 0042. Neither this
  design nor implementation self-accepts 0042.
- **AC8 — Delivery evidence:** all six house gates pass, with exact nonzero
  100/100/100 coverage, no exclusions or floor changes, no new dependency,
  no TMPDIR override and no changes to bundles or gate policy. Record actual
  results; an unavailable tool or sandbox refusal is not a passing gate.

The design commit contains only this spec, its plan/tasks and proposed 0042.
The later implementation commit contains the refusal, tests and contract
sentences together, as 0033 requires. This seat does not push or close issues.
