# Plan — Bounded DATA path, real 8BITMIME, byte-exact `/raw`

Implementation plan for
[`spec.md`](spec.md) (epic #13 wave 6, issues #8 + #16). Ruling and
rejected alternatives live in
[`docs/decisions/0022-bounded-data-raw-bytes.md`](../../docs/decisions/0022-bounded-data-raw-bytes.md)
(proposed).

## Technical context

- The SMTP session reads through `BufReader<Conn>` (`src/smtp.rs:212`),
  where `Conn` is the transport enum (plaintext TCP or TLS after
  STARTTLS). A byte-oriented reader over `AsyncBufRead` is automatically
  transport-agnostic — the same code serves plaintext and TLS, which is
  why one STARTTLS test proves the contract there once.
- The in-tree precedent for a capped line reader is `pop3.rs`'s
  `read_line_capped` + `drain_line` pair (`src/pop3.rs:78-130`):
  `fill_buf`/`consume`, stop buffering at the bound, drain to the
  terminator, keep the stream in sync. The DATA helper reuses that *shape*
  with two differences — bytes out (never `String`) and the cap in as a
  parameter (commands keep their `const`, DATA derives `cap + 1` from the
  message cap). No unification: each path keeps exactly one owner.
- The refusal authority is already correct and pinned:
  `raw.len() + payload.len() > cfg.max_message_size` → `over_limit` sticky
  → drain the rest → `552 5.3.4 Message too big` (`src/smtp.rs:429-437`,
  `tests/smtp_edges.rs:172`). The change touches *where the cap is
  consulted* (during the read, not after the line arrives), not the
  refusal semantics.
- Tests run real servers in-process via `tests/common/mod.rs`
  (`start_smtp(SmtpConfig { .. })` / `start_with(..)`); the boundary tests
  set `max_message_size: 64` exactly like the existing 552 test. No knobs,
  no env, no external client crates.

## Constitution check

| Guarantee | Verdict |
|---|---|
| 0001 — accept == stored | **Untouched.** The `250` is still written only after the synchronous `store.insert` returns (`src/smtp.rs:452-456`); the reader change sits entirely before the chaos check and the insert. The `552` path stores nothing and says so. |
| 0002 — await subscribes before it checks | **Untouched.** No change to the long-poll path. |
| 0003 — pruning is oldest-only | **Untouched.** No eviction change. The two caps' product (`max_per_inbox × max_message_size`) is documented in README as the worst-case envelope; a global byte budget remains refused. |
| 0005 — extraction at ingest | **Untouched.** `build_email` keeps its `&[u8]` signature and runs at insert; a binary body simply yields fewer `text`/`html` views — pre-existing for any parse-resistant mail. |
| Dependency policy | No new dependency; `deny.toml` untouched. |
| Coverage | Exact 100/100/100 — every new branch has a named test below; `coverage(off)` forbidden. |

The `/raw` header/byte delta and the DATA memory-shape change are public
contract behavior → recorded in decision 0022 (`proposed`), per the 0010
precedent. Nothing here weakens a frozen guarantee, so no acceptance is
required for the guarantees themselves — only for the observable deltas.

## Project structure — files touched

| File | Change |
|---|---|
| `src/smtp.rs` | One free byte-oriented bounded reader beside the DATA arm (~25–35 lines, one small enum, `Line`/`OverCap` vocabulary); the DATA arm (`:405-457`) rewires to it: bytes accumulate, terminator checked on `b".\r\n"`/`b".\n"`, unstuffing on bytes (`starts_with(b"..")` → drop one byte — predicate **unchanged**), `over_limit`/`552`/insert ordering exactly as today. Doc comment states the `cap + 1` slack. |
| `src/api.rs` | `get_raw` (`:365-370`) becomes a three-line delegation to `compat::raw_response(&e)`; return type `ApiResult<Response>`. Net-negative diff: the lossy conversion is deleted, not paralleled. |
| `src/compat.rs` | One visibility word: `fn raw_response` → `pub(crate) fn raw_response`. |
| `src/static/openapi.json` | `:245` description `text/plain` → `message/rfc822` (aligning with README:232 and the compat entry at `:166`). |
| `README.md` | The SMTP surfaces sentence (`:202`) gains the cap semantics clause (over-cap → `552`, nothing stored, DATA never buffers past the cap; bytes verbatim, binary round-trips byte-identical through `/raw`); the API section notes the two caps' product as the worst-case envelope. |
| `tests/common/mod.rs` | `SmtpConn::data_bytes(&[u8])` (send raw bytes + `.` terminator, read the final reply) and `http_get_bytes(addr, path) -> (u16, String, Vec<u8>)` (raw TCP, status, header block, body split on the first `\r\n\r\n`). No new dependency. |
| `tests/smtp_edges.rs` | New: binary round-trip, over-cap drain/sync, at-cap `+1` pin; extended: EHLO `250-8BITMIME` assertion, one binary-body chaos case. |
| `tests/starttls.rs` | New: binary round-trip over an upgraded session. |
| `tests/persistence.rs` | New: binary body survives the data-file restart round-trip. |
| `docs/decisions/0022-bounded-data-raw-bytes.md` | Proposed decision (committed with this spec). |

## Data model & contracts

- **Reader contract** (the only new vocabulary): one enum with
  `Complete(Vec<u8>)` / `OverCap` / `Eof`. `Complete` carries the wire line
  exactly as received, LF included (identical to today's `read_line`
  semantics, bytes instead of `String`). The bound is `cap + 1` on the
  accumulated wire line; once blown, drain to the terminator (or EOF)
  without buffering and return `OverCap`. `Eof` with no bytes is the peer
  being gone (nothing stored — pinned today by the vanish test). The
  buffer never holds more than `cap + 1` plus one `fill_buf` chunk;
  message-level memory stays O(cap) under and O(1) once over.
- **Wire contract (DATA):** unchanged shapes — `354`, terminator `.` line,
  `..`-unstuffing, `552 5.3.4 Message too big` with nothing stored, `250`
  only after the synchronous insert. Changed capability: non-UTF-8 bytes
  are accepted and preserved instead of killing the session; an
  unterminated over-cap line is refused instead of being buffered.
- **HTTP contract (`/raw`):** body = exact stored bytes;
  `Content-Type: message/rfc822`; `Content-Disposition: attachment;
  filename="<id>.eml"` (from the shared owner). UTF-8 mail bodies are
  byte-identical to before, so every existing `.contains()`-style
  assertion keeps passing.

## Quickstart (how the contracts are driven)

Real servers, real protocol, no mocks — the pattern is
`start_smtp(SmtpConfig { max_message_size: 64, ..Default::default() })`,
a step-by-step `SmtpConn`, and `http_get_bytes` for the byte-exact GET.
The binary probe body: `Subject: binary\r\n\r\n` + bytes containing
`0xFF 0xFE 0x00`, a `NUL`, a bare `CR`, a `..`-stuffed line and a literal
`.`-starting line, sent via `data_bytes`, then `GET
/api/v1/messages/{id}/raw` compared with `assert_eq!` on `Vec<u8>`.

## Risks

1. **`/raw` consumers pinned on `text/plain` or on lossy bytes.**
   *Mitigation:* README:232 and the compat OpenAPI entry already document
   `message/rfc822`; UTF-8 bodies are byte-identical so existing assertions
   pass; the header delta is recorded in decision 0022 for operator
   acceptance. Residual: a consumer keying on the exact header set must
   take the new `Content-Disposition` — one line in the decision.
2. **The `+1` slack reads as magic and gets "simplified" away.**
   *Mitigation:* two-line comment at the constant + the at-cap stuffed-line
   test, which goes red the moment the slack is dropped (an exactly-at-cap
   stuffed message would take a spurious `552`). Safe side, but loud.
3. **Drain regression desyncs the stream** (the next command is read from
   mid-line). *Mitigation:* the over-cap test asserts the same connection
   completes a following small mail with `250`; the existing
   `oversized_data_gets_552_and_is_not_stored` keeps passing.
4. **Binary bodies break parsing/extraction surfaces.** *Mitigation:*
   `build_email` already parses bytes; mail-parser handles hostile input
   by yielding fewer views; a parser panic unwinds only the session task —
   fail-closed, no `250` without storage (pre-existing posture). The chaos
   test gains a binary body to prove ordering over hostile bytes.
5. **Coverage: an arm of the new reader measures uncovered.** *Mitigation:*
   the branch→test map is explicit (Complete → round-trip/at-cap tests,
   OverCap+drain → over-cap and the existing 552 test, Eof → the existing
   vanish test, delegation → existing `/raw` tests); the exact
   100/100/100 gate is the arbiter and the fix is a test, never
   `coverage(off)`.
6. **STARTTLS-only regressions.** *Mitigation:* the reader sits on
   `BufReader<Conn>` (transport-agnostic) and the STARTTLS round-trip test
   pins the contract over TLS once.
7. **Scope creep on review** (command caps, knobs, JSON views riding along).
   *Mitigation:* the cuts are enumerated in the spec and decision 0022;
   anything outside the table above is a new decision, not an edit.
