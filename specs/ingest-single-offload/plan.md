# Plan: one blocking-pool job per accepted mail

Spec: [spec.md](spec.md). No decision is drafted: the change alters no
decision text, no documented tunable, and no default.

## Technical context

- Single Rust crate, tokio multi-thread runtime.
- `Store::insert` is synchronous. It takes `self.mutation` (`std::sync::Mutex<()>`)
  around plan → `record` (shutdown admission, pre-mirror seam, SQLite commit)
  → apply → fan-out.
- `Store::offload` is `spawn_blocking` plus `resume_unwind`.
- Today the DATA arm (`src/smtp.rs` ~1633–1659) and `seed_inbox`
  (`src/api.rs` ~2349–2392) each await two offloads per mail: first the
  parse, then `insert_async`.
- The MCP seed (`src/mcp.rs` ~910–935) builds inline and then makes one
  `insert_async` trip. It is out of scope and unchanged.

## Approach

1. **Reproduce first (AC-1).** Before any production edit:
   - Write a temporary probe test (`tests/zz_issue333_probe.rs`, deleted
     before the commit) covering the five shapes in the spec. It uses a
     2-worker runtime and C ≤ min(30, nproc/4) concurrent jobs.
   - Read CPU from `getrusage(RUSAGE_SELF)` through the existing `libc`
     dev-dependency.
   - For the lock diagnostic, a temporary local edit of `Store::insert` does
     `try_lock` first, counts a miss in an atomic, then calls `lock()`. The
     edit is reverted before the commit.
   - Run it as `timeout 300 cargo test --release --locked --test zz_issue333_probe -- --nocapture`
     and record the numbers and the load in the PR.
   - Stop and report a spec defect if the evidence contradicts the cause.
2. **DATA arm.** Replace the two awaits with one job:
   `Store::offload({ let job = store.clone(); let inbox = inbox.to_string(); move || job.insert(build_email(&raw, &inbox, &rcpts, from)) }).await`.
   - Match `Ok(stored)` and write `250 2.0.0 OK: stored as {stored.id}`.
   - Match `Err(_)` and write the fixed `451 4.3.0 storage commit failed`.
   - Keep `signal_data_parse_submission_for_test()` immediately before the
     submission, and keep `_in_flight` parked around it.
   - Rewrite the comment block above the call: one job holds parse then
     whole mutation; the parse is still outside the boundary (0030 §1).
   - Update the serve-loop shutdown comment (~413–424): a session aborted
     while parked at its DATA await can now leave a parse-and-insert job
     behind. That job's mutation meets the boundary after the stop has
     fired, so `record` refuses it (0024).
3. **Seed path.** Use the same shape:
   - `Store::offload(move || build_seed_email(..).map(|email| store.insert(email)))`.
   - The outer `Err(reason)` maps to `bad_request` (400, nothing stored).
     The inner `Err(_)` maps to 503, with the fixed text.
   - Update the doc comment to say the build and the commit are one job.
4. **Instrument.** Add a `#[cfg(test)] thread_local! OFFLOADS_FOR_TEST: Cell<usize>`
   in `src/store.rs`, incremented at the top of `Store::offload`, with a
   `pub(crate) fn offloads_on_this_thread_for_test() -> usize` reader.
   - The counter is thread-local and follows the precedent of `smtp.rs`
     `INJECTED_ACCEPT_ERRORS` and `DATA_PARSE_SUBMITTED_FOR_TEST`: the
     owner test's current-thread runtime polls the session, so the count is
     never shared across the suite (0020 §4–5).
   - Its reader is used by the AC-2 and AC-3 tests, so the seam has an
     installing test.
5. **Regression tests.**
   - AC-2 and AC-3 tests go inline in `src/smtp.rs` and `src/api.rs`, using
     real listeners on current-thread runtimes. Each takes the counter's
     delta across one accepted mail and asserts 1.
   - The AC-4 lowered-cap concurrent DATA burst goes in `tests/burst.rs`.
   - AC-5's `tests/ingest_cpu.rs` uses only the public API (`start_with`,
     `Config.data_file`, a raw SMTP client over the shared `tests/common`),
     with a temp data file under `target/`. It needs no crate-private seam
     (0020 §3).
6. **Gates.** Run the full inventory `--locked`, and record the receipts in
   the PR.

## Constitution check

| Rule | Status |
|---|---|
| 0001 accept == stored (frozen) | held — 250 follows the awaited `Ok` of the unchanged `insert` |
| 0002 subscribe-before-check (frozen) | untouched |
| 0003 oldest-only (frozen) | untouched; AC-4 burst test pins it under concurrency |
| 0005 extraction at ingest | held — `build_email` still runs before the 250 |
| 0014 one boundary | held — std mutex unchanged, no second lock |
| 0023 §2/§5 whole mutation, panic symmetry | held — `insert` whole inside the job; `resume_unwind` unchanged |
| 0030 §1 parse before boundary, no second offload helper | held — same `offload`, parse precedes `insert` |
| 0020 test placement | inline default; `tests/` for the public-API CPU leg; seams `#[cfg(test)]` |
| Dependencies | none added; `libc` already a dev-dependency |
| Coverage 100% exact | every changed arm (`Ok`, `Err`, seed 400/503) driven by a test |

## Project structure (files touched)

- `src/smtp.rs`: the DATA arm, two comments, and the AC-2 test.
- `src/api.rs`: `seed_inbox` and the AC-3 test.
- `src/store.rs`: the `#[cfg(test)]` offload counter and its reader. No
  production line changes.
- `tests/burst.rs`: the AC-4 capped concurrent burst.
- `tests/ingest_cpu.rs` (new): the AC-5 CPU relation.
- No `Cargo.toml` change is needed, since integration tests are
  auto-discovered.

## Research, data model, contracts, quickstart

- **Research:** the panel's probe numbers (spec "Why"), to be re-confirmed
  by AC-1.
- **Data model:** unchanged.
- **Contracts:** the wire is unchanged. That covers 250, 451, 400 and 503,
  the fixed texts, and the reply ordering.
- **Quickstart:** `timeout 600 cargo test --release --locked --test ingest_cpu -- --nocapture`
  prints the six CPU-per-mail figures.

## Risks

- **Peak pool threads in data-file mode rise slightly.**
  - The fused job holds its thread across parse and the commit wait.
    Previously the parse thread was released before the commit thread was
    taken.
  - This is bounded by the session cap (0036): at most one thread per
    in-flight session, which is the same bound the insert leg already had.
  - Mitigation: AC-5 measures data-file CPU too. No pool tunable is added.
- **The CPU relation already passes pre-fix on glibc.**
  - The issue's gap is musl-and-container-specific, so on glibc the
    relation holds even without the fix.
  - Mitigation: AC-2's counter is the pre-fix-failing detector. AC-5 guards
    the gross 3–5× class. AC-6 requires container numbers, or an explicit
    statement that they are owed.
- **CI noise on 3–4-core runners.**
  - Mitigation: the comparison is a ratio within one process, takes the
    median of three alternating runs, has a 1.25× margin, and makes no
    absolute-ms assertion.
  - If verify shows flakes, the margin is the thing to revisit, with
    evidence. The structural test is never the one dropped.
- **The cause is outside Swarmail (musl allocator under CFS) and the
  container gap persists.**
  - Mitigation: the PR says so with the numbers. No allocator dependency
    is added.
  - The admission gate stays rejected unless AC-1's contention diagnostic
    shows the boundary lock is the residual. In that case the run returns
    as a spec defect.
- **The shutdown abort leaves the fused job running.**
  - The fused job now reaches `insert`; previously the parse job alone was
    left.
  - Mitigation: `record` refuses after the stop (0024), and
    `persist_tests::an_in_flight_commit_neither_holds_stop_nor_half_lands`
    must pass unchanged. The comment at `src/smtp.rs` ~413 is corrected.
- **Counter leakage across tests.**
  - Mitigation: the counter is thread-local, and tests assert a delta
    across one mail, not an absolute value. The server must be on the test's
    current-thread runtime. A test that spawns its server on another
    runtime would read 0 and fail loudly, never pass by accident.
