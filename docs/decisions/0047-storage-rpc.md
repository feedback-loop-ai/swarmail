# 0047 — File identity, disjoint overflow and correlated RPC refusals

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-18.
- **Scope:** Epic #120 batch 2, issues #129, #130, #132, #133.
- **Spec:** [storage-rpc](../../specs/storage-rpc/spec.md).
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

### 1. Own the file as well as the existing sidecar

Retain 0033's exact supplied-path sidecar acquisition, nontruncation,
nonblocking attempt and never-unlink behavior. Before SQLite open, also
open/create the data file without truncation and take one exclusive std
`File::try_lock` on that file. On Unix preserve SQLite's 0644 creation ceiling
subject to umask; do not alter existing permissions. Store both handles after
connection state so the connection drops first. Both guards must precede
schema changes, restore and binding and remain held for `Persist` lifetime.

Either lock's contention gives the existing named InUse refusal; other
failures give path/cause-bearing Io refusal. Explicit data-file open can
therefore change a former Sqlite startup error into Io. Startup may leave an
empty file after failure but never writes database contents before ownership.
Raw SQLite connections, WAL/FULL, the 5000 ms busy default and mutation
ordering stay unchanged. No SQLite EXCLUSIVE mode, retry or lock stealing.

Cooperating current servers refuse ordinary symlink and hard-link aliases on
supported local filesystems. The same-path sidecar retains compatibility
with the previous sidecar-only binary. Those old binaries through distinct
aliases, noncooperating writers, replacement of a live data/lock namespace,
unproven single-file bind mounts and remote lock semantics remain outside
the proven contract. No universal filesystem or distributed-lease promise.
Restart reloads surviving state; it does not repair prior multi-writer loss.
Keep the configured database pathname stable across restarts: locking the
inode does not relocate a crashed owner's WAL to another hard-link basename.

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

Adopt both panels' identity guard, simplicity's unchanged sidecar name and
robustness's creation-mode discipline. Canonicalization alone loses the
hard-link requirement; adding it to identity locking adds complexity without
closing a required gap. Cost: one additional held descriptor, startup
open/lock, carefully ordered destruction and possibly multiple inert alias
sidecars. Reject robustness's claim that old binaries hold neither lock:
the preceding implementation already holds a sidecar, but not an identity lock.

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

The design change contains this proposal and its three spec artifacts only.
Implementation adds the decision index entry and dated pointers in earlier
records with their text/status intact; it lands README/module claims with
code and tests. None of those pointers constitutes operator acceptance.
