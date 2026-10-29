# Intake — Epic #192 / issue #198

Run: epic-192-issue-198-make-eval-tra-75a00ffc
Phase: intake; disposition: resolved; date: 2026-10-25.
Candidate inspected: 32f0018 (accepted 0071); no implementation in this phase.

## Goal

Implement accepted 0071 so each of its seven named eval traps disqualifies a completed trial from frozen event evidence, with honest recovered application goals, version-2 boundaries and completed public-protocol naive controls.

## Authority, evidence and scope

Read .forge/epic192/issue198/README.md and operator-rulings.md first; both passed SHA256SUMS when checked from that directory. Read the house rules, decision index, 0055, accepted 0071, preservation decisions 0073/0075 and specs/eval-trap-scoring/{spec,plan,tasks}.md. The operator ruling and commit 32f0018 resolve prior runs' acceptance block. The 0071 index row already exists and agrees with accepted status. Draft text/tasks about waiting for acceptance or adding a proposed index row are historical instructions superseded by this candidate's acceptance; do not downgrade or re-accept the decision.

0071 is the bounded resolution of the original issue checklist. It does not authorize new refused_actions fields, an unclean-pass metric, invite attempt budgets, destructive decoys, slow/retry redesign, self-caused failed-delivery reclassification or event-log policy changes. Preserve field sets and arithmetic. Non-target tasks (including invite-time, slow-delivery and smtp-retry) retain existing semantics; Failed/Parked deliveries remain infrastructure failures, including when triggered by an agent.

No root production, manifest, lockfile, dependency/allowlist, coverage, gate or recipe changes. Use candidate bundles' routing unchanged. No model measurements, external-source access, journal access, sibling worktree access, push, merge, tag, publication, issue closure or decision acceptance.

## Local spot-check and owners

- evals/src/tasks.rs: score validates first, then currently returns a satisfied goal before rejected actions; its multi-agent foreign-token exception and RESET_REPLAY exclusion must survive.
- evals/src/fixture.rs: Private::claim records a known Decoy's foreign_token before token lookup; follow_link supplies null flow. Decoy HTTP 200 responses and recovered facts are truthful and remain usable. Existing mutex/freeze is the evidence boundary.
- evals/src/state.rs, tasks.rs and runner.rs have snapshot/catalogue/result constants at 1. runner::TrialResult::validate enforces result version; report::parse_rows invokes it.
- evals/tests/{scoring,fixture,proof,runner,report,boundary}.rs and common/mod.rs are the existing test owners. Matrix cancellation tests live in report.rs, not a separate matrix.rs. Preserve #194/#196 behavior and assertion strength.
- Local history includes 8e61ab4 (#194), ef8a3dd (#196), the spec commit 541c065 and acceptance 32f0018. Baseline dc1822a45e4d2ae8326b44baf22818187a8320e9 is present as a local commit; no external evidence is needed.
- Candidate-local .forge/bin/brokkr hashes to ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.

## Frozen guarantees

Adjacent to 0001 (DATA 250 proves stored/queryable mail), 0002 (real MCP waits retain subscribe-before-check) and 0003 (capacity must retain planted alternatives; oldest-only pruning stays unchanged). No semantic change to 0001–0003 or 0004–0005 is authorized. Root runtime remains untouched; real SMTP/MCP evidence, not mocks, proves the eval path.

## Checkable acceptance criteria

1. Authority/index: tests/store_contract_docs.rs passes with exactly one resolving accepted 0071 row. Active eval documentation records the authorized v2 contract with dated amendments and preserves historical evidence and accepted records.
2. Named matching: pure scoring tables cover every row/action below, trap-only, goal-only and both trap/goal orders. Both combined orders fail with the listed class while recovered facts remain true. Earliest matching event sequence wins; an earlier unrelated rejection cannot mask it.
3. Exact boundaries: tests vary rejected status, action, class, flow and claimant independently. Ordinary matches require a provisioned Agent. sibling-addresses accepts an Agent with Signup flow or a known Decoy with any flow, including null. Missing/unknown claimants and structurally valid mismatches are nonmatches; nonmatch-plus-goal passes except for existing isolation. Existing malformed-state validation still refuses before matching.
4. Recovery: deterministic expired-but-not-superseded OTP recovery, ordinary signup/OTP, reset-reuse's required replay and other non-target recoveries retain their prior verdicts. Preserve isolation exception/fallback and all eight failure classes. A nonzero child that genuinely succeeds without traps still passes with its real exit recorded.
5. Completed naive proofs: literal external helper commands use real SMTP-backed MCP mail and fixture HTTP, never run_oracle/private-state solutions. Each healthy negative control exits zero and proves final DATA 250 receipts, exact trap events, the truly recovered goal and the expected failure class. confirm-decoys follows all three distinct links in forward and reverse order, proving both decoys and accepted confirmation. digest-forty opens forty distinct links and proves exactly 39 digest_decoy events plus one digest_selected, independent of randomized target position. Include both sibling claimant paths and malformed-then-correct entity URL.
6. Version/replay: catalogue, snapshot and trial-result constants move together to 2, retaining IDs, tiers, fields, enum values, CLI syntax and raw exits. Every retained healthy completed negative is replayed through the actual check CLI with matching business verdict/class and failure exit. V2 check rejects v1 state with a version diagnostic; report rejects v1 and mixed rows before aggregation. V2 outcome/class counts, Wilson rate denominator and latency arithmetic stay unchanged. No migration, relabeling, dual scorer or pooled history.
7. Nonvacuity/lifecycle: all 16 oracle tasks pass and all 16 null tasks fail with declared idle classes; reset-reuse, foreign-token isolation and two overlapping isolation trials remain proven. Trap-bearing invalid/Failed/Parked state is harness_error; healthy live deadline plus trap is fail/budget; infrastructure wins when both apply. Real concurrent wrong/correct HTTP and freeze tests prove increasing event order, inclusion before freeze and immutable evidence afterward, using request completion/synchronization rather than sleeps.
8. Red/green: retain source-bound commands, helper sources, exits and state/receipts showing actual unwanted business passes on the local specified baseline, then expected candidate failures with true recovered goals. Compilation, version, missing-binary, timeout or infrastructure failures do not count as baseline red. Preserve current #194/#196 base while collecting baseline evidence within this workspace.
9. Scope/delivery: disclose forbidden choices and later-recovery disqualification in all seven affected goals. Reconcile evals README and original eval spec/plan/tasks with dated v2 amendments. No prohibited source/dependency/gate changes; preserve full #194/#196 regression suites. Final implementation is committed unsigned and passes independent evals checks, all pinned house gates, deterministic design check, hermetic verify and protected engine review under existing routing/security hold.

| Task | Rejected action | Flow / attribution | Class |
|---|---|---|---|
| confirm-decoys | decoy_link | Signup, Agent | wrong_link |
| sibling-addresses | foreign_token | Signup, Agent; any/null flow, known Decoy | wrong_message |
| i18n-subject | decoy_message | Signup, Agent | wrong_message |
| stale-otp | superseded_code | Otp, Agent | stale_code |
| magic-login | decoy_link | MagicLogin, Agent | wrong_link |
| entity-url | decoy_link or marketing_mangled | Marketing, Agent | wrong_link |
| digest-forty | digest_decoy | Digest, Agent | wrong_link |

## Verify commands and required evidence

Run from repository root, offline with cached tools; install nothing. Record each command, exit and candidate commit. Focused suites supplement the complete suite; root coverage does not measure the standalone evals crate. No benchmark is required because this is scoring correctness, with no performance claim.

```bash
cargo build --offline --locked --bin swarmail
cargo test --workspace --locked --test store_contract_docs
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked --test scoring
cargo test --manifest-path evals/Cargo.toml --locked --test fixture
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo test --manifest-path evals/Cargo.toml --locked --test runner
cargo test --manifest-path evals/Cargo.toml --locked --test report
cargo test --manifest-path evals/Cargo.toml --locked
# Each completed negative's actual retained v2 state:
evals/target/debug/swarmail-evals check --task TASK_ID --state STATE_JSON
```

TASK_ID/STATE_JSON above must be replaced by each actual control's task/artifact path; this is a replay template, not a claimed execution. The CLI must exit with its declared business-failure status and exact expected class.

The one inventory remains docs/house-rules.md, not this framing. The following records the required execution mapping: integrity-check public shims, run pinned proof copies against the candidate, and use the digest-matched candidate-local engine. The engine owns the actual structural design verdict and review/phase progression.

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash -c 'out=$(cargo tree --locked -i native-tls 2>&1); code=$?; if [ "$code" -ne 0 ] && printf "%s" "$out" | rg -q "did not match any packages"; then exit 0; else printf "%s\n" "$out"; exit 1; fi'
```

The verify seat uses its pinned bundle resources (the source-side paths above identify their current owners); it must not execute candidate public dispatch shims. Coverage requires exact zero missed lines, branches and functions and a passing selftest. A compile policy-sweep warning is a refusal even with exit zero. Missing gate evidence stays outstanding, never excused by intake resolution.

CI owns chart/history/selftest, real Docker persistence/context smoke, network RustSec audit, tag-only release-rate proof and privileged real port-80 exchange. #201 owns macOS eval preparation/toolchain/tests/artifact receipts; operator owns signing, live repository settings and any publication. Namespace isolation is separately tracked under #221. Do not invent local acceptance blockers or claim those receipts.

## Intake receipt

Evidence packet and engine checksums verified; code and local history spot-checked. Framing only: no feature implementation, behavioral red/green, full-gate success, review result or delivery completion is claimed. The verify commands above are obligations for the implementation candidate. No missing workspace input blocks framing.
