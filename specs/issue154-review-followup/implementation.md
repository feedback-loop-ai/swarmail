# Implementation record — issue #154 operator continuation

Date: 2026-10-29. Implements [spec.md](spec.md), [plan.md](plan.md) and
[tasks.md](tasks.md) under proposed
[0097](../../docs/decisions/0097-focused-review-repair.md). This is a re-proof
of a committed tree, not new production work, and it is **not** a claim that
the story is complete: the obligations under "Outstanding" below are open.
`specs/focused-review-repair/implementation.md` stays as history and is not
edited.

## Identities (AC1)

| Name | Value |
| --- | --- |
| Starting candidate | `d7647a6dd3f17d917d8bdf45ca31ff4025c498f8`, tree `ed588b3dd16cb11514f97fa25473ce17c3addd6d` |
| Implementation head (code) | `1979c8e946d5f222d3c81eeb119801826f8f450e`, tree `f9201e2950dcce3e09edbb06535e484a871fa9b9` |
| Design commit (gates first ran here) | `68f57002adf06021237f23da04b0988208a55bde`, tree `4197963e302afb68a8b6695c12cc5851de5ee004` |
| Implement-stop event | `.forge/evidence/issue154-repair-followup-stop/implement-event.json`, SHA-256 `544a04f9f99350ddabfaf44266d4461bdfa81b726616cf3c9e853f547f491ae6` (re-hashed, matches) |
| Previous panel | `.forge/evidence/issue154-focused-review-stop/panel-event.json`, SHA-256 `b1ebc081b3ede7669f1d7de9b8251676b7426c8c25ee38c136fdb8382b39891a` (re-hashed, matches; it ruled on `5dc3bc94…` only, no verdict transfers) |
| Pinned engine | `.forge/bin/brokkr` SHA-256 `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` = `bundles/self/tooling.json` `binary_sha256` (0.12.0, `731189f`) |
| Shims | `sha256sum --check --strict bundles/self/scripts/shims.sha256` exits 0 for all seven entries |

The final candidate HEAD and tree cannot be written into the commit that
carries this file. They are in `target/receipts/<HEAD>/BINDING` and
`SHA256SUMS` (untracked) and in the implement result's `inputs`
(`candidate_head`, `candidate_tree`). A receipt naming any other HEAD is not a
pass.

## What was re-read (T2; AC2, AC3, AC4, AC7, AC10)

No gap was demonstrated, so no production code, test, script, dependency,
decision or historical spec changed: `git diff 1979c8e HEAD -- src tests evals
docs/decisions README.md` is empty, and `git diff --stat 1979c8e HEAD` shows
only the four files of this directory before this record.

- **C1/G1.** `LinkMetadata` restores only through a map-only reader
  (`deserialize_map`, `visit_map`, no `visit_seq`) over the closed
  `deny_unknown_fields` `Members` set; `anchor_text` and `rel` are
  required-nullable; `LinkSource` deserializes only from the strings `text` /
  `html` (`src/model.rs`). Witnesses pass:
  `persist::tests::metadata_null_empty_present_and_corrupt_are_distinct`,
  `tests/persistence.rs::a_corrupt_occurrence_column_refuses_to_serve`.
- **G2.** `mcp_revisions::focused::served_prose_states_additive_discovery`
  passes.
- **C2.** No conflict-marker lines under `evals/`, `src/`, `tests/`; the
  standalone crate builds locked.
- **#152/#153 ports.** `mcp::tests::every_tool_has_exactly_one_object_output_schema`
  and `api::tests::the_router_owns_one_tool_rate_across_connections_revisions_and_members`
  pass.

## Gates (T3, T5; AC5, AC6)

Run from the worktree on the design commit `68f5700` (tree `4197963…`), whose
code equals `1979c8e`; receipts under `target/receipts/68f5700…/`. Every
command exited 0:

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets
  --locked -- -D warnings`
- `cargo test --workspace --locked`: 29 test binaries, 0 failed, including the
  real-server `persistence`, `focused_waits` and `mcp_revisions` suites.
- `.forge/bin/brokkr compile --bundle bundles/self`: "76 valuations over 14
  groups, 0 unruled".
- `scripts/coverage-gate.sh`: exact 100/100/100 (1316/1316 branches,
  1948/1948 functions, zero missed lines); `scripts/coverage-gate.sh selftest`
  pass.
- pinned `design-check.sh selftest`, `delivery-proof-test.sh`,
  `publish-guard-test.sh`, `seal-branch-test.sh` (7 cases, 10 mutants).
- `cargo deny check licenses` ok; `cargo tree -i native-tls --locked` returned
  only the explicit "did not match any packages" outcome (exit 101, the
  accepted absence proof).
- `cargo test --manifest-path evals/Cargo.toml --locked --no-run` builds; the
  spawn-free `--test scoring` passes 18/18.

## Outstanding — retained, not proved (T4; AC5, AC9)

- **Coordinated eval proof: unavailable.** The trial-launching eval
  regressions and `focused_tools_complete_link_and_code_traps_without_guessing`
  were **not run**. No authentic host-wide coordination exists on this host;
  #201 suite-wide coordination is unproved. No `trial_scope`, scratch copy,
  private lock, sequential command or paid trial stands in for it.
- **#152 / #153.** Reviewed upstream heads and current `main` remain
  unavailable; nothing further was imported. The sources and dispositions are
  as recorded on `1979c8e` (`specs/focused-review-repair/implementation.md`).
- **#151.** The operator-owned paid baseline and actual-client proofs are
  missing.
- **CI-owned:** platform, network, chart, Docker and port-80 evidence, and the
  #201 macOS evidence.
- **Operator:** sealing (0056), and the decision-number collisions
  (0090/0091/0092/0096 and the provisional 0094/0097) — nothing accepted,
  renumbered or rewritten.
- **Engine verify and the five protected members** have not yet run on this
  tree; none of their verdicts is claimed or anticipated here.

## Residual carried (AC8)

`has_security_residual: true`, `max_residual_severity: "high"`: the inherited
review-integrity / result-attribution residual — the pinned engine completes
instead of refusing the cross-attempt case (0081 and its must-fail witness).
Carried as a fact, not a clearance or a hold. No attack was reproduced, no
access widened, the engine not re-pinned, no verdict invented, and no claim
that an upstream merge changed the pinned engine. The protected security
member owns its verdict and any hold; the engine owns verify and advancement.
The unbounded per-mail occurrence and snapshot cost stays a documented
availability residual.

## Result

Returned `complete` for the implementation only; the story is open.

## Addendum — 2026-10-30, after the operator's two merges

Dated note; the text above is history and is not edited. It describes the tree
at `1979c8e`. The branch head is now
`c8c90e92b878056d1e6958c277ad8d07a9b5269d` (tree
`2a71fb8ef621e8bf0bc3723d33d9dd3e99b04426`), which adds the operator's merges
of main `9785f81` and main `ad8d6c9` (#281). No `src/` line moved since
`1979c8e`; the merges carry only main's recipe, charter, decision and
delivery-test changes.

- **Layout.** brokkr's files live under `.brokkr/` (decision 0100, accepted):
  `bundles/self/...` above is now `.brokkr/recipes/self/...`, and the pinned
  engine is the one `.brokkr/recipes/self/tooling.json` names, not
  `.forge/bin/brokkr`.
- **Engine pin.** `a9893b4` plus four patches
  (`swarmail-engine-a9893b4-p4`), binary SHA-256 `29dc3309…`. The Pinned-engine
  row above (`ab935558…`, `731189f`) names the earlier pin.
- **Diff claims.** "`git diff 1979c8e HEAD` is empty" for `docs/decisions/`
  held at `1979c8e` and is not literally true at this head: main's decisions
  0098–0102 and the 0081 addendum arrived by merge. The `src/` claim still
  holds.
- **Verify and panel.** The engine's verify pass and the first protected panel
  ran on `c8c90e9`; the panel's verdicts are its own and are not transferred
  here. Gate receipts under `target/receipts/` bind the heads they name only.
- **Residual in AC8.** The predicate "the pinned engine completes instead of
  refusing the cross-attempt case" was the `731189f` engine's. Decision 0081
  records, dated 2026-10-30, the full routing proof re-run with the exact
  binary `29dc3309…` at main `9785f81`: exit 0, every `cross-attempt`,
  `cross-effect`, `missing` and `edit` case parks `awaiting_operator`. For
  this pin the predicate no longer holds, so this record no longer carries
  `has_security_residual: true` / `high` for it. Nothing here is a security
  clearance: the protected security member owns that verdict; no attack was
  reproduced, no access widened, no engine re-pinned by this feature. The
  unbounded per-mail occurrence and snapshot cost stays a documented
  availability residual (0094).
- **Still outstanding, unchanged.** Coordinated standalone eval and #201
  (including macOS), #152/#153 reviewed heads, #151 paid baseline and actual
  clients, CI-owned proofs, operator seal (0056), and the decision-number
  collisions (including the OPTIONS proposal renumbered 0090 → 0101 by an
  earlier merge; `specs/focused-review-repair/` keeps citing 0090 as
  history). `tasks.md` T1–T8 stay unticked; this record stands in for them.
