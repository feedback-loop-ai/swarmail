# MCP refusal proof follow-up

Date: 2026-10-28. Epic #164, issue #153. Design supplement to
[mcp-report-output](../mcp-report-output/spec.md); historical T8 stays unchanged.

## Feature and rationale

Complete the bounded assessment of T8 refusal evidence for modern structured
MCP output, strengthen concrete missing adapter assertions, and record each
remaining gap by evidence class. A refusal retains its existing taxonomy,
message, correlation and notification behavior without structured success data.
Four unavailable responses alone do not discharge all of T8.

This is a test-and-evidence change, not a semantic change to a frozen guarantee,
public contract or ruling. No new decision is declared. Proposed
[0079](../../docs/decisions/0079-mcp-version-negotiation.md),
[0080](../../docs/decisions/0080-mcp-source-reconciliation.md),
[0082](../../docs/decisions/0082-mcp-structured-output.md) and
[0105](../../docs/decisions/0105-mcp-report-output.md) retain their statuses and
historical text. Compatibility under [0045 §8](../../docs/decisions/0045-delivery-repairs.md)
is not acceptance. Integration owns provisional-number collisions.

The epic's tokens-per-success, invalid-argument/tool-misuse rates and
turns-per-task context remain operator measurement obligations. These protocol
proofs claim neither metric improvement nor actual-client compatibility.

## Evidence and framing

Starting candidate: `8d205d7d59a58c31c926db0df29f989ea43f6f29`, tree
`38adb6403a59b4b1bf13058865bf90e55c8ed696`. Its only delta from `393247c` is
44 lines in the existing deliverability owner harness. Parent run `5572d327`
verified `393247c2e22a0f418db24ec720a6935d6547fa20`, tree
`09ad1b537d693bb303cd2d1ba591572459b8abc8`, in event
`c58c3ab6-e724-4bcf-a133-180ff82660e6` and entered still-live protected review.
That event is not verification of the changed candidate.

Both run-local positions were read: `.brokkr/state/design/positions/simplicity.md`
and `robustness.md`. `.brokkr/state/tasks/` is absent; framing comes from the
supplied task and resolved intake result `d6b2f2dc-62b4-4ea2-8dcb-b24adffad14e`.
No missing intake file is manufactured. The issue packet at
`.forge/epic192/issue153` passed its directory-relative manifest. The root-relative
manifests for `.forge/evidence/prior-153`, `unavailable-wire-gates` and
`issue152-annotation-review` passed. These are run-local evidence identifiers,
not tracked Markdown links or substitutes for committed requirements.

Candidate-local receipts record passing gates and exact coverage
17372/17372 lines, 1202/1202 branches and 1781/1781 functions on `8d205d7`.
They are historical local execution evidence, not fresh gates for a later HEAD,
engine verification or protected reviewer results. Local baseline `3a2eef9`
proves only its recorded compatibility; actual protected #152 evidence is owed.

## Panel rulings and costs

1. Adopt simplicity's existing owners, real worker exhaustion and test-only
   scope. Reject new production fixtures, public switches, dependencies,
   schema frameworks, retries, second analysis reads and raised bounds.
   Cost: some process and router ownership claims remain explicitly unproved.
2. Adopt robustness's complete envelope expectations and explicit proof matrix
   over selected-field checks or citing a shared error helper. Require modern
   null/container cases and notification cases across both revisions.
   Simplicity's narrower reliance on neighboring notification tests loses here:
   its lower test cost leaves combinations inferred. Accept focused new tests,
   without expanding production behavior or historical golden captures.
3. Choose simplicity's honest in-process unavailable attribution over requiring a
   spawned-binary unavailable receipt for this supplement. The Held seam exists
   in a cfg(test) library owner; Binary lives in an integration test built against
   the production library. There is no current cross-boundary seam. Do not bridge
   that gap with a fixture-enabled executable or guessed target/debug path.
   Cost: unavailable across executable startup/CLI/pipes remains unproved, even
   though the actual bridge function uses real HTTP.
4. Retain the inherited LOW shared-router rate wire gap. The router creates its
   private limiter internally; the direct-dispatch test owns another context.
   Neither a replacement test router nor a 1025-request timed burst proves the
   production router's shared ownership deterministically. Robustness's preference
   for closing this row loses within the existing-seams scope. Cost: a per-request
   limiter regression can still escape these tests. Any later owner-seam proposal
   must be explicit; this supplement does not waive the gap.
5. Reconcile robustness's notification/read-barrier proposal with actual bridge
   concurrency: ping proves liveness and correlation, not completion of earlier
   concurrent tools. Observe a seed effect directly or finish EOF draining before
   counting effects and asserting the entire transcript. No sleep or assumed
   request ordering. Cost: explicit completion observations and cleanup.
6. Reject changing report rules to emit reserved Link/empty/extreme forms, broad
   webhook/store hardening and rewriting historical specs. Existing typed and
   subsystem witnesses retain their ownership; this work adds refusal evidence.

## T8 evidence ledger

The modern matrix means June `2025-06-18` and November `2025-11-25` separately.
HTTP means a real loopback listener. Binary means a spawned production stdio
executable. Bridge-function means actual `stdio::bridge` with in-memory streams
and real HTTP, in the owner test process. Existing witnesses below were inspected;
implementation must execute and record its final witnesses, not infer a pass.

| Obligation | Existing witness and evidence class | Required disposition |
| --- | --- | --- |
| Default/explicit March bytes | `structured::legacy_bytes`, `historical_twelve_tool_fixture_is_preserved`; HTTP and binary, independent raw captures | Retain raw comparison, exact case lengths/IDs and fixture hashes; no reserialization or golden regeneration |
| Missing argument ID, numeric ID, extra key | `structured::modern_all_tools_http_and_stdio`; modern HTTP/binary | Retain; extend with focused missing/null/malformed matrix |
| Null report ID; omitted/null/array/scalar arguments | Legacy tests only for some shapes; modern capture lacks these explicit cases | Add both modern revisions x HTTP/binary; exact correlated -32602 envelopes |
| Missing mail | Workflow plus `structured::report_stored_bytes_over_modern_wire`; modern HTTP/binary | Retain exact text-only tool refusal |
| Failed analysis | Workflow's directly inserted historical mail with malformed stored bytes; modern HTTP/binary | Retain restored-row attribution; do not call it SMTP or unavailable |
| Unavailable analysis | `deliverability::concurrency_tests::report_workers_do_not_hold_store_guards`; modern HTTP/bridge-function | Strengthen complete literal envelope; keep real workers and SMTP queryability/prune/delete proof; binary remains missing |
| Storage refusal | `structured::modern_storage_errors_and_revision_isolation`; real SQLite refusing triggers, modern HTTP/binary | Retain exact seed/delete/clear-inbox/clear-all refusal and effects checks |
| Invalid wait timeout | Same modern storage witness, null and 60001 | Strengthen to complete correlated tool-error envelopes |
| Shutdown | `structured::modern_parked_wait_shutdown_keeps_error_shape`; modern HTTP/binary; report worker/slot owner test separately | Retain observed active waits and exact shutdown refusal; normal timeout is a successful timeout arm |
| Rate | `mcp::tests::exhausted_tool_rate_refuses_before_effects_and_keeps_control_available`; direct dispatch, all revisions | Retain refusal/no-effects/no-subscription/silence/control/refill proof; shared production-router wire exhaustion remains LOW/unproved |
| Mixed revisions | Storage/isolation witness, HTTP alternating headers and distinct legacy/modern binaries | Retain no cross-exchange structured-output leakage |
| Notifications | June HTTP admitted seed plus legacy/lifecycle and direct-dispatch rate silence | Add June/November HTTP/binary seed and malformed-report silence; unavailable notification only HTTP/bridge-function |
| Envelope/bridge bounds | Revision/lifecycle suites and `structured::real_bridge_dual_output_size_cost` | Retain errors, correlation and continuation at unchanged 10 MiB cap; no replay or full-conformance claim |

## Acceptance Criteria

- **AC1 — Attributed ledger.** Every ledger row has a named executable witness,
  revision, transport/process class, expected envelope/effect, and final candidate
  command/exit evidence or an explicit missing-proof disposition. Historical T8,
  captures, decisions and prior findings remain byte-identical. No blanket
  statement that T8 is discharged while the binary/rate gaps remain.
- **AC2 — Modern argument refusals.** For both modern revisions over HTTP and
  binary stdio, independently test omitted arguments, empty object, `id:null`,
  numeric ID, null/array/scalar container and an extra key. Pin the complete
  JSON-RPC envelope with the sent request ID and -32602; literal messages are
  `missing id`, `id must be a string`, `arguments must be an object of named
  arguments`, and `unknown argument: extra` as applicable. No `result` or
  `structuredContent` occurs. Omitted request ID is separately a notification;
  explicit JSON-RPC `id:null` remains a correlated request.
- **AC3 — Real unavailable refusal.** Observe all JOBS worker-entry acknowledgments
  using Held before requesting another stored report. Keep REST 503, SMTP 250
  followed by queryability, oldest-only pruning and mutation progress. For each
  modern HTTP request assert status 200 and exact
  `{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"deliverability analysis unavailable"}],"isError":true}}`.
  Compare bridge-function output independently to that literal envelope, check
  negotiated revision, successful EOF/exit and exact response cardinality. No
  structured success, production fixture, weakened bound or binary overclaim.
- **AC4 — Existing refusal taxonomy.** Execute the named missing-mail, failed,
  storage, shutdown, legacy/raw, mixed-revision and bridge-cap witnesses. Null
  and oversized wait timeouts have full expected correlated `isError:true`
  envelopes, one text block and no structuredContent, using their existing
  literal sentences. Rate stays explicitly direct-dispatch evidence, with its
  effects/subscription/silence/control/refill assertions intact.
- **AC5 — Notifications and continuation.** On both modern revisions, HTTP
  notifications return 202 with zero body bytes; binary notification workflows
  yield no notification reply after EOF drain. Test admitted seed exactly once
  and malformed-report refusal without effects. Require correlated ping plus
  exact transcript cardinality and successful shutdown. Also test unavailable
  report notification silence and subsequent ping against the held server via
  HTTP and bridge-function. Observe completion rather than assuming ping is a
  tool barrier; compare ordinary response sets by ID if concurrency permits
  reordering. Keep binary-unavailable and wire-rate notifications unclaimed.
- **AC6 — Unchanged guarantees and scope.** The implementation diff is confined
  to existing test owners and this supplement's evidence. Preserve synchronous
  accept/storage, subscribe-before-scan, oldest-only pruning, ingest extraction,
  bounded lazy analysis, worker-owned permits, report cache/bytes/deadlines,
  shared 1024-per-second rate and bridge cap. No dependency/license, production
  API, bundle/policy, fixture flag, coverage exclusion or denominator change.
- **AC7 — Current local gates.** Execute every house gate and framed design/
  delivery proof on the clean unsigned implementation candidate; bind command,
  exit, HEAD/tree and logs. Require locked dependency use, exact zero missed
  lines/branches/functions, coverage selftest, publish/seal proofs, licenses,
  unchanged shims, fail-closed native-tls absence and digest-checked local brokkr.
  Any failure stays named; local green does not assert engine verification.
- **AC8 — Faithful handoff.** Record inherited findings with original severities,
  all missing prerequisites and the five separate protected reviewer obligations.
  Do not forge, downgrade or substitute verdicts. Missing external receipts stay
  explicitly outstanding rather than becoming local completion criteria.

## Protected findings and external prerequisites

Preserve the complete prior-153 evidence, not just this summary. Run `20b54c48`
remains awaiting_operator in review: event
`ac3d0439-c41c-4415-8296-544de921621c` (seq 834) records tests indeterminate;
settled correctness, guarantees, independent and security members retain the
HIGH security-relevant inherited review-integrity/result-attribution residual.
Their HIGH findings are not averaged with another member's LOW description.
Preserve MEDIUM actual-client gaps, LOW OPTIONS/shared-rate/documentation and
route-record findings, missing historical/live-work evidence and INFO findings
at their original member-specific severities. This design is no new verdict.

No attack reproduction, reviewer-access widening, fabricated result, engine
re-pin or claim that an upstream merge fixes the pinned engine. The engine and
security reviewer retain their verdict authority. Parent review remains live.
The separate #152 OPTIONS repair is on another candidate: reconcile its actual
review before landing and rerun combined-candidate proofs. Accepted
[0043](../../docs/decisions/0043-mcp-origin-boundary.md)'s guard-admitted 405/no-CORS
and hostile-Origin-first refusal remain governing; C1–C6 do not waive them.

Actual #152 baseline/protected review, #151 paid baseline, Claude Code/Codex/
MCP Inspector/Claude Desktop receipts, CI-owned proofs and operator seal remain
separate merge prerequisites. Host-wide standalone-eval coordination under #201
is unproved; a private lock is not evidence. Run neither standalone eval suites
without that authentic coordination nor paid trials. Preserve the selected
Terra 6.1 intake, Opus 5.5 implementation, separate Astra correctness/guarantees,
official Flash security, inherited Claude independent and DSH Spark GLM-5.3-Flash
tests routes. Do not change routing. No push, merge, publication, tag, issue
closure, decision acceptance or feature-completion claim is authorized here.
