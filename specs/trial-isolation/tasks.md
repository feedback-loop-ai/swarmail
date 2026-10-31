# Trial isolation tasks — preparation on main

Ordered. Each task names the test that proves it. AC references point to
[spec.md](spec.md). A proposed decision is not authority to activate a
contradictory contract (accepted 0045 §8); concurrency stays off and no row
version changes in this run.

- [x] **T01 — Carry the decision and its index row (AC-S8, AC-O1).** Keep
  `docs/decisions/0110-trial-isolation.md` as `proposed` (committed by the
  chief with this design) and add its row to `docs/decisions/README.md` in the
  implementation commit. **Test:** `design-check.sh` resolves `Decision: 0110`
  to one clean proposed record the spec links; `git diff main --
  docs/decisions/0055-agent-eval-harness.md docs/decisions/0076-sequential-eval-trials.md`
  is empty; the README has exactly one 0110 row.
- [ ] **T02 — Establish the leak live, before any boundary (AC-S1).** Add
  `evals/tests/trial_isolation.rs` with an always-on test: hold an idle signup
  victim (`held_idle`), a same-uid helper reads the victim agent's `EVAL_*`
  from `/proc/<pid>/environ` after finding it among the victim runner's
  descendants, completes signup through real MCP and the real fixture, and the
  victim row is `pass`. **Test:** that test, green on every host; discovery is
  bounded to the victim runner's descendants.
- [x] **T03 — The pure boundary module (AC-S4).** Add `evals/src/isolation.rs`
  with the errno-name table, the bind allowlist resolver and the ordered step
  list, `#[cfg(target_os = "linux")]` body and a refusing `cfg` stub
  elsewhere; one line in `lib.rs`. **Test:** inline unit tests assert the
  errno table is total and closed, the resolver refuses relative/missing/
  non-regular/symlinked executables and accepts the named sources, and the
  step list orders identity maps first, `/proc` before pivot, and the
  parent-death signal after the last credential change.
- [x] **T04 — Entry, refusal and the injected-failure seam (AC-S2, AC-S3,
  AC-S5).** Add the hidden `--isolated` and repeatable `--isolated-expose`
  flags to `Cmd::Trial`; when set, run `isolation::enter` (taken as a
  parameter) before the Tokio runtime and continue into the unchanged `Trial`
  arm on success, else emit one `refused_row` and exit 2. **Test:** a unit
  test injects an entry failing `clone`/`ENOSPC` and asserts the row is
  `harness_error`, stderr names `boundary_unavailable:clone:ENOSPC`, exit 2
  and no sentinel; CLI tests cover the `SWARMAIL_BIN`-unset, `--artifacts-dir`,
  `--wrapper-profile`, bad-`--isolated-expose` and `--concurrency 2` refusals;
  `git diff main --stat -- src/` (root) is empty.
- [x] **T05 — Skip discipline for the live proofs (AC-S6).** Add the probe
  helper to `evals/tests/common/mod.rs`: a real null-agent `trial --isolated`
  classified from its refusal. **Test:** a namespace-dependent test marked
  `#[ignore = "trial-isolation: needs unprivileged user namespaces"]` prints
  `SKIP trial-isolation: boundary_unavailable:<op>:<ERRNO>` and panics when the
  boundary is unavailable; the standalone evals suite is green with it shown as
  ignored.
- [ ] **T06 — Hostile live proof, boundary holds (AC-L1).** The `#[ignore]`
  proof: victim A runs `trial --isolated /usr/bin/sleep` held under the
  deadline; the trusted test reads A's coordinates from the host `/proc`;
  trial B runs `trial --isolated` with the T02 helper as its agent and A's
  coordinates in argv; the helper reports every denied route by exit status.
  **Test:** on a capable host, the helper exits 0 (all routes denied), A's row
  is `fail` in signup's declared idle class with no accepted confirmation,
  never `pass`/`budget`/`harness_error`; a leak makes it red.
- [x] **T07 — Fresh HOME and own services through the boundary (AC-L2).** An
  `#[ignore]` proof. **Test:** on a capable host, a real child sees a fresh
  writable HOME, the previous isolated trial's sentinel is absent, and the
  oracle passes signup inside the boundary through its own MCP and fixture.
- [x] **T08 — Lifetime bound on both ends (AC-L3).** An `#[ignore]` proof.
  **Test:** on a capable host, killing the controller leaves no trial process
  alive; killing the init leaves no agent alive; an interrupt emits the
  interrupted row and the controller re-raises the signal; no orphan of A is
  alive when B starts.
- [x] **T09 — Documentation (AC-S8).** Add a dated section to
  `evals/README.md` and `evals/run-plan.md`: the hidden flag, its refusals,
  the host proof command, trials remain sequential, no row claims isolation.
  **Test:** a requirement-to-observation read against 0076 and 0110; no
  guarantee downgrade, no `confinement` relabel, no claimed reproduction of
  concurrency.
- [ ] **T10 — Gates (AC-S7).** Run the full house gate inventory locked plus
  the standalone evals `fmt`, `clippy --all-targets` and `cargo test --locked`.
  **Test:** command/exit/log receipts for every required check bound to the
  candidate, literal zero-missed root coverage with its selftest, publish-guard
  and seal proofs, `cargo deny check licenses` and the fail-closed native-tls
  absence; CI-owned obligations retained, not claimed; the run does not report
  complete while AC-L1..L4 and AC-O1 are open.
- [x] **T11 — Egress question to the operator (AC-L4, AC-O1).** Carry 0110's
  egress question and recommendation; `--isolated` refuses wrapper profiles
  until the operator rules. **Test:** no egress code, launcher, daemon,
  dependency or credential copy is added; `cargo deny check licenses` passes
  unchanged; the decision stays proposed.

Status 2026-10-31: T02 (live leak baseline) and T06 (hostile proof) are open —
see spec.md, Implementation status. T07 and T08 are written but have never
run (they need a namespace-capable host). T10 is recorded in the delivery
notes of this run.
