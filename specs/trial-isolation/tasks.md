# Trial isolation tasks — the boundary is bubblewrap

Ordered. Each task names the test that proves it. AC and R references point
to [spec.md](spec.md) as revised 2026-11-01 after the review return. A
proposed decision is not authority to activate a contradictory contract
(accepted 0045 §8); concurrency stays off and no row version changes. Tasks
closed by earlier commits that this revision keeps stay closed; tasks the
review reopened are unchecked again with their new test; the review's
findings are T13–T20.

- [x] **T01 — Carry the decision and its index row (AC-S8, AC-O1).**
  `docs/decisions/0115-trial-isolation.md` stays `proposed`, now with its
  *After the first review* subsection (committed by the chief with this
  design); the README index row stands. **Test:** `design-check.sh` resolves
  `Decision: 0115` to one clean proposed record the spec links;
  `git diff a4f552d -- docs/decisions/0055-agent-eval-harness.md
  docs/decisions/0076-sequential-eval-trials.md` is empty; the README has
  exactly one 0115 row.
- [x] **T02 — Establish the leak live, before any boundary (AC-S1).** Kept
  as built, rescoped to Linux by T13. **Test:**
  `the_leak_is_live_a_same_uid_helper_makes_an_idle_victim_pass` and the
  probe's negative control green on Linux; discovery is bounded to the
  victim runner's descendants.
- [x] **T12 — Delete the raw-syscall worker (AC-S9).** Done at bdc504f.
  **Test:** `grep -rn "pivot_root\|SYS_clone\|uid_map\|setpriv" evals/src
  evals/tests` is empty.
- [x] **T03 — The pure bwrap parts (AC-S4).** Done at bdc504f; T14 and T18
  revise the golden argv and the hostile-argv test.
- [x] **T04 — The controller and the parameter seam (AC-S2, AC-S3, AC-S5).**
  Done at bdc504f; T15 and T18 revise the precondition order and the
  stand-in tests.
- [x] **T05 — Skip discipline for the live proofs (AC-S6).** Done at
  bdc504f; T13 adds the Linux gate.
- [ ] **T13 — Make the evals suite compile on macOS (R3; AC-S1, AC-S6,
  AC-S7).** Gate with `cfg(target_os = "linux")`: in `tests/common/mod.rs`
  `find_victim_agent`, `boundary_observe`, `launch_victim` and every helper
  naming `ProcessIdentity`/`process_descendants`/`identity_alive`; in
  `tests/trial_isolation.rs` every test and signature that names them, the
  `#[ignore]`d live proofs included, and the always-on AC-S1 pair; in
  `src/main.rs` `every_refused_boundary_is_a_named_harness_error_and_no_agent_runs`.
  Make `the_host_layout_is_read_from_the_host` assert against the host's own
  `symlink_metadata`. Keep every `cfg(not(target_os = "linux"))` platform
  test. **Test:** on Linux the suite is unchanged (same test count minus
  nothing); a by-name audit (`grep -n` of each gated item across
  `evals/src` and `evals/tests`) shows no ungated use; CI's
  `test (macos-latest)` job is the compile evidence and is not claimed by
  the seat.
- [ ] **T14 — `--as-pid-1` (R1; AC-S4, AC-L3).** Insert `--as-pid-1` after
  `--new-session` in `isolation::argv`; update the doc comments that
  describe bwrap's reaper (`supervise`, `launch`) to say the runner is PID 1
  and that bwrap's `wait` returns after the kernel empties the namespace.
  **Test:** the golden argv test lists `--as-pid-1` exactly once, after
  `--new-session` and before `--clearenv`; the stand-in `bwrap_probe` test
  still refuses by name (the stand-in sees the new token in its argv); AC-L3
  on the host proves the lifetime chain with the runner as init.
- [ ] **T15 — Refuse unexposed external agents under `--isolated` (R2, R8;
  AC-S3).** Add `EXTERNAL_AGENT_UNSUPPORTED = "isolation_external_agent_unsupported"`
  and check it in `isolation::request` after `EXPOSE_INVALID` and before
  `SOURCE_INVALID`: a `Some(agent)` whose canonical path is not among the
  canonical expose files refuses. Reorder `isolate()` so `SWARMAIL_BIN` and
  the agent program are resolved only after the artifacts, wrapper, expose
  and external-agent checks. Expose the sentinel agent in the stand-in tests
  and the test executable wherever it is the agent in `trial_isolation.rs`.
  **Test:** the precondition-order unit table gains the new name in its
  place; `isolated_preconditions_refuse_through_the_real_cli` gains: an
  unexposed `/bin/sh` refuses `isolation_external_agent_unsupported` with
  one harness-error row and exit 2 and no agent sentinel; the two-fault cases
  (`--artifacts-dir` plus a missing program refuses
  `isolation_artifacts_unsupported`; a missing expose plus `/bin/sh` refuses
  `isolation_expose_invalid`); an exposed external program reaches the
  controller (refuses `boundary_unavailable:bwrap_probe` in the seat, by
  name, not a precondition). One assertion in the matrix argv tests that no
  rendered trial argv contains `--isolated`.
- [ ] **T16 — Complete the void rule (R4; AC-L1).** In `cross_trial_probe`,
  before routes 14 and 15, parse `/proc/net/tcp` and `/proc/net/tcp6` for
  `LISTEN` local ports and return 90 when A's MCP or fixture port is among
  them; delete the same-role authority comparisons; keep the prompt/HOME
  path void. **Test:** the always-on negative control is extended with a
  case where the "victim" MCP port is a port the probe's own process is
  listening on, which must return 90 (Linux); the hostile live proof keeps
  its two-retry bound and named third failure on the host.
- [ ] **T17 — Extend the in-boundary observer (R1, R6, R7; AC-L2).** In
  `boundary_observe`: `/proc/1/exe` must read as the runner path the test
  passes (`runner=`), else 42; walk `/home` recursively and return 45 on any
  entry that is neither a bound file nor a directory on the path to one;
  for every numeric `/proc/<pid>` readable, for every entry of
  `/proc/<pid>/fd`, if opening it as a directory succeeds and that directory
  contains an entry named by any passed host PID, return 50 (an unreadable
  fd table counts as denied); record on stderr, without asserting, the
  nested-userns result, whether `/proc/1/mem` opens `O_RDWR`, and the
  `kernel.yama.ptrace_scope` value the test read on the host. Pass
  `--deadline-ms 30000` to the oracle trial. **Test:** `observer_check`
  names code 50; the oracle trial's argv in the test carries the deadline;
  on the host the observer exits 0 or 100 and prints its three observations;
  a bwrap that still installs a reaper is red at 42 by name.
- [ ] **T18 — Small drifts (R9, R10, R13, R14; AC-S2, AC-S4, AC-S9).**
  Canonicalize the agent program once in `isolate()` and use that path in
  both the bind and `inner_args`; replace the single hostile-argv example
  with a table over `--ro-bind`, `--share-net`, `--as-pid-1`, `--`,
  `--isolated` and `--chdir` at every index; in the stand-in `bwrap_probe`
  test capture the controller's stderr and assert the stand-in's
  `bwrap: <marker>` line is there and not in the row; remove the raw-worker
  sentence from `admission::root_for`'s doc comment. **Test:** a unit case
  with an agent program under a symlinked directory renders the canonical
  path inside and binds the same path; the table test passes; the stderr
  assertion passes; `grep -n "stages its mount points" evals/src` is empty.
- [ ] **T06 — Hostile live proof, boundary holds (AC-L1).** As at bdc504f
  with T16's void rule. **Test:** on this host, the probe exits 0 (all
  routes denied), A's row after its agent is killed from outside is `fail`
  in signup's declared idle class, never `pass`/`budget`/`harness_error`; a
  leak or a third void makes it red.
- [ ] **T07 — Fresh HOME, own services and the observed topology (AC-L2).**
  As at bdc504f with T17's observer. **Test:** on this host, the observer
  exits 0 (or 100 with the nested-userns observation) and prints its
  observations; a nonzero code names the first violated check.
- [ ] **T08 — Lifetime bound on both ends (AC-L3).** As at bdc504f with the
  runner as PID 1. **Test:** on this host, `SIGKILL` to the controller
  leaves no trial process alive; `SIGKILL` to the runner's host PID leaves no
  agent alive and the controller exits 2 with the `isolated_trial_no_row`
  row; `SIGTERM` to the controller after the agent exists yields the runner's
  interrupted row, exit 2 and no survivors; no orphan of A is alive when B
  starts.
- [ ] **T19 — Documentation (AC-S8).** Revise the dated #221 sections of
  `evals/README.md` and `evals/run-plan.md`: `--as-pid-1` and why; the
  external-agent refusal and that an isolated row is not an eval result;
  macOS compiles the suite and proves the platform refusal only; the proof
  command has not yet run; stock Ubuntu 24.04 apt bubblewrap is below the
  floor. **Test:** a requirement-to-observation read against 0076 and 0115;
  no guarantee downgrade, no `confinement` relabel, no claim that CI or
  macOS proves the boundary, no claim that the proof ran.
- [ ] **T10 — Gates (AC-S7).** Run the full house gate inventory locked plus
  the standalone evals `fmt`, `clippy --all-targets` and `cargo test
  --locked`. **Test:** command/exit/log receipts for every required check
  bound to the candidate, literal zero-missed root coverage with its
  selftest, publish-guard and seal proofs, `cargo deny check licenses` and
  the fail-closed native-tls absence; neither lockfile nor `deny.toml`
  changed; CI-owned obligations retained, not claimed; the run does not
  report complete while AC-L1..L4 and AC-O1 are open.
- [x] **T11 — Egress question to the operator (AC-L4, AC-O1).** Carried in
  0115, reframed for bwrap; T15 is what keeps real drivers out until the
  operator rules. **Test:** no egress code, launcher, daemon, dependency or
  credential copy is added; `cargo deny check licenses` passes unchanged; the
  decision stays proposed.
- [ ] **T20 — Live proof outstanding (AC-L1..L3).** After T13–T18 the
  implement seat runs the `--ignored` command in the seat and shows every
  live proof red with `SKIP trial-isolation: boundary_unavailable:bwrap_probe`;
  the dispatcher runs the same command on the host after the run. **Test:**
  the seat's red-by-name run is in the implement notes; the host run is the
  only green that closes T06–T08.

Status 2026-11-01 (chief, after the review return): T13–T20 are open for the
implement seat, in that order, T06–T08 behind them and proven only by the
dispatcher's one command on this host. The earlier status notes of the
implement seat (bdc504f) stand in git history.
