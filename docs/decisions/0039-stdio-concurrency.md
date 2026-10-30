# 0039 — Concurrent stdio lines preserve batch replies and recover from bad input

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed.
- **Date:** 2026-10-18
- **Amends:** [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
  §4's scalar-only bridge answer rule; preserves §2's HTTP envelope semantics.
- **Scope:** run 16, issues #72 and #90; [specification](../../specs/stdio-concurrency/spec.md).

## Context

The bridge serializes independent lines and mistakes arrays for notifications.
A successful batch can lose every reply; a wait can time out before the seed
on the next line is even submitted. UTF-8 decoding failure terminates the
bridge. These are transport defects, not changes to the server's store or
tool execution guarantees.

## Proposed ruling

1. Parse each nonblank input line once locally, from bytes. Invalid UTF-8 or
   malformed JSON receives one local -32700 with null ID and no HTTP POST;
   later lines continue. This selects #90's acceptance criterion over its
   conflicting prose about silently skipping invalid UTF-8. Process a final
   unterminated line at EOF; retain whitespace-line silence.
2. Share envelope classification beside `mcp::dispatch`. Bounds take
   precedence: empty/over-cap arrays are invalid batch envelopes. An admitted
   array answers when an element carries an ID; a single object answers when
   it carries an ID, including null. Other id-less parsed shapes remain
   silent. Both dispatch and bridge use this rule without suppressing the
   execution of notifications. Successful replies retain JSON values and
   batch order, serialized as one compact line. Empty HTTP bodies stay silent.
3. On transport failure, a single request receives its existing -32000 with
   its ID. An admitted batch receives one array of -32000 objects, one per
   ID-bearing element, in order, preserving duplicates/null. An empty or
   oversized batch receives one null-ID -32000, even if it contains IDs.
   Notifications remain silent even if a peer sends a spurious body. Failures
   mean uncertain execution; never retry or promise that a mutation rolled back.
4. Allow 16 outstanding HTTP exchanges, including notifications, using one
   `FuturesUnordered` collection and one loop owning stdout. Apply input
   backpressure at capacity. No task queue, writer channel, semaphore,
   submission-order output buffer or per-method priority. Ready exchanges
   may answer ahead of earlier lines; clients sequence dependent mutations
   themselves. Within one HTTP batch, the server's sequential order stays.
5. Offer a complete JSON line plus newline to `write_all`, then flush.
   Preserve stderr logging. On EOF drain admitted work, then exit 0. On
   stdin I/O failure stop admission, drain, then exit 1. Stdout write/flush
   failure is terminal: exit 1 and drop outstanding futures. Dropping sockets
   does not cancel effects already executing on the server. Network failures
   answer as above and do not terminate the bridge.
6. Retain existing HTTP parsing, five-second dial budget, derived exchange
   budget (`MAX_AWAIT_TIMEOUT_MS + BRIDGE_SLACK`, currently 65 seconds),
   response cap and identity-encoding-only posture. No line-byte cap is added.
   The concurrency limit bounds outstanding exchanges, not process memory.
   Slow stdout can indefinitely stall polling and EOF drain; network deadlines
   alone do not promise a wall-clock process exit bound.

## Alternatives and consequences

Adopt simplicity's single owner and futures collection; reject robustness's
spawned workers and separate writer queue. The price is no request-level panic
isolation and exchange polling stopping while stdout is blocked. Adopt
robustness's shared classifier; the price is a small MCP dispatch refactor
requiring semantic parity coverage. Reject the proposed input cap this run:
unbounded line allocation remains a real memory-exhaustion exposure and needs
a separately specified refusal/drain contract before repair.

For oversized batches reject robustness's per-element transport errors in
favor of one envelope failure, consistent with the existing HTTP refusal.
Clients that submit over-cap batches may see their per-ID promises expire.
Admitted batches do not have that avoidable correlation gap. Sixteen long
waits can still delay a seventeenth request. Cancellation, keep-alive, chunked
decoding and priority admission remain separate work. A panic or broken
stdout can lose replies; the bridge offers no durability promise.

## Guarantees and proof

Preserve synchronous insert-before-250 (0001), subscribe-before-check (0002),
oldest-only pruning (0003) and extraction at ingest (0005). In particular,
0002 makes concurrent wait/seed correct whether the seed or wait arrives first.
No store, persistence or protocol-listener changes are required.

The linked spec's AC1–AC9 require real `mcp` and `serve` subprocesses for
batch and concurrency regressions; malformed recovery, admission, EOF,
transport-failure shape, writer failure and HTTP parity are also pinned.
Use 0020 test placement and 0035 exact coverage rules; a red per child must
fail behaviorally, not merely fail compilation. Only the operator accepts
this document; do not mark 0029 amended before acceptance.

## Pointer — 2026-10-18 (proposed 0047)

[0047](0047-storage-rpc.md) §§3–4, also **proposed**, update §§1/2/6 here (issues #132,
#133): the shared classification owes an `id`-less object without a string `method` a
null-id `-32600`; the bridge dials port 80 for a URL that names no port, keeping `Host` as
spelled; and a line whose body exceeds the server's 2,097,152-byte limit is refused
locally, never posted, with `-32600` in the server's wording and the line's own ids.
Reading and parsing a line stay unbounded, as §6 names.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit. The operator accepted
this record and the later records below on 2026-10-19; the text above, the
0047 pointer included, stays as written, and that pointer's "proposed" is
historical. Each marker uses the later record's own verb and scope. The
amendment of 0029 §4 that the paragraph above deferred to acceptance is now
marked in [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md).

- *(Updated by [0044](0044-compat-validation.md) (accepted 2026-10-19):
  §§2–4's scalar-silence and sequential-server-batch descriptions — the
  server answers every malformed body with a JSON-RPC error and runs batch
  members concurrently. This pointer was missing until 2026-10-19.)*
- *(Updated by [0047](0047-storage-rpc.md) (accepted 2026-10-19): §§1/2/6,
  as the pointer above describes.)*

## Proposed reconciliation pointer — 2026-10-26

The operator-directed #152 implementation is recorded in proposed
[0079](0079-mcp-version-negotiation.md) and proposed
[0080](0080-mcp-source-reconciliation.md). §4 independent-line admission is fenced during initialization only; steady-state concurrency and §§2/6 complete-reply forwarding and identity-only parsing remain. The latter leaves the general HTTP client SSE obligation unresolved.
This dated pointer preserves the accepted historical body and status. It neither
accepts a proposal nor claims an incompatible amendment has been accepted.
See the [clause ledger](../../specs/mcp-version-negotiation/reconciliation.md)
for source evidence, compatible repairs, remaining costs and external ownership.
