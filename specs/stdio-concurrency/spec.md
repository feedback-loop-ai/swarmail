# Feature specification: concurrent stdio batches

Date: 2026-10-18. Design handoff for run 16, issues #72 and #90.
Implementation and operator acceptance remain outstanding.

## Feature and rationale

`swarmail mcp` must forward batch replies and allow independent input lines
to make progress concurrently. Today `request_id` treats every array as a
notification, discarding successful mutations' replies; the serial bridge
also parks a seed behind the wait it would satisfy. Invalid UTF-8 ends the
process and malformed JSON receives no parse-error reply.

Keep the HTTP endpoint as the executor of tools and batches. Repair the
bridge's classification, input recovery and scheduling together. This changes
the public stdio contract and amends decision 0029 §4; proposed
[0039 — Concurrent stdio batches](../../docs/decisions/0039-stdio-concurrency.md)
records that amendment. Only the operator accepts the decision. The HTTP
envelope semantics of 0029 §2 and frozen guarantees 0001–0003 remain intact.

## Framing and explicit panel rulings

Read both run-local positions, the feature file `.brokkr/state/run16-feature.txt`
and issue bodies `.brokkr/state/epic70/{72,90}.md`. The task directory contains
only older run-12 framing, not a run-16 intake document. The named run-16
feature and issue acceptance criteria therefore govern this design.

| Conflict or agreement | Ruling and cost |
|---|---|
| Both positions require batch replies, notification silence, local parse errors, bounded concurrency and EOF drain. | Adopt all five. Keep the HTTP implementation, transport deadlines and dependency set. |
| Simplicity proposes `FuturesUnordered`; robustness proposes semaphore + `JoinSet` + bounded writer channel for panic isolation. | Adopt the single-loop `FuturesUnordered` design with one stdout owner. No spawn, semaphore, channel or reorder buffer. Reject request-task panic isolation: an unexpected Rust panic can terminate the bridge and lose unrelated replies. I/O failures are handled, not panics. This avoids task/queue shutdown states that neither issue requires. |
| Simplicity keeps local classification; robustness requires a shared answer predicate beside the server envelope. | Adopt shared classification in `src/mcp.rs`, used by dispatch and the bridge. Pay for a small behavior-preserving server refactor and parity tests to remove the drift that caused #72. No general transport abstraction. |
| Robustness requires a line-byte cap and oversize-tail drain; simplicity defers it. | Defer the cap. It creates a new input refusal/resynchronization contract beyond the framed defects. The losing robustness position costs real protection: one arbitrarily long line can exhaust memory, and 16 exchanges do not imply a process memory ceiling. Document this residual without claiming bounded total memory. |
| Both favor per-ID errors; robustness extends them to oversized arrays with IDs, simplicity treats invalid batch envelopes as one failure. | Choose simplicity for empty/oversized envelopes: one null-ID transport error, matching their single upstream envelope refusal. No per-element promises are made for rejected batches. Clients sending oversized batches may still expire individual pending IDs; admitted batches receive per-ID errors. |
| #90 prose says skip non-UTF-8; its acceptance requires an error for each bad line. | The acceptance criterion wins: one local null-ID -32700 for each invalid UTF-8 or malformed JSON line. No POST and no exit. |
| #90 suggests one task per request. | Independent futures satisfy the required concurrency; separate Tokio tasks are an implementation suggestion. Each input array remains one sequential HTTP batch. |
| Robustness suggests a timing-margin ping test. | Prove response order and waiter registration causally, using the real five-second wait deadline as a bound, without elapsed-time assertions or sleeps. |

## Contract

One complete nonblank input line is parsed once by the bridge. The server
still parses its own HTTP body. Valid UTF-8 whitespace-only lines are ignored;
the final nonempty unterminated line is processed at EOF. Invalid UTF-8 is
never repaired with replacement characters. Parsed values, including silent
notifications and scalars, still reach the HTTP endpoint so notification
effects are preserved.

Classification checks the batch bounds before notification silence:

| Parsed input | Successful HTTP outcome | HTTP transport failure |
|---|---|---|
| Object with an `id`, including null | Forward JSON result/error | One -32000 with the original ID |
| Array of 1 through `MCP_BATCH_CAP` elements, with at least one ID-bearing element | Forward response array, preserving member order | One array of -32000 objects, one per ID-bearing element in input order |
| Empty or over-cap array, regardless of IDs | Forward the upstream single null-ID -32600 refusal | One null-ID -32000 object |
| Other parsed values without an ID, including valid all-notification arrays and scalars | No stdout, even if the peer sends a body | No stdout |

Preserve duplicate IDs, null IDs and their JSON values without coercion.
Do not recursively discover IDs inside nested arrays. Each batch remains one
output line, never one line per element. `Outcome::Empty` remains silent for
every input. Local malformed-line replies are one -32700 object with null ID,
with a message distinguishing invalid UTF-8 from JSON syntax failure.
Transport errors denote uncertainty about execution, not rollback; never retry.

At most 16 HTTP exchanges may be outstanding, including notifications.
`BRIDGE_CONCURRENCY` is independent of the coincidentally equal MCP batch cap.
Stop consuming additional lines at capacity; no queued futures waiting for a
slot. Emit ready replies without waiting for earlier lines, preserving order
only within a batch. Simultaneously ready exchanges have no promised ordering.
Clients needing ordered mutations must await a reply before submitting the
dependent line. A single writer emits complete compact JSON plus newline
through one `write_all` call, then flushes; no logging goes to stdout.
Whitespace normalization is allowed; response JSON values and array order
must remain unchanged. A write call is not a promise of OS-level atomicity.

EOF stops admission and drains admitted exchanges before exit 0. A genuine
stdin I/O error stops admission, drains admitted work while stdout works,
then exits 1. A stdout write or flush error immediately returns failure and
drops outstanding exchange futures. This closes sockets but cannot undo work
already submitted to the server. Transport failures answer by the table and
leave the bridge usable. Preserve the five-second dial deadline, derived
65-second exchange deadline and 10 MiB response-read cap. These bound network
legs when polled; a blocked stdout consumer can delay polling and EOF exit
indefinitely. No end-to-end 70-second exit guarantee is made.

## Acceptance Criteria

1. **AC1 — Batch parity (#72).** The real `swarmail mcp` subprocess talking
   to a real `swarmail serve` produces the same JSON array as HTTP for two
   ping requests, including both IDs. Mixed batches answer only ID-bearing
   entries. A batch with an unknown-method request preserves its error.
2. **AC2 — Silence and envelope refusals.** All-notification batches and
   single notifications produce zero stdout bytes on success and failure;
   blank lines and parsed scalars stay silent. Empty and 17-element arrays
   produce the server's single -32600 refusal, including over-cap arrays
   containing only notifications. A spurious peer body cannot defeat silence.
3. **AC3 — Failure correlation.** Failed admitted batches produce exactly
   one array line with one -32000 per actual ID, preserving null, duplicates
   and order, excluding notifications. Single requests preserve their IDs.
   Empty/oversized failed batches produce one null-ID error regardless of IDs.
   A subsequent healthy exchange succeeds; no automatic retry occurs.
4. **AC4 — Causal concurrency (#90).** Adjacent wait (4000 ms), matching seed
   and ping lines return seeded mail with `timed_out: false`. Separately,
   establish a five-second waiter through its labeled registration metric,
   submit ping, observe its reply before the wait, then seed to release the
   waiter. Run both through the real subprocess/server pair. Decision 0002
   makes the adjacent wait/seed case correct in either arrival order.
5. **AC5 — Malformed-line recovery.** Invalid UTF-8, malformed JSON and a
   valid ping produce exactly two null-ID -32700 objects and the ping reply,
   then exit 0 at EOF. The first two cause no HTTP POST, even with an
   unreachable server. Local errors use the common error envelope.
6. **AC6 — Admission and framing.** At most 16 exchanges execute, including
   notifications; the next line is not consumed at capacity and becomes
   eligible after completion. A response completing during a partial stdin
   line neither loses nor duplicates that line. CRLF input, whitespace and
   a final unterminated request work. Every emitted line parses independently
   as the expected JSON-RPC object/array, including a multiline JSON HTTP body.
7. **AC7 — Lifecycle and writes.** Closing stdin with concurrent work pending
   yields every admitted reply before exit 0. Injected stdin I/O failure
   drains admitted replies then returns 1. Write and flush failure return 1
   without waiting for outstanding long polls; recording-writer tests prove
   a complete line is offered in one `write_all`, with a subsequent flush.
8. **AC8 — Existing transport contract.** Existing long-wait, derived-deadline,
   dial/read refusal, status/body validation, response cap, empty-body,
   slash-safe URL, http-only message and stderr-only logging tests pass.
   A shared MCP classification test pins parity with actual dispatch for
   every row of the contract table; HTTP notification effects still execute.
9. **AC9 — Evidence and gates.** Record a behavioral red/green regression per
   child against the reviewed serial/scalar-only behavior, with commands,
   SHAs and failed assertions. All six framed gates pass, with coverage exact
   100/100/100 and no exclusions, allowlist changes or new dependencies.

## Scope and residuals

No changes to storage, persistence, SMTP, POP3, extraction, loopback guards,
listener recovery, gate scripts or delivery policy. Keep all run-16 named
preservation constraints, including the capped awaits registry and 30-second
test helper deadline. No TLS, pooling, chunked HTTP support, cancellation
registry, priorities, transactional batches or new CLI settings. Sixteen waits
can delay a seventeenth ping; cancellation notifications still do not cancel
server waits. Input allocation and slow stdout are unbounded residuals.
One TCP connection per POST retains ephemeral-port/TIME_WAIT pressure.
