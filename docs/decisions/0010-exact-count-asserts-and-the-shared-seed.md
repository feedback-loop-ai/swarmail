# 0010 — Exact-count asserts and the one shared seed

- **Status:** accepted — ratified 2026-10-11 at epic #13 wave-4 intake (run
  `epic-13-wave-4-completion-the-de-40d5452a`): every anchor re-verified on
  the working tree (`src/mcp.rs:221-222` string schemas and the five string
  `inputSchema` sites, the `.max(1)` clamp at `src/api.rs:142-143`,
  `seed_inbox` at `src/api.rs:424`, `build_email` at `src/smtp.rs:523`) and
  no contradiction with the code or the frozen guarantees found, so no
  amendment was required. The "wave 5" label vs the run's "wave 4
  completion" is cosmetic; issues #4/#5/#6 and branch
  `feat/epic13-04-05-06-contract-repairs` are the authoritative anchors.
- **Date:** 2026-10-11
- **Supersedes:** none
- **Companion:** epic #13 wave 5 (issues #4, #5, #6)
- **Numbering:** 0010 is the next free number on this branch. The unmerged
  wave-3 branch `feat/epic13-03-14-watcher-atomicity` carries an unrelated
  provisional 0010 (`watcher-channel-lifetime`, also `proposed`); if both
  land, the operator renumbers one at acceptance.

## Context

Three published contracts lie on the wire. Five MCP tools publish
`inputSchema` as a JSON string because two `&str` constants are embedded in
`json!` (`src/mcp.rs:221-222`); `swarmail_seed_email` advertises an `html`
argument the handler drops, while REST's seed endpoint handles HTML — two
raw builders, one of them wrong; and `assert?count=0` clamps to one
(`AwaitQuery::count()` applies `.max(1)`), so the negative assertion an
agent needs — "nothing from this sender arrived" — is impossible, even
though the OpenAPI document already promises an "exact match count".

None of this touches the frozen guarantees: 0001's synchronous insert,
0002's subscribe-before-check and 0003's oldest-only pruning are inputs to
the wave, not surfaces of it. What the wave does change is public contract
behavior — the assert endpoint's observable answer flips on `count=0`
(`409→200` on an empty inbox, `200→409` on a one-message inbox), MCP seed
gains the HTML its schema already advertises, and the five schemas change
JSON type — so the *rulings behind the change* are recorded here rather
than left in a diff.

## Ruling

1. **A WAIT keeps minimum-one; an ASSERT is exact, zero included.** The two
   consumers of the old clamp have different contracts and get different
   named accessors on `AwaitQuery`:
   - `min_one_count()` = `count.unwrap_or(1).max(1)` — `GET …/await` and
     MCP `swarmail_wait_for_email`. A caller who means "wait for at least
     one" and passes `0` by accident must not get an instant success
     against an empty mailbox: that is the flake shape 0002 exists to kill,
     wearing a green coat.
   - `exact_count()` = `count.unwrap_or(1)` — `GET …/assert`, which tests
     the real **filtered** match count (`store.count(&inbox, &filter)`) and
     must answer `200 {matched: 0}` on an empty inbox and `409 {expected:
     0, matched: 1}` on a one-message inbox. `assert?count=0` is the
     negative-assertion primitive; `assert` never subscribes or waits — a
     snapshot is a snapshot.
   The clamp never moves into `Store::wait_for`: `wait_for(want=0)` already
   behaves honestly (subscribe → scan → deadline loop); the min-one rule is
   query-layer semantics. The two min-one clamps that remain (REST await,
   MCP wait) are named and pinned by test so no future cleanup re-merges
   them into the assert path by accident.

2. **Seed construction has ONE owner, and the fixture constructor refuses
   hostile fields.** A single constructor beside `build_email` in
   `src/smtp.rs` owns the defaults (`from = "fixture@swarmail.dev"`,
   `subject = "(no subject)"`, absent body → empty), the raw shape
   (HTML preserved verbatim with `MIME-Version: 1.0` +
   `Content-Type: text/html` when `html` is given; plain text otherwise),
   html-wins precedence when both are given, and the `build_email` call
   (extraction stays at ingest, 0005). REST and MCP both call it; neither
   builds raw bytes again; parsing stays local to each adapter.
   The constructor rejects `CR`, `LF` or `NUL` in `to`, `from` or `subject`
   **before** any insert — REST `400` naming the field, MCP `isError` with
   the same reason. The asymmetry with the SMTP sink is deliberate and is
   the point: `build_email` runs on hostile bytes all day (a mail sink
   stores whatever the wire carries; only oversize gets `552`), while a
   fixture *promises* the caller that the mail that lands is the fixture
   that was asked for. A fixture whose envelope field smuggles a header
   lands as a mail that lies about itself — the same lie-class that makes
   an assert meaningless. Hostile bytes are a capture problem; hostile
   fields are a fixture-contract problem.

3. **Published schemas are objects, and their agreement with handlers is
   enforced by test.** The two string constants become object values under
   the same names; `inputSchema` is a JSON object for all twelve tools. A
   test-time parity check asserts every `required` argument is enforced by
   its handler — closing the drift class that produced #5 (a schema
   advertising an argument the handler ignores). Runtime validation of tool
   arguments against `inputSchema` is refused: a new capability, a new
   failure mode per call, a new 100% branch burden for nothing the issues
   name.

## Why

The three repairs share one property: each brings an adapter into line with
what the other adapter and the docs already promise. The seed fix is the
sharpest case — an HTML fixture seeded over MCP stores `html: null` with no
links or codes, silently, per adapter, while the schema advertises the
argument. One constructor ends the drift; two builders *are* the bug. And
the exact-count fix is what makes the seed path trustworthy in the first
place: `seed` (act) and `assert?count=0` (nothing arrived) compose into the
deterministic clear → act → assert loop the docs teach — including the
chaos pair, where a `data` chaos rule stores nothing and the assert
confirms it deterministically.

## Consequences

- `assert?count=0` behavior flips (empty: `409→200`; one message:
  `200→409`). No test or doc pinned the old behavior; OpenAPI's `default: 1`
  for an absent `count` remains true.
- Seeding a fixture with hostile envelope fields is refused where it used to
  store a lying mail. Nobody legitimate seeds CR/LF in envelope fields; the
  gain is a fixture that cannot lie about itself.
- Both-given `text` and `html` stores HTML and drops the text — documented
  (tool description, `SeedRequest` description, llms.txt) rather than
  silent. Multipart/alternative would be a different, larger change.
- Exact-count asserts are deterministic per 0004's private per-test inboxes
  and 0001's synchronicity; they are a *moment*, not a span — a caller
  asserting zero while its own mail is in flight races itself, and the
  clear → act → assert recipe is the deterministic answer. Exact counts
  hold only for mail younger than the per-inbox cap (0003, as documented).
- Schema/handler agreement is enforced by test, not by types — the honest
  residual of refusing `schemars`.
- The seed constructor is new production code under the exact 100/100/100
  gate: every arm is wire-reachable and ships with a real-protocol test.
  The gate refusing an uncovered arm is the constitution working.
- Zero diff to `src/store.rs`, `src/persist.rs` and the SMTP accept path:
  the `250` sequence stays byte-identical. No new dependencies.
