# Implementation plan — structured MCP report output

Implements [spec.md](spec.md) under proposed
[0091](../../docs/decisions/0091-mcp-report-output.md).

## Technical context

One Rust crate, thirteen fixed MCP tools, existing Tokio/Axum/serde_json/regex
and real HTTP/binary-stdio harnesses. No dependency addition or feature switch.
The named .specify/templates/plan-template.md is absent in this checkout; this
plan supplies the required technical context, constitution check, structure,
research, data model, contracts, quickstart and risks in this file.

This is a design-only commit. It does not implement the report schema or attest
green implementation gates. The engine's next deterministic check alone rules
the design artifacts; implementation, verify and protected review remain owed.

## Constitution check

Preserve 0001–0005 and accepted 0029/0039/0044/0047 error, notification and
transport contracts; retain the operator's 0079/0080 C1–C6 disposition and
0082 historical clarification. Lazy report analysis authorized separately under
0085 never replaces ingest extraction. 0091 adds a modern output promise, not an
amendment weakening accepted guarantees; all proposals remain proposed under
0045 §8. Frozen weakening is refused even with a proposal.

Keep 0020 test placement: branch-stable tests inline and measured; concrete cold
failure paths in the existing cfg(test) sibling harness beside their owner;
public protocol witnesses in tests/. Use per-test inboxes/fresh servers and
deterministic readiness/barriers, chaos 0/100 and generated TLS fixtures where
needed. Do not move tests merely to escape coverage. Exact zero-missed lines,
branches and functions, permissive-only locked dependencies, shims and protected
review stay unchanged. Work writes use only authorized workspace hands.

## Research and decisions

The inspected catalogue already includes the report and marks it read-only for
modern revisions. OUTPUT_SCHEMAS lacks that name. The existing text_result
serializes one Value then moves it into structuredContent for modern success;
report dispatch already uses this owner. Preserve these paths.

The historical structured workflow and projections cover twelve tools.
schema_ok currently rejects enum, maxItems, minLength and pattern; schema
numeric bounds are restricted to +/-300000 and extracted as i64. Widen that
finite proof deliberately; do not loosen its fail-closed behavior. The four
OpenAPI report components have closed objects, u64 bounds, exact patterns and
256 findings. Their description separately owns the compact 65536-byte limit.
Production collectors emit Header/MimePart and authentication unknown findings;
Link and an empty report need typed evidence, not a new production fixture mode.

The old report TOOLS description is stale. March compatibility takes precedence
over editing that shared literal: modern tool_descriptors overrides just the
modern description, and current public docs explain retained legacy text.
The old #152/#153 artifacts and 0082 are historical. New plan instructions
supersede prospective edits to them for this follow-up only.

Decision numbering uses 0091: tracked records extend through 0089, while the
supplied prior mixed-routing review attributes 0090 to its separate route
design. Do not reuse that known allocation or lower reserved gaps. Implementation
adds the 0091 index row; the chief's commit is exactly the three design files
and this new proposal.

## Implementation sequence

1. Establish evidence before changing behavior. Preserve historical fixture,
   specs and prior findings by digest. Re-run the focused suite to record actual
   current failures. Inspect/build 3a2eef948ed26392fe8ce2254f4b522964459a35 in an
   isolated directory entirely inside the worktree, using authorized workspace
   commands and installed locked/offline tools. Do not switch the candidate
   branch, alter source to synthesize March behavior, or write sibling checkouts.
   Record its tree, source/build procedure and captured executable digest.
2. Capture a new named March transcript from the unmodified pre-#153 baseline.
   Use a small capture harness against real HTTP and the actual baseline stdio
   executable, fixed request IDs/inboxes and repeatable stored-byte fixtures.
   Existing restored-row harness data may supplement the public workflow when
   explicitly recorded as test fixtures; the production baseline binary must
   not be patched. Compare omitted/explicit March transports. Record raw bodies,
   identities and permitted dynamic substitutions before testing the new output.
   If unavailable, record the concrete gap and keep AC6 unchecked.
3. Add local report fragments to OUTPUT_SCHEMAS using output_object, all tagged
   location arms, exact bounds and patterns from the spec. Keep the formatter,
   error helper and store/report selection unchanged. Extend exact schema-name
   and annotation checks to all thirteen names rather than only the count.
   Correct the report's modern description in the revision projection only.
4. Extend schema_ok/evaluate/numeric_bound in the existing test harness.
   Prevalidate every branch. Admit only the two inspected patterns with existing
   regex or a finite matcher tested independently; unknown patterns refuse.
   Handle enum/maxItems/minLength and malformed keyword shapes explicitly.
   Exact integer comparison can use checked i128 for signed/u64 values; integral
   floats require range-aware comparison that never rounds an integer bound or
   saturates 2^64 to u64::MAX. Pin both valid and invalid encodings before relying
   on these helpers. Keep unknown-keyword controls by replacing newly supported
   examples with truly unsupported assertions.
5. Independently write four source pins from inspected served OpenAPI; retain the
   existing six pins. Extend only the explicit structural projection. Resolve
   finite local source refs for comparison, then require reference-free emitted
   schemas. Strip only classified metadata in schema positions; preserve all
   assertions and request defaults. Test mutations on copied fixtures for each
   constraint and each source projection, independently of production builders.
6. Extend Wire/workflow with a real successful report request before the fixture
   message is deleted, plus report refusals. Require exact case/name sets and
   equal lengths before row pairing. Use the captured baseline for March/default
   bytes and modern text; also use independently stated expected payloads.
   Test June/November x HTTP/stdio, with text-only seed-ID extraction, empty/
   nonempty existing tools, report findings, wait arms, optional/null historical
   fields and chaos states. Compare full report findings to expectations and
   REST, never just two fields created by text_result.
7. Add branch coverage for every typed report form through existing owner-local
   tests, exposing no production knob. Cover zero/max coordinates, all severities,
   explicit-null header occurrence, empty paths and report. Wire fixtures cover
   production-reachable forms; label reserved forms typed-only. Reuse existing
   analysis-failure/unavailable seams to prove sanitized errors through adapters
   when public input alone cannot deterministically reach them.
8. Preserve/extend refusal, notification, mixed-version and actual bridge-byte
   witnesses. Retain real below/above-cap and March-fit/modern-overflow tests,
   measured including escaping, correlated errors and continued operation.
   Preserve existing store, SMTP, watcher, pruning, report cache/worker/deadline,
   chaos and webhook regression suites. Do not introduce runtime retry, second
   analysis, stronger snapshot semantics or general conformance repairs.
9. Update current README/llms/OpenAPI descriptions without changing OpenAPI
   assertions or March descriptors. Index 0091 as proposed. Add baseline
   provenance to the fixture README without replacing its historical paragraphs.
   Record implementation witness names and remaining limitations in this
   feature's artifacts; keep all earlier specs/receipts and 0082 byte-identical.
10. Finish tracked work and commit unsigned. Execute the entire gate inventory
    on the clean candidate, recording command, status, candidate/tree and log
    identities. Fix only concrete in-scope failures and recommit/rerun as needed.
    Engine verify binds the resulting committed candidate; engine review gathers
    the five current results. Do not author other seats' results or call a
    candidate commit shipment.

## Project structure and files touched

| Path | Responsibility |
| --- | --- |
| specs/mcp-report-output/{spec,plan,tasks}.md | This current design and later actual implementation status |
| docs/decisions/0091-mcp-report-output.md | New proposed modern report contract supplement |
| src/mcp.rs | Missing report schema fragments; modern-only description and catalogue assertions |
| tests/mcp_revisions.rs | Exact inventory/annotations and existing revision isolation witnesses |
| tests/mcp_revisions/structured.rs | Finite evaluator, four source pins/projection, real workflows, independent expected values and legacy comparison |
| tests/fixtures/mcp-structured-output/march-current-main.jsonl; README.md | New independent thirteen-tool baseline plus separate provenance |
| src/deliverability.rs; src/deliverability/concurrency_tests.rs | Existing cfg(test) owners for typed report/seam proofs only where necessary; no production evaluator change |
| src/mcp/batch_tests.rs; tests/mcp.rs; tests/p2.rs; tests/stdio.rs; tests/deliverability.rs | Extend only concrete uncovered refusal/adapter cases in their existing owners |
| README.md; src/static/llms.txt; src/static/openapi.json | Current revision/rule/size/evidence descriptions; OpenAPI assertions unchanged |
| docs/decisions/README.md | Index 0091 as proposed during implementation |

The original march.jsonl, historical specs/mcp-version-negotiation/ and
specs/mcp-structured-output/, existing decisions including 0082, and prior
run evidence remain unchanged. No planned production edits to Store, persistence,
SMTP, extraction, report evaluation, webhook, stdio, Cargo, bundles or gate owners.
A concrete defect beyond this list is documented and assessed against scope
before adding it; it never authorizes an engine attack or repair.

## Data model and contracts

No stored representation, request contract or rule-set identity changes.
Use the already serialized Report/Finding/Location/Severity model. Schema
fragments promise successful modern payload shape; they never decide whether an
effect may occur. Error/JSON-RPC helpers remain outside success formatting.
Closed output objects intentionally require a schema change when new fields
are later added. OpenAPI projections preserve the existing twelve-tool
serializer rules plus the new report equality.

Typed tests and wire tests are separate evidence classes. Owner-local empty/
Link reports validate reserved serialization contracts without claiming production
reachability. Byte bounds/order/dedup/cache/job limits stay producer proofs rather
than invented JSON Schema keywords. Snapshots and representation duplication
preserve uncertain outcomes on lost replies.

## Quickstart and verification

Offline, installed tools only. Through workspace hands, verify the supplied
packet manifest, candidate-local engine digest against bundles/self/tooling.json
and shim hashes before execution. Pin expected digest from that file, not a
sibling binary or PATH. Use in-worktree temporary/build/evidence directories.
The selected engine digest at design is
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.

Run focused suites first:

```bash
export CARGO_NET_OFFLINE=true
cargo test --locked --test mcp_revisions
cargo test --locked --test mcp
cargo test --locked --test p2
cargo test --locked --test stdio
cargo test --locked --test deliverability
```

Then execute every command in the single
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
including all gates after a failure; use verified pinned owners under 0050:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Require exact zero missed lines/branches/functions, no sweep warnings and unchanged
shims. Prove native-tls absence with cargo tree --locked -i native-tls failing
specifically because it did not match any packages; a different error is not
absence. Retain command/log/exit status and candidate identity for every check.
No installation, coverage denominator reduction or permission workaround.

The design seat checks its four committed artifacts and the pinned design
prerequisite checker; that is not the implementation's full verification.
A changed pinned bundle layer requires a freshly compiled run before engine
verification; no such change is planned. Standalone evals are not a shortcut for
gate evidence: do not execute them without authentic host-wide same-user
nonoverlap and captured executable digests before/after. Missing coordination
is a reported prerequisite gap. Do not execute the known review-integrity attack
matrix to manufacture evidence.

CI retains platform/macOS, network RustSec, chart/baseline/history, Docker and
real port-80 checks; tag-only rate/publication proofs are separate. Actual #152
baseline/protected review, paid #151 baseline, actual clients and operator seal
remain external. The selected route and all prior residuals are preserved.
Only the chief result is drafted; deterministic check, implementation, verify,
review and shipment verdicts belong to their respective phases.

## Risks

- **Schema appears complete while permissive:** exact name coverage, closed
  objects, complete required sets, independently expected findings and mutation
  controls. No null schema or count-only witness passes.
- **u64 rounding silently admits overflow:** exact integer comparisons and
  independent boundary encodings near 2^53, i64::MAX, u64::MAX and 2^64;
  integral floats handled without rounded bounds/saturating casts.
- **Projection masks drift:** pin original components first, classify metadata
  at schema positions only, then test deliberately altered constraints/wrappers.
  Literal duplication is accepted; automatic regeneration is not proof.
- **Legacy self-comparison:** capture from a real separate #152 executable,
  record digests and raw bytes, freeze old fixture, reject absent provenance.
  A local baseline receipt never substitutes for protected baseline review.
- **Reserved forms mistaken for wire coverage:** label owner-fixture positives
  separately, require production successes for every tool/transport/revision,
  and keep impossible production cases explicitly unclaimed.
- **Stale description repaired on March accidentally:** override modern text
  only; byte-test March descriptors and keep public historical attribution.
- **Larger replies:** keep both representations and actual-byte cap witnesses.
  Smaller pages are practical advice, not a guarantee that one email fits.
  Response duplication and caller-held Arcs remain memory costs.
- **Effect/format race:** one captured payload, existing snapshot/wait owners
  and no post-effect validation/retry. Lost output never implies rollback.
- **Flaky or misleading proof:** unique inboxes/fresh destructive servers,
  deterministic seams/deadlines, full error/notification expectations and strict
  transcript lengths. Keep report worker-held reservations and inherited TLS
  test discipline.
- **Evidence overclaim:** all historical findings retain original severity;
  engine reviews remain protected/read-only, unknowns stop, host coordination
  is not inferred from the box, and CI/operator prerequisites remain outstanding.
