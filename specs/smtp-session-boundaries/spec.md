# SMTP session boundaries

Date: 2026-10-18. Status: design draft for run 13. Issues: #73, #81, #82
(epic #70). Reviewed implementation baseline: `bacadbe`.

## Feature and rationale

Repair the existing SMTP session loop so body bytes cannot become commands at
an incomplete DATA terminator, refused envelope commands cannot alter a live
transaction, and malformed credentials cannot select an inbox named after a
password. These are one session-ownership problem, delivered together.

Inputs read: both run-local positions at
`.brokkr/state/design/positions/{simplicity,robustness}.md`, the feature at
`.brokkr/state/run13-feature.txt`, `.brokkr/state/run13-intake-framing.md`, and
the three verbatim issue bodies under `.brokkr/state/epic70/`. The framing
files currently under `.brokkr/state/tasks/` concern run 12 and are historical,
not authority for this feature. This document contains the resulting contract
without requiring those ignored files to survive.

This changes public protocol behavior and amends earlier rulings. Proposed
decision [0034](../../docs/decisions/0034-data-terminator-boundary.md) records
the complete DATA boundary, declared SIZE refusal, and AUTH/session changes.
Only the operator accepts it. Number 0034 is vacant and explicitly assigned
by this intake; existing 0035 stays byte-identical despite its historical
numbering note about result sealing.

## Synthesis and rulings

| Dispute or agreement | Ruling and cost |
|---|---|
| Both positions: repair the existing loop and retain bounded byte reading. | Adopt. No framework, general command AST, second scanner, new dependency, feature flag or operator knob. Local helpers must have measured tests. |
| Both: retain actual preceding CRLF, including discarded oversized lines. | Adopt without approximation. Guessing true permits false termination; guessing false loses the legitimate oversize client's 552. Cost: one bit of reader metadata and deterministic split-boundary tests. |
| Simplicity keeps separate sender/recipients; robustness wants `Option<Envelope>`. | Adopt robustness's private envelope type. It makes an absent sender with retained recipients unrepresentable and centralizes reset/consume. Reject a general session-state framework. Cost over simplicity: update each envelope use and its tests. |
| Simplicity requires brackets; robustness preserves bracketless paths. | Adopt strict brackets; delete the permissive fallback. Existing bracketless/inverted-path acceptance pins become refusal pins. Cost: those nonconforming clients must change; no full mailbox grammar or DNS validation is added. |
| Overflowing all-digit SIZE: simplicity says 552, robustness says 501. | Adopt simplicity: a syntactically valid integer exceeding representable size exceeds the configured cap. Cost: distinguish invalid digits from arithmetic overflow with tests. |
| Simplicity adds an AUTH bit alongside a String; robustness uses `Option<String>`. | Adopt robustness: `None` routes to default, `Some(name)` records success, including `Some("default")`. Cost: adapt delivery/metrics access; avoid duplicate identity/success state. |
| Robustness applies state/syntax/parameters before chaos; simplicity emphasizes MAIL. | Apply that order to both MAIL and RCPT, including recipient capacity before RCPT chaos. Cost: malformed or out-of-state RCPT no longer triggers an injection rule. Do not broaden this into unrelated command rewrites. |
| Both permit MAIL before greeting; robustness explicitly preserves bare greetings. | Adopt and document both. No greeting flag or hostname validation. Cost: this sink remains deliberately lenient on greeting sequence/arguments. |
| Robustness refuses extra AUTH tokens. | Adopt the small syntax check: at most mechanism plus one initial response. Cost: clients relying on ignored tokens now receive 501. |

## Required behavior

### DATA ownership

After 354, a whole wire line equal to `.\r\n` ends DATA only if the previous
wire line ended in CRLF, or if this is the immediate empty-message terminator.
All read paths, including capped-line draining, supply the true ending.
Boundary state comes from wire bytes before dot unstuffing, never from the
possibly capped stored buffer. EOF without a newline does not invent CRLF.

Other bytes retain decision 0022's policy: bare LF, NUL and non-UTF-8 are
verbatim; only a leading `..` loses one dot. A `.\r\n` line preceded by bare
LF is payload, including its dot. The CRLF ending the preceding payload
line remains stored. A `.\n` line is always payload.

Size refusal remains sticky. Keep the existing remaining-cap allowance,
one-byte dot-stuffing slack and minimum three-byte terminator allowance.
Drain through the real terminator, answer 552 once, consume the envelope,
store nothing and accept a subsequent transaction. EOF/idle expiry before a
real terminator closes without storage or a mid-DATA reply. A bare-LF-only
sender ending with `\n.\r\n` must send a later valid boundary or will time out.

### Envelope commands

Successful HELO and EHLO reset the envelope exactly as RSET and preserve
AUTH. HELO returns one `250` line; EHLO retains its advertised capabilities
and conditional STARTTLS/AUTH. Bare greetings remain accepted. MAIL before
any greeting remains accepted and must be stated in README.

MAIL requires a case-insensitive `FROM:` prefix and a bracketed reverse-path;
RCPT requires `TO:` and a bracketed nonempty forward-path. Optional whitespace
after the colon is permitted. Missing/wrong prefixes, missing/reversed or
extra angle delimiters, bracketless paths, and a suffix attached directly to
the closing bracket receive 501. `MAIL FROM:<>` is valid; `RCPT TO:<>` is not.
Preserve local names such as `<alice>` and UTF-8 paths; do not introduce an
`@` requirement, a DNS lookup, or full RFC mailbox validation.

Whitespace-separated MAIL parameters have case-insensitive names/keywords:

| Parameter | Accepted form | Refusal |
|---|---|---|
| SIZE | `SIZE=` followed by one or more ASCII decimal digits, value <= cap | 501 for absent/nondigit/signed value; 552 for value > cap, including all-digit overflow |
| BODY | `BODY=7BIT` or `BODY=8BITMIME` | 501 for other/missing value |
| SMTPUTF8 | bare `SMTPUTF8` | 501 for a value |
| Any other name | none | 555 |

Duplicate recognized parameters receive 501. Inspect parameters left to right;
the first error wins. All RCPT suffix parameters receive 555. Accepted SIZE
does not override actual DATA byte accounting; a false declaration cannot
bypass the cap. These supported parameters match existing EHLO advertisements
and remain usable under the documented pre-greeting leniency.

MAIL order: open transaction -> 503; path syntax -> 501; parameter validation;
chaos; create envelope only on acceptance. RCPT order: absent envelope -> 503;
path syntax -> 501; parameters -> 555; recipient cap -> existing 452; chaos;
append only on acceptance. State refusals win even over malformed syntax.
Every refused MAIL/RCPT leaves sender and all existing recipients unchanged.
At a recognized DATA terminator the envelope is consumed before any final
outcome (250/552/451); RSET and successful STARTTLS also clear it.

### AUTH ownership

PLAIN must decode to exactly `authzid NUL authcid NUL password` (three fields,
two NULs). Only the strictly decoded UTF-8, trimmed, nonempty authcid names
the inbox. Keep the existing base64 decoder and accept-any password policy;
empty passwords and empty/nonempty authzid are allowed, and authzid does not
override authcid. Malformed input answers `501 5.5.2`, changes no identity,
creates no inbox and permits a later successful attempt.

With AUTH enabled, a live envelope or earlier successful AUTH causes 503
before decoding or issuing any continuation. Successful AUTH as `default`
counts. Otherwise missing mechanism or extra tokens returns 501; an unknown
nonempty mechanism returns 504. AUTH-disabled configuration retains 502.
PLAIN initial/continuation and LOGIN initial/prompt forms remain supported;
LOGIN still consumes the password and preserves its cancellation behavior.
RSET/HELO/EHLO preserve AUTH success. Successful STARTTLS resets identity,
AUTH success and envelope along with the existing buffered plaintext reset.

## Acceptance Criteria

- **AC1 (#73):** Over real TCP, `foo\n.\r\nbar\r\n.\r\n` stores exactly
  `foo\n.\r\nbar\r\n`; `foo\n.\nbar\r\n.\r\n` stores exactly
  `foo\n.\nbar\r\n`. A command-shaped body line such as NOOP remains in
  `/raw`; an appended QUIT gets 221 directly after the single final DATA
  reply, exposing any extra command reply without timing-based silence checks.
- **AC2 (#73):** Proper CRLF termination and immediate empty DATA succeed.
  Dot transparency, binary/NUL/raw fidelity, actual cap and exact-at-cap
  dot-stuffed bodies retain their existing results and queryability after 250.
- **AC3 (#73):** Deterministic reader tests cover full-buffer and drain
  over-limit exits with CRLF and LF, CR/LF split across chunks (including
  entry to drain), and EOF in drain. Wire tests after oversized LF and CRLF
  lines prove correct 552 alignment, zero stores, and a successful following
  small mail. Fragmented normal reads preserve the boundary too.
- **AC4 (#73/#81):** EOF and idle expiry mid-DATA store nothing; incomplete
  terminators never expose command text. Existing DATA chaos, storage-failure,
  transaction consumption and pipeline regressions remain green.
- **AC5 (#81):** MAIL/RCPT/EHLO/MAIL and the HELO equivalent accept the new
  MAIL and store only its sender/recipients; DATA immediately after reset
  returns need-MAIL 503. HELO has one reply line. AUTH survives both greetings
  and RSET. README explicitly records pre-greeting MAIL and bare greetings.
- **AC6 (#81):** Table-driven wire tests cover every refused path form above,
  including bare MAIL, MAIL TO, bare RCPT, RCPT FROM, RCPT bare@x, empty RCPT
  and bracketless/inverted paths, with exact reply codes and unchanged
  envelope contents. Null MAIL, local names and UTF-8 paths still succeed.
- **AC7 (#81):** SIZE below/at/above cap, zero, leading zeroes, empty,
  nondigit, signed and overflowing values have the specified outcomes;
  BODY/SMTPUTF8 valid forms, invalid values, duplicates, unknown MAIL and
  RCPT parameters are covered. Refusals do not create or alter an envelope.
  Lying with an accepted small SIZE still yields 552 for oversized DATA.
- **AC8 (#81):** With deterministic probability-100 MAIL chaos enabled after
  an envelope opens, nested MAIL returns 503 and the old envelope can finish
  intact. With RCPT chaos, absent MAIL and malformed RCPT still get 503/501,
  and cap refusal remains 452. Valid commands still trigger configured chaos.
- **AC9 (#82):** PLAIN with zero, one or more than two NULs returns 501;
  snapshot inboxes before/after refusal and then deliver to prove unchanged
  default routing, no password-named inbox, and no creation by AUTH alone.
  A failed attempt can be followed by successful PLAIN or LOGIN.
- **AC10 (#82):** PLAIN with empty/nonempty authzid and LOGIN initial/prompt
  forms route to authcid. Second AUTH returns 503 without prompts or routing
  change, including first success as default, after RSET and after greetings.
  Missing mechanism/extra tokens give 501; unknown mechanism gives 504;
  disabled AUTH retains 502; transaction-in-progress AUTH retains 503.
- **AC11 (#81/#82):** A real STARTTLS session proves reset of AUTH/envelope,
  fresh AUTH success and the existing plaintext-buffer discard. Existing
  pipelining tests and a real Python stdlib smtplib send pass, including
  SIZE and explicit BODY=8BITMIME/SMTPUTF8 options with UTF-8 addresses.
- **AC12 (batch):** At least one new real-server wire regression for each
  child is demonstrated failing on `bacadbe` and passing on the repaired
  tree; record names, commands, assertions and exit statuses. Old and new
  green alone is insufficient. Tests follow 0020; no mock SMTP server,
  sleep synchronization, probabilistic chaos or new shared mutable state.
- **AC13 (batch):** Six gates in plan.md pass on the implementation tree;
  coverage is exact 100/100/100 with no exclusions or TMPDIR override. No
  dependency or license allowlist change. Decision 0034 remains proposed;
  0035 and all listed do-not-undo repairs remain intact. No issue is closed.

## Scope and guarantees

The design seat delivers only this spec, plan, tasks and proposed 0034; it
does not claim implemented criteria or a release. Subsequent implementation
updates README and the decisions index as planned. No POP3, storage schema,
HTTP API, extraction, pruning, await registration or coverage-gate redesign.
Store::insert stays synchronous; existing awaited parse/commit-before-250
ordering stays intact (0001, 0023, 0030). Subscribe-before-check (0002),
oldest-only pruning (0003) and ingest-time extraction (0005) are preserved.
