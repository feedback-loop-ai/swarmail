# Tasks: parse admission

- [x] T1 — Reproduce and confirm the cause (AC-1): glibc in-process and over
  real SMTP, lock-miss counter, standalone musl probe at 1/2/8/30 jobs and
  caps 2/4/8. Result: the fusion hypothesis is falsified; the candidate cause is
  allocator contention among parallel pool threads on musl.
- [x] T2 — `Store::parse_gate`, `parse_width`, `parse_permit` (+ test seams) in
  `src/store.rs`. Proof: `store::parse_gate_tests` (AC-4).
- [x] T3 — Gate the DATA arm in `src/smtp.rs` (permit before the submission
  signal; moved into the offload closure; released before `insert_async`).
  Proof: the DATA gate test (AC-2) and the existing 451/parse-submission/CA
  tests unchanged.
- [x] T4 — Gate `seed_inbox` in `src/api.rs`. Proof: the seed gate test (AC-3).
- [x] T5 — `tests/ingest_cpu.rs` (AC-5).
- [x] T6 — Decision 0124 (proposed), its index row, and the network-posture
  guide sentence.
- [x] T7 — Every house gate `--locked` (AC-7): fmt, clippy, test, binary build,
  evals fmt/clippy/test against the candidate binary, recipe compiles (verify
  if no brokkr), coverage and selftest, publish-guard, seal, deny.
