# Trial isolation tasks — the boundary is bubblewrap

Ordered. Each task names the test that proves it. AC references point to
[spec.md](spec.md) as revised 2026-11-01. A proposed decision is not authority
to activate a contradictory contract (accepted 0045 §8); concurrency stays off
and no row version changes in this run. Tasks closed by the 2026-10-31
implementation that this revision keeps stay closed; tasks it reopens are
unchecked again with their new test.

- [x] **T01 — Carry the decision and its index row (AC-S8, AC-O1).**
  `docs/decisions/0115-trial-isolation.md` stays `proposed`, now with its
  *Revision 2026-11-01* section (committed by the chief with this design);
  the README index row from 2026-10-31 stands. **Test:** `design-check.sh`
  resolves `Decision: 0115` to one clean proposed record the spec links;
  `git diff main -- docs/decisions/0055-agent-eval-harness.md
  docs/decisions/0076-sequential-eval-trials.md` is empty; the README has
  exactly one 0115 row.
- [x] **T02 — Establish the leak live, before any boundary (AC-S1).** Kept
  as built: `the_leak_is_live_a_same_uid_helper_makes_an_idle_victim_pass`
  and the probe's negative control. **Test:** both green on every host;
  discovery is bounded to the victim runner's descendants.
- [x] **T12 — Delete the raw-syscall worker (AC-S9).** Remove
  `evals/src/isolation/linux.rs`, `ERRNO_NAMES`/`errno_name`, `Step`/`STEPS`,
  `remount_flags`, `Observed`/`verify`/`interfaces`/`status_field`, the
  `Enter` type and `enter`, and the test-side `with_private_admission`;
  narrow `Refusal::Boundary` to `{ step }`. **Test:** `cargo clippy
  --all-targets --locked -- -D warnings` in `evals/` is clean with no dead
  code; `grep -rn "pivot_root\|SYS_clone\|uid_map\|setpriv" evals/src
  evals/tests` is empty; the kept unit tests for `resolve_source` and
  `bind_plan` still pass.
- [x] **T03 — The pure bwrap parts (AC-S4).** In `evals/src/isolation.rs`
  add `BWRAP`, `BWRAP_MIN`, `TMP_BYTES`, `parse_version`,
  `system_root_options` and `command(&Request, inner)`; beside `Cmd` add
  `inner_args(&Cmd)` destructuring `Cmd::Trial` without `..`. **Test:**
  inline unit tests: the version parser's accept/refuse table; the golden
  argv for a request with an agent and one exposed file (exact list, no `/`
  bind, no `-try`/`--unshare-all`/`--share-net`, `--clearenv` before every
  `--setenv`, `--size` immediately before `--tmpfs /tmp`, the first `--`
  immediately before the runner); the property that agent argv containing
  `--ro-bind`, `--share-net`, `--` and `--isolated` never precedes the first
  `--`; the clap round trip over a fully populated `Cmd::Trial` equals the
  original minus `isolated`/`isolated_expose` with the agent program
  absolute.
- [x] **T04 — The controller and the parameter seam (AC-S2, AC-S3, AC-S5,
  AC-S6 platform arm).** Replace `isolation::enter` with
  `isolation::launch(request, inner, bwrap: &Path)`: platform check,
  `bwrap_absent`, `bwrap_version` (5 s), `bwrap_probe` (10 s, real argv with
  `/usr/bin/true`), host admission, spawn on the main thread with
  `process_group(0)`, `kill_on_drop`, null stdin, piped stdout/stderr copied
  through, `close_range` CLOEXEC sweep in `pre_exec`, `SIGTERM`/`SIGINT`
  handling by signalling the runner's host PID, and the exit judgement. Wire
  `main` to `block_on` it for `--isolated` instead of entering the trial arm;
  remove the `unsafe set_var`. **Test:** `main.rs` unit tests pass, in turn,
  an absent path, a generated `bubblewrap 0.9.0` script, a `0.11.1` script
  that fails everything else, and a script that passes the probe then exits
  1 silently; each yields a `harness_error` row, exit 2, the named reason on
  stderr, no admission taken before the probe passed, and no agent sentinel.
  A `cfg(not(target_os = "linux"))` test proves `platform` (CI macOS) and a
  Linux unit test proves the platform-neutral refusal function. The existing
  CLI precondition tests (AC-S3) and `git diff main --stat -- src/` empty
  (AC-S5) stay green.
- [x] **T05 — Skip discipline for the live proofs (AC-S6).** Update
  `boundary_unavailable`/`require_boundary` in `evals/tests/common/mod.rs`
  to classify `boundary_unavailable:<step>` and the `#[ignore]` reason to
  `trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user
  namespaces`. **Test:** run explicitly in the seat, every live proof prints
  `SKIP trial-isolation: boundary_unavailable:bwrap_probe` and fails; the
  standalone evals suite is green with them shown as ignored.
- [ ] **T06 — Hostile live proof, boundary holds (AC-L1).** Rewrite
  `a_hostile_same_uid_agent_in_trial_b_cannot_reach_trial_a`: victim A is
  launched by the test through `isolation::command` (a new
  `launch_victim` helper in `common/mod.rs` with the controller's stdio
  discipline), its agent `/usr/bin/sleep` under the deadline; the test reads
  A's coordinates from the host `/proc`; trial B runs the real
  `trial --isolated --isolated-expose <test exe>` with the probe as agent;
  a route-90 void is retried at most twice. **Test:** on this host, the
  probe exits 0 (all routes denied), A's row after its agent is killed from
  outside is `fail` in signup's declared idle class, never
  `pass`/`budget`/`harness_error`; a leak or a third void makes it red.
- [ ] **T07 — Fresh HOME, own services and the observed topology (AC-L2).**
  Keep the fresh-HOME and oracle-signup proofs; add the `boundary_observe`
  helper mode and its test, with a canary file under the host `/tmp` and a
  canary descriptor held open in the test before launch. **Test:** on this
  host, the observer exits 0 (uid, `/proc`, `lo` usable, host paths and
  canary file absent, tmpfs size, four-variable environment, no stray
  descriptor, empty `CapEff`, `NoNewPrivs` 1) and prints its nested-userns
  observation to stderr; a nonzero code names the first violated check.
- [ ] **T08 — Lifetime bound on both ends (AC-L3).** Rewrite the three
  lifetime proofs for bwrap's topology. **Test:** on this host, `SIGKILL` to
  the controller leaves no trial process alive; `SIGKILL` to the runner's
  host PID leaves no agent alive and the controller exits 2 with the
  `isolated_trial_no_row` row; `SIGTERM` to the controller yields the
  runner's interrupted row, exit 2 and no survivors; no orphan of A is alive
  when B starts.
- [x] **T09 — Documentation (AC-S8).** Revise the dated #221 sections of
  `evals/README.md` and `evals/run-plan.md`: bwrap is the mechanism, the host
  prerequisite, the closed refusal set, the proof command, trials remain
  sequential, no row claims isolation. **Test:** a requirement-to-observation
  read against 0076 and 0115; no guarantee downgrade, no `confinement`
  relabel, no claimed reproduction of concurrency, no claim that CI proves
  the boundary.
- [ ] **T10 — Gates (AC-S7).** Run the full house gate inventory locked plus
  the standalone evals `fmt`, `clippy --all-targets` and `cargo test
  --locked`. **Test:** command/exit/log receipts for every required check
  bound to the candidate, literal zero-missed root coverage with its
  selftest, publish-guard and seal proofs, `cargo deny check licenses` and
  the fail-closed native-tls absence; neither lockfile nor `deny.toml`
  changed; CI-owned obligations retained, not claimed; the run does not
  report complete while AC-L1..L4 and AC-O1 are open.
- [x] **T11 — Egress question to the operator (AC-L4, AC-O1).** Carry 0115's
  egress question, reframed for bwrap; `--isolated` refuses wrapper profiles
  until the operator rules. **Test:** no egress code, launcher, daemon,
  dependency or credential copy is added; `cargo deny check licenses` passes
  unchanged; the decision stays proposed.

Status 2026-11-01 (chief): T12, T03, T04, T05, T06, T07, T08, T09 and T10
are open for the implement seat. T06–T08 are written in the seat and proven
only by the dispatcher's one command on this host after the run; the seat
must show them red with the named `bwrap_probe` skip, never green.

Status 2026-11-01 (implement seat): T12, T03, T04, T05 and T09 are done and
proven in the seat (unit tests in `evals/src/isolation.rs` and
`evals/src/main.rs`, the always-on `tests/trial_isolation.rs`, and the
`--ignored` run showing `SKIP trial-isolation: boundary_unavailable:bwrap_probe`
red for all seven live proofs). T06, T07 and T08 are **written and compiled but
not run**: `/usr/bin/bwrap` 0.11.1 refuses with `ENOSPC` inside the seat, so
they stay open until the dispatcher's one command passes on the host. T10: every
gate in the inventory passed locked except `brokkr compile --bundle
.brokkr/recipes/self`, which could not run because the pinned brokkr binary
(`tooling.json`, 0.12.0) is not installed in the seat; the recipe is untouched.
Two deviations from the drawn design, both small: the private `/tmp` is mounted
before the file binds (spec implementation note), and the interrupt/no-runner
arm is proven by a re-executed child test process rather than by a signal to
the test process.
