# Ordered tasks: deterministic timing proofs

Implement [spec.md](spec.md) through [plan.md](plan.md). Check a task only
when its paired proof has executed and its evidence is recorded. The design
commit does not mark implementation, controls, gates or delivery complete.
Root owns launches and CPU allocation; this seat is design only.

- [x] **T1 — Preserve provenance and scope (AC1, AC11).** Record candidate
  HEAD, the eight source hashes and a complete retained-receipt hash manifest
  outside the historical receipt directory. Inspect the task intake and both
  council positions; do not import G context. **Proof:** baseline matches the
  plan's hashes, recorded LCOV/host receipt still report the actual failed
  coverage, and the initial/final tracked diff fits the allowlist.

- [x] **T2 — Establish adapter proof access first (AC3).** In tests/api.rs
  and tests/mcp.rs build fully available requests for public router/dispatch
  with fresh Store and a live shutdown sender. Use Axum's existing
  ServiceExt supertrait through a generic bound; add no dependency.
  **Proof:** targeted tests compile; explicit zero returns the exact result
  on one poll, omission and positive timeout demonstrate Pending and then
  resolve under controlled stop/match. Preserve real HTTP zero legs. If an
  unrelated suspension prevents the distinction, report its actual source
  owner before broadening; a weaker Store-only test cannot check this task.

- [x] **T3 — Replace every 9.1 elapsed refusal assertion (AC1–AC3).** Cover
  REST malformed/zero, MCP malformed/zero and
  wait_over_the_cap_is_refused_without_parking. Use exclusive inboxes,
  establish cumulative row attribution with a valid registration and assert
  zero refusal deltas. Keep status, text, matched counts and seeded
  non-consumption. **Proof:** cargo test --locked --test api and
  cargo test --locked --test mcp; positive row advances once, malformed/
  over-cap rows do not advance, zero registers once and leaves no active
  wait. Record default/zero and refusal/valid input controls distinctly
  from source mutants; fail if attribution is unavailable.

- [x] **T4 — Replace stdio's sleeping peer (AC4).** Read the complete real
  request, signal observed, hold the socket on release and own peer/bridge
  cleanup. **Proof:** a_silent_server_answers_as_a_read_timeout in
  cargo test --locked --test stdio observes the request before validating
  id 11 / -32000 / read timeout and releases/joins afterward. A deliberately
  closing peer must not satisfy that exact timeout outcome; inspect cleanup
  on a forced unmet-event diagnostic. No five-second synchronization sleep.

- [x] **T5 — Prove the helper's actual DATA-read timeout (AC5).** Replace
  prewritten dialogue with command/reply progression to DATA, withhold 354,
  retain HELPER_DEADLINE and compare the precise timeout panic. Keep owned
  client/peer handles and correct the false body-arrival comment.
  **Proof:** the targeted smtp_edges test and full
  cargo test --locked --test smtp_edges pass; bypassing the actual
  read_line_named timeout fails the named DATA mechanism assertion, then
  byte-exact restoration and targeted green pass. Outer expiry never counts
  as the helper's expected result.

- [x] **T6 — Introduce owner-local listener observation (AC9–AC10).** Capture
  a cfg(test) probe once per serve future; observe actual spawned handles,
  parked accept and admission harvest, with exclusive injection where
  needed. Remove global REAPED proof credit without changing production
  behavior. **Proof:** both installed and ordinary uninstalled paths execute
  in library tests; a second server's harvest cannot change the first
  probe. Review non-test expansion/diff to ensure no observer ships.

- [x] **T7 — Replace both across-batches proofs (AC9–AC10).** Use one
  cap-two owner per protocol, first a one-completion batch, then two
  completions, with a pinned wait polled Pending before each release and
  Ready after actual harvest. Check finished AbortHandles before the one
  next connection; assert first-reply admission, exact local counts 1 and 3,
  and a genuine full-cap refusal. **Proof:** both protocol instances of
  the_accept_loop_reaps_completed_sessions_across_batches pass with
  nonzero executed test counts, including same-runtime two-owner isolation.
  Removed/shutdown-only harvest, first-batch-only harvest and shared-credit
  controls fail their intended assertions; restore hashes and rerun green.

- [x] **T8 — Replace both 500ms backoff proofs (AC6).** Add only the installed
  cfg(test) readiness latch at the actual backoff select operand. Observe
  Pending with a real live session; stop must abort/drain and return while
  the latch is closed. Exercise explicit release/retry and no-observer timer
  paths. **Proof:** SMTP and POP3 backoff tests pass; sleep-before-shutdown
  mutants fail named stop-before-release assertions under generous
  diagnostic fences. Release/abort/join cleanup, exact restoration and
  targeted green are recorded. Health, cap, watch and budgets are preserved.

- [x] **T9 — Replace 9.5 calendar margins (AC7).** Use the first fixture's
  actual received_ms/id at equality and checked +1 across list/await/assert;
  remove since from the later count=2/limit=1 case. **Proof:** the targeted
  API test includes/excludes the exact id, retains malformed refusals and
  exact counts, and the full API suite passes without now offsets or
  assumptions about the second fixture's timestamp.

- [x] **T10 — Strengthen the existing 9.6 ordering proof (AC8).** Remove
  duplicate elapsed comparison, explicitly assert the commit remains held
  after stop, retain no-250/listener/live/disk/counter checks, and require one
  post-release notification and commit. **Proof:** cargo test --locked --lib
  an_in_flight_commit_neither_holds_stop_nor_half_lands passes; a temporary
  premature DATA acknowledgement in src/smtp.rs fails the held no-250
  assertion. Exact source restoration and targeted green pass. No G import
  or Store/Persist semantic change.

- [x] **T11 — Complete control ledger and scope audit (AC1, AC11).** Record
  commands, source hashes, precise mutant diffs, named red failures, exact
  restored hashes and green logs for T5/T7/T8/T10; distinguish T2/T3/T9 input
  discrimination from source mutation. Preserve every observed failure.
  **Proof:** no mutant remains, retained receipt manifest is unchanged,
  all targeted suites pass on restored source, and each finding maps to
  its final source hash and actual executed test. No elapsed correctness
  assertion or sleep synchronization remains at the affected sites.

- [x] **T12 — Run the full gate inventory (AC10–AC12).** Execute every gate
  in docs/house-rules.md with --locked where specified, including exact
  coverage plus selftest, publish guard, keyless seal proof, licenses and
  pinned compile without unsafe sweep warning. Add wrapper-digest proof,
  design-check selftest, delivery tamper proof and fail-closed native-tls
  absence. **Proof:** saved commands/exit codes show all clean; full LCOV
  reports missed=0 for lines, branches and functions, and the replacement
  pending/ready listener paths are measured. No exclusions, denominator
  reduction, ignored tests, normalized counts or selective rerun waiver.

- [ ] **T13 — Execute configured independent review and hermetic delivery
  (AC12).** Let the existing engine drive actual hermetic verify, four
  configured reviews and ship; do not launch another run or change models.
  **Proof:** actual journal receipts bind the restored committed candidate
  to DashScope Qwen Max correctness, Claude Opus security, official DeepSeek
  Flash guarantees (never Pro), DashScope GLM 5.3 tests and verify/ship.
  Retain protected review/security holds and resolve required work before
  claiming delivery. Host logs cannot substitute; label CI-only evidence
  separately. Never push, merge, tag or publish.

- [x] **T14 — Record finding-by-finding disposition (AC11–AC12).** Append the
  final evidence index and any dated clarification to the owning artifacts
  without erasing historical failures. **Proof:** each of 9.1–9.6 and both
  reaping proofs has final source identity, actual control/green evidence,
  final gate and delivery receipts or a specific outstanding limitation.
  No invented completion, unperformed CI claim or whole-epic claim.

## Design handoff check

The chief stages exactly spec.md, plan.md and tasks.md in this directory
and commits unsigned with an imperative message explaining why. HEAD must
remain that design commit for the sequence's final pinned check. Run-local
positions and the mandatory result JSON stay uncommitted. Candidate-copy
structural preflight/selftest may be reported as such; only the final
deterministic sequence step can decide designed/fail. Chief result is
drafted, with committed paths, and never chooses the next phase.

## Implementer disposition — 2026-10-23

T1–T12 and T14 have executed proof in [evidence.md](evidence.md), including
all final source hashes, red/restored controls, both preserved coverage
failures and the final exact 12729/12729, 756/756, 1409/1409 report.
T13 remains the engine's downstream work. The implementer result advances
only this locally gated phase; it does not claim verify/review/ship ran.
