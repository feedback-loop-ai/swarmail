# Implementation plan — structured MCP outputs

Implements [spec.md](spec.md), with proposed
[0082](../../docs/decisions/0082-mcp-structured-output.md).

## Technical context

One Rust crate using existing Tokio/Axum/serde_json, twelve fixed tools and real
protocol suites. No production or dev dependency is added. The named
`.specify/templates/plan-template.md` is absent; this plan supplies its requested
technical context, constitution check, structure, research, data model, contracts
and quickstart sections. No additional design artifacts are needed.

## Constitution check

Preserve synchronous insert/queryability before SMTP 250 (0001), subscription
before scan (0002), oldest-only pruning (0003), real isolated protocols (0004)
and ingest-only extraction (0005). Formatting occurs after the existing operation
from its retained value. Store, persistence, SMTP, extraction and webhook code
need no change. Preserve accepted 0029/0039/0044/0047 error/transport behavior
and operator-retained C1–C6 under proposed 0080. Decision 0082 is additive and
stays proposed; it accepts neither itself nor 0079/0080. Do not alter routing,
license allowlist, denominator, shims or gates. Tests follow 0020 placement rules.

## Research and choices

The verified packet and [intake](intake.md) establish scope. Existing Revision,
tool_descriptors, text_result and tool_call in src/mcp.rs own this change.
Email/address serializers omit optional fields and raw but always emit nullable
envelope_from; chaos emits all event keys and delay_ms. Served OpenAPI is 3.1.0,
with reusable components but loose response requiredness and some optional-null
allowances. The spec enumerates output tightenings instead of a REST migration.
The bridge forwards complete results and caps responses at 10 MiB; duplicated
representations can newly cross that cap.

The six absent upstream tools/schema snapshots are named in spec.md. No fresh
source audit/fetch is needed for this finite supplied contract. Preserve source
hashes and the clause ledger, updating applicability only with implementation
witnesses. Both panel positions are run-local evidence; the spec commits the ruling.

## Implementation approach

1. Before changing runtime, capture raw catalogue/result bytes from dc67f4a's
   behavior (current design-only descendants have identical runtime). Record the
   base and explicit substitutions. No sibling trees or branch switches are needed.
   Capture raw HTTP bodies/stdio lines, not a serde round-trip. Fix request IDs and
   inboxes; substitute dynamic IDs/timestamps at known locations with exact
   occurrence checks. Fixed stored Email fixtures prove historical provenance and
   signed-time cases. Preserve text escaping during substitution.
2. Thread Revision from handle_revision through tool_call to the success helper.
   Keep the March envelope expression; modern adds structuredContent from the
   same owned Value after text serialization. Route every success through it.
   Error/RPC builders stay separate. Update direct owner-test calls explicitly;
   no inference from text/global state, new effect, read or runtime validation.
3. Add a finite schema mapping beside the catalogue, reusing Email/address,
   output-chaos and wrapper fragments. Require one entry per tool; test exact
   domain equality. Add outputSchema in the existing modern descriptor projection.
   Leave TOOLS/input admission intact. Explicitly match supported Revision enum
   variants and share one modern schema family. Inline fragments are simplest;
   local $defs are allowed with complete references and the same proofs.
4. Extend tests/mcp_revisions.rs with a finite test-only evaluator/schema checker.
   Support only emitted type/type-array, properties, required, items,
   additionalProperties:false, minimum/maximum, minItems, const, anyOf/oneOf and
   chosen local $ref/$defs. Enumerate annotation-only keys such as description,
   default and title. Reject other keys, malformed keyword values and unresolved,
   nonlocal or cyclic references. Check every schema branch before data validation
   so an unused alternative cannot hide an unsupported assertion. Property names
   inside properties are data, not schema keywords. Integer checking follows JSON
   numeric integrality; bounds preserve integer values without lossy narrowing.
   Selftest valid/invalid examples for each emitted assertion and overlapping oneOf.
5. Fetch OpenAPI from the real server, resolve references independently, remove
   only enumerated annotations and apply the spec's exact projections. Assert
   source constraints before adding requiredness/removing null/adding bounds;
   normalization must not erase new source drift. Compare complete structures and
   independently pin mandatory fields/literals. Mutated test copies prove drift
   detection. Production schemas never come from the test's OpenAPI projection.
6. Execute all-tool workflows over real HTTP and real binary stdio for both
   modern revisions. Each validates against its returned schemas. Include empty/
   full variants, both waits, historical Email and chaos defaults. A text-only
   consumer reads content alone; structured checks are a separate observer. Use
   fresh servers for clear_all/chaos, per-test inboxes for scoped operations, mixed
   revision requests on one server and separate bridges for independent negotiation.
7. Preserve and extend modern refusal/notification witnesses: errors lack
   structuredContent; admitted notifications stay effectful/silent. Existing seams
   own storage-failure/shutdown/rate ordering. Tests with cold failure paths use
   existing sibling harnesses under 0020; deterministic inline tests stay measured.
   No new production flags or gratuitous harness relocation.
8. Add real-bridge cap cases using controlled mailbox bodies/pages within ingest
   limits. Measure encoded bytes, including escaping. Show below-cap dual success,
   above-cap refusal and a March-fitting response whose modern form exceeds the
   cap. Assert complete correlated error and continued bridge usability. Use
   read-only queries for crossings; existing failure contracts cover uncertain
   mutation receipts. No bridge rewrite, truncation, replay or cap increase.
9. Align descriptions, scoped #152 applicability and 0082/index pointers. Keep
   C1–C6 and source/witness identifiers unchanged. Correct current absence claims;
   preserve dated #152 receipts as historical evidence.
10. Commit implementation unsigned, run all gates on that commit and retain
    commands/status/logs/candidate identity in ignored evidence. Repair failures
    within scope and rerun affected checks. Mark only proven tasks complete.

## Project structure and files touched

| Path | Responsibility |
| --- | --- |
| specs/mcp-structured-output/{spec,plan,tasks}.md | Design and implementation task status |
| docs/decisions/0082-mcp-structured-output.md | Refine existing proposal; never accept it |
| src/mcp.rs | Revision plumbing, success formatter, schema fragments/mapping, modern descriptors and stable owner tests |
| tests/mcp_revisions.rs | Finite evaluator/selftests, OpenAPI pins, workflows, legacy bytes, text consumer and size proof |
| tests/mcp.rs; tests/p2.rs; tests/stdio.rs | Reuse helpers/witnesses; extend only uncovered error/transport cases |
| tests/fixtures/mcp-structured-output/ (if needed) | Reviewable raw legacy fixture data with base/substitution provenance, not generated-at-test-time expectations |
| Existing src/mcp/*_tests.rs harnesses as needed | Cold-path storage/shutdown/rate proofs in their 0020 owners |
| src/static/llms.txt; src/static/openapi.json; README.md | Revision-qualified output/compatibility/size descriptions; OpenAPI assertion structure unchanged |
| specs/mcp-version-negotiation/reconciliation.md | Structured applicability rows/witness pointers only; exact conflict register unchanged |
| docs/decisions/README.md | Update 0082's intake-only wording during implementation; retain proposed status |

No model, chaos, Store, persistence, SMTP, webhook or stdio runtime changes are
planned. No Cargo, bundles, gate or deployment changes. The chief commits exactly
three design files and existing 0082; later implementation owns the other files.
Design-time tests do not prove unimplemented structured output.

## Data model and contracts

No stored data changes. One original Value supplies both representations. The
spec's inventory defines fields, and its REST/MCP table is the sole permitted
structural difference list. A schema is an output promise, not a new input
validator or serialization-time refusal.

Do not invent cross-field transactions: wait counts can observe a later state,
and chaos can change between update and snapshot. Captured values remain stable
while later inserts may prune mail. Historical code arrays retain length. Serde
escaping keeps untrusted strings as data; raw stays absent and no HTML or links
are executed. Structure does not solve consumer prompt injection.

## Quickstart and verification

Offline, installed tools only. Verify the packet from its directory and engine
identity before using the candidate-local engine:

```bash
(cd .forge/epic192/issue153 && sha256sum --check --strict SHA256SUMS)
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
export CARGO_NET_OFFLINE=true
cargo test --locked --test mcp_revisions
```

Run the complete [house inventory](../../docs/house-rules.md#gates--part-of-every-change),
using pinned script owners after shim verification under 0050:

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

Require no policy-sweep warning from compile and zero missed lines, branches and
functions. Run every gate even if one fails. Fail-closed native-tls absence also
requires `cargo tree --locked -i native-tls` to fail specifically with
`did not match any packages`; success or another Cargo error fails the proof.
After the chief's commit run `bash bundles/self/scripts/design-check.sh check .`:
HEAD must touch exactly one spec directory and its artifacts/decision must be clean.
Keep the design commit last in this seat; implementation makes its own commits and
reruns gates. Repeat green checks only when changes or failures justify it.

CI owns macOS/network/chart/Docker/privileged-port proof. The operator owns actual
clients, baseline/metric trials, live settings, sealing and merge. Local success
neither substitutes for those obligations nor lifts the #151 baseline merge hold.

## Risks

- **Weak/drifting schemas:** exact catalogue coverage, closed objects, required
  fields, negative controls and independent served-OpenAPI projections. Vocabulary
  growth requires extending and proving the bounded evaluator.
- **Projection hides drift:** check source constraints before transformation;
  mutate nested requiredness/types/nullability/bounds. Handwritten duplication
  remains an accepted finite maintenance cost.
- **Legacy byte regression:** pre-change raw fixtures, unchanged March helper and
  catalogue, bounded dynamic substitutions and HTTP/binary stdio proof.
- **Representation race:** one owned payload and immutable exchange revision;
  no second read or shared session mode.
- **Larger responses/memory:** retain text and the 10 MiB cap, document smaller
  practical pages and that one large Email can still exceed it. Escaping makes
  growth nonuniform. No truncation or raising the cap to hide the cost.
- **Post-commit failure mistaken for rollback:** preserve transport uncertainty;
  no replay. Reply loss can follow commit, and memory-mode restart/best-effort
  webhooks gain no durability promise.
- **Flaky broad proof:** dedicated servers for destructive/chaos tests, readiness/
  seams and deadlines without sleep ordering, probabilities 0/100 only. Existing
  TLS suites own generated certificates and expiry-path proofs.
- **Scope/governance creep:** reuse storage/TLS/concurrency witnesses, preserve
  C1–C6/routing/proposal statuses and #151 hold. Fixtures never establish actual
  client compatibility or favorable token/misuse metrics.
