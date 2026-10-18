# Delivery gates that prove their verdicts

Epic #120, batch 5: #123, #124, #125, #140 and #143. Design baseline:
`18374cc`, containing batches 1–4 and descending from review baseline
`2a325c9`. This is a design commission, not evidence of implementation.

Decision: 0050 — [proposed delivery-gates ruling](../../docs/decisions/0050-delivery-gates.md).
Only the operator accepts decisions. Existing authorities are accepted
[0007](../../docs/decisions/0007-hermetic-gates-single-model.md),
[0009](../../docs/decisions/0009-agent-council-hybrid.md),
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0027](../../docs/decisions/0027-living-listeners-bounded-sessions.md), and
proposed [0035](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md)
and [0045](../../docs/decisions/0045-delivery-repairs.md). Their status is
unchanged by implementation. 0050 records the additional refusal rules
explicitly; it does not authorize weakening an accepted guarantee.

## Feature and rationale

A successful delivery verdict must establish its advertised facts. Today
verify executes mutable worktree proof scripts; design can advance from
chief's `drafted` without executing its validator; residual review can ship
without safety facts. Exact coverage misses several demonstrated production
shapes, and several tests cannot detect the regressions they claim to guard.
The unsupported closure claims in #105/#119 make actual execution, including
negative controls, the completion criterion for this batch.

Repair the existing coverage gate first, move its complete executable proof
chain into the pinned bundle, wire deterministic design validation, close
residual admission, and strengthen the named tests. Retain one canonical
implementation per proof. Public script entry points remain compatible.

Inputs read for this synthesis: both run-local panel positions;
`.brokkr/state/epic120/{120,123,124,125,140,143}.md`, including comments;
`batch5-intake-framing.md`; and the three historical commissions under
`.brokkr/state/tasks/`. Those commissions explain the rejected Bash
recognizer and existing by-effect architecture; their old landing commands
do not supersede this run. The committed requirements below stand without
access to those ignored files.

## Panel rulings

| Question | Ruling and explicit cost |
|---|---|
| Overall scope | Adopt both positions' existing-gate approach and #140-before-#123 order. Reject a new gate framework, engine fork, dependency expansion and Bash Rust lexer. Cost: the existing compiler and engine interfaces constrain implementation. |
| Production configurations | Adopt simplicity's measured/no-coverage/release expansion comparison. Reject changing the measured profile to disable debug assertions; retain what current tests execute. Reject robustness's debug flag shortcut as the sole release comparison. Cost: release dependencies may need another build; this still proves expanded-source equivalence, not optimized-code equivalence. |
| Local packages | Choose simplicity's single-application-package harvest with explicit refusal of unharvested local production dependencies and extra application workspace packages. Reject general recursive harvesting. Root identity is canonical `manifest_path`, never metadata order. Cost: future local modularization must extend the gate before admission; standalone scanner tooling is outside the application workspace. |
| Wrapper tampering | Adopt simplicity's bounded diagnostic continuation: record integrity refusal and still collect the pinned coverage refusal for the uncovered-line fixture. Reject robustness's proposal that a covered candidate with an `exit 1` wrapper should pass: required integrity checks make it fail. Cost: tampered wrappers can cause refusal, but cannot replace the coverage algorithm or grant passage. |
| Design identity | Choose simplicity's unique `specs/<slug>/` touched by chief's HEAD commit, as 0045 specifies. Reject robustness's merge-base range selection, which can revive an old design. Cost: unrelated intervening commits and split design commits require a new chief commit. No mtime fallback. |
| Phase-level drafted and unknown residuals | Adopt robustness's explicit hard stops instead of leaving these results unmatched. Keep safe residual admission conditional. Cost: malformed facts require correction and re-entry rather than an engine-dependent default. No normalization or inferred safety. |
| Decision validation | Keep newly declared semantic decisions proposed, as accepted 0009 requires. Reject blanket acceptance of either status for new declarations. Ordinary links to accepted authorities are references, not declarations. Contract-doc tests compare actual file/index statuses, so later operator acceptance is supported. Cost: reusing an accepted proposal as a new declaration is refused; cite it as authority instead. |
| Engine proof | Adopt simplicity's real dispatch/evaluator requirement. Reject robustness's structural-only fallback as completion evidence. A linked pinned-core harness is acceptable if no evaluation CLI exists; an unexecuted harness is outstanding work. Cost: engine access may block proof even when JSON compiles. |
| Runtime observability | Use existing metrics, ready-line addresses and test-only owner seams. Reject new runtime metrics, idle CLI flags and socket activation. Both panel positions assumed or proposed surfaces the tree does not need. Cost: private ordering proofs may belong in an owner harness rather than `tests/`. |
| Port handoff wording | Replace release-then-rebind with child-owned port 0 plus existing bound-address output, and held occupied-port refusal cases. A parent cannot retain an exclusive bind until an independent child binds that same port. This satisfies the issue's race-free, env-sensitive proof without claiming descriptor handoff. Cost: separate successful and occupied-port cases instead of one chosen-port success case. |
| Decision process | Unlike both positions' preference for no new record, draft 0050 because dependency admission, config comparison and explicit policy destinations refine rulings. Cost: a proposed record to review; no operator acceptance is inferred. |

## Requirements

The pinned proof root and candidate workspace are separate inputs. Verify
and design execute local proof code only from dispatched pinned resources,
including scanner source/manifests/lock and prerequisite support. Wrapper
templates/digests are pinned too. Candidate sources, manifests, specs,
workflows and chart files remain the material assessed. Compiler, Cargo
cache and arbitrary build-configuration provenance are outside this claim.

The design sequence ends `positions -> chief -> check`; chief returns only
`drafted`, check returns `designed|fail`, and its result governs advancement.
Use the recorded engine's bounded explicit validation phase only if actual
dispatch proves the sequence grammar cannot do this. Preserve one retry
then stop, and prove failure counting rather than assuming it from JSON.

Residual admission requires the conjunction
`has_security_residual: false` and
`max_residual_severity_at_most: "medium"`. Preserve specific security and
severity hard stops and append an unknown-input stop after the allow rule.
Do not change clean-review or protected-review semantics.

The gate continues to count exact integers, capture production dep-info
before expansion, and scan canonical compiler tokens. Configuration
differences, ambiguous identity, unsupported local production packages,
failed/empty expansion and forbidden attributes refuse. Keep the existing
compiler-derived carve-out narrow; no source lexer or general cfg solver.

Tests must observe the operation whose contract they assert. In particular,
a raw socket connected before stop is not proof its head reached a handler.
Coordinate an actual handler request inside the drain and require a complete
503 stopping reply there. Preserve 0027's lawful closure of connections
outside that window and the independent stalled-body deadline proof.

## Acceptance Criteria

1. **AC1 — #140 configuration coverage.** Real compiler selftest fixtures
   `v1-cfg-not-coverage` and `v2-release-only` refuse. Each root production
   lib/bin is expanded under the measured flags, without the coverage cfg
   pair, and under the shipped release profile without that pair. Any
   difference refuses naming the unit/configuration; failed or empty dumps
   refuse. Clean equivalent controls pass; no measured profile is weakened.
2. **AC2 — #140 package coverage.** Select exactly one root by canonical
   root `Cargo.toml` manifest path. `v3-dep-benches`, `v3b-dep-neutral` and
   `v5b-ws-member-dep` refuse through selftest, including the original
   hidden-production fixture with a member `aaa` sorting first. Refuse
   unharvested local production dependency closure and additional application
   workspace packages, including paths outside the repository. A standalone
   nondependency scanner crate and the ordinary root remain accepted.
3. **AC3 — #140 exclusions.** Expand/scan the library test unit as well as
   production units; `v6-test-helper-off` refuses. A real compiler fixture
   with only an inner exclusion turns selftest red when inner-attribute
   handling is removed; its inner-allow twin passes. A `pub fn` exclusion
   diagnostic names the function. Existing A–G, hidden-directory,
   declaration and count checks survive, and the real tree remains exact
   100/100/100. House rules distinguish structural placement from by-effect
   refusals without editing accepted 0020.
4. **AC4 — #123 pinned verification.** Actual dispatched verify runs the
   canonical coverage gate, selftest, publish guard/proof and transitive
   executable helpers from pinned bytes. Modified, missing, no-op and
   substituted public wrappers/helper entry points produce named integrity
   refusals. With an uncovered candidate line and public coverage script
   replaced by `exit 0`, the actual seat records both integrity failure and
   canonical coverage failure and returns `fail`. Early integrity refusal
   alone does not satisfy the uncovered-line proof. With clean wrappers,
   the complete hermetic seat passes on a covered candidate.
5. **AC5 — #123 pin identity.** Verify checks the launched brokkr against
   the dispatched tooling record, refusing mismatch or unprovable identity;
   version alone cannot establish the recorded source SHA. Changing
   canonical proof code changes the compiled bundle digest. An already
   pinned dispatch retains its old proof bytes when the candidate copy is
   changed, including scanner/helper substitutions. Result JSON is valid,
   atomically published, and preserves diagnostics without lossy escaping.
6. **AC6 — #124 artifact identity.** Actual dispatch validates the unique
   spec directory touched by chief's HEAD commit and the pinned dialect
   prerequisites. Require tracked, nonempty spec/plan/tasks at HEAD,
   acceptance heading, checkbox task and risks heading; require matching
   clean workspace artifacts and valid, tracked, clean, explicitly declared
   proposed decisions. Zero/multiple slugs, missing/empty files or headings,
   untracked/dirty artifacts or decisions, missing/bad decisions, missing or
   failing prerequisite support all fail. An old valid spec with newer mtime
   cannot rescue a current missing-risks draft. Both ordinary checkouts and
   linked worktrees enforce the checks; Git errors refuse.
7. **AC7 — #124 advancement.** A valid committed design reaches implement
   only after the exec validator emits `designed`. Chief-only `drafted`
   cannot advance and is a hard stop if leaked as a phase result. The first
   validation failure retries design once and the second stops, demonstrated
   through actual dispatch and the recorded policy evaluator. Validator and
   prerequisites are covered by AC4's pin boundary.
8. **AC8 — #125 residual safety.** Execute the real pinned evaluator over
   each field absent, null and wrong-typed; wrong-case/unknown severities;
   every supported severity crossed with both booleans; and security-hold.
   Only explicit false plus a valid severity at most medium selects the
   residual shipping route. Unsafe/unknown facts stop; existing clean-review
   behavior is preserved. Pinned compilation exits zero with no residual
   policy-sweep warning, without suppression.
9. **AC9 — #143 lifecycle and reads.** A drop-instead-of-503 mutant fails
   the synchronized handler proof unconditionally; EOF/reset never skips
   the assertion. Stop assertions derive from the 10 s server fence with
   only a separately named small deadline-observation allowance; no 30 s
   allowance masquerades as the fence. Do not add process-wall validity
   assertions contrary to 0027. Every read in `tests/common/mod.rs`, including
   helpers added in batches 1–4, has a deadline naming its helper and step.
   Real peers withholding each reply trigger those named failures within
   the bound, with an outer watchdog detecting a removed deadline.
10. **AC10 — #143 documentation/configuration/ports.** Mutating a documented
    runtime constant, including SMTP `DEFAULT_MAX_SESSIONS`, fails a
    contract-doc test. File/index status disagreement fails in either
    direction; accepting 0036/0037/0038/0044 and updating their index rows
    together passes in a disposable fixture. Independent Config idle
    overrides affect the actual intended protocol and leave the other
    unchanged; crossed/ignored wiring mutants fail. Rendered serve help
    pins current Host/Origin policy and both idle defaults. Env bind tests
    use no free-port release window, fail when each env binding is ignored,
    and use ready-line addresses plus held-listener refusal controls.
11. **AC11 — #143 allocations.** A copy inserted in actual `post_json`
    request construction fails the sharing proof. A declared cold-path
    `src/pop3/*_tests.rs` allocation harness observes real RETR construction
    and `payload_len`: adding a second full RETR body or a per-mail sizing
    allocation fails, while normal construction and sizing pass. Exclude
    fixture setup, isolate counters across parallel tests and observe the
    actual execution thread; no RSS/timing estimate substitutes.
12. **AC12 — #143 cleanup and decode.** Remaining scratch fixtures have
    drop guards covering database sidecars. Deliberate assertion-failure
    runs leave no new owned `swarmail-*` artifacts in the actual temporary
    root, assessed against a before snapshot without deleting unrelated
    files or overriding TMPDIR. The search-past-window decoder mutant fails
    a real output or traversal assertion while existing exact extraction
    pins pass. Forced kill/abort is outside an unwinding Drop guarantee.
13. **AC13 — evidence and preservation.** Each repair has a named regression
    red against `2a325c9` behavior and green after repair, also checked against
    batch base `18374cc` where applicable. #143's already-correct behaviors
    use explicitly named broken-tree mutants, not a fictitious baseline
    failure. Record commands, candidate/pin identity, statuses and decisive
    output, restore mutants, and map tests to all five issues in signed
    implementation commit messages. Reconcile #105/#119 claims with those
    receipts. Full house gates and fail-closed native-tls absence must pass
    before feature completion; inaccessible brokkr proof stays outstanding
    for its authorized verify seat, never becomes a static substitute.

## Preserved scope and non-goals

Preserve synchronous commit before acceptance, subscribe before scan,
oldest-only pruning and extraction at ingest; the single-writer lock,
webhook plane, stdio bridge, Origin guard, envelope-sender column, five-second
busy bound, awaits cap and extraction exact-count pins; 0026 startupProbe,
0030 chart knobs and 0019 republish refusal. No new dependencies or license
allowlist changes. No chart release, broad documentation sweep, decision
acceptance, engine upgrade, push, PR, merge or issue closure. Those later
epic stories remain separate. This design adds no persistence or wire
contract and must not reshape server shutdown merely to ease a test.
