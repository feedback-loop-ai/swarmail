# Parse admission: one ingest parse per usable core (issue #333)

Created: 2026-11-05. Status: implemented; decision
[0124](../../docs/decisions/0124-parse-admission.md) proposed. Issue #333.
Run: `issue-333-in-memory-mode-burns-2-1f7c3a65`. Base: main `5950af6`.
Operator direction 2026-11-05: "fix 333".

> **Revision, same day.** The first design (commit `1f8699d`) fused parse and
> insert into one blocking-pool job on the working cause "two handoffs per
> mail". The implementation's AC-1 falsified that cause (below), as the design
> required it to, and the seat stopped. This spec is the revision the evidence
> supports; the fusion is not shipped.

## Feature

The SMTP DATA arm and the HTTP seed handler wait for a permit from a
per-store semaphore, the **parse gate**, before they submit their CPU-bound
parse to tokio's blocking pool. The gate's width is
`std::thread::available_parallelism()` (affinity mask and cgroup CPU quota
aware, never below 1). The permit moves into the pool closure and is released
when the parse ends; it is not held across `Store::insert`.

No public API, flag, environment variable, chart value, dependency, or default
changes. `Store::insert` is untouched. Decision **0124** (proposed) records the
bound because it changes the "offloading is not an admission bound" statement
in `docs/guide/network-posture.md`.

## Why — the cause, with its evidence (AC-1)

Not a spin, not a lock, not blocking-pool churn from two handoffs. The
**candidate** cause — plausible, partly measured, not yet shown at the issue's
magnitude — is allocator contention among *parallel* parse threads, on the
static musl image under a CFS quota. Full numbers and commands are in decision 0124 §Context; in short:

- **The first working cause is falsified.** On glibc, two-trip memory
  0.408/0.372/0.376 ms/mail against one-trip 0.388/0.386/0.392 (two CPUs,
  7 980 mails/run, `timeout 300`). Fusing changes nothing.
- **The mutation lock is not the cost.** A temporary `try_lock`-miss counter
  (reverted) saw 1.5–3 % misses in memory mode at two CPUs; only data-file mode
  queues on it (75–97 %), because the fsync under it paces sessions.
- **On glibc the issue does not reproduce at all**, in-process or over real
  SMTP: memory 0.60–0.85 ms/mail, data file 1.1–1.3.
- **On static musl, parallelism is what costs.** Standalone probe (tokio +
  `mail-parser`, 24.7 KB mail, two-worker runtime, 60 000 parses per round,
  three rounds, `timeout 120`, 30 jobs at once ≤ nproc/4 = 32): at 16 usable
  CPUs musl doubles from 0.100 to 0.20–0.23 ms/parse with kernel time equal to
  user time; glibc stays flat (0.054 → 0.058). Pinned to 2 CPUs nothing
  happens, because a 30-thread burst cannot run more than two at once — which
  is why the earlier `taskset` reproduction saw nothing. A CFS quota is not a
  CPU mask: `--cpus=2` lets all 30 pool threads run together on a big host.
- **The cap fixes it, and only at the core count.** musl, 30 jobs, 16 CPUs:
  no cap 0.226–0.239 ms; cap 8 0.224–0.231; cap 4 0.189–0.196; cap 2
  0.110–0.133 (kernel time 6.2–6.7 s → 1.1–2.3 s).
- **Why data-file mode looked cheaper:** its fsync parks the pool threads, so
  fewer run at once; it escapes the contention by accident.

**Magnitude gap, named.** The probe's excess is about 0.10–0.13 ms per parse
(mail-parser alone, i686 musl); the issue's excess is about 8–15 ms per mail
(11.9–18.7 ms in memory against about 3.5 ms with the data file) on x86_64
musl under `--cpus=2`. The probe covers the parser, not the full `build_email`
(HTML pipeline, link and code extraction), and the allocations outside the
gate — rustls on the two runtime workers, the insert jobs, the session buffers
— contend on the same musl lock. The gate may therefore close the gap only
partly. **#333 is mitigated, not closed:** it stays open until the dispatcher's
bench runs against an image built from this tree. If the effect is small, the
next suspects are the ungated in-memory insert on the pool and musl
free/madvise churn.

Not measured here: the musl image under `--cpus=2`. The sandbox has no Docker
daemon, no `perf` access and no musl C toolchain (so `ring` and bundled SQLite
cannot build for musl). Those numbers are owed by the dispatcher's `load.py`
and `scale.py`.

## Guarantees and authorities

- [0001](../../docs/decisions/0001-synchronous-insert-lossless.md), frozen:
  `Store::insert` unchanged and synchronous; the `250` is written only after
  the awaited insert returns `Ok`.
- [0023](../../docs/decisions/0023-off-worker-commits.md) §5: a panicking parse
  still re-raises in the session (and returns its permit).
- [0030](../../docs/decisions/0030-loopback-plane-bounded-results.md) (the parse
  move it extends 0023 §1 with; its own §1 is the loopback default): the
  parse still runs before the mutation boundary through the one
  `Store::offload`; the gate is taken by the caller, no second helper.
- [0014](../../docs/decisions/0014-one-ordered-mutation-boundary.md),
  [0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
  [0003](../../docs/decisions/0003-oldest-only-pruning.md),
  [0005](../../docs/decisions/0005-extraction-at-ingest.md): unchanged.
- The no-spin instruments (`CLOSED_PROBE`, the starved-ticker test) are
  untouched.

## Acceptance Criteria

- **AC-1: cause stated with evidence before the change** (above, and decision
  0124). Met by the implementation; the first working cause was refuted.
- **AC-2: the DATA arm is gated.** With every slot held, a complete DATA
  session is parked at the gate: the parse is not submitted, nothing is stored.
  Released, the parse is submitted, the mail commits, and the `250` names it.
  Proof: `smtp::tests::a_data_parse_waits_for_a_gate_slot_then_the_250_follows_the_commit`,
  red when the arm skips the gate.
- **AC-3: the seed handler is gated** the same way. Proof:
  `api::tests::a_seed_waits_for_a_parse_gate_slot_then_answers_with_the_stored_mail`,
  red when the handler skips the gate.
- **AC-4: the bound is real and exact.** Eight sessions, two slots: exactly two
  parses run at once (a high-water mark counted by the test's own closures),
  six wait, all eight finish, and every slot returns. A panicking parse returns
  its slot and still propagates; a waiter that goes away gives up its place;
  the commit is not behind the gate. Proof: `store::parse_gate_tests`.
- **AC-5: the CPU relation, bounded, in its own process.**
  `tests/ingest_cpu.rs`, one `#[test]`: real SMTP, ~30 KB multipart mail, 8
  concurrent sessions on one client thread, 240 mails/run, six alternating
  runs, a two-worker server runtime, `getrusage`. Median in-memory CPU per mail
  ≤ 1.25 × median data-file CPU per mail, with a 120 s watchdog on each run's
  client side (a stalled server fails by name; nothing asserts on speed). On glibc it
  passes with or without the gate (ratio ≈ 0.75); it guards the issue's class,
  and AC-4 is the detector of the cause.
- **AC-6: load reported.** Every load at most nproc/4 workers for at most 10
  minutes, named and timed: see the PR.
- **AC-7: house gates** clean and `--locked`; no new dependency, `Cargo.lock`
  and `deny.toml` unchanged. The brokkr bundle compile is left to verify when
  the sandbox has no brokkr binary.
