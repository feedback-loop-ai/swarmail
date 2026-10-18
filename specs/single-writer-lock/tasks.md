# Tasks: single-writer lock

Implement in this order against [spec.md](spec.md) and [plan.md](plan.md).
These are implementation tasks; none is marked done by drafting the design.
Pair every completed task with its actual proving result in run-local evidence.

- [ ] **1. Establish the branch and baseline (AC6, AC8).** Confirm Rust >=
  1.89, audit callers/retained owners and existing gates, and add the real
  second-server regression. **Proof:** run the new test against old behavior
  and record a behavioral failure (second server remains alive/announces
  readiness), with guaranteed child cleanup; compilation failure is not red.
- [ ] **2. Implement startup admission and error contract (AC1, AC2, AC4).**
  Add suffix-appended sidecar open and one nonblocking try_lock before SQLite;
  own the File after state, add OpenError and propagate it through Store::open.
  **Proof:** inline tests assert typed/exact contention, database absence when
  the sidecar is held, drop/reopen, distinct I/O/SQL causes, and failed-open
  release. Directly exercise both lock-error conversion cases.
- [ ] **3. Pin sidecar semantics and compatibility (AC3, AC6).** Verify
  extension-distinct files, sentinel preservation, no unlink, empty counts,
  raw connection access and final-owner lifetime; fix only necessary fixture
  ownership/cleanup. **Proof:** inline persist/store tests, existing
  `store::persist_tests`, lifecycle/persistence suites and both busy-default
  tripwire tests pass without sleeps, retries or early unlock.
- [ ] **4. Prove the shipped binary (AC5).** Extend the regression to check
  refusal before readiness, the first server's continued health and SMTP/query
  behavior, killed/reaped owner reuse with retained mail, and graceful reuse.
  **Proof:** `cargo test --workspace --locked --test binary --test lifecycle
  --test persistence` passes; the new test retains the sidecar through each
  restart and uses readiness/exit synchronization and bounded child cleanup.
- [ ] **5. Land truthful contract documentation with code (AC7).** Add the
  exact 0033 §4 sentence and crash/overlap notes, correct implementation
  metadata/index, add the dated historical correction and index proposed
  0042. **Proof:** compare README/module wording against 0033 §4 and verify
  all relative links/statuses; rerun the refusal/reuse test to substantiate
  the documented behavior. No accepted ruling is silently rewritten.
- [ ] **6. Verify all required gates (AC6, AC8).** Run fmt write then check,
  clippy, full tests, bundle compile, exact coverage and license check with
  the plan's commands and no TMPDIR override. **Proof:** record each exit
  status plus exact nonzero 100/100/100 counts and coverage of added failure
  arms; retain existing burst/rate and hard-kill durability results. Any
  unavailable gate remains explicitly owed.
- [ ] **7. Commit the implementation as one reviewable change (AC7, AC8).**
  Include refusal, tests and README/module sentence together, preserve signed
  history and hand off to protected review. **Proof:** inspect the signed
  commit's path list and diff against all acceptance criteria; check no
  run-local evidence, gate-policy edits, dependency changes or unrelated
  repairs entered it. This design seat never pushes or closes an issue.
