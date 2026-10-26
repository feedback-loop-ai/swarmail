# Plan: GLM Flash seats

## Technical context

Configuration and delivery-contract change in one Rust crate. JSON office
overlays and existing dsh alias resolution express the routing. Rust tests
pin definitions; Python exercises the real pinned CLI and controlled
proof participants. No application source, dependency, storage migration,
new provider, credential or public mail contract changes are needed.

Use `.forge/bin/brokkr` with SHA256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Do not substitute a same-version engine, patch it, or alter its pin. The
repository has a spec template but no plan template; this document supplies
the technical context, constitution, structure, research, data model,
contracts, quickstart and risks required by the spec-kit dialect.

## Constitution check

0081 is proposed and narrowly records the explicit 2026-10-26 instruction;
no seat accepts it. Accepted 0057/0058 otherwise govern, with 0045 §8 and
0050's proof ownership intact. Frozen 0001–0003 and extraction-at-ingest
0005, real-protocol tests, exact 100/100/100 coverage, dependency policy,
protected review and security hold remain. Review tool permissions must not
be enlarged merely because the provider changes. Commit unsigned, never push.

This chief commit contains exactly spec.md, plan.md, tasks.md and proposed
0081. Indexing 0081 and any dated pointers belong to implementation. The
existing decision-inventory test therefore rejects this intermediate
unindexed proposal, and workspace tests/coverage cannot be represented as
all green. Run every requested gate and record its actual result anyway;
never edit the test, add an extra committed file or conceal the expected
failure to satisfy two incompatible phase obligations. The final design
validator assesses the four committed artifacts; it cannot certify routing
implementation or discharge the compatibility prerequisite.

## Research and decisions

Both panel positions and the intake framing were read. The supplied evidence
checksums matched; README contains no prior run entries. Local intake records
baseline `a037dc9`, passing house gates before this proposal, and the separate
full dispatcher cross-attempt failure. These are baseline observations,
not final-route evidence. The decision index contains no 0081; the current
dated numbering instruction controls the older ledger.

The chief's disposable probe established that explicit `efforts: {}` gives
empty resolved maps, and fast compilation selects GLM tests with null effort.
It also established that both boxed intake offices are blocked under dsh and
Standard/Careful cannot compile with preserved hands. Evidence remains in
ignored run-local output, named `glm-flash-chief-inheritance`, including
resolved office JSON and final compile stderr. No live provider was invoked.

This supersedes both panel positions' assumption of an immediately viable
three-office patch. The design fixes the required target and refuses a
permissions shortcut. First reproduce this prerequisite with the candidate's
full realm/dialect setup. If it persists, record the exact unavailable
capability and stop dependent implementation; do not make speculative
production edits or claim partial routing as accepted. A separately authorized
compatibility change would require a revised plan and fresh proof, not an
implicit exception in this task. In particular, do not set unsupported
adapter capabilities to supported or delete intake hands to silence compile.

## Project structure and touched files

| Path | Planned change / proof responsibility |
| --- | --- |
| `agents/codex-intake.json`, `agents/codex-careful-intake.json`, `agents/codex-review-tests.json` | Only models and effort maps: singleton GLM and `{}`; preserve all other fields. Conditional on resolving the prerequisite without violating scope. |
| `tests/delivery_recipes.rs` | Exact office-specific models/efforts, dsh mapping and effortless/local metadata; retain charter/hands checks on intake and exact lane topology. |
| `tests/codex_review_routing.py` | Extend existing resolved matrix with exact three-office comparison and mutations; assert successful status and compiled effective effort. Preserve dispatcher controls. |
| Other existing proof files discovered by scoped search | Update only executable pins of the superseded intake/tests assignments; no new framework or blanket allowlist. |
| `agents/README.md`, `bundles/README.md`, `docs/house-rules.md` | Correct current route descriptions, careful intake effort and provider limitations without changing the gate inventory. |
| `bundles/codex/bundle.json`, `bundles/codex-careful/bundle.json`, `bundles/codex-fast/bundle.json` | Description text only where stale; composition/seat metadata stays identical. |
| `docs/decisions/0081-glm-flash-seats.md` | This chief's proposed record; preserve its status. |
| `docs/decisions/README.md` | Add proposed 0081 row during implementation. |
| `docs/decisions/0057-codex-delivery-recipes.md`, `docs/decisions/0058-codex-review-routing.md` | Dated pointer to proposed amendment if needed; accepted bodies remain byte-preserved. |

Preservation inputs: base offices/charters, `adapters/dsh.json`, all other
offices, bundle policies, `bundles/self`, engine/tooling, gate scripts/shims,
Cargo files and application `src/`. No extra design artifact or index edit
belongs in the chief's four-file commit. Existing `intake.md` remains intact.

## Implementation approach

1. Verify evidence/engine digests and clean starting revision. Resolve baseline
   manifests/offices and capture non-routing JSON plus inherited charter hashes
   against `a037dc9`. Search current docs and executable tests for old pins;
   distinguish historical decisions and valid Sonnet fallback references.
2. Reproduce the measured GLM/hands conflict in an isolated fixture with the
   same agents/adapters/bundles/realm/dialect. Require per-candidate `ok`, not
   merely command exit zero. If unresolved, report the exact prerequisite;
   continue only independent documentation/proof preparation, not a false
   successful route patch. Do not amend engine or permissions within this work.
3. Once an authorized compatible basis exists and this design is reconciled,
   edit the three overlays in place. Empty effort maps replace inherited
   metadata; retain office identity and every other field. Resolve and compile
   again before trusting source edits.
4. Extend existing Rust checks with exact GLM expectations only for these
   offices. Preserve the full hands/charter loop for intake. Extend Python
   using one local comparator for actual and mutated records: office name,
   provider, chain, concrete route, local classification, empty effort,
   resolution status and compiled candidate effort. Reject each AC6 mutation
   through that same comparator. Keep unrelated model/effort assertions exact.
5. Compare full panels, fast selectors, protected script origins/hands/boundaries,
   non-routing office fields and inherited charter identity to baseline.
   Keep existing dispatcher, fallback, aggregation and candidate-fence controls.
   A test of mutant records proves the comparator, not live driver support.
6. Index the decision, add only dated historical pointers, and correct current
   summaries. Quote operator provenance in tracked prose rather than linking
   ignored evidence. Do not globally replace Sonnet or claim Codex intake still
   has higher Careful effort.
7. Commit the implementation before final verification. Run the focused and
   full proofs separately, all compilations and every house gate. Record
   actual exit codes, candidate hash, engine digest and fresh evidence paths;
   retain full errors. No failed or unexecuted check becomes a success claim.

## Data model and contracts

No new schema. Existing office name, base charter, model array, efforts map,
provider resolution and compiled candidate tuple remain the entities. An
empty effort map is configuration, while manifest `effort: null` is the
observable absence of selected effort; prose says “not applicable.” The
adapter's local classification is metadata, not evidence of live availability.

Review remains gate-class, read-only, with separate required results and
unchanged result vocabulary/input keys. Preserve `result`, `inputs`, `notes`
and explicit safety/scope/fixes facts. Missing or invalid results refuse;
no silent paid substitution or skipped tests reviewer is allowed. Existing
attempt/deadline limits apply. The baseline dispatcher correlation defect
means those declarations alone cannot establish dispatcher-wide enforcement.

## Quickstart and verification

Work offline, install nothing, and set `CARGO_NET_OFFLINE=true` and prepend
`$PWD/.forge/bin` to PATH. Verify the engine digest above and strict public
shim digests in `bundles/self/scripts/shims.sha256`. Run the pinned scripts
against the candidate as required by 0050; the public shims name those gates.

For implementation, first run:

```bash
cargo test --locked --test delivery_recipes --test delivery_constitution --test store_contract_docs
```

Compile `self`, `codex`, `codex-careful` and `codex-fast` using
`.forge/bin/brokkr compile --bundle bundles/<name>` and reject policy-sweep
warnings. Then run the existing full dispatcher proof with a fresh output
directory and independently invoke `resolved_matrix`:

```bash
python3 tests/codex_review_routing.py --engine .forge/bin/brokkr --out .forge/proofs/glm-flash-full-UNIQUE
python3 - <<'PROOF'
from pathlib import Path
import runpy
proof = runpy.run_path('tests/codex_review_routing.py')
out = Path('.forge/proofs/glm-flash-resolved-UNIQUE')
out.mkdir(parents=True, exist_ok=False)
proof['resolved_matrix'](Path('.forge/bin/brokkr').resolve(), out.resolve())
PROOF
```

Replace UNIQUE once per execution; do not overwrite evidence. Run the entire
canonical [house inventory](../../docs/house-rules.md#gates--part-of-every-change),
including fmt, locked clippy/workspace tests, exact coverage,
coverage selftest, publish guard, keyless seal proof and licenses. Also run
pinned design-check selftest, delivery-proof-test and the verify seat's
fail-closed native-tls absence proof. The intake framing gives exact commands;
no network-backed CI ownership becomes a local criterion. Retain a separate
full-dispatcher outcome: the known `1-cross-attempt` assertion expects refusal
but baseline completes, and subsequent controls are unexecuted after that stop.

For this design's final HEAD use
`bash bundles/self/scripts/design-check.sh check .`. Keep the four-file design
commit last. Record design gate outcomes in ignored run evidence and the
required result receipt, so gate reporting does not add another commit.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Pinned engine refuses dsh intake hands | Measured prerequisite; retain hands and refusal. No in-scope compatible implementation is known; operator-directed compatibility work is required before claiming AC1/AC8. |
| Effort inherited from base or stale Sol/Sonnet map | Explicit `{}`; assert resolved empty map and manifest null effort. No base-office or adapter change. |
| Alias looks right but provider/route/status is wrong | Exact route tuple and negative controls; command exit zero alone never passes. |
| Broader allowlist hides moved implementer or lost role | Per-office source assertions, full resolved matrix and unchanged-role comparison. |
| dsh lacks native permissions/MCP enforcement | Retain capability truth, charters, gate class and candidate-change refusal; never claim equivalent confinement or add powers. |
| Free route unavailable or poor review | Existing attempts/deadlines and missing-result refusal; no paid fallback. Availability and model choice remain operator-owned. |
| Known engine correlation failure obscured by route green | Preserve failing control and separately report full vs focused proof, including later unexecuted cases. No engine re-pin in this feature. |
| New ADR omitted from index during design | Required four-file commit defers index; report failing inventory/workspace/coverage checks honestly, then implementation adds row before final gates. |
| Proposed text mistaken for acceptance | One proposed status; explicit dated source, no historical rewrites or unrelated approvals. |
| Concurrent proof outputs collide | Fresh ignored directories and captured candidate/engine identity for every run. |
