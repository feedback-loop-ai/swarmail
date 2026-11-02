# Persistence — the restart copy

[← README](../../README.md) · [Guide index](README.md)

By default the store is memory — fast, and gone when the process exits.
With a data file, every insert, delete and clear is committed to bundled
SQLite (WAL, fsync-per-accept) and the full state — inboxes, messages with
raw bytes, the `emails_inserted`/`emails_dropped` counters — is restored
before the server answers anything:

```
swarmail serve --data-file /var/lib/swarmail/mail.db   # or SWARMAIL_DATA_FILE=…
```

Restored mail does not re-fire webhooks or waiters — it is
already-delivered state, not a new delivery.

**One server per data file** — a second process opening the same file
refuses at startup (a `<data-file>.lock` advisory lock held for the
process's lifetime). It exits non-zero with `the data file is already in
use by another server: <path>` before binding anything (decision
[0033](../decisions/0033-one-server-per-data-file.md)). The lock dies with
the process: a stopped or killed server's file is immediately reusable, and
no manual unlock ever exists — the `mail.db.lock` file left beside the data
file is inert and is never deleted. A replacement started while its
predecessor is still draining is refused until that predecessor exits;
retrying belongs to the supervisor. Restart reloads what is on disk; it does
not repair a file two servers already wrote together before this lock
existed (decision [0042](../decisions/0042-single-writer-lock.md)).

The refusal follows the *file*, not its name: besides the sidecar, the
server holds an identity guard,
`/tmp/swarmail-db-locks-<owner uid>/<device>-<inode>.lock`, named by the
data file's owner, device and inode — which every symlink and hard link of
the file shares, in any directory — so a second server reaching it through
an alias is refused the same way (decision
[0047](../decisions/0047-storage-rpc.md), issue #129). The root is always
`/tmp`, whatever `TMPDIR` says. The server never locks the SQLite database
or its `-wal`/`-shm` files itself: on macOS and the BSDs such a lock
conflicts with SQLite's own. The owner directory must be private (0700) to
the data file's owner and is never repaired; an unsafe directory or guard
refuses startup. Guard files are inert after the server stops and are never
deleted by it; exclude `/tmp/swarmail-db-locks-*` from age-based `/tmp`
cleanup while any server runs, and clear it only when none does. A data
file the server creates is no wider than SQLite's own 0644 (subject to
umask); an existing one keeps its mode and bytes. The guarantee covers
cooperating current servers running as the data file's owner, on a local
filesystem with working `flock`, sharing one `/tmp` (separate containers
with private `/tmp` mounts do not share guards; another user's server may be
refused with an I/O error rather than "in use"). Still **not** refused: an
older, sidecar-only swarmail reaching the file through a different alias
(the same path still collides on the sidecar); a writer that takes no lock
at all; replacing the file or its directory under a live server; single-file
bind mounts (unproven); and network filesystems, whose lock semantics vary.
Keep one configured path across
restarts — after a crash, SQLite looks for the `-wal` file beside the name
it was given, so restarting through another hard link's name would miss
the crashed server's last commits.

A missing ordinary data path is created. A dangling symlink refuses startup
as `OpenError::Io` for the supplied path with `ENOENT`; the symlink remains
unchanged and its missing target is not created. Relative data paths and
paths through a symlinked parent are admitted. When cooperating servers
share the stable namespace, either spelling of the same file is refused as
`InUse` while an owner holds it, with the rival's supplied spelling in the
error; it can be opened after that owner drops. These spelling checks prove
the existing ownership behavior without attributing the refusal to one
particular guard.

**Memory is not globally bounded** (decision
[0038](../decisions/0038-store-memory-and-timestamps.md)).
`SWARMAIL_MAX_PER_INBOX` caps messages per inbox — `0` disables it — and is
not a global byte or RSS cap: the inbox count is unlimited, and all
persisted rows restore before bind without application of that cap. A
resident mail costs its raw bytes, decoded bodies and extracted fields, its
index entries, any copy a watcher still holds, and the buffers of concurrent
requests; the data file's size on disk is not its size in RAM. Each distinct
awaited inbox name keeps a watcher channel — about 780 B per name observed
on one build, not a fixed ceiling — until that inbox is cleared or emptied
or the process restarts; awaiting the same name again reuses it. This is
separate from the 4,096-name cap on the await-metrics registry.

**`received_at` / `received_ms` are parse-start stamps**: sampled when
`build_email` begins, before parsing and before acceptance. They are not
the sender's `Date` and not a commit sequence, and need not sort in
acceptance order across sessions or clock steps — a large mail can be
stamped before a small one that is accepted first. Per-inbox order, POP3
numbering, oldest-only pruning and the data file's restore order all follow
acceptance. Restart is exercised by
`tests/persistence.rs`: the exact state comes back, and a clear racing a
250-answered insert never loses the answered mail.
