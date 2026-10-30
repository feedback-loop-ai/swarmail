# Implementation evidence — issue #152

Historical record of the earlier negotiation/source-reconciliation runs.
Current commission and C1–C6 disposition follow [spec.md](spec.md) and the
[operator-directed register](reconciliation.md#exact-conflict-register).
Earlier prospective repair/defer wording and gate receipts do not authorize
runtime changes or prove this reframing candidate.

Date: 2026-10-26. Metric: **compatibility — the client compatibility matrix**.
This records local delivery of the proposed [0079](../../docs/decisions/0079-mcp-version-negotiation.md)
contract, without accepting that decision or claiming the external client matrix.

## Implemented behavior and ownership

`src/mcp.rs` owns three finite revisions, proposal staging, header selection,
revision-aware envelope classification and catalogue projection. `src/api.rs`
passes request headers after its existing bounded extraction. Parsing still
precedes version-header validation; Host/Origin guards precede both. HTTP has no
shared negotiation state. A legacy batch initializes only legacy members.

`src/stdio.rs` owns the bridge lifecycle and immutable exchange metadata.
Initialize fences future admission without draining earlier parked work first.
Only a matching successful supported response advances negotiation; failed modern
intent suppresses notifications and refuses operations, including legacy-shaped
arrays, until explicit retry. Modern initialization requires a strictly empty
HTTP 202 acknowledgment: even a whitespace body leaves operations disabled.
Ordinary whitespace-body forwarding retains its existing behavior. Steady-state
operations remain concurrent. Modern arrays refuse before dialing or silence;
size errors on modern arrays keep one null-ID reply.

No production storage, SMTP, extraction, webhook, dependency, gate, recipe or
accepted historical decision changed. 0079 is indexed as proposed. The packet
and engine digests were checked locally; the packet lists no earlier outcomes.

## Executed proof mapping

| Obligation | Executed tests |
| --- | --- |
| AC1: negotiation and legacy batch carve-out | `tests/mcp_revisions.rs::revision_negotiation_matrix`; retained legacy MCP tests |
| AC2: headers, parse/size precedence, stateless isolation | `tests/mcp_revisions.rs::revision_headers_and_isolation`; owner non-text-header test; `tests/stdio.rs::revision_exchange_fences_and_headers_follow_the_result` |
| AC3: lifecycle, queued lines, explicit recovery | `tests/mcp_revisions.rs::bridge_revision_lifecycle`; `tests/stdio.rs::failed_modern_negotiation_never_releases_queued_effects_as_legacy`, `initialized_failure_stays_fenced_and_modern_arrays_never_dial`; measured lifecycle owner test |
| AC3: modern concurrency | `tests/mcp_revisions.rs::bridge_modern_wait_allows_seed`, sequenced by the real labeled await gauge |
| AC4: all array shapes, zero effects, no dial, oversize | `tests/mcp_revisions.rs::modern_batches_refuse_before_effects`; the initialized failure/no-dial test above; `mcp::tests::revision_admission_has_no_transient_mutations_or_waiters` |
| AC5: preserved RPC taxonomy, IDs, result/error split | `tests/mcp_revisions.rs::revision_rpc_edges`; unchanged legacy MCP/stdio batch, cap, concurrency, notification, framing and transport proofs |
| AC6: every descriptor, unchanged schemas, no #153 fields | `tests/mcp_revisions.rs::revision_tool_annotations`; result-shape checks in `revision_rpc_edges` |
| AC7: immediate SMTP visibility and subscribe-before-scan | `tests/mcp_revisions.rs::revision_smtp_visibility`; the admission owner test installs the existing post-subscribe hook and inserts in the exact pre-scan window for both modern revisions |
| AC8: local delivery gates | Full house inventory plus native-tls absence, design selftest and delivery-boundary proof; candidate-bound command receipts are retained in ignored local evidence and the final implementation result |

The admission owner test installs the existing mutation and subscription hooks,
proves zero transient entries for modern arrays, and exercises legacy positive
controls. Wire tests compare populated mail and chaos state as well. Controlled
upstream-response tests use actual TCP exchanges and an input-consumption
observer; the application and binary tests run against real servers.

Two temporary regressions were executed and restored:

- Reverting revision-aware classification to legacy classification made the
  notification-array witness fail with `no reply line` (exit 101).
- Allowing a mismatched initialize-response ID to install its revision made the
  lifecycle witness fail with `failed initialize cannot install a revision`
  (exit 101).

Both exact focused tests passed after restoration. The full locked workspace
suite also passed. The first instrumented run identified the required README
suite-inventory row for the new test binary; that row was added before final
verification. The next exact report found only the impossible empty completion
arm behind the bridge's nonempty polling guard; that redundant arm was removed,
retaining the guard's invariant explicitly. Final success requires exact
zero-missed lines, branches and
functions, the coverage selftest and every other local gate on the committed
candidate; receipts never substitute a rounded percentage or an external claim.

## External evidence remains unproven

Missing normative inputs remain exactly the versioned 2025-06-18 and 2025-11-25
Basic Lifecycle, Basic Transports, JSON-RPC message/schema constraints and Server
Tools/ToolAnnotations snapshots named in [the spec](spec.md). No network source
was fetched and no November-only source reconciliation is asserted.

Claude Code, Codex, MCP Inspector and Claude Desktop receipts are not supplied.
Their exact client versions, candidate identity, negotiated revisions, transports
and initialize/list/call outcomes remain CI/operator-owned and unproven. Local
protocol tests do not make these four rows green. macOS, real port 80,
RustSec/network, Helm and Docker remain owned by CI as the charter states.
The operator owns #151's baseline, the merge hold and branch sealing. The PR
stays unmerged until that baseline exists; this run neither needs nor runs it.
No push, merge, tag, publication, issue closure or decision acceptance occurred.


## Source reconciliation run 998279d4 — 2026-10-26

The earlier sections are historical receipts for run 95b27901. Their missing-source
statement is superseded: all 118 packet hashes and the candidate-local engine
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f verify.
Official sources are pinned at dae694f61a31f7a3ec2af3aff709e11f70f61e48.
[reconciliation.md](reconciliation.md) records 378 normative prose rows, all
relevant TS/JSON symbols, individual security duties and exact remaining conflicts.

Behavioral tests failed before each of five compatible repairs: version admission
on non-POST methods; required initialize result metadata before state installation;
modern wait wording; served OpenAPI; and actual invocation rate limiting. Named
commands, baseline/test-patch hashes, assertion failures and same-test successes
are in the ledger. Added HTTP/binary rows prove omitted version and excluded
2026-07-28, lost/truncated recovery, per-method invalid headers and twelve schemas.
Transient mutation/subscription counters prove rate refusals cause no effects,
with live March controls. Output tests preserve untrusted HTML/control/envelope-like
strings as inert typed text without exposing raw mail or adding RPC/content blocks.

C1/C2/C3/C4/C6 remain accepted-contract conflicts; C5 is a justified example/SHOULD
departure. This is local reconciliation, not full revision conformance. 0079/0080
stay proposed. Actual Claude Code, Codex, MCP Inspector and Claude Desktop receipts
remain operator-owned and owed. CI platform/network/chart/Docker/port-80 evidence,
operator acceptance/sealing and #151 baseline/merge hold remain unchanged.

Implementation checkpoint is ready for committed-candidate gate execution; final
local results are appended below only after every required proof has run.


## Final local gate receipt — source reconciliation

Executed offline on unsigned implementation commit
`1292151ff774a13b3be91b5bb5eb52ba62a6c354`, tree
`32c3d562d7cbbafe257a8f63ec9f45ffcd17a6f5`. Every command below exited 0.
Run-local `final-<gate>.json` records commit/tree, exact command, exit and elapsed
time; `final-<gate>.log` holds output under
`.forge/issue152-reconciliation-proof/`. Repeated final-candidate runs replace
those run-local records with their actual HEAD/tree, without changing this
historical implementation receipt. A tool response timed out at 300 seconds
during the combined test/coverage call; the command continued and its persisted
exit records independently confirmed both successes.

| Proof | Executed command |
| --- | --- |
| Format | cargo fmt --all -- --check |
| Lint | cargo clippy --workspace --all-targets --locked -- -D warnings |
| Workspace tests | cargo test --workspace --locked |
| Pinned constitution | .forge/bin/brokkr compile --bundle bundles/self |
| Exact coverage | bash scripts/coverage-gate.sh |
| Coverage selftest | bash scripts/coverage-gate.sh selftest |
| Publish guard | bash scripts/publish-guard-test.sh |
| Keyless sealing proof | bash scripts/seal-branch-test.sh |
| Licenses | cargo deny check licenses |
| Design selftest | bash bundles/self/scripts/design-check.sh selftest |
| Delivery proof | bash bundles/self/scripts/delivery-proof-test.sh |
| Shim integrity | sha256sum --check --strict bundles/self/scripts/shims.sha256 |

Engine digest matched the prescribed ab935558…a885f before compile. The locked
`cargo tree --locked -i native-tls` absence proof accepted only the specific
“did not match any packages” error and would refuse both package presence and
other command failures. Bundle compilation emitted no policy-sweep warning.
Exact coverage is **15191/15191 lines, 882/882 branches, 1634/1634 functions**;
zero missed, no rounding. Report: `target/coverage/run.akuA3g/` and the gate's
normal `target/coverage/` summary/report paths. The coverage selftest passed.

The receipt commit only completes documentation/checklists, fixes the Markdown
index-row separation and adds an explicit March response-only-batch witness.
Production source is identical to the verified implementation. The final
candidate reruns all house gates and supplemental proofs, with their HEAD/tree
recorded in the run-local JSON files. T1–T11 are locally complete; E1–E3 and
C1/C2/C3/C4/C6 remain open by their named owners/accepted authority. This does
not claim full MCP conformance, real-client compatibility or decision acceptance.
No push, merge, tag, publication or issue closure occurred.


## Operator-directed reframing — 2026-10-26

Run `epic-164-issue-152-reframe-the-m-a4a37e5f` follows the operator's
2026-10-26 '#152 conformance' ruling. The [shared register](reconciliation.md#exact-conflict-register)
now records C1–C6 as operator-chosen deviations, with separate observations for
March, June and November, source strength/actor, contract attribution, witness
and compatibility cost. C1's retained HTTP 200 is explicitly the operator's
choice, not a status required by 0029/0044/0047. The source mismatch remains.

README, llms.txt and served OpenAPI describe three-revision negotiation,
March-only batches and retained deviations, without full revision conformance.
The served-contract test checks the wording and tracked register pointer;
the response-disposition test changes only its attribution comment. Runtime
Rust and all behavioral assertions remain unchanged from 9c0c9d4. OpenAPI parses
and is structurally identical outside description values. The source manifests,
clause/schema source references and W1–W9 identifiers remain intact; W9 now
states the prior review's shared-budget wire-proof limitation explicitly.
Earlier intakes and implementation receipts are labeled historical.

The full 119-entry packet and pinned engine digest verify. Current proof records
belong under `.forge/issue152-reframe-proof/` (ignored evidence locator, not a
tracked hyperlink), separate from the hashed packet. Each gate receipt records
the executed command, candidate commit/tree, exit and log; this section does not
substitute historical logs for current execution. Final gate results are recorded
below for the implementation commit; final-candidate receipts bind the later
documentation commit separately.

Original issue AC3 is still **operator-owned and owed** for Claude Code, Codex,
MCP Inspector and Claude Desktop. Each live receipt needs client/version,
candidate, transport, revision, initialize/list/call results, location and verdict.
#151 wrappers and the epic exit matrix decide practical compatibility. CI owns
macOS, chart/history, Docker, RustSec and port-80 evidence. The #151 baseline
merge hold remains; 0079 and 0080 remain proposed. No decision acceptance or
issue closure follows from this local documentation implementation.


### Local gate receipt — operator-directed reframing

All 15 inventory/supplemental commands exited 0 offline on unsigned implementation
commit `e48e5e4d85431bc460a49c982de1b4c19460c911`, tree
`c5974c792250536f126cefb8de44d1f1e2848b03`. Receipts/logs are under
`.forge/issue152-reframe-proof/implementation-gates/`, with exact commands,
HEAD/tree, engine identity, exits and elapsed times. They cover fmt, locked
clippy/workspace tests, candidate-local brokkr compile, exact coverage and its
selftest, publish-guard/keyless-seal proofs, licenses, shim integrity, design
selftest, delivery proof, fail-closed native-tls absence and documentation
preservation. Pinned script owners ran after their dispatch shim hashes verified.
The engine matched `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`;
compile produced no policy-sweep warning. No gate or recipe changed.

Coverage: **15191/15191 lines, 882/882 branches, 1634/1634 functions**, literal
zero missed; selftest passed. Implementation-run coverage evidence is
`target/coverage/run.uvZnpr/`. The workspace response timed out at 300 seconds;
the command continued, and persisted per-command exits plus `done` proved the
entire inventory completed successfully. A timeout is not counted as a pass.

The isolated MCP revision suite passed all 12 tests and stdio all 38. The full
workspace/coverage runs repeat those witnesses. The documentation preservation
proof compares OpenAPI outside descriptions, runtime/behavioral-assertion scope,
all 378 normative source rows, schema source rows, source manifest, C1–C6 columns,
W1–W9 identifiers, proposal statuses and changed tracked links against 9c0c9d4.
Manual source/review inspection supplies the actor/strength/attribution judgment;
string checks alone do not establish conformance.

This receipt's commit changes documentation/checklists only. The final-candidate
run records every inventory/supplemental command again in
`.forge/issue152-reframe-proof/final-gates/`, bound to its actual final HEAD/tree;
those receipts, not a relabeled implementation log, are the final proof. The
result is written only after every final exit is zero and the tracked tree is
clean. Local tasks T1–T7 are complete; original issue AC3 and E1–E3 remain owed
by their named owners. Runtime behavior, both proposed statuses and #151's
unmerged-PR hold are preserved.
