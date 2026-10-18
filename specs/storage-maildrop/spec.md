# Storage and maildrop repairs

Date: 2026-10-18. Design status: drafted; implementation remains outstanding.
Scope: epic #70 children #71, #83, #84, #86 and #87 in this Rust crate.
Reviewed baseline: `25ad4df`, after the DATA-boundary and listener repairs.

## Feature and rationale

POP3 must release a blocked writer at its configured deadline, advertise the
size the client actually retains, and avoid copying mail merely to count it.
SMTP and POP3 single-line replies must send text and CRLF together so a
sequential client does not pay the reproduced delayed-ACK penalty. DELE must
honor the existing missing-message contract. Operators must be told the real
memory and timestamp semantics, and existing data files must stop maintaining
an unused index.

The applicable intake is
`.brokkr/state/intake/run15-storage-maildrop-framing-08e65395.md`, together
with `.brokkr/state/run15-feature.txt` and the five issue bodies. The files
under `.brokkr/state/tasks/` concern older run-12 work; none frames run 15.
Both `.brokkr/state/design/positions/simplicity.md` and `robustness.md` were
considered. This document owns the synthesis; run-local evidence is not a
deliverable or an alternative specification.

This change amends public contracts through two new decisions, both
**proposed**, with acceptance reserved for the operator:

- [0037 — POP3 size accounting](../../docs/decisions/0037-pop3-size-accounting.md)
  amends 0028 §1's dot-inclusive count and scratch-allocation requirement.
- [0038 — Store memory and timestamps](../../docs/decisions/0038-store-memory-and-timestamps.md)
  amends 0021's bounded-residue claim and 0026's total-memory sizing claim,
  and records parse-start timestamp semantics.

Existing 0021, 0026 and 0028 stay byte-identical. Their amendments live in
0037/0038, discoverable through the decision index and current documentation.
0034, 0035 and 0036 remain untouched.

## Panel rulings

| Conflict | Ruling and reason | Cost of declining the alternative |
|---|---|---|
| #71 timeout handling | Adopt both positions' single `bound_io` around each complete LIST/UIDL reply, and robustness's explicit silent write-timeout exit. No goodbye write, no timer around a commit. | A non-reader can still occupy a slot for POP3's default 600 seconds. Complete commands can retain a session indefinitely under 0036. |
| #83 socket policy | Adopt simplicity's complete-buffer `write_all`, retaining SMTP flush in the same bound. Reject optional TCP_NODELAY in this batch: the observed split is repaired directly and the required bench judges the result. | Future multi-write shapes could reintroduce latency; the structural regression prevents this shape in the helpers. If the bench fails, the task remains unfinished; no waiver or silent socket-policy change. |
| #84 derivation | Adopt robustness's one serializer with a counting sink that excludes transparency dots; retain simplicity's allocation-free, PASS-time derivation off the async worker. Reject an ingest-time cache/Email field and reject independent counting and writing parsers. | Each login still scans O(maildrop bytes), and concurrent logins can contend on the blocking pool. No schema, model-field, restore-recomputation or cache-consistency obligations are introduced. |
| #86 RETR | Adopt simplicity's one reply buffer filled directly from the live Email Arc. Reject robustness's streaming adapter: the requested bound is one additional representation and complete-buffer replies keep one deadline and one write. | At 256 retrievals of approximately 50 MiB each, about 12.5 GiB of additional reply payload remains possible before normalization, stuffing and allocator overhead. This is not an RSS ceiling. |
| #86 snapshot and lookup | Adopt both positions' live existence check before DELE marking and direct map lookup for USER/PASS. Offload the entire PASS snapshot, not only its byte counter. | Deletion after the DELE check remains possible; QUIT's idempotent ordered deletion handles it. Listings remain stale snapshots and no maildrop lock is added. |
| #87 watcher lifetime | Choose simplicity's documented-residue alternative, expressly allowed by the issue. Reject last-receiver guards and sweeps near the proven subscribe-before-scan lifecycle. | Distinct awaited names can retain channels indefinitely; roughly 780 B/name was observed on the review build, with additional variable costs. No aggregate registry bound or reclamation promise is made. |
| #87 global cap | Choose the documented absence of a global cap. Reject cross-inbox eviction and defer robustness's admission cap, typed insert error and accounting knob. | Inbox cardinality and total resident memory remain unbounded. An overloaded process can OOM and repeatedly OOM during restore. A future admission cap must specify restore and all accounting semantics independently. |
| #87 timestamps | Keep parse-start stamps and document their divergence from acceptance order. Reject monotone acceptance stamps and a new clock seam. | Cross-session timestamp lists can disagree with per-inbox acceptance order and pruning. Even monotone milliseconds alone would not establish strict total order when UUID tie-breaks retain parse order. |
| #87 persistence | Adopt both positions' `DROP INDEX IF EXISTS` for existing files, and robustness's insistence that the plan test exercise the actual full restore query. | A small open-time schema operation remains; no migration framework or replacement index is justified. |

Decision 0005 continues to govern extraction of links/codes at ingest.
Transport-size derivation at PASS is the existing 0028 boundary computation,
not reparsing or extracting message content. The sink changes its allocation
cost and dot treatment; it does not move extraction into a query.

## User scenarios

1. A client stops reading LIST or UIDL. The reply deadline expires, its
   connection ends, and a later peer can use the released session slot.
2. A sequential SMTP client sends one command at a time. Replies arrive as
   complete lines; STARTTLS still receives its 220 before the TLS handshake.
3. A POP3 client retrieves dot-leading or non-CRLF mail. Advertised size
   equals the normalized payload after unstuffing; UIDLs and payload bytes
   otherwise retain their established meaning.
4. HTTP removes a message after POP3 PASS. Both RETR and DELE answer
   `-ERR no such message`, while snapshot numbering and listings stay stable.
5. An operator sizes a long-running instance or restores a data file.
   Documentation exposes unlimited inbox cardinality, watcher residue and
   full restore costs instead of promising a global bound.

## Acceptance Criteria

- **AC1 (#71):** Multiline LIST and UIDL each execute one complete-buffer
  write under the configured `bound_io` deadline. Expired writes terminate
  silently, with no follow-up write. A real-socket backpressure test for
  each command proves session release and re-admission under a real test
  deadline. A genuinely full cap still refuses; ordinary response framing
  is unchanged except the size corrections specified in AC4. Shutdown drains
  the tracker, and POP3's production default remains 600 seconds.
- **AC2 (#83):** Both single-line helpers make exactly one application-level
  `write_all` for text plus CRLF under the existing bound. SMTP's flush is
  retained within that bound, with the 220 flushed before STARTTLS takes the
  plain stream. Structural tests pin the call shape and bytes; real TLS
  handshake and sequential SMTP/POP3 tests remain green. This is not a
  promise of one kernel syscall, TLS record or TCP packet.
- **AC3 (#83):** A real loopback bench measures a sequential SMTP transaction
  below 5 ms per command in the reproduction's debug profile. Record build,
  sample count, per-command means and p50/p99, and baseline/repair results.
  Each measured command's mean must be below 5 ms; include MAIL, RCPT, DATA
  prompt and final DATA acceptance separately. No timing threshold enters
  `cargo test`; a failed bench remains an unmet criterion.
- **AC4 (#84):** PASS/RSET summaries, STAT, both LIST forms and RETR's header
  use one normalized, unstuffed payload count, excluding the status line and
  final terminator. The 30-octet fixture with three dot-leading lines reports
  30 everywhere, while its transmitted payload occupies 33 octets. Test
  bare LF, bare CR, mixed endings, empty bytes, absent final newline and
  8-bit bytes. Preserve existing line normalization and RETR/TOP payload
  behavior; verify the received unstuffed payload against an independent
  expected-byte oracle and pin UIDL stability across DELE/RSET.
- **AC5 (#86):** After external removal, DELE returns `-ERR no such message`
  without adding a mark or changing snapshot numbering. Correct the existing
  test's contradictory comment and assertion. Live-message marking, RSET,
  idempotent QUIT and partial-commit failure behavior remain covered.
- **AC6 (#86):** RETR owns at most one additional full payload representation:
  its final reply buffer. The body is appended directly from the live Arc;
  there is no intermediate stuffed body or deep Email/raw clone. A structural
  regression or allocation bench fails the old two-buffer shape. Reply size,
  stuffing, terminator and one bounded write remain correct.
- **AC7 (#84/#86):** Size counting allocates no per-message payload buffer.
  The entire PASS snapshot/list/reverse/size operation runs in one awaited
  offload for every inbox size; only metadata returns to the session. A
  deterministic real-server interleaving proves HTTP `/healthz` responds
  while actual large-PASS work is held off the sole async worker. No sleep
  or elapsed-millisecond assertion proves this property.
- **AC8 (#86):** `Store::has_inbox` uses one `inboxes.contains_key` lookup.
  USER and PASS use it, with the PASS recheck preserved. Tests cover present,
  absent and emptied inboxes, including after restore; structural inspection
  proves name enumeration and sorting are absent from those paths.
- **AC9 (#87):** README, store module documentation, chart sizing comments
  and proposed 0038 state: the per-inbox message cap is not a global byte or
  RSS cap, zero disables it, inbox count is unlimited, and all persisted
  rows restore before bind without application of that cap. Memory includes
  raw/decoded/extracted fields, indexes, watcher-held Arcs and concurrent
  buffers. No claim that database file size equals RAM consumption remains.
- **AC10 (#87):** Proposed 0038 amends 0021's residue claim with the observed
  approximately 780 B per distinct awaited empty name, explicitly a
  build-dependent observation rather than a fixed ceiling. State retention
  until existing clear/empty/restart paths, reuse for repeated names, and
  the difference from the separate 4,096-name await-metrics registry cap.
  Registry-count tests preserve residue, reuse, release, atomic first
  subscriptions, Closed-arm resubscription and lag handling. Do not claim
  timeout or cancellation releases the channel or returns RSS to baseline.
- **AC11 (#87):** README and 0038 state that `received_at`/`received_ms`
  sample `build_email` entry before parsing and acceptance; they are not
  sender Date or commit sequence and need not sort in acceptance order
  across sessions or clock steps. Per-inbox order, POP3 numbering, oldest
  pruning and SQLite sequence order remain acceptance-based. Deterministic
  fixtures with deliberately reversed timestamps prove pruning/restore
  follow insertion order, without requiring elapsed-time or monotonic-clock
  assumptions. Existing within-session ordering tests remain green.
- **AC12 (#87):** Fresh and legacy databases have no `emails_inbox` index
  after open, retain `emails_seq`, and preserve all rows/counters. Reopen is
  idempotent. The plan test EXPLAINs the exact full-column restore query used
  by `load`, examines all plan rows, finds `emails_seq` and no temporary
  sort. A legacy-file real-server restart test combines SMTP/POP3 preservation
  with the absent-index assertion, giving #87 a concrete production red pin.
- **AC13 (contracts):** README, helper comments, tests and proposed decisions
  express the same rules. The index lists 0037/0038 as proposed. Existing
  0021/0026/0028 and 0034/0035/0036 are byte-identical to baseline. No new
  dependencies, license exceptions, global cap, timestamp rewrite, watcher
  lifecycle change or socket-option change is introduced.
- **AC14 (proof and gates):** Record red/green evidence per child against
  `25ad4df` and all six gates at exact 100/100/100 coverage, plus the bench.
  Compilation failure is not negative proof. For #83 the structural test and
  bench are the issue's expressly permitted red proof. For #87's unchanged
  runtime alternatives the red proof is the changed documentation contract;
  the index-removal restart supplies additional real-protocol production
  proof. Do not label unchanged behavior repaired. Preserve the frozen
  guarantees and every intake do-not-undo constraint.

## Scope limits and delivery

No implementation, benchmark result, green gate or operator acceptance is
implied by this design. SMTP commit-before-250, synchronous `Store::insert`,
subscribe-before-scan, oldest-only pruning and ingest extraction remain
mandatory. Preserve DATA framing/session rules, reap-after-accept,
Pending/Ok recovery, 0036 trickle wording, the 40-character reference window,
eager interrupt, fallible `search_needle`, loopback guard, compat staging,
page caps, await-metrics cap and the 30-second helper deadline.

The design deliverables are exactly this spec, plan.md, tasks.md and the two
linked proposed decisions. Implementation later follows the framed branch,
PR, 17-check, review and ship workflow. This seat never pushes or closes an
issue. An inability to write Git metadata must be disclosed as an uncommitted
draft; passing file-shape checks cannot substitute for the requested commit.
