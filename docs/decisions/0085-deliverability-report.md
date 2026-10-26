# 0085 — Lazy, versioned deliverability reports

- **Status:** proposed
- **Date:** 2026-10-26
- **Scope:** Epic #251 issue #252; additive report and predicate contracts.
- **Authority:** operator's #251 ruling of 2026-10-26: analyze stored bytes lazily,
  cache results, never run analysis on SMTP; built-in deterministic spam only in
  later stories, no external scorer. Future authentication uses an existing
  permissive crate with justified widening. This story needs no dependency.
- **Preserves:** 0001–0003 and ingest extraction under 0005; 0010 exact assert /
  minimum-one wait; 0014/0021 read and watcher ordering; 0029/0044/0074 admission;
  0023/0030 offload and existing panic symmetry. No accepted ruling is amended.
- **Number:** 0085 is assigned by this task and absent before this proposal;
  reserved/missing earlier numbers are not reused.

## Decision proposed

The normative contract, including literal field names, ordering, limits, rule
semantics and error text, is [the specification](../../specs/deliverability-report-contract/spec.md#contract).

1. One `rule_set_id` and ordered typed `findings` represent derived analysis.
   Each finding has stable `rule_id`, explicit `info < warning < error`, a
   tagged location and nonempty fix hint. Root header locations use lowercase
   names plus occurrence/null; MIME locations use structural child paths;
   link locations use part path plus transfer-decoded source-byte offset.
   Sort by rule, typed location, severity and hint, then remove exact duplicates.
   Rule/parser/location/hint/order changes require a rule-set version change.
2. `swarmail-demo/1` contains only `demo.marker.v1`: the first root
   X-Swarmail-Demo value, unfolded and ASCII-space/tab-trimmed, selects one of
   the three lowercase severity tokens. It emits one fixed hint or nothing.
   A typed `demo_enabled` flag is true by default, with no new public knob.
   Scan at most 65,536 header bytes; no MIME/body parse or HTML extraction.
   Malformed/binary handling follows the specification's byte rules. Oversized
   analysis is a failure, not a clean or truncated report.
3. Lazily analyze immutable stored bytes outside Store mutation/map guards and
   outside SMTP, seed, restore and fan-out. Never alter stored extraction or
   rows. This implements the operator's separate analysis product; it does not
   generalize 0031's display carve-out or move link/code extraction from ingest.
4. One Store-owned process-local cache has 128 slots, each keyed by validated
   Weak object identity plus exact immutable config and rule-set id. Public id,
   naked address and TTL are not freshness proofs. Selected request Arcs retain
   existing snapshot semantics: a pre-delete read may finish, new post-delete
   lookups cannot. No final liveness check or stronger snapshot is promised.
5. Pinned slots serialize same-key computation. Evict oldest unpinned slots;
   all-pinned capacity refuses. At most four evaluations enter the existing
   blocking pool; the actual worker retains its permit and slot guard through
   cancellation/unwind. Cache bookkeeping never executes analysis. Retain at
   most 256 findings and 65,536 compact JSON report bytes per slot, failing
   explicitly on excess. These are cache/job limits, not RSS or latency claims.
   Ordinary failures are not cached; panics follow Store::offload symmetry.
6. REST gets `/api/v1/messages/{id}/deliverability`; MCP gets one
   `swarmail_get_deliverability_report` tool; the existing message page gains
   an escaped report section. All consume one typed result. Missing messages
   keep not-found behavior; unavailable/failed analysis yields fixed sanitized
   errors, never empty success. MCP uses main's current 2025-03-26 contract:
   report JSON in a text block through the existing result helper. Add report
   structuredContent/outputSchema when #153 lands as a later, small follow-up,
   using its shared owner; it is not a prerequisite for this issue.
7. `no_findings_at_or_above` qualifies ordinary-filter matches for REST
   assert/await and MCP wait. Empty reports qualify, failed reports do not.
   Assert remains exact including zero; wait remains minimum-one. Qualify
   before paging, retain newest-first order and existing total semantics.
   No new list/count filter, universal-inbox assertion or alternative wait loop.
   Subscribe before first scan and after Closed, retain subscription during
   analysis, recover lag by scanning, freeze inputs for one request, retain one
   deadline/shutdown race. An incomplete analysis scan at deadline is an
   unavailable error; a complete insufficient scan retains existing timeout.
8. #253 extends a pure collector with explicit immutable observation/policy
   inputs, adding their full identity to the key. It owns expiry, refresh and
   wake semantics. No ambient DNS/time, resolver, registry or authentication
   crate is introduced here. Fixture providers prove the seam locally.

## Alternatives ruled and costs

Robustness wins bounded Weak-cache ownership over simplicity's Email-owned
slot/reset-on-insert. Pay eviction, admission and 503 paths; keep ingest,
constructors and persisted Email free of derived state. Simplicity wins the
existing snapshot boundary over robustness's final liveness recheck: a selected
read can complete after deletion, but this does not resurrect future lookups.
Robustness's artificial marker beats a missing-Subject warning because it proves
all severities without implying a substantive quality check. Structural paths
and source offsets settle locator identity without inventing a link traversal.
Simplicity wins one wait owner, server rendering and static rule composition.

Reject a persistent cache, background scanner, extra protocol negotiator, new
executor, mutable raw mail, dynamic rule framework, real authentication checks
and policy networking. Cold requests pay bounded scanning; eviction/config
changes pay recomputation; four jobs still share process/OS resources; caller
snapshots and transport waiters remain outside cache accounting. Those limits
are explicit rather than claimed away by a successful report.

## Dependency, proof and disposition

The operator's current brief (2026-10-26, run
`epic-251-issue-252-implement-the-22ac6e57`) withdraws the earlier instruction to
follow #153 now. This issue runs on current main in parallel with epic #164.
There is no design or completion dependency on 0079/0080/0082 or #152/#153
artifacts. Spec AC7 proves the current 2025-03-26 text-block contract; report
structuredContent/outputSchema is a later, small follow-up when #153 lands.
No newer MCP conformance, decision acceptance or protocol change is implied.
0085 remains proposed. The previous IMPL-BLOCKED record stays historical.

Spec AC1–AC10 and [paired tasks](../../specs/deliverability-report-contract/tasks.md)
require typed fixtures, identity/cancellation/deadline interleavings, real SMTP /
REST / MCP / stdio / UI proofs, all local house gates and exact coverage. CI-only
proofs remain CI-owned. Add the proposed index row at the permitted documentation
step; never accept this proposal or alter accepted records. Compatible work may
proceed under 0045 §8 and the supplied operator direction. A concrete accepted
contract contradiction still waits for the operator; frozen weakening is refused.
