# 0010 — Commits run off the async workers: the disk leg is ordering, not thread identity

- **Status:** proposed (awaiting operator acceptance)
- **Date:** 2026-10-11
- **Supersedes:** none
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) — accept == stored, unweakened

## Context

The review found that with `--data-file` the synchronous SQLite mirror commit runs inside async
worker threads: the SMTP session task calls `store.insert(email)` (`src/smtp.rs:455`), the
mutation mirrors synchronously (WAL + `PRAGMA synchronous = FULL`, `src/persist.rs:84`), and the
`250` is written on the next line. The ordering is correct — decision 0001's disk leg holds — but
the fsync executes on a shared tokio worker, so one slow, hung, or dying disk pins a thread that
other sessions' banners, `NOOP`s, long-poll `await`s and deletes need. `src/persist.rs:12-13`'s
claim that the commit "cannot stall the runtime" is true of the mutex and false of the commit
inside it.

The repair moves the whole mutation onto tokio's blocking pool behind one awaited seam
(`spawn_blocking` + four `*_async` wrappers on `Store`), with the reply written only after the
mutation resolves. That repairs the placement without changing any observable behaviour — but it
does add public async surface beside the frozen founding sentence "`Store::insert` is
synchronous", so the interpretation must be recorded where future intake seats will read it.

## Decision

1. **The disk leg of decision 0001 is ordering and atomicity, not thread identity.** The
   guarantee is: the mail is in memory and committed to disk **before** the acceptance reply, the
   commit outcome is known before the reply, and the mirror transaction is all-or-nothing. Which
   thread executes the commit is not the guarantee. Async handlers may run the whole mutation on
   tokio's blocking pool via an awaited wrapper, provided the reply byte follows the resolution.
2. **The synchronous mutation methods remain the single owner of mutation logic.**
   `Store::insert`/`delete`/`clear`/`clear_all` stay synchronous and unchanged; the awaited
   wrappers offload the *whole* mutation (memory half, prune, mirror commit, fan-out) as one
   unit. No memory-half/commit-half split, no async rewrite of the methods, no mode branch.
3. **Fire-and-forget write-behind is forbidden.** No queue, no background flush, no
   eventual-consistency window, no batching or group commit. The awaited result is the contract;
   a wrapper that resolves before the commit outcome is known is a 0001 violation.
4. **`PRAGMA synchronous = FULL` (`src/persist.rs:84`) and the reply ordering at
   `src/smtp.rs:456` are load-bearing lines.** Any softening of the pragma, or any reorder that
   writes the acceptance reply before the commit outcome is known, is a new decision document the
   operator signs — never an edit.
5. **Panic symmetry holds.** A mutation that panics on the blocking pool re-raises in the
   awaiting caller, exactly as the synchronous call would; per-commit isolation (one task
   unwinds, the runtime and other sessions continue) is a property of the mechanism, not a
   behaviour change.

## Why

- The founding bench (0001's consequences) measures the sync memory-only store; ingest throughput
  with `--data-file` is commit-rate-bound today and stays so. This change promises nothing about
  throughput. What it promises: when the disk is slow, hung, or failing, exactly the sessions
  waiting on it pay — parked tasks, not blocked threads.
- A dedicated writer thread was refused: a panicked writer loop is a permanent wedge, and a
  rendezvous actor needs a shutdown drain or `RunningServer::stop` hangs. `spawn_blocking` needs
  neither; the persist mutex is already the writer queue and the blocking pool is already the
  writer threads.
- The alternative reading — that "synchronous" in 0001 pins the calling thread — would outlaw a
  pure thread-placement repair and force either an actor (more machinery, more ways to be wrong)
  or leaving the stall in place. Ordering is what the tests can prove and what the operator
  relies on.

## Consequences

- `Store` gains four additive public methods (`insert_async`, `delete_async`, `clear_async`,
  `clear_all_async`); the sync methods keep their signatures and all existing callers, tests, the
  bench, and the wave-2 gate closures compile unchanged.
- The commit → `250` window widens by one task wakeup (microseconds): a kill in that window
  leaves a stored, committed mail the client never saw — the reverse (a `250` without a commit)
  is impossible in both designs.
- A distressed disk presents as SMTP latency (parked sessions), not worker starvation; unbounded
  waiting is the honest posture, and any "protection" (bounded queue, `451` under pressure) is
  new accept-path semantics requiring its own decision.
- Every new production line ships with a test: the structural stall pin, the ordering and
  panic-symmetry pins, the real-protocol propagation leg, and a hard-kill (`SIGKILL`) restart
  leg — the only net that can see an ack-before-fsync, commit-after-`250`, write-behind, or
  softened-pragma regression, because graceful-shutdown restarts cannot.
- The measured before/after worker-stall numbers are recorded verbatim (docs note or PR body);
  if the win is not clear, the operator rules on the numbers as measured.
