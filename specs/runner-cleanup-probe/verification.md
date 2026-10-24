# Issue #225 implementation proof

The runner cleanup regression uses a private per-probe Unix stream.
Owner-local test code retains entry permission, cleanup release and return
acknowledgement with watch channels; the parent retains events in a channel.
The named regression no longer polls hold markers or uses timed non-exit/exit
as its ordering verdict. Existing sibling probes retain their marker interface.

Pending is reported distinctly from the preliminary real join poll and the
continuation actually awaiting that join. Actual run_trial return is
published before a separate acknowledgement gate keeps the runtime alive for
inspection. The same completion predicate rejects live owned work in bypass
cases and requires cessation in positive cases. Final child exit, direct-child
reaping and Probe::finish remain mandatory.

All executable edits are in evals/src/runner.rs under existing or new
cfg(test) boundaries. The new owner-local seam is Linux-only, like the
existing inline probe regression. No dependencies, features, coverage
exclusions, production cleanup changes or bundle routing changes were added.

## Authority and baseline

Both .forge/epic192/issue225/README.md and operator-rulings.md pass
sha256sum -c SHA256SUMS from their directory. The candidate-local engine
.forge/bin/brokkr hashes to the required
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.
The packet has no prior-run entries. Existing intake evidence, including
the intermittent matrix-stdout backpressure failure and successful retry,
remains in framing.md and intake-verification.json; this repair does not
claim to fix that unrelated failure.

At starting HEAD e3d2e65, runner source equals the plan's baseline
f873523:evals/src/runner.rs byte for byte. The exact baseline filter passed
one test (33 filtered out). Logs and comparison inputs are in target/issue225/.

## Causal matrix and delay

With CARGO_NET_OFFLINE=true, run:

    cargo test --manifest-path evals/Cargo.toml --locked --lib runner::test_probes::runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults -- --exact --nocapture

The named test itself runs the entire matrix twice:

| Case | Ordinary | Delayed |
| --- | --- | --- |
| SIGTERM positive | held + sampled/awaited Pending, release, return, clean finish | same, after pre-entry/permit-entry |
| SIGINT positive | same | same |
| panic positive | same | same |
| server-death positive | same | same |
| SIGTERM bypass | sampled Pending, held, actual return with live work rejected, checked rescue | same, after pre-entry/permit-entry |
| SIGINT bypass | same | same |

Delayed mode uses child-local EVAL_TEST_CLEANUP_DELAY=1. An owner reports
pre-entry(pid) and cannot enter until the controller sends permit-entry.
This is an enforced scheduler-independent delay with no minimum duration.
Permission and release apply to later owners too. Logs include every
case/mode and checkpoint/permission. A 120-second IPC watchdog diagnoses a
hang only; no timeout establishes correct waiting or mutant detection.

The initial handshake filter passed all twelve cases, one test total.
The full inline probe suite passed 14 tests, including installed-seam
checks for multiple/late holders, retained release/events, disconnection,
malformed events, and injected-failure teardown. Teardown releases all
three gates before existing identity-safe rescue and checks the probe is
reaped and its isolated test entrypoint finished successfully.

One earlier command was cut off by the workspace tool's 30-second command
limit (exit 124); it is not proof. The completed invocation used a larger
tool execution allowance. Initial development clippy exposed two missing
fields in existing Probe initializers; those were fixed before proof runs.

## Mutation and restoration

Temporary mutation in test-only Groups::cleanup_pids, after the preliminary
Pending observation and existing bypass control, before the real await:

    +                    #[cfg(target_os = "linux")]
    +                    if cleanup_probe::CURRENT.get().is_some() {
    +                        return Ok(());
    +                    }
                         let mut recorded = false;
                         std::future::poll_fn(|cx| {

The exact filter above ran one test and exited **101**:
0 passed; 1 failed; 36 filtered out. The positive SIGTERM case failed at
the semantic assertion:

    runner returned before the awaited cleanup fence: live owned work

Test runtime was 0.33 seconds on the committed candidate; compilation succeeded. No EOF or watchdog
expiry supplied the verdict. The candidate starts independent cleanup owners
before spawning their test waiters, so bypass cannot discard a waiter
before its owned hold has even been scheduled.

The mutation was restored with an EXIT trap from a saved source copy.
target/issue225/mutant.patch, mutant-filter.log and mutant-filter.exit
retain the precise patch, output and exit. The identical restored filter
passed one test with all twelve cases (4.26 seconds); see restored-filter.log.
The teardown test also checks the intentional panic's exact payload, so
an unrelated panic cannot satisfy it. After that refinement was committed,
the mutant and restored pass were repeated on the exact committed source;
the restored source is byte-identical to that commit.

## Production equivalence

Before editing, and on the candidate, run identically:

    CARGO_NET_OFFLINE=true RUSTC_BOOTSTRAP=1 cargo rustc --manifest-path evals/Cargo.toml --lib --locked -- -Zunpretty=expanded

Compiler: rustc 1.98.0 (88d9e12ae), LLVM 22.1.8,
x86_64-unknown-linux-gnu, default dev profile/features, without --test.
RUSTC_BOOTSTRAP permits compiler expansion; no toolchain was installed.
Compare the **whole canonical non-test library expansion**, including the
runner, with cmp; no source stripping or metadata normalization.

Baseline and candidate expansion are byte-identical, SHA-256:
615d358b04d7398aae8d01311874e0600f6669d55eb82f71627b76d87a04c4fe.
Files: target/issue225/{baseline,candidate}-expanded.rs; compiler identity
and stderr are retained alongside them. Diff review confirms every executable
hunk is test-only; shipping cleanup loops and budgets are unchanged.

## Final candidate gates

All final gates ran offline on implementation commit
5e49874c8d86931fc328f86a38bfbf5a868c573a. Full command, exit, duration,
log hash, all twelve case/checkpoint records and comparison facts are in
[implementation-verification.json](implementation-verification.json).
The follow-up commit changes only this proof, its receipt and task status.

| Required check | Result |
| --- | --- |
| evals fmt and locked all-targets clippy with -D warnings | pass |
| evals full tests | initial exit 101 at the intake-recorded matrix stdout backpressure test; complete retry with --no-fail-fast exit 0 |
| Exact regression filter | one test passes, all twelve ordinary/delayed cases |
| Engine digest and fixed shim integrity | pass |
| Root fmt, locked all-targets clippy and locked workspace tests | pass |
| Candidate-local pinned brokkr compile --bundle bundles/self | pass, no policy-sweep warning |
| Exact coverage | 13,878/13,878 lines; 782/782 branches; 1,508/1,508 functions |
| Coverage selftest | pass |
| Pinned design selftest and delivery tamper-boundary proof | pass |
| Publish guard proof and seal-branch keyless proof | pass |
| cargo deny check licenses and fail-closed native-tls absence | pass |

The first final full evals invocation reproduced the same baseline
matrix_stdout_backpressure_is_bounded_and_still_observes_signals failure
at main.rs:2312 (NotFound), after all 37 library tests passed. Its output
remains in target/issue225/evals-test.log. The explicit complete retry was:

    CARGO_NET_OFFLINE=true cargo test --manifest-path evals/Cargo.toml --workspace --locked --no-fail-fast

It exited zero: 37 library tests, 13 binary tests and all six integration
targets (2 + 14 + 5 + 14 + 32 + 13), 130 tests total, including the
previously failing test. No source change or silent retry hides the first
failure; the intermittent baseline issue remains outside #225.

Coverage evidence is target/coverage/run.W6xtfk/coverage-summary.json and
lcov.info, with zero missed lines, branches or functions. Public scripts
were used after validating their pinned shim bytes; the additional design
and delivery proofs ran directly from bundles/self/scripts. Root coverage
excludes standalone evals under accepted 0055 and does not prove coverage
of these inline eval tests.

The final production expansion comparison still passes byte for byte with
the SHA-256 above. After all gates, the mutant/restored proof was repeated
on the committed source and restoration checked with cmp and git diff;
no executable candidate bytes changed.

CI owns chart/history/selftest, real Docker smoke, network RustSec audit,
tag-only rate proof and port-80 exchange. The operator owns sealing, hosted
settings and publication; evals macOS evidence belongs to #201 CI. No remote
or operator action is claimed.
