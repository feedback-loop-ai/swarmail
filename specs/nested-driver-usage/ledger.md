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

# Second rework: run issue-404-…-8cc2a397 (after the hold on a73ac17a)

Part of #404. Base 2f1b878d (the chief's revision). The commits are unsigned
and stay on `feat/route-b-tokens-404`. Nothing was pushed. Held paths: only
`.brokkr/recipes/eval/bundle.json` (approved by name); no lock, `deny.toml`,
`scripts/`, `.github/` or root `src/` change.

## The security hold (all four fixed)

| item | fix | proof that fails without the fix |
|---|---|---|
| 1. gate on the actual selection (c46fc46c) | `wrappers::argv_selects_paid` decodes each agents-file entry's `--config` selection; `matrix --brokkr` refuses `brokkr_paid_unbound` for any paid entry (profile or argv-only) before dispatch. Every paid (v2) wrapper without a grant is the recipe route and proves `binds::verify_live` before any credential read, direct as well as nested. | `report.rs::brokkr_mode_refuses…` (argv-only v2 entry behind a free `oracle` slot); `wrappers::tests::the_paid_decision_reads_the_selection_the_argv_names`; `launch::paid_selection_rechecks…` (bare direct dsh → `paid_unbound: recipe_bind_`, HOME empty, CLI never ran). |
| 2. coordinator-owned binding (c46fc46c, 3c10e6df) | `evals/src/grant.rs`: the baseline coordinator gives each paid slot's worker one `SOCK_SEQPACKET` endpoint per expected agent (fd 4..); the worker hands agent `i` its own at fd 4; the wrapper sends its binding and launches only on `granted`. Both sides read `SCM_CREDENTIALS`: the reply must come from an ancestor running this executable, the request from a descendant of the coordinator's own worker. One answer per endpoint. Reserved runner switches (`--paid-launch`, `--launch-report`, either spelling) refuse `agent_argv_reserved_flag` in `trial` argv, agents files and `validate_request`. `pin.release()` runs on every nested route. | `nested.rs::an_unbound_nested_selection_refuses_before_any_spend`: self-computed binding → `paid_grant_channel_missing`; caller-made socket answered `granted` by the test → `paid_grant_unauthenticated`; wrong grant → `paid_grant_refused`; second launch on one endpoint → `paid_grant_channel_closed`. `a_three_agent…`: the worker with a paid context but no endpoints → `paid_grant_channel_missing`, driver never ran. `runner.rs::caller_argv_never_carries_a_runner_switch`, `report::tests::agent_files_refuse_bad_shapes`, `grant::tests`. The nested tests now launch through `grant-standin` (the coordinator's own endpoint code); `paid_baseline.rs` (6 tests, 98 s) drives the real coordinator for direct paid launches. |
| 3. no production env bypass (c46fc46c) | `EVAL_TEST_STANDIN_DRIVER_SHA256` is gone. A release build admits `DRIVER_RELEASE_SHA256` alone; a debug build (`debug_assertions`) also admits the `paid-cli-standin` built beside it, and refuses any paid launch whose credential holds no `"SYNTHETIC-` value (`paid_seam_build_refused`), on every route, genuine pin included. | `wrappers::tests::a_nested_pin…` (the old variable admits nothing; synthetic/real credential table); `nested.rs::a_nested_contract…` and `an_unbound…` (altered driver bytes → `driver_release_unrecognized` with the old variable set); the stand-in still drives every offline proof. A release-profile build was not run (CI builds debug); the release arm is `cfg(not(debug_assertions))` returning `false`. |
| 4. actual codex class (0fbe6563, f6efe3f7) | `ToolSurface::declared_class` records `read-only sandbox (driver-selected)` or `workspace-write …`, read by `wrappers::argv_codex_class` from the same selection that sets the driver's `--sandbox` and the contract identity; the worker's expected surface and the coordinator's agree (`WorkerContext::apply`). Validation admits the two classes (or `unread`, which never launches). | `provenance::via_brokkr_tests::a_via_brokkr_surface_names_what_its_route_applies`; `nested.rs::a_nested_contract…` per class: driver argv `--sandbox <class>` and `make_provenance` descriptor. |

## Medium and low items

| item | disposition |
|---|---|
| claude via descriptor | fixed (0fbe6563): the amended via profiles state the nested launch (acceptEdits, user-scope MCP, settings allow-list, model and effort) with unrestricted builtins as a residual; the codex via descriptor drops `ephemeral` and ignored config. Superseded via profiles keep their published descriptor (the default-contract golden rows carry `claude_2_1_310_via_brokkr`). The test pinning equality was replaced. |
| baseline report | fixed (728ae012): `baseline_execute` uses `artifacts::companions` over `<output>/paid` and `report_with_usage`; `Plan.report_argv` prints `--usage-from <output>/paid`. `paid_baseline.rs` runs the printed command against the retained root; `baseline.rs` pins the argv. **Not executed:** a non-dry-run `baseline --via-brokkr` product end to end (no nested baseline fixture exists; the nested companion reader is exercised by the trial and report tests instead). |
| duplicate owners on resume | fixed (728ae012): `Evidence.owners` claims every committed leaf in `Ledger::read`. The baseline-shaped root proof (`duplicate_owners_refuse_over_a_baseline_root`, inside the three-agent test) runs both `duplicate usage owner` arms and `usage row mismatch` over `paid/<attempt>/trials/<id>` with an interrupted attempt and a capture tree. **Not separately executed:** the `Ledger::read` claim itself (needs a nested baseline product). |
| honest proof claims | fixed (a408a081): T51 now claims release-only admission for release builds; T53 splits the host-route real driver and the mount primitive (both run by the dispatcher on 2026-11-24 at a73ac17a) from brokkr applying the bundle to a seat, which stays open. |
| bind list | fixed (831f9cdb): `~/.local/bin`, `~/.claude.json`, `~/.dsh/spark.env` `ro` in the bundle, `binds.rs` and the recipe pin; a missing source (the systemd resolver included) refuses `recipe_bind_missing`. Claude and codex auth stay `ro`; a paid claude/codex launch on the recipe route refuses `paid_unbound: recipe_auth_unruled` until the operator rules (the smoke needed them writable). |
| docs | fixed (a408a081): README's re-prove claim (now true for dsh; claude/codex refuse), single-leaf vs whole-root ownership, "trial's figures come", the grant, reserved switches and trust set in `wrappers.md`. |
| coverage (low) | added: both duplicate-owner arms and row mismatch; `usage_companion_unretained` (`a_scored_nested_trial_without_retention_is_a_harness_error`); the `usage.json` stage and the marker after it in `every_publication_stage…`; the mask arm; an injected 200 ms grace for `deaf` (debug-only `EVAL_TEST_DRIVER_EXIT_GRACE_MS`, capped at the production 3 s). |
| assignment attempt | the companion stays version 1. Its slot's `matrix_id` is the paid attempt id, and resume ties it to the attempt through the plan and the row's cell (0138's dated note). No separate field. |

Cold defensive arms left as the panel accepted: `binds.rs` `recipe_bind_unverifiable` (unreadable `/proc/self/mountinfo`), `Reaper::reap`'s EINTR loop, `artifacts::trial`'s invalid-companion decode arm, `interpreted` `package_too_large` at `FILE_LIMIT`, and new in `grant.rs`: `MSG_TRUNC`/`MSG_CTRUNC`, missing `SCM_CREDENTIALS`, `socketpair`/`setsockopt` failure and the ancestor walk's 64-step bound.

## Host-gated proofs (the dispatcher runs them; not run here)

From `evals/`, on the candidate:

```bash
cargo test --locked --test wrappers -- --ignored a_read_only_bind_rejects_a_write_and_verifies_in_its_namespace
EVAL_BROKKR_DRIVER=~/.local/share/brokkr-0.15.2/brokkr cargo test --locked --test wrappers -- --ignored real_driver
```

Both passed at a73ac17a (`.evidence/host-proofs-2026-11-24.md`). `real_driver`
now launches through `grant-standin` and also asserts that fd 4 never reaches
the driver or CLI. Still unobserved: brokkr applying the bundle's binds to a
recipe seat and the release driver inside one. Operator-only and unticked: one
authenticated session per family (T55).

## Gates (this seat, Linux)

- Root, at f6efe3f7's tree: `cargo fmt --check` and evals `fmt --check`
  passed. Root `clippy --workspace --all-targets --locked -D warnings`, `cargo
  build --locked --bin swarmail` and `cargo test --workspace --locked` passed
  (all result lines 0 failed, 3 min 41 s). `coverage-gate.sh` passed at exact
  100/100/100 (functions 3504/3504; root `src/` untouched) and its selftest
  passed. `publish-guard-test.sh` and `seal-branch-test.sh` passed. `cargo
  deny check licenses`: ok.
- Recipes: the pinned brokkr 0.12.0 is not on this host, so **the pinned
  compile was not run**; CI's `brokkr bundle (pinned)` is its proof. With
  v0.15.2 (`.evidence`), `.brokkr/recipes/eval` compiles with exit 0, no
  `lets the run go on without reading`, and the three new binds in the
  output; the other recipes fail under 0.15.2 on unrelated provider checks,
  as before.
- evals: `clippy --all-targets --locked -D warnings` clean (Linux). **macOS
  not compiled here**: Linux-only code is under `cfg(target_os = "linux")`
  with refusing stubs elsewhere, and the new tests carry the non-Linux ignore.
- evals full suite, at ceb85398's tree: `SWARMAIL_BIN=<candidate
  target/debug/swarmail> cargo test --locked --no-fail-fast` passed, 27
  result lines, 0 failed (24 min 49 s on the shared host; lib 222, wrappers
  31 + 2 ignored, paid_baseline 6, report 44, runner 39). An earlier run at
  a408a081 failed the two default-contract golden tests (the superseded
  `claude_2_1_310_via_brokkr` descriptor had changed); f6efe3f7 restores it.
- Added test time: about +3 s in the nested group (one new single-agent
  trial, the three-agent test's refused first worker, the forged-grant
  launches), less about 2.8 s saved by the injected 200 ms grace; unit and
  report additions are under 1 s.

## Open (stated, not hidden)

- `baseline --via-brokkr` has still not run a nested product end to end, so
  the baseline's report.json over real nested companions and the
  `Ledger::read` duplicate-owner refusal are executed only through the
  shared reader (`artifacts::companions`, `UsageOwners::claim`) and the
  direct product, not on a nested baseline.
- The pinned 0.12.0 recipe compile, macOS clippy/tests, and the host-gated
  proofs on this head are CI's and the dispatcher's.
- A release-profile build of `swarmail-evals` was not built here; the
  seam's absence there rests on `cfg(debug_assertions)`.


## Third rework (run …-50013b57, after the security hold on 6074b83e)

Part of #404. Implement seat, Linux, 2026-11-24. Commits 82308737 through
761451c6 on top of the chief's 7fd5587c.

| finding | change | proof (executed here) |
|---|---|---|
| **security hold** (pre-queued reply) | `grant.rs` protocol `swarmail-grant/1`: the wrapper pins the endpoint's creator (`SO_PEERCRED` pid + `/proc` start time, live ancestor running this exe), refuses input queued before it asks (`paid_grant_reply_stale`), sends 128 bits from `getrandom` (`paid_grant_entropy_unavailable` on failure), and accepts only the registered pid's (`SCM_CREDENTIALS`, start time unchanged) byte-exact echo of nonce + binding (`paid_grant_reply_unbound`, `paid_grant_unauthenticated`). The coordinator refuses an expired or other binding and echoes the nonce. | `grant::tests::the_coordinator_answers_only_its_own_fresh_binding` (pure `answer`/`judge` table), `grant::tests::an_endpoint_names_its_creator_and_any_queued_packet`; `nested.rs::an_unbound_nested_selection_refuses_before_any_spend` now drives the debug `grant-standin --fault prequeue|nonce|contract|sender` (stale, unbound, unbound, unauthenticated; each under 5 s, no driver record, empty HOME) beside the kept channel-missing, forged-peer, refused and replay cases; `paid_baseline::one_command_…` passes through the real coordinator (98 s). |
| ruling 4 | dated threat model in `grant.rs`'s module doc and `wrappers.md`'s paid boundary; the spec already carried it (7fd5587c), its "still requires repair" sentence now says repaired. No "admits nothing" same-uid claim remains. | text |
| also 1 (macOS) | `paid_selection_rechecks_…` carries the non-Linux ignore; the bare refusal is the new portable `a_bare_paid_launch_refuses_unbound_on_every_platform`; the forged-peer pair is a Linux-only `cloexec_pair` helper (atomic `SOCK_CLOEXEC`, panel low 4). | `cargo clippy --target aarch64-apple-darwin --all-targets --locked -- -D warnings` clean (with a stub C compiler for ring's build script; type-checks, does not link or run). It caught a duplicate `ignore` before commit. macOS *execution* is CI's. |
| also 2 (full inventory) | `binds::verify_recipe`: DNS + all nine `~/` read-only entries resolved against the seat HOME + selected paths, then `~/.dsh/profiles` must be a writable `overlay` mount (`recipe_overlay_missing`, `recipe_overlay_not_private`). The HOME is read from the selection's nearest `.config/swarmail-evals` ancestor (`recipe_bind_home_unknown` otherwise) instead of a new request field: `trial.sh` is a held path this run may not edit, and the selection directory is itself a declared, re-proven bind. | `binds::tests::the_recipe_check_proves_every_declared_source_and_the_overlay` (writable sibling under `.volta`, missing `.dsh/spark.env`, absent / ordinary-rw / ro-overlay profiles, valid overlay). Docs: wrappers.md recipe bullet, inventory paragraph and count (thirteen ro + one overlay), README both passages, T53, bundle description. |
| also 4 (release seam) | `wrappers::driver_admitted`; ignored `release_seam.rs::release_standin_is_not_admitted` (refuses to pass in a debug build). | **Ran here**: `cd evals && cargo test --release --locked --test wrappers -- --ignored release_standin_is_not_admitted` → 1 passed at 37a44cb9's tree (38 s incl. build). |

### Not done in attempt 1 (required; attempt 2 below closes also 3)

- **Also 3, the three baseline-CLI proofs**: duplicate-owner refusal through
  `baseline --resume`, the paid baseline's own `report.json` over nested
  companions plus replaying `report_argv`, and the codex class change
  through `baseline --resume`. They need an offline nested `baseline
  --via-brokkr` product: `fake_driver.py` hard-codes one `FAMILY` per
  driver file while the baseline passes one `--via-brokkr` path for all
  three families, and no stand-in driver mode yet plays an agent that
  passes a deliverability task. That fixture is the next attempt's work.
- Lows left: exact inert-fixture debug admission (correctness 5 / plan
  item 5; `SYNTHETIC-` substring still decides), the empty-families matrix
  fixture (correctness 6), explicit wrong-slot ownership test. Shell
  indirection hiding a wrapper from matrix preflight and deliberate
  same-uid activity stay residuals (ruling 4; the wrapper's own check is
  mandatory).
- Cold arms not exercised, by reason: `grant::place`'s two `dup` failures,
  `grant-standin`'s `program_missing`/spawn failure, `run_child_assigned`'s
  endpoint-allocation refusal (needs fd exhaustion), the exit-grace clamp,
  `getrandom` failure, and `serve_faulty`'s `fork` failure arm.

### Host-gated proofs for the dispatcher (from `evals/`, on this head)

```bash
cargo test --locked --test wrappers -- --ignored a_read_only_bind_rejects_a_write_and_verifies_in_its_namespace
EVAL_BROKKR_DRIVER=~/.local/share/brokkr-0.15.2/brokkr cargo test --locked --test wrappers -- --ignored real_driver
cargo test --release --locked --test wrappers -- --ignored release_standin_is_not_admitted
```

The mount proof now also binds the whole fixture inventory read-only
(EROFS on `.dsh/spark.env`) and mounts the profiles as a bubblewrap
`--tmp-overlay` (write succeeds, source unchanged, `verify_recipe` passes
inside and fails on the host). It could not run in this seat (`bwrap:
Creating new namespace failed … ENOSPC`). The third command passed here.
Still unobserved: brokkr applying the binds to a recipe seat; operator
sessions (T55) stay unticked.

### Gates run here (Linux)

evals `fmt --check`, `clippy --all-targets --locked -D warnings` (Linux and
the aarch64-apple-darwin type-check), `cargo test --lib` (224 passed),
`--test wrappers` (all non-ignored passed, 152 s), `--test paid_baseline`
(6 passed, 98 s); root `fmt --check`, `cargo test --test delivery_recipes`
(23 passed). **Not run in this attempt**: the full evals suite, root
clippy/test/coverage/selftest, publish/seal proofs, `cargo deny`, and the
pinned brokkr 0.12.0 compile (not on this host; CI). Root `src/` is
untouched. Added test time: about +1 s in the nested group (four fault
launches), under 0.1 s of unit tests.

## Third rework, attempt 2 (same run, 2026-11-24)

Part of #404. Implement seat, Linux. Commits 8f3a4370 through f2156e9e on
top of 27419145.

| finding | change | proof (executed here) |
|---|---|---|
| also 3 (three baseline-CLI proofs; correctness 1–3 and 7, security 3, tests 1) | `common::paid::Paid::nested()`: the offline baseline fixture routed through one stand-in driver (`fake_driver.py` with `FAMILY=None` takes the family from `driver <family>`; per-family finals in the mode file), CLIs under the names the driver looks up, dsh's entry starting the pinned runtime, every family's nested pin (codex `workspace-write`). `standin_python` moves to the shared fixture (reused by `wrappers_launch`). | `paid_baseline::a_nested_baseline_reports_its_companions_and_resume_refuses_what_they_contradict` drives the real `baseline --via-brokkr` (32 slots): the printed plan's `report_argv` ends `--input <out>/paid.jsonl --usage-from <out>/paid`; the baseline's own `report.json` has no `incomplete_attribution` and each of six groups reads exactly 5 × the family's session bill (total and cache_read), `tokens_per_success` typed `no_passes`; running `report_argv` as printed gives the identical report. **Duplicate owner**: leaf B (another attempt) takes leaf A's driver attempt id with its own trial/slot/profile and a re-bound marker; both leaves verify alone, `baseline --resume` exits 2 with `duplicate usage owner` and writes no report. **Codex class**: every retained codex row records `workspace-write sandbox (driver-selected)`; a selection changed to `read-only` refuses the resume (`baseline_provenance_invalid`); restored, it reopens; a retained row rewritten to `read-only` under the fixed selection (result digest repaired, leaf valid alone) refuses the same way. Restored, the complete product reopens to its own `report.json` bytes, and the driver record shows no launch since the first run. 105 s. |
| correctness low 6 (empty-families fixture) | The matrix preflight test (`report.rs::brokkr_mode_refuses_without_artifacts_realm_or_a_nesting_deadline`) and the argv unit table now name a well-formed paid claude v2 selection; the unit table asserts the reader parses it as paid. | both tests pass |

**Host-gated for time, not privilege.** The nested baseline test is
`#[ignore]`: the smallest product is 32 slots, about 105 s, more than the
CI budget allows ("a few seconds" per the brief). It ran three times here,
all passing (the last at f2156e9e's tree). Dispatcher command (from `evals/`):

```bash
cargo test --locked --test paid_baseline -- --ignored a_nested_baseline
```

The other host-gated proofs, unchanged from attempt 1, stay listed above
(mount proof, real driver, release seam).

### Lows left, with reasons

- Exact inert-fixture debug admission (correctness 5, tasks.md line 14):
  the offline fixtures differ by family (a codex JWT whose payload encodes
  the canary, a literal `SOURCE-REFRESH…` refresh token), so an exact
  admission rule means redesigning each fixture's shape. Under ruling 4, a
  debug build is a non-release driver, and the release build carries no
  seam (`release_standin_is_not_admitted`, run in attempt 1). Debt.
- Explicit wrong-slot/attempt ownership test through the version-1
  assignment chain (tasks.md line 13): `Companion::validate`'s
  `usage_binding_mismatch` is unit-tested; the CLI-level swap is not built.
  Debt.
- tasks.md line 10's group with a pass *and* a scored failure: the
  stand-in driver does not yet play an agent that passes a task, so the
  baseline-level proof covers the all-fail group (typed `no_passes`).
  Per-success arithmetic stays proven at the report unit level. Debt.
- Grant serve accepting any same-exe descendant (correctness 4): a
  deliberate same-uid bypass, a residual under ruling 4.
- Assignment id outside the row (third panel low 3): the deviation is
  recorded in 0138's note. The version-1 slot binds through `matrix_id`.

### Gates run here (Linux, at f2156e9e's tree unless noted)

- **Root**: `fmt --check`; `clippy --workspace --all-targets --locked -D
  warnings`; `cargo test --workspace --locked` (all pass, 0 failed, 3 min
  41 s); `cargo build --locked --bin swarmail`; `coverage-gate.sh` (exact
  100/100/100); `coverage-gate.sh selftest` (pass); `publish-guard-test.sh`
  (OK); `seal-branch-test.sh` (7 cases, 10 mutants); `cargo deny check
  licenses` (ok). These ran at 1d7bc19a. The commit after it touches only
  `evals/tests` and this spec folder.
- **Evals**: `fmt --check`; `clippy --all-targets --locked -D warnings`
  (Linux, and an aarch64-apple-darwin type-check with a stub C compiler,
  which does not link or run); `SWARMAIL_BIN=<root target/debug/swarmail>
  cargo test --locked`, the full suite once at 1d7bc19a: 663 passed,
  0 failed, 11 ignored, 24 min 50 s wall on a shared, loaded host. f2156e9e
  changes only the ignored test, re-run above.
- **Not run here**: the pinned brokkr 0.12.0 recipe compile (no brokkr
  binary in this box; CI's `brokkr bundle (pinned)`). macOS execution is
  CI's.
- **Added CI time**: about 0 s. The new baseline test is ignored, and the
  two low-6 cases add well under 0.1 s.
- **Unchanged**: root `src/` is untouched. The only held path changed on
  this branch is the approved `.brokkr/recipes/eval/bundle.json`, from
  attempt 1; this attempt touched none.

## Third rework, review return on c93e3931 (same run, 2026-11-24)

Part of #404. Implement seat, Linux. All five members returned `residual`:
one medium (security 1, the deliberate same-uid class) and lows and infos
elsewhere. This attempt fixes the only security finding a code change can
close on this branch, closes the cheap lows and answers the rest below.

| finding | change | proof (executed here) |
|---|---|---|
| security 2 / correctness 2 (seam marker in any field) | `wrappers::seam_build_refuses_real_spend` reads only the authorizing values for the selection's credential key: `claude` → `claudeAiOauth.accessToken`, `codex` → `tokens.access_token` and `OPENAI_API_KEY`, `dsh:<route>` → that route's `key`. Each present must begin `SYNTHETIC-` or be a JWT whose signature segment decodes to bytes that begin so; at least one must be present. The fixtures whose spark key (`SPARK-CANARY`) or stale codex token (`…sig`) were not synthetic now are. | `wrappers::tests` seam table: decoys with the marker in `scopes`, mid-value, in a non-string, in the id token beside a real JWT access token, in the other dsh route, with a real `OPENAI_API_KEY`, no authorizing field, unknown key, unparsable file. `--test wrappers` 35 passed (3 ignored), `--test paid_baseline` 6 passed. The first full run of `wrappers` caught the two non-synthetic fixtures, as the change intends. |
| correctness/guarantees/independent low (wrapper → `source_home` → `verify_live` wiring unproven) | `binds::verify` checks the presence of every source before protection. A missing declared HOME source therefore refuses `recipe_bind_missing` even where a host's writable DNS file would refuse first. | `a_recipe_selection_reproves_every_declared_home_source` (real wrapper, dsh selection under `<tmp>/source-home/.config/swarmail-evals`): `~/.volta` absent → `recipe_bind_missing`; created → `recipe_bind_writable`; HOME empty, no CLI launch. An empty-inventory mutant fails it (observed). It needs a host with all four DNS inputs and returns early otherwise. This seat lacks two, so I created them temporarily in the seat's own `/etc` and `/run` to run the proof and the mutant. CI's runners and the operator's host carry them. 0.1 s. |
| guarantees 1 / independent 4 (0138's note) | 0138's closing paragraph states the build now in force (authorizing field, not one exact fixture, and why) and that the release and baseline-CLI proofs exist, host-gated. 0138 stays `proposed`. | text |
| guarantees 2 / tests 1 / correctness 5 (tasks.md) | Boxes reconciled: ticked items are delivered, unticked are open, each with a one-line reason. | text |
| guarantees 5 (T51) | T51 names the authorizing-field check and the release-profile proof with its command. | text |
| independent 3 (mount-proof fixture) | `~/.local/share/brokkr-0.15.2` is now made a directory: only `.claude.json` and `spark.env` are files. | compiles; the proof is host-gated |

### Answered, not changed

- **Security 1 (medium), the deliberate same-uid class.** A same-uid
  ancestor running this executable can answer the wrapper's nonce, and the
  serving side trusts any same-exe descendant of the worker. Under the
  operator's 2026-11-24 ruling 4 (spec.md, wrappers.md, grant.rs) this is a
  residual, not a hold. Such an actor could equally run the CLI directly
  with the operator's credentials. The security member classed it so too.
- **Correctness 1 (swapped agent/assignment refusal)** is the same class;
  tasks.md says so.
- **Independent 1 / guarantees 4 (the baseline-CLI proofs are ignored).**
  They are host-gated for time (about 105 s, against the CI budget the
  brief fixes). The dispatcher must run the command below on the final
  head before merge.
- **Correctness 3 / tasks line 10 (a group with a pass and a scored
  failure at the baseline entry point).** Debt. The stand-in driver cannot
  play a passing agent, and the arithmetic is unit-proven in report/usage.
- **Correctness 4 (companion version 1).** As recorded in 0138's note.
- **Independent 6 (`SO_RCVTIMEO` expiry reads as `paid_grant_channel_closed`;
  `runs_this` checked at registration only).** Low and diagnostic. The
  start-time check at receipt covers pid reuse, and exec keeps pid and start
  time, which is the same-uid class.
- **Version-1 wrong-slot swap at the CLI** (tasks line 13). Debt;
  `usage_binding_mismatch` is unit-tested.

### Host-gated proofs for the dispatcher (from `evals/`, on the final head)

```bash
cargo test --locked --test wrappers -- --ignored a_read_only_bind_rejects_a_write_and_verifies_in_its_namespace
EVAL_BROKKR_DRIVER=~/.local/share/brokkr-0.15.2/brokkr cargo test --locked --test wrappers -- --ignored real_driver
cargo test --release --locked --test wrappers -- --ignored release_standin_is_not_admitted
cargo test --locked --test paid_baseline -- --ignored a_nested_baseline
```

Added CI time: about 0.1 s (the new wrapper-level test, and the seam
table's 16 cases).

### Gates run here (Linux, at 3ad93a6f's tree; the commit after it changes only this ledger and tasks.md)

- **Root**: `fmt --check`; `clippy --workspace --all-targets --locked -D
  warnings`; `cargo test --workspace --locked` (exit 0, every result line
  0 failed, about 2 min); `cargo build --locked --bin swarmail`;
  `coverage-gate.sh` (exact 100/100/100, 6 min 29 s); `coverage-gate.sh
  selftest` (pass); `publish-guard-test.sh` (OK); `seal-branch-test.sh`
  (7 cases, 10 mutants); `cargo deny check licenses` (ok). Root `src/` is
  untouched.
- **Evals**: `fmt --check`; `clippy --all-targets --locked -D warnings`
  (Linux, and the aarch64-apple-darwin type-check with stub `cc`/`ar`,
  which does not link or run); `SWARMAIL_BIN=<root target/debug/swarmail>
  cargo test --locked`, the full suite once: 664 passed, 0 failed, 11
  ignored, 1502 s wall.
- **Host-gated, run here**: `release_standin_is_not_admitted` in the
  release profile (1 passed, 34 s including the build); `a_nested_baseline`
  (1 passed, 104 s). The mount and real-driver proofs could not run here
  (no user namespaces in this seat; no brokkr 0.15.2 binary).
- **Not run here**: the pinned brokkr 0.12.0 recipe compile (no brokkr
  binary on this host; this attempt touched no `.brokkr/` path, and CI's
  `brokkr bundle (pinned)` job and the verify seat run it). macOS
  execution is CI's.

## Third rework, review return on 9f28d4e8 (same run, 2026-11-24)

Part of #404. Implement seat, Linux. All five members returned `residual`.
The worst finding was security 1, medium: the deliberate same-uid class.
The rest were low or info. This attempt closes the cheap lows, and below
it answers the medium with evidence.

| finding | change | proof (executed here) |
|---|---|---|
| correctness 2 / independent 4 (CLI-level wrong-slot swap through the version-1 chain) | `a_nested_baseline` adds a case: leaf B's companion names leaf A's `slot`, and B's marker is re-bound to the new bytes, so it is hash-valid. The baseline's refusal now carries the leaf's own reason (`attempt <id>: trial_invalid <trial>: <reason>`); before, `main.rs` dropped it. | `baseline --resume` exits 2 with `usage_binding_mismatch` and writes no report; restored, the rest of the test holds. 1 passed, 106 s (host-gated, at f7c4a851). |
| guarantees 2 / independent 2 / tests 2 / correctness 4 (inventory wiring green-by-skip) | `a_recipe_selection_reproves_every_declared_home_source` still skips on a host that lacks a DNS input, and names the missing inputs, but on GitHub's runners (`GITHUB_ACTIONS=true`) it fails instead, so in CI it executes or goes red. The first version keyed on `CI`, which this delivery seat also sets; the full suite caught that, and the key changed. | Here it skipped (this seat lacks two DNS inputs; `GITHUB_ACTIONS` unset), and with `GITHUB_ACTIONS=true` it fails naming them (observed). CI's ubuntu runner is a bare VM with systemd-resolved. |
| tests 4 (ignore reason) | The nested baseline's `#[ignore]` reason and doc say it is Linux-only (grant endpoints, `/proc`). | text |
| correctness 5 (release_seam comment) | The comment and module doc say the release pin refuses after the private selection read and before the grant, the credential read or any launch. | text |

### Answered, not changed

- **Security 1 (medium), the deliberate same-uid class.** This is a residual
  under the operator's 2026-11-24 ruling 4, not a hold. The evidence for
  how far it reaches:
  - *Post-peek reply.* A reply counts only when its kernel-stamped
    `SCM_CREDENTIALS` name the registered coordinator pid with an unchanged
    start time. Without `CAP_SYS_ADMIN`, a process cannot claim another
    pid's credentials. So a packet sent after the freshness peek by
    anything other than the coordinator process refuses. The `sender`
    fault proves it: `nested.rs`, the right nonce and binding sent by a
    forked child, refused. Answering the nonce also means reading it from
    the coordinator's end, which only the coordinator holds (`SOCK_CLOEXEC`).
  - *What remains.* Two paths still answer: the coordinator process
    itself, if a same-uid actor started it or injected into it, and a
    same-exe descendant of the worker. Both require deliberately running
    or tampering with this executable as the operator's uid. Ruling 4
    excludes that actor, who could run the CLI with the credentials
    directly. spec.md, wrappers.md and grant.rs record it.
- **Correctness 1 / guarantees 1 / independent 1 / tests 1 (baseline-CLI
  proofs are `#[ignore]`).** They are host-gated for time (about 106 s,
  against the CI budget). The dispatcher runs the command below on the
  final head.
- **Correctness 1 / independent 4 (a baseline group with a pass).** This
  stays debt. The stand-in driver cannot play a passing agent, and the
  per-success arithmetic is unit-proven.
- **Tests 3 (elapsed < 5 s in the fault loop).** Kept. The bound is half the
  coordinator's 10 s `SO_RCVTIMEO`. That margin is what tells a refusal
  that comes before the wait from one that times out.
- **Tests 5 (`driver_admitted` only in the release proof).** Kept. It is
  the release-profile check, and the debug suite cannot reach it.
- **Guarantees 3 / independent 3 (mount and real-driver proofs not
  observed on this head).** This seat has no user namespaces and no
  brokkr 0.15.2 binary. The dispatcher runs them.

### Host-gated proofs for the dispatcher (from `evals/`, on the final head)

```bash
cargo test --locked --test wrappers -- --ignored a_read_only_bind_rejects_a_write_and_verifies_in_its_namespace
EVAL_BROKKR_DRIVER=~/.local/share/brokkr-0.15.2/brokkr cargo test --locked --test wrappers -- --ignored real_driver
cargo test --release --locked --test wrappers -- --ignored release_standin_is_not_admitted
cargo test --locked --test paid_baseline -- --ignored a_nested_baseline
```

Added CI time: 0 s. The new case is in the ignored test, and the CI
assertion is a branch.

### Gates run here (Linux; root `src/` untouched, one `evals/src/main.rs` line changed)

- **Root**: `fmt --check`; `clippy --workspace --all-targets --locked -D
  warnings`; `cargo build --locked --bin swarmail`; `cargo test --workspace
  --locked` (every result line 0 failed, 219 s); `coverage-gate.sh` (exact
  100/100/100, 397 s); `coverage-gate.sh selftest` (pass);
  `publish-guard-test.sh` (OK); `seal-branch-test.sh` (7 cases, 10
  mutants); `cargo deny check licenses` (ok).
- **Evals**: `fmt --check`; `clippy --all-targets --locked -D warnings`.
  The full suite ran once with `SWARMAIL_BIN=<root target/debug/swarmail>
  cargo test --locked`: 663 passed, 1 failed, 11 ignored, 1499 s, at
  f7c4a851 plus the ledger commit. The one failure was the `CI`-keyed
  assertion above, because this seat sets `CI=true`. After the switch to
  `GITHUB_ACTIONS`, the whole `--test wrappers` target ran again: 33
  passed, 0 failed, 3 ignored. No other target changed between the two
  runs.
- **Host-gated, run here**: `a_nested_baseline`, 1 passed, 106 s.
- **Not run here**: the pinned brokkr recipe compile (no brokkr binary in
  this box; no `.brokkr/` path was touched). macOS execution is CI's.

## Fourth rework, review return on 1370fe63 (same run, 2026-11-24)

Every member returned `residual`; no hold. Part of #404.

### Changed

| finding | change |
|---|---|
| security low / correctness low 2: `wrappers.md` listed "a caller-made socket" as refused before the threat-model paragraph | the refusal list now says "a socket whose creator is no live ancestor running this executable", which is what `grant::request` checks; a same-uid creator that is such an ancestor is the ruling-4 class |
| security low: shell indirection hides a paid wrapper from `matrix --brokkr`'s argv preflight | `wrappers.md` now says the preflight reads the argv as written and is an early refusal, not the boundary; the wrapper's own launch-time check (bare paid launch → `paid_unbound`, proven by `a_bare_paid_launch_refuses_unbound_on_every_platform` and the nested refusal tests) still refuses before any spend. Parsing shell scripts would be a heuristic and is not attempted |
| correctness low 5 / independent low 4: AC5 said "assignment attempt" without the qualifier | spec AC5 now says the attempt is carried as the slot's `matrix_id`, as 0138's dated build note records |

### Answered, not changed

- **Security medium residual (deliberate same-uid creator / `serve` trusting
  any same-exe descendant of the worker).** Routed as a residual by the
  operator's 2026-11-24 ruling 4, recorded in `spec.md`, `wrappers.md` and
  `grant.rs`. Binding the agent ordinal/argv into the grant would not change
  the class: a same-uid process can run the CLI directly.
- **Proof placement (all members).** The baseline-CLI, release-seam, mount
  and real-driver proofs stay host-gated under the CI budget; commands
  below, unchanged. Their test content is unchanged since they last passed.
- **No passing baseline group (AC6 per-success at the entry point).**
  Unchanged debt, tasks.md line 10: the stand-in driver cannot play a
  passing agent; the arithmetic is unit-proven.
- **Info items** (DNS-input skip off GitHub, overlay fstype unobserved,
  SO_RCVTIMEO reason, macOS execution CI-only): unchanged, as recorded.

### Host-gated proofs for the dispatcher (from `evals/`, on the final head)

```bash
cargo test --locked --test wrappers -- --ignored a_read_only_bind_rejects_a_write_and_verifies_in_its_namespace
EVAL_BROKKR_DRIVER=~/.local/share/brokkr-0.15.2/brokkr cargo test --locked --test wrappers -- --ignored real_driver
cargo test --release --locked --test wrappers -- --ignored release_standin_is_not_admitted
cargo test --locked --test paid_baseline -- --ignored a_nested_baseline
```

Added CI time: 0 s (documentation only).

## Race isolation (run issue-404-…-a2022fe0, 2026-11-24)

Part of #404 and #164. Spec, plan and tasks: `specs/eval-race-isolation/`.
Test races only; no production behaviour, boundary, ruling or proof meaning
changes. Host: shared Linux box, load average 19–24 throughout (other
sessions), so wall times below are noisy.

### Finding 1 — driver pin (`a_nested_contract_binds_the_driver_pin_and_its_flags`)

- **Shared state.** The test wrote and removed the process-global
  `EVAL_TEST_STANDIN_DRIVER_SHA256` with `std::env::set_var` while sibling
  test threads read the environment and spawn children; so did the inline
  `wrappers::tests` pin proof. Both writes are deleted. The variable has
  admitted nothing since hold 3 on a73ac17a, so the writes proved nothing
  the child form does not.
- **The adversarial input is kept, child-scoped.** The existing
  altered-driver launch in `an_unbound_nested_selection_refuses_before_any_spend`
  now carries the variable, naming the altered bytes' digest, through
  `Command::env` on the wrapper child; it still refuses
  `driver_release_unrecognized`. The release-profile refusal
  (`release_seam.rs`) already passes it the same way and is unchanged.
- **Point-of-use check.** After the restore and reselect, the test asserts
  that the restored private bytes equal the originals and that the selected
  digest, the private driver's digest and the digest of the image the debug
  admission hashes (`current_exe`'s profile directory, one up from `deps/`,
  joined with `wrappers::STANDIN_DRIVER_NAME`) agree, reporting each read as
  `Ok`/`Err` and the image path. A recurrence now names the diverging input
  instead of surfacing as the verifier's refusal.
- **Mechanism, honestly bounded.** From the code, nothing in the `wrappers`
  harness writes `target/debug/paid-cli-standin` (every fixture copies it,
  `install_native`/`install_configured`; `tests/identity.rs` and
  `tests/runner.rs` build into private `--target-dir`s), every fixture path
  is a private `/tmp/wrapper-proof-*` directory, and the debug admission
  never reads the environment. The only process-shared state this test
  touched was the environment write, which is undefined behaviour under
  concurrent `getenv`/spawn in other threads; it is removed. I could not
  reproduce the Ubuntu refusal locally before or after, so the CI failure's
  exact interleaving is not demonstrated; the new check makes the next one
  self-describing. No production defect is shown.
- **Production-file exception.** `evals/src/wrappers.rs` changes only inside
  `#[cfg(test)] mod tests`: the unsafe `set_var`/`remove_var` pair and the
  assertion between them are replaced by a comment pointing to the child
  proof. No non-test line changes.
- **Evidence.** `cargo test --locked --test wrappers` (default threading,
  load 23.6): 33 passed, 3 ignored (host-gated, unchanged), 152.7 s.
  Added time: one file read and three SHA-256s of a ~1 MB image per family,
  well under 0.1 s.

### Finding 2 — grant lifetime (`paid_selection_rechecks_the_pin_and_validity_before_exec`)

- **Shared state: the wall clock.** `bound()` and the second Claude launch
  built the binding as `<identity>:<now>` — valid through the current
  integer second only. The coordinator stand-in answers with
  `grant::answer(…, now())`, reading the clock again after the wrapper has
  probed, spawned and sent its request; `unexpired` is `t >= now`. When a
  second ticked in between — likelier the more sibling tests load the host —
  a valid request was refused and the wrapper reported
  `paid_grant_refused`. This is the fixture's horizon, not a production
  defect: production bindings are horizons the coordinator chooses, and
  `answer`'s rollover semantics (granted at `t`, refused at `t+1`) are
  already proven by `grant::tests::the_coordinator_answers_only_its_own_fresh_binding`
  (unchanged; no `grant.rs` edit was needed).
- **Fix.** One helper, `launch_horizon()` = `common::paid::now() + HOUR`,
  builds every intended-valid direct binding; argv and the coordinator's
  expected binding come from the same string (`bound()`, and the second
  Claude launch now takes both from one `[flag, binding]`). The horizon is
  inside both positive logins (claude's ends in 2033, dsh's in 2100). The
  margin refusal keeps its horizon beyond claude's expiry (2100), the drift
  and invalid-binding cases keep theirs, and expired fixtures stay expired.
- **Evidence.** Same wrappers run as finding 1: 33 passed, 152.7 s. Added
  time: 0 s (no new launch or wait).

### Finding 3 — report admission (`brokkr_mode_commits_a_harness_error_row_over_a_passing_state`, macOS)

- **Shared state: the fixed trial lock and one agents file.**
  `brokkr_mode_refuses_without_artifacts_realm_or_a_nesting_deadline` holds
  no `trial_scope`. Its two `agents_file` calls both wrote
  `<dir>/paid/agents.json`, so the argv-only fixture (oracle first, then a
  direct paid wrapper) replaced the profile declaration before the "bound"
  probe ran. That probe, in stand-in mode `ok` on the bound realm, passed the
  preflight and dispatched the free oracle slot; the stand-in ran the
  recipe's real `trial.sh`, whose trial took
  `/tmp/swarmail-evals-<uid>/trial.lock` (`admission.rs`: a fixed root that
  ignores HOME and TMPDIR, so macOS's `/var/folders` alias does not separate
  it). `agreeing_case` holds the tests' cooperating scope
  (`/tmp/swarmail-evals-tests-<uid>.lock`), not that lock; its oracle slot
  overlapping the stray trial got `concurrent_trials_refused` (accepted
  0076, production behaving as ruled), the matrix stopped and filled the
  unstarted suffix — three rows, one logged request: the `left: 1,
  right: 3` failure. Linux can interleave the same way; macOS's slower
  runner made the window wider.
- **Fix (test-only).** The argv-only declaration gets its own directory and
  the test asserts the two paths differ and that `paid` still names
  `claude-nested`. All three paid-preflight probes run the stand-in's
  `no-row` mode, which logs the dispatch and exits 3 before `trial.sh` on
  either OS, so none can take the trial lock even if a negative control
  regresses; each probe has its own request log. The probes now assert:
  each unbound declaration — exit 2, empty stdout, `brokkr_paid_unbound`,
  zero requests, no artifact root; the bound profile — exactly one request
  and `trial_row_missing`.
- **A latent vacuity, found while doing so.** Even with the right file, the
  bound probe never dispatched: its argv `/nonexistent/agent` refused as
  `agent_program_unavailable`, and the old assertion (only "no
  `brokkr_paid_unbound`") passed anyway. The declaration now names
  `common::TRUE_PROGRAM`, which the no-row stand-in never runs, so the
  control proves dispatch. The unbound refusals are unchanged.
- **Audit.** Every other dispatching `matrix` call in `tests/report.rs`
  holds `trial_scope` (`agreeing_case`, `defect_case`, the signal cases via
  their callers, the overlap proof); the remaining unscoped calls in this
  test are preflight refusals (exit 2, no rows) that never dispatch.
  `sequential_matrix_refuses_overlap_and_unsupported_concurrency` still
  passes.
- **Evidence.** `cargo test --locked --test report` (default threading,
  load 12–20): 44 passed, 153.7 s. Added time: the bound probe now stops
  at `no-row` instead of running an oracle trial, so the test is faster;
  net under 0 s.

### Full evals suite runs (Linux, this host)

Candidate binary: `cargo build --locked --bin swarmail --message-format=json`,
compiler-artifact `executable` =
`/home/valentin/source/feedback-loop-ai/swarmail-landing/rb-404/target/debug/swarmail`.
Command, from `evals/`, one suite at a time, default test threading:
`SWARMAIL_BIN=<that path> timeout 45m cargo test --locked`.

| run | tested revision | started (UTC) | elapsed | outcome |
|---|---|---|---|---|
| 1 | `ebb8362c` (code); load 17.9 → 13.4 | 2026-11-24 23:10:51 | 1523 s | exit 0 — 693 passed, 0 failed, 11 ignored (host-gated, unchanged), 28 test binaries |
| 2 | `1bcc5309` (code as `ebb8362c`; ledger-only since); load 24.0 → 8.1 | 2026-11-24 23:48:11 | 1498 s | exit 0 — 693 passed, 0 failed, 11 ignored, 28 test binaries |
| 3 | `0c51b37c` (code as `ebb8362c`); load 7.5 → 8.0 | 2026-11-25 00:13:30 | 1511 s | exit 0 — 693 passed, 0 failed, 11 ignored, 28 test binaries |
| 4 | `ba48ac14` (code as `ebb8362c`; next implement attempt, same run); load 13.9 → 13.4 | 2026-11-25 00:41:48 | 1511 s | exit 0 — 693 passed, 0 failed, 11 ignored, 28 test binaries |
| 5 | `71667c66` (code as `ebb8362c`); load 12.7 → 22.2 | 2026-11-25 01:07:10 | 1514 s | exit 0 — 693 passed, 0 failed, 11 ignored, 28 test binaries |

Five consecutive passes on identical code (`ebb8362c`; every later
commit touches only `specs/`), run one at a time: runs 1–3 by the first
implement attempt, runs 4–5 by the next attempt of the same run, each
under `timeout 45m` (run 3 under `timeout 30m`). **AC4's five-run
streak is complete.** A later code change restarts it.

### Gates run here (Linux, code at `ebb8362c`; later commits are ledger-only)

Root `src/` is untouched by this run.

- `cargo fmt --all -- --check` (root and `evals/`): clean.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
- `cd evals && cargo clippy --all-targets --locked -- -D warnings`: clean.
- `cargo build --locked --bin swarmail`: ok (artifact path above).
- `cargo test --workspace --locked`: exit 0, 2212 passed, 0 failed,
  57 ignored, 220 s.
- Evals suite: run 1 above.
- `bash scripts/coverage-gate.sh`: pass, exact 100/100/100, 396 s.
- `bash scripts/coverage-gate.sh selftest`: pass, 20 s.
- `bash scripts/publish-guard-test.sh`: OK, 7 s.
- `bash scripts/seal-branch-test.sh`: 7 cases and 10 mutants passed, 5 s.
- `cargo deny check licenses`: licenses ok.
- **Not run here**: the pinned brokkr 0.12.0 recipe compile — no brokkr
  binary in this box, as in every earlier seat of this run. No `.brokkr/`
  path was touched (not even the approved `.brokkr/recipes/eval/bundle.json`).
  The verify seat and CI's `brokkr bundle (pinned)` own it.
- **CI-owned, not observed here**: macOS execution (the report fix's
  `no-row` probes use only the stand-in's pre-`trial.sh` exit, which
  `defect_case("no-row", …)` already runs on macOS), Helm, Docker smoke,
  RustSec.

### Diff audit (task 4)

`git diff 1112d05d..ebb8362c -- evals/`: three test files and one
`#[cfg(test)]` block in `evals/src/wrappers.rs` (the deleted unsafe env
pair). No other `evals/src/` line, no root `src/`, no held path, no
`#[ignore]` added or widened, no lock or dependency change. No
`std::env::set_var`/`remove_var` remains under `evals/tests/` or in
`evals/src/wrappers.rs`. The one left in `evals/src/main.rs` is the
test `matrix_probe_entrypoint`, which writes `SWARMAIL_BIN` only when
re-entered as a dedicated probe child process (its `MATRIX_PROBE_ENV`
guard returns at once in the normal harness), so it never mutates a
shared harness's environment; it is outside this run.

### Gates re-run on the final tree (Linux, `71667c66`, code as `ebb8362c`)

Second implement attempt, 2026-11-25, after runs 4 and 5. Load 13–40.

- `cargo fmt --all -- --check`, root and `evals/`: clean.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  root and `evals/`: clean.
- `cargo build --locked --bin swarmail --message-format=json`: the
  artifact path above.
- `cargo test --workspace --locked`: the **first run failed one test**,
  124 s, at load 40. The test was
  `mail_compare::api_section::only_the_holder_enters_the_delegated_scope`
  (`tests/mail_compare/api_section.rs:5606`), which hit `ExecutableFileBusy`
  ("Text file busy") when it spawned a script it had just written. A sibling
  thread's fork held the write descriptor across the exec. The test and its
  file come from main, through `5b4a99ed`. Under root `tests/`, this branch
  changes only `tests/delivery_recipes.rs`, and this run is scoped to the
  evals races, so it is reported here and not changed. The rerun passed:
  exit 0, 2212 passed, 0 failed, 57 ignored, 223 s.
- `bash scripts/coverage-gate.sh`: pass, exact 100/100/100, 393 s.
- `bash scripts/coverage-gate.sh selftest`: pass, 20 s.
- `bash scripts/publish-guard-test.sh`: OK.
- `bash scripts/seal-branch-test.sh`: 7 cases and 10 mutants passed.
- `cargo deny check licenses`: licenses ok.
- **Not run here:** the pinned brokkr 0.12.0 recipe compile. No brokkr
  binary is on this host's PATH. No `.brokkr/` path changed. The verify
  seat and CI's `brokkr bundle (pinned)` own it.
- **CI-owned:** macOS execution, Helm, Docker smoke, RustSec.

Added CI time for the three fixes: at most about 0.1 s net. The report
probe now stops at `no-row`, which makes it faster. The pin check adds
three hashes per family.
