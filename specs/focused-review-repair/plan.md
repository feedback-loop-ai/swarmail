# Implementation plan — focused review repair

Implements [spec.md](spec.md) and proposed
[0097](../../docs/decisions/0097-focused-review-repair.md).
The plan template is absent from .specify/templates; this plan supplies its
required technical context, constitution check, structure, research, data model,
contracts, quickstart and risks. The explicit commission governs artifact location.

## Technical context

One production Rust crate using existing serde/serde_json, rusqlite, Tokio and
Axum. Evals is outside the root workspace. Reuse types, proof owners and locked
dependencies; add no dependency, feature, allowlist entry or schema migration.
The chief commits only the three new spec files and one proposed decision.
Intake and position files remain uncommitted.

## Constitution check

Preserve 0001–0005 and accepted mutation, admission, lifecycle, extraction, test
and gate rules. Metadata failures propagate through existing startup errors
(0042). 0029/0070's REST chaos reader restriction is not a prohibition on internal
stored metadata readers and is not widened. Preserve 0043 OPTIONS semantics.
Strict eval v2 follows 0063/0071/0076, not the historical conflicting arm.
0097 grants no accepted-ruling exception under 0045 §8.

Use real SQLite/servers, fresh stores/inboxes, ephemeral ports, instance-local
barriers/clocks, chaos poles 0/100 and generated certificate fixtures. Inline
branch-stable tests stay measured under 0020; cold-path harnesses need a concrete
cold-path reason. No production test flags, coverage exclusions, sleep-as-sync
or shared global mutable test switches.

Implementation first adds the proposed 0097 index row after collision recheck.
The chief's four-file constraint leaves that task outstanding; any resulting
inventory failure is an unmet gate, not a waiver or whole-gate success claim.

## Research and chosen implementation

LinkMetadata's derive generates sequence admission despite deny_unknown_fields;
LinkSource's enum derive admits tagged objects. The persistence bridge already
propagates serde failure as column-19 Text conversion failure. Remove the generated
alternate admission at the existing type owners, not query/ingest guard layers.

1. Retain Serialize and the existing type/field shape. Implement map-only
   LinkMetadata Deserialize with a private closed field helper preserving
   required_nullable. Delegate through MapAccessDeserializer; expose no visit_seq.
   Decode raw map entries so duplicate keys cannot collapse in a Value prepass.
2. Decode LinkSource as a string matched exactly to text/html, retaining lowercase
   serialization. Reject tagged objects, lists, null and numeric representations.
3. Keep the vector/SQL NULL bridge and error propagation in persist::link_metadata;
   no fallback, migration, new validation framework or schema version.
4. Extend both existing corruption tests with complete positional occurrences,
   object-valued sources for both variants and raw duplicate-key controls. Prove
   load and startup, drop Store/Persist owners before restart, assert the
   metadata-specific cause, restore valid populated bytes and query a real focused
   result. Universal startup failure must not satisfy the negative test.

Both positions agree on this mechanism. Robustness governs proof breadth;
simplicity governs implementation size. The spec records the tradeoffs.

## Project structure and files touched

| File | Implementation or proof responsibility |
| --- | --- |
| src/model.rs | Two narrow readers and branch-stable owner tests if needed. |
| src/persist.rs | Extend metadata_null_empty_present_and_corrupt_are_distinct; retain bridge. |
| tests/persistence.rs | Extend a_corrupt_occurrence_column_refuses_to_serve and paired populated startup/query; preserve migration/order/refusal witnesses. |
| README.md; src/static/llms.txt; src/static/openapi.json | Correct additive discovery and schema proof claims without unrelated schema changes. |
| src/mcp.rs | Reconcile object-root assertion and applicable shared protocol controls; preserve fifteen descriptors/formatter. |
| src/api.rs | Reconcile method-aware OPTIONS admission and shared-router-budget proof. |
| tests/mcp_revisions.rs; tests/mcp_revisions/structured.rs; tests/mcp_revisions/focused.rs; tests/stdio.rs; tests/mcp.rs | Revision/transport, report/refusal/silence, served prose, legacy and focused controls. |
| src/deliverability/concurrency_tests.rs | Adapt #153's real unavailable-report transport control with existing cfg(test) ownership. |
| evals/src/artifacts.rs; evals/src/provenance.rs; evals/src/results.rs | Resolve conflicts under strict current v2/provenance/privacy/retention semantics. |
| evals/tests/identity.rs; evals/tests/provenance.rs; evals/tests/fixture.rs | Identity/provenance regressions and preserved focused application traps. |
| evals/src/runner.rs; evals/tests/boundary.rs; evals/tests/common/coordination.rs | Inspect 4806ab8 dependencies and preserve applicable binary/lifecycle checks; edit only as reconciliation requires. |
| docs/decisions/README.md | Add 0097 as proposed after allocation check; no acceptance/renumbering. |
| New follow-up evidence record | Source/final identities, commands, statuses and missing proofs; no historical receipt copying. |

Existing src/extract.rs, src/store.rs, src/mcp/focused.rs and SMTP/webhook suites
are preservation owners, not a commission for architecture changes. No blanket
import of 4806ab8's CI, bundles, charter, routing or historical-spec changes.
Coordinator availability is an external prerequisite, not authority to build an
alternative gate in this story.

## Stack integration order

Record and recheck source identities from the spec. Integrate 393247c's schema
object-type assertion separately from absent root ref, then applicable changes
through df8f610 including 8d205d7/4227aef unavailable-report and exact modern
refusal/notification controls. Adapt current checks to fifteen tools while
retaining independent original descriptor/result captures.

Inspect 7f2e2129 ancestry: f0b2b14 contains runtime OPTIONS/shared-budget repair;
later commits correct served prose. Port behavior into current owners without
replacing focused tools/report output or importing older model-route selections.
Prove Host/Origin first, admitted OPTIONS 405/Allow/no-CORS regardless of version,
other-method version refusal and POST body-limit/parse precedence.

Resolve each eval conflict semantically against 4806ab8. Strict current v2,
provenance, seven-trap precedence, private retention and cleanup win over v1
fallback. Keep #154's additional fixture absent from that candidate. Inspect
runner/captured-binary dependencies before adapting tests. Commit and compile
the actual resulting eval tree; a repaired scratch copy proves nothing here.

Record changed source IDs and any unavailable eventual reviewed heads. A newer
main or candidate supplies neither a protected verdict nor a pinned-engine fix.

## Data model and contracts

No database or public output schema change. The internal occurrence contract is
the spec's exact four-member object, string source, required nullable strings and
stable order. SQL NULL remains unknown history. Failure stops restore before
readiness and preserves original bytes for operator repair; never rewrite damage.

No new REST focused endpoint, public Email metadata projection or URL trust
operation. Text/structuredContent remain one captured value. March/default keeps
existing entries/results followed by two descriptor additions. Focused schemas
are independently stated; REST projections remain pinned to served OpenAPI.
No golden regeneration from the implementation under test.

## Proof sequence and quickstart

Run targeted tests as changes land, then final receipts against the committed
candidate. The charter remains the single gate inventory; this is not a reduced
replacement. No commands below are claimed executed by the chief.

```bash
cargo test --locked metadata_null_empty_present_and_corrupt_are_distinct
cargo test --locked --test persistence
cargo test --locked --test focused_waits
cargo test --locked --test mcp_revisions
cargo test --locked --test stdio
cargo test --locked --lib every_tool_has_exactly_one_object_output_schema
cargo test --manifest-path evals/Cargo.toml --locked --no-run
```

Compilation does not authorize trial-launching tests. Inspect the actual eval
runner; execute affected standalone regressions and the focused fixture only
after authentic host-wide admission is evidenced. The requested selector is
`cargo test --manifest-path evals/Cargo.toml --locked --test fixture focused_tools_complete_link_and_code_traps_without_guessing`;
it names the required proof, not permission to bypass a full-suite coordinator.
Use the established coordinator's supported entry; an authentic full suite may
supply this fixture's execution. Do not create a filtered launcher, private lock,
wrapper, --test-threads=1 workaround or serial-process substitute.

Record authoritative shared admission, exclusive ownership through completion
and cleanup, actual final source/captured-binary identity, command, outcome and
cleanup. If unavailable, leave AC6/#201 unproved and continue independent work;
never declare full completion. tests/focused_waits.rs retains its separate real
application evidence and cannot stand in for the canonical eval fixture.

Before final gates verify engine SHA-256 against tooling.json and shims with
`sha256sum --check --strict bundles/self/scripts/shims.sha256`.
Use digest-checked `.forge/bin/brokkr compile --bundle bundles/self`.
The engine verify seat executes pinned script copies, not candidate dispatch
shims. Run all charter gates with --locked where applicable: fmt, clippy,
workspace tests, bundle compile, exact coverage/selftest, publish/seal proofs
and licenses, plus fail-closed native-tls absence and required pinned design/
delivery proofs. Unavailable/failing checks remain open. Do not execute the
inherited cross-attempt attack witness in this story; preserve its recorded
HIGH residual and owner without converting missing proof to pass.

Keep receipts outside the frozen candidate where needed to avoid changing the
tree they prove. A later tracked evidence commit creates a new final candidate
requiring identity-aware revalidation. Engine verification and five protected
members own final verdicts. CI, actual clients, #151 paid baseline, #201 macOS
and operator sealing remain separate prerequisites.

## Risks

| Risk | Mitigation and remaining cost |
| --- | --- |
| Alternate serde shapes still pass | Complete positional/tagged-source controls via raw SQLite, load and startup; raw duplicate-key negatives. |
| Restart fails for the wrong reason | Drop old owners; require metadata conversion cause; repair bytes and prove populated live query. |
| One malformed row prevents startup | Intentional refusal; preserve bytes for operator repair, no drop/backfill. |
| Parent port loses focused/report behavior | Inspect ancestry/hunks, retain fifteen-tool captures and paired report/focused tests. |
| Shared wait refactor regresses deadlines | Keep owners separate; deterministic terminal/Closed/Lagged/zero/shutdown witnesses. |
| Eval compiles after losing semantics | Strict v2/provenance/retention negatives, current scorer and extra fixture; no wholesale side selection. |
| Launcher mistaken for actual admission | Require authoritative host-wide ownership/cleanup receipts; keep missing proof open. |
| Root gates hide standalone breakage | Separate locked test build and coordinated regressions on actual source/binary. |
| Number/head allocation changes | Recheck identities and collisions; operator disposition, 0097 proposed. |
| Metadata/snapshot amplification | Preserve documented cost and full semantic selection, never conceal ambiguity by truncation. |
| Historical green/design result mistaken for clearance | Fresh candidate-bound receipts and missing-proof ledger; engine/security ownership remains. |
| Inherited HIGH integrity residual | Exact classification/typed facts; no attacks, access widening, re-pin or unpublished-fix claim. |
