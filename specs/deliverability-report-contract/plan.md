# Implementation plan — deliverability report contract

[Specification](spec.md), [proposed decision 0085](../../docs/decisions/0085-deliverability-report.md).
This plan is design work. Its behavior tests are implementation obligations.
Current-main scope was corrected at [re-intake](intake-current-main.md) under
explicit operator instruction; earlier runs remain historical.

## Technical context

One Rust 2024 crate, Tokio/Axum, serde/serde_json, immutable stored Arc<Email>
objects, synchronous ordered mutations and a SQLite mirror. Existing real-server
suites exercise SMTP, REST, MCP HTTP/stdio and the UI. Use existing dependencies;
no Cargo manifest/lockfile/license changes are planned. The new demo is a bounded
root-header byte scan, not a second MIME parser or content-quality suite.

The requested `.specify/templates/plan-template.md` is absent. As in existing
plans, this document supplies technical context, constitution check, project
structure, research, data model, contracts, quickstart and risks directly. No
separate research/data-model/contracts files are needed for this design.

## Constitution check

0001: no evaluation, cache callback or report serialization in SMTP, insert,
seed, restore or fan-out. Store construction may initialize an empty service.
0002/0021: retain one wait loop and subscribe-before-scan, including Closed.
0003: cache Weak references do not own mail or affect retention. 0005/0031:
new analysis leaves every stored extracted field verbatim; no lazy links/codes.
0010: exact assert and minimum-one wait persist. 0014's corrected snapshot
limitations remain; no whole-store transaction is implied. 0029/0044/0074:
strict admission precedes effects. 0023/0030: bounded work leaves async workers
without changing Store::offload panic symmetry. 0045 §8 permits compatible work;
this proposal accepts no decision and licenses no frozen weakening.

Tests use real servers and deterministic per-owner seams. Inline tests are the
default; cold failure diagnostics alone justify an owner-local *_tests.rs
harness under 0020. No global mutable probe, sleep synchronization, attribute
coverage exclusion, intermediate chaos probability or dependency widening.
Keep bundle/recipe routing unchanged. Commit unsigned; no push or publication.

The chief's final design artifacts are spec.md, plan.md, tasks.md and proposed
0085. The proposal already has its required decision-index row on this candidate;
retain its proposed status. Structural design-check and the full house gates
are distinct proofs. Use the current recipe's commit and validator contract.

## Research and resolved choices

Both panel positions and tracked intake were read after verifying the supplied
packet hashes. Local source confirms Email.raw is byte-exact and skipped in JSON;
get/list hand out immutable Arcs; Filter::matches executes under inbox guards;
wait_for subscribes before scanning and after Closed; offload re-raises panics.
message_view is server rendered with esc, so a second browser workflow is
unnecessary. Current mcp.rs has only PROTOCOL_VERSION 2025-03-26 and text_result.

Choose the bounded Weak cache to avoid a derived field on Email and resets on
insert/clone/restore. A Weak protects object identity even after its object dies;
upgrade must succeed and ptr_eq must match the current selected Arc. Arc::make_mut
with weak references disassociates those references, while try_unwrap/drop ends
the old object's lifetime. New objects cannot validate that old identity. Tests
must exercise these public ownership escapes rather than assuming IDs are unique.

The operator's current brief withdraws the earlier structured-output prerequisite.
Implement on current main independently of epic #164, with report JSON in the
existing MCP 2025-03-26 text block. There is no dependency on 0079/0080/0082.
The report's structuredContent/outputSchema integration follows when #153 lands
as a later, small follow-up, outside this issue's completion criteria.

## Project structure and files

| File | Work |
|---|---|
| `src/deliverability.rs` (new), `src/lib.rs` | Report/severity/location/config types; bounded demo scan; canonicalizer; cache/service and errors; pure collector and typed input-key owner. Declare the new module. |
| `src/store.rs` | Own an empty report service; expose stored-snapshot analysis internally; extend the existing wait owner with optional qualification. Leave Filter::matches, mutation order, insert and persistence untouched. |
| `src/api.rs` | Report GET, empty-query admission, typed optional assert/await severity, shared failure mapping and qualified snapshot counting. |
| `src/mcp.rs` | One tool/input schema, shared severity admission and wait option; current 2025-03-26 text_result formatting. |
| `src/ui.rs` | Escaped server-rendered report section on message view; offload report/render work; explicit empty/failure states. |
| `src/static/openapi.json`, `src/static/llms.txt`, `README.md` | Report schema, route/errors, demo disclaimer, predicate/count meaning and examples; document current text results and the later #153 follow-up. |
| `docs/decisions/README.md` | Proposed 0085 index row; no change to existing statuses. |
| `tests/api.rs`, `tests/mcp.rs`, `tests/stdio.rs`, `tests/ui.rs` | Real-server surface/validation/parity and wait tests. |
| `tests/persistence.rs`, `tests/burst.rs`, `tests/p2.rs`, `tests/rate.rs` | Extend lazy/restart proof where public protocols suffice; retain acceptance/extraction/count guarantees. |
| Owner-local test modules; optional `src/deliverability/concurrency_tests.rs` and `src/store/deliverability_tests.rs` | Deterministic ownership, admission and wait interleavings; use separate files only for concrete cold-path need. |
| `src/model.rs`, `src/persist.rs`, `src/smtp.rs`, `src/extract.rs`, dependencies, routing | Read/test preservation owners; no planned production changes. |

## Data model and cache algorithm

`Report { rule_set_id, findings }`, `Finding { rule_id, severity, location,
fix_hint }`, typed severity and tagged location enums implement the spec's wire
shape. `AnalysisConfig { demo_enabled }` is immutable and compared by value.
Use explicit ordering ranks, never enum debug text or lexical severity order.
A test-only version/input fixture demonstrates key changes without a public
version switch or a production registry.

Suggested concrete cache is a short mutex around a VecDeque of at most 128
Arc<Slot>s. Each slot holds Weak<Email>, config, version and an Arc async mutex
of optional completed Arc<Report> (plus its compact serialized bytes if shared).
The deque owns one Arc. Lookup/pinning and oldest-unpinned eviction happen under
the same short lock; a slot is unpinned only when the deque is its sole owner.
Use checked equality/Weak upgrade, no hash/pointer integer index. A linear scan
of 128 slots is acceptable and avoids another invalidation/index owner.

1. Admit all request fields, freeze config and get candidate Arc(s) from Store.
   Drop map guards. The service never owns Store, avoiding a reference cycle.
2. Pin an existing full-key slot, or reserve a new one, evicting the oldest
   unpinned slot if necessary. If all slots are pinned, return unavailable.
   This critical section neither awaits nor scans raw bytes.
3. Await the slot's owned async lock. Recheck completed result under that lock.
   Same-key callers share its success, independent of surface. A canceled
   waiting caller drops its pin; no evaluator was submitted for that caller.
4. On a miss, try-acquire an owned four-slot semaphore permit. If unavailable,
   return fixed unavailable and leave the cache cell empty; never submit queued
   blocking work. Move the permit, slot pin, selected mail Arc and owned mutex
   guard into Store::offload. The worker, not the caller, owns these until exit.
5. Evaluate and canonicalize, checking maximum findings and encoded bytes before
   publication. Check provider contributions while collecting to avoid building
   an unbounded temporary Vec. Store only successful complete reports. Guards
   release on ordinary error and unwind; no persistent Computing state or custom
   panic catcher is needed. A canceled caller's completed worker may populate
   the valid cache, without needing another runtime task to publish it.
6. Return the selected snapshot's result. No final liveness recheck. Every later
   surface request starts again with a live lookup. Restored Store owns an empty
   cache; cache eviction is recomputation, not mail deletion.

The payload ceiling is encoded JSON size, not allocator accounting. At most
128 retained cache payloads of 65,536 bytes (8 MiB encoded) plus bounded typed
representation/key/slot overhead are owned by the deque. Caller-held report
Arcs can outlive eviction and are outside that cache bound. There are at most
four admitted report jobs per Store, including jobs whose callers canceled;
multiple Stores and unrelated offload jobs have their existing separate costs.
No custom threads, scheduler, persistent table or sweeper.

## Implementation sequence and contracts

First implement types, pure demo scanner, canonicalizer and fixtures. Header
names are nonempty printable ASCII excluding colon, with no whitespace before
the colon. Strip each line ending; unfolding replaces the continuation newline
and its leading space/tab run with one space. A malformed line ends the previous
field's continuation association. Bound the complete header block even when
analysis is disabled, so resource failure policy is config-independent. No body
walk, HTML parsing or fresh extraction occurs.

Next implement the service with the ownership proofs above, then expose the
per-message route and legacy tool. Serialize the same Report on all surfaces.
Keep operational failure separate from finding severity: a provider failure
returns Err, not an error-severity finding that an `error` threshold might hide.
REST 400/404/503 and existing legacy MCP -32602/isError are pinned by tests.
Report outputSchema/structuredContent is deferred to a later, small follow-up
when #153 lands. Current-main text results fully satisfy AC7 here.

Extend one internal wait loop with optional analysis settings, preserving the
public unqualified wait entry point if its signature is externally used. The
cheap path must remain identical when omitted. Add a shared qualifier over
owned ordinary-filter snapshots for asserts and waits; never change Filter.
A scan freezes its candidate set and evaluates sequentially newest-first.
Assertions scan all candidates for an exact count. A wait may stop once want
qualifiers exist, since it returns at most want; insufficient scans must complete
to report a matched count. Pages are selected only after qualification.

Keep subscribe before snapshot, receiver alive through work, and re-subscribe
before the next scan on Closed. Race analysis/slot waits against the one original
deadline and existing shutdown receiver. A completed insufficient scan can yield
the existing timeout/count; an interrupted scan yields unavailable without a
partial matched value. Zero deadline permits only immediately ready completion.
New arrivals during a scan remain queued notifications for a subsequent scan.
No pending-report wake topic, polling interval or detached candidate fan-out.

For UI, obtain the same report and render escaped textual locators/hints beside
existing message content. Report failure renders an explicit failure section;
the selected message page stays 200 because the message itself exists, while
missing mail remains 404. The REST report endpoint itself returns 503 on that
failure. The list page never requests reports. Keep all report strings escaped
even though current rule strings are fixed; adversarial fixture providers prove
that a later provider cannot create markup or a URL fetch.

Finally prove report JSON text-result parity on current HTTP and stdio tools;
update living user docs and retain the proposed index row. Document the later
#153 follow-up without a prerequisite. Do not edit historic specs or accepted
rulings. Ordered tasks assign each proof explicitly.

## Quickstart and validation

After implementation, start the existing server with its usual local defaults.
Send raw SMTP mail containing `X-Swarmail-Demo: warning`, then retrieve its id
through the ordinary message API. `GET /api/v1/messages/{id}/deliverability`
returns swarmail-demo/1 and one warning. Calling MCP
`swarmail_get_deliverability_report` with that id yields identical report content.
The existing message page displays it. An assert on that inbox with
`count=0&no_findings_at_or_above=warning` succeeds (zero qualifying messages);
using threshold error with count=1 succeeds. This demonstrates per-message
qualification, not a universal health claim. Mail without the marker qualifies
at every threshold. An empty inbox never satisfies a wait, even with count=0.

Run focused owner and real-server suites first, then every command in the sole
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change).
Use offline Cargo with --locked where specified, check the engine digest
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, and run
`.forge/bin/brokkr compile --bundle bundles/self`. Check
`bundles/self/scripts/shims.sha256` and execute the canonical bundle copies of
coverage, coverage selftest, publish-guard and seal-branch proofs. Also run
design-check/selftest, delivery-proof-test and fail-closed native-tls absence.
Exact coverage requires zero missed lines, branches and functions. No installs,
network access, weakened checks or inherited baseline receipts substitute for
new execution on the committed candidate. Retain command/exits/log hashes and
commit/tree identity in run-local evidence. Bind verification to the committed candidate.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| MCP scope drifts into epic #164 | Pin current 2025-03-26 JSON-in-text results and unchanged initialization; defer report structuredContent/outputSchema to the small follow-up when #153 lands. |
| Derivation delays SMTP indirectly | Never evaluate under store guards; four bounded header-scan jobs only, offloaded; prove SMTP commits while jobs are held. Shared OS/pool contention remains, no latency SLA. |
| Same id/different bytes or ownership escape | Weak upgrade + ptr_eq and full inputs, live selection first; prove clone, make_mut/try_unwrap, replacement and restart. Never key on id/TTL alone. |
| Cancellation refunds capacity early | Worker owns permit, slot pin and owned lock; deterministic abort/unwind tests, with no separate publication task. |
| Cache growth/eviction races | 128 pinned-aware slots, one short bookkeeping lock, bounded complete payloads; refuse all-pinned pressure. Eviction may cause later recomputation. |
| Mail disappears during analysis | Preserve selected snapshot behavior, fresh requests return not-found; no cache retrieval by public id. Document no whole-store atomicity. |
| False clean on parse/size/deadline failure | Typed errors, sanitized operational responses, no truncated report or partial exact count. Additional 503s are intentional. |
| Wait loses arrivals/clears | Reuse existing receiver loop; retain subscription during analysis and re-subscribe before scan; barrier/lag/clear tests with no sleeps. |
| Provider makes future reports stale | Explicit immutable provider inputs and key identity; #253 owns refresh/wakeup before external observations can ship. |
| Demo is mistaken for deliverability assurance | Synthetic rule namespace and explicit empty-report disclaimer in every user-facing explanation. No score or authentication success. |
| New framework expands exact coverage burden | One module, bounded deque, existing offload/wait owner and static collector; no dynamic registry, executor or new dependency. |
