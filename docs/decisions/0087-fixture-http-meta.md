# 0087 — Discard fixture HTTP request metadata and name GET-body refusals

- **Status:** proposed
- **Date:** 2026-10-26
- **Issue:** Epic #164, issue #151 part 3
- **Specification:** [Fixture HTTP metadata](../../specs/fixture-http-meta/spec.md)
- **Supplements:** [0055](0055-agent-eval-harness.md) and the helper boundary
  described by proposed [0083](0083-cli-wrapper-launch.md), compatibly with
  accepted [0075](0075-eval-privacy.md) and [0044](0044-compat-validation.md).
- **Number:** 0087 is next free in the verified 2026-10-26 operator ledger and
  candidate; 0079–0083 remain allocated. Historical reserved numbers stay reserved.

## Context

The supplied Claude smoke shows standard params._meta causes the fixture-http
helper to reject otherwise valid eval_http calls. All three supported MCP
revisions permit request metadata and make progress responses optional. The
operator directs acceptance without echo and permits either empty-GET-body
normalization or a named tool refusal. This proposal records the observable
helper contract change without accepting itself or changing accepted rulings.
The source packet hashes and panel synthesis are preserved in the linked spec;
tracked authority is [the operator rulings](../rulings/epic-192.md).

## Proposed ruling

1. Admit and discard the reserved params._meta field at supported request
   locations, including initialize and tools/call, on 2025-03-26, 2025-06-18
   and 2025-11-25. Keep initialized notifications silent. Metadata has no
   effects, retention, forwarding, echo or progress emission. No new strict
   metadata validator or general revision-conformance claim is introduced.
2. Add only the explicit discarded Call field. Keep tool params and arguments
   closed everywhere they are currently closed; arguments._meta still refuses.
   Keep name/arguments required, the sole eval_http descriptor, existing error
   codes and all unrelated permissive dispatch behavior. Unsupported methods
   remain unsupported. Existing bounded JSON parsing still owns input limits.
3. For valid tool arguments, any present GET body, including empty string or
   explicit null, produces result.isError=true with fixed text
   http_get_body_not_supported, no JSON-RPC error and no HTTP request. Omitted
   GET body and POST JSON serialization keep their current meanings. All other
   helper failures retain the finite fixture_request_refused projection.
4. Preserve identity injection, relative-origin/path and header admission,
   no-proxy/no-redirect behavior, line/argument/response bounds, deadlines and
   response bytes. Do not add retries, metadata-dependent state or capabilities.
5. Prove the change failing-first through the real stdio child and loopback
   HTTP for every supported revision, full streams and request capture,
   closed-shape negatives, GET/POST controls and oversized metadata. Run the
   standalone eval suite and every local house gate on the committed candidate.

## Alternatives and costs

Adopt simplicity's existing owners and single discard field; adopt robustness's
complete leakage/no-effect regression matrix. The panels agree on runtime
behavior. The larger proof costs test code and child-lifecycle discipline,
while avoiding an MCP framework or another protocol owner. Ignore metadata
rather than validate/echo it: non-object values may be tolerated, and progress
is unavailable. Keep GET body refusal rather than normalize empty strings:
clients must omit the field, with a clear tool-level reason to guide correction.
Never publish arbitrary error strings to gain that diagnostic.

## Preservation and delivery boundary

This compatible repair changes only eval implementation and preserves frozen
0001–0003 and ingest extraction 0005. It changes no root MCP behavior or error
mapping; the helper's existing unknown-tool code remains -32602. HTTP effects
and a reply are not one transaction; no retry or exactly-once claim is added.
No dependencies, routing or baseline execution belong to this work.

The chief must commit exactly this proposal and its three spec artifacts.
That excludes docs/decisions/README.md, while the root inventory test requires
its row. Record that mandatory gate failure honestly. Subsequent evals-only
implementation cannot silently repair it either: operator/documentation-owner
authorization and integration are required before delivery is green. This is
an explicit outstanding prerequisite, not a waiver or reason to alter the test.
Only the operator accepts a decision. Structural design validation, local
implementation proof, CI evidence and the operator's live baseline are separate.
