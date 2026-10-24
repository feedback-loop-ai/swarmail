# Delivery gates that prove their verdicts

Epic #120, batch 5: #123, #124, #125, #140 and #143. Design baseline:
`18374cc`, containing batches 1–4 and descending from review baseline
`2a325c9`. The design was committed in `d1f9c43` and implemented in
`d1dc551`; this resume confirms the design, not feature completion.

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

A successful delivery verdict must establish its advertised facts. At the
review baseline,
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

## Clarification — 2026-10-19 resume

Historical entry for the first resume; the second-resume entry below controls
current identity and comparator facts. Its acceptance obligations still stand.

Run `resume-point-read-first-epic-120-2c563256` retains this slug, AC1–AC13
and proposed 0050. The operator reports that the previously staged tree was
committed unchanged after signing returned. Existing signed history stays;
the resume adds a signed confirmation commit touching only this spec,
plan and tasks. No new semantic ruling or successor decision is needed.
The original panel rulings below remain the design history; the following
rulings reconcile both current resume positions explicitly.

| Resume question | Ruling and cost |
|---|---|
| Confirm or redesign | Adopt both positions' confirmation of the existing design and single proof owner. Reject a new framework, spec directory, dependency or ADR. Cost: all existing acceptance obligations remain, including missing execution evidence. |
| HEAD identity | Adopt both positions' fresh signed confirmation commit. Inspection confirms `d1dc551` touches no spec directory, resolving robustness's conditional observation. Reject branch-history discovery or validator changes. Cost: a small additional commit on resume. |
| Engine identity | Adopt robustness's operator-established build digest precondition and simplicity's rejection of an arbitrary PATH checksum as provenance. `bundles/self/tooling.json` has SHA/version but no `binary_sha256`; verify must refuse until trusted build evidence binds the launched binary to the recorded revision. The operator records that identity in a signed commit; this design seat does not supply it. Cost: verification cannot complete on the present tooling record, even if component gates pass. |
| Actual verify proof | Adopt simplicity's distinction between component and seat evidence. Reject robustness's assertion that the existing delivery harness already supplies the actual-seat proof: inspection shows separate integrity and coverage invocations, without invoking `verify-seat.sh`. Cost: AC4 still needs actual dispatched red and covered-control executions; a minimal harness repair is allowed if required. |
| Expansion allowance | Adopt simplicity's targeted probe of the comparator's global `true` to `false` substitution. Existing AC1 and plan checkpoint 3 do not authorize arbitrary configuration-dependent boolean changes. Reject assuming the debug-assert comment proves the allowance safe. Cost: a real compiler negative control and, if it passes incorrectly, a narrow repair before AC1 can close. |
| Scope of proof | Adopt both positions' named limits: structural design validation, expanded-source comparison and unwinding cleanup. Reject using those limits to waive required dispatch, evaluator or mutation tests. Cost: unexecuted proofs remain open; no new runtime shutdown or general attestation architecture is introduced. |

For this resume, a design confirmation satisfies AC6's artifact identity only
after the deterministic validator checks the committed HEAD. It does not
establish AC4/AC5/AC7/AC8 or replace AC13's final gates. An absent engine
digest remains a named verification refusal, never permission to skip or
weaken that check. Implementation changes only demonstrated acceptance gaps,
in new signed commits; operator acceptance of 0050 remains separate.

## Clarification — 2026-10-19 second resume

Run `resume-point-2-read-first-batch--206e588f` confirms the same feature,
AC1–AC13 and proposed 0050 on `epic120-delivery-constitution` at `83107fb`.
Retain `d1f9c43`, `d1dc551`, `d37533c`, `93992af` and `83107fb` unchanged.
The chief appends one signed commit containing only this spec, plan and tasks;
no new semantic ruling or decision is introduced. The previous dated entry
records what was outstanding then, not a request to repeat completed repairs.

| Second-resume question | Ruling and cost |
|---|---|
| Confirm or reconstruct | Adopt both positions' confirmation and one canonical proof owner. Reject rebuilding delivered code from unchecked task boxes, a new slug, framework, dependency or ADR. Cost: reconcile receipts and preserve every outstanding acceptance test. |
| HEAD identity | Adopt both positions' fresh signed artifact commit: `83107fb` changes tooling only, so it cannot identify the design under the existing validator. Reject discovery fallbacks or changes to the pinned validator. Cost: one more confirmation commit; no unsigned fallback. |
| Engine identity and provenance | Adopt both positions' distinction between authorized executable bytes and source provenance. `83107fb` records `binary_sha256` as `38c698a4648f6c7032f33b25bbcddb635377b9450abb0e97fe04aade1fbf95d1`; `binary_provenance` expressly says this operator build is not a build of the recorded `sha`. The missing-record blocker is resolved. Reject claiming source/build equivalence or replacing the record with an arbitrary PATH checksum. Cost: the seat's digest-bound build and CI's source-pin build each still need warning-free compilation; that agreement alone does not prove all engine behavior equivalent. |
| Expansion comparator | Adopt both positions' recognition of the delivered `93992af` repair: no-coverage comparison is byte-exact; the release allowance is the narrow equal-length `if true {` to `if false {` line change. Retain `v4-coverage-bool`, `v4b-debug-bool` and the debug-assert green twin. Reject another speculative rewrite or a wider allowance. Cost: final-tree selftest evidence remains required. |
| Comparator limit | Adopt robustness's explicit limit: a hand-written `if cfg!(debug_assertions) { ... }` can produce the same admitted release shape. No such source expression is currently present. This is expanded-source comparison with the existing compiler carve-out, not behavioral or optimized-code equivalence. Defer the proposed source-token tripwire. Cost: future use of that shape needs review; no stronger guarantee is claimed. |
| Actual verify proof | Rule for simplicity and the existing AC4. Reject robustness's proposal to treat separate harness diagnostics plus a clean dispatched seat as proof that the actual seat records both failures on the tampered, uncovered candidate. Those executions establish different facts. Cost: the seat-invoking red fixture remains required in this batch; reconcile existing receipts first, then add a minimal signed harness repair only if evidence is missing. It is not waived as a post-shipment follow-up. |
| Remaining scope | Adopt both positions' preservation of policy predicates, strict design identity, single-package coverage and #143 test owners. Reject bypass flags, pin changes made merely to pass, and unrelated runtime work. Cost: a blocked or unrun proof remains open even when prior local gates were green. |

The operator's next authorized binary replacement must update its digest and
honest provenance together in a signed edit; mismatch continues to refuse.
This design seat neither supplies a replacement nor changes the tooling record.
The comparator's prior mutant results are recorded in `93992af`; recognizing
that repair is not a claim that this seat reran those tests. Actual launch
identity, red/green seat dispatch, evaluator results and final gates remain
execution obligations. No feature-completion or shipping verdict follows from
this design confirmation; only the operator can accept 0050.

## Clarification — 2026-10-19 third resume (controlling)

Run `resume-point-3-read-first-batch--e444e793` confirms `delivery-gates`
on `epic120-delivery-constitution` at `ad02e3c`. The operator's ruling in
chat on 2026-10-19 authorizes amending proposed 0050 §4 in place to the
engine-forced shape already implemented by `ad02e3c`. The implementation
seat makes that amendment in a new signed commit; this chief commits the
three design artifacts. No successor decision is needed and 0050 remains
proposed. Earlier dated entries and panel rulings remain historical evidence;
this entry supersedes their instruction to keep 0050 unchanged.

Both current panel positions and
`.brokkr/state/epic120/resume-point-3-intake-framing.md` were read. The
commissions under `.brokkr/state/tasks/` concern older run12 work. The
following rulings are self-contained and control this documentation resume.

| Third-resume question | Ruling and cost |
|---|---|
| Scope | Adopt both positions' four-document boundary: this spec, plan, tasks and proposed 0050. Reject new files, dependencies, tests, policy changes and rebuilding earlier batch repairs. Cost: existing evidence obligations remain open until reconciled. |
| Rule shape | Adopt both positions' ABOVE-MEDIUM → OK → UNSAFE sequence and the operator ruling. Reject a separate security stop followed by an unknown-only fallback: the recorded engine refuses the latter as dead because it fires on no present valuation. Cost: engine reachability constrains the shape; a future split needs a new ruling and proof. |
| Diagnostic precision | Adopt robustness's dated marker in 0050 §4, named pin test and preserved history, within simplicity's existing owners. The final rule ID covers both security and unknown facts; inspect recorded typed inputs to distinguish them. Cost: the ID alone cannot explain which cause applied. No new diagnostics or schema are required. |
| Truth-table precedence | Adopt robustness's malformed-fact cases but correct its claim that absent security with any severity always selects UNSAFE. A valid above-medium severity selects ABOVE-MEDIUM first even if security is absent, null or wrong-typed. Remaining unsafe/unknown rows select UNSAFE. Cost: proof must assert first-match rule identity as well as hard-stop outcome. |
| Commit count and ownership | Adopt simplicity's signed implementation amendment, but reject intake AC-7's exactly-one-total-commit count: the chief contract requires a signed HEAD artifact commit, and the commission assigns the decision amendment to implementation in a later signed commit. Require one chief confirmation plus one implementation amendment, without rewriting either. Cost: two commits after ad02e3c; the combined diff still contains exactly four documents. |
| Evidence | Adopt both positions' unchanged pin, diff scope and warning-free compile checks. Reject treating the compiler sweep or prior green gates as the full evaluator/dispatch proof. Cost: prior receipts must be reconciled and remaining AC7/AC8 execution completed before shipping. |

The previous implementation receipt reports actual verify red/green evidence
and warning-free compilation after the fix, but explicitly leaves the full
residual evaluator truth table and actual design dispatch/retry open. This
confirmation neither reruns nor discharges those obligations.

### Third-resume acceptance

- **D1:** Implementation amends only 0050 §4, retains proposed status, and
  includes an in-section marker citing the 2026-10-19 operator ruling in chat,
  the engine's dead-rule refusal, the three-rule order, explicit allow
  predicates, unconditional fallback and no coercion/default/inference.
  Name the existing pin test specified in AC8. No accepted history is edited.
- **D2:** The residual requirement, AC8, plan §4 and task T08 agree with
  amended 0050. Preserve all dated entries and the original panel table;
  historical rule names may remain, but no current instruction restores the
  unknown-only rule. Review these four documents, not a repository-wide purge.
- **D3:** The combined `git diff ad02e3c..HEAD --name-only` after implementation
  contains exactly `docs/decisions/0050-delivery-gates.md` and
  `specs/delivery-gates/{spec,plan,tasks}.md`. The existing policy and pin test
  are byte-identical to `ad02e3c`; no behavior change is authorized.
- **D4:** The unchanged delivery-constitution suite passes, the recorded
  engine compiles without residual-policy warnings, and the full final-tree
  house gates and existing AC1–AC13 evidence obligations are satisfied before
  completion. Blocked or unrun proof is recorded separately from passed proof.
- **D5:** The chief and implementation commits both verify as GPG-signed;
  the branch remains checked out and clean, with append-only history and no
  push, PR, merge or issue closure. Design validation checks chief's HEAD
  before implementation advances; no validator or discovery fallback changes.

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
`max_residual_severity_at_most: "medium"`. Per the operator ruling of
2026-10-19, keep specific stops first where the engine admits them: the
committed order is `REVIEW-RESIDUAL-ABOVE-MEDIUM` (hard stop),
`REVIEW-RESIDUAL-OK` (the explicit conjunction permits ship), then
`REVIEW-RESIDUAL-UNSAFE` (one final unconditional hard stop, no `when`).
The fallback catches remaining security residuals and absent/null/malformed
facts alike. No coercion, defaults or missing-fact inference. Preserve
`security-hold`, clean-review routes and the protected review boundary.

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
   Include both fields absent and combinations of invalid security with
   valid above-medium severity. Only explicit false plus a valid severity
   at most medium selects `REVIEW-RESIDUAL-OK` → ship. Assert first-match
   order: above-medium severity selects `REVIEW-RESIDUAL-ABOVE-MEDIUM` →
   stop/hard; all remaining unsafe/unknown rows select unconditional
   `REVIEW-RESIDUAL-UNSAFE` → stop/hard. Preserve clean-review and
   security-hold behavior, with no coercion, defaults or missing-fact inference
   (operator ruling, 2026-10-19). The unchanged structural pin
   `tests/delivery_constitution.rs::residual_admission_requires_explicit_safe_facts_and_unknowns_stop`
   passes but does not substitute for evaluator execution. Pinned compilation
   exits zero with no residual policy-sweep warning, without suppression.
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

## Recovery reconciliation — 2026-10-24 (NEXT-171)

This note preserves the original AC1–AC13 obligations and dated history. The
run-local receipt index is
[`delivery-proof-residuals/receipts.md`](../delivery-proof-residuals/receipts.md);
its raw, hash-checked records remain under the ignored `.forge/epic192` run
folder.

- **AC8/T08:** the recovered evaluator run records all 105 distinct expected
  outcomes through the recorded resolver and `Machine::evaluate`, including
  exact rule/destination/severity or exact refusal text. Its permissive-policy
  control fails 17 assertions; the restored table passes. The 12 aggregation
  regressions preserve absent, null and malformed facts as unknown, and the
  pinned self compile exits cleanly. T08 is reconciled as complete by those
  receipts and the targeted structural test.
- **Live self and R04:** remain open. This implementer seat cannot launch the
  nested providers or create a canonical Brokkr journal. The reported
  `epic-192-issue-171-live-self-bun-c69c9fa3` stopped before design after DSH
  transport failures and is not a qualifying design/refusal/retry/pass run.
  No actual pinned verify-seat combined red receipt or clean live control was
  produced here. The canonical script edits in 0072 also remain unperformed
  pending current ownership reconciliation.
- **D04:** remains open because its proof obligation spans all original
  AC1–AC13, beyond the evaluator slice. No broader delivery completion follows
  from T08 or the issue-specific local receipts.

Current work follows accepted 0050, unsigned commits under accepted 0056, and
the engine route under accepted 0057. Proposed 0072 remains proposed; the
index entry is not acceptance. No runtime, wire, dependency, license or
coverage-floor contract changed.
