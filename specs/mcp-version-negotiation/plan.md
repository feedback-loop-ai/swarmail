# Implementation plan — operator-directed MCP framing

Implements [spec.md](spec.md) and revised proposed
[0080](../../docs/decisions/0080-mcp-source-reconciliation.md).

## Technical context

One Rust crate, existing Tokio/Axum/serde_json owners and real-protocol tests.
Negotiation already exists; this implementation changes documentation, static
OpenAPI descriptions and scoped documentation assertions/comments only.
No new dependencies, modules, flags, stores or protocol machinery. The requested
`.specify/templates/plan-template.md` is absent; its required sections follow.

## Constitution check

Preserve synchronous insert before SMTP 250, subscribe-before-scan, oldest-only
pruning and extraction at ingest (0001–0003/0005). Preserve existing refusals,
Host/Origin checks, limits, dispatch classification, concurrency and initialization
fences. The operator's retention ruling is binding; neither proposal is accepted.
No new contract or weakened guarantee is authorized. Keep current bundles/routing,
protected review/security hold, exact coverage, engine pin and dependency policy.

The design commit changed exactly spec.md, plan.md, tasks.md and existing 0080.
The [implementation receipt](implementation.md#operator-directed-reframing--2026-10-26)
records the ledger/public-description alignment and its candidate-bound proofs.
Historical receipts are retained, never substituted for current gate execution.

## Research and implementation approach

1. Reverify packet SHA256SUMS, engine digest and current framing. Use the prior
   source ledger and review; do not restart the completed clause audit. The
   current ruling supersedes old prospective repair commands, not source MUSTs.
2. Revise the existing reconciliation register in place, retaining its anchor.
   Add explicit March/June/November observations, obligation strength/actor,
   witness, operator disposition and cost for C1–C6. Preserve all source hashes,
   clause/schema rows and W1–W9 identifiers. Update 'unresolved' explanations to
   mean retained source mismatch, never pending permission to keep behavior.
3. Align the ledger introduction, current spec/plan/tasks and proposed 0080.
   Preserve dated implementation notes as history. Review 0079, its index entry,
   earlier feature receipts/intakes and dated pointers for current contradictions;
   correct only current summaries/pointers, labeling earlier claims historical.
   Never rewrite an accepted ruling's historical body or status.
4. Align README, llms.txt and OpenAPI descriptions with the short public claim in
   the spec and a tracked register pointer. Keep schema/structure and observable
   statuses, codes, headers, limits and tool semantics unchanged.
5. Correct the C1 attribution comment in revision_response_disposition while
   keeping every assertion. Extend revision_served_contract only for served
   negotiation/non-conformance/retention wording and its documentation pointer.
   Reuse all behavioral witnesses. No artificial behavioral red/green repairs,
   new test framework, broad linter, mocks or production seam is justified.
6. Inspect the full diff from 9c0c9d4 for scope and contradictions, then commit
   implementation unsigned. Run all gates on that commit and retain receipts.
   Repair documentation/proof failures within scope; never hide a failing gate.

## Project structure and files touched

| Path | Work |
| --- | --- |
| specs/mcp-version-negotiation/{spec,plan,tasks}.md | Current authoritative design; implementation records only proven task completion. |
| specs/mcp-version-negotiation/reconciliation.md | Sole detailed per-revision deviation register, corrected dispositions and unchanged source/witness ledger. |
| specs/mcp-version-negotiation/implementation.md; earlier intakes | Preserve dated receipts; narrowly clarify historical versus current claims when needed. |
| docs/decisions/0080-mcp-source-reconciliation.md | Record operator retention and correct C1 authority; proposed throughout. |
| docs/decisions/0079-mcp-version-negotiation.md; docs/decisions/README.md | Correct current summary/pointers if needed; keep both proposal statuses. |
| docs/decisions/0029-*.md, 0039-*.md, 0044-*.md, 0047-*.md | Only dated pointer wording if contradictory; accepted historical bodies/statuses unchanged. |
| README.md; src/static/llms.txt; src/static/openapi.json | Bounded public claim and tracked pointer; OpenAPI descriptions only. |
| tests/mcp_revisions.rs | C1 comment and served-document assertions only, preserving behavioral assertions. |

No executable edits to src/mcp.rs, api.rs, stdio.rs, Store, persistence, SMTP,
extraction or webhooks. No Cargo, bundles, gate, license or deployment changes.
The prior review's OPTIONS/rate-plumbing findings are not reopened by this task.

## Data model

No runtime data model changes. Keep Revision::{March, June, November}, stateless
HTTP request selection, immutable per-exchange bridge revision, existing lifecycle
owner and shared tool budget. The only new organization is documentation columns
within the existing register: C identifier, three observed revisions, source
strength/actor, authority distinction, operator disposition, cost and witness.
No independent registry, schema generator or per-revision policy file is added.

## Contracts and verification design

C1 remains HTTP 200/-32600; C2 retains accepted ID values; C3 keeps JSON/SSE Accept
and identity JSON parsing; C4 keeps current admission/forwarding; C5 preserves
error taxonomy; C6 keeps complete-value forwarding separate from state install.
Existing revision_response_disposition, revision_rpc_edges, transport/annotation
and stdio suites prove behavior. Their assertions must not be relaxed or flipped.

For OpenAPI, parse base and candidate JSON, recursively remove only `description`
values while retaining every other key/value and array order, and assert equality.
Additionally inspect changed descriptions to preserve their status/header/limit
claims. The served test proves delivery of the edited wording. Review each register
row against source/review evidence because wording assertions cannot prove source
strength or the distinction between sender duties and receiver tolerance.

No new behavioral tests are needed for unchanged execution. Existing tests remain
real HTTP/SMTP/binary and controlled TCP peers; no sleep-based synchronization,
calendar-bound fixtures, new seams or coverage exclusions are introduced.

## Quickstart and verification

Offline, installed tools only. Check packet and shim integrity before executing
pinned proof owners. Use `.forge/bin/brokkr`, whose SHA-256 is
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.

```bash
(cd .forge/epic192/issue152 && sha256sum --check --strict SHA256SUMS)
sha256sum .forge/bin/brokkr
sha256sum --check --strict bundles/self/scripts/shims.sha256
export CARGO_NET_OFFLINE=true
cargo test --locked --test mcp_revisions
cargo test --locked --test stdio
```

Run every command in the single
[house inventory](../../docs/house-rules.md#gates--part-of-every-change): fmt,
locked clippy/workspace tests, `.forge/bin/brokkr compile --bundle bundles/self`,
exact coverage and selftest, publish-guard proof, keyless seal proof and licenses.
Under 0050 run `bundles/self/scripts/coverage-gate.sh` (also `selftest`),
`publish-guard-test.sh` and `seal-branch-test.sh` after shim verification.
Supplement with pinned `design-check.sh selftest`, `delivery-proof-test.sh` and
fail-closed native-tls absence: `cargo tree --locked -i native-tls` must fail
specifically with 'did not match any packages'; any other Cargo error fails proof.
Compile must emit no policy-sweep warning. Coverage requires zero missed lines,
branches and functions, with its selftest passing independently.

Run every gate even after a failure. Store command, exit, log and candidate
commit/tree plus engine identity under a fresh ignored evidence directory; never
rewrite the hashed packet. Rerun affected proofs after fixes. Do not install,
change routing, weaken gates or infer external evidence from local successes.

The design seat's four-artifact commit and sequence check are historical.
Implementation commits its own work unsigned and records gates against that
committed candidate. Design-time gates do not prove the implementation.

## Risks

- **Contradictory commissions:** remove current repair/defer directions across
  owners, preserve old receipts as historical, and inspect all current pointers.
- **C1 over-attribution persists:** state exactly which envelope/non-object/
  classification rules the old decisions pin and identify operator choice for 200.
- **Retention is mislabeled conformance:** source obligation and observable cost
  remain explicit per revision; C5 stays an example/SHOULD departure, not a MUST.
- **Register copies drift:** one detailed register, stable anchor, short linked
  summaries, source hashes and W1–W9 references preserved.
- **Documentation edit changes schema or runtime:** structural JSON comparison,
  cumulative diff review and unchanged behavioral witnesses plus all house gates.
- **Known compatibility debt affects clients:** keep four real-client receipts
  owed; #151 wrappers/epic matrix decide impact; preserve the baseline merge hold.
- **Failure guarantees silently expand:** commit can precede a lost reply; webhook
  delivery is bounded best effort; slow stdout and shared budgets remain limits.
  No retry, rollback, exactly-once, durable outbox or general-client promise.
- **Local proof overclaims:** receipts bind to the committed candidate; CI/operator
  obligations stay external. Missing local tools/evidence are named, never faked.

## Dated clarification — 2026-10-26 (historical source-reconciliation run)

The following entries describe that earlier run only. Its 'new tasks' and contract
attribution refer to its then-current framing, superseded by this design; they
commission no repair now.


The earlier implementation reused store observers, required a truly empty modern
initialized acknowledgment and blocked arrays after failed modern negotiation.
Those changes and their dated receipt remain authoritative historical evidence in
implementation.md. Source availability has changed: local reconciliation replaces
the old external source-provisioning task. The new tasks are not marked complete
by the earlier run's passing gates.

### Implemented reconciliation — 2026-10-26 (historical)

The clause ledger is reconciliation.md (378 normative prose rows, per-revision
schema rows and separate security child duties). Repairs use existing owners:
non-POST header fallback in api.rs, required result metadata at Lifecycle::complete,
modern wait projection and served OpenAPI, plus ToolRate in mcp.rs. The rate is
one Arc per router inserted through the MCP route's Extension and shared by
request contexts; no AppState/Store/client-key registry or timer is added.
The fixed budget is 1024 attempts per monotonic second; refusal is isError
before effects, including notifications and March members. Owner tests drive
exact instants and live mutation/subscribe observers. Accepted response/ID/
forwarding/parser contracts remain explicit C1–C6 ledger dispositions.

## Dated clarification — 2026-10-26 operator disposition

The operator retains C1–C6. This plan replaces the earlier prospective source
repair approach with documentation alignment, unchanged runtime and honest
compatibility costs. 0079 and 0080 remain proposed.
