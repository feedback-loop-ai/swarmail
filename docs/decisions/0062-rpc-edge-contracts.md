# 0062 — RPC edge contracts and the supplied ID ruling

- **Status:** accepted (operator, 2026-10-26)
- **Date:** 2026-10-24
- **Scope:** Epic #192, issue #180; six stdio/MCP residuals.
- **Specification:** [RPC edge contracts](../../specs/rpc-edge-contracts/spec.md).
- **Records:** the operator's supplied 2026-10-24 directive amending accepted [0047](0047-storage-rpc.md) §3's supplied-ID domain.
- **Clarifies:** accepted [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md) §§1/4 framing, staging and tracing proofs, preserving accepted [0044](0044-compat-validation.md) §§1/2 batch behavior.

## Authority and dated amendment — 2026-10-24

The operator directive in the #180 delivery task states: "amend accepted
decision 0047 to reject Boolean, object and array JSON-RPC IDs with error
-32600 and id null, while preserving valid string, number and null IDs."

This exact behavioral choice is already authorized by that directive.
This record is proposed as required by the semantic-design workflow; neither
the chief nor the panel accepts it. Its status does not withdraw the supplied
instruction or authorize broader changes. Implementation appends a dated
2026-10-24 addendum to 0047 with that directive as provenance, preserving
historical body/status, explicitly superseding §3's unrestricted supplied-ID
sentence and historical absence of ID-domain validation. No repeat ruling is
requested. Index 0062 as proposed during implementation.

## Ruling recorded for implementation

Only supplied JSON string, number and null IDs are valid; every serde_json
number retains existing value semantics, including negative/fractional values.
Boolean/object/array IDs refuse -32600/id null before method dispatch, tool
effects or waiter registration. An invalid supplied ID is not a notification.
Missing ID plus string method remains effectful and silent. Malformed objects
and non-objects retain the existing invalid-request rule.

Use one ID-domain predicate beside reply_id. Classification normalizes invalid
supplied IDs to owed null positions; handle enforces refusal before effects.
Keep Envelope and concurrent batch execution, input reply order, duplicate/
null slots, notification omission, bounded admission before effects and
unspecified effect order. HTTP and stdio agree. Local bridge transport errors
remain -32000 and oversize refusals remain -32600 with existing wording;
normalization does not invent upstream validation. Complete upstream responses
remain verbatim.

The operator's chosen compatibility cost: clients formerly using invalid IDs
lose both permissive echo and tool execution. No version/params validation,
integer-only restriction or numeric canonicalizer is implied. A handler-only
change loses local correlation; normalization without a handler guard still
executes invalid mutations. Both are refused.

## Compatible repairs and preservation

For recognized Content-Length, short bodies refuse as truncation before
content interpretation; exact/excess length and no-length behavior remain.
Cap/header/status precedence and existing deadlines remain. No allocation to
advertised length, retries, chunked decoder or malformed-header policy change.
A truncated response can follow a completed mutation; transport refusal means
uncertain execution, never rollback.

Checked usize staging refuses unrepresentable values with named -32602,
preserving absence defaults, wrong-type/null rules and tool bounds. Overflow
needs actual 32-bit runtime proof; compile-only evidence is insufficient.

Binary tests must cause the existing stdio read warning and observe stderr,
empty stdout and exit 1. Host tests exercise production construction and fail
if dial_target rewrites original authority; actual port-80 networking remains
separate CI proof. Move the stray timeout test into its existing cfg(test)
module; retain exact coverage and 0020 test placement.

Preserve 0001 accept==stored, 0002 subscribe-before-check, 0003 oldest-only
pruning and 0005 extraction at ingest. No storage, persistence, SMTP, webhook,
dependency, gate or recipe-policy change is required.

## Alternatives, costs and delivery

Adopt both panels' existing-owner repairs and robustness's additional batch,
silence and recovery wire witnesses. More tests cost execution and host
provisioning, but a happy-path reply or clean stdout cannot prove the boundary.
Reject new frameworks, HTTP replacement, width simulation, global admission
changes and test relocation to escape measurement.

Both panels recommended only a 0047 addendum. The chief rejects omitting a
new proposed record because the design contract requires one for a semantic
change. The addendum is an implementation task; the design commit contains
only this record and three spec artifacts. The temporary missing index row
is named, never treated as a green document-integrity gate.

AC1–AC10 and paired tasks control implementation. All house gates and exact
coverage remain mandatory for delivery. Actual 32-bit, supported-host stderr,
privileged port-80 and CI-owned evidence remain separate obligations. Drafted
does not mean issue completion or decision acceptance.
