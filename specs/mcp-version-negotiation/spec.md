# MCP version negotiation — operator-chosen deviations

Date: 2026-10-26. Epic #164, issue #152.
Feature slug: `mcp-version-negotiation` (continue the existing feature).
Run: `epic-164-issue-152-reframe-the-m-a4a37e5f`.
Inspected implementation base: `9c0c9d4`; intake commit: `3cadc83`.
Metric: client compatibility; version negotiation is not full revision conformance.

Decision: 0080 — [MCP source reconciliation](../../docs/decisions/0080-mcp-source-reconciliation.md) (proposed, revised in place).

## Feature and rationale

Describe the existing negotiation of 2025-03-26, 2025-06-18 and 2025-11-25,
with batching for 2025-03-26 only. Full revision conformance is not claimed.
Retain C1–C6 as documented deviations by the operator's explicit 2026-10-26
choice. Correct attribution, dispositions and user-facing claims without changing
runtime behavior. Practical compatibility is determined by #151's wrappers and
the epic's exit client matrix; a negotiated date alone cannot establish it.

The previous source-reconciliation run stopped at REVIEW-RESIDUAL-UNSAFE with
spec_defect true and no security residual. Its independent reviewer correctly
identified C1's excessive attribution: the older decisions do not mandate HTTP
200 for response-shaped objects. The operator subsequently chose retention.
That choice resolves this framing commission, not the unmet source obligations.
It neither accepts 0079/0080 nor proves client compatibility.

This design supersedes the previous spec/plan/tasks' prospective repair commands.
Use [operator-ruling-intake.md](operator-ruling-intake.md) for the current intake;
the requested `.brokkr/state/tasks/` is absent. The earlier
[source intake](source-reconciliation-intake.md), [intake](intake.md),
[implementation receipt](implementation.md) and dated clarification entries are
historical evidence. The ledger/public-description alignment is recorded in the
[current implementation receipt](implementation.md#operator-directed-reframing--2026-10-26);
their pre-design wording must not override this commission.

## Authority and provenance

The supplied packet README and operator-rulings were read and verified against
SHA256SUMS. All 119 packet entries verify. Required digests:

- README: `ca63fae88ee143f0f1b11b828130cb54da58c887ac8ba9a5eaea1df09a8cc952`.
- Operator rulings: `c8fbb03e09d73ec68e215edebfb3aa6b1e5062b16869d4a36c614aefe2c6cbf0`.
- Candidate engine: `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.

Packet locator: `.forge/epic192/issue152/`. Its README prior-runs heading is empty;
`review-998279d4.md` supplies the relevant review. These are evidence locators,
not tracked hyperlinks. The operator-rulings '#152 conformance' row of 2026-10-26
is the authority for retention. Read no journal, sibling tree, GitHub or CI service.
The source manifest below retains official snapshots at upstream commit
`dae694f61a31f7a3ec2af3aff709e11f70f61e48`.

Accepted [0029 §2](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md),
[0044 §1](../../docs/decisions/0044-compat-validation.md),
[0047 §3 and its dated ID amendment](../../docs/decisions/0047-storage-rpc.md), and
[0039 §§2/6](../../docs/decisions/0039-stdio-concurrency.md) explain the retained
contracts. They do not compel every detail of all six deviations. Preserve their
accepted bodies/statuses. [0079](../../docs/decisions/0079-mcp-version-negotiation.md)
and 0080 stay proposed, including index entries. Updating existing proposed 0080
is specifically commissioned; this wording correction creates no new runtime
contract or ruling and does not allocate 0081.

## Synthesis and explicit rulings

Both panel positions were read in full. They agree on the operator's disposition;
there is no remaining runtime choice to vote on. Resolve their scope pressures:

| Question | Adopt, reject or reconcile | Cost accepted |
| --- | --- | --- |
| Documentation size | Adopt simplicity's existing files and one detailed register in reconciliation.md; adopt robustness's complete removal of contradictory prospective instructions. Reject a disclaimer-only patch. | More than a README edit, but no second independent policy registry or new documentation framework. |
| C1 status | Adopt both members' attribution correction and operator-directed retention. Reject the old HTTP-400/empty-body repair and the claim that any status repair necessarily contradicts accepted rulings. | The known HTTP 200 transport deviation remains; normative conformance is not earned by the ruling. |
| Proof depth | Adopt robustness's actor/source-strength/per-revision distinctions and simplicity's existing W1–W9 witnesses. Reject a fresh source audit, invented behavioral red/green repair or new runtime machinery. | Manual source-grounded review remains necessary; string assertions alone cannot prove correct attribution. |
| Failure analysis | Preserve robustness's guarantee and failure-boundary analysis as constraints, not new SMTP/webhook/TLS work. Simplicity's bounded scope wins over expanding the acceptance matrix. | Existing uncertain outcomes, best-effort delivery and shared-budget limits remain. |
| Governance | Reuse proposed 0080 and the existing feature directory, as both members recommend. Reject another ADR, decision acceptance or rewriting accepted history. | Existing proposal debt and the external client receipts remain owed. |

## Required contract and deviation register

Preserve existing revision selection, March-only batching, annotations, headers,
initialization fences, ID/error taxonomy, rate limit, transport bounds and tool
semantics. Do not add the 2026 revision or #153 structured outputs. No executable
Rust, request/response schema, dependency, lockfile, recipe, gate or coverage-policy
change is authorized. Static descriptions and narrowly scoped documentation
test assertions/comments may change; behavioral assertions remain intact.

Use [reconciliation.md's register](reconciliation.md#exact-conflict-register) as
the single detailed record, keeping that anchor, source hashes, clause/schema
rows and W1–W9 witness identifiers. Revise its current dispositions to
operator-chosen deviations; an unresolved source mismatch must not become
'conforms'. Spec and 0080 reference that register rather than repeat it.
The register must have distinct 2025-03-26, 2025-06-18 and 2025-11-25 columns,
plus source obligation/strength/actor, disposition, witness and compatibility
cost. Explicit 'same as March/June' is allowed only where behavior is identical.
Its required observations are:

- **C1 — response disposition:** all three revisions return HTTP 200 with
  -32600 `missing method` and carried valid IDs for unsolicited success/error
  objects. March response-only and mixed batches retain per-member replies;
  June and November arrays instead receive the whole-array, pre-dispatch
  -32600/null refusal. All three Transports/Sending Messages item 4 requires
  empty 202 acceptance or HTTP-error rejection. 0029 pins RPC envelopes, 0044's
  explicit 200 applies to non-objects, and 0047 pins notification classification;
  none compels 200 for response-shaped objects. The independent review's
  HTTP-400-with-existing-envelope observation corrects that attribution; it is
  not a repair commission. The operator retains the status and its mismatch.
- **C2 — IDs:** all three retain string/number/null IDs, fractional values and
  owned null/fractional-ID output mismatches. March admitted batches preserve
  duplicate/null slots; June/November arrays refuse instead. March/June error
  schemas require a RequestId-shaped ID; November permits absence, not explicit
  null. Separate sender integer/non-null/uniqueness duties, receiver tolerance,
  TypeScript number versus generated integer, and owned output obligations.
  No tightening, renumbering or ID-history cache follows.
- **C3 — bridge HTTP duties:** all three advertise JSON/SSE Accept tokens but
  consume identity JSON/empty responses only; external SSE and applicable
  session-ID/404 lifecycle duties remain unsupported. The paired server stays
  stateless JSON, with GET 405 for valid headers. That pairing does not discharge
  the general HTTP client's JSON/SSE MUST or external session duties.
- **C4 — tolerated senders:** all three preserve legacy missing jsonrpc/client
  fields and currently tolerated invalid envelopes/params, including forwarding.
  March tolerates batched initialize despite its sender prohibition; June and
  November arrays refuse before dispatch. Sender shape/lifecycle duties are
  distinct from receiver admission obligations. These are compatibility fixtures,
  not conforming senders or a promise to admit every invalid tool argument.
- **C5 — error taxonomy:** all three retain unknown-tool -32601, staging -32602
  and the wait isError split. Unknown-tool -32602 is an example, not a mandated
  code. Preserve the example/SHOULD distinction and November's additional
  validation/execution versus protocol-error explanation. Do not invent a MUST
  violation or erase this deliberate departure from the register.
- **C6 — stdout:** all three can forward complete JSON values with malformed
  MCP envelopes (missing required fields or simultaneous result/error). Initialize
  metadata validation controls state installation only. Valid-MCP-stdout duties
  remain unmet for malformed peers; complete JSON is not necessarily a valid MCP
  message. This does not claim syntactically invalid JSON is forwarded.

Public wording must name all three negotiated dates, March-only batching, the
absence of a full revision conformance claim, and operator-chosen documented
C1–C6 deviations, with a tracked documentation pointer. Keep descriptions short;
do not copy the register into README, llms.txt or OpenAPI. Preserve OpenAPI JSON
structure and values outside description fields, including statuses, codes,
headers, limits and schemas. No tracked hyperlink may target ignored `.forge/`.

## Acceptance Criteria

These criteria govern local implementation; [tasks.md](tasks.md) and the
[implementation receipt](implementation.md#operator-directed-reframing--2026-10-26)
record proof status. Original issue AC3 remains separately owned below.

1. **AC1 — One current commission.** Spec, plan, tasks, reconciliation and 0080
   consistently say version negotiation, no full revision conformance, and
   operator-chosen C1–C6 retention. A source/review-grounded inspection finds no
   current instruction to repair these deviations or wait for acceptance before
   retaining C1. Dated previous-run receipts remain labeled historical. C1's
   envelope/status distinction is explicit and no old ruling is falsely credited.
2. **AC2 — Six inspectable deviations.** The shared register contains exactly
   C1–C6, with all three revision columns and the observations/source strengths
   above. Source manifests, clause/schema rows and W1–W9 links remain intact.
   Compare with the supplied snapshots and review, not just keyword matches;
   existing response, ID, transport and stdio witnesses still assert the same wire.
3. **AC3 — Public description parity.** README, llms.txt and served OpenAPI carry
   the bounded claim and tracked pointer. `revision_served_contract` asserts the
   served wording. JSON parses and compares equal to base 9c0c9d4 outside
   description values. No documented status, code, header, cap or schema changes.
4. **AC4 — Preservation and governance.** Review the cumulative diff from
   9c0c9d4: only documentation/static descriptions and scoped documentation tests
   or comments change. Behavioral assertions and executable runtime stay intact.
   Both decisions and index entries remain proposed; accepted bodies/statuses,
   frozen guarantees and ingest extraction are unchanged. No dependency, gate,
   routing or coverage-policy change; no tracked ignored-evidence links.
5. **AC5 — Candidate-bound local proof.** Every house gate and the supplemental
   proofs in plan.md pass on the committed candidate, with commit/tree, command,
   exit and log receipts. Verify the candidate-local engine digest; run exact
   coverage plus selftest, requiring zero missed lines, branches and functions.
   Previous-run receipts are history, not evidence of this candidate's execution.
6. **AC6 — Honest outstanding work.** The four actual-client rows stay explicitly
   operator-owned and owed. Preserve the #151 baseline merge hold and CI-owned
   platform/network proofs. No decision acceptance, issue closure or compatibility
   completion is inferred from documentation checks or local protocol tests.

## External obligations retained

Original issue AC3 (distinct from this design's local AC3) remains operator-owned.
Receipts must record client/version, candidate commit, transport, selected
revision, initialize/list/call results, evidence location and verdict.

| Client | Receipt | Owner/status |
| --- | --- | --- |
| Claude Code | Not supplied | Operator — owed |
| Codex | Not supplied | Operator — owed |
| MCP Inspector | Not supplied | Operator — owed |
| Claude Desktop (fourth client) | Not supplied | Operator — owed |

#151's wrappers and the epic exit matrix determine practical impact. CI owns
macOS, chart/history, Docker smoke, network-backed RustSec and real port-80 jobs.
Operator owns live settings, decision acceptance, sealing, #151's baseline and
merge; the PR stays unmerged until the baseline exists. No local macOS,
namespace/network proof, paid trial or live GitHub read is required. No push,
merge, tag, publication, issue closure or decision acceptance is authorized.

## Dated clarification — 2026-10-26 source reconciliation (historical)

The earlier implementation clarification remains in implementation.md: its test
mapping, existing observers, strict empty-202 acknowledgment and failed-negotiation
array refusal remain historical evidence. The earlier completed task lists refer
to those runs, not this reframing. Local source provisioning superseded the older
missing-source task. The earlier rate repair is already implemented: one shared
1024-attempt/second monotonic tools/call budget, before staging/effects, with
control methods available. No rate repair or new behavioral test is commissioned.
The previous review's OPTIONS and rate-plumbing findings stay recorded in its
packet; this bounded wording task neither repairs them nor claims them discharged.

## Dated clarification — 2026-10-26 operator disposition

The later operator ruling supersedes the prospective instruction to fix every
compatible deviation and C1's deferred HTTP-400 proposal. C1–C6 remain explicit
source differences by operator choice. No acceptance is inferred for 0079/0080.

## Source manifest

Paths below are relative to the packet's `mcp-spec/`. Each digest is copied from
verified local bytes; provenance is the upstream commit above. This is a manifest,
not a claim that every clause has already been audited by the design seat.

```text
86c6929d37272f1ac946d547a76742d1de0665b9cf6b64b879eabad3e4a1fd0d  2025-03-26/spec/basic/index.mdx
271f4c9751583fa5d8afe3efb09da5d622707664cd2eb764b7ec5713fb161b15  2025-03-26/spec/basic/lifecycle.mdx
320118fe48117b83cec097bfa1923258826f5e21abde7929b9dff0014236dc57  2025-03-26/spec/basic/transports.mdx
0a36ed8c4506483fba6175ee3ecea8d9140b18b7b3d8b4355aca14e9227cadfb  2025-03-26/spec/server/tools.mdx
bdc457208cb1b49061f3fbbae24144b7dbae282c746d82aa689ddd98329743e7  2025-03-26/schema/schema.ts
e720669548c8100a4282c49e580efd6ddf7f28899ea786fc8db251dbdb356131  2025-03-26/schema/schema.json
c1dc3ce16b22a9bbcf330da334b3b6bf72312c81a95ec42b9bd94460efa86087  2025-06-18/spec/basic/index.mdx
9800e66e16cb0b71c339f6007fd14f6d17cc48741d4a8d381cc38f916bb6d05e  2025-06-18/spec/basic/lifecycle.mdx
df1217279334b6f3af8bb457191884ba831dce2c5389d7a0556ba920270ca902  2025-06-18/spec/basic/transports.mdx
6c0bc03b6579eef309a4a81660a707df20360d011e464c1b74eae56f96e6417e  2025-06-18/spec/server/tools.mdx
7d7f4cc6f3adb76e3fd5e1eee6bab8b1e183cfaf719da66d2aa5661737ca9539  2025-06-18/schema/schema.ts
af845e7e5b9d27107d1690f0936022546177a1403e63ffb11470135b296a2e01  2025-06-18/schema/schema.json
bd275064995d6e36dbb51c059be97e81c3eb7ceafc932e0276a7fc0a84c30fa4  2025-11-25/spec/basic/index.mdx
45a6e8b7fb8c96e7b9ba1b0a3c727e8451c1e55bf56bb62f3ab63fddc365b919  2025-11-25/spec/basic/lifecycle.mdx
a247fdbb3cc25c805ef43124db18d9b60a56669b3e65bd163dffb76f4129dfc0  2025-11-25/spec/basic/transports.mdx
39e56ad4f3d1ff1cb28ee62283e02947cd97db8aa6190782d629f4562a0f354c  2025-11-25/spec/server/tools.mdx
e74b56e73b2e37bdb595f74ba22e428ad7f07aa3519355ba661d681298ed38ac  2025-11-25/schema/schema.ts
268a5f82ba70fd7e4b6dc4aa1e64f116f74b4d0edcb69dc046829c79dd4e97e7  2025-11-25/schema/schema.json
```
