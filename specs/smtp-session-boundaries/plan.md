# Implementation plan: SMTP session boundaries

## Technical context

One Rust crate, Tokio TCP/TLS listeners; existing `src/smtp.rs` owns commands,
AUTH and DATA, with bounded byte reads via `read_line_capped`/`drain_line`.
Reuse existing standard-library parsing, base64 and transport dependencies.
No new dependency, persistent field, configuration, endpoint or test seam.
This plan implements [the spec](spec.md) and proposed
[0034](../../docs/decisions/0034-data-terminator-boundary.md).

No plan template exists in `.specify/templates/` on this baseline. Its sole
spec template describes an unrelated multi-repository workspace; use the
requested technical context, constitution check, project structure, research,
data model, contracts and quickstart here for this single-crate feature.

## Constitution check

Preserve frozen guarantees 0001–0003 and ingest extraction 0005. Keep the
awaited off-worker parse and insert path, commit failure and shutdown refusals.
The local DATA recognition changes only when that path is entered. Reader
allocation remains bounded under 0022; no normalization at ingest or serve.
0028 refusal semantics are strengthened and its repeat-AUTH behavior narrowed
through proposed 0034; operator acceptance remains separate.

0020 governs test placement: deterministic reader/parser cases stay measured
inline; wire behavior belongs in existing integration suites. No harness move
or exclusion to rescue coverage. 0035 and the coverage gate remain unchanged.
Preserve the extraction character window, eager interrupt, fallible search,
loopback guards, staged compat kind, page caps, bounded awaits registry with
its `_uncounted` row, and 30-second helper deadline/derived outer bound.

## Project structure and files touched

| Path | Planned work |
|---|---|
| `src/smtp.rs` | Private envelope type, optional authenticated inbox, strict path/parameter helpers, reader-ending metadata and local command ordering; deterministic inline tests. |
| `tests/smtp_edges.rs` | New red/green regressions and acceptance matrices; replace permissive bracket tests with refusal/state tests. |
| `tests/starttls.rs` | Prove identity and transaction reset and fresh authentication over real TLS. |
| `tests/common/mod.rs` | Audit DATA helpers; change only if necessary for a valid CRLF terminator. Existing helpers append CRLF before dot already. |
| `README.md` | Document greeting leniencies, strict paths/parameters, complete DATA boundary, one AUTH per TLS epoch and accept-any password/authzid behavior. |
| `docs/decisions/README.md` | Implementation adds the proposed 0034 index row without changing other decisions. |
| `docs/decisions/0034-data-terminator-boundary.md` | Proposed semantic ruling, delivered by design. |
| `specs/smtp-session-boundaries/{spec,plan,tasks}.md` | Design and later clarification/task evidence; this design commit contains exactly these three and 0034. |

## Research findings and choices

- Current DATA termination tests only the current `.\r\n` line. Both
  over-cap reader exits discard the predecessor ending; repair both, not just
  the session comparison. `Bytewise` already gives deterministic one-byte
  fragmentation. TCP write boundaries cannot prove buffer segmentation.
- MAIL and RCPT currently invoke chaos before sequence checks. MAIL's chaos
  arm explicitly clears a live envelope. Move validation first and remove
  that cleanup. Preserve DATA's independent failure/consume rules.
- `arg_address` has permissive bracket/key-value fallbacks explicitly pinned
  by two tests. Replace these expectations intentionally; do not retain dead
  acceptance branches for coverage. Null reverse-path is still valid.
- EHLO already advertises SIZE/8BITMIME/SMTPUTF8; rejecting BODY/SMTPUTF8 would
  break conforming smtplib callers. No extension registry is needed.
- PLAIN currently takes the second or first split field. Require three fields
  before selecting authcid. The existing PLAIN/LOGIN success test already
  uses separate connections, so it needs no artificial split.
- Existing tests exercise the smtplib LOGIN shape but do not invoke Python.
  Add real stdlib smtplib verification as a run-local integration probe against
  the built server; it must fail loudly if unavailable, not silently skip.

## Data model and algorithm

Keep all state private to one `session` invocation:

| Value | Meaning / mutation |
|---|---|
| `Option<Envelope { from: String, rcpts: Vec<String> }>` | None means no MAIL; Some with empty sender is legal null MAIL; accepted MAIL creates, RCPT appends; reset/terminator consumes atomically. |
| `Option<String>` authenticated inbox | None means unauthenticated default; Some means successful AUTH, including literal default. Borrow `as_deref().unwrap_or("default")` where routing is needed; clone only for owned offload capture. |
| `prev_crlf: bool` | DATA-local, initially true for the immediate-empty case; update from every nonterminator wire line, including discarded lines. |
| `Line::TooLong { crlf: bool }` | Ending of the drained line; command and AUTH callers ignore the flag. |

Retain one reader. On its buffered over-cap exit inspect the accumulated
line ending before discarding it. On its drain exit seed the drain with the
last consumed byte and carry that byte across chunks; report whether the LF
was preceded by CR. At EOF report false and let EOF end the session without
storage. This adds constant metadata, not a second body buffer.

Test termination against the previous flag before updating it from the
current line. Update even when size refusal is already sticky. Compute before
unstuffing; do not derive it from `raw`. Retain the cap/slack/minimum allowance
and existing drain deadline. Take the envelope once at the real terminator,
before over-limit, DATA-chaos or insertion outcomes. Avoid fallback/default
envelopes that could conceal an invalid transition; structure access around
the established DATA state check.

Replace `arg_address` with a small helper returning path and parameter suffix
or a syntax refusal. Match prefixes without case sensitivity; require ordered
angle delimiters and whitespace before any suffix. Keep path content opaque
beyond framing/nonempty-RCPT rules; preserve UTF-8/local paths. A small MAIL
parameter helper tracks seen SIZE/BODY/SMTPUTF8, validates left to right, and
maps to the spec's 501/552/555 replies. Validate ASCII digits before checked
integer conversion so digit overflow is a 552. Do not trust declared size
for buffering or actual DATA enforcement.

Use the spec's MAIL/RCPT validation order. HELO/EHLO and RSET assign no
envelope; STARTTLS additionally clears authenticated identity after upgrade.
AUTH disabled/state checks precede syntax. Validate argument token count,
then existing mechanism/continuation decoding; assign Some(inbox) only on
success. The same private identity value carries the repeat-AUTH guard.

## Contracts and test design

The normative reply/transition tables and AC1–AC13 live in spec.md. Tests must
check stored sender/recipient/raw/inbox values alongside replies. For absence
of injected commands, pipeline a real terminator and QUIT: expect exactly the
DATA outcome then 221, rather than sleeping to guess that no reply arrived.

Reader tests exercise whole-buffer, capped and drain paths, with controlled
chunks and Bytewise, including CR immediately before entering drain, LF as
the next byte, and EOF. All production branches remain measured. Real TCP
tests use fresh servers/per-test inboxes and existing timeout helpers.
Oversize tests use small in-process caps; no giant allocation benchmark.
Change chaos from zero to 100 only after the initial envelope exists, using
existing acknowledged configuration mechanisms; no race or sleep-as-sync.

Collect red evidence in an isolated worktree at `bacadbe`, carrying only new
public-API regression tests into it. Choose independent pins for mixed DATA,
EHLO reset and one-NUL PLAIN; assert the specific baseline failure, not an
unrelated compile/environment failure. Run those same pins green after the
repair and retain command/output evidence under ignored `.brokkr/state/`.

## Quickstart and verification

On a writable feature branch, implement tasks in order, then run:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --locked --test smtp_edges
cargo test --workspace --locked --test starttls
cargo test --workspace --locked --lib smtp
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
cargo deny check licenses
```

No TMPDIR override; coverage must be exact 100/100/100. The Python probe uses
the built real server on temporary loopback ports, stdlib smtplib and urllib:
EHLO, login, send ASCII mail and UTF-8-address mail with BODY/SMTPUTF8 options,
then inspect inbox/raw and envelope via HTTP. Capture script, command and
result run-locally so another seat can reproduce it. No new package install.

This design seat commits exactly the four requested design files and never
pushes. Implementation/verification/review own code and delivery gates;
later shipping follows the framed PR/check workflow and never closes issues.
A design-stage gate pass on unchanged production is not implementation proof.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Split CR/LF or a drained line silently loses history. | Both reader exits report endings; test entry-to-drain and multi-chunk splits plus EOF. Never guess. |
| Malformed LF-only senders no longer finish. | Document the stricter boundary; existing idle deadline closes with no store. Preserve byte fidelity rather than normalize. |
| Structural state edits miss a reset or route metrics to the wrong inbox. | One optional envelope/identity; wire tests for each reset, default AUTH, failure and TLS upgrade; preserve awaited delivery path. |
| Strict parsing breaks old probes or ordinary clients. | Intentionally replace malformed acceptance pins; retain local/UTF-8/null-MAIL forms and advertised parameter support; real smtplib probe. |
| SIZE parsing differs across integer widths or hides overflow. | ASCII digit validation plus checked parse; cap-relative and overflow tests; DATA still enforces actual bytes. |
| New 503 precedence masks chaos tests. | Separate invalid-command assertions from valid injected failures; probability 0/100 only. |
| Test rewrites become vacuous or evade exact coverage. | Red baseline per child, exact reply and stored-state assertions, inline deterministic tests; unchanged 0020/0035 gate. |
| Signing or read-only Git prevents the design commit. | Preserve reviewable files and record the precise blockage; never claim a commit or bypass filesystem restrictions. A writable authorized continuation must stage only the four paths. |
| Operator returns proposed 0034. | Keep it proposed; encode requested changes in owning spec/plan/tasks, preserving dated clarifications. Do not silently edit accepted 0022/0028 or 0035. |
