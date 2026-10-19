# Ordered resume tasks — release safety and truthful records

## Status — 2026-10-19, re-drive 3

The preceding task reconciliation is committed in `3b218b0` above `8478924`.
Both panel positions confirm it. Retain the prior implementation/evidence map
and ordered R01–R07 below; no completed implementation task is reopened.
For this resume, R01 preserves both commits and commits only this run's dated
three-artifact confirmation. Its paired proof remains the exact-path, clean
tree, signature and HEAD-bound design checks. A structural precheck alone
does not complete R01's signature or final sequence obligations.

R04's permitted repair must be a new signed descendant of this confirmation,
preserving `3b218b0` as well as `8478924`. R05 evidence names the actual final
HEAD, not the panel's earlier inspection target. All unchecked tasks remain
proof obligations; historical green logs do not check them off. In
particular, R06 remains the CI owner's actual event-baseline/Helm and Docker
execution, and R07 retains signature verification and operator handoff.

## Status — 2026-10-19, re-drive 2

Implementation is present in signed commit
`847892409134bee082e8a3edcdb0243eb720a361`. This supersedes the original
"These tasks are unexecuted" statement. The first run's local gates were
reported green before signing failed; the operator's assistant committed the
unchanged staged tree. This is prior evidence, not a fresh gate verdict.

The original T01–T18 implementation obligations are reconciled below rather
than reopened. The ordered unchecked tasks now describe this resume's proof
and delivery work. An unchecked proof does not mean its implementation is
missing. [Spec AC1–AC9](spec.md#acceptance-criteria) remain the acceptance
contract; [plan.md](plan.md) retains detailed ownership and proof requirements.
0051 remains proposed; no task grants operator acceptance.

## Prior implementation and evidence map

The following test names and mutation evidence are recorded in `8478924`'s
commit body and exist in the tree. Their fresh execution is due under R02–R06.

| Original tasks | Present work and paired proof | Remaining evidence |
|---|---|---|
| T01 | Existing branch/base and operator ruling recorded; `git merge-base --is-ancestor 2a325c9 8478924` proves ancestry. | R01 checks resume identity, not a new branch cut. |
| T02–T03 (#141) | Guard require-present mode and explicit image dependency; `publish-guard-test.sh` cases 14–16 exercise result/status poles, original v0.1.1 refusal and independent-chart wiring mutants. | Fresh guard/wiring proof. |
| T04–T05 (#141) | Dynamic chart coherence and event-baseline bump checks; `chart-verify.sh selftest` exercises real Git histories, and verify mode renders coherent/mismatched twins. | Fresh selftest; actual event baseline and Helm render in CI. |
| T06–T07 (#141) | Quickstart flags, recursive context exclusions and Docker smoke wired; `the_install_examples_clean_up_and_pull_what_they_name`, `ci_runs_the_chart_history_check_and_the_docker_smoke`, and real Docker lifecycle/context positive controls. | Docker execution remains CI-only and outstanding locally. |
| T08–T10 (#142, D01–D21) | Corrected process/license/runtime/release prose and help; `the_gate_inventory_has_one_home_and_everyone_points_at_it`, `the_readme_names_every_ci_job_and_every_root_suite`, `the_license_allowlist_is_the_one_house_rules_names`, `the_corrected_release_and_runtime_claims_stay_corrected`, plus binary/protocol tests. | Fresh checks and D-register source audit; retain operator handoff for external issue corrections. |
| T11–T12 (#142, D22–D24) | OpenAPI domain/status/subject repairs; API tests `the_published_await_and_assert_domains_match_the_wire`, `every_published_operation_lists_and_answers_the_guard_403`, `each_published_status_is_answered_on_its_operation`, `the_thread_subject_is_the_published_oldest_subject`; inline `the_served_openapi_publishes_the_await_and_assert_domains`. | Fresh real-server/schema proof. |
| T13–T15 (#144) | Authorized statuses, complete index and dated pointers; `every_decision_file_has_exactly_one_resolving_index_row`, `the_inventory_refuses_missing_dangling_duplicate_and_malformed_entries`, `a_status_change_passes_only_when_record_and_index_move_together`, `every_audited_amendment_is_linked_on_both_sides`, `the_pointers_0045_promised_are_required`. | Fresh fixtures/real-tree audit; accepted text remains intact. |
| T16 (#144/#141) | Compatible-proposal rule, partial-release recovery and unpublished chart 0.1.2; `house_rules_state_the_rule_for_code_under_a_proposal` and guard/chart tests. | Fresh proof, with 0051 still proposed and Cargo still 0.1.0. |
| T17 | Prior local gates and per-issue discriminating tests reported by the first run and commit body. | Fresh final-tree inventory, pinned verify, CI-only checks and review remain due. |
| T18 | Signed implementation commit `8478924` contains the issue/test mapping. | Preserve it unchanged; verify signatures and clean handoff for this resume. |

## Ordered work

- [ ] **R01 — Confirm resume identity and commit the reconciled design.**
  Preserve `epic120-release-docs` and `8478924`; retain the existing slug and
  proposed 0051. Commit only spec.md, plan.md and tasks.md, signed, with the
  three issues in the subject. **Test:** ancestry and signature verification,
  exact committed-path audit and clean tree; the pinned
  `bundles/self/scripts/design-check.sh check .` identifies this chief's HEAD,
  passes the required headings/checkboxes and linked proposed decision, and
  invokes the pinned prerequisites. The final sequence check owns advancement.

- [ ] **R02 — Confirm #141 against existing owners (AC1–AC3).**
  Execute the canonical publish-guard proof and chart history selftest; review
  workflow admission and the existing version fixtures against present Cargo/
  appVersion 0.1.0 and chart 0.1.2. **Test:**
  `bash scripts/publish-guard-test.sh`, `bash scripts/chart-verify.sh selftest`
  and the delivery-constitution suite. They must refuse failed/skipped/
  cancelled image prerequisites, missing/unknown images, the original v0.1.1
  independent-chart regression, and invalid/unbumped history, while admitting
  green twins. Actual Helm and Docker execution is R06, never a static pass.

- [ ] **R03 — Confirm #142 and #144 (AC4–AC8).**
  Reconcile D01–D24 against current code and dated release corrections; retain
  all listed limitations and the existing operator handoff. Check index/status
  membership, amendment semantics and 0051's proposed pointers. **Test:**
  `cargo test --workspace --locked --test store_contract_docs`,
  `cargo test --workspace --locked --test api` and the existing binary/inline
  schema tests in the full suite. Use the named prior mutation evidence and
  current negative fixtures; compare accepted ADR bodies with the original
  base, allowing only authorized metadata and dated addenda. No external
  comments or issue closure are part of this task.

- [ ] **R04 — Repair only a demonstrated gap, otherwise make no change.**
  If R02/R03 or verification exposes an unmet issue criterion, repair its
  existing owner with a new signed commit above `8478924`. **Test:** a paired
  regression fails the broken behavior and passes the repair, with existing
  green controls retained and the issue/test mapping in the commit body.
  If no gap exists, record the no-change disposition; no synthetic repair,
  dependency, version bump or new decision is required. Recheck affected
  proofs after any actual repair.

- [ ] **R05 — Produce fresh final-tree local verification (AC9).**
  Run the complete [house inventory](../../docs/house-rules.md#gates--part-of-every-change)
  in the plan's required order, with formatting before checks, locked Cargo
  invocations, exact coverage and selftest, publish-guard proof, license check
  and fail-closed native-tls absence. **Test:** successful outputs bound to
  the assessed tree; pinned brokkr compile and hermetic verify execute in the
  verify seat. Prior green logs do not satisfy this task. Record unavailable
  execution honestly; do not lower coverage or install tools in verify.

- [ ] **R06 — Obtain CI-only proof (AC2–AC3/AC9; CI owner).**
  **Test:** `scripts/chart-verify.sh` against the actual event baseline and
  real Helm renders; `scripts/docker-smoke.sh` proves real SMTP 250 survives
  graceful stop and hard kill under uid 65532, anonymous volumes are removed,
  and recursive target sentinels are excluded with positive controls. Keep
  other CI-only obligations as named in house rules, including tag-only rate
  proof when releasing. This remains outstanding until CI runs it. Missing
  local Helm/Docker does not block the offline phases or authorize a claim of
  CI success; the operator's assistant obtains CI evidence before merging.

- [ ] **R07 — Review and hand off the signed branch.**
  **Test:** the required review panel rules on the final tree, hermetic ship
  checks pass, `git verify-commit` verifies new author commits, and Git shows
  the correct branch with a clean tree and no tracked run-local evidence.
  Preserve append-only history and the operator's exclusive push/PR/merge/
  issue actions. A signing failure stops delivery; it never permits an
  unsigned commit. Report local proof and outstanding CI evidence separately.
