# 0028 — Wire-honest POP3 octets, AUTH refusals that change nothing, and the snapshot-not-lock maildrop

- **Status:** accepted (operator, 2026-10-16 — "accept all")
  Original draft status: proposed (awaiting operator — acceptance is the operator's;
  the batch ships together with this decision)
- **Date:** 2026-10-15
- **Amends:** [0004](0004-per-test-inboxes-via-smtp-auth.md) (accepted) §1
  and §3 — the amendment is a *refusal posture*, not a new identity rule:
  the username still names the inbox, and credentials that fail to name
  one now refuse instead of silently succeeding.
- **Records:** the POP3 octet rule (issue #44) and the maildrop
  snapshot-not-lock posture (issue #45) — postures the tree documents for
  the first time, with the alternatives rejected and priced.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md) — untouched (0003 is *why* the lock
  is refused, §3); [0005](0005-extraction-at-ingest.md) — untouched: no
  ingest normalization, no new `Email` field; [0015](0015-boundary-failure-contract.md)
  — implemented more honestly, not amended: §1's "nothing reports deleted
  and resurrects" becomes true; [0020](0020-concurrency-tests-and-the-coverage-gate.md)
  — its disciplines bind every test this batch adds.

## Context

Four protocol-conformance defects (issues #42-#45), two of which are
contract changes the issues name explicitly:

1. **POP3 advertises one size and sends another.** STAT, LIST, the
   PASS/RSET summaries and RETR's own `+OK n octets` answer the stored
   raw length while RETR rewrites the wire — every bare LF becomes CRLF
   and every dot-leading line is stuffed (`src/pop3.rs`
   `write_stuffed`/`write_stuffed_limited`). For any mail with bare-LF
   lines the advertised size is short. README documented the old
   behaviour and `tests/pop3.rs` pinned it.
2. **AUTH failures succeed.** `AUTH LOGIN` ignored its initial response
   and answered an unconditional `235` for undecodable credentials, a `*`
   cancel, and an empty username — silently keeping the previous or
   `default` inbox. That is a cross-test contamination channel: parallel
   tests pooling one inbox is exactly what 0004 exists to prevent, and
   `from_utf8_lossy` mints a *second* channel (every malformed username
   collapses into the same U+FFFD-minted inbox).
3. **The POP3 session view has no recorded posture.** The maildrop is a
   PASS snapshot with no lock; listings and fetches can disagree after
   mail is removed elsewhere, and nothing written says that is the
   intended shape — so the next run would rediscover it as a bug.

## Decision

### §1 — A POP3 size is the length of the bytes RETR transmits

For every POP3 size — RETR's `+OK n octets`, `STAT`, the `LIST` argument
and its multi-line form, the `PASS` and `RSET` summaries — the count is
the length of the bytes RETR would transmit for that message:
**CRLF-terminated, dot-stuffed, excluding the final `.\r\n`.** For plain
CRLF mail that equals the stored raw length (existing pins keep their
values); for any mail with bare-LF or dot-leading lines it is larger.

The count is **derived by running the writer, never stored a second
time**: the session derives sizes from the same `write_stuffed` logic
RETR uses — RETR counts its own body buffer, and the PASS snapshot
derives each `MaildropMsg::size` through the same one function. There is
no mirror of the wire logic anywhere in `src/`, so no drift is possible
by construction; there is no pairing test, because there is nothing to
pair.

This is RFC 1939's end-of-line accounting as the issue cites it (§11: a
server that stores a one-character end-of-line counts it as two octets)
made structural: the server's own transmission is the unit of count.

**Rejected alternatives, priced:**

- *An ingest-time wire-size column / a new `Email` field:* duplicates the
  one quantity the issue is about, drags the persistence schema and a
  migration, and must be kept consistent with a writer that can still
  change — the same drift, moved into storage. Also touches the REST/MCP
  contract (`Email.size` keeps its meaning: the stored raw length, served
  byte-honest at `/raw`).
- *A mirrored counting function* (`wire_octets` walking the same rules as
  the writer): a second copy of the wire logic plus a permanent pairing
  test that exists only to detect the drift the shape invites.
- *A size cache:* a cache can drift; a derivation cannot.
- *Ingest-time CRLF normalization:* rewrites stored mail (the `/raw`
  contract and 0005's byte-honesty) to fix an advertisement — the wrong
  side of the mismatch.

**Cost accepted:** one scratch-buffer allocation per counted message
(bounded by `MAX_MESSAGE_SIZE` and the per-inbox cap; the same order as
the buffers RETR and the `store.list` snapshot already compose — one
reused buffer is fine), and a PASS-time O(inbox bytes) derivation pass
(the snapshot already materializes every mail).

### §2 — `235` means the credentials named an inbox; any failure is a `501` that changed nothing (amends 0004)

In accept-any mode, **`AUTH PLAIN` and `AUTH LOGIN` assign the session's
inbox only on success.** The LOGIN initial response (RFC 4954 §4) is the
username; the username is decoded strictly (base64 → UTF-8 → trim);
a credential that fails to name an inbox — undecodable base64, an empty
decoded username, a `*` cancel at any prompt, a non-UTF-8 username —
answers `501 5.5.2` and **changes nothing**: the previous inbox (or
`default`) stays, and a later delivery flows there.

`535 5.7.8` is deliberately absent, not forgotten: accept-any has no
credential *validity* to reject, so 535 would assert a check that does
not exist. It is **reserved** for the day credentials are checked; a 535
that never fires is dead-arm vocabulary the exact 100/100/100 coverage
gate makes a permanent tax.

**Cost accepted:** the wire cannot distinguish "malformed credential"
from "credential that names nothing" — no client can act on the
distinction, and the failure is visible in the reply code (`501`) rather
than a silent `235`. **Consequence:** per-test isolation (0004) is
restored for the smtplib shape — `AUTH LOGIN <b64-username>` prompts only
for the password and lands in the named inbox.

### §3 — The maildrop is a snapshot, not a lock

A POP3 session's view is the PASS-time snapshot: **listings answer from
the snapshot, fetches answer from the store.** `STAT`/`LIST`/`UIDL`
report the snapshot (numbering stable mid-session, per the module's
contract); `RETR`/`TOP` answer `-ERR no such message` when the store no
longer holds the mail (cleared over HTTP, deleted in another session, or
pruned); `DELE` on such a number answers the same. No maildrop lock is
taken.

**Why not a lock:** honoring a lock that keeps a snapshot's mail
fetchable would require exempting locked inboxes from oldest-only pruning
(0003) — unbounding memory under a hung client — and HTTP bypasses the
lock entirely (the same mail is served from the same store at `/api/v1`),
which would make the lock security theater.

**What a session may rely on:** its own `DELE` marks (until `QUIT`),
its own committed deletes (never resurrected — 0015 §1, now implemented
with a `gone` marker on the slot), and the numbering of its snapshot.
**What it may not:** that mail it did not delete is still fetchable.

**Cost accepted:** `LIST`/`STAT` over-report for mail removed elsewhere
until the session ends, while `RETR`/`TOP` answer honestly — a bounded,
documented skew instead of an unbounded lock.

## Consequences

- `src/pop3.rs` derives sizes at the boundary and carries the `gone`
  slot; `src/smtp.rs` consumes the envelope at the terminator, refuses
  out-of-sequence commands, terminates on `.` CRLF only, and refuses AUTH
  credentials that name nothing. **Zero new `src/` files, zero new
  dependencies, zero new types.**
- `README.md` states the octet rule; the module docs state the snapshot
  posture and the refusal posture; the pinned silent-`235` tests and the
  pinned stored-size expectations are rewritten.
- The exact 100/100/100 coverage gate is the design instrument: every new
  line and branch ships with a wire leg or a branch-stable inline unit
  test; `coverage(off)` is forbidden.
- Both this decision and the batch remain `proposed` until the operator
  accepts; until then the tree is judged against this text only by this
  run's own review.

## Pointer — 2026-10-18 (proposed 0046)

[0046](0046-smtp-pop3-edges.md) §4, which is **proposed**, would amend §2 (issue #128):
the AUTH username and the POP3 `USER` argument are no longer trimmed. After strict
decoding both pass the shared `model::validate_inbox` rule, which refuses a blank name and
any Unicode control character and keeps every other name verbatim, padding included. A
refusal still answers `501 5.5.2` or `-ERR` and changes nothing. Nothing above is
rewritten by this pointer.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit, after the operator's
2026-10-19 ruling accepting 0034–0050 and 0055. The text above — the ruling
and the 0046 pointer — stays as written; each marker names a later record
with that record's own verb and scope, and nothing above is rewritten by it.

- *(Amended by [0034](0034-data-terminator-boundary.md) (accepted
  2026-10-19): strengthens the DATA boundary this record describes and
  narrows §2's session behavior to **one successful AUTH per TLS epoch**.
  §2's "the previous inbox (or `default`) stays" still holds for a refused
  credential, but it no longer implies a session may AUTH again: after a
  successful AUTH, a further AUTH answers `503` without a prompt, even for
  identity `default`; failed attempts may retry; RSET/HELO/EHLO keep the
  identity and a successful STARTTLS clears it (0034 §5). PLAIN requires
  exactly three NUL-separated fields.)*
- *(Amended by [0037](0037-pop3-size-accounting.md) (accepted 2026-10-19):
  §1's size definition, derivation mechanism and accepted allocation cost.
  A POP3 size counts the normalized CRLF payload **without** the
  transparency dots RETR adds (RFC 1939 §11), so "dot-stuffed" in §1's rule
  no longer describes the count. §§2 and 3 continue to govern.)*
- *(Amended by [0046](0046-smtp-pop3-edges.md) (accepted 2026-10-19): §2's
  explicit base64 → UTF-8 → trim username rule, as the 0046 pointer above
  describes — names are no longer trimmed and pass the shared
  `model::validate_inbox` rule verbatim. The pointer's "proposed" is
  historical: 0046 is accepted.)*

## Erratum — 2026-10-24 (issue #184)

This correction qualifies §2 and the 0046 pointer above; it records existing behavior and changes no protocol rule. SMTP AUTH refusals have distinct precedence and replies: disabled AUTH returns 502; a mail transaction in progress or repeated AUTH returns 503 before credential decoding; an unsupported mechanism returns 504; eligible AUTH syntax or credential failures return 501. These refusals do not assign or alter the SMTP inbox identity. One successful AUTH per TLS epoch remains the rule under 0034.

POP3 authorization behaves differently: USER clears the pending username before validating its argument. A failed USER returns -ERR, leaves no pending identity, and makes a subsequent PASS return -ERR USER required first. Therefore the 0046 pointer's general refusal wording must not be read as saying a failed POP3 USER preserves pending identity. Strict verbatim name validation and all accepted SMTP/POP3 behavior remain unchanged.
