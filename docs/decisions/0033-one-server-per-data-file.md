# 0033 — One server per data file: the data file refuses a second process

- **Status:** accepted (operator, 2026-10-16 — "accept all")
- **Date:** 2026-10-16
- **Files:** `src/persist.rs` (`open` gains the sidecar lock and the named
  refusal), `src/persist.rs` (the refusal's inline test),
  `README.md` + `src/persist.rs`'s module doc (the contract sentence).
  **Not yet implemented** — filed by the run-11 prove-ship receipt
  ([specs/fidelity-prove-ship-receipt/](../../specs/fidelity-prove-ship-receipt/spec.md))
  at `791f83d`, where the tree still tolerates a second connection. The
  operator accepts **and** lands it in one commit (the refusal, its test,
  and the sentence together — never the sentence alone), or refuses it and
  nothing lands; the divergence risk below is then the recorded posture.
- **Implemented:** run 19 (issue #85, 2026-10-18) lands §§2–4 as written
  in one change — the `<data-file>.lock` refusal in `Persist::open`, its
  inline and real-process tests, and the §4 sentence in the module doc and
  README. The "Not yet implemented" note above is the state at filing,
  kept as history. See the dated addendum at the end for §1.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) (the lock is
  taken synchronously at open, before anything binds; `insert` gains no
  await and the mutation boundary is unchanged),
  [0003](0003-oldest-only-pruning.md) (untouched — one writer per file is
  what the eviction order already assumes),
  [0023](0023-off-worker-commits.md) (the commit stays the one wait; the
  lock is not a second one — it is taken once at open, off the hot path),
  [0025](0025-shared-domain-validation.md)/[0018](0018-shutdown-observed-waits-and-explicit-bounds.md)
  §3 (the refusal is named, not a bare error string).

## §1 The rule

Today nothing refuses a second server on one data file: `Persist::open`
takes a second `Connection` happily (`src/persist.rs:88`). SQLite's
`busy_timeout = 5000` (dependency-set, pinned by
`the_opened_connection_keeps_the_dependency_busy_default`) serializes the
*bytes*, so nothing corrupts and no commit is lost. What breaks is the
*semantic*:

- two live servers each hold a partial view; the file is the truth and
  the memory is the view, so a restart **corrects** that server's
  stale view (its `record_delete`/`record_insert` rows and the peer's
  commits are all in the shared database) — the standing hazard is a
  *live* peer serving rows the file no longer holds, and counters/seq
  that only reconcile at restart;
- `next_seq` collides across processes — restore stays deterministic
  (`(seq, rowid)`, #54), but "monotonic, no duplicates" becomes true only
  per-process;
- each process's prune plan reads its own memory: a cap eviction on
  server A leaves the evicted rows in B's memory until B's next restart
  corrects it — a stale live view, not a resurrection.

For a test sink this is a misconfiguration, not a hostile input — but the
tree's own refusal posture is that **misconfiguration fails loudly at the
boundary it enters** (a bad bind exits non-zero; a malformed TLS pair
refuses before bind; a corrupt row's type refuses at load). The second
server is the one startup input that is still quiet.

> **One server per data file.** `Persist::open` refuses — with a named
> startup error and a non-zero exit — when another process already holds
> the data file. A partial view of the data file is not a degraded mode;
> it is a lie the tree refuses to tell.

## §2 The mechanism: a sidecar advisory lock, std-only

`Persist::open` takes an exclusive advisory lock on a sidecar file beside
the data file — `<data-file>.lock` — via std's `File::try_lock`
(stabilized in Rust 1.89; the tree's rustc is 1.98, so **no new
dependency**). The lock is acquired synchronously on the open path,
before anything binds; a refused lock returns the named error (`the data
file is already in use by another server: <path>`), `Store::open` errs,
and `main` exits non-zero exactly as it does for an unusable file. The
kernel releases the lock when the process dies — a crashed server leaves
no stuck lock, only an inert sidecar file, and the data file is
immediately re-openable.

**Why not `PRAGMA locking_mode = EXCLUSIVE`** — the other zero-dependency
mechanism this run's robustness seat named: the tree's own proof family
for [0001](0001-synchronous-insert-lossless.md) is built on a **second
connection to a live data file**. `tests/common/mod.rs::hold_write_lock`
(`BEGIN IMMEDIATE`, used at `tests/lifecycle.rs:447` and `:568`) stalls a
live commit to prove a burned commit lands losslessly; the
mirror-inspection legs that open the data file directly
(`tests/lifecycle.rs:505, 520, 621, 849`) would have to move behind the
store's connection close or route through the API, since an EXCLUSIVE
store holds the file for its lifetime; and
`src/persist.rs::load_surfaces_a_corrupt_row_type` vandalizes a row
through a second connection (`:341`). EXCLUSIVE locking breaks all of it
and would demand a stall seam plus a rework of the frozen guarantee's
proof family — machinery churn in the one place the realm cannot afford
it. **Cost of losing EXCLUSIVE:** one inert sidecar file beside the data
file. The store never unlinks it (unlinking a held lock file is a race;
it lingers and is harmless). **Cost of the alternative:** the 0001 proof
family rebuilt. The sidecar wins because it refuses exactly the thing the
rule names — a second *server*, the only thing that opens the file
through `Persist` — and touches nothing else.

## §3 The test

The refusal is testable in-process, deterministically, with no second
process and no sleep: `flock` is per-open-file-description, so a second
`Persist::open` on a held file fails from the same process. The inline
`#[cfg(test)]` test (0020 rule 1 — both branches on the green path,
measured): open a `Persist`, then `Persist::open` the same path again;
the second open errs with the named sentence. The existing
second-connection tests are **unaffected** — they open raw rusqlite
connections and never the sidecar lock — so the vandal, the mirror
reads, and `hold_write_lock` stay exactly as written. The new production
lines ship with this test; the coverage gate stays exact.

## §4 The contract sentence, landed with the refusal

The module doc and README state the rule where every other contract
lives (#57's discipline — the sentence lands in the same commit as the
refusal, never before it):

- `src/persist.rs` module doc: "**One server per data file** — a second
  process opening the same file refuses at startup (a `<data-file>.lock`
  advisory lock held for the process's lifetime)."
- README's data-file section: the same sentence plus the crash note —
  the lock dies with the process; a stopped or killed server's file is
  immediately reusable, and no manual unlock ever exists.

## §5 What this decision does not do

- It does not add a dependency, a format change, a version gate, or a
  migration matrix; old data files open as today and gain only an inert
  sidecar file at first open.
- It does not renew, heartbeat, or otherwise keep the lock alive
  artificially — the kernel owns the lock's lifetime, which is why a
  crash cannot strand a data file.
- It does not change the busy bound, the journal mode, `synchronous`,
  or the mutation boundary; 0001's shape is untouched and re-proved by
  the standing suites.
- It does not forbid a second *connection*: tests keep their
  second-connection machinery by design. The rule is one **server** per
  file, and the server is the only thing that opens the file through
  `Persist`.

## Addendum (2026-10-18, run 19 — issue #85)

§1 said a restart "corrects" a two-server state and that counters and
sequences "reconcile at restart". Issue #85 disproved that: two live servers
on one file under a cap of two accepted five mails and left four rows, a
collided sequence, and an inserted counter of three, all of which a restart
reloads as they are. Restart reloads persisted state; it does not
reconstruct the independent writers' event history. That §1 passage is kept
as the historical claim and is not a current recovery promise. The
correction, and the typed `persist::OpenError` the implementation carries,
are recorded in [0042](0042-single-writer-lock.md) — *proposed*, awaiting
the operator; this addendum does not accept it.
