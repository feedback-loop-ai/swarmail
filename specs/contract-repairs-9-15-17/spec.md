# Spec — Contract repairs #9/#15/#17: append-stable thread keys, a loud shim DELETE, one chaos delay rule

Epic: feedback-loop-ai/swarmail#13, wave 7 (issues #9, #15, #17). Branch:
`feat/epic13-09-15-17-contracts`.

Decision: 0011 —
[docs/decisions/0011-shim-delete-contract.md](../../docs/decisions/0011-shim-delete-contract.md)
(status `proposed` — the operator accepts; the change ships together with
it). #9 and #17 need no ADR: both are repairs *toward* what the docs and
the issue already promise — the code is what lies — so the contract
surface moves back under its documented intent. #15 deliberately changes a
documented behavior, so it carries the decision document.

## The problem — three contracts that self-destruct

All anchors re-verified on the working tree this session.

1. **#9 — the advertised thread URL changes under ordinary growth.**
   `group_threads` names each thread by its component's *lexicographically
   smallest* member and hashes that into the URL key
   (`src/threads.rs:227`). A reply whose message-id sorts before the
   root's flips the representative — and with it the key the UI, the REST
   thread view and the SSE feed just advertised. The README already
   promises a "stable `key`" (`README.md:313`,
   `src/static/llms.txt:74`); the code is what lies.
2. **#15 — one syntax error wipes every inbox.** `compat::delete_messages`
   (`src/compat.rs:257-272`) maps an absent, empty, unparseable,
   wrong-typed **or empty-`ids`** body to `clear_all()`. In a
   thousand-session parallel run, one session's typo'd body is every
   session's data loss, and it manufactures the #6-style false pass: the
   test that meant to assert "mail arrived" observes an empty inbox and
   passes. Decision 0004 names `clear_all` (i.e.
   `DELETE /api/v1/delete-all`) as "the only cross-inbox destructive
   endpoint" — the shim's fallback made that sentence false.
3. **#17 — a delay-only DATA chaos rule stores the mail it was supposed to
   break.** `connect`, `mail_from` and `rcpt` apply a fired rule's delay
   and then always fail (`src/smtp.rs:202-210, 374-383, 388-396`). DATA
   (`src/smtp.rs:439-450`) nests its reply behind
   `if rule.error.is_some()`, so a delay-only DATA rule delays, then falls
   through to `store.insert` and a `250` — a chaos knob that cannot fail,
   silently defeating exactly the retry-suite testing the feature exists
   for, and contradicting `ChaosEvent::Data`'s own doc ("Fail after DATA
   (mail is NOT stored)").

## Ruling — how the two positions were reconciled

The panel's positions (`.forge/design/positions/{simplicity,robustness}.md`,
both `pass`) agree on the shape of all three repairs. They conflict on
four; this spec rules rather than averages.

**Adopted from both.** #9 keeps the union-find, the node naming and all
grouping byte-for-byte and changes only what the key is hashed from. #15
deletes the `clear_all` fallback — the handler gets *smaller* — and
deletion keeps flowing synchronously through `Store::delete`, so watchers
wake and REST/SSE/UI cannot disagree. #17's entire production change is
removing DATA's `error.is_some()` nesting; `connect`, `mail_from` and
`rcpt` are already correct and get zero diff. No new files, no new types,
no new dependencies; the wave is net-negative production code plus one
constitutionally mandated ADR.

**Ruled for simplicity on #9 — the anchor is the conversation's own first
message, recomputed per request.** The key becomes
`thread_key(&thread_node(emails[0]))` (`src/threads.rs:227`): the FNV-1a
of the *oldest* mail's node — its `Message-ID`, else its `In-Reply-To`/
first `References` entry, else the normalized-subject node, else its own
store id. A reply is always newer than the conversation it joins, so
appending can never change the oldest member and the key is append-stable
by construction; the issue's exact scenario (root `<z@x.io>`, later reply
`<a@x.io>`) keeps the original URL. **Rejected: robustness's ingest-stamped,
persisted `thread_id`.** Three grounds. *Scope:* the intake pre-ruled it —
A4 says "no data migration is needed because threads are recomputed per
request" and a documented migration note is the required artifact; the
wave's contract is append-stability, not immortality. *Semantics:* with
`Arc<Email>` sharing, an ingest-time stamp can never merge retroactively,
so a bridge mail (`References: [a, b]` across two threads) would split
conversations where today's union-find merges them — the issue pins
"keep grouping by in_reply_to chains", and the stamp design changes
grouping where the anchor design does not. *Blast radius:* a new `Email`
field, two per-inbox store indexes, a `persist.rs` `ADDED_COLUMNS`
migration, and a bounded hostile-header walk inside the synchronous 0001
insert path — every branch of it 100%-coverage-gated — in a wave whose
other two repairs are deletions. **The losing side's cost, named and
recorded:** the key changes once when the conversation's oldest mail
leaves the newest-200 view window, is deleted, or is cap-pruned (0003
evicts the oldest first); a restart after a pruned root re-keys; and two
mails accepted in the same millisecond tie-break on id. The migration
note (README *Threads and the live UI* + the `src/threads.rs` module doc)
states every one of these honestly, and the persisted-anchor ADR is the
pre-authorized follow-up should the realm ever need stability beyond
append. Robustness's other two #9 findings — that per-request roots are
unstable under `VIEW_CAP` and under the cap — are *true* and are exactly
what the migration note documents; they are the residual, not the design.

**Ruled for robustness on `{"ids": []}` — it is a 400, not a silent
`{"removed": 0}`.** The intake's B-criteria pin 400 only for
absent/empty/unparseable *bodies* and leave the empty selector open;
robustness closes it, simplicity keeps it a no-op. The destructive
endpoint gets one wire rule — *name a non-empty set of ids, or nothing
happens* — with no case analysis on the wire. A silent success shape
whose only client is a buggy one is precisely the #6 failure shape: the
environment silently cooperating with a test's cleanup bug, leaving stale
mail to poison its own later assertions. **The losing side's cost,
named:** a client that legitimately computes an empty batch must skip the
call (or read the 400 as "nothing to delete"), and Mailpit compatibility
narrows by one more behavior than the minimum. **Ruled for robustness
again on the 400's body:** it carries a short JSON reason via the
`ApiError` already imported in `compat.rs` — the same style the
Mailpit-side search on the same surface already answers with
(`src/compat.rs:288-292`); simplicity's bare status line would cost one
line saved and one question unanswered. **Ruled against robustness on
scope:** no `inbox` selector (per-inbox wipe already exists at
`DELETE /api/v1/inboxes/{inbox}/messages`, `src/api.rs:67-69` — a second
one widens the destructive surface), and no dry-run/confirm/wipe-token
ceremony: the hazard is closed by *refusing* the destructive path, not by
surrounding it. `DELETE /api/v1/delete-all` is untouched — it is 0004's
sanctioned wipe and stays the only one.

**Ruled for both on #17 — one invariant, stated once:** *a fired chaos
rule always applies its delay, then always produces that stage's failure;
it never produces a success reply.* Concretely: connect = wait, then
close **without a banner** (an explicit error is written first when
given — a refusal greeting, legal under RFC 5321 — no banner exists to
replace); `mail_from`/`rcpt`/`data` = wait, then the explicit error or
the default `451 4.3.0 chaos`. The default stays 4xx deliberately: `451`
invites the retry a chaos test exists to observe; a 5xx would poison the
message permanently. The production change is deleting DATA's nesting so
the code matches `ChaosEvent::Data`'s doc. Docs carry the per-stage table
(README *Chaos* section), one sentence on `ChaosRule::error` (`None`
means *the stage's default failure*, not *no failure*), and the two
semantics robustness pinned because tests can drift around them: the
over-limit `552 5.3.4` check precedes the DATA chaos check (real resource
failures precede injected ones — true in the code today,
`src/smtp.rs:435-439`), and the DATA delay applies after the body is
received, before the reply (the transfer itself is never slowed).
**Ruled against robustness on the delay cap:** the proposed 400 above a
documented `delay_ms` maximum is refused this wave — it is not in the
issue, it is a new contract on `PUT /api/v1/chaos` *and* MCP
`swarmail_set_chaos` with an arbitrary threshold, and the failure it
forecloses (a fat-fingered delay parking sessions) is self-inflicted and
recoverable (clear chaos, restart) under the documented no-idle-timeout
posture. **That refusal's cost, named:** an operator can still stall
real sessions with one typo; the residual is routed to its own issue
below. **Ruled against robustness on the test harness:** no injected
clocks — `probability: 100` makes firing deterministic, `delay_ms` in the
*rule* exercises the delay line end-to-end over real sockets, and the
*test* sleeps never (bounded `tokio::time::timeout` only where a close
must be observed, the pattern `tests/smtp_edges.rs:399-413` already
accepts). Simplicity's harness wins; a `Clock` trait would mint
coverage-gated branches whose only job is proving tokio sleeps.

## Named residuals — the honest-losses ledger

Routed, not smuggled in. None is a wave-7 contract.

- **Thread keys re-key once** when the oldest mail leaves the window /
  store (accepted cost of the #9 ruling; documented in the migration
  note).
- **FNV-1a 64-bit is collidable** by an adversary who knows a target
  anchor. Impact is an ambiguous thread-view URL in a test tool — no mail
  is lost, joining is by id equality, not by hash. Accepted.
- **An unclamped chaos `delay_ms`** with no idle timeout behind it
  (refused in-wave, above).
- **A single unterminated line in DATA** parks an unbounded buffer
  (`read_line` into `String`; `max_message_size` bounds the accumulated
  body, not one line). Pre-existing, adjacent to #17's theme; its own
  issue (a per-line cap in the DATA loop).
- **Cert expiry mid-run** fails handshakes in ways that look like chaos.
  Harness hygiene (generate per run); the cheap operability line — log
  `not-after` at startup — belongs to that same issue.

## Frozen guarantees — checked

0001-0003 untouched: no stored thread state is added (#9), the 400 path
and the 451 path store nothing and both fire strictly before any insert
(#15, #17), and the `250` sequence at `src/smtp.rs:452-456` is
byte-identical before and after this wave. 0004 is *restored*: the shim's
fallback stops being a second cross-inbox wipe. 0005 untouched: no
extraction changes. Detail in [plan.md](plan.md) → Constitution check — the
zero-diff demand on `src/store.rs`, `src/persist.rs` and the accept path
is binding for review.

## Acceptance Criteria

Wave-level: every gate stays green — `cargo fmt --all -- --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`,
`brokkr compile --bundle .`, `bash scripts/coverage-gate.sh` (exact
100/100/100, missed == 0), `cargo deny check licenses`. No new
dependencies. Real servers in every new test; no mocks; no sleeps.

### #9 — append-stable thread keys

- **AC1 (the issue's exact regression; red on current code).** A thread
  containing a mail with `Message-ID <z@x.io>`, then a reply
  `Message-ID <a@x.io>` / `In-Reply-To <z@x.io>` (`a` sorts before `z`),
  keeps the **same** thread key in `GET /api/v1/inboxes/{inbox}/threads`
  before and after the reply arrives. Exists twice: a unit test in
  `src/threads.rs` (`key(group([z])) == key(group([z, reply]))`) and one
  over real SMTP in `tests/ui.rs` (send, record key, reply, assert the
  key is unchanged and `count == 2`).
- **AC2.** The key is a pure function of the conversation's oldest
  member's node — `Message-ID`, else `In-Reply-To`, else first
  `References` entry, else the normalized-subject node, else the mail's
  own id — and is invariant under appending further replies.
- **AC3.** Grouping is preserved: `a_reference_chain_is_one_thread_in_conversation_order`,
  `in_reply_to_alone_links_a_reply`, `cyclic_references_collapse_into_one_thread`,
  `subject_fallback_groups_chainless_mail`,
  `duplicate_message_ids_still_land_in_one_thread` and the real-SMTP
  `tests/ui.rs` thread suite pass **unmodified**.
- **AC4.** A migration note — README *Threads and the live UI* plus the
  `src/threads.rs` module doc — states that keys for pre-existing mail
  are rebuilt by the new derivation on the next request (no data
  migration), that previously advertised URLs may change once for old
  data, and names the residual re-key conditions (oldest mail leaving the
  newest-200 window; deletion/cap-prune of the oldest; equal-received_ms
  id tie-break).

### #15 — the shim DELETE contract (decision 0011)

- **AC5 (B1).** `DELETE /api/v1/messages` with **no body** answers `400`
  and the store is byte-identical afterwards — asserted by a subsequent
  list/count of **both** seeded inboxes, not just the status code.
- **AC6 (B2).** An **empty** body answers `400` and deletes nothing.
- **AC7 (B3).** An **unparseable** body — non-JSON, wrong-typed,
  `{"ids": null}` — answers `400` and deletes nothing.
- **AC8 (ruling).** `{"ids": []}` answers `400` and deletes nothing (not
  a silent `{"removed": 0}`).
- **AC9 (B4).** `{"ids": [a1, b2]}` answers `200 {"removed": 2}`, deletes
  exactly those ids, and every other mail survives — the previously valid
  case is unchanged.
- **AC10 (B5).** No request shape on this route wipes all inboxes;
  `clear_all` is reachable only via `DELETE /api/v1/delete-all`, whose
  behavior is pinned unchanged in the same rewritten test (0004's
  posture holds again).
- **AC11 (B6).** The docs describe the new contract: the README API-table
  row, the `openapi.json` delete operation on `/api/v1/messages` and the
  `llms.txt` shim line — riding in the same commit as the wire change.
- **AC12 (B7).** `tests/compat.rs::shim_deletes_go_through_the_store` is
  rewritten to endorse the new contract — the 400s (each followed by an
  exact-count/before-after assert including the *other* inbox surviving),
  the selective delete through the real store, and the `delete-all`
  alias. The wipe-on-garbage sections are gone.
- **AC13 (B8).** `docs/decisions/0011-shim-delete-contract.md` exists
  with status `proposed`, states the contract, names each
  previously-destructive input (absent body, empty body, non-JSON, JSON
  without a valid `ids` array, `{"ids": []}`), gives the
  parallel-test-isolation rationale (tying to #6's false-pass hazard),
  and records the rejected alternatives (wipe-behind-a-flag, inbox
  scope, silent no-op).

### #17 — one delay-only chaos policy

- **AC14 (C1).** One policy table (README *Chaos* section, plus a doc
  sentence on `ChaosRule::error`) defines delay-only behavior per stage,
  and the implementation equals it at all four stages: connect = wait
  then close without a banner; `mail_from`/`rcpt`/`data` = wait then
  `451 4.3.0 chaos`; an explicit `error` replaces the default at every
  stage. The two pinned semantics are written down: over-limit `552`
  precedes the DATA chaos check; the DATA delay applies after the body is
  received, before the reply.
- **AC15 (C2).** Deterministic (no sleeps): with a delay-only connect
  rule (`probability: 100`, `delay_ms > 0`), the session yields **no line
  at all** — the client's reply is `""` under a bounded wait — neither a
  banner nor a `451`.
- **AC16 (C3).** Deterministic (no sleeps), one test per stage: with a
  delay-only rule at `mail_from`, at `rcpt` and at `data`, the reply is
  exactly `451 4.3.0 chaos`; the DATA case additionally asserts the exact
  store count is unchanged — the mail is **not** stored.
- **AC17 (C4).** Explicit-error rules keep today's behavior at every
  stage, delay still applied: connect answers the custom line, `mail_from`
  answers `450 busy`, data answers `554 rejected` (the existing sections,
  updated only where they endorsed the DATA fall-through).
- **AC18 (C5).** The exact 100/100/100 gate stays green: the deleted DATA
  branch shrinks the denominator, the flattened arm is executed by both
  DATA sections (delay-only and custom-error), the 400 arm by the
  rewritten shim tests, and the one changed line in `group_threads` by
  the AC1/AC3 tests. No `coverage(off)` anywhere.
