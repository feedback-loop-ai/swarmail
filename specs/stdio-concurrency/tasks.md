# Tasks: concurrent stdio batches

Execute in order against [spec.md](spec.md) and [plan.md](plan.md).
Checkboxes describe implementation work; none is completed by drafting.

- [ ] **T1 — Capture the two behavioral regressions.** Extend real mcp/serve
  subprocess tests with a two-ping batch and adjacent wait/seed/ping using
  isolated inboxes. **Proof:** AC1 and AC4 fail on the reviewed bridge by
  empty stdout and `timed_out: true`, respectively; record commands, baseline
  SHA and failing assertions in ignored evidence. Compilation failure is not proof.
- [ ] **T2 — Share MCP envelope classification.** Add the small classifier
  beside dispatch and use it in HTTP without skipping notification effects.
  **Proof:** inline table cases and `tests/mcp.rs` pin AC2/AC8 for singles,
  null/duplicate IDs, mixed/nested entries, scalars, empty/oversized batches,
  statuses and effectful silent notifications; existing HTTP tests pass.
- [ ] **T3 — Recover byte-framed input and shape replies.** Replace Lines,
  parse once, use shared classification/error envelope, implement admitted
  batch errors and invalid-envelope errors. Compact responses into one line.
  **Proof:** AC1–AC3/AC5/AC6 tests cover both bad-line kinds without POST,
  next-ping survival, CRLF/blank/final-no-newline inputs, multiline HTTP JSON,
  all failure-table rows and silence despite a spurious peer body.
- [ ] **T4 — Schedule bounded independent exchanges.** Add the 16-entry
  FuturesUnordered loop with guarded reads/completions and persistent partial
  buffer; include notifications in the bound. **Proof:** AC4 real-binary
  wait/seed and registered-wait/ping regressions turn green; deterministic
  AC6 tests prove capacity, seventeenth-line non-consumption, resumed admission
  and partial-line preservation across a completion, without sleep synchronization.
- [ ] **T5 — Finish lifecycle and checked output.** One complete-line
  write_all plus flush, EOF drain, stdin-error drain and immediate output-error
  failure. **Proof:** AC7 tests separately cover EOF with concurrent work,
  admitted reply before stdin-error exit 1, write failure, flush failure and
  pending-future drop; recording writer pins the complete offered buffer.
- [ ] **T6 — Preserve transport behavior and document the contract.** Update
  module/README wording and add proposed 0039 to the decision index; describe
  cross-line ordering, failure uncertainty, unbounded input, stalled stdout,
  saturation and unchanged cancellation. **Proof:** AC8 existing stdio/binary
  tests (including long wait, cap, deadlines, status, URL and stderr) pass;
  review each documented rule against the spec table and executable tests.
- [ ] **T7 — Reconcile evidence and run all required gates.** Record repaired
  SHAs/results for T1, run targeted suites then fmt write/check, clippy,
  workspace tests, bundle compile, exact coverage and license check with the
  framed flags/PATH. **Proof:** AC9 has actual behavioral red/green evidence
  per child and all six gate exits are zero, exact 100/100/100, with no
  exclusions, dependency widening or unexplained untested production lines.
- [ ] **T8 — Prepare reviewed delivery.** Check the full diff against frozen
  guarantees and scope; retain proposed status until operator acceptance and
  preserve dated clarifications in owning artifacts. **Proof:** acceptance
  checklist maps AC1–AC9 to test/evidence results; signed feature-branch
  history and the later protected review plus 17 CI checks are green before
  authorized landing. Never push from the design seat or close issues.
