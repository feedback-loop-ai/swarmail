# Spec — protocol conformance #42/#43/#44/#45: one SMTP state machine and a POP3 wire that matches itself

Run 8: issues [feedback-loop-ai/swarmail#42](https://github.com/feedback-loop-ai/swarmail/issues/42),
[#43](https://github.com/feedback-loop-ai/swarmail/issues/43),
[#44](https://github.com/feedback-loop-ai/swarmail/issues/44),
[#45](https://github.com/feedback-loop-ai/swarmail/issues/45). Base:
`13c3064` (run 7 landed). Branch and PR are the operator's.

Decision: 0028 —
[docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md](../../docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md)
(status `proposed` — the operator accepts; the batch ships together with
it). It carries the two contract changes the issues name explicitly — the
POP3 octet rule (#44) and the AUTH refusal posture amending 0004 (#43) —
plus the third posture the panel found decision-worthy, the
snapshot-not-lock maildrop (#45). #42 and #45's remaining edges are
conformance repairs *toward* RFC 5321 §2.3.8/§4.1.4, RFC 4954 §4 and RFC
1939: the code is what lies, so the commit cites the sections and no
further ADR is minted.

## The problem — four defects, two files, one lie each

All anchors re-verified on the working tree at `13c3064` this session; the
issues pin older lines, so the shapes are cited from HEAD.

1. **#42 — the SMTP transaction outlives its own completion.** The
   envelope is two locals (`mail_from: Option<String>`, `rcpts:
   Vec<String>`, `src/smtp.rs:548-549`) cleared in exactly three places —
   `MAIL` (`:746-747`), `RSET` (`:917-918`), STARTTLS restart (`:662-664`)
   — and **never after DATA**: the DATA arm ends in four reply paths (552
   `:853-861`, chaos `:862-876`, commit-refused `451` `:906-913`, stored
   `250` `:898-905`) and not one touches the envelope. Every symptom in
   the issue is downstream: recipients bleed into the next mail; a second
   `DATA` stores under the old envelope; a chaos-rejected `MAIL` leaves
   the previous envelope live; and the pipelining client whose `MAIL` was
   refused but whose `DATA` got `354` sends a lone `.` and stores an empty
   mail. Separately, the terminator is the disjunction
   `bytes == b".\r\n" || bytes == b".\n"` (`:838`) — a bare-LF
   dot-terminates the message and hands the rest to the command parser:
   the SMTP-smuggling class, forbidden by RFC 5321 §2.3.8.
2. **#43 — `AUTH LOGIN` ignores its initial response and never refuses.**
   The `LOGIN` branch (`src/smtp.rs:706-724`) always prompts
   `334 VXNlcm5hbWU6` even when the AUTH line carried the username, so
   Python's `smtplib` (which sends it inline) answers the *Username*
   prompt with the password and the inbox becomes the password —
   decision 0004's per-test isolation silently breaks. There is no
   empty-username check (PLAIN has one, `:946-953`),
   `String::from_utf8_lossy` (`:714`) mints an inbox no client sent, and
   the password line is read and discarded (`:718`) before an
   unconditional
   `235` — so undecodable base64 or a `*` cancel *succeeds*, keeping the
   previous or `default` inbox and pooling parallel tests' mail. The
   pinned tests (`tests/smtp_edges.rs` :94-95 area and
   `auth_login_with_undecodable_username_stays_default`,
   `auth_plain_with_empty_authcid_stays_default`) pin the silent `235`.
3. **#44 — POP3 advertises one size and sends another.** STAT, LIST, the
   PASS/RSET summaries and RETR's own `+OK n octets` all answer
   `msg.size` — the stored raw length (`src/pop3.rs:171-175`, snapshotted
   at `:315-323`) — while RETR rewrites the wire through
   `write_stuffed_limited`/`write_stuffed` (`:631-655`): every bare LF
   becomes CRLF and every dot-leading line is stuffed. For the corpus the
   suite already holds, advertised 26 / transmitted 28. README
   (`:314-315`) documents the current behaviour and
   `tests/pop3.rs:216-219` pins it — a contract change, hence the
   decision.
4. **#45 — POP3's session view disagrees with the store and with itself.**
   (a) A `USER` that fails (`-ERR missing username` or
   `-ERR no such mailbox`, `:295-303`) never clears the pending name, so
   `PASS` logs into the *previous* mailbox. (b) The maildrop is a PASS
   snapshot with no lock: mail deleted or pruned elsewhere keeps its
   number until `RETR`/`TOP` answer `-ERR no such message` (`:401-422`) —
   the listing and the fetch disagree. (c) On a partial `QUIT` failure the
   loop (`:489-518`) breaks after the first refused delete but leaves the
   *committed prefix's numbers in `deleted`* — so `RSET` (`:479-488`)
   clears the set and resurrects a delete that has committed, which
   decision 0015 §1 (accepted) forbids in so many words: "nothing reports
   deleted and resurrects; nothing lies." (d) The `QUIT` comment
   (`:489-504`) says the failing suffix is left "unmarked" while the code
   keeps every mark — the prefix is marked but *gone from the store*, so
   `RETR` answers `-ERR message already deleted` for mail that no longer
   exists. The store-side question (how an emptied inbox changes USER's
   answer) is tracked separately and is out of scope here.

## Ruling — how the two positions were reconciled

The panel's positions (`.forge/design/positions/{simplicity,robustness}.md`,
both `pass`) agree on the bones: both refuse a session-state enum rewrite,
an ingest-time size field, a maildrop lock, and any second store API; both
demand exactly one decision document. They conflict on five points; this
spec rules rather than averages.

### #42 — the envelope is consumed once, before any outcome

**Adopted from both: the transaction is a transition, never a cleanup.**
The envelope dies at the moment the session commits to an outcome — there
is no per-outcome cleanup to forget, and the risky region (build → park →
`insert_async`, `:878-897`) runs against a *consumed* transaction, so the
abort path (the parked insert an `abort` cannot cancel) and every failure
arm inherit nothing.

**Ruled for simplicity on the site: the take happens at the terminator.**
Robustness wants the clear when the `354` is written; simplicity wants it
where the terminator is recognized, immediately before `build_email`. They
are observationally equivalent — the take runs *before* the outcome
branches either way, and every mid-DATA exit (`Line::Eof` `:834`,
`TimedOut` `:835`, `Err` `:836`) `return`s, so no surviving session ever
holds a stale envelope — and the terminator site is smaller: the taken
values feed `build_email` directly, deleting today's
`mail_from.clone().unwrap_or_default()` (`:879`). **Cost of refusing the
354-site:** none observable; if an implementer ever finds a path where the
terminator is not reached but the session *survives*, that path must take
the envelope too — the spec's invariant ("no outcome path, and no abort
path, runs cleanup") is what review checks, not the site.

**Ruled for robustness on the refused `MAIL`: a chaos-refused `MAIL`
consumes the envelope.** The MAIL arm's chaos branch (`:736-745`) clears
`mail_from`/`rcpts` before answering. A `MAIL` *attempt* — accepted or
refused — ends the current transaction (RFC 5321 §4.1.4: a new `MAIL`
begins one); simplicity's "closes for free after any terminated DATA"
leaves the ordering MAIL → RCPT → MAIL(*refused*) → DATA storing under an
envelope the client believes it replaced, and the issue names the
chaos-rejected-MAIL leak as a defect in its own right. **Cost (the
losing side's):** two measured statements inside the existing chaos branch
plus one wire leg (a probability-100 rule at `mail_from` mid-transaction —
a pole, deterministic per 0020) — a permanent line-and-branch tax; and a
client that relied on the old envelope surviving a refused `MAIL` finds it
gone. That reliance *was* the defect.

**Ruled against robustness on the `503`'d nested `MAIL`: state unchanged.**
A second `MAIL` while a transaction is open answers `503 5.5.1` and
consumes *nothing* — bad-sequence refusals leave the transaction the
client believes is alive (RFC 5321 §4.1.4's own recourse is `RSET`);
consuming on 503 would silently kill it. The chaos arm differs because
there the refusal is a *failed attempt* (the attempt itself ended the old
transaction), not a refusal to begin.

**Adopted from robustness (its point 7): the two 503s stay distinct.**
`DATA` with no current `MAIL` answers `503 5.5.1 Error: need MAIL
command` — today it answers `need RCPT` (`:787-795`), reporting
out-of-sequence-ness with the wrong word; `DATA` with a `MAIL` but no
`RCPT` keeps `need RCPT` and leaves the transaction open (the client may
still add recipients). `RCPT` without `MAIL` already answers `need MAIL`
(`:761-769`). **Cost:** one extra measured branch and its leg — the
distinction is conformance honesty, not decoration.

**Adopted from both: the terminator is `.` CRLF only.** `:838` loses
`|| bytes == b".\n"`. A bare-LF `.` line is payload, passed verbatim by
the existing dot-unstuffing (`:842-846` strips only `..`); no lookahead,
no second read path. The per-line allowance floor `.max(3)`
(`:812-816`) **stays** — the CRLF terminator is three bytes and must be
recognizable at the cap and while draining — with the comment's
"`.` CRLF or `.` LF" clause reworded.

**Refused (simplicity's cuts, upheld):** the `SessionState`/`Transaction`
enum, a `reset_transaction` helper, a new 554 for "empty mail refused",
and any ingest-time "was the envelope real" flag. The transaction is two
locals with a per-command gate; the empty-mail defect was never a missing
refusal — it was the leak. The STARTTLS reset (`:662-664`) stays inline:
this batch adds **zero new session fields**, and a discipline is pinned
instead — *any* new SMTP session field must be reset in the STARTTLS arm
(hoist a reset function the day a third field joins). Robustness's
"pending LOGIN continuation survives the handshake" fear is groundless:
the continuation is read inside the `AUTH` arm on the same reader, and a
`STARTTLS` arriving there is a continuation line that fails to decode —
refused, not executed.

### #43 — the initial response is the username; refusal changes nothing

**Adopted from both: `AUTH LOGIN <b64>` takes the initial response as the
username and prompts only for the password** (RFC 4954 §4) — smtplib's
shape. Both mechanisms funnel every credential through one decode; the
inbox is assigned **only on success**.

**Ruled for robustness on the code: `501 5.5.2` is the only refusal.**
Undecodable base64, an empty-after-trim username, a `*` cancel at any
prompt and invalid UTF-8 all answer `501 5.5.2` and change nothing.
Simplicity's two-variant result (`501` for undecodable, `535 5.7.8` for
empty) is rejected: in accept-any mode there is no credential *validity*
to reject — 535 would assert a check that does not exist — and a 535 that
never fires is dead-arm vocabulary the exact 100/100/100 gate makes a
permanent tax. **Cost of the losing side:** the wire cannot distinguish
"malformed" from "names nothing" — no client can act on the distinction,
and 0028 records 535 as *reserved* rather than absent-by-accident. A
consequence worth stating: **zero new types** — with one refusal code, no
two-variant result is minted; `auth_plain_inbox`'s `Option` shape and one
decode helper carry the whole batch.

**Adopted from robustness: decoding is strict UTF-8, no lossy rescue.**
`from_utf8_lossy` maps *every* malformed username to the *same*
U+FFFD-minted inbox — one pooled mailbox for all malformed senders, the
exact cross-test contamination channel 0004 exists to prevent. **Cost:** a
client sending non-UTF-8 credentials gets `501` — SMTP AUTH credentials
are UTF-8 by RFC 4954, and no client relied on an U+FFFD inbox. The
existing `.trim()` stays (PLAIN's parity), so a whitespace-only username
trims to empty → `501`; the password is never validated — 0004's password
is decorative — and the only password inspection is the bare-`*` cancel
check (one comparison, the minimum that honors RFC 4954 §4's cancel).

**The decision, in one sentence (0028 §2, amending 0004):** `235` means
*the inbox named by the credentials is now the session's inbox*; any
credential that fails to name one answers `501 5.5.2` and changes
nothing — the pinned silent-`235` tests are rewritten to pin the refusal.

### #44 — the count is what RETR transmits, derived from the writer

**Ruled for robustness's shape: the count is produced by running the
writer, never by a second copy of its logic.** A tiny `wire_len(raw)`
stuffs `raw` into a scratch buffer and returns its length — the writer's
own output, so the count *is* the wire by construction. The PASS snapshot
derives `MaildropMsg::size` through it; RETR counts its own body buffer
(stuff into `body`, then `+OK {body.len()} octets`) so the one reply that
transmits cannot lie. Simplicity's `wire_octets` — a mirrored walk of the
same rules, drift-pinned by a unit test — is **rejected**: a second copy
of the wire logic is precisely the drift the issue is about, and the
pairing test is a permanent tax that exists only to detect the drift the
shape itself invites. **Cost of the losing side:** one scratch-buffer
allocation per counted message — bounded by `MAX_MESSAGE_SIZE` and the
per-inbox cap, the same order as the buffers RETR and the `store.list`
snapshot already compose (one reused buffer, `clear()`ed per message, is
the implementer's freedom) — and a ~4-line reshape of RETR's arm
(`src/pop3.rs:410-416`).

**The law includes dot-stuffing.** The framing's "CRLF-normalized length"
is shorthand: the existing corpus (`.lead\r\nmid\n.\r\r\nno-newline`) has
a dot-leading line whose wire is *longer* than its content by the stuffing
byte. The rule 0028 states is the issue's own title: **a POP3 size is the
length of the bytes RETR transmits for the message — CRLF-terminated,
dot-stuffed, excluding the final `.\r\n`.** For plain CRLF mail that
equals the stored length (today's pins keep their values); for any other
mail it grows.

**Refused (both):** an ingest-time wire-size column (duplicates the one
quantity the issue is about, drags the persistence schema), changing
`Email.size` (the REST/MCP contract — the stored raw is still served
byte-honest at `/raw`), a size cache (a cache can drift; a derivation
cannot), and any migration. The snapshot already materializes every mail
via `store.list` (`src/store.rs:498`), so the derivation is a local walk.

### #45 — the store's truth moves into the slot; no lock

**Adopted from both: a failed `USER` clears the pending name** — two
assignments in the USER arm's `-ERR` branches (`src/pop3.rs:295-303`), so
`PASS` afterwards answers `-ERR USER required first`. The PASS arm's own
re-check (`:305-311`) stays.

**Ruled for simplicity on the representation: `gone: bool` on the
`MaildropMsg` slot**, not robustness's second `BTreeSet<usize>`. A commit
marks the entry it removed — the fact lives on the slot it describes, so
no second collection can disagree with the first, and the three consults
(`resolve` `:554-568`, `counts` `:569-582`, `live_numbers` `:583-595`)
read the slot they already index. The QUIT loop moves each successful
number out of `deleted` and sets its slot's `gone`; the first failure
breaks with the suffix unmarked (at-least-once, 0015 §1). **Cost:**
none functional — the set shape would touch the same three consults with
two collections to keep in sync.

**Adopted from both: `RSET` clears `deleted` and nothing else.** A
committed delete is the store's truth, not a mark: it cannot be
un-deleted by `RSET`, which closes the 0015 §1 violation. After a partial
`QUIT`: the committed prefix's numbers are `gone` (so `RETR` answers
`-ERR no such message` — honest at last), the failing suffix stays
marked (`-ERR message already deleted`, still in the store), and `STAT`
/ `LIST` agree with what `RETR` can serve. A retry `QUIT` applies the
suffix idempotently (`Store::delete`'s honest `Ok(false)` on an absent
id, `src/store.rs:565-591`) and signs off. The `QUIT` comment
(`:489-504`) is rewritten to that contract in 0015 §1's own words — the
comment is the one place the current tree *says* the wrong thing.

**Ruled for both: no maildrop lock.** Robustness's ground is decisive: a
lock that keeps a snapshot's mail fetchable must exempt locked inboxes
from oldest-only pruning (0003), which unbounds memory under a hung
client; simplicity adds that HTTP bypasses the lock entirely, making it
security theater. **Robustness wins the "write it down" point:** the
snapshot-not-lock posture is recorded in 0028 §3 and pinned by a wire leg
(clear an inbox over HTTP mid-session → `LIST` still names the mail —
the snapshot is the session's stable view — and `RETR` on it answers
`-ERR no such message` — the store's truth). That is the documented skew:
*listings answer from the snapshot; fetches answer from the store.*

### Test placement (decision 0020, binding)

Zero new files under `src/`; no harness moves. Every cold path this batch
adds is drivable on the real wire through existing instruments: the
selective SQLite trigger (`tests/common::refusing_trigger_except`) for
the partial `QUIT`, probability-100 chaos rules for the refusal arms, the
HTTP API for the mid-session skew — so 0020 rule 3 keeps them in
`tests/pop3.rs` and `tests/smtp_edges.rs`, and rule 1 keeps the
branch-stable unit tests (`counts`/`resolve` with `gone` slots; the AUTH
decode helper) inline in their owning modules. No sleeps, no wall-clock
assertions, chaos at the poles only, per-test fresh stores/inboxes.

## Named residuals (accepted, not hidden)

- A bare-LF-terminating client now waits on `.\n` until the idle deadline
  answers `421`/`-ERR` (0027's bound). That *is* the smuggling class the
  issue closes; the bound is the fence.
- `gone` is session memory: it dies with the socket. 0015's convergence
  contract is cross-session — a fresh session re-derives truth from the
  store — and a retried `QUIT` re-lands nothing.
- `LIST`/`STAT` over-report for mail removed elsewhere, until the session
  ends; `RETR`/`TOP` answer honestly. The skew is documented (0028 §3)
  and bounded — the snapshot is a fixed materialization, not a growing
  one.
- PASS pays an O(inbox bytes) stuffing pass to derive sizes, bounded by
  the per-inbox cap (0003); it already clones every raw byte via
  `store.list`.
- `535 5.7.8` never fires in accept-any mode — reserved, recorded in 0028.
- Nested `MAIL`/`AUTH` answering `503` is stricter than servers that
  implicitly reset; a non-conformant client gets the standard retryable
  refusal and keeps its transaction.
- The store-side "how an inbox was emptied changes USER's answer" is
  deliberately untouched (framing: tracked separately).

## Frozen guarantees — checked

| Guarantee | Effect of this batch |
|---|---|
| 0001 accept == stored | **Untouched.** The take runs before `build_email`/`park_delivery`/`insert_async`; no `await` is added between the terminator and the commit; the `250` is still written only after the insert resolves (`:897-905`). A refused `DATA`/`MAIL` stores nothing. |
| 0002 await subscribes before it checks | Untouched — no watcher code changes. |
| 0003 oldest-only pruning | **Protected.** The maildrop lock is refused *because* honoring it would exempt locked inboxes from pruning; pruning never negotiates with a session. The snapshot is a copy. |
| 0004 per-test inboxes | **Amended by 0028 (proposed).** The username still names the inbox for any credentials; a credential that fails to name one is refused `501` instead of silently succeeding — isolation restored, not weakened. |
| 0005 extraction at ingest | Untouched — no ingest normalization, no new `Email` field, `Email.size` keeps its meaning (stored raw length); `/raw` stays byte-honest. |

**Zero-diff demand (binding for review):** `src/store.rs`,
`src/persist.rs`, `src/model.rs`, `src/extract.rs`, `src/api.rs`,
`src/mcp.rs`, `src/chaos.rs`, `src/threads.rs`, `src/compat.rs`,
`src/webhook.rs`, `src/tls.rs`, `src/config.rs`, `src/feed.rs`,
`src/stdio.rs`, `src/ui.rs`, `src/lib.rs`, `src/main.rs` carry zero diff.
The batch touches `src/smtp.rs`, `src/pop3.rs`, `README.md` (one
sentence), `tests/smtp_edges.rs`, `tests/pop3.rs`, and — only if a leg
genuinely needs a helper — `tests/common/mod.rs`. Anything else means the
implementation has left the spec.

## Acceptance Criteria

Numbered; C1-C6 are #42, A1-A4 #43, O1-O3 #44, P1-P4 #45, G the wave
gate. Each is testable on the wire against real servers (no mocks).

- **AC1 (C1, the leak).** After any completed `DATA` — stored `250`, or
  refused `552`, chaos `451`, or commit-refused `451 4.3.0 storage
  commit failed` — a following `RCPT` answers
  `503 5.5.1 Error: need MAIL command` and a following `DATA` answers
  `503 5.5.1 Error: need RCPT command`; the pipelining chain
  `MAIL` → `RCPT` → `DATA` → `MAIL`(refused) → `DATA` → `.` stores
  **nothing** (no empty mail — the second `DATA` gets `503`, and the lone
  `.` is answered as a command, not data).
- **AC2 (C2, the terminator).** A session sending `foo\n.\nbar\r\n.\r\n`
  as data stores **exactly one** message whose raw payload contains
  `.\n`; nothing after the first `.\n` is parsed as a command (no split
  message, no smuggling). `DATA` ends only on `.` CRLF.
- **AC3 (C3, out-of-sequence).** A second `MAIL` while a transaction is
  open answers `503 5.5.1` and leaves the transaction intact (a following
  `DATA` still succeeds); `AUTH` while a transaction is open answers
  `503` and the transaction survives (the envelope is still usable).
- **AC4 (C4, refused MAIL).** With a probability-100 chaos rule at
  `mail_from` mid-transaction, the refused `MAIL` answers its error and
  the previous envelope is gone: a following `DATA` answers `503 5.5.1`
  and nothing is stored (exact store count unchanged).
- **AC5 (C5, distinct 503s).** `DATA` with no `MAIL` at all answers
  `503 5.5.1 Error: need MAIL command` — not `need RCPT`; `DATA` with a
  `MAIL` but no `RCPT` answers `503 5.5.1 Error: need RCPT command` and
  leaves the transaction open (the client may add recipients).
- **AC6 (C6, no regressions).** The existing STARTTLS (state reset),
  pipelining and dot-unstuffing tests —
  `tests/smtp_edges.rs`, `tests/starttls.rs`, `tests/p2.rs`,
  `tests/burst.rs` — pass **unmodified**.
- **AC7 (A1, the initial response).** `AUTH LOGIN <b64-username>` answers
  exactly one prompt, `334 UGFzc3dvcmQ6`; after the password the reply is
  `235`; a mail sent in that session lands in the **named** inbox
  (the smtplib-shaped real-protocol leg — the issue's own scenario).
- **AC8 (A2, refusals).** Undecodable base64 at the username prompt or in
  the initial response, an empty decoded username, a `*` cancel at any
  prompt (PLAIN and LOGIN), and a non-UTF-8 username each answer
  `501 5.5.2`, change nothing, and the session's inbox stays what it was
  (a later delivery flows to `default` or the previously authenticated
  inbox — assert via the HTTP API).
- **AC9 (A3, PLAIN parity).** `AUTH PLAIN !!!not-b64!!!`,
  `AUTH PLAIN` with an empty authcid, and a `*` cancel at the PLAIN
  prompt answer `501 5.5.2` — the three pinned silent-`235` tests are
  rewritten to pin the refusal.
- **AC10 (A4, the contract).** The rewritten tests pin: `235` iff the
  credentials named an inbox, which is now the session's inbox; the
  decision document records the posture amending 0004.
- **AC11 (O1, the rule).** For bare-LF mail, CRLF mail and mixed/dot-
  stuffed mail (the corpus `retr_stuffs_leading_dots…` already holds),
  every advertised size — RETR's `+OK n octets`, `STAT`, the `LIST`
  argument and multi-line form, the `PASS` and `RSET` summaries — equals
  the number of bytes RETR transmits for that message (CRLF-terminated,
  dot-stuffed, excluding the final `.\r\n`).
- **AC12 (O2, derived expectations).** `tests/pop3.rs`'s `octets()` (and
  any expectation) is derived from the wire transformation applied to the
  fixture bytes — not from the stored length — and at least one leg pins
  `advertised == len(transmitted bytes)` directly (`data_bytes()`
  length).
- **AC13 (O3, the documents).** README states the rule (replacing the
  `:314-315` sentence) and decision 0028 §1 records it with the rejected
  alternatives (mirror function, ingest field, cache).
- **AC14 (P1, USER).** A `USER` that answers `-ERR` (missing username or
  no such mailbox) clears the pending name; a following `PASS` answers
  `-ERR USER required first` — not a login into the previous mailbox.
- **AC15 (P2, no resurrection).** In the partial-`QUIT` leg
  (`refusing_trigger_except`): after the mid-loop `-ERR storage commit
  failed`, `RETR` on a **committed** number answers
  `-ERR no such message` (never "message already deleted"), `RETR` on the
  failing suffix answers `-ERR message already deleted`, `RSET` answers
  a summary that agrees with what `RETR` can serve, and a retry `QUIT`
  applies the suffix and signs off.
- **AC16 (P3, the skew is documented and tested).** A wire leg clears an
  inbox over HTTP mid-session: `LIST`/`STAT` still name the snapshot's
  mail and `RETR`/`TOP` on it answer `-ERR no such message` — the
  documented posture (0028 §3), not a lock.
- **AC17 (P4, the comment).** `src/pop3.rs`'s `QUIT` comment states the
  code's contract in 0015 §1's words: the committed prefix's numbers
  leave the mark set and their slots are marked gone; the failing suffix
  stays marked, unapplied; nothing reports deleted and resurrects.
- **AC18 (G, the gates).** `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked`,
  `brokkr compile --bundle bundles/self`, `scripts/coverage-gate.sh`
  (exact 100/100/100 — every new line measured; `coverage(off)`
  forbidden) and `cargo deny check licenses` are green; the zero-diff
  demand holds; the binary tests' `smtp=` startup-line contract stays
  plain when piped (no ANSI change).
