# 0022 — The DATA path is bounded and byte-honest; `/raw` serves the stored bytes

- **Status:** proposed (awaiting operator)
- **Date:** 2026-10-12
- **Supersedes:** none (strengthens the wire behavior of 0001 and 0005; touches neither's ruling)
- **Provenance:** originally filed as `0011-bounded-data-raw-bytes`;
  renumbered 0011 → 0022 at issue #32's docs pass — two 0011 records
  predate it on main, and the shim-delete contract kept 0011.
- **Companion:** epic #13 wave 6 (issues #8 and #16), branch
  `feat/epic13-08-16-raw-bytes`; spec at
  [`specs/bounded-data-raw-bytes/spec.md`](../../specs/bounded-data-raw-bytes/spec.md)
- **Numbering:** 0011 was the next free number on this branch at filing. The two
  provisional 0010s (`watcher-channel-lifetime`, now [0021](0021-watcher-channel-lifetime.md),
  on the unmerged wave-3 branch, `exact-count-asserts-and-the-shared-seed`
  merged) are already
  noted in their own records; if the operator renumbers at acceptance,
  this file moves with the spec reference.

## Context

The framing's three contracts are all advertised today and all three lie at
their contract surface. Measured against the real server at HEAD `9f1c6e3`
(probe preserved run-local under `.forge/tmp/`):

1. **The cap bounds what is stored, not what is buffered.** The DATA arm
   (`src/smtp.rs:405-457`) reads through tokio's `read_line` into a `String`
   (`:416`) and consults the cap (`:429`) only per *terminated* line.
   `read_line` does not return until LF or EOF, so a hostile peer streaming
   one unterminated line grew the server's RSS by the full size of the line
   *before the cap was ever consulted* — 60 MiB in, 60.1 MiB RSS grown, only
   then `552 5.3.4`. The wire is bounded only in the happy case.
2. **8BITMIME is advertised but a non-UTF-8 body kills the session.**
   `250-8BITMIME` is on the wire (`:262`), but `read_line` requires valid
   UTF-8 and the `?` propagates out of `session()`: a body containing
   `0xFF 0xFE 0x00 0x80` ends with an abrupt close mid-DATA — no reply, no
   `552`, nothing stored, no error the client could react to.
3. **`/raw` re-encodes by its own admission.** `src/api.rs:365-370` returns
   `String::from_utf8_lossy(&e.raw)`; axum stamps it `text/plain;
   charset=utf-8`. A binary body uploaded byte-perfect is served destroyed
   with U+FFFD mush. README:232 and the compat path's own OpenAPI entry
   (`src/static/openapi.json:166`) already promise `message/rfc822` — the
   swarmail-native endpoint and `openapi.json:245` are the outliers.

None of this touches the frozen guarantees: 0001's synchronous insert,
0002's subscribe-before-check and 0003's oldest-only pruning are inputs to
the wave, not surfaces of it, and extraction stays at ingest (0005). What
the wave does change is observable public-contract behavior — the `/raw`
response's headers and bytes, the session's survival on non-UTF-8 DATA, and
the memory shape of the DATA path — so the *rulings behind the change* are
recorded here rather than left in a diff, per the 0010 precedent.

## Ruling

**1. The DATA path reads bytes through one bounded reader; the cap is
enforced during the read.** The DATA arm stops using `read_line`. A
byte-oriented helper (shaped after `pop3.rs`'s proven `read_line_capped`/
`drain_line` pair, bytes out, cap in) reads through `fill_buf`/`consume` up
to and including the LF. The wire-line bound is `max_message_size + 1`: the
`+1` is exactly the dot-stuffing slack, so a wire line of `cap + 1` bytes
unstuffs to *exactly* `cap` payload bytes (a legal at-cap message) and any
line longer than `cap + 1` has unstuffed payload `> cap` whether it was
stuffed or not. Over the bound, the reader drains to the terminator without
buffering and the arm answers the **existing** `552 5.3.4 Message too big`;
nothing is stored; the session stays in sync and completes a following small
mail with `250`. The existing accumulation check (`raw.len() +
payload.len() > cfg.max_message_size`) remains the sole refusal authority —
the reader bounds memory, the check decides refusal. Session DATA memory is
therefore O(cap) while under and O(1) once over; the longest wire line is
bounded by the operator's cap. EOF mid-DATA keeps today's semantics: partial
input is not a message, nothing is stored.

**2. Verbatim means verbatim, including line endings.** What arrived is
what is stored: `CRLF` and bare `LF` alike, `NUL`s and `0x80` bytes with no
normalization at ingest and none at serve. `size` remains `raw.len()` of the
unstuffed bytes. The dot-unstuffing predicate is **unchanged**: a line
starting with `..` drops one byte; everything else is stored as received.

**3. 8BITMIME becomes true.** Once no DATA byte is routed through a
`String`, a non-UTF-8 body is the supported case, not a killed session. The
advertised extension and the accepted bytes finally agree.

**4. `/raw` serves the stored bytes verbatim as `message/rfc822`.**
`get_raw` delegates to the exact-bytes owner that already exists —
`compat::raw_response` (`src/compat.rs:601-611`), made `pub(crate)`, one
visibility word. The observable deltas on `GET /api/v1/messages/{id}/raw`:

| | before | after |
|---|---|---|
| Body | `from_utf8_lossy` text (U+FFFD on hostile bytes) | the exact stored bytes |
| `Content-Type` | `text/plain; charset=utf-8` | `message/rfc822` |
| `Content-Disposition` | absent | `attachment; filename="<id>.eml"` (rides along from the shared owner) |

For UTF-8 mail the body bytes are unchanged; the headers change for every
mail. Both raw endpoints (`/api/v1/messages/{id}/raw` and the compat
`/api/v1/message/{id}/raw`) become byte- and header-identical: one owner of
the exact-bytes shape, not two. `openapi.json:245` is repaired to say
`message/rfc822`, matching README:232, which documented this shape all
along.

**5. The cap stays a compiled default.** `SmtpConfig::max_message_size`
(50 MiB) remains the single number: the `SIZE` advertisement, the refusal
check and the wire-line bound (`cap + 1`) all derive from it, pinned by one
test that sets the cap to 64 and checks advertisement *and* enforcement.
No env/CLI knob ships in this wave.

## Rejected alternatives — and what rejecting them costs

These are the panel's conflicts, ruled; the losing side's cost is named so
the operator can overrule with eyes open.

- **An operator knob (`SWARMAIL_MAX_MESSAGE_SIZE`, env + CLI).** Rejected.
  The boundary tests override the cap in-process today
  (`tests/smtp_edges.rs:174`, `max_message_size: 64`), README already
  documents the cap, and a knob nothing demanded is surface to document,
  configure and keep at the exact-100% gate. *Cost:* an operator who needs a
  different cap must rebuild. The follow-up is one `#[arg]`, one assignment
  in `From<&ServeArgs>`, and a `config_cli` test — a defensible ordinary
  change if the story materializes, not a redesign.
- **The RFC 5321 §4.5.2 receiver rule for dot-unstuffing** (strip the first
  byte of *any* non-terminator line starting with `.`, so a non-conformant
  `.foo` stores `foo`). Rejected. For a conformant sender the incumbent
  `..` predicate is exactly the RFC; a line starting with a single `.` that
  is not the terminator cannot be sent by one. Changing the predicate
  rewrites stored bytes for input the contracts do not define and would ride
  a semantic change through a repair wave. *Cost:* a non-conformant sender's
  `.foo` line is stored verbatim as `.foo`, not unstuffed — pre-existing
  behavior, unchanged; a two-line follow-up with its own test if the
  operator wants RFC-exactness.
- **Capping command/AUTH-continuation lines (`500 5.5.2 Line too long`).**
  Out of this wave. The framing bounds the DATA path; command verbs are a
  different path with pinned behavior. *Cost:* a hostile peer can still grow
  one session task's heap with an unterminated command line — pre-existing,
  one task, and the plaintext idle posture is unbounded by documented
  design. The byte reader lands in `smtp.rs`, so the hardening follow-up is
  small.
- **`SIZE=` enforcement on `MAIL FROM`.** Rejected: the declared size is the
  client's claim, not a bound; enforcement lives where the bytes are.
  *Cost:* a client may lie about `SIZE`; nothing trusts it.
- **Truncated stores, `552`-and-close, JSON re-encoding, a global byte
  budget, cross-protocol reader unification, `coverage(off)`.** All refused.
  A truncated prefix is the silent truncation the contract forbids; an
  abrupt close can make a pipelining client die on a broken pipe and never
  *hear* the `552` — drain-then-refuse is the only posture in which the
  rejection is guaranteed deliverable. JSON views cannot carry invalid
  UTF-8 at all, so "exact bytes" has no meaning there. A global byte budget
  changes which mail is evicted and collides with 0003's oldest-only rule;
  it belongs to the operator as its own decision. A shared reader needs a
  parameterized cap, a parameterized return type and two drain behaviors
  for exactly two callers.

## Consequences

- A `250` still means queryable (0001): the reply is written only after the
  synchronous `store.insert` returns, and the reader change sits entirely
  before the chaos check and the insert. A `552` now also means the server
  never buffered the body it refused.
- Binary mail round-trips: wire → store → `/raw` byte-identical, over
  plaintext and STARTTLS, and across a data-file restart. Binary bodies
  yield fewer `text`/`html` views — pre-existing for any parse-resistant
  mail; extraction (0005) is about *when*, not about inventing views.
- Over-cap rejection costs bandwidth (the body is drained and discarded),
  not memory or honesty. Hostile peers can waste wire time; they cannot
  waste heap or make the server lie.
- Two caps compose: worst-case DATA memory is `max_per_inbox ×
  max_message_size`; the README states the product. A global byte budget
  remains refused until the operator asks in its own decision.
- The exact 100/100/100 coverage gate applies to every branch the reader
  adds; the RSS measurement that proved Finding 1 stays run-local evidence —
  the committed suite proves behavior, not process memory.
