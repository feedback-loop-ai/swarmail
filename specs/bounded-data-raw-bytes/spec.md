# Spec — Bounded DATA path, real 8BITMIME, byte-exact `/raw`

Epic: feedback-loop-ai/swarmail#13, wave 6 (issues #8 and #16 — #16 lands
with #8 by design). Branch: `feat/epic13-08-16-raw-bytes`.

Decision: 0022 —
[docs/decisions/0022-bounded-data-raw-bytes.md](../../docs/decisions/0022-bounded-data-raw-bytes.md)
(status **`proposed`** — awaiting operator; it records the public-contract
deltas and every ruled conflict with the losing side's cost).

Diff surface: `src/smtp.rs`, `src/api.rs`, the proving tests, plus two
deliberate one-liners outside it (`src/compat.rs` gains a visibility word,
`src/static/openapi.json` one description string). No new dependencies.

## The feature

Three contracts, one repair each:

1. **#8 — a documented per-message byte cap that bounds the DATA *path*.**
   The cap exists (`SmtpConfig::max_message_size`, 50 MiB, advertised as
   `SIZE`, refused with `552 5.3.4`, nothing stored) but today it is
   consulted only after each line has been fully buffered
   (`src/smtp.rs:416` → `:429`): an unterminated line grows the heap
   unbounded *before the cap is ever read* (measured: 60 MiB in, 60.1 MiB
   RSS grown, then the `552`). The cap must bound buffering too: the wire
   line is bounded at `cap + 1` during the read, over-cap input is drained
   to the terminator without buffering, and the refusal answer, the
   nothing-stored rule and the in-sync session are exactly today's.
2. **#16a — 8BITMIME accepted, body bytes preserved verbatim.** `250-8BITMIME`
   is advertised (`src/smtp.rs:262`) but the DATA body is read through a
   `String` (`read_line`, `:416`), so any non-UTF-8 byte kills the session
   mid-DATA — abrupt close, no reply, nothing stored. The reader becomes
   byte-oriented; hostile bytes become the *supported* case; what arrived is
   what is stored, line endings included, no normalization anywhere.
3. **#16b — the API returns the EXACT received bytes.** `get_raw`
   (`src/api.rs:365-370`) re-encodes via `from_utf8_lossy` and answers
   `text/plain; charset=utf-8` — the one endpoint whose contract is
   exactness is a charset re-encoding surface. A binary body uploaded by
   SMTP must round-trip byte-identical. `compat::raw_response`
   (`src/compat.rs:601-611`) already owns that exact shape
   (`message/rfc822`, raw `Vec<u8>`); `get_raw` delegates to it, and
   `openapi.json:245` is repaired to match README:232, which has promised
   `message/rfc822` all along.

## Why

The three contracts are advertised on the wire today, and all three are lies
at their contract surface — measured against the real server at HEAD
`9f1c6e3`, not argued from doc comments. A mock server that destroys bytes
or dies on `0xFF` is worse than no 8BITMIME advertisement at all: it makes
the agent integrations this realm exists for debug a heisenbug instead of
reading a mail. The repair is small because the realm is already
byte-honest everywhere else — `Email.raw: Vec<u8>` (`src/model.rs:53`), a
raw BLOB in persist with a non-UTF-8 round-trip unit test, `build_email`
parsing `&[u8]` — the two liars are the DATA reader and `get_raw`, and the
fix is one reader plus one delegation.

## The ruling — position synthesis

Both positions converge on the same skeleton: one bounded, byte-oriented
reader in the DATA arm (cap enforced *during* the read, O(1) once over,
drain-to-terminator, existing `552`, nothing stored, session in sync) and
`/raw` serving stored bytes verbatim as `message/rfc822`. The synthesis
adopts that skeleton and rules the four genuine deltas:

- **ADOPTED — simplicity's `+1` wire-line bound** (`cap + 1`, the
  dot-stuffing slack) **reconciled with robustness's accumulator
  enforcement.** The reader bounds the wire line at `cap + 1` and signals
  over-cap mid-line; the existing accumulation check
  (`raw.len() + payload.len() > cap`) stays the sole refusal authority.
  The `+1` makes the two never disagree: a wire line of exactly `cap + 1`
  stuffed bytes unstuffs to exactly `cap` (legal at-cap mail), and any
  longer line has unstuffed payload `> cap` whether stuffed or not. One
  constant, one comment, one pinning test — no semantic edge to test
  around. Cost of the alternative (robustness's rolling-tail discard-scan):
  more machinery for the same bound, and a second refusal authority to keep
  in agreement.
- **ADOPTED — robustness's verbatim corollary** (line endings stored as
  received, `CRLF` and bare `LF` alike, `size == raw.len()` of unstuffed
  bytes). This is today's behavior made explicit and pinned; no
  normalization is introduced anywhere between wire and response bytes.
- **REJECTED — robustness's operator knob** (`SWARMAIL_MAX_MESSAGE_SIZE`
  env + CLI). The decisive fact: the boundary tests override the cap
  in-process *today* (`tests/smtp_edges.rs:174`, cap 64), README:202 already
  documents the 50 MiB cap, and `SmtpConfig::max_message_size` stays the
  single owner from which the `SIZE` advertisement, the refusal check and
  the line bound all derive — pinned by one test that sets the cap and
  checks advertisement *and* enforcement. *What the losing side costs:* an
  operator needing a different cap must rebuild; the follow-up is one arg,
  one assignment and a `config_cli` test if the story ever materializes.
- **REJECTED — robustness's RFC 5321 receiver-rule rewrite of
  dot-unstuffing** (`any` line starting with `.` loses its first byte, so a
  non-conformant `.foo` would store `foo`). The incumbent `..` predicate is
  exactly correct for a conformant sender, and the contracts govern those
  bytes. *What the losing side costs:* a non-conformant sender's `.foo`
  line is stored verbatim rather than RFC-unstuffed — pre-existing,
  unchanged, and a two-line follow-up if the operator asks.
- **SCOPED OUT — robustness's command/AUTH line capping** (`500 5.5.2 Line
  too long`), which robustness itself concedes the chief may scope out and
  simplicity cuts. *Cost accepted and documented (decision 0022):* a hostile
  peer can still grow one session task's heap with an unterminated command
  line — pre-existing, one task, idle posture unbounded by documented
  design; the follow-up is small *because* the reader lands in `smtp.rs`.
- **ADOPTED — a decision document (against simplicity's cut #6).** The wave
  changes observable public-contract behavior (`/raw` headers and bytes,
  session survival on non-UTF-8 DATA, the DATA memory shape); the 0010
  precedent records exactly this class rather than leaving rulings in a
  diff, and the ruled conflicts above need an operator-visible record.
  *Cost (simplicity's side, accepted):* one more file of prose at review.

Also refused, with both positions behind it: no truncated prefix store (the
silent truncation the contract forbids), no `552`-and-close (a pipelining
client must be able to *hear* the rejection — drain-then-refuse is the only
posture that guarantees it), no `SIZE=` enforcement on `MAIL FROM` (a
client's claim, not a bound), no JSON re-encoding (a JSON string cannot
carry invalid UTF-8; the contract names the retrieval path), no
cross-protocol reader unification with pop3 (two callers do not amortize a
parameterized abstraction), no global byte budget (collides with 0003's
oldest-only rule; belongs to the operator as its own decision), and
`coverage(off)` nowhere.

## Non-goals

Streaming/chunked acceptance; partial stores on over-cap; DATA read
deadlines (the plaintext idle posture is unbounded by documented design);
backpressure; base64 JSON views; POP3/SMTP reader unification; any new
dependency. The RSS measurement that proved the buffering defect stays
run-local evidence (`.forge/tmp/`): the committed suite proves behavior;
the memory bound is structural, pinned by the drain branch and the in-sync
test, not by a flaky process-memory assertion.

## Acceptance Criteria

1. A DATA body containing bytes outside UTF-8 (`0xFF`, `0xFE`, `0x00`, a
   `NUL`, a bare `CR`) is accepted with `250`, stored, and served
   **byte-identical** through `GET /api/v1/messages/{id}/raw`
   (`assert_eq!` on `Vec<u8>`), with the response `Content-Type` equal to
   `message/rfc822` and the stored `size` equal to the body length.
2. The same byte-identical round-trip holds **over STARTTLS** and **across
   a data-file restart** (serve → drop → respawn on the same file →
   `/raw` identical).
3. A dot-stuffed line containing hostile bytes (and a line beginning with a
   literal `.`) unstuffs and round-trips exactly: `..x` stores `x`;
   the terminator check works on bytes (`b".\r\n"` / `b".\n"`).
4. An over-cap **unterminated** wire line (dribbled in small writes, no LF,
   then terminated) is drained without buffering: the reply is the existing
   `552 5.3.4 Message too big`, nothing is stored (inbox count unchanged),
   the server answers `/healthz` afterwards, and the **same connection**
   completes a following small mail with `250`.
5. A message of **exactly** `max_message_size` unstuffed bytes is accepted
   with `250` — including when it arrives as one dot-stuffed wire line of
   `cap + 1` bytes — pinning the `+1` slack; a `cap + 1` payload is refused
   with `552` and stores nothing.
6. With the cap overridden to a small value, `EHLO` advertises
   `250-SIZE <cap>` **and** DATA enforces exactly that cap — one source of
   truth, advertised == enforced.
7. `250-8BITMIME` remains on the EHLO reply (and now means it: criterion 1).
8. `GET /api/v1/messages/{id}/raw` answers with the exact stored bytes and
   `Content-Type: message/rfc822` (plus the shared owner's
   `Content-Disposition: attachment; filename="<id>.eml"`), and
   `openapi.json` describes the route as `message/rfc822`. Existing
   UTF-8 `/raw` assertions keep passing unchanged (body bytes identical).
9. Verbatim line endings: a body sent with `CRLF` and bare-`LF` lines is
   stored and served with exactly the line endings it arrived with — no
   normalization at ingest, none at serve.
10. All six gates are clean (`fmt`, `clippy -D warnings`, `test`,
    `scripts/coverage-gate.sh` at the exact 100/100/100 floor,
    `cargo deny check licenses`, `brokkr compile --bundle .`), with no new
    dependencies and every new production branch executed by the suite.
