# Tasks: one blocking-pool job per accepted mail

Ordered. Each task names the test that proves it.

- [ ] T1 — Reproduce and confirm the cause (AC-1).
  - Write a temporary `tests/zz_issue333_probe.rs` covering five shapes:
    sequential in memory, parse-only, two-trip, one-trip, and data file.
  - Use a 2-worker runtime, C ≤ min(30, nproc/4), and `getrusage` CPU.
  - Add a temporary `try_lock`-miss counter in `Store::insert`.
  - Run `timeout 300 cargo test --release --locked --test zz_issue333_probe -- --nocapture`.
  - Record the numbers, the load, and the wall time.
  - Delete the probe and revert the counter.
  - **Proof:** the recorded numbers show one-trip clearly below two-trip.
    Otherwise stop and report a spec defect.
- [ ] T2 — Add the `#[cfg(test)]` thread-local offload counter in
  `src/store.rs` (`Store::offload` increments it; `offloads_on_this_thread_for_test`
  reads it).
  - **Proof:** the T3 and T4 tests install and read it. Coverage stays at
    100% exact.
- [ ] T3 — Fuse the DATA arm into one `Store::offload(parse → insert)` job
  in `src/smtp.rs`.
  - The 250 uses the stored id, and `Err` answers `451 4.3.0 storage commit
    failed`.
  - Keep the parse-submission signal and the `_in_flight` park.
  - **Proof:** add an inline test,
    `data_arm_submits_one_blocking_job_per_accepted_mail`, on a real
    listener with a current-thread runtime. It asserts the counter delta is
    1 for one mail; record that it read 2 before the change.
  - **Proof:** the existing 451 commit-failure and parse-submission tests
    pass unchanged.
- [ ] T4 — Fuse `seed_inbox` in `src/api.rs` (`build_seed_email(..).map(|e| store.insert(e))`).
  - **Proof:** add an inline test, `seed_submits_one_blocking_job_per_mail`,
    asserting a delta of 1.
  - **Proof:** the existing seed tests for 400-names-field-stores-nothing
    and 503 commit failure pass unchanged.
- [ ] T5 — Correct the comments in `src/smtp.rs`: the serve-loop shutdown
  comment (~413–424, which now leaves a parse-and-insert job that `record`
  refuses after stop) and the DATA-arm comment. Also correct the
  `seed_inbox` doc.
  - **Proof:** `src/store/persist_tests.rs::an_in_flight_commit_neither_holds_stop_nor_half_lands`
    passes unchanged.
- [ ] T6 — Pin 0001 and 0003 under concurrency in `tests/burst.rs` (AC-4).
  - Add `capped_concurrent_burst_keeps_newest_and_every_250_is_queryable`:
    a cap of 5 and 8 concurrent sessions × 5 mails to one inbox.
  - It asserts exactly 5 stored, that each is among the last accepted
    in commit order, and that every 250 id received was queryable at reply
    time or evicted by a later one.
  - **Proof:** the test itself; `tests/persistence.rs` passes unchanged for
    the data-file leg.
- [ ] T7 — Add the CPU relation in `tests/ingest_cpu.rs` (AC-5).
  - The file holds one `#[test]`: a 2-worker server runtime, one client
    thread with C = 8 sessions, and ~30 KB multipart/alternative mail.
  - It runs memory and data file alternately, 3 runs × ≤ 300 mails each,
    and prints all figures.
  - It asserts median memory ≤ 1.25 × median data-file CPU per mail, under
    a 120 s deadline.
  - **Proof:** `timeout 600 cargo test --release --locked --test ingest_cpu -- --nocapture`
    passes, and `cargo test --workspace --locked` passes in debug.
- [ ] T8 — Gather the evidence for the PR (AC-6).
  - Gather the cause, the T1 and T7 numbers, and every load run with its
    command, timeout, threads, nproc, and duration.
  - Add the fixed container's numbers, or a statement that the musl image
    could not be built here and those numbers are owed to the dispatcher's
    40-connection bench.
  - **Proof:** the PR body lists each item.
- [ ] T9 — Run every house gate `--locked` (AC-7): fmt, clippy, test, the
  binary build, evals fmt, clippy and test against the candidate binary,
  the recipe compiles, the coverage gate and its selftest, publish-guard,
  seal, and deny.
  - If the sandbox has no brokkr binary, the recipe compiles are left to
    verify, and the PR says so.
  - **Proof:** each gate's receipt shows exit 0, and `git diff --stat main -- Cargo.lock evals/Cargo.lock deny.toml`
    is empty.
