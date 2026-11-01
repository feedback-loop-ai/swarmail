# Ordered tasks — gate the eval harness in CI and the verify seat

Each task is checked only with its paired proof. Nothing is `complete` while
a box is open. No push, no merge, no decision accepted, commits unsigned.

- [ ] **T1 — Baseline the eval gates by hand on the untouched tree (R8).**
  From `8dfdf6b`, run the quickstart's resolution and the three evals
  commands; record exit codes, the resolved path and SHA-256, test counts,
  and cold/warm `time` per command in `implementation.md`. Confirm
  `cargo fetch --locked --offline --manifest-path evals/Cargo.toml` resolves.
  **Test:** all three exit 0; the fetch exits 0 offline.
- [ ] **T2 — Wire CI (R1).** Add `components: rustfmt, clippy`, the cache
  `workspaces`, and the four named steps to the `test` job before the
  port-80 steps.
  **Test:** `tests/delivery_constitution.rs` CI-shape test gains needles for
  `working-directory: evals`, the three evals commands and the resolution
  line, and passes; `grep -c evals .github/workflows/ci.yml` is nonzero;
  `python3 -c 'import yaml,sys; yaml.safe_load(open(".github/workflows/ci.yml"))'`
  parses (or an equivalent local YAML check).
- [ ] **T3 — Wire the verify seat (R2).** Add the header sentence, the three
  `run` lines after root `cargo test`, and the pass-note names.
  **Test:** fake `cargo` in `VerifySeatFixture` emits an artifact record on
  `build` and the fixture creates `evals/`;
  `verify_seat_runs_gates_without_login_and_preserves_failure_evidence`
  asserts the nine-entry gate order; `verify_runs_the_pinned_gates_and_checks_the_public_shims`
  asserts the three labels; `pinned_bundle_shims_and_proof_dispatch_are_digest_bound`
  carries the new seat digest; `brokkr compile --bundle .brokkr/recipes/self`
  exits 0 with no sweep warning; `sha256sum --check --strict
  .brokkr/recipes/self/scripts/shims.sha256` passes.
- [ ] **T4 — House rules inventory (R3).** Add the evals lines and the one
  sentence to § Gates.
  **Test:** `the_gate_inventory_has_one_home_and_everyone_points_at_it`
  with the three new needles passes; `grep -c evals docs/house-rules.md` is
  nonzero.
- [ ] **T5 — 0055 addendum and index row (R4).** Append the 2026-10-31
  addendum; extend the README row's description.
  **Test:** `git diff 8dfdf6b -- docs/decisions/0055-agent-eval-harness.md
  | grep -c '^-[^-]'` is 0; `every_index_row_matches_its_records_status`,
  `every_decision_file_has_exactly_one_resolving_index_row` and
  `every_audited_amendment_is_linked_on_both_sides` pass;
  `sequential_contracts_and_decision_links_are_honest` passes.
- [ ] **T6 — Specs, README, Cargo.toml comment (R5).** Dated clarifications in
  both specs; README gates paragraph, past-tense ownership sentences and the
  #149 path; the manifest comment.
  **Test:** AC7's greps are empty; `grep -c '#201'` is nonzero in each of the
  four boundary-checked files; `cd evals && cargo test --locked --test
  boundary` passes.
- [ ] **T7 — Red proofs in scratch copies (R6).** Bump a shared root lock
  entry; separately change the root MCP text-block emission. Record both
  failures and revert.
  **Test:** transcript in `implementation.md` shows
  `every_evals_package_is_already_in_the_root_lock` failing, and the proof
  suite failing on the MCP mutation; `git status` clean of both mutations.
- [ ] **T8 — Shallow-checkout provenance check (plan risk).** Clone the
  candidate detached at depth 1 and run `cd evals && cargo test --locked
  --test provenance` with `SWARMAIL_BIN` set.
  **Test:** exits 0, or the finding and its `fetch-depth` remedy are
  recorded and applied in T2.
- [ ] **T9 — Every gate, locked (R8, AC10).** Root fmt, clippy, test; bundle
  compile; coverage gate and selftest; design-check selftest;
  delivery-proof test; publish-guard, seal-branch tests; deny; native-tls
  absence; shims digest; the three evals gates against the resolved binary.
  **Test:** each exit code recorded in `implementation.md`; coverage missed
  == 0 in lines, branches and functions.
- [ ] **T10 — PR text (R7, AC8, AC9).** Draft in `implementation.md`: the
  per-gate wall-time table (Linux local cold/warm; CI cells marked "from the
  PR's CI run"), the stdout-residual citation
  (`drain()`, `readiness_continues_draining_after_the_first_valid_announcement`),
  the deny gap, the no-host-lock ruling, and the operator follow-ups (raise
  the timeout only on a measured >~21-minute macOS job).
  **Test:** the draft names every AC and no invented number; commit unsigned.
