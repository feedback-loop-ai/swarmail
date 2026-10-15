# Plan — protocol conformance #42/#43/#44/#45 (one SMTP state machine and a POP3 wire that matches itself)

Implementation plan for `specs/protocol-conformance-42-43-44-45/spec.md`.
All anchors re-verified against the working tree at `13c3064` this
session.

## Technical context

- Single Rust crate; production under `src/`, real-protocol suites under
  `tests/`. The batch lives in **two production files**:
  - `src/smtp.rs` — the session loop (`session`, `:534-944`): the
    envelope locals `:548-549`; the STARTTLS reset `:662-664`; the `AUTH`
    arm `:669-734` (PLAIN `:683-705`, LOGIN `:706-724`,
    `auth_plain_inbox` `:946-953`, `b64_decode` `:955-960`); the `MAIL`
    arm with its chaos-first check `:736-748`; the `RCPT` arm
    `:750-785`; the `DATA` arm `:786-915` (pre-check `:787-795`, the 354
    `:796-801`, the allowance `:812-816`, the terminator `:838`, the
    unstuffing `:842-846`, the 552 `:853-861`, chaos `:862-876`,
    build/park/insert `:878-914`); `RSET` `:916-920`.
  - `src/pop3.rs` — the session (`handle_conn`, `:254-545`): `USER`
    `:295-303`, `PASS` snapshot `:305-334`, `RETR` `:401-422`,
    `DELE` `:462-477`, `RSET` `:479-488`, the `QUIT` loop + comment
    `:489-518`; `MaildropMsg` `:171-175`; `resolve` `:554-568`,
    `counts` `:569-582`, `live_numbers` `:583-595`;
    `write_stuffed` `:631-636`, `write_stuffed_limited` `:637-655`.
- The suites that speak the protocol: `tests/smtp_edges.rs` (AUTH/chaos/
  transaction legs; `base64_of` helper; the three pinned silent-`235`
  tests around `:94-95`, `:544-585`), `tests/pop3.rs` (`octets()`
  helper `:18-20`, the pinned stored-size test `:200-224`, the
  partial-`QUIT` leg `:740-853`), `tests/common/mod.rs` (`start`,
  `start_with`, `http_json`, `smtp_send`, `SmtpConn`, `Pop3`,
  `refusing_trigger_except` `:925-935`), plus `tests/starttls.rs`,
  `tests/p2.rs`, `tests/burst.rs`, `tests/rate.rs` as regression nets.
  No mocks anywhere in the loop.
- Coverage is measured by `scripts/coverage-gate.sh` — literal 100/100/100,
  missed == 0, `coverage(off)` forbidden. The batch is **net-positive a
  few dozen lines**; every new branch ships with a wire leg or a
  branch-stable inline unit test (0020 rule 1), and no cold path needs
  the harness (rule 3 keeps the partial-`QUIT` and chaos legs in
  `tests/` — they drive real SQLite triggers and probability-100 rules,
  not crate-private seams).
- No new dependencies, no new types, no new `src/` files. `base64`,
  `std`, tokio and the existing test helpers suffice. Model/policy
  unchanged (`glm-flash` pinned).

## Constitution check

| Guarantee | Effect of this batch |
|---|---|
| 0001 accept == stored | **Untouched.** The envelope take at the terminator runs *before* `build_email`/`park_delivery`/`insert_async`; no `await` is added between the terminator and the commit; the `250` is written only after the insert resolves (`:897-905`). Every refusal (`503`, `552`, `451`) fires before any insert and stores nothing. |
| 0002 await subscribes before it checks | Untouched — no watcher or long-poll code changes. |
| 0003 oldest-only pruning | **Protected.** The maildrop lock is refused because honoring it would exempt locked inboxes from pruning; the snapshot stays a copy. `DELE` still flows through `Store::delete` on `QUIT` — never the pruning path. |
| 0004 per-test inboxes | **Amended by 0028 §2 (proposed).** The username still names the inbox; a credential that fails to name one refuses `501` instead of silently succeeding — the smtplib shape and the refusal posture restore the isolation the silent `235` broke. |
| 0005 extraction at ingest | Untouched — no ingest normalization, no new `Email` field, `Email.size` unchanged; `/raw` stays byte-honest. |

**Zero-diff demand (binding for review):** `src/store.rs`,
`src/persist.rs`, `src/model.rs`, `src/extract.rs`, `src/api.rs`,
`src/mcp.rs`, `src/chaos.rs`, `src/threads.rs`, `src/compat.rs`,
`src/webhook.rs`, `src/tls.rs`, `src/config.rs`, `src/feed.rs`,
`src/stdio.rs`, `src/ui.rs`, `src/lib.rs`, `src/main.rs` — zero diff.
`tests/common/mod.rs` — expected zero diff; a helper lands there only if
a leg genuinely needs one. If any other file changes, the implementation
has left the spec and goes back.

## Project structure — files touched

| File | Change |
|---|---|
| `src/smtp.rs` | 1) Terminator: `:838` loses `|| bytes == b".\n"`; reword the allowance comment's `.` CRLF or `.` LF clause (`:806-811`). 2) Envelope: after the data loop breaks, `mail_from.take()` + `std::mem::take(&mut rcpts)` feed `build_email` (`:878-879` loses its `.clone().unwrap_or_default()`); every outcome path inherits the consumed transaction. 3) `MAIL` chaos arm (`:736-745`): clear `mail_from`/`rcpts` before answering — a refused attempt ends the transaction. 4) `MAIL` after the chaos check: if `mail_from.is_some()`, answer `503 5.5.1 Error: nested MAIL command` (state unchanged). 5) `DATA` pre-checks (`:787-795`): `mail_from.is_none()` → `503 5.5.1 Error: need MAIL command`, else `rcpts.is_empty()` → `need RCPT` (transaction left open). 6) `AUTH` arm: while a transaction is open answer `503 5.5.1` (after the `accept_any_auth` 502 gate, so a disabled-AUTH session keeps its 502). 7) PLAIN/LOGIN: `*` at any prompt or in the initial response → `501 5.5.2`; decode strictly (`b64_decode` → `String::from_utf8`, no lossy, no silent `Some`); assign `inbox` only on success; LOGIN takes the initial response as the username and prompts only `334 UGFzc3dvcmQ6`; the password is inspected only for a bare `*`. One small helper (decode-or-refuse) is welcome if it keeps the arms flat — but zero new types. |
| `src/pop3.rs` | 1) `MaildropMsg` gains `gone: bool` (`:171-175`); `PASS` snapshot builds it `false` and derives `size: wire_len(&e.raw)`. 2) New `fn wire_len(raw: &[u8]) -> u64` beside the writer (`:631`): stuff into a (reused) scratch buffer, return its length — the count *is* the writer's output. 3) `RETR` (`:410-416`): stuff into a `body` buffer, then `+OK {body.len()} octets` — the reply counts its own bytes. 4) `QUIT` loop (`:489-518`): each successful delete moves the number out of `deleted` and sets its slot's `gone`; first failure breaks with the suffix unmarked; the comment is rewritten to 0015 §1's words. 5) `resolve`/`counts`/`live_numbers` (`:554-595`): `gone` slots answer/skip as "no such message"; `deleted` keeps only unapplied marks. 6) `RSET` (`:479-488`): unchanged code — `deleted.clear()` — now honest because committed deletes are not in the set. 7) `USER` (`:295-303`): both `-ERR` branches clear the pending name. 8) Module doc + `MaildropMsg`/`wire_len` docs state the 0028 postures. |
| `README.md` | One sentence: `:314-315`'s "octet counts are the stored raw sizes (RETR normalizes bare-LF to CRLF in transit…)" becomes "octet counts are the sizes RETR transmits — CRLF-terminated, dot-stuffed, excluding the final `.`" (decision 0028 §1); the POP3 paragraph gains the snapshot-not-lock sentence and the AUTH refusal sentence. |
| `tests/smtp_edges.rs` | Rewrite the three pinned silent-`235` tests to pin `501 5.5.2`; add the legs AC1-AC5 and AC7-AC9 name (envelope-consumption chains, the smuggling leg, nested MAIL/AUTH, the chaos-refused-MAIL leg, smtplib-shaped LOGIN, cancel/empty/undecodable/non-UTF-8 for both mechanisms). |
| `tests/pop3.rs` | Rewire `octets()` (`:18-20`) to derive from the wire transformation; rewrite the pinned stored-size test (`:200-224`) and every size expectation; add the advertised==transmitted pin, the bare-LF/mixed size legs, the USER-clear leg, the gone/no-such-message assertions in the partial-`QUIT` leg, and the HTTP-clear skew leg. |
| `tests/common/mod.rs` | Expected zero diff. |
| `docs/decisions/0028-…` + index row | Already committed with this spec (proposed). |

## Contracts — the wire shapes this batch pins

- SMTP refusals: `503 5.5.1 Error: need MAIL command` (RCPT/DATA without
  MAIL; DATA after a consumed transaction), `503 5.5.1 Error: need RCPT
  command` (DATA with MAIL, no RCPT — transaction stays open),
  `503 5.5.1 Error: nested MAIL command` (second MAIL mid-transaction —
  transaction stays open), `503 5.5.1` for AUTH mid-transaction,
  `501 5.5.2` for every credential that fails to name an inbox (both
  mechanisms, any prompt or initial response), `504 5.5.4` unchanged for
  unknown mechanisms, `502 5.5.2 AUTH disabled` unchanged.
- SMTP successes: `235 2.7.0 Accepted` only after credentials named an
  inbox (that inbox is now the session's); the LOGIN flow is
  `334 UGFzc3dvcmQ6` then `235`; `250 2.1.0 OK` after `MAIL`;
  `250 2.0.0 OK: stored as {id}` unchanged.
- POP3 sizes: `+OK n octets` (RETR), `+OK k (m octets)` (LIST arg),
  `+OK k messages (m octets)` (STAT, PASS, RSET summaries) — all with
  `m = Σ wire_len` and `k` counting only servable mail (not `gone`, not
  marked).
- POP3 errors: `-ERR no such message` (gone, store-missing, or
  out-of-range), `-ERR message already deleted` (unapplied mark),
  `-ERR USER required first` (PASS after a failed/absent USER),
  `-ERR storage commit failed` (0015, unchanged text).

## Quickstart

```bash
cargo test --test smtp_edges            # AC1-AC5, AC7-AC10
cargo test --test pop3                  # AC11-AC16
cargo test --test starttls --test p2 --test burst --test rate   # AC6
cargo test --lib pop3 --lib smtp        # inline unit tests (gone slots, decode)
bash scripts/coverage-gate.sh           # exact 100/100/100
```

By hand: deliver mail with a bare-LF body over SMTP; `STAT` over POP3
must equal the bytes `RETR` sends (`nc` and count). Authenticate with
`AUTH LOGIN` the way smtplib does — the mail lands in the username's
inbox. Send `foo\n.\nbar\r\n.\r\n` as data — exactly one mail, no split.
`AUTH LOGIN` with garbage → `501 5.5.2`, then deliver → `default` inbox.

## Risks

| Risk | Mitigation |
|---|---|
| **The take-at-terminator site is load-bearing.** If an implementer moves the take *after* an outcome branch (or leaves `mail_from.clone()` beside the take), a failure arm leaks again — the exact defect. | The spec's invariant, not the site, is what review checks: *no outcome path and no abort path runs cleanup; the envelope is consumed before the risky region.* Task 2's legs prove all four outcomes; the coverage gate refuses an unmeasured arm. |
| **CRLF-only terminator breaks a client that terminates with bare LF.** Such a peer now waits until the idle deadline. | That is the SMTP-smuggling class the issue closes; the wait is bounded by 0027's fence (`421 4.4.2`). Named residual in the spec; no compatibility knob (0022 §5: no knob). |
| **AUTH refusal flips a behaviour real clients see.** smtplib's `login()` sends the username inline (now honored); a client that relies on `235`-for-garbage had no working isolation anyway — its mail was pooling. | The smtplib-shaped leg is the issue's own scenario and must pass; the refusal legs assert the inbox is *unchanged*, not just refused. Decision 0028 §2 records the posture for the operator. |
| **Octet-count change invalidates every pinned size in `tests/pop3.rs`.** | Expected: the pins keep their values for CRLF-only fixtures (`raw_of` emits CRLF) and grow only where the fixture has bare LFs or dot-leading lines. The helper rewire (task 7) makes the *derivation* the contract, so future fixtures cannot re-pin the wrong number. |
| **Coverage gate refuses a new arm** (e.g. the nested-MAIL 503 or a cancel branch measured uncovered). | The refusal is the gate working: the fix is a test, never `coverage(off)`. Task 10 runs the gate; each new branch's leg is named in tasks 1-8. |
| **The `gone` marker races the retry-`QUIT` idempotency.** | `Store::delete`'s honest `Ok(false)` on an absent id (`src/store.rs:565-591`) is untouched; the retry leg re-lands nothing and signs off. The partial-`QUIT` leg (0015's instrument) proves the convergence. |
| **Drift between the decision, README and the code.** | The decision is committed *with* the spec; README's one sentence and the module docs are task 8; review checks the three against each other. |

## Gates

`cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets
--locked -- -D warnings`; `cargo test --workspace --locked`;
`brokkr compile --bundle bundles/self`; `scripts/coverage-gate.sh`
(exact 100/100/100); `cargo deny check licenses`. Verify additionally
runs the coverage selftest, the publish-guard proof and the fail-closed
native-tls probe, network denied, under the 3-hour budget.
