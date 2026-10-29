# Intake — port shipped #199 sequential trials onto current main

Run: `epic-192-issue-199-port-the-ship-82604c8b`  
Phase: intake only. Outcome: resolved framing; no implementation or gate-success claim.

## Goal

Port the shipped #199 sequential-trial admission, fresh-HOME proof and honest concurrency contracts onto c13a507 while preserving current main's #198 trap scoring, #225 cleanup probes and #200 provenance/retention behavior.

## Authority and bounded observations

Read `.forge/epic192/issue199/README.md` and `operator-rulings.md` first; all seven entries in that directory's SHA256SUMS verified successfully, including both authority files, port.md and the three patches. Read house rules, decision index, accepted 0055/0071 and 0058, proposed 0063, and shipped 0076/spec acceptance text. No GitHub, CI, journal or sibling-worktree access was used. No missing external fact blocks framing.

The operator's #199 ruling authorizes refusal of concurrent trials, fresh HOME and sequential documentation/0055 text. Namespace isolation belongs to #221. This scoped direction does not accept proposed 0076. Keep 0076, with exactly one proposed index entry; 0077 is already used by #195. The older packet ledger's next-free entry does not override port.md/current index.

HEAD is c13a507, matching port.md. History spot-check includes c13a507 (#200), f2efbb7 (#225) and aec4b46 (#198). `TrialResult` already has required provenance and RESULT_SCHEMA is already 2. `TrialRequest` contains fixture, matrix and executable/agent identity inputs. `RuntimeDirs::create/apply` already supplies private HOME/TMPDIR/XDG maps with env_clear. `matching_trial_row` validates one bounded row and matching labels/status; `run_child_assigned` owns worker runtime. Runner cleanup retains signal observation, actual owned cleanup and post-cleanup scoring. Current proof.rs still has the cooperative `two_concurrent_isolation_trials_both_pass`; it must be replaced, not treated as adversarial isolation evidence.

Candidate `.forge/bin/brokkr` SHA-256 matches `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` and bundles/self/tooling.json. Inspected pinned verify-seat gates and current recipe definitions. Routing is owned by the engine using supplied bundles, with no route edits or intake-selected transition.

## Scope and guarantee boundary

Port the three shipped patches as reviewed behavior, resolving conflicts in the files listed by port.md rather than replacing main's files wholesale. Expected owners: evals/src/{admission,runner,main,lib}.rs, trial-producing eval test helpers and proof/runner/report/boundary suites, evals/README.md and run-plan.md, proposed 0076/index, and the harness/sequential spec/plan/tasks. Adapt additional existing provenance fixtures/retained readers only where adding the required concurrency field requires it.

This work sits outside the SMTP/store guarantees 0001 (accept means queryable), 0002 (subscribe before check) and 0003 (oldest-only pruning). Preserve all three and 0004/0005; no root production behavior changes are authorized. The nearby accepted contract is 0055 §6/§9 and its overlapping proof, narrowed by the explicit operator ruling and recorded with a dated addendum plus proposed 0076. Accepted 0071 remains authoritative for trap precedence and all three business versions at 2. Proposed 0063 remains proposed and its shipped provenance behavior stays intact. Do not restore the old schema-1 reader or add v3, new scoring, dependencies, allowlist changes, delivery routing, namespaces, bearer isolation, retries or model measurements.

## Checkable acceptance criteria

1. **Admission before work.** One cooperating trial per effective uid sharing host /tmp, across library and independent CLI processes/checkouts. Acknowledged active owner causes a contender to return `concurrent_trials_refused`, harness-error row and exit 2 where applicable, before preparation/prompts/trial children. Vary HOME, TMPDIR, cwd and artifact paths without bypassing admission. Unsafe/unavailable rendezvous returns `trial_admission_unavailable` without work; wrong-mode, symlink, nonregular, hard-link and owner cases retain the shipped safety controls. Successful cleanup permits a later oracle pass. Scope is advisory runner cooperation, not confinement of arbitrary same-uid processes.
2. **Explicit requests and matrix accounting.** CLI trial/matrix and library accept default/1; parsed 0/2 produce `concurrency_unsupported`, never silent downgrade. Malformed syntax remains value-free `arguments_invalid`. Parsed trial refusals emit one row; invalid matrix requests emit none with exit 2. Matrix/trial and matrix/matrix controls refuse overlap before child launch. Preserve sequential product order/cardinality, committed prefix, active-row cleanup, zero-time refused suffix, raw statuses, no retries and 2 > 1 > 0 exits. Matrix takes no parent reservation conflicting with its worker. Finite bounded worker-category handling must retain current provenance/context validation, output draining and cleanup-stop behavior.
3. **Ownership through cleanup.** Reservation survives body cancellation, normal/error completion, TERM, INT and a dropped library waiter on a surviving runtime, through actual cessation/reaping, drains, artifact finalization and private-state disposal. Acknowledged cleanup holds keep contenders refused; release permits later admission. Removing the hold makes the causal ordering control fail. Unproved cleanup stops later matrix admission with full suffix accounting. Preserve #225's load-independent event/Pending handshake and #194/#200 signal registration, fixed cleanup budgets, source/build helper cancellation and one cleanup owner; do not replace them with sleeps, reduced assertions or a second supervisor.
4. **Fresh HOME.** Two sequential real helper agents record distinct writable generated HOME paths, neither caller HOME; a first-trial sentinel is absent in the second. Reuse RuntimeDirs and retain per-agent private directory lifetime, closed environment maps and #196 canary/native-argv/artifact controls. Multiple agents inside one inbox-isolation trial remain supported.
5. **Rows compose with current provenance.** Every emitted/retained row carries required integer `concurrency: 1`, including pass/fail, active/setup/retention error, unsupported/overlap refusal, interrupted and never-started suffix rows. It denotes the supported execution limit, not observed overlap or number of agents. Result/catalogue/snapshot stay at version 2; retain complete typed provenance, seed/coordinates, binary/source/agent identity, finite unavailable reasons, immutable retained equality and actual replay/check validation. Shared row validation, report, matrix worker and retained evidence readers refuse absent, noninteger, 0 or >1 concurrency; retain rejection of v1/mixed and pre-provenance v2 rows. Update every constructor/fixture without inferred defaults, schema migration, changed outcomes, failure taxonomy or report arithmetic.
6. **Discriminating real-protocol controls.** Restore shipped `sequential_poacher_control_refuses_competing_trial`: acknowledged idle signup victim, competing launch with inert marker refused before agent execution, causal release, victim fails `gave_up` with no accepted completion, then later oracle passes. A temporary test-only admission bypass must make this control fail and be restored before final gates. No cross-trial exploit client is required. Replace the cooperative overlapping-trial proof; coordinate trial-producing parent test scopes so ordinary default-thread Cargo tests pass. Preserve all sixteen oracle/null controls and within-trial foreign-token/inbox tests. Keep all accepted #198 completed-naive-command/recovered-goal trap controls and replay equality: infrastructure before deadline before earliest named trap before goal, seven traps, v2-only semantics.
7. **Honest linked contracts and proposal.** evals/README.md, local issue-#148 AC4 replacement in harness spec/dated 0055 addendum, and linked local #151 evals/run-plan.md each state sequential baseline, fresh HOME, per-row concurrency and the exact limitation: a same-uid agent can read another concurrent trial's shared-TMPDIR prompt, use its MCP/app/identity/inbox context, complete its task and make its do-nothing victim pass. State advisory scope and hard-death/runtime-teardown cleanup limits; #221 owns namespaces and #201 macOS eval proof. Correct live overlap proof requirements while preserving historical text, harness positive-control AC4, main's #198/#200 additions and decision statuses. Restore 0076/spec/plan/tasks with a dated current-base integration explanation: the old result-version bump is historical because main is already v2 with provenance/traps. A document/link test fails on missing statements, broken links, duplicate/wrong-status 0076 entries or revived live overlap requirements.
8. **Candidate-bound completion.** Commit authorized work unsigned before final verification. All commands below pass for that candidate through current pinned proof/routing; exact root coverage has zero missed lines, branches and functions and passing selftest. Standalone eval gates pass with default test threading; no deleted/weakened #194/#196/#198/#200/#225 assertions. Clean tracked diff contains no unauthorized root source/dependency/allowlist/routing change. Historical ship/review/gate receipts do not prove this port. Only the engine records verify/protected review, and only the operator accepts decisions or seals/merges.

## Verification commands and proof ownership

Run offline with preprovisioned tools; install nothing. Verify the engine bytes before compile. House-rule public commands are listed here for traceability; hermetic verify first checks shims.sha256 and runs pinned bundles/self/scripts copies, as its existing verify-seat.sh requires.

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
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
```

Fail-closed native-tls absence proof (also retain eval dependency-boundary tests):

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  echo 'REFUSAL: native-tls is present' >&2; exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2; exit 1
fi
```

Root gates do not execute the standalone eval crate. Run separately:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
cargo test --manifest-path evals/Cargo.toml --locked sequential_
cargo test --manifest-path evals/Cargo.toml --locked --test runner two_trials_use_fresh_ports_and_identities
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo test --manifest-path evals/Cargo.toml --locked --test runner
cargo test --manifest-path evals/Cargo.toml --locked --test report
cargo test --manifest-path evals/Cargo.toml --locked --test boundary
cargo test --manifest-path evals/Cargo.toml --locked runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults
cargo test --manifest-path evals/Cargo.toml --locked cleanup_handshake_
```

Design maps the shipped sequential_ controls (CLI/library admission, unsafe rendezvous, cleanup lifetime, unsupported requests, matrix overlap, row validation and document links) and current provenance/retained replay suite to AC1–8; no-match filters are not proof. Preserve current completion-fence/source-helper/both-signal controls and full suite rather than importing old fixtures without provenance. The temporary bypass red result must be the violated admission/ordering assertion, not compile, schema or tooling failure. No performance claim requires cargo bench.

CI owns macOS eval evidence through #201, privileged port-80 exchange, Helm/chart baseline and selftests, Docker smoke, network RustSec and tag-only release-rate-5k. The operator owns live GitHub settings, sealing and external issue updates. These are ownership records, not new local acceptance prerequisites. No namespace/network/model-baseline proof is requested. No push, merge, tag, publication, closure or decision acceptance is authorized.

## Intake handoff

Framing is the deliverable of this seat. Only packet/engine integrity and bounded source/history observations were executed here; implementation, full house/eval gate execution and protected review remain outstanding for their designated phases. Commit this framing unsigned and return resolved only for recorded actionable intake. The engine alone chooses the next phase.
