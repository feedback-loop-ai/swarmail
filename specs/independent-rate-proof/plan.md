# Plan — Independent rate and subprocess proof

## Technical context

One Rust crate with real-protocol integration suites and deterministic cfg(test)
owner seams. Baseline identity and evidence attribution are in [spec.md](spec.md).
The repository has no `.specify/templates/plan-template.md`; this plan supplies
its requested sections directly. No remote research, dependency, runtime, schema,
engine, route or permission change is planned. Chief commits only this directory's
spec/plan/tasks; implementation supplies the tests in a later phase.

## Constitution check

Preserve 0001–0005 and current persistence/shutdown ordering, accepted 0039
concurrency as subsequently amended, 0043 guard/method precedence, 0045 §8 proposal
boundaries, 0050 pinned proof identity and 0056 unsigned authorship/operator seal.
Follow 0020: deterministic branch-stable owner assertions stay inline; public
subprocess witnesses stay in tests/. No coverage exclusion, mock transport,
sleep-as-sync, shared global mutable test state or mid-band chaos. Retain exact
coverage and permissive-only dependencies. Existing proposed statuses and all
predecessor AC1–AC11 remain unchanged; no ADR is warranted by proof strengthening.

## Research and chosen approach

Source/evidence inspection confirms the numerical and subprocess gaps; the separate
mixed-panel event merely prompted local inspection. Checksums prove consistency,
not protected-review integrity. `tests/binary.rs::McpChild` already supports a
custom `mcp --url` command, bounded send/reply and EOF/exit, stderr discarded to
avoid backpressure, and kill/reap on I/O failure. Reuse it instead of expanding
`tests/mcp_revisions.rs::Binary`, whose finish reads stdout before its exit timeout.

1. Strengthen `tool_rate_uses_exact_monotonic_windows` using generated t and literal
   counts/instants, including immediate 1025th refusal and all of the second window.
   Continue calling the production owner with supplied monotonic instants. Replace
   CAP/WINDOW-derived exhaustion/refill expectations in the two existing dispatch
   and router witnesses with literals; keep their observations and causal controls.
2. Add a small test-local controlled TCP/HTTP peer in `tests/binary.rs`. Bind an
   ephemeral loopback listener; parse real request bodies/headers, return complete
   Content-Length-framed HTTP-200 JSON replies, and record every request with its
   ID/method/header. Own each connection task. Use explicit channels/protocol
   milestones to hold/release initialization; queue input before release. No new
   production clock or input-consumption seam is needed.
3. Add table-driven malformed-initialization cases and ordinary-exchange cases.
   Keep the two defect fixtures otherwise valid. Use unique IDs and compare the
   whole JSON value, not just the ID or error. Preserve operation/notification
   distinctions and header observations. Drive successful explicit recovery and
   an admitted operation as a positive control after each failed initialization.
   An unversioned ping alone proves neither completed negative accounting nor
   recovered operation admission.
4. Keep serving/recording unexpected traffic until the child has drained and exited;
   then stop accepting and join owned peer exchanges before inspecting the trace.
   Assert exact response/trace contents after cleanup so ordinary assertion failures
   do not abandon a live child. Bound accept/body/read/write/stdout/exit/join waits;
   use existing kill/reap paths on awaitable failure and an owned abort/drop fallback
   for cancellation. Do not serialize normal bridge exchanges to simplify fixtures.
5. Execute focused tests with nonzero counts and all final-candidate gates. Inspect
   any failure before considering an existing-owner compatible correction. A failure
   never authorizes changing C6 forwarding or adding a review-engine bypass. Keep
   required unsafe/unavailable execution explicitly unmet rather than weakening a
   proof. Leave protected provider dispatch to the engine's subsequent review phase.

## Project structure and files touched

| Path | Planned treatment |
| --- | --- |
| `specs/independent-rate-proof/{spec,plan,tasks}.md` | Only tracked paths in chief's unsigned design commit. |
| `src/mcp.rs` | Inline test assertions only: numerical owner and exhausted dispatch witness. |
| `src/api.rs` | Existing router test setup/expectations only; preserve real owner and observers. |
| `tests/binary.rs` | Focused C6 subprocess cases and minimal local real-peer support using McpChild. |
| `tests/stdio.rs`, `tests/mcp_revisions.rs`, `src/mcp/batch_tests.rs` | Retain/execute lifecycle, malformed forwarding, revision and concurrency coverage; no planned edits. |
| `tests/binary/helper_reliability.rs`, `tests/binary/http_reliability.rs` | Retain existing bounded-helper proofs; extend only if a necessary helper change warrants it. |
| `tests/delivery_recipes.rs`, `tests/delivery_constitution.rs` | Execute unchanged route/constitution proofs. |
| `.forge/evidence/`, phase result path | Ignored receipts via permitted workspace tool; never staged. |
| Historical specs, decisions, runtime owners, Cargo files, bundles/adapters/agents, shims and served descriptions | Preserve; verify applicable contracts and proposal statuses without edits. |

## Data model

No runtime or persisted data changes. Test-owned state consists of a monotonic
instant, literal expected count/interval, two malformed reply values, unique request
IDs and a per-test complete peer trace. Retain one ToolRate Arc per production
router, immutable revision per exchange and the bridge-owned initialization fence.
Use existing receipt formats for command, commit/tree, exit and binary digest;
unknowns remain unknown rather than defaulting to a pass.

## Contracts

Rate: exactly 1024 attempts in the existing fixed monotonic one-second window,
charged before staging/effects; notifications and March members count. Refusals do
not move the window. Control/discovery remain available. Supplied instants prove
arithmetic, real router connections prove ownership, and retained concurrent tests
prove scheduling; none replaces the others.

C6: the same complete JSON value is forwarded, including both result/error when
present. Valid installation metadata and a valid envelope are separately required
for revision installation. Modern initialization failure keeps operations fenced;
explicit valid recovery restores admission. No whitespace fidelity, SSE/session,
fairness, retry, durability or full-conformance contract is added.

## Quickstart and validation

Use the permitted workspace tool. Check clean candidate identity, evidence manifests
and shim hashes. Bind the absolute `.forge/bin/brokkr` executable to tooling.json
and the spec's digest before using it for self and mixed-opus-terra compilation.
The final hermetic verifier uses its dispatched pinned scripts, not mutable shims.

Focused diagnostics (all intended filters must match tests):

```bash
cargo test --locked --lib tool_rate_uses_exact_monotonic_windows
cargo test --locked --lib exhausted_tool_rate_refuses_before_effects_and_keeps_control_available
cargo test --locked --lib the_router_owns_one_tool_rate_across_connections_revisions_and_members
cargo test --locked --lib revision_admission_has_no_transient_mutations_or_waiters
cargo test --locked --test binary
cargo test --locked --test stdio
cargo test --locked --test mcp_revisions
cargo test --locked --test delivery_recipes
cargo test --locked --test delivery_constitution
```

Also run the full authoritative [house inventory](../../docs/house-rules.md#gates--part-of-every-change),
not a reduced copy: locked clippy/tests, format, pinned compile, exact coverage and
selftest, publish/seal proofs and licenses, plus pinned design-check/selftest,
authorized delivery proof, shim integrity and fail-closed native-tls absence. Keep
required CI proofs with CI. Record actual outcomes and final candidate/executable
bindings; later tracked changes require fresh verification and review. Chief's
structural precheck cannot substitute for the sequence's pinned final check.

No standalone eval launch without real host-wide nonoverlap through cleanup and
exact executable binding; otherwise record unavailable/not run. No private lock,
provider probe or attack reproduction is part of this design. The inherited HIGH
remains blocking even if every application test passes.

## Risks

| Risk | Mitigation and retained cost |
| --- | --- |
| Oracle follows incorrect production values | Use literal counts and boundary instants in owner and contract-bearing setup; deliberate duplication is required. |
| Partial refill escapes a one-call boundary assertion | Exhaust the full second window and assert the next refusal at the same instant. |
| In-process fixture is mislabeled binary proof | Use `binary()`/CARGO_BIN_EXE_swarmail with actual child pipes and real TCP; record the executed artifact binding. |
| Negative trace checked before concurrent effects arrive | Observe EOF/drain/exit, join peer exchanges, then check complete trace; answer unexpected traffic and include recovery controls. |
| Hanging child or orphaned peer invalidates the suite | Reuse bounded child helpers, defer assertions until cleanup, bound all peer waits, own join/abort fallback; stderr stays nonblocking. |
| New helper obscures existing concurrency | Keep peer support local/minimal and preserve concurrent wait/seed/ping and batch witnesses unchanged. |
| Runtime deviations silently become repair scope | C6 forwarding and OPTIONS mismatch remain; no new validator/runtime abstraction or ADR without reconciled semantic need. |
| Green tests are mistaken for trustworthy protected review | Preserve HIGH security-relevant hold and require independently admissible resolution; a roster or completed panel cannot clear it. |
| Evidence outlives candidate identity | Freeze final commit/tree and executable digests, rerun after tracked edits, never promote historical receipts. |
| Local test isolation is mistaken for host-wide eval exclusion | Leave full eval unavailable until actual coordination and cleanup binding exist; retain #151/client/CI/operator obligations. |
