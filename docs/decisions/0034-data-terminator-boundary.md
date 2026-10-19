# 0034 — Complete DATA boundaries and deliberate SMTP session transitions

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance is the operator's.
- **Date:** 2026-10-18
- **Issues:** #73, #81, #82 (run 13, epic #70).
- **Spec:** [SMTP session boundaries](../../specs/smtp-session-boundaries/spec.md).
- **Amends:** [0022](0022-bounded-data-raw-bytes.md), solely its earlier
  rejection of declared SIZE enforcement; preserves its bounded reader and
  byte policy. Strengthens the DATA boundary described by
  [0028](0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md) and
  narrows its §2 session behavior to one successful AUTH per TLS epoch.
- **Preserves:** frozen 0001–0003, per-test identity 0004, ingest extraction
  0005, existing awaited commit/parse ordering, 0020 and 0035.
- **Numbering:** 0034 is vacant on the baseline and expressly assigned to
  this file by the run-13 intake. 0035's historical result-sealing reference
  is not this ruling and is left untouched; this decision concerns SMTP only.

## Context

The LF-oriented reader currently accepts any `.\r\n` line as end-of-DATA,
even when the preceding payload line ended in bare LF. A body containing
`body\n.\r\nNOOP\r\n` is truncated and NOOP executed. The oversized-line
paths discard the ending, so a comparison-only repair is incomplete.

The same session keeps stale envelopes across greetings, accepts malformed
paths, ignores declared SIZE/unknown parameters and runs MAIL chaos before
the nested-MAIL refusal. PLAIN selects a second field from a malformed
two-field credential, exposing a password as an inbox. Repeated AUTH changes
identity. These are public contract changes even though no frozen guarantee
is weakened; this proposed record covers the combined batch.

## Ruling

1. **The preceding CRLF is part of recognizing end-of-DATA.** A whole dot-CRLF
   line is a terminator only after a CRLF-ended wire line, or immediately
   after 354 for empty DATA. The bounded reader reports the actual ending
   of discarded oversized lines, including CR/LF across buffer boundaries.
   EOF without LF supplies no valid boundary. Boundary recognition does not
   depend on stored bytes or available accumulation capacity.
2. **All other payload retains 0022's exact byte policy.** Bare LF, NUL,
   binary bytes and mis-preceded dot lines are payload. Only leading `..`
   loses one dot. No normalization, new raw representation or size unit.
   The preceding payload CRLF stays stored. Oversized DATA drains to the
   real boundary before its existing 552 and envelope consumption. Incomplete
   DATA closes on EOF/idle expiry without a store or mid-DATA reply.
3. **Envelope refusals preserve the transaction.** HELO/EHLO reset it like
   RSET; HELO replies once, EHLO keeps its capabilities. MAIL/RCPT require
   their correct case-insensitive keyword and angle-bracketed path; only MAIL
   may have an empty path. Keep local and UTF-8 paths. State, syntax,
   parameter checks and RCPT capacity precede chaos and mutation. Nested MAIL
   always answers 503 and keeps the old sender and recipients. Consume an
   envelope at the true DATA terminator before every completion outcome.
4. **Declared SIZE now refuses impossible requests early.** Amend 0022's
   rejected SIZE-enforcement alternative: decimal values above the configured
   cap, including all-digit overflow, answer 552 at MAIL. Invalid values
   answer 501. The actual DATA cap remains authoritative regardless of the
   declaration. Support existing advertised `BODY=7BIT|8BITMIME` and bare
   `SMTPUTF8`; invalid recognized values/duplicates get 501, unknown MAIL
   parameters and all RCPT parameters get 555. Spec.md defines the full
   deterministic syntax and error precedence table.
5. **Only a well-formed AUTH can select identity, once per TLS epoch.** PLAIN
   requires exactly three NUL-separated fields and a strict UTF-8, trimmed,
   nonempty authcid; malformed credentials get 501 and change nothing.
   Preserve accept-any passwords (including empty) and ignored authzid, with
   no credential database or 535. After successful AUTH, further AUTH returns
   503 without a prompt, even for identity default. Failed attempts permit
   retry. Missing mechanism or surplus tokens is 501, unknown mechanism 504,
   disabled AUTH retains 502, and AUTH during an envelope retains 503.
   RSET/HELO/EHLO preserve identity; successful STARTTLS clears identity and
   envelope along with buffered plaintext. LOGIN keeps its existing forms
   and password/cancellation behavior.
6. **Retain specific sink leniencies, named in README.** MAIL before greeting
   and greetings without a hostname remain accepted. Do not add hostname/DNS
   verification or full mailbox grammar. These do not license malformed path
   framing or additional AUTH fields.

## Rejected alternatives and costs

- **Treat every drained line as CRLF:** retains the injection/truncation gap
  when an oversized LF line precedes dot-CRLF. **Treat it as LF:** loses the
  legitimate oversize client's terminator and 552. Actual metadata costs a
  bit plus split-boundary tests and preserves both contracts.
- **Normalize line endings or create a second scanner:** normalization
  changes stored mail under 0022; a second scanner creates competing boundary
  owners. Retain the existing bounded reader. Cost: LF-only clients ending
  with `\n.\r\n` must provide a valid later boundary or time out.
- **Keep bracketless/inverted paths for old tests:** rejected in favor of
  clear framing. Cost: affected nonconforming clients must update. Local
  addresses and SMTPUTF8 still work; full mailbox validation remains outside
  scope.
- **Return 501 for digit overflow:** rejected because it is syntactically
  decimal and necessarily larger than the configured cap. Cost: explicit
  syntax-versus-overflow handling, still bounded by the command-line cap.
- **Separate mutable sender/recipients and duplicate AUTH-success bit:** use
  a private optional envelope and optional authenticated identity instead.
  Cost: update local state uses; benefit: reset and success cannot drift
  between parallel fields. No general state-machine framework is introduced.
- **Permit identity switching on repeated AUTH:** rejected. Cost: callers
  needing another inbox open a fresh connection (or complete STARTTLS first).
  Refused credentials still never mutate the selected inbox.

## Consequences and proof

The SMTP sink becomes stricter at payload, envelope and credential boundaries
while retaining byte fidelity and declared capabilities. A successful final
250 still follows the existing awaited parse/commit and means queryable;
Store::insert remains synchronous. Watcher subscription, pruning, storage
schema and extraction timing do not change.

The spec's AC1–AC13 require real-server regressions red on `bacadbe` and green
after repair for each child, exact raw/envelope/inbox assertions, deterministic
reader splits, TLS reset and real smtplib compatibility. Six house gates,
including exact coverage without exclusions, remain mandatory. This record
does not assert implementation, operator acceptance or gate completion.

## Pointer — 2026-10-18 (proposed 0046)

[0046](0046-smtp-pop3-edges.md) §§3–4, also **proposed**, updates §§3/5/6 here (issues
#127, #128). A nonempty path that is blank after Unicode trimming, or that carries a
control character, is refused `501` like any other malformed path. A quoted local part
holding `>` and a literal source route are named limitations, not supported grammar.
AUTH names pass the shared inbox rule untrimmed.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit. The operator accepted
this record and the later record below on 2026-10-19; the text above, the
0046 pointer included, stays as written, and that pointer's "proposed" is
historical. Each marker uses the later record's own verb and scope.

- *(Updated by [0046](0046-smtp-pop3-edges.md) (accepted 2026-10-19):
  §§3/5's path and trimmed-authcid wording, as the pointer above describes —
  blank or control-bearing paths refuse `501`, and AUTH names pass the
  shared inbox rule untrimmed.)*
