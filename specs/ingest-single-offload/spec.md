# One blocking-pool job per accepted mail (issue #333)

Created: 2026-11-05. Status: draft. Issue #333. Run:
`issue-333-in-memory-mode-burns-2-1f7c3a65`. Base: main `5950af6`.
Operator direction 2026-11-05: "fix 333".

## Feature

The SMTP DATA arm (`src/smtp.rs`, the `Store::offload(build_email)` and then
`insert_async` pair) and the HTTP seed path (`src/api.rs` `seed_inbox`, the
same pair) submit **one** blocking-pool job per accepted mail. That job runs
the parse and then the unchanged synchronous `Store::insert`. Today each mail
makes two trips to tokio's blocking pool: one parses, one commits.

No public API, tunable, default, dependency or decision text changes. **No
decision number is taken** (0124 stays unassigned; see "Rulings" below).

## Why

Issue #333 measured the in-memory mode, which does strictly less work, at
3–5× the CPU of `--data-file` mode at 40 concurrent connections. The extra
CPU sits on short-lived blocking-pool threads.

The panel's simplicity seat ran a bounded in-process probe: glibc, release
build, 2 runtime workers, 30 concurrent DATA-shaped jobs, N = 2000, each run
≤ 8 s under `timeout 300`. The results:

| Shape (30 at once) | CPU ms/mail | stime/burst |
|---|---|---|
| parse only (1 trip) | 0.62–0.69 | 60–100 ms |
| today: parse trip + `insert_async` trip | 1.04–1.16 | 930–1130 ms |
| one trip, parse + `insert` | 0.69–0.73 | 160–180 ms |
| data file, today's shape | 1.35–1.42 | 1090–1150 ms |

The one-trip shape takes the same mutation lock the same number of times.
Its probe even sent all mail to *one* inbox, which contends harder. Even so,
it spends about 6× less kernel time. That clears the mutation lock, the
DashMaps and the watcher fan-out. The excess comes from the **second
handoff per mail**: park/unpark, futex wakeups, and pool threads spawned and
reaped. In data-file mode the fsync under the boundary paces sessions, and
that hides the handoff cost.

The working cause is therefore **blocking-pool churn from two handoffs per
mail**, not a spin. The implementer must re-confirm it before changing code
(AC-1). The issue's 10× was not reproduced on glibc. The shipped image is a
static musl binary under a CFS quota, which may make each wake costlier.
This design does not chase an allocator: an allocator swap is a new
dependency, which is refused.

## Rulings on the panel

- **Adopted from simplicity:** fuse parse and insert into one `Store::offload`
  job. Also adopted: the evidence above, and the finding that a literal
  "memory CPU ≤ data-file CPU" test already passes on the unfixed glibc
  tree, so it cannot be the only regression proof.
- **Adopted from robustness:**
  - the reproduce-and-falsify discipline (AC-1), including its mutation-lock
    contention diagnostic;
  - a regression proof that fails on the pre-fix tree;
  - the CPU relation the issue asks for, measured in its own process;
  - the failure table's 0003 hazard: the std mutation boundary stays the one
    ordering and is never replaced.
- **Rejected from robustness: the async `tokio::sync::Mutex` admission gate.**
  - The probe exonerates the boundary lock: the same lock acquisitions cost
    6× less once the second handoff is gone.
  - The gate would add a second lock order (gate → pool → std mutex).
  - It would also make a test that leaks the pre-mirror seam block every
    async mutation of that store.
  - It serializes deletes and clears behind inserts for no measured gain.
  - Cost to the losing side: in memory mode, a pool thread may still block
    for microseconds on the std mutex, instead of parking as a future. If
    the evidence after the fix shows real boundary contention (AC-1's
    diagnostic), the implementer stops and reports it as a spec defect. The
    gate is not added ad hoc.
- **Scope: simplicity's DATA-arm-only scope is overruled for the seed path.**
  - `seed_inbox` has the identical two-trip shape on a public ingest surface,
    and fusing it is the same few lines. It is in scope.
  - The MCP `swarmail_seed` path builds inline and makes one trip. It is not
    a two-trip site and is unchanged.
  - Cost to simplicity: one more call site and its test.
- **No new helper.** 0030 §1 records "no second offload helper". Both sites
  call the existing `Store::offload` with a closure that calls
  `Store::insert`. `insert_async` and the other `*_async` wrappers stay as
  they are.

## Guarantees and authorities

- [0001](../../docs/decisions/0001-synchronous-insert-lossless.md), frozen:
  `Store::insert` is unchanged and synchronous. The `250` is written only
  after the awaited job returns `Ok`. With a data file, `Ok` still means
  committed under `synchronous = FULL`.
- [0023](../../docs/decisions/0023-off-worker-commits.md) §2: the whole
  mutation still runs as one unit inside the job. §5: panic symmetry holds
  through `offload`'s `resume_unwind`. A parse panic is still a dead
  session, as under 0030.
- [0030](../../docs/decisions/0030-loopback-plane-bounded-results.md) §1: the
  parse still runs **before** the mutation boundary, never inside it, so the
  boundary stays O(row-deletes) and a hostile body cannot hold the lock.
- [0024](../../docs/decisions/0024-shutdown-refusal-at-the-boundary.md): a
  job left behind by a shutdown abort reaches `insert`, and `record` refuses
  it once the stop has fired. Nothing is applied.
- [0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
  [0003](../../docs/decisions/0003-oldest-only-pruning.md),
  [0005](../../docs/decisions/0005-extraction-at-ingest.md),
  [0014](../../docs/decisions/0014-one-ordered-mutation-boundary.md):
  unchanged. Waits, the plan → commit → apply boundary, pruning and
  extraction at ingest are untouched.
- The no-spin instruments (`CLOSED_PROBE`, `src/store.rs` ~47, and the
  starved-ticker test, ~1660) stay exactly as they are.

## Acceptance Criteria

- **AC-1: cause stated with evidence, before the code change.**
  - The implementer runs a bounded in-process probe, release build, comparing
    CPU per mail (`getrusage(RUSAGE_SELF)` or `/proc/self/stat`) for these
    shapes:
    - sequential in memory;
    - concurrent parse only;
    - concurrent two-trip (today);
    - concurrent one-trip;
    - concurrent data file.
  - On the two-trip and one-trip shapes, it also counts mutation-lock
    contention with a temporary `try_lock`-miss counter. That counter is a
    diagnostic only and is deleted before the commit.
  - The PR states the cause and these numbers, together with the named
    command, its `timeout`, the concurrency, the nproc, and the wall time.
  - If the numbers contradict the working cause (the one-trip shape is not
    clearly cheaper than two-trip, or contention dominates), the implementer
    stops and reports a spec defect instead of shipping this fix.
- **AC-2: the DATA arm makes exactly one blocking-pool submission per
  accepted mail.**
  - The submission is one `Store::offload` call whose closure calls
    `build_email` then `Store::insert`.
  - The reply is `250 2.0.0 OK: stored as <id>`, with the id taken from the
    stored mail. A commit error still answers `451 4.3.0 storage commit
    failed`.
  - Proof: a deterministic inline test counts offload submissions on its own
    current-thread runtime and asserts 1 per mail. It is built on a
    thread-local `#[cfg(test)]` counter in `Store::offload`, which is not
    process-global. The same test asserts 2 on the pre-fix tree, so it is a
    real regression detector.
- **AC-3: the seed path makes exactly one submission per seeded mail.**
  - It still answers `400` naming the field for a refused fixture, and
    stores nothing.
  - It answers `503 storage commit failed` on a commit error.
  - Proof: the same counter, driven through the real HTTP handler, with the
    refusal and commit-failure arms still covered.
- **AC-4: frozen and accepted guarantees hold.** Every existing test in
  these places passes unchanged:
  - `src/store.rs`;
  - `src/store/persist_tests.rs`, including
    `an_in_flight_commit_neither_holds_stop_nor_half_lands`;
  - `src/smtp.rs`, `tests/burst.rs`, `tests/p2.rs` and `tests/persistence.rs`.
  Each guarantee keeps a specific proof:
  - 0003: a lowered-cap concurrent DATA burst keeps exactly `cap` mails, the
    newest ones.
  - 0001: every id that received a `250` is queryable.
  - With a data file, every id that received a `250` is present after
    reopening the store.
- **AC-5: the CPU relation, as a bounded CI-shaped test in its own
  process.**
  - The test is `tests/ingest_cpu.rs` with exactly one `#[test]`, so
    `RUSAGE_SELF` sees only this binary.
  - It drives real SMTP sessions with ~30 KB multipart/alternative mail
    against an in-process server: memory, then a temp data file, alternating,
    3 runs each.
  - Load: one client thread with C = 8 concurrent sessions, ≤ 300 mails per
    run, and a 2-worker server runtime.
  - It asserts median memory CPU per mail ≤ 1.25 × median data-file CPU per
    mail, and prints all six numbers.
  - It stays under a 120 s internal deadline. The deadline is a bound on the
    run, not a validity assertion.
  - The CPU figure includes the client, which is identical in both modes.
  - This leg guards the issue's 3–5× class. AC-2 is the pre-fix-failing
    detector.
- **AC-6: load reported.**
  - Every load the implementer applies is at most nproc/4 load-generating
    threads (on this host, nproc = 128) for at most 10 minutes. Each run
    uses a named command under `timeout`.
  - The PR lists each run.
  - The PR either gives container numbers on the fixed image (the issue's
    40-connection bench), or states that the sandbox cannot build the musl
    image and that those numbers are owed by the dispatcher's
    `load.py`/`scale.py`.
- **AC-7: house gates.** Every gate in `docs/house-rules.md` § Gates is clean
  and `--locked`:
  - the evals crate against the candidate binary;
  - coverage at 100% exact, and its selftest;
  - publish-guard and seal proofs;
  - `cargo deny check licenses`.
  There is no new dependency, and `Cargo.lock` is unchanged. If the sandbox
  has no brokkr binary, the bundle-compile gate is left to verify, and the
  PR says so.
