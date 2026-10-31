# Tasks — issue #154 operator continuation

Ordered obligations under [spec.md](spec.md) and [plan.md](plan.md). Design
drafting checks none off. Every task names the test or proof that closes it;
a missing proof leaves the task open and is recorded, never substituted.

- [ ] **T1 — Bind identities (AC1).** Record in
  `specs/issue154-review-followup/implementation.md` the starting candidate,
  implementation head, implement-stop digest `544a04f9…91ae6`, panel digest
  `b1ebc081…891a`, engine digest from `tooling.json` and shims check.
  **Proof:** `git rev-parse HEAD HEAD^{tree}` and `sha256sum` outputs match
  the record; `sha256sum --check --strict bundles/self/scripts/shims.sha256`
  exits 0.

- [ ] **T2 — Re-read every finding's witness against the code (AC2, AC3,
  AC4, AC7).** Confirm the map-only/string-only readers in `src/model.rs`, the
  column-19 bridge in `src/persist.rs`, the served-prose test and the
  #152/#153 ports; fix only a demonstrated gap, in its owner, within the
  robustness invariants.
  **Proof:** `cargo test --workspace --locked` passes
  `metadata_null_empty_present_and_corrupt_are_distinct`,
  `a_corrupt_occurrence_column_refuses_to_serve`,
  `served_prose_states_additive_discovery`,
  `every_tool_has_exactly_one_object_output_schema` and
  `the_router_owns_one_tool_rate_across_connections_revisions_and_members`;
  `git diff 1979c8e HEAD -- src/ tests/ docs/decisions/` is empty or shows
  only a recorded, witnessed fix.

- [ ] **T3 — Prove the standalone eval build (AC5).** Run the locked
  `--no-run` build and the spawn-free scoring tests of `evals/`.
  **Proof:** both exit 0 with receipts; no conflict markers under `evals/`,
  `src/`, `tests/`.

- [ ] **T4 — Record the eval execution gap honestly (AC5).** Do not run the
  trial-launching regressions or
  `focused_tools_complete_link_and_code_traps_without_guessing` without
  recorded authentic host-wide coordination; write the gap and #201 as
  unproved in the record.
  **Proof:** record names the missing coordination and
  `coordinated_eval_proof: "unavailable"` appears in the implement result
  `inputs`; no `trial_scope`, scratch-copy or paid-trial receipt is presented
  as evidence.

- [ ] **T5 — Run the full gate inventory on the committed tree (AC6).**
  fmt, locked clippy, locked workspace tests, pinned compile, pinned
  coverage gate plus `selftest`, pinned `design-check.sh selftest`,
  `delivery-proof-test.sh`, `publish-guard-test.sh`, `seal-branch-test.sh`,
  `cargo deny check licenses`, `native-tls` absence.
  **Proof:** every command exits 0; coverage reports zero missed lines,
  branches and functions; receipts under `target/receipts/` with `SHA256SUMS`
  name the HEAD and tree.

- [ ] **T6 — Commit the record unsigned and rebind (AC1, AC6, AC9).** Commit
  `implementation.md` (and any witnessed fix) with a why-body message; if the
  commit changed the tree, rerun T5 so receipts bind to the final HEAD.
  **Proof:** `git status --porcelain` is empty; `git log --format=%G? -1`
  shows `N`; the receipts' recorded HEAD equals `git rev-parse HEAD`.

- [ ] **T7 — Carry the residual and return the typed result (AC8, AC9,
  AC10).** Return `complete` with `inputs` carrying `candidate_head`,
  `candidate_tree`, `local_gates_clean`, `coordinated_eval_proof`,
  `has_security_residual: true`, `max_residual_severity: "high"`; notes name
  AC4/#201, #152/#153 reviewed heads, #151 paid baseline and actual clients,
  CI-owned platform/network/chart/Docker/port-80 and #201 macOS evidence, and
  operator sealing as outstanding. Return `blocked` only for a gate failure
  not repairable within AC7, quoting the output.
  **Proof:** the result file has exactly `result`, `inputs`, `notes`; no
  decision accepted, nothing pushed, merged, published, tagged or closed;
  `git diff 1979c8e HEAD -- docs/decisions/` is empty.

- [ ] **T8 — Engine verify and five protected members (external to the
  implementer).** The verify seat reruns the inventory from the pinned bundle
  on the final candidate; Fable correctness, Flash security and guarantees,
  Spark GLM-5.3-Flash tests and Opus independent rule with their own
  attributed results.
  **Proof:** engine-recorded verify result and five member-attributed panel
  records bound to the final HEAD; the security member alone owns any hold.
