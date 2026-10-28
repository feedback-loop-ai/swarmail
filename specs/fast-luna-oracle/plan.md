# Plan: repair the fast Luna oracle

## Technical context

One Rust crate; the existing `delivery_recipes` integration-test target uses
serde_json and filesystem reads rooted at `CARGO_MANIFEST_DIR` to inspect recipe
and office definitions. No new dependency, runtime path or test harness is needed.
Only a spec template is present in `.specify/templates/`; this plan supplies the
technical context, constitution, structure, research, data-model, contracts and
quickstart sections requested by the repository's speckit dialect.

## Constitution check

This is compatibility with accepted 0057/0058, not an amendment. Preserve
0001–0005, exact 100% line/branch/function coverage, the dependency allowlist and
0050's pinned proof ownership. Preserve the recorded engine classification and
0058's five-result review route, with no unauthorized fallback or DeepSeek Pro.
If semantic/security work is required, stop and surface the scope conflict.
The design phase commits only these three documents, unsigned under 0056; it
claims neither implementation completion nor a green delivery gate inventory.
The original stale assertion remains until the implementation phase.

## Research and evidence

The current intake and both positions agree on the two stale expectations.
Direct inspection confirms that all four cases in
`bundles/codex-fast/bundle.json` name `codex-fast-implementer`, whose office has
models `["luna"]` and effort `xhigh`. The adjacent test pins the same office and
`adapters/codex.json` mapping to `gpt-6-luna`. The named selector test still
expects `["sol"]` and `efforts.sol == "high"`. No unresolved architecture or
provider-dispatch research is needed to correct these two expectations.
The synthesis and rejected alternatives are recorded in spec.md.

## Project structure and touched files

| Path | Role |
| --- | --- |
| `specs/fast-luna-oracle/spec.md` | Chief-owned requirements and synthesis. |
| `specs/fast-luna-oracle/plan.md` | Chief-owned implementation and proof approach. |
| `specs/fast-luna-oracle/tasks.md` | Chief-owned ordered work and paired tests. |
| `tests/delivery_recipes.rs` | Sole later implementation edit: two assertions in the named selector test. |
| `bundles/codex-fast/bundle.json`, `agents/codex-fast-implementer.json`, `adapters/codex.json` | Read-only contract evidence. |
| `docs/house-rules.md`, accepted 0050/0056/0057/0058 | Read-only gate, proof, commit and routing authority. |

Run-local positions, framing, gate logs and result JSON stay ignored and
uncommitted. No research.md, data-model.md or contract files are necessary for
this two-line correction; their relevant content is contained here.

## Implementation approach

1. Establish the unchanged candidate and run the exact named regression before
   editing. Record its actual Luna-versus-Sol failure, or surface a different
   failure; a compiler/setup failure does not prove this defect.
2. In the existing four-case loop replace exactly:

   ```rust
   assert_eq!(overlay["models"], json!(["sol"]), "{class}: Codex only");
   assert_eq!(overlay["efforts"]["sol"], "high");
   ```

   with:

   ```rust
   assert_eq!(overlay["models"], json!(["luna"]), "{class}: Codex only");
   assert_eq!(overlay["efforts"]["luna"], "xhigh");
   ```

3. Run the exact regression and the complete target. Inspect the diff against
   the chief's HEAD to prove all other bytes, including selectors, reviewer
   checks and the neighboring test, survived. Existing exact equalities supply
   the negative model/fallback/effort protection; no new test framework is needed.
4. Execute the full mandatory gate inventory against the repaired candidate.
   Preserve the verifier's pinned scripts and checked shims, use the
   candidate-local `.forge/bin/brokkr` matched to `bundles/self/tooling.json`, and
   keep verification offline with no installation. A missing proof or unrelated
   failure is a recorded blocker, not permission to broaden this repair.
5. Let the existing protected review and ship machinery assess the recorded
   route and candidate. Keep all role results distinct. No historical run or
   earlier gate receipt substitutes for evidence on this candidate.

## Data model and contracts

No data model, migration, wire protocol, API or configuration schema changes.
The existing selector cases, office model array and effort map remain read-only
inputs. The independent normative expectations become Luna and xhigh. Their
failure on a future incompatible route is intended. Structural tests prove
these definitions; they do not prove live provider availability or dispatch.
The chief result remains exactly `result`, `inputs`, `notes`, with `drafted` as
its result; only the deterministic final check rules design advancement.

## Quickstart and verification

Before editing, then again after the two-line correction:

```bash
cargo test --locked --test delivery_recipes fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers -- --exact
```

After editing:

```bash
cargo test --locked --test delivery_recipes
```

Then execute every gate in the authoritative
[house inventory](../../docs/house-rules.md#gates--part-of-every-change), with
its locked Cargo checks, pinned compile, exact coverage plus selftest,
publish/seal guard proofs, license check and fail-closed native-tls absence.
The same inventory states which proofs only CI owns; report that distinction.
Do not replace full verification with the targeted regression. Preserve pinned
verify/ship ownership under 0050.

For this design commit, the local structural preflight is
`bash bundles/self/scripts/design-check.sh check .`; the sequence's final
pinned execution remains authoritative. Confirm `git diff-tree` lists exactly
these three design files and the worktree is clean. Keep the design commit last.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Fix weakens reviewer/default protections | Two-line replacement and exact diff audit; retain every surrounding assertion. |
| Oracle becomes circular or permits fallback | Literal singleton equality and exact xhigh effort; retain the independent office/adapter test. Accept duplicated normative literals. |
| A cheap-lane preference is mistaken for a recorded reclassification | Preserve TRIAGE-ENGINE and its five results; document the extra four reviewer invocations. No routing edits. |
| Full gates expose an unrelated failure | Record it separately and stop scope expansion; retain all proof obligations without claiming green. |
| Structural checks are oversold as runtime proof | Bound the claim to definition agreement; no new live-provider, persistence or availability assurance. |
| Design draft is confused with completed repair | Keep implementation tasks unchecked and distinguish drafted design from downstream implementation, verify, review and ship. |
