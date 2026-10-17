# Tasks: SMTP session boundaries

Ordered implementation tasks. Each checkbox requires its stated proof before
completion; this design does not mark implementation as done. AC references
refer to [spec.md](spec.md).

- [ ] **T1 — Establish discriminating regressions for #73, #81 and #82.**
  Add independent mixed-terminator, EHLO-reset and one-NUL-PLAIN wire tests to
  `tests/smtp_edges.rs`. **Proof:** each fails for the specified reply/state
  defect against `bacadbe` in an isolated worktree, with commands, failing
  assertions and statuses recorded; repeat these exact pins green in T8
  (AC1, AC5, AC9, AC12).
- [ ] **T2 — Preserve DATA boundary metadata in the existing reader.**
  Add TooLong ending metadata and seeded bounded drain tracking; change the
  DATA predicate/update order without normalization. **Proof:** measured
  reader tests for CRLF/LF on buffered/drained exits, split CR/LF including
  drain entry, ordinary fragmented lines and EOF; wire raw/alignment, empty,
  oversize-recovery and byte-fidelity cases pass (AC1–AC4).
- [ ] **T3 — Make envelope transitions atomic and reset greetings.**
  Introduce private `Option<Envelope>`, update RSET/STARTTLS/DATA consumption,
  clear envelope on greetings and give HELO one reply line. **Proof:** real
  EHLO/HELO reset and following need-MAIL cases, null sender, DATA refusal,
  storage failure, pipeline and existing shutdown pins pass (AC4–AC6).
- [ ] **T4 — Validate paths/parameters before chaos or mutation.**
  Remove bracket fallbacks; implement MAIL parameter table and RCPT suffix
  refusal; put state, syntax, parameters and recipient cap ahead of chaos.
  **Proof:** path and parameter matrix checks exact codes and preserved
  envelope; SIZE digit overflow/actual-size enforcement; probability-100
  nested-MAIL, invalid RCPT, capped RCPT and valid-chaos cases (AC6–AC8).
- [ ] **T5 — Make AUTH success explicit and require PLAIN triples.**
  Use optional identity; refuse repeated AUTH, missing mechanism and surplus
  tokens; retain PLAIN/LOGIN continuations and accept-any semantics.
  **Proof:** field-count, malformed/cancelled, retry, default-success,
  repeated-AUTH and routing tests include inbox snapshots proving no leaked
  password name; disabled/in-transaction precedence remains pinned (AC9–AC10).
- [ ] **T6 — Prove session reset and client compatibility together.**
  Extend real TLS reset tests and audit existing DATA helpers; keep all
  pipeline pins. **Proof:** pre-TLS identity/envelope/buffer are cleared,
  post-TLS AUTH succeeds, greetings/RSET retain identity, and real Python
  smtplib ASCII and UTF-8/BODY/SMTPUTF8 sends have correct stored envelopes
  and bytes; capture the reproducible Python probe (AC5, AC10–AC11).
- [ ] **T7 — Publish the precise protocol posture.**
  Update README's leniencies and refusals and add proposed 0034 to the decision
  index. **Proof:** compare each documented form to the wire test matrix;
  resolve relative links, confirm status proposed and byte-identical 0035,
  and verify no dependency/configuration/gate changes (AC13).
- [ ] **T8 — Verify the completed implementation.**
  Run the targeted suites and all six exact commands in plan.md, plus the
  real smtplib probe; retain all three red/green pairs. **Proof:** all commands
  exit zero; coverage reports zero missed lines/branches/functions; review
  the diff against frozen guarantees and the do-not-undo list. Report any
  blocked gate honestly, with no floor reduction/exclusions (AC1–AC13).
- [ ] **T9 — Hand the verified branch to the next delivery seats.**
  Make signed implementation commits with why-focused bodies; leave issues
  open and proposed 0034 unaccepted. **Proof:** Git commit/path/signature
  inspection and evidence links match T8's tree. The design seat never pushes;
  subsequent shipping alone performs the framed PR and 17-check workflow.
