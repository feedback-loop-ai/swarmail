# Intake framing — continue issue #196 from the committed port

Date: 2026-10-24. Run: `epic-192-issue-196-continue-the--4a724525`.
Phase: intake only. Handoff: `529d9160970152cd9b767925be71afdae4c44fe7` (unreviewed).

## Goal

Apply the operator's #201 macOS-proof ownership ruling to the existing eval privacy design, verify the committed #196 port without weakening #194 cancellation or either regression suite, repair only demonstrated defects, and obtain the full accepted-0058 careful review.

## Evidence and authority

Read the local issue README and operator-rulings first. All entries pass
`(cd .forge/epic192/issue196 && sha256sum -c SHA256SUMS)` when run from the
packet directory. The initial checksum invocation from repository root used
the wrong relative base; the corrected invocation passed every entry.

Read docs/house-rules.md, the decision index, accepted 0055 and 0058, proposed
0073 and 0075, the existing privacy specification/tasks and prior intake.
Spot-checks covered RuntimeDirs::apply, private artifact creation/publication,
reqwest no_proxy builders, matrix run_child/capture, and the pinned verify
inventory. Git history confirms 529d916 records the unchanged implementation
handoff on the existing design and matrix-cancellation history. Initial git
status was clean. Prior gate/mutation summaries are supplied evidence, not
fresh verification or review verdicts.

The candidate-local `.forge/bin/brokkr` SHA-256 matches
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
The controlling platform direction is the 2026-10-24 row **macOS evidence for
`evals/`** in [operator-rulings.md](../../docs/rulings/epic-192.md):
`evals/` is not in CI until #201; macOS-only eval proof belongs to #201's CI
gating. This supersedes this issue's stale requirement for local macOS receipts.
It does not remove Linux proof or claim that any CI evidence exists.

## Frozen guarantees and scope

No direct change to 0001 (synchronous accept == stored), 0002 (subscribe before
check), or 0003 (oldest-only pruning) is authorized. The work sits beside
accepted 0055's eval lifecycle, preserving 0001–0005 and #194's implemented
0073 cancellation, cleanup ordering and accounting. Proposed 0075 compatibly
strengthens the eval privacy boundary and stays proposed; 0073 stays proposed.
No decision acceptance or accepted-rule amendment is part of this task.

Primary artifacts are specs/eval-privacy-port/{spec,plan,tasks}.md and proposed
0075's related proof wording. Production/test changes, if a failing criterion
requires them, stay in the existing eval owners and suites, with corresponding
owning documentation. Preserve historical receipts as historical evidence.
Do not replay the already-upstream routing assertion repair or change delivery
policy, root production semantics, dependencies, licenses or coverage policy.

## Checkable acceptance criteria

1. **Platform correction is explicit and consistent.** Replace spec AC13's
   requirement for macOS execution here with a dated citation to the operator
   ruling: evals is outside CI on every OS until #201, which owns macOS eval
   preparation/toolchain, clippy/tests and artifact-primitive proof. Reconcile
   current plan/tasks and proposed 0075 proof wording that otherwise still make
   those receipts a completion blocker for #196, including AC23's platform
   cross-reference. A text comparison must show that every existing Linux
   preparation, custom-target, artifact, mutation and gate obligation remains.
   Do not mark macOS or CI proof as executed or add namespaces, live GitHub,
   network or any other proof unavailable to this seat.
2. **Every other existing criterion is verified.** Retain spec AC1–AC12 and
   AC14–AC26 with their assertions and named plan/test mapping. Candidate-bound
   Linux receipts identify commands, actual exits, candidate/tree and OS. Whole
   environment maps, generated PWD, native argv, explicit preparation selectors,
   all-client no_proxy, safe output/Admin/CLI errors, bounded drains, private
   pre-payload modes and no-clobber publication must pass their existing named
   tests. Review the 11 supplied intended-assertion mutation receipts against
   the committed implementation and AC8/AC19; reproduce any missing, stale or
   insufficient proof and record restored-green results. All mutations must be
   absent before final gates. A historical aggregate pass cannot discharge an
   unproved criterion.
3. **Cancellation and accounting remain discriminating.** Retain every #194
   test and its assertion strength, using the existing baseline/packet mapping
   in plan.md. Execute both signal paths, repeated-signal and admission/commit
   races, Cargo ownership, cleanup Pending hold/release and bypass controls,
   leader/descendant and observation/expiry cases, output/EOF pressure and guard
   teardown cases. Preserve schema 1, committed prefix, post-cleanup active row,
   fresh zero-time refused suffix, no later launch, actual statuses/lifetimes,
   report denominators and failure precedence. Integrated privacy ownership
   tests must prove private runtime directories survive the same cleanup fence.
   Real oracle/null/wrong-action/isolation proofs run through SMTP/HTTP/MCP.
4. **Corrections follow failures.** Record each demonstrated defect with its
   failing criterion/test, make only the necessary repair, and rerun affected
   proofs plus final gates. No deleted/diluted test, runtime test-control
   forwarding, raw-output escape hatch or weakened bound can resolve a failure.
   If no implementation defect is demonstrated, preserve the handoff's code.
5. **All final Linux gates pass independently for both crates.** Run the
   commands below through the pinned proof chain, preserving exact root
   coverage at zero missed lines, branches and functions and its selftest.
   Record nonzero exits as failures; root green cannot substitute for evals
   fmt, locked all-target clippy and the full standalone suite.
6. **Review and close-out obey the engine.** The review phase supplies five
   fresh separate candidate-bound accepted-0058 careful results: Astra
   correctness, official DeepSeek Flash security, Sonnet 5.5 tests, Astra
   guarantees and independent Fable 5.1. Intake/implementation supplies no
   review verdict. Protected review, worst verdict and security hold remain
   binding; no author substitute, combined result, DeepSeek Pro or unauthorized
   fallback. Resolve returns and reverify changed candidates. Commit all work
   unsigned before returning from the responsible phase; never push, merge,
   tag, publish, close an issue or accept a decision.

## Verify gate commands

Intake records these commands; build/test execution belongs to implementation
and hermetic verify. Verify runs offline and installs nothing. Check the engine,
tooling, public shim hashes and pinned resources first. Public inventory script
names below identify the house gates; verify checks those shims and executes
corresponding pinned bundle copies, as required by 0050 and house rules.

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
sha256sum -c bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
```

Use verify-seat.sh's fail-closed native-tls probe: `cargo tree -i native-tls`
must fail specifically with `did not match any packages`; success or any other
error refuses. Bundle compilation must not emit a policy-sweep warning refusal.

Focused Linux proof commands supplement, and do not replace, the full suites:

```bash
cargo test --locked --test store_contract_docs
cargo test --manifest-path evals/Cargo.toml --locked --test runner
cargo test --manifest-path evals/Cargo.toml --locked --test report
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo test --manifest-path evals/Cargo.toml --locked artifact_
cargo test --manifest-path evals/Cargo.toml --locked matrix_completion_fence_is_observed_and_its_negative_control_exits_early
cargo test --manifest-path evals/Cargo.toml --locked runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults
```

Use the existing plan's remaining exact focused commands and mutations to
resolve criterion-level evidence gaps. Retain Linux toolchain/custom-target and
selected artifact-primitive receipts. Charter CI-only chart/history/selftest,
Docker, network RustSec, tag release-rate and privileged port-80 obligations
remain CI-owned; macOS eval proof is separately #201-owned. Do not query CI,
GitHub, journals or sibling worktrees to supply them.

Intake result: resolved framing. No implementation, gate pass, review verdict
or delivery completion is claimed by this artifact.
