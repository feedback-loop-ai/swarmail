# Ordered tasks: storage and maildrop repairs

The design handoff creates these tasks; unchecked items are implementation
work, not claims of completion. Each task names the proof it owes. Governing
contracts are [spec.md](spec.md), [plan.md](plan.md) and proposed 0037/0038.

- [ ] **T1 — Establish branch and red-proof ledger (AC14).** Record `25ad4df`
  as the reviewed behavior baseline, preserve existing user changes, and
  create isolated negative-proof worktrees where permitted. **Proof:** clean
  scoped diff, recorded SHA and baseline gate outcomes; every later child
  records an assertion/bench failure and repaired pass, not a compile error.
- [ ] **T2 — Bound multiline writes (#71, AC1).** Wrap LIST and UIDL in the
  existing single-operation bound; retain silent timeout exit and 600-second
  default. **Proof:** separate real backpressure regressions establish a
  pending reply write, expire it and observe slot re-admission; ordinary
  framing, genuine-full-cap refusal, #111 admission and shutdown drain tests
  pass. The old unbounded write fails under the same observation/deadline.
- [ ] **T3 — Send complete reply lines (#83, AC2).** Assemble text plus CRLF
  once in both helpers; keep SMTP flush and all existing bounds. **Proof:**
  inline recording-writer tests fail the old two-call shape, pass one complete
  write, and pin flush ordering; real sequential SMTP/POP3 and STARTTLS
  handshake tests pass with unchanged acceptance ordering.
- [ ] **T4 — Add and run the latency bench (#83, AC3).** Extend `read_stall`
  with real sequential loopback SMTP and explicit samples/profile reporting.
  **Proof:** at least 100 measured transactions after warmup on baseline and
  repair in the same debug profile; each MAIL/RCPT/DATA-prompt/final-250 mean
  below 5 ms on repair, with p50/p99 reported. Keep thresholds out of tests;
  retain a failed measurement as an unmet criterion.
- [ ] **T5 — Share serialization and allocation-free counting (#84, AC4/7).**
  Implement buffer/count emission sinks over one existing-normalization
  traversal and exclude added transparency dots from the counter. **Proof:**
  independent literal fixtures for 30/33 octets, LF, CR, mixed endings, empty,
  no-final-newline, 8-bit and TOP limits; all POP3 size replies equal received
  unstuffed payload, UIDLs stay stable, and the old dotted count fails.
- [ ] **T6 — Offload PASS and simplify existence checks (#86, AC7/8).** Move
  list/reverse/metadata/counting into one awaited offload, add direct
  `Store::has_inbox`, and replace both enumeration call sites. **Proof:** real
  current-thread POP3/HTTP interleaving responds to health while actual PASS
  work is held, with safe independent timeout cleanup; inline-work negative
  control fails the progress assertion. Structural no-payload-allocation and
  one-map-lookup checks plus present/absent/emptied/reopened inbox tests pass.
- [ ] **T7 — Remove RETR's intermediate body (#86, AC6).** Fill the final
  reply directly from the live Arc, using the snapshot's corrected size and
  one bounded write. **Proof:** structural old-shape negative control or
  allocation bench distinguishes two full representations from one; existing
  and new real-wire byte/stuffing/terminator/size tests pass.
- [ ] **T8 — Restore DELE parity (#86, AC5).** Observe live existence before
  marking; preserve snapshot numbering and QUIT semantics. **Proof:** update
  `the_session_reports_mail_removed_elsewhere` so HTTP removal makes both
  RETR and DELE answer `-ERR no such message`; old DELE fails. Verify mark
  state, survivor numbering, RSET, normal deletion and partial-commit retry.
- [ ] **T9 — State the selected store contracts (#87, AC9–11).** Update
  README, store docs and chart comments for unlimited aggregate memory,
  observed per-name residue, whole-file restore and parse-start timestamps.
  Link 0038 without editing accepted decisions. **Proof:** focused
  documentation tests fail the old claims, while registry residue/reuse/
  release/Closed/lag/atomic-subscribe tests and reversed-stamp pruning/restore
  tests pass. Record these as documentation changes and preservation evidence,
  never as new channel reclamation or acceptance-time stamping.
- [ ] **T10 — Retire the unused index on old and new files (#87, AC12).**
  Execute `DROP INDEX IF EXISTS emails_inbox` on open and share actual full
  restore SQL with EXPLAIN. **Proof:** fresh/legacy/reopen tests retain rows,
  counters and `emails_seq`; every plan row lacks a temp sort. Real SMTP/POP3
  legacy-file restart retains mail and asserts the retired index is absent;
  baseline fails that final assertion, supplying #87's production red pin.
- [ ] **T11 — Reconcile contracts and index the proposals (AC13/14).** Make
  size helper comments, README, new doc tests and 0037/0038 agree; list both
  decisions as proposed and retain all dated clarification entries. **Proof:**
  contract tests pass; diff against `25ad4df` shows no changes to
  0021/0026/0028/0034/0035/0036, dependency files or protected gate behavior;
  review the five-child evidence ledger for genuine, correctly labeled reds.
- [ ] **T12 — Run the required gates and finish the implementation handoff
  (AC14).** With `$HOME/.cargo/bin` on PATH, run fmt, workspace/all-target
  locked clippy with warnings denied, workspace locked tests, bundle compile,
  coverage without TMPDIR override and license deny. **Proof:** all six exits
  zero, coverage has exactly zero missed lines/branches/functions, and T4's
  bench meets its criterion. Retain frozen-guarantee, DATA, listener and
  shutdown regressions; report environmental failures as failures. Update
  these checkboxes only with evidence. Subsequent review/ship follows intake's
  branch/PR/17-check workflow; this design seat never pushes or closes issues.

## Design validation — 2026-10-18

The five artifacts pass the nonempty-file, required-heading, checkbox,
relative-link and proposed-status checks. Protected decision files remain
byte-identical to HEAD. Formatting, clippy, bundle compilation and license
checks passed. Baseline tests failed on 22 socket-permission errors (178
passed); the coverage run failed on the same restricted socket operations,
so no passing coverage result is claimed. Logs are under the ignored
`.brokkr/state/design/run15-validation/` directory.

Branch creation and exact-path `git add` both failed because `.git` is
mounted read-only. These are uncommitted drafts; the requested signed commit
remains outstanding. No implementation task above is marked complete, and
no remote operation was performed.
