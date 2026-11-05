# 0124 — Parse admission: at most one ingest parse per usable core

- **Status:** proposed
- **Date:** 2026-11-05
- **Scope:** Issue #333 ("in-memory mode burns 2–7× the CPU of data-file mode
  under concurrent SMTP ingest"), delivered from `main` 5950af6 on the
  `claude-careful` recipe, run `issue-333-in-memory-mode-burns-2-1f7c3a65`.
  Operator direction 2026-11-05: "fix 333". The number was assigned by the
  dispatcher.
- **Specification:** [parse admission](../../specs/ingest-parse-gate/spec.md);
  [plan](../../specs/ingest-parse-gate/plan.md),
  [tasks](../../specs/ingest-parse-gate/tasks.md).
- **Extends:** accepted [0023](0023-off-worker-commits.md) §1 and accepted
  [0030](0030-loopback-plane-bounded-results.md) (its Extends of 0023 §1: the
  parse moved onto the pool; §1 there is the loopback default) — the parse leg still runs
  on the blocking pool through the one `Store::offload`; it now begins with an
  asynchronous admission. Compatible work under 0045 §8: it preserves every
  accepted ruling and amends none, so it may land while this is proposed.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) (`Store::insert`
  is untouched and synchronous; the `250` still follows the commit, with a data
  file the committed one), [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md),
  [0005](0005-extraction-at-ingest.md),
  [0014](0014-one-ordered-mutation-boundary.md) (the parse stays before the
  boundary, and the boundary lock is not held across the gate),
  [0024](0024-shutdown-refusal-at-the-boundary.md), and the 0020 §5 test
  disciplines.
- **Touches (descriptive text, not a ruling):** the sentence in
  [the network-posture guide](../guide/network-posture.md) that "offloading is
  not an admission bound either" — true of everything else on the pool, no
  longer of the ingest parse.

## Context

Issue #333 measured the shipped image (static musl, `docker run --cpus=2`,
40 one-connection-per-mail sessions, ~30 KB multipart mail) at 8–29 ms of CPU
per mail in memory mode against 4–8 ms with `--data-file`, on short-lived
threads that peak near 30: tokio's blocking pool. Sequential cost was flat at
~2 ms in both modes.

The tree was searched for the section the issue asked for, and it is not
there. On a glibc build, in-process and over real SMTP, the in-memory mode is
*cheaper* than the data file (0.60–0.85 ms against 1.1–1.3 ms per mail) and no
spin, retry loop or contended lock turns up:

- the mutation lock (`Store::mutation`) is taken without waiting for 97–98 %
  of in-memory mails at two CPUs; only the data-file mode queues on it, and it
  does so because the fsync under it paces the sessions;
- fusing the parse and the insert into one pool job — the first hypothesis,
  panel-proposed — does not move CPU or kernel time (0.408 / 0.372 / 0.376 vs
  0.388 / 0.386 / 0.392 ms per mail, two CPUs, 7 980 mails per run), so the
  second handoff per mail is not the cause;
- the no-spin instruments (`CLOSED_PROBE`, the starved-ticker test) are
  unaffected.

What does reproduce — and is offered as the **candidate** cause, not a proven
one (see *Magnitude* below) — on a static musl build (`i686-unknown-linux-musl` is the
only musl target this sandbox carries; the full crate needs a C toolchain for
`ring` and bundled SQLite that is not installed, so the probe is a standalone
crate with `tokio` and `mail-parser` — the same parser — and the same 24.7 KB
multipart mail), is allocator contention among parallel pool threads. 60 000
parses per round, three rounds, a two-worker runtime, `timeout 120`, 30
concurrent jobs (`nproc` = 128, so at most `nproc/4` = 32), median CPU per
parse:

| build, CPUs usable | 1 job at a time | 30 jobs at a time |
|---|---|---|
| glibc x86-64, 16 | 0.054 ms | 0.058 ms |
| musl i686, 16 | 0.100–0.120 ms | 0.204–0.226 ms, kernel time about equal to user time (glibc: a sixth) |
| musl i686, 2 (taskset) | 0.088 ms | 0.096 ms |

glibc is flat; musl, whose allocator is one global lock, doubles when parses
run in parallel, and the doubling is mostly futex and scheduler time. Pinned
to two CPUs the contention cannot form — a 30-thread burst can only run two at
once — which is why a `taskset` reproduction, and the glibc one, see nothing.
**A CFS quota is not a CPU mask:** under `--cpus=2` all 30 pool threads may run
at the same moment on a 128-core host, burn the quota in a few milliseconds
fighting for the allocator, and then be throttled for the rest of the period.
The data-file mode escapes it by accident — the fsync parks the pool threads,
so fewer are ever running — which is why the cheaper mode was the dearer one.

Capping the in-flight parses at the usable core count removes it *in the
probe*. The same probe, 30 sessions, 16 CPUs, musl i686:

| parse concurrency cap | CPU per parse | kernel time |
|---|---|---|
| none | 0.226–0.239 ms | 6.2–6.7 s |
| 8 | 0.224–0.231 ms | 6.5–6.9 s |
| 4 | 0.189–0.196 ms | 4.8–4.9 s |
| 2 (the quota) | 0.110–0.133 ms | 1.1–2.3 s |

The cap has to match the cores the process may use; a loose one buys nothing.

**Magnitude.** The probe's excess is about 0.10–0.13 ms per parse; the issue's
is about 8–15 ms per mail (in memory 11.9–18.7 ms against about 3.5 ms with a
data file). The probe is mail-parser alone on i686 musl; the issue is the full
`build_email` (HTML pipeline, link and code extraction) on x86_64 musl under
`--cpus=2`, where the two runtime workers (rustls implicit TLS), the insert
jobs and the session buffers contend on the same global allocator lock outside
the gate. The gap between the two is not reconciled here, so the gate may close
only part of the reported excess. If the bench shows a small effect, the next
suspects are the ungated in-memory insert on the pool and musl free/madvise
churn.

## Proposed ruling

1. **The parse is admitted.** The SMTP DATA arm and `POST …/seed` take a permit
   from the store's parse gate, a `tokio::sync::Semaphore`, before they submit
   their parse to the blocking pool, move the permit into the pool closure, and
   release it when the parse ends — returns, panics, or is dropped unrun.
2. **The width is the usable cores:** `std::thread::available_parallelism()`,
   which reads the affinity mask *and* the cgroup CPU quota — 2 under
   `--cpus=2` on any host — and is never less than 1. It is not a flag, an
   environment variable or a chart value: there is no new tunable, and the
   Helm chart and the documented defaults are unchanged.
3. **The wait is asynchronous.** No pool thread parks on the gate; a waiting
   session is a parked future, admitted first-come first-served. A session
   that goes away while waiting gives its place up.
4. **Only the parse is gated.** The permit is released before
   `Store::insert_async`. The 0014 mutation boundary and the persist mutex
   already order the commits, and a permit held across an fsync would let one
   slow disk starve the next mail's parse. `Store::insert` and every other
   mutation are exactly as before (0001).
5. **One offload.** The parse still goes through the one `Store::offload`
   (0030's parse move: no second helper); the permit is taken by the caller
   (`Store::parse_permit`) and moved into its closure.
6. **The MCP `swarmail_seed_email` seed build** builds inline on the async
   worker today and is unchanged; it is not an ingest surface of the pool.

## Considered and not done

- **Fusing parse and insert into one pool job** (the design's first answer):
  measured to change nothing (above) and it moves the boundary 0030 drew — the
  parse would run inside the same closure as the commit.
- **An allocator swap** (jemalloc, mimalloc): fixes the musl lock at its root
  and is a new dependency; the dependency allowlist is the operator's. It
  stays available if the operator wants it; this ruling does not need it.
- **`max_blocking_threads`**: bounds every blocking user at once — commits
  parked on the mutation boundary or an fsync, the webhook CA builds, the
  compat renders — so a low bound lets parked work starve the parse and a high
  one bounds nothing. The gate bounds exactly the work that is CPU, and
  releases before the work that parks.
- **An async mutex in front of `Store::insert`:** rejected by the design
  panel — the mutation lock is not the cost, and a second lock order is a
  hazard to 0003.

## Consequences

- At most `cores` mails are parsed at once; the rest of a burst waits for a
  permit rather than for the allocator. Throughput is unchanged on a host whose
  cores are the limit, because the parse is CPU-bound and the cores were
  already the ceiling.
- A hostile body occupies a slot for as long as its parse takes (the SMTP size
  limit bounds it); `cores` such sessions delay other parses. Before, they
  would have competed for the same CPU instead of queueing. The gate adds no
  amplification, and the parse still happens before the mutation boundary, so
  no lock is held through it.
- With one usable core the parses are serial. That is what one core does.
- **Not claimed:** a measurement of the musl image under `--cpus=2`, or that
  the gate removes the whole reported excess. The probe above is the evidence
  this sandbox can produce; the container numbers (the 40-connection bench
  against an image built from this tree) are owed by the dispatcher's
  `load.py` and `scale.py`. Issue #333 is mitigated by this change and stays
  open until they are in.

## Proof

- `src/store.rs` `parse_gate_tests`: no more than the gate's width of parses
  ever run at once (a high-water mark across eight sessions and two slots,
  counted by the test's own closures — red when the width is widened); a
  panicking parse returns its slot and still propagates (0023 §5); a waiter
  that goes away gives up its place; the commit is not behind the gate; the
  default width is the usable core count.
- `src/smtp.rs` and `src/api.rs`: with the only slot held, a real DATA session
  and a real seed request are parked at the gate with nothing stored; released,
  the `250` follows the commit (0001), the seed answers with the stored mail.
  Each is red when its call site skips the gate.
- `tests/ingest_cpu.rs`: one test, its own process, real SMTP sessions
  (~30 KB, 8 concurrent, 240 mails, six alternating runs): in-memory CPU per
  mail is at most 1.25× the data file's, by `getrusage` (which also counts the
  client thread, so the margin is looser than it reads), with a watchdog that
  fails a stalled run by name. On glibc it passes with or without the gate: it
  guards the relation, not the cause; the parse-gate tests above are the
  cause's detector.
