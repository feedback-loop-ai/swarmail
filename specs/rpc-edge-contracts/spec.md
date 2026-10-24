# RPC edge contracts — issue #180

Date: 2026-10-24. Epic #192; run epic-192-next-180-luna-xhigh-del-7b62e4e5.
Status: implementation present; exact coverage, 32-bit runtime and external host/CI proofs remain required.

Decision: 0062 — [RPC edge contracts](../../docs/decisions/0062-rpc-edge-contracts.md), proposed.

## Feature and rationale

Repair six observed stdio/MCP residuals without replacing either transport.
A short HTTP body must not become success merely because its prefix is valid
JSON. An invalid JSON-RPC ID must not authorize tool effects. Tests must
observe a real binary warning and the exact Host bytes, and integer staging
must refuse overflow on the executing target. Move the misplaced timeout
test under its existing test module.

Authority is accepted [0029](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md),
[0044](../../docs/decisions/0044-compat-validation.md) and
[0047](../../docs/decisions/0047-storage-rpc.md), together with the explicit
operator directive supplied for #180 on 2026-10-24: amend 0047 to reject
Boolean, object and array IDs with -32600/id null while retaining string,
number and null IDs. That exact choice is settled; no further ruling is
requested. New 0062 remains proposed, not an assertion of operator acceptance
of a document the operator has not seen. Implementation records the supplied
directive as a dated addendum to 0047 without rewriting its history/status.

## Synthesis and explicit rulings

Both read positions are run-local evidence at
`.brokkr/state/design/positions/simplicity.md` and `robustness.md`, grounded
in `.brokkr/state/tasks/epic-192-next-180-luna-xhigh-del-7b62e4e5.md`.
They substantially agree; there is no invented architectural disagreement.

| Position | Ruling and cost |
|---|---|
| Simplicity: repair existing owners, retain Envelope and one local ID predicate. Robustness: validate before content/effects and share correlation. | Adopt both. A predicate used by reply_id and handle is sufficient; no request model or validation module. Cost: two call sites need separate correlation and no-effect proofs. |
| Simplicity: one short-body TCP witness. Robustness: also prove batch positions, notification silence and next-line recovery. | Adopt the stronger wire matrix in existing tests. A lone request is insufficient evidence for the shared classifier. Cost: additional real exchanges and fixture maintenance; no new production mechanism. |
| Both: amend 0047 only, with no new ADR. | Reject the no-new-ADR recommendation because this chief's explicit semantic-design contract requires a new proposed record. Commit 0062 now; put the authorized dated 0047 addendum and index row in implementation. Cost: an extra record and a temporarily missing decision-index row at this design-only checkpoint. A proposal grants no acceptance. |
| Both: real bad-stdin process and production-used Host construction proof. | Adopt. Clean stdout on a refused connection does not prove tracing, and default-port dialing does not prove Host bytes. Cost: Linux/macOS process evidence plus retained privileged port-80 proof. |
| Both: checked usize conversion, actual 32-bit runtime proof, and move only the stray test. | Adopt. Reject width simulators and broad test relocation. Cost: unavailable target/runner leaves a required proof outstanding. |
| Robustness: preserve storage/SMTP/webhook failure boundaries; simplicity: keep their implementation out of this issue. | Adopt preservation requirements without a new storage test project. Uncertain mutation execution after transport loss remains accepted; no automatic retry or rollback claim. |

Reject an HTTP library replacement, chunked decoding, stricter malformed or
duplicate Content-Length policy, version/params validation, integer-only IDs,
retries, new queues, global memory limits, CI redesign and dependencies.
Their existing limitations remain: identity HTTP only, read-to-close/deadline,
unbounded stdin-line allocation and slow-stdout exposure. Preserve accepted
0039's 16 concurrent stdio exchanges and 0044's concurrent admitted batches.

## Acceptance Criteria

1. **AC1 — Framing before content (12.1).** With a recognized Content-Length,
   fewer available body bytes yields an error naming truncation before UTF-8,
   JSON, whitespace silence or upstream-error forwarding. Cover short valid
   JSON, whitespace, invalid UTF-8 and non-2xx JSON-RPC error bodies. Keep
   existing cap/header/status precedence; exact length, zero length, no length
   and excess-tail slicing remain unchanged. Existing malformed/unrepresentable
   lengths retain their handling. Correct the fixture claiming nine bytes
   for eight-byte `{"id":1}`; replace length-9999/`{}` success.
2. **AC2 — Truncation on the wire (12.1).** A real loopback HTTP peer sends a
   complete header, short body and EOF. Through the bridge this yields -32000
   with the owed request ID, ordered batch slots (duplicates/null included),
   and silence for valid notifications. A subsequent line succeeds. Complete
   upstream error objects remain forwarded verbatim. No automatic retry.
3. **AC3 — ID domain before effects (12.2).** Both Booleans and empty/nonempty
   object/array IDs yield HTTP 200 JSON-RPC -32600/id null before any method,
   tool mutation or waiter registration. Strings, integers, negative and
   fractional numbers and explicit null keep their JSON values. Missing ID
   plus string method remains an effectful silent notification; malformed
   id-less objects and non-object members remain -32600/null. Invalid IDs
   take precedence over method/argument dispatch. Real HTTP and stdio tests
   cover this table and prove an invalid-ID mutation changes no isolated
   inbox, with an effectful valid notification/control proving the observation.
4. **AC4 — Correlation and admission (12.2).** One ID-domain predicate feeds
   reply_id and handle. An invalid supplied ID owes a null position, never
   silence. Mixed admitted batches keep input reply order, duplicate and
   explicit-null positions and omit only notifications; execution remains
   concurrent with unspecified effect order. Empty/over-16 batches refuse
   before effects. Local bridge connection/truncation failures retain -32000;
   oversized bodies retain -32600 and existing size wording, both using the
   same normalized slots. Complete upstream replies are unchanged; network
   failure does not synthesize server ID-validation results.
5. **AC5 — Actual tracing route (12.3).** Retain the protocol stdout test and
   add a bounded binary subprocess with deterministic stdin read failure,
   explicitly enabled child warning filter, exit 1, identifiable
   `stdio read error` in stderr and empty stdout. Disabling tracing or routing
   its writer to stdout must fail. Prove supported Linux/macOS behavior;
   EOF alone and generic in-process Boom readers are insufficient.
6. **AC6 — Verbatim Host (12.6).** Tests assert the production-generated
   CRLF-delimited Host field preserves a portless hostname, IPv4, bracketed
   IPv6 and explicit-port spellings, including supplied case. Replacing Host
   authority with dial_target must fail. Preserve path joining, length bytes,
   default dialing and the real ignored port-80 exchange; its Linux/macOS CI
   execution remains separate evidence.
7. **AC7 — Checked target-width staging (12.5).** arg_usize uses checked
   conversion, preserving absent defaults, wrong-type/null refusals, zero
   semantics and tool-specific caps. Ordinary-target tests exercise controls.
   Actual 32-bit execution accepts usize::MAX at staging and rejects
   4_294_967_296 with a field-naming -32602 before work, including a real MCP
   call (for example search offset). A 64-bit run or cross-build alone cannot
   prove overflow refusal. Record target, command and receipt; unavailable
   runtime remains an open criterion.
8. **AC8 — Test ownership (12.4).** Move
   a_connect_past_the_deadline_names_the_timeout into the existing
   `#[cfg(test)] mod tests` in src/stdio.rs, retain assertions, and prove
   discovery/execution. No unrelated module migration or coverage exclusion.
9. **AC9 — Authority and preservation.** Append the dated 2026-10-24 operator
   amendment to 0047 §3, explicitly superseding unrestricted supplied-ID
   behavior and the historical absence of ID-domain validation. Keep accepted
   text/status, index 0062 as proposed, and reconcile affected current comments
   and contract claims. Preserve 0001 accept==stored, 0002 subscribe-before-
   check, 0003 oldest-only pruning and 0005 ingest-time extraction; storage,
   persistence, SMTP and webhook production behavior stays unchanged.
10. **AC10 — Honest delivery evidence.** Capture discriminating behavioral
    reds and restored greens, then satisfy every gate in
    [house rules](../../docs/house-rules.md#gates--part-of-every-change), exact
    coverage/selftest, pinned-copy/shim verification and fail-closed native-tls
    absence included. Keep recipe commit 450bef65c581b3a4791cf94dbc145912814bd6ed.
    Name outstanding 32-bit, supported-host and CI-owned proofs; no issue/epic
    closure based on design alone.

## Delivery boundary

The chief commits exactly this directory's spec.md, plan.md, tasks.md and
proposed 0062. No source or accepted-decision edit belongs in this commit.
Implementation first adds the index row before full document-integrity tests:
this structural design checkpoint does not claim that suite green with an
unindexed new record. Only the deterministic check owns the designed verdict.
