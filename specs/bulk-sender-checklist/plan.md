# Implementation plan — bulk-sender checklist

[Specification](spec.md); [proposed 0089](../../docs/decisions/0089-bulk-sender-checklist.md).

## Technical context

One Rust 2024 crate, Tokio, serde and pinned mail-parser 0.11. Existing
src/deliverability.rs owns Inputs, Collector, Service and errors; Store owns
report qualification/waits. Current evaluation is demo-only, with no trusted
authentication provider. No new dependency, endpoint, executor, public knob or
persistence field is needed. The requested plan template is absent from
.specify/templates; this document supplies its required sections. The available
spec template's multi-repository scaffolding does not apply to this crate.

## Constitution check

0001–0003 stay unchanged: analysis never enters acceptance, subscription precedes
scans, and cache ownership never changes pruning. 0005/0031 ingest links/codes
and sender-honest bodies remain unchanged; operator #251 authorizes this separate
lazy product. Preserve 0085 snapshot semantics and worker-held reservations
through cancellation/unwind, with no analysis under Store/cache bookkeeping guards.
Compatible work may proceed under 0045 §8; accept no decision.
0020 keeps branch-stable tests measured inline and cold diagnostics in owner-local
harnesses. Public wire tests do not gain private seams. No global mutable state,
sleep synchronization, coverage exclusions or intermediate chaos probabilities.
Keep bundle routing and dependency policy unchanged. Commit unsigned, never push.

## Research and settled choices

Read both positions, tracked intake, decision index, 0001–0005, proposed
0085/0089, evaluator/service and existing test owners. Packet hashes verified.
Legacy tasks and plan-template paths are absent; existing local framing suffices.
The specification's dated adjudication rules each conflict and names its cost.

A full mail-parser invocation is unnecessary for metadata presence and can
decode/allocate more than needed. Use a bounded outer MIME metadata walk in the
existing owner. Existing parsing primitives may operate on bounded slices only
where complete consumption and allocation behavior are established. A post-parse
part count is not a pre-allocation bound. SMTP sender_text_part supplies the
sender-honest intent, not display-generated content or a function to relocate.

## Project structure and files

| File | Planned work |
|---|---|
| src/deliverability.rs | Version, static checks/hints, bounded header/grammar/MIME helpers, immutable facts and cache input propagation, measured inline tests. |
| src/deliverability/concurrency_tests.rs | Real-server immutable auth fixtures and cache/wait/held-worker proofs requiring private seams and cold diagnostics. |
| tests/deliverability.rs | Raw SMTP pairs, exact findings, public unknowns, MIME/header/budget failures, surface and predicate proofs. |
| tests/stdio.rs; tests/api.rs, tests/mcp.rs, tests/ui.rs where affected | Updated report expectations, current MCP text parity, preserved admission/escaping. |
| README.md, src/static/llms.txt, src/static/openapi.json | Living checklist descriptions, source/scope, threshold and limit documentation; no schema shape change. |
| src/store.rs, src/smtp.rs, src/persist.rs, src/api.rs, src/mcp.rs, src/ui.rs | Preservation owners; no planned production adapter/ingest changes. Update owner test expectations as needed. |
| tests/burst.rs, tests/p2.rs, tests/rate.rs, tests/persistence.rs | Run frozen-guarantee regressions; add a narrowly missing preservation witness only when necessary. |
| docs/decisions/0089-bulk-sender-checklist.md | Chief synthesis; existing proposed index row needs no edit. |
| specs/bulk-sender-checklist/{spec,plan,tasks}.md | Chief artifacts; historical intake and #252 specs stay untouched. |

## Data model

Retain serialized Report/Finding/Severity/Location. Use private static hint
strings and IDs. One bounded header scan retains only relevant original values,
occurrence counts and MIME metadata, including the demo's first occurrence.
Do not retain an unbounded vector of duplicate fields or duplicate scanners.

Add equality-comparable three-state facts and an internal authentication snapshot
to Inputs: policy presence, valid From, DKIM pass/alignment, SPF pass/alignment
and bounded immutable effective policy/observation identity. All facts/identity
participate in equality alongside version/config. Slot matching retains selected
email Weak identity. Future #255 must bind evidence to that mail/policy; production
here defaults to all Unknown with no sender-controlled source.

The per-Service cfg(test) seam supplies immutable observations before requests.
Freeze inputs at request entry, including a qualification request across all
candidates; never reread mutable fixture state between candidates. Cache
transition tests use successive immutable snapshots through private with_inputs,
not mutation of an in-flight observation or an ID-keyed global registry.
Owner-local fixtures run real SMTP and report HTTP, exercising actual rule logic.

## Implementation approach and contracts

1. Extend the existing root-byte scan for checked fields and occurrence counts,
   preserving demo behavior. Full-consumption grammar helpers implement CFWS,
   Message-ID, calendar Date, absolute URI list and exact Post. Do not use
   permissive token extraction as validation or introduce a URL dependency.
   Calendar arithmetic uses no clock.
2. Walk outer MIME metadata iteratively with a depth-bounded stack. Check raw
   length first, count headers/parts before allocating, carry ancestor attachment
   state, and apply context-sensitive defaults (including multipart/digest).
   Match complete delimiter lines and closing boundaries; embedded messages
   remain opaque. Do not transfer-decode leaf bodies. Walk the entire structure
   even after finding plain text; ambiguous/incomplete metadata returns Failed.
3. Compose the seven rules statically through Collector::push/finish. Use the
   exact hints, locations and three-valued formulas in spec. Policy, alignment
   and authentication remain independent. Ordinary errors never cache.
4. Preserve Service admission/offload/publication and Store's one qualify/wait
   owner, adding only immutable input propagation. Replace demo-only expected
   outputs; production now has three error unknowns. Positive qualifying tests
   use trusted facts on owner-local real servers, not disabled checks or injected
   report rows. Adapters continue serializing the same typed result.
5. Update living docs and prove shared surfaces. Keep visible limits: structural
   unsubscribe is not signed-header proof, text presence is not equivalence,
   DKIM-or-SPF is scoped and sources are not live-verified policy.

## Proof matrix

Each task names its proof. Inline tables exhaust grammar and three-valued
combinations. Owner-local real-server tests cover auth positive/negative pairs;
tests/deliverability.rs covers production no-evidence behavior. Unit-only auth
tests do not satisfy the paired wire requirement.

For every inclusive budget generate a valid at-limit and one-over structure,
keeping other budgets below their limits: body padding for raw size, header
padding/counts for per-block/aggregate limits, independent part/depth fixtures.
Include absent separators, missing closing boundaries, ambiguous metadata,
encoded multipart and header-like body lines. Report errors must leave raw
SMTP-accepted mail retrievable. Deterministic barriers prove held analysis
does not block SMTP insertion/pruning. Retain existing worker cancellation and
capacity proofs. No latency or total RSS assertion.

## Quickstart

After implementation, submit raw SMTP mail with Message-ID: <bulk@example.test>,
Date: Mon, 26 Oct 2026 12:00:00 +0000,
List-Unsubscribe: <https://example.test/unsubscribe>,
List-Unsubscribe-Post: List-Unsubscribe=One-Click and a genuine text/plain body.
GET /api/v1/messages/{id}/deliverability returns swarmail-bulk/1 with exactly
three authentication not-evaluated error findings (no demo marker).
Current MCP swarmail_get_deliverability_report returns the same JSON in a text
block; the UI displays it. Exact zero qualified messages succeeds, but a wait
cannot qualify this mail. Remove Post and see the extra one-click defect.
No request to example.test occurs.

## Validation and handoff

Run focused cargo test --locked --test deliverability, cargo test --locked --lib
deliverability:: and cargo test --locked --test stdio report, then all commands
in the sole [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change).
Set CARGO_NET_OFFLINE=true; install nothing. Verify .forge/bin/brokkr digest
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f and compile
bundles/self with that binary. Verify shims.sha256, then execute canonical
bundle coverage/selftest, publish-guard and seal-branch proofs. Preserve --locked
where required. Also run design-check selftest, delivery-proof-test and
fail-closed native-tls absence. Reject compiler text "lets the run go on without
reading" even on exit zero. Record command/exits/log hashes and tested commit/tree
in ignored run-local evidence. No intake receipt substitutes for fresh gates.

The chief runs gates on its documentation commit and design-check against HEAD.
Implementation/verify repeat on their own committed changes. CI/operator proofs
remain with those owners. Stage exactly spec.md, plan.md, tasks.md and proposed
0089 for the chief commit. Keep evidence/result/positions run-local and design
commit last. No implementation completion claim follows a valid design.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Unknown looks like clearance | Distinct IDs/error severity; all production thresholds exclude until genuine evidence. Exact zero still counts qualified mail only. |
| Legal legacy mail flagged | Explicit current syntax/restricted URI profile and fixtures; no SMTP refusal or byte repair. |
| Metadata walk drifts from MIME | Contextual defaults, attachment ancestry, complete structure and explicit failure; no general renderer. |
| Limits checked too late | Raw check first, bounded stack, pre-allocation accounting and no body decoding; exact boundary tests. Some accepted mail lacks analysis. |
| Forged or stale auth | Private trusted facts, full immutable identity plus Weak mail identity, spoof/cache-transition fixtures; #255 owns provenance. |
| Positive predicates hard to test | Real owner-local server with immutable facts; public production tests prove unknowns. No public fixture control. |
| Scanner changes demo semantics | Preserve first occurrence/unfolding/byte behavior and old cases; version new rules. |
| Shared pool contention | Four worker-held jobs, no Store guards/ingest work and barrier proofs; no OS fairness or latency promise. |
| Coverage burden | Static composition, measured exhaustive helpers and only cold diagnostics in harnesses; exact gate/selftest mandatory. |
| Provider guidance evolves | Fixed cited/versioned hints, future updates bump rule set; no live synchronization or certification claim. |
