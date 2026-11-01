# Implementation record — gate the eval harness in CI and the verify seat

Implements [spec.md](spec.md) (issue #201, epic #192). Date: 2026-10-31. Base:
`8dfdf6b`, design commit `c0ad5c7`. Nothing here was pushed, merged,
published or accepted; no paid trial ran; no dependency, `deny.toml` or
lockfile changed.

## What changed

| File | Change |
|---|---|
| `.github/workflows/ci.yml` | `test` matrix job (ubuntu + macOS): toolchain `components: rustfmt, clippy`; cache `workspaces` `. -> target`, `evals -> target`; named steps `Resolve the candidate Swarmail binary`, `evals fmt`, `evals clippy`, `evals test` after `cargo test --workspace --locked` and before the port-80 steps. Timeout unchanged (30). |
| `.brokkr/recipes/self/scripts/verify-seat.sh` | Header names `evals/Cargo.toml` as the second manifest the host fetched; three `run` lines (`evals cargo fmt`, `evals cargo clippy`, `evals cargo test`) between root `cargo test` and `brokkr compile`; the last resolves `SWARMAIL_BIN` from `cargo build --locked --bin swarmail --message-format=json | jq` and fails closed (`test -x`); the pass note names the evals gates. |
| `tests/delivery_constitution.rs` | Seat digest re-pinned (`b42a7928…`); fake `cargo` answers `build` with a decoy bin, a lib and the real bin record; gate-order test now expects the four extra cargo calls and asserts each eval gate's cwd and that the suite got the resolved binary (not an inherited `SWARMAIL_BIN`); new `verify_seat_refuses_the_eval_suite_without_an_executable_candidate_binary`; new `ci_gates_the_eval_harness_on_both_operating_systems`. |
| `tests/store_contract_docs.rs` | The inventory test requires the three evals commands in house rules § Gates. |
| `docs/house-rules.md` | § Gates lists the build and the three evals commands and states why (standalone workspace, both OSes, candidate binary, deny gap). |
| `docs/decisions/0055-agent-eval-harness.md` | Dated addendum 2026-10-31 appended; `git diff` shows no removed line. |
| `docs/decisions/README.md` | 0055 row: "; amended 2026-10-31: CI and verify wiring (#201)"; status unchanged. |
| `evals/README.md`, `evals/Cargo.toml`, `evals/run-plan.md`, `specs/agent-eval-harness/spec.md`, `specs/eval-measurement-repairs/spec.md` | Deferral text dropped, "proposed" corrected for 0055 and 0076, ownership sentences to past tense, #149's location corrected (the `fixture` subcommand, `evals/src/fixture.rs`). |

The `#201` token survives in `evals/README.md`, `evals/run-plan.md`,
`specs/agent-eval-harness/spec.md` and 0055 (asserted by
`sequential_contracts_and_decision_links_are_honest`, which passes).

## Finding 3 (stdout channel): already closed

No code changed. `drain()` in `evals/src/runner.rs` retains one typed,
validated announcement and discards every other byte; the only
`unbounded_channel`s left are `#[cfg(test)]` seams. The proof is
`readiness_continues_draining_after_the_first_valid_announcement`
(`evals/tests/runner.rs`): 1 MiB after the announcement, pass, bounded output.
It is now gated by the new `evals test` step. Nothing was posted to #151.

## Wall time per gate (R7, AC9)

Linux, local, debug profile, `--locked`, on a 128-core workstation (a CI
runner has far fewer cores — treat as a floor). Cold = `rm -rf evals/target`;
warm = the same commands again. `SWARMAIL_BIN` was the resolved candidate
binary in both.

| Gate | Linux cold | Linux warm | ubuntu CI | macOS CI |
|---|---|---|---|---|
| Resolve binary (`cargo build --locked --bin swarmail`, root already tested) | 16.7 s (first build of the plain bin) | 0.2 s | from the PR's run | from the PR's run |
| `evals fmt` | 0.5 s | 0.5 s | from the PR's run | from the PR's run |
| `evals clippy` | 13.2 s | 1.9 s | from the PR's run | from the PR's run |
| `evals test` (245 tests, 0 ignored) | 618.4 s | 588.8 s | from the PR's run | from the PR's run |

The CI cells are the per-step durations GitHub shows for the PR's own run;
this run cannot push or observe CI, so none is invented. **The `evals test`
step is runtime, not compile time** (compile was ~17 s cold): trials are
sequential under 0076 and the suites are wall-clock bound. Cold per-binary
runtimes: lib 70 s, bin 24 s, baseline 49 s, boundary 0.4 s, fixture 0.4 s,
identity 55 s, proof 56 s, provenance 71 s, report 42 s, runner 108 s,
scoring 0 s, wrappers 124 s. Together with the root suite (176–186 s locally)
the root suite plus the evals suite is roughly 13 minutes of test time locally;
on 3–4 core runners and on macOS expect more. The job timeout stays 30
minutes per the design ruling; **operator follow-up: if the first run shows a
`test` job above ~21 minutes, raise `timeout-minutes` to 45 with that
measurement in the comment** (never a new job — a new, non-required context).

## Red proofs (R6, AC4, AC5) — scratch copies, nothing committed

(a) Root `Cargo.lock` `serde` 1.0.229 → 1.0.1 in a scratch copy; `cd evals &&
cargo test --locked --test boundary every_evals_package_is_already_in_the_root_lock`:

```
test every_evals_package_is_already_in_the_root_lock ... FAILED
assertion `left == right` failed: packages outside the root lock must be exactly the named 0055 exceptions
  left: {("pulldown-cmark","0.13.6",…), ("serde","1.0.229",…), ("unicase","2.10.0",…)}
 right: {("pulldown-cmark","0.13.6",…), ("unicase","2.10.0",…)}
test result: FAILED. 0 passed; 1 failed
```

(b) Root `src/mcp.rs` tools/call text block changed from `v.to_string()` to
`format!("OK {v}")` in a scratch copy, built, then `cd evals &&
SWARMAIL_BIN=<mutant> cargo test --locked --test proof`: **5 of 6 failed**
(`every_task_passes_with_the_oracle`, `every_task_fails_with_null_and_its_declared_idle_class`,
`wrong_actions_fail_in_their_observable_classes`,
`completed_naive_controls_recover_the_goal_but_lose_the_trial`,
`sequential_poacher_control_refuses_competing_trial`); the oracle trial exits 1
(`gave_up`) because `content[0].text` is no longer JSON. The same suite with
the clean binary: pass.

Incident, recorded for honesty: the mutant was first built with
`CARGO_TARGET_DIR` pointing at this worktree's real `target/`, so
`target/debug/swarmail` was the mutant for a while and a warm `evals test`
run failed in `retained_child_conflicts_with_cancellation_winning_commit_without_mutation`
(its real-worker oracle trial failed). `cargo clean -p swarmail` and a rebuild
restored the original binary (SHA-256 `a268d9c4…`, identical to the first
build), the trial passes, and every figure and receipt below was taken after
the restore. The scratch copy is deleted; `git status` shows no mutation.

Also exercised: with `|| true` appended to the `evals clippy` step in a copy of
`ci.yml`, `ci_gates_the_eval_harness_on_both_operating_systems` fails
(`evals clippy must not carry || true`); the file was restored.

## Gate receipts (local, `--locked`, final tree)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test --workspace --locked` | exit 0; 1112 passed, 0 failed, 10 ignored (1110 before: +2 new tests) |
| `cd evals && cargo fmt --all -- --check` | exit 0 |
| `cd evals && cargo clippy --all-targets --locked -- -D warnings` | exit 0 |
| `cd evals && SWARMAIL_BIN=<resolved> cargo test --locked` | exit 0; 245 passed, 0 failed, 0 ignored |
| `bash scripts/coverage-gate.sh` | exit 0; exact 100/100/100 (lines 20941/20941, branches 1602/1602, functions 2108/2108) |
| `bash scripts/coverage-gate.sh selftest` | exit 0 |
| `bash scripts/design-check.sh selftest` | exit 0 |
| `bash .brokkr/recipes/self/scripts/delivery-proof-test.sh` | exit 0 |
| `bash scripts/publish-guard-test.sh` | exit 0 |
| `bash scripts/seal-branch-test.sh` | exit 0 |
| `cargo deny check licenses` | exit 0 (`licenses ok`) |
| native-tls absent (`cargo tree -i native-tls` reports no match) | exit 0 |
| `sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256` | exit 0 |
| `cargo fetch --locked --offline --manifest-path evals/Cargo.toml` | exit 0 (baseline, before any edit) |
| Detached, depth-1 clone of the commit (how `actions/checkout` presents the tree): `cd evals && cargo test --locked --test provenance` and `--test boundary` | exit 0 (29 and 19 passed); no `fetch-depth` change needed |
| `brokkr compile --bundle .brokkr/recipes/self` | **not run here**: no `brokkr` binary exists in this seat's box (`/runtime` holds only a home directory; the tooling pin names a release artifact the box cannot fetch). The change touches only the seat script's body, which `bundle.json` references by path; the verify seat and the `brokkr bundle (pinned)` CI job run it. |

Limits that remain true: this run's own pinned verify seat predates the change
and will not run the new lines (the gate-order test proves the new seat's
sequence by execution against a fake `cargo`); the both-OS green is CI-only
proof; the macOS time is unobserved. `grep -rn fixture-app specs/` matches only
this specification and plan quoting the dead path, not a live claim.

## PR text (draft)

**Gate evals/ in CI and the verify seat (#201)**

`evals/` is its own workspace, so no gate reached its fmt, clippy, oracle/null/
isolation proofs or the lock-subset boundary test. The `test` jobs (ubuntu and
macOS) now run, from `evals/`, `cargo fmt --all -- --check`,
`cargo clippy --all-targets --locked -- -D warnings` and `cargo test --locked`
as named steps, with `SWARMAIL_BIN` resolved from Cargo's artifact record of
`cargo build --locked --bin swarmail` and asserted executable (the harness's
build-when-unset path cannot run in a gate). The pinned verify seat runs the
same three offline; house rules § Gates lists them. No job, status context,
dependency or script was added; 0055 changes by dated addendum only.

Proof: a shared root-lock entry bumped → `every_evals_package_is_already_in_the_root_lock`
fails; the MCP text block no longer JSON → the proof suite fails (5/6).
Stdout residual (finding 3): already closed by `drain()`, proved by
`readiness_continues_draining_after_the_first_valid_announcement`.

CI wall time added (per step; fill from this PR's run):

| Gate | ubuntu | macOS | Linux local (cold/warm, 128 cores) |
|---|---|---|---|
| Resolve binary | _run_ | _run_ | 16.7 s / 0.2 s |
| evals fmt | _run_ | _run_ | 0.5 s / 0.5 s |
| evals clippy | _run_ | _run_ | 13.2 s / 1.9 s |
| evals test | _run_ | _run_ | 618 s / 589 s |

Operator follow-ups: raise the `test` job's 30-minute timeout to 45 if the
first run shows it above ~21 minutes; evals licences stay checked by manifest
path (root `cargo deny` does not read `evals/Cargo.lock`); no host lock or
roster assertion in the seat (revisit with evidence).
