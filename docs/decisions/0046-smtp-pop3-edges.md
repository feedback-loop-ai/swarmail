# 0046 — SMTP and POP3 transport progress and verbatim identities

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance is the operator's.
- **Date:** 2026-10-18.
- **Issues:** #121, #126, #127, #128, #131; epic #120 batch 1.
- **Design:** [spec](../../specs/smtp-pop3-edges/spec.md),
  [plan](../../specs/smtp-pop3-edges/plan.md),
  [tasks](../../specs/smtp-pop3-edges/tasks.md).
- **Proposes amendments to:** [0027](0027-living-listeners-bounded-sessions.md)
  §4's operation-level POP3 write bound and SMTP cleanup scope;
  [0028](0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md) §2's
  explicit base64 → UTF-8 → trim username rule.
- **Updates proposals:** [0034](0034-data-terminator-boundary.md) §§3/5's
  path and trimmed-authcid wording; [0036](0036-smtp-trickle-lifetime.md)
  §§4/5's POP3 write scope; [0044](0044-compat-validation.md) §3's shared
  admission rule. Strengthens the webhook filter admission covered by
  [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md).
- **Preserves:** 0001–0005, ordered mutations, 0020 test placement, 0035's
  coverage gate, existing storage and delivery safeguards. No frozen
  guarantee is weakened. 0046 is the next free number at this design's base.

## Context

A timed-out SMTP reply enters unbounded TLS shutdown, retaining a session
slot after its I/O deadline. Neither protocol sets TCP_NODELAY, so pipelined
groups incur delayed-ACK latency. Opaque SMTP paths admit blank/control
strings. AUTH and USER trim names that HTTP keeps verbatim, and control
characters can expose credential material in inbox names. POP3 documents an
idle bound but places one deadline around a potentially large whole reply.

Run 2's patch supersedes run 1 and contains the local repairs, two declared
cold-path harnesses and a server write observer. This run ratifies that
design with corrected handoff and proof wording. Prior gate/red reports
remain unverified claims until rerun on the final implementation. Client
receive duration alone cannot establish server progress, and elapsed write
durations cannot literally identify pending polls.

## Proposed ruling

1. A failed SMTP session returns its original error and drops its transport
   without further peer I/O. A successful session attempts graceful shutdown
   under one SMTP idle window; cleanup error/expiry remains best effort.
   JoinSet retains ownership until the connection task finishes. The already
   bounded STARTTLS handshake keeps its dedicated configured deadline. A
   failed socket dropped with unread input may send RST rather than FIN;
   this is intended, not a reason to reintroduce peer I/O after failure.
2. Set TCP_NODELAY on every accepted SMTP and POP3 socket before any reply,
   including cap refusal. Setter failure drops that connection and leaves
   the listener accepting. No reply-batching framework is introduced.
3. Reject a nonempty SMTP path blank after Unicode trim, or any path with a
   `char::is_control` character. Only literal `<>` is the null sender; RCPT
   rejects null. Refusals retain current state precedence and preserve the
   envelope before chaos/mutation. Accepted paths are verbatim, padding
   included. Quoted `>` remains unsupported; source routes remain literal.
   Document these limitations instead of broadening the parser in this batch.
4. One `validate_inbox` rejects blank-after-Unicode-trim names and controls
   (C0, DEL, C1) and returns every valid name unchanged. AUTH PLAIN authcid
   and LOGIN username use strict decoding without trimming. Invalid eligible
   credentials answer 501 5.5.2 and leave identity unchanged. This explicitly
   proposes replacing accepted 0028's trimming rule; it is not a claim that
   verbatim AUTH was already the accepted contract. Preserve existing framing,
   state refusal precedence, accept-any passwords and successful AUTH limits.
5. POP3 USER receives the exact argument after one separating space; strip
   only the actual line delimiter and decode command bytes strictly. Invalid
   USER clears pending identity and answers `-ERR`. Non-UTF-8 commands end
   unanswered before dispatch, including PASS; lossy replacement cannot select
   a mailbox. Other command argument rules remain unchanged.
6. REST, MCP, UI, compat, seeds and webhook filters use the same predicate.
   Refuse before effects; invalid webhook PUT preserves the installed list.
   Webhook targets are memory-only, initialized empty by run_on, with no
   persisted configuration loader. Historical inbox rows are not rewritten;
   control-bearing names can remain discoverable but validated by-name reads,
   waits and clears refuse. Document global/ID cleanup where available,
   without claiming these names will automatically age out.
7. POP3 session output uses a partial-write progress loop. Each positive
   local `write` count renews the window for the remaining bytes; zero on a
   nonempty slice is WriteZero. Error/expiry ends the session. Keep the 600 s
   default, command-read deadlines and whole-operation accept-cap refusal.
   All session replies use this owner. SMTP writes, dot stuffing, octet
   counts, snapshot metadata, fetch-from-store and deletion semantics stay.
8. A reply has no total deadline or minimum rate. A progressing peer can
   retain its slot and assembled reply indefinitely. Progress means a local
   write completion, not remote application consumption; socket buffering can
   hide a stopped reader until backpressure occurs. No availability or global
   memory guarantee is added. State this consistently in module docs, README
   and this decision. At defaults, `max_sessions × max_message_size` is
   256 × 50 MiB = 12.5 GiB of raw-message-scale exposure, not a memory
   ceiling: wire encoding expansion, retained mail/snapshots, allocation and
   socket overhead are additional. The change permits indefinite retention
   of assembled replies. POP3 is plaintext today; future STLS must reassess
   the deadline around flushing, since TLS writes may only buffer records.

## Alternatives and costs

Both panels' small runtime designs are adopted. Reject granting a failed TLS
writer another cleanup window: the original timeout has already established
failure. Cost of dropping it is possible missing close_notify/reset; clean
sessions keep bounded graceful close and can spend one further idle window
there (300 s by default). TCP_NODELAY costs a syscall and drops
one socket if setting the option fails.

Quote-aware parsing and source-route stripping are declined: the issue
permits documented limitations, and rewriting accepted envelope data is a
larger contract change. Valid quoted-angle paths remain unsupported; literal
routes and padding stay stored exactly. A shared normalized name/type or
migration would expand this batch and change existing valid HTTP identities.
Verbatim names instead require callers to send exact padding. Unicode format
characters are not controls; this rule does not eliminate visually confusing
identities. Non-UTF-8 POP3 commands, formerly lossily decoded, now disconnect.

The whole-reply deadline is rejected because it truncates otherwise
progressing large replies (50 MiB / 600 s implies roughly 87 KB/s). Fixed-size
`write_all` chunks still impose a per-chunk transfer threshold; direct partial
writes express the selected contract. A total lifetime or minimum rate would
require new policy without a framed requirement. The chosen cost is indefinite
slot and reply-buffer retention, potentially across all admitted sessions.

**Dated clarification — 2026-10-18, run 3:** both current panels support
run 2's cold-path placement and local runtime repairs. Adopt robustness's
RST, memory-exposure and future-STLS disclosures, plus armed-chaos refusal
proof. Reject its literal as-is ratification where stale instructions and
proof claims remain. On the observation dispute, simplicity prevails: the
existing observer measures write entry/completion/count, not pending polls.
Retain it with a server span beyond one window, delayed positive writes,
exact received bytes and a behavioral truncation red with the old deadline.
Add the progress harness's missing independent whole-test deadline and
failure-safe abort/join cleanup; its current happy-end cleanup is insufficient.
A 1 s window/50 ms per 256 KiB pace is a fixture, not synchronization. The
cost is that individual pending polls remain unobserved; narrow test-only
observation and harness code still carry the proof. Both panels now agree
the handshake is bounded and webhook targets have no persistence loader.
No new runtime architecture follows from those resolved concerns.

## Proof and decision discipline

Spec AC1–AC12 and the ordered paired tasks require real-protocol red/green
evidence for each changed behavior, exact stored bytes/IDs, no-effect
refusals, an every-caller inventory, pipelined measurements, server-observed
progress and stalled-reader release. Keep inline branch-stable tests and
place only concrete cold paths in declared sibling harnesses. Run all gates
at exact 100/100/100; never reduce coverage or expand dependencies to fit.

Implementation indexes this as proposed and adds dated proposal pointers
without editing accepted historical bodies. The user's commission requests
these repairs, but only the operator can accept this record. The document
does not claim implementation, gate completion, a signed commit or shipment.
