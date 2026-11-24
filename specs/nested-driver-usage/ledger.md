# Implementation ledger: run issue-404-…-b30e41de (rework after the security hold)

Part of #404 and epic #164. Base 2a3c01e1 (the chief's revision). These are the
operator's 2026-11-24 rulings, and each finding of `.evidence/review-417bfe9b.md`
with its disposition and the proof. The commits are unsigned and stay on
`feat/route-b-tokens-404`. Nothing was pushed.

## Operator rulings

1. **Boundary, fail closed** (72a9253b, 0d99c5e9).
   - A nested wrapper starts the driver only under one of two bindings:
     - the host baseline's `--paid-launch` binding, or
     - verified read-only recipe binds, read from its own `/proc/self/mountinfo`
       (`evals/src/binds.rs`).
   - The recipe check covers the four DNS inputs plus the selection, driver,
     CLI program, dsh package root and credential file. Each must sit under an
     `ro` mount with no writable mount stacked inside.
   - Without a binding the wrapper refuses before any credential read, HOME
     write or spawn: `nested_paid_unbound: recipe_bind_*` or
     `driver_release_unrecognized`.
   - `matrix --brokkr` with any paid agent refuses `brokkr_paid_unbound` before
     a slot unless the bundle declares every F5 bind.
   - Proofs:
     - `nested.rs::an_unbound_nested_selection_refuses_before_any_spend` and
       `a_nested_trial_without_a_binding_never_starts_the_driver`: the
       sentinel driver record stays absent.
     - `report.rs::brokkr_mode_refuses_without_artifacts_realm_or_a_nesting_deadline`
       covers an unbound bundle and the shipped one.
     - The `binds::tests` mount-table tables cover writable, stacked, missing,
       symlinked resolver and escaped names.
   - **F5 binds.** `.brokkr/recipes/eval/bundle.json` is the one held path the
     operator approved by name; no other held path was touched.
     - The `trial` seat declares these `ro`: `/etc/resolv.conf`,
       `/run/systemd/resolve`, `/etc/nsswitch.conf`, `/etc/hosts`,
       `~/.config/swarmail-evals` (selection), `~/.local/share/brokkr-0.15.2`
       (driver), `~/.local/share/claude`, `~/.volta`, `~/.claude` and `~/.codex`.
     - `~/.dsh/profiles` is an overlay.
     - The root `tests/delivery_recipes.rs` now pins exactly this set. A guessed
       or writable trial bind is refused, and so is any bind on the gate.
     - The source paths are inferred from 0083/0137 and the plan; this run had
       no smoke record of host paths. The dispatcher should confirm that they
       exist before a paid recipe run (`evals/wrappers.md`).
   - **Real mount proof:** `evals/tests/wrappers_launch/mount_proof.rs`.
     - It is **host-gated (ignored)** because this seat cannot create a user
       namespace (`bwrap: … max_*_namespaces exceeded (ENOSPC)`; `unshare -rm`
       fails the same way).
     - In bubblewrap, it writes through each read-only target to an
       owner-writable source and requires EROFS, plus a private write.
     - It checks the sources after teardown, and checks that the namespace's
       mountinfo passes `binds::verify` while the host's fails.
     - Dispatcher command:
       `cd evals && cargo test --locked --test wrappers mount_proof -- --ignored`.
       **Not run here.**
   - **Codex:** only `read-only` and `workspace-write` are admitted.
     - `danger-full-access` refuses `driver_sandbox_unconfined`. Tested in the
       unit table and in `the_driver_pin_refuses_before_any_private_write_or_spawn`.
     - The via codex `ToolSurface::declared` restriction now states the
       admitted classes and that the class is bound in the contract identity.
       The direct "read-only sandbox" is no longer claimed.
     - Residual: the descriptor is per profile, so it names the rule. The
       row's actual class is bound in `cli.sha256`, which resume and the
       launch binding compare, and is not spelled out per row.
     - The docs propose `workspace-write` for the operator's session.
2. **Multi-agent trials score** (30040b96).
   - `settle_launch` keeps one `Session` per expected agent in assignment
     order: a receipt, `receipt_missing`, or `agent_unstarted` for a foreign,
     duplicate or missing launch.
   - The first infrastructure ending is the trial's.
   - `Companion::totals` is the checked per-field sum. Any unavailable member
     makes that field unavailable, and overflow is `sum_overflow`.
   - Proofs:
     - `nested.rs::a_three_agent_nested_trial_commits_every_sessions_usage`
       runs a real `trial --task inbox-isolation` under a coordinator-shaped
       worker context and paid binding. Three stand-in sessions give
       distinguishable bills, a scored `fail` grade and three distinct
       attempts, and the sums are checked.
     - The `usage::tests` and `runner` unit tables cover lost, foreign,
       duplicate, explicit zero, absent, overflow and non-finite USD.
3. **0138's companion** (30040b96).
   - The schema-2 row has no `nested_usage`. A new row decodes with the strict
     `results::read` (unit test).
   - `usage.json` (version 1, source, trial_id, slot, profile, sessions) is
     published before `metadata.json` and bound there.
   - `artifacts::trial` verifies the companion and refuses an orphan, a
     missing or tampered companion, and a version, profile, roster, slot or
     duplicate-attempt mismatch. Matrix replay and `report --usage-from`
     refuse duplicate owners across leaves.
   - A scored nested row without a committed companion becomes
     `harness_error` (`usage_companion_unretained`).
   - `report` refuses a scored nested row without `--usage-from`
     (`nested_usage_requires_artifacts`) and never reads journal tokens for it.
   - 0138 stays `proposed`. A dated note records where the build differs:
     slot = matrix coordinates, no attempt-directory field, the refusal points,
     superseded via profiles, and the stand-in digest.

## Panel findings

| finding | disposition |
|---|---|
| tests MEDIUM: macOS compile of `real_driver.rs` | fixed (72a9253b): hard `cfg` removed; the test is `#[ignore]` everywhere, so its imports and helper are live on every platform. **Not compiled for macOS here**: `cargo clippy --target aarch64-apple-darwin` stops at ring's C build (no Apple cross toolchain). New code uses only libc items present for Apple (`waitid`, `P_PID`, `WNOWAIT`, `WEXITED`, checked in libc 0.2.190). CI's macOS job is the proof. |
| correctness 6 / independent 5: bounded shutdown | fixed (a113a985): stdin closed on every ending; `DRIVER_EXIT_GRACE` 3 s, then SIGKILL; `waitid(WNOWAIT)` before reaping so the kill never hits a reused pid. Proof: `garbage` (protocol error after accept; stand-in exits on stdin EOF) and `deaf` (ignores shutdown, holds stdout) modes keep their typed receipts. |
| correctness 7 / guarantees 5: production pin | fixed: coordinator admission (`paid_contracts`, nested) and the recipe-route wrapper require `DRIVER_RELEASE_SHA256` (`driver_release_unrecognized`); the archive line was already required. **Deviation from plan wording:** the offline stand-in is admitted by the test-equipment variable `EVAL_TEST_STANDIN_DRIVER_SHA256`, read only in the coordinator's process (the runner rebuilds agent environments, so the recipe route admits the release alone). It adds one trusted digest and skips no measurement. There was no other way to drive the coordinator offline with a stand-in. Reviewers may weigh it. |
| correctness 8: scored row without record | fixed (see ruling 3) |
| correctness 9 / independent 6: deadline kill | documented (`evals/README.md`, `wrappers.md` endings table): no receipt on termination; `receipt_missing`; group figure `missing_required_count` |
| independent 7: failure classes differ | documented (`evals/README.md`) |
| guarantees 4: README "Rows are unchanged", `context` wording, T52/T53 | fixed (0653a3f5); T51–T53 now separate executed, host-gated and operator-only evidence |
| correctness 10 / independent 8 (INFO) | not changed: `capabilities.supports` closed set and `/usr/bin/env node` decoy remain open (info level) |
| tests LOW arms | (2) non-UTF-8 link target executed; (4) dsh nested identity with no effort executed via `contract_identity` in every dsh launch; (5) DashScope overlay executed; (6) launch reader one byte over limit executed; (7) `read-only` deserialized; (1) `package_too_large` at FILE_LIMIT still unexecuted (needs >200k entries or a production seam, left as debt); (3) codex `driver_cli_unresolvable` is unreachable: an absolute program that passed `is_file` always has a parent; (8) `baseline --via-brokkr` non-dry-run is still unexecuted end to end (the paid binding is exercised through the worker context instead); (9)–(11) unchanged defensive arms |

## Gates (this seat, Linux, head d64835de unless noted)

- Root: `cargo fmt --check` passed. `clippy --workspace --all-targets --locked
  -D warnings` passed. `cargo test --workspace --locked` passed after the
  delivery_recipes update (38 result lines, all 0 failed). `cargo build
  --locked --bin swarmail` passed.
- Coverage: `scripts/coverage-gate.sh` passed at exact 100/100/100 (34741 lines,
  2274 branches, 3504 functions). The `selftest` passed. `publish-guard-test.sh`
  passed. `seal-branch-test.sh` passed. `cargo deny check licenses` passed.
- evals: `fmt --check` and `clippy --all-targets --locked -D warnings` passed.
  `SWARMAIL_BIN=<candidate> cargo test --locked` passed in full (all 27 result
  lines, 0 failed, about 25 min on the loaded host) at the tree before d64835de.
  d64835de adds two small test extensions, run on their own afterwards.
- The opt-in real release (`EVAL_BROKKR_DRIVER=.evidence/…/brokkr`,
  `real_driver`) passed for all three families on the host route, in 1.5 s.
- **Pinned brokkr compile: not run.** The pinned engine 0.12.0
  (`tooling.json`) is not on this host. With v0.15.2, `.brokkr/recipes/eval`
  compiles clean: no policy-sweep warning, and the binds appear in the output.
  The other recipes fail under 0.15.2 on unrelated provider-measurement
  checks. CI's `brokkr bundle (pinned)` job is the proof.
- **Not run:** the macOS clippy and tests (CI), and `mount_proof` (host-gated,
  dispatcher).
- Added test time: the nested group went from 4.0 to 8.3 s wall. That includes
  one intended 3 s grace wait and the three-agent trial, which takes about
  4 s. Report and unit additions are under 1 s. The total is about 5 s on CI.

## Operator-only

The authenticated finished-path session per family (T55) stays open. A bare
`trial` now fails closed, so the session runs through the host baseline's
authorized nested product or the recipe with binds (`evals/README.md`).
No single-trial paid entry point exists. Choosing the product is the
operator's call.
