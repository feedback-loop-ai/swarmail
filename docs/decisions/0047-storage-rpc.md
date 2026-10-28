# 0047 — File identity, disjoint overflow and correlated RPC refusals

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance is the operator's.
- **Date:** 2026-10-18.
- **Scope:** Epic #120 batch 2, issues #129, #130, #132, #133.
- **Spec:** [storage-rpc](../../specs/storage-rpc/spec.md).
- **macOS fix-up (2026-10-18):**
  [macos-identity-lock](../../specs/macos-identity-lock/spec.md) controls §1
  and supersedes the original spec's data-file-lock instructions. This is
  an in-place amendment of a proposal, not operator acceptance.
- **Supplements:** accepted [0033](0033-one-server-per-data-file.md) §2's
  sidecar mechanism and proposed [0042](0042-single-writer-lock.md) §§1/5's
  lifetime, error and alias scope.
- **Proposes amendment to:** accepted
  [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
  §2's missing-ID notification predicate.
- **Updates proposals:** [0039](0039-stdio-concurrency.md) §§1/2/6,
  [0043](0043-mcp-origin-boundary.md) §6 and
  [0044](0044-compat-validation.md) §§1/2. No earlier status is changed.

## Context

The current sidecar follows the supplied name, so two servers can reach one
file through different aliases. Symlink writers violate cap/accounting;
hard-link writers can lose acknowledged mail through separate WAL files.
The commission requires exclusion by file identity, beyond the issue's
canonicalization suggestion. `_uncounted` is a valid inbox and cannot also
identify overflow. An id-less malformed object is not a notification. The
stdio bridge knows IDs before sending, yet portless authorities fail dialing
and body-limit responses can discard that correlation.

## Proposed ruling

### 1. Own an identity sidecar as well as the existing pathname sidecar

**Diagnosis and amended mechanism (macOS fix-up, 2026-10-18).** The initial
implementation at `794e919` took an exclusive std `File::try_lock` on the
database itself before opening SQLite. On ordinary Linux local filesystems,
flock and POSIX fcntl record locks are independent. Darwin/BSD advisory-lock
interfaces interoperate: flock is owned by the open file description, while
SQLite's POSIX locks are process-owned. They can conflict even within our
process. SQLite's write-lock request while selecting WAL can therefore fail
behind our own flock, explaining the reported macOS DatabaseBusy failures.
A shared flock is not a repair: it still conflicts with a write lock.
Network-filesystem emulation can also invalidate Linux's independence.
This is the lock-semantics reasoning, not a claim of a local Darwin run.

Application locking must never target the database or its `-wal`, `-shm` or
`-journal` companions. SQLite continues to take its own locks. Retain 0033's
exact supplied-path `.lock` acquisition, nontruncation, creation-mode ceiling,
nonblocking attempt and never-unlink behavior. Replace only the second guard
with the following Unix identity rendezvous:

`/tmp/swarmail-db-locks-<database-owner-uid>/<device-hex>-<inode-hex>.lock`

Read an existing database's uid/device/inode with `std::fs::metadata`, following
database symlinks without opening a descriptor. Opening and closing an extra
descriptor could release this process's POSIX locks during an in-process
refused alias open. If absent, exclusively create a regular file read/write,
nontruncating, no wider than SQLite's 0644 subject to umask; read its opened
handle's metadata and close it before SQLite. If a competing creator wins
(AlreadyExists), read metadata once without opening the existing file.
Nonexclusive create after NotFound would reintroduce the close hazard.
Existing file permissions/content are unchanged. Namespace replacement
during lookup/open is outside the stable-namespace contract.

The fixed root is not derived from temp_dir(), TMPDIR, HOME, cwd, a database
parent or the calling user's identity. Hard links in different directories
share the same file-owner/device/inode tuple and therefore the same guard;
neither canonicalization nor a parent-local sidecar can provide this property.
Linux and macOS are the target proof platforms. An unsupported identity
platform must refuse persistent startup with Io, never fall back to locking
SQLite or to pathname-only ownership.

Create the owner directory with mode 0700; before using an existing directory,
require symlink_metadata to show a real directory, owned by the database
owner, with private owner-access permissions (0700). Otherwise refuse with
path/cause-bearing Io; do not repair permissions or choose another root.
Create identity guards exclusively at 0600 or validate existing entries before
opening: regular, singly linked, owned by the database owner, no group/other
access, not a symlink. Validate the opened metadata before locking too.
This prevents planted symlinks/hard links from redirecting the identity lock
onto SQLite and rejects FIFOs before a blocking open. The verified private,
stable parent is the trust boundary, not a promise against hostile same-user
replacement. Creation races converge to a validated existing entry.

Take one nonblocking exclusive File::try_lock on the identity sidecar. Either
guard's contention gives the existing exact named InUse refusal for the
supplied database path; all other filesystem/metadata/validation failures
give path/cause-bearing Io. SQL failures remain Sqlite. Startup may leave an
empty file after failure but never writes database contents before ownership.
Both guards precede schema changes, restore and binding, and remain held for
Persist lifetime. Store both handles after connection state: SQLite must
finish closing/checkpointing before ownership releases. No guard is deleted
on drop, crash or failed startup. Raw SQLite writers, WAL/FULL, the exact
5000 ms busy default and mutation ordering stay unchanged. No SQLite
EXCLUSIVE mode, lock retry, stealing, lease or heartbeat.

The alias contract covers cooperating current servers under the database
owner's credentials, on supported local filesystems sharing one stable
data/guard namespace. Same path, relative spellings, symlinked parents,
symlink files and hard links (including another directory) are refused in
both acquisition orders. A first owner is admitted with hard links present.
Cross-user access may refuse as Io rather than InUse; it never selects a
separate caller-keyed guard. Cross-container/private-tmp mount namespaces can
split the identity rendezvous and are outside this guarantee. The original
sidecar still retains same-path compatibility where its namespace is shared.

Guard files are inert after release but persist per distinct file identity.
Never unlink them while any server uses the namespace. Operators on hosts
with age-based `/tmp` cleanup must exclude this directory for the entire
owner lifetime: removing a held guard allows a new inode and can silently
admit an alias rival. No heartbeat is claimed to close arbitrary deletion.
The directory may be cleared only when no server uses it.

Old sidecar-only binaries through distinct aliases, noncooperating writers,
live data/lock replacement or chown, hostile same-user namespace mutation,
unproven single-file bind mounts and remote lock semantics remain outside
the proven contract. No universal filesystem or distributed-lease promise.
Restart reloads surviving state; it does not repair prior multi-writer loss.
Keep the configured database pathname stable across crash restarts: identity
exclusion does not relocate a crashed owner's WAL to another hard-link name.

**Proof:** Linux-only `server_guards_do_not_lock_sqlite_files` must take fresh
exclusive flock probes on a live database and real WAL/SHM files, plus a
journal if present. Its database case fails on `794e919`, passes with this
repair, and is not run on Darwin where SQLite's own locks can block it.
`identity_lock_preserves_raw_sqlite_and_busy_bound` proves raw BEGIN IMMEDIATE,
commit, subsequent owner commit and the 5000 ms default. That witness passes
on Linux before and after; it is not the Linux baseline-red proof. Preserve
and widen both-order inline/real-process alias tests to another directory
and different child TMPDIRs, guard-release and startup-order proofs, and a
separate-process witness that refused in-process aliases do not drop SQLite
write locks. Original #129 alias reds remain against `2a325c9`; the macOS
self-lock regression was introduced later. CI `test (macos-latest)` after
the operator's assistant pushes is the final Darwin proof; it is pending
at design time.

**Why the repair holds on Darwin (reasoning, not a Darwin run).** Darwin's
advisory locks are kept per vnode; flock and fcntl requests on the *same*
vnode meet in one list and conflict. Locks on different vnodes never
interact. After the repair our only flocks are on two files SQLite never
opens — `<path>.lock` and the identity guard under
`/tmp/swarmail-db-locks-<uid>/` — and SQLite's record locks fall on the
database and `-shm` alone, so no request of ours can share a vnode with a
request of SQLite's. A planted symlink or hard link cannot bring the
database's vnode back under our flock: the guard must be a singly linked
regular file (not a symlink) in a private directory, checked before and
after open. The existing-file identity read is a `stat`, not an open, so
no descriptor of ours on the database can be closed under SQLite's POSIX
locks either — the same per-process close rule holds on Darwin and Linux.

**Implementation receipt (2026-10-18, Linux).** The journal case is an
assertion that WAL mode left no `-journal`, not a conditional probe. The
separate-process witness is `refused_aliases_preserve_the_owners_sqlite_locks`
in `tests/sqlite_locks.rs`, alone in its binary (a child forked beside a
sibling test briefly shares that test's flocked descriptors, which made an
in-crate placement flaky): with a live owner and a raw writer of the same
process in a write transaction, refused hard-link and symlink opens must
leave a zero-timeout raw child refused both a write and a switch out of WAL
(which needs the database file's exclusive lock); after the commit the
child writes, and the owner's shared lock still refuses the WAL switch. It
uses only the public `Persist` API. Against
`794e919`'s data-file lock both new tests are red on Linux — the database
probe reports `WouldBlock`, and the child's WAL switch succeeds beside a live
owner because the refused alias closed a descriptor on the database — and
both are green with the identity sidecar.

### 2. Give overflow an impossible inbox name

Use exactly one LF character as the overflow key. It is nonempty and refused
by the existing shared inbox validator; do not change that validator or
reserve `_uncounted`. Keep the current bounded map and existing-name increments:
4096 tracked real names plus at most one folded row. Compute the cumulative
total and render its rows under one lock, replacing the current separate
total/read locks so each scrape reconciles exactly. Other metric families'
externally maintained total semantics remain unchanged.
The Prometheus encoder renders overflow as `inbox="\n"`, distinct from the
legal literal backslash-n name (`inbox="\\n"`). A tracked `_uncounted` keeps
only its own count. New names first seen after capacity have no own row and
fold exactly; the cap limits attribution, never admission. README documents
the selector migration and exact folded/total accounting.

### 3. Share the notification predicate

An object is silent only if it lacks an `id` key and has a string `method`.
Supplied IDs, including null, retain their existing behavior. Other id-less
objects owe -32600/null ID; non-objects retain their existing refusal. Put
the rule in `reply_id` and use it for both classification and handle so HTTP
and stdio agree, including transport errors and admitted batch positions.
Keep string-method notifications effectful and silent even for unknown
methods. Empty/over-16 batch refusal and ordered concurrent batch replies
stay unchanged. This does not add version, ID-domain or params validation.

### 4. Default dialing and refuse oversized bridge bodies locally

An omitted HTTP authority port means 80 for dialing, with bracketed IPv6
handled explicitly and malformed supplied ports refused. Preserve the
original Host authority spelling and existing URL/path/deadline behavior.

After existing parse/classification, compare the exact body bytes that would
be posted with the shared 2,097,152-byte MCP limit. Above it, do not dial or
post: use -32600 with shared HTTP size-refusal wording, one error for Single,
one ordered array for admitted Batch reply positions, one null-ID error for
InvalidBatch and silence for Silent. Preserve ID values/duplicates and omit
notification members. Equality is admitted. Malformed JSON/UTF-8 still gets
-32700; blank lines stay silent; subsequent lines continue. No retry.

Do not re-address upstream null-ID responses in this batch. A lower-limit
remote server/proxy may still produce an unmatchable forwarded error. The
local limit bounds outgoing bodies, not stdin allocation: reading and parsing
large lines remain unbounded, and slow stdout can still stall the bridge.

## Alternatives and costs

For the macOS fix-up, adopt both panels' two-sidecar identity mechanism and
std-only scope. Adopt robustness's verified private owner directory over
simplicity's bare `/tmp` file: an unverified shared entry can redirect the
lock onto the database. Cost: directory validation and covered refusal paths.
Adopt metadata-only existing-file lookup over simplicity's opened descriptor;
strengthen robustness's creation step to exclusive create plus metadata on
AlreadyExists. Cost: a creation-race branch and stable-namespace assumption.
Retain connection-first destruction despite robustness calling it tidiness:
ownership must outlive SQLite close. Adopt simplicity's startup-only scope
and robustness's documented-cleaner fallback over a refresh timer. Cost:
operators must prevent live cleanup; no timer would close arbitrary unlink.

Reject canonicalization as insufficient for hard links, nlink > 1 refusal
as rejecting a legitimate first owner, data-file fcntl bytes as violating
the no-SQLite-file-lock rule and vulnerable to descriptor close, and SQLite
EXCLUSIVE as breaking raw-connection proofs. Accepted costs are two held
sidecar descriptors, persistent inert guards, a fixed writable shared-root
prerequisite, and explicit cross-user/private-namespace limits. No new
dependency, public configuration or lock-management service.

Adopt simplicity's LF sentinel over robustness's separate metric family.
The validator already supplies a disjoint domain, so structural registry and
metric changes are unnecessary. Cost: unusual but valid escaped label,
dashboard migration and tests coupling validation with encoding. Reserving
`_uncounted` would revoke access to a legal name and is refused.
Adopt robustness's consistent snapshot requirement: it costs a small rendering
change even though its separate-family design is declined.

Adopt both panels' shared reply predicate and local size refusal. Reject a
handler-only fix, input-byte truncation and upstream error rewriting. The
costs are newly visible errors for malformed objects, continuing unbounded
input memory and the named version-skew correlation residual. Keep a real
port-80 acceptance proof despite its environment cost; helper-only evidence
cannot establish the issue's explicit criterion.

## Preservation, proof and delivery

Preserve 0001/0002/0003/0005, all protected batch-1 and earlier features,
license policy and exact coverage. Spec AC1–AC10 and its paired tasks require
Rust/local and real-process alias proofs, exact registry/encoding tests,
real HTTP/stdio errors and port-80/oversize exchanges, baseline-red receipts
and final gates. No production line is excluded from measurement.

The original design contained this proposal and the three storage-rpc
artifacts. The macOS fix-up design contains this amended proposal and only
the three macos-identity-lock artifacts. Implementation reconciles the
decision index, previous spec lock instructions and misleading proposal
pointers, preserving accepted text/status; it lands README/module claims
with code and tests. §§2–4 and their tests remain unchanged. None of these
documents constitutes operator acceptance or completed macOS verification.

## Dated amendment — supplied JSON-RPC ID domain (2026-10-24)

The operator's directive supplied for issue #180 on 2026-10-24 amends this
accepted decision's §3 ID behavior: only JSON string, number and null IDs
are valid. A supplied Boolean, object or array ID is an invalid request;
answer it with JSON-RPC error `-32600` and `id: null`, before method
dispatch, tool effects or waiter registration. Such an invalid supplied ID
is not a notification. Valid string, number (including negative and
fractional numbers) and explicit null IDs retain their JSON values.

This amendment supersedes §3's unrestricted supplied-ID behavior and the
historical absence of ID-domain validation. It does not change the existing
missing-ID notification predicate: only an object with no `id` key and a
string `method` is a notification, and a valid notification remains
effectful and silent. The shared classifier normalizes invalid supplied
IDs to null reply positions; the handler independently refuses them before
effects. Batch ordering, duplicate and explicit-null positions, concurrent
admission, bridge transport/size error codes, and verbatim complete
upstream replies remain as previously ruled.

**Provenance:** the exact ID-domain choice above was supplied by the
operator in the 2026-10-24 issue #180 delivery task. Proposed decision
0062 records its implementation framing; this dated addendum preserves this
decision's accepted status and historical text.

## Proposed reconciliation pointer — 2026-10-26

The operator-directed #152 implementation is recorded in proposed
[0079](0079-mcp-version-negotiation.md) and proposed
[0080](0080-mcp-source-reconciliation.md). §§3–4 modern arrays refuse before effects and initialization installs only a supported, matching, complete required result. The accepted supplied-ID amendment, legacy duplicate/null slots and verbatim complete replies remain authoritative; null-ID outputs and unsolicited-response status remain unresolved source conflicts.
This dated pointer preserves the accepted historical body and status. It neither
accepts a proposal nor claims an incompatible amendment has been accepted.
See the [clause ledger](../../specs/mcp-version-negotiation/reconciliation.md)
for source evidence, compatible repairs, remaining costs and external ownership.
