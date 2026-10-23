# Implementation receipt — lane E, issue #176

Date: 2026-10-23. Run: `epic192-first-wave-lane-e-lifecy-1d453f48`. Triage: `engine`. Design commit: `ac5cbe1`, approved by the engine-owned deterministic check before implementation. Verified base: `168ac299e0fe16ecdd8fa0e0218cb7bc4e0a261c`. This receipt accompanies the unsigned implementation commit; it establishes this seat's local work, not completion of epic #192 or an independent review verdict.

## Finding and acceptance disposition

| Finding | Implemented proof or correction | Acceptance |
|---|---|---|
| 8.1 | Inline `tests::stop_joins_serve_before_drained_and_dispatcher_before_return` drives actual stop, holds tasks in both actual trackers, checks drained before/after serve release and pending stop until dispatcher release. | AC1; moved-send mutation fails the early-drained assertion. Held tasks witness tracker ownership only. |
| 8.2 | SMTP drain comment permits stop-time refusal before admission, whole admitted outcomes and uncertain no-reply commits/retry duplicates. | AC3; SMTP code unchanged. |
| 8.3 | Main calls private `start_after_registration` before invoking its startup continuation. `tests::signals_are_polled_before_startup` supplies local pending probes and immediate assertions; no watcher can satisfy them. | AC4; omitted SIGINT, omitted SIGTERM and registration deferred to watcher each fail. Existing real-child SIGINT, SIGTERM and forced-70 tests pass in workspace tests. |
| 8.4 | `lifecycle_tests::stalled_body_selects_drain_deadline` observes complete real HTTP 100 Continue, retains incomplete body/socket through serve completion, then requires the one-shot emitted only by the selected deadline arm. `graceful_http_completion_emits_no_deadline` requires receipt closure instead. | AC2; missing-receipt and bypassed-deadline mutants fail for their respective reasons. The old weaker duplicate was removed only after both negative controls passed. |
| 8.5 | Lifecycle/binary comments and the binary docstring describe router-entry/live request gauges, not parsing, blocking-worker admission, mutation queue position or a passed stop check. The held SQLite lock separately prevents storage. Related lib comments describe owned trackers and detached tails. | AC3; no test assertions or runtime policies changed by this prose correction. |
| 8.6 | Held-lock lifecycle docstring describes observed labeled requests; stale 300 ms narrative removed. | AC3. |

AC5: the API receipt is private, `cfg(test)` only, consumed by explicitly polling serve entry before spawning in an isolated current-thread runtime. Installed deadline, installed graceful and uninstalled paths execute. Stop and signal proofs remain inline. The root `src/lifecycle_tests.rs` harness is declared under `cfg(test)` because the byte-wise complete-head reader has cold EOF/oversize diagnostics and the watchdog has a failure-only abort; seam access is not the placement reason (0020). No dependency, feature, license, gate, timer or runtime shutdown-fence change. No sleeps were added.

## Commands, counts and artifacts

Run-local evidence root: `.forge/epic192/lifecycle-evidence/`. `mutations.json` records exact commands, exits, patch/original/mutated SHA-256 values and exact-byte restoration for each control. Every selected mutation test runs one test, fails with exit 101 for the named assertion/watchdog, and passes before and after with exit 0. The graceful control also passes in the final workspace suite.

| Mutation receipt prefix | Failure oracle | Patch SHA-256 |
|---|---|---|
| `stop-order` | drained preceded the serve-task join | `e8a8cda2231fc378c40ea074ab73d036c98402bbb09131a19a86d737b498eb1e` |
| `deadline-receipt` | the selected drain-deadline arm emitted no receipt | `ed32de44d952646c3e30a7e42c755b4cff40aa2d88e0c3c100bc7d11c67e8581` |
| `deadline-bypass` | stalled-body drain watchdog expired | `f4de08c997e1e75ddf1890fe36bdd0ba424b483b091c718fcbaa371acef2b6f0` |
| `signal-int` | SIGINT was not polled before startup | `cb53a820d589d474a58544352fe259eba9fe480d1cc6f6a7b1c44fa8184fb658` |
| `signal-term` | SIGTERM was not polled before startup | `647a1702310a1e87a1db7860f5d3dd718fe8e9780bb002c91cc7011901e9db09` |
| `signals-late` | SIGINT was not polled before startup | `7583ed0bc508b172ce8f1b70b121e950c142471ca609b0d5cca92e9df4320ccc` |

Receipt prefixes name `<prefix>.patch`, `<prefix>-before.log`, `<prefix>-red.log`, `<prefix>-restored.log` and adjacent `.exit` files. `signals-late` removes the eager registration block so production registration is left to the watcher's later recv poll; the deterministic startup continuation fails before background progress. No temporary mutation is in this commit.

Baseline `cargo test --locked --test lifecycle` passed 13 tests; `cargo test --locked --test binary` passed 32 with one existing ignore, recorded in `baseline-{lifecycle,binary}.{log,exit}`. Final workspace tests passed 773 with 3 existing ignores across targets: 326 library tests, one binary unit test, 12 lifecycle integration tests, 32 binary integration tests and all other workspace suites. Filters and target summaries are retained in logs. The lifecycle count decreases by one because its weak ten-second test is replaced by the two decisive owner-side tests.

All following commands exited 0. Full commands and log paths are in `gates-{base,coverage,proofs}.json`; each gate has `<gate>.log` and `<gate>.exit`. Cargo uses the launcher-mounted toolchain, 16 build jobs and offline cached dependencies.

| Gate | Executed command |
|---|---|
| fmt | `cargo fmt --all -- --check` |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| workspace-tests | `cargo test --workspace --locked` |
| compile | `brokkr compile --bundle bundles/self` |
| recipe-structure | `cargo test --locked --test delivery_recipes` |
| shim-integrity | `sha256sum --check --strict $PIN/shims.sha256` |
| coverage | `bash $PIN/coverage-gate.sh` |
| coverage-selftest | `bash $PIN/coverage-gate.sh selftest` |
| design-selftest | `bash $PIN/design-check.sh selftest` |
| delivery-tamper | `bash $PIN/delivery-proof-test.sh` |
| publish-guard | `bash $PIN/publish-guard-test.sh` |
| seal-branch | `bash $PIN/seal-branch-test.sh` |
| licenses | `cargo deny check licenses` |
| dependency-tree | `cargo tree --locked` |
| native-tls | `Fail-closed check of cargo tree --locked -i native-tls; success requires nonzero exit plus expected no-match text` |

`PIN` is the absolute candidate path to `.forge/epic192/lifecycle-evidence/pinned-bundle/self/scripts`. These are byte-preserving copies read from immutable `/home/valentin/source/feedback-loop-ai/swarmail/.forge/epic192/start-now/eight-lanes/baseline/bundles/self`; the external path is outside the workspace box. `proof-source.json` and `pinned-files.sha256` record all original/copied proof identities. Candidate canonical scripts/tooling equal those baseline bytes; no proof, public shim or shim digest was edited. Proofs execute the baseline copies against the candidate, not candidate-controlled dispatch.

The installed `.forge/bin/brokkr` has launcher-recorded SHA-256 `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`, identical to the external launched engine, not merely its version. Its exact path/digest is in `engine.sha256`. Compilation produced self digest `b4ac63031be430f6d235d2a7d2a1b3684bf957f599256a7f81bca3661dcef741` and policy sweep `76 valuations over 14 groups, 0 unruled`; no policy-sweep refusal warning.

Exact coverage: **12,552/12,552 lines, 724/724 branches, 1,396/1,396 functions**. Report: `target/coverage/run.47cIEJ/lcov.info`; summary: `target/coverage/run.47cIEJ/coverage-summary.json` (SHA-256 `800bc9b090bd0ca46bddb103767c614c923df9ad098673989fea18f6846f0364`). Gate and selftest passed without exclusions or a lowered denominator. All coverage/refusal/mutation proof output remains in the named logs.

The full locked dependency tree succeeded. The raw `cargo tree --locked -i native-tls` separately exited **101** with the expected `did not match any packages` diagnostic naming `native-tls`, recorded in `native-tls-raw.{log,exit}`; the expected refusal was accepted by the fail-closed proof, not an arbitrary Cargo failure. Seal proof reports seven cases and ten mutants, keyless.

## Invariants, ownership and residuals

Preserved accepted 0001–0005: synchronous Store insertion/SMTP acceptance, subscribe-before-check waits, oldest-only pruning, real protocol and ingest extraction. Store/persistence code, ordered admission and atomic committed-or-refused outcomes (0023/0024), writer ownership (0042/0047), ten-second HTTP fence and existing runtime shutdown fences (0027/0050), and stop → serve joins → drained → dispatcher joins (0029/0041) remain intact. Interrupted clients may receive no reply despite commit, or no effect after refusal. A surviving Store may retain its writer lock. Detached late work can outlive its tracked caller or miss the dispatcher ledger; embedders must quiesce producers. Gauges promise only their actual measured stages.

Changed code/test files: `src/lib.rs`, `src/main.rs`, `src/api.rs`, SMTP drain comment only in `src/smtp.rs`, new declared `src/lifecycle_tests.rs`, `tests/lifecycle.rs`, and only held-lock signal/drain prose in `tests/binary.rs`. F-owned bridge/stdout/stdio sections are untouched. No shared-helper edit or cross-lane integration request. This receipt and task status are under the reserved spec slug. ADR 0062 is unconsumed: no new semantics, proposal, acceptance or decision-index edit.

Pending external obligations: the engine-owned deterministic verify seat and four fixed independent review lenses, any security hold, CI-only Helm/history/selftest, Docker persistence/context/volume smoke, network RustSec, tag-only release-rate-5k and real port-80 exchange, then operator sealing/landing. None is claimed by these local tests. No push, merge, tag, publication, issue comment, journal fabrication or historical supersede was performed. Canonical active/historical journal paths retain their supplied roles. No operator decision is needed for this compatible implementation. #177/#179/#188/#189 and all other epic stories remain outside this stream.
