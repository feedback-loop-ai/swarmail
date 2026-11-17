# Each tool's best evidenced wait

Issue #361, decision 0132 story C. Initial design, 2026-11-17; based on
story A at `9ac77db`. There is no review return in this run.

## Authority and purpose

Implement accepted [0132](../../docs/decisions/0132-mail-comparison-protocol-v4.md)
§7, §9–11 and Implementation story C, including its 2026-11-16 acceptance.
That acceptance already authorizes streams, 10 ms tight polling and the
1,000 ms backstop. This design implements that contract without amending a
ruling or a server contract, so it declares no new decision.

The comparison currently measures a common poll instead of the wait a user
would choose. V4 must compare the best evidenced method beside the same
100 ms poll and a separately labelled 10 ms poll, including the target CPU
cost. Notifications wake authoritative scans; they never supply the answer.
This makes lossy channels usable without claiming that they are lossless.

## Scope and synthesis

Adopt simplicity's existing `Owned::find`/`FindScan` and `runner::observe`
ownership, one minimal shared `clients/stream.rs`, and story A's launch,
resource, cleanup and admission machinery. Reject a client-framework rewrite,
second supervisor, message cache, replay engine or per-event task. The cost
is retaining the existing scanner interface until stories B/D extend it;
there is no shared `clients/` directory on this base to assume already built.

Adopt robustness's readiness evidence, cancellation-safe parsing, independent
backstop and bounded wire handling. A sequential loop is acceptable only if
connection/readiness and scanning cannot hide the timer or lose partial
bytes. Reject the interpretation of simplicity that merely awaiting a
reconnect inline is sufficient. The cost is explicit pending connection,
scan and notification state; it buys bounded behavior during disconnects.
No detached reconnect service is permitted.

Rule for evidence over convenience: neither HTTP 101 nor an arbitrary delay
proves registration. A ping/pong or application signal qualifies only when
pinned source establishes its ordering after registration. Mailpit and
MailHog remain candidates, not preselected streams. A documented gap permits
labelled polling; it cannot be described as proof that no channel exists.
Reject choosing polling merely to avoid investigating an evidenced channel.
The cost is source inspection and a separate pinned-binary proof for every
selected stream; some targets may honestly remain polling.

Adopt both positions' v4-only accounting and historical compatibility. Reject
one exclusive outcome that conceals duplicate plus unrelated answers: retain
both facts even where an existing primary outcome is kept. Extra notification
events are not duplicate answers. This costs additive v4 facts and admission
checks, not a second measurement ledger.

## Acceptance Criteria

1. **Evidence and selection.** Every one of the nine draft-v4 `native_wait`
   cells is resolved with pinned evidence; none remains `not_inspected`.
   Lock v5 wait documents record path, byte count, SHA-256 and attribution,
   validated and rechecked at use. Each best-method justification states
   why higher-priority methods do not apply: cited blocking native wait,
   else qualified stream, else 100 ms poll labelled `no documented wait`
   with a cited absence or precise evidence gap. Blocking wait support and
   push-channel support are distinct facts. Swarmail uses REST await.
   Official MailSlurper stays `does_not_start`, blocked with no number.
2. **Stream qualification.** Every selected stream cites route, producer,
   client registration and observable readiness ordered after registration.
   Its pinned-binary proof establishes readiness, sends by real SMTP, then
   obtains an event-triggered authoritative match. Finding mail only by an
   initial or backstop scan does not qualify the channel. No candidate is
   promoted based on an upstream claim from memory or a successful upgrade.
3. **Wire clients.** `clients/stream.rs` supplies a minimal RFC 6455 client
   and SSE reader using already locked tokio/ring/base64 facilities. The
   WebSocket handshake checks status, Upgrade/Connection tokens and the
   accept key derived from a fresh random key using ring. It handles split
   and coalesced input, continuation and interleaved control frames,
   client masking, ping/pong and close; rejects invalid opcode, RSV,
   masking, length, control FIN/size, text UTF-8 and unsupported negotiation.
   SSE validates status/content type and HTTP transfer framing, including
   chunked bodies, split UTF-8, CR/LF boundaries and multi-line data.
   Comments and incomplete events at EOF are not notifications. Headers,
   lines, frames, assembled messages/events and buffers have tested finite
   limits checked before allocation; trickle input cannot renew a deadline.
4. **Readiness and scans.** Stream observation establishes cited readiness
   before its complete initial scan. Later complete scans follow delivery
   events, re-established readiness after reconnect and the independent
   1,000 ms backstop. The complete existing scanner and exact match checks
   are authoritative, including pagination and multiplicity. Events cannot
   satisfy a wait, supply content, or shortcut a scan to an event's id.
   Failed/incomplete scans are errors, not empty answers.
5. **Bounds and cancellation.** One absolute observation cutoff covers
   opening, readiness, scans and reconnection, with existing request-bound
   grace and late/censored semantics retained. No post-cutoff scan succeeds
   as timely. Established connection loss permits bounded reconnect attempts
   without disabling backstop scans; initial readiness failure is an error,
   never silent method fallback. Malformed transport is an error, not an
   empty timeout. Event floods, trickle reads and failed reconnects cannot
   starve cutoff/backstop. Pending parser bytes survive cancellation; scans
   serialize, event coalescing retains a wake that arrives during a scan,
   and every owned connection/task ends before observation cleanup completes.
6. **Profiles and lifecycle.** V4 has `wait-common` (100 ms for every target),
   `wait-best` (criterion 1), and `wait-tight` (10 ms for every target,
   labelled no tool's documented method). Each has 100 present, 100 future
   and 20 empty waits per repetition, three repetitions, 30 s regular and
   1 s empty cutoffs, a fresh instance per profile, and the accepted enforced
   resource budget. Stream backstop is exactly 1,000 ms. Validators reject
   wrong/missing/duplicate modes, citations, profiles, counts, intervals,
   pairings and v4-only fields on historical protocols. Restore the draft's
   wait operation budget to the longest wait plus one request bound.
7. **Rows and timing.** Every started operation keeps a row. Stream answers
   carry `initial_scan`, `event`, `reconnect` or `backstop` from the scan
   that produced them. Rows retain expected occurrences and unrelated-answer
   counts before deduplication. Reports count duplicate and unrelated facts
   independently, including both in one answer; a correct empty wait is
   not a missed expected message. Expected mail with no timely return is
   missed/censored, with late and transport failure reasons retained.
   Preserve v2 present/future timing and signed ack offsets. Future
   `ack_to_return` is signed client-return minus SMTP-250 timestamp, never
   an event timestamp and never clamped to zero; unavailable timestamps
   remain unavailable. SMTP rejection/ambiguity stays separate from wait
   outcome, with no automatic resend or restart.
8. **Resource and admission integration.** Waits use story A's chunk owner,
   sampling and profile windows, target CPU accounting and cgroup cleanup
   receipts. Chunk/operation deadlines cover wait-path sends and QUIT too.
   Reports and admission recompute method/profile counts, timing, observation
   causes, independent outcome counts and resource summaries from validated
   rows; tampered facts, missing rows/windows/receipts and wrong pairings
   refuse by name. OOM/target exit remain measured failures; infrastructure
   or unproved cleanup remains exit 2. Only the waits owed marker is cleared
   when the entire path works; API/testexec debt and draft refusal remain.
9. **Real proofs.** Each selected stream has its own named `#[ignore]` proof
   against digest-checked executable/runtime bytes, failing rather than
   skipping when absent. Run individually under `timeout 600 taskset -c
   0-31`, with load before/after and cleanup evidence. Document exact names,
   environment variables and commands after evidence selects the rows.
   Default tests do not discharge this criterion. No selected stream may
   ship with its proof merely written but not executed successfully.
10. **Compatibility and scope.** No `src/`, adapter tool, dependency package
    set, license allowlist or accepted-decision body changes. Preserve
    [0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
    [0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
    [0003](../../docs/decisions/0003-oldest-only-pruning.md) and
    [0005](../../docs/decisions/0005-extraction-at-ingest.md).
    Old protocol/lock/collection bytes, serialization and admission behavior
    remain unchanged. Active v3/lock-v4 files stay active until story E;
    this story edits `docs/evals/draft/` only for v4/v5. No freeze, client
    hash fabrication, collection, publication, headline, paid run, push or
    merge. Necessary wrapper wiring preserves story A's bounds.
11. **Verification.** Deterministic parser and state-machine regressions,
    real loopback protocol tests and all selected pinned-binary proofs pass.
    Use barriers or controlled clocks, never sleep as synchronization or
    probabilistic schedules. Pass the complete
    [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
    the locked comparison bench build and decision inventory tests. Keep
    CI-owned proofs distinguished from locally executed checks. Design
    validation alone does not establish implementation completion.
