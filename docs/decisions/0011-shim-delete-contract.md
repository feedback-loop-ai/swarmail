# 0011 — The shim DELETE contract: an explicit selector or a 400

- **Status:** accepted (operator, 2026-10-16 — "accept all") — drafted at epic #13 wave-7
  design (issues #9, #15, #17; run
  `epic-13-wave-7-three-contract-li-710b205d`). The change ships together
  with this document; the operator accepted both.
- **Date:** 2026-10-12
- **Supersedes:** none. Amends the *practice* under decision 0004, not
  0004 itself — 0004's sentence "clear_all … is the only cross-inbox
  destructive endpoint" becomes true again; nothing in 0004 changes.
- **Companion:** epic #13 wave 7 (issue #15). Branch
  `feat/epic13-09-15-17-contracts`.
- **Numbering:** 0011 is the next free number on this branch — the 0010
  number is already doubled (`0010-exact-count-asserts-and-the-shared-seed`,
  `0010-watcher-channel-lifetime`), so 0011 is a new unique file.

## Context

`DELETE /api/v1/messages` is the Mailpit/MailHog compat shim's delete.
Its handler (`src/compat.rs`, `delete_messages`) maps *any* body it
cannot use — absent, empty, unparseable, wrong-typed, or an empty `ids`
array — to `None`, and `None` calls `store.clear_all()`: the whole store,
every inbox. The doc comment endorses it ("an absent, empty or
unparseable body wipes everything, like upstream's decoders do"), so this
is a documented behavior, and changing it needs a decision.

That behavior is not a degenerate corner; it is the most likely request a
real client sends wrong:

- `Content-Type: text/plain` with valid JSON inside — a rejection;
- a typo'd key (`{"id": [...]}`) or `{"ids": null}` — a rejection;
- an empty body from a client that dropped it on a redirect — a wipe;
- `{"ids": []}` — a client that computed its batch and got nothing — a
  wipe.

In the world decision 0004 built — a thousand parallel test sessions
sharing one server, each owning its inbox — one session's typo is every
session's data loss. Worse, it manufactures the #6-style false pass: the
test that meant to assert "mail arrived" observes an empty inbox and
*passes*, because the environment silently cooperated with the bug. And
it contradicts 0004's own posture: `clear_all` was supposed to be the
only cross-inbox destructive endpoint, reachable only through the
explicit `DELETE /api/v1/delete-all` alias.

## Ruling

**`DELETE /api/v1/messages` deletes exactly the non-empty `ids` array and
replies `200 {"removed": n}`. Every other body is a `400` that deletes
nothing.**

- Deletion requires an explicit selector: a JSON object with a non-empty
  `ids` array of message ids. The deletion flows synchronously through
  `Store::delete` (decision 0001: accept == stored; decision 0002:
  removals wake watchers), exactly as the selective path does today.
- A `400` carries a short JSON reason and destroys nothing. It is the
  same error surface the Mailpit-side search on this shim already answers
  with.
- `{"ids": []}` is a `400`, not a silent `200 {"removed": 0}`. A
  destructive endpoint gets one wire rule — *name a non-empty set of
  targets, or nothing happens* — with no case analysis on the wire. A
  silent no-op success shape has exactly one client: a buggy one whose
  cleanup silently did nothing and whose later assertions now run against
  stale mail.
- No `inbox` selector is added. Per-inbox wiping already exists
  (`DELETE /api/v1/inboxes/{inbox}/messages`); a second selector would
  duplicate it and widen the destructive surface. `DELETE
  /api/v1/delete-all` is unchanged and remains the only cross-inbox
  wipe.

The previously-destructive inputs, named so the change is auditable —
each was a wipe, each is now a `400`:

1. an absent body;
2. an empty body;
3. a non-JSON body;
4. a JSON body without a valid `ids` array (missing, wrong-typed, or
   `null`);
5. `{"ids": []}`.

## Why

Per-test isolation is the product (0004). A fallback that turns "could
not parse your request" into "destroy every inbox" is the single worst
failure direction a test tool can have: it converts a client bug into
cross-session data loss and then hides the loss behind a green run. The
repair restores the proven posture rather than embalming the bug — the
same order the house applies to every hardening smell: delete the
unnecessary path first.

## Alternatives rejected

- **Keep the wipe behind a flag or header** for compatibility. The
  compatible behavior is the footgun; a compat flag would embalm it.
  Clients that legitimately want the bulk wipe have `delete-all`.
- **Add an `inbox` scope to the body.** Duplicates
  `DELETE /api/v1/inboxes/{inbox}/messages`, widens the destructive
  surface, and mints 100%-coverage-gated branches for a capability the
  native API already owns.
- **Treat `{"ids": []}` as `{"removed": 0}`.** Tempting (it is a valid
  selector naming the empty set), rejected: it is a second success shape
  whose only client is a buggy one, and a silent no-op on a destructive
  endpoint is the #6 failure shape wearing a 200.
- **A dry-run/confirm flag or wipe token.** Ceremony around the
  destructive path instead of refusing it; every field is contract
  surface to document, test and defend.

## Consequences

- **Breaking** for clients that relied on garbage = wipe. Migration is
  one line: send explicit ids, or use `DELETE /api/v1/delete-all`. Mailpit
  compatibility narrows by one behavior — the compatible behavior was the
  hazard.
- A client that computes an empty batch must skip the call (or read the
  400 as "nothing to delete"); the empty batch is deliberately not
  accommodated with a silent success.
- `tests/compat.rs::shim_deletes_go_through_the_store` is rewritten to
  endorse this contract: the 400s each followed by a before/after exact
  count over **both** seeded inboxes (proving nothing was destroyed, not
  just that the status changed), the selective delete through the real
  store, and the `delete-all` alias pinned as the sole wipe.
- The shim's docs (README API table, `openapi.json`, `llms.txt`) change
  in the same commit as the wire; this decision is the authority for why.
- Frozen guarantees: 0001/0002 are untouched and 0002 is *protected* —
  deletion stays inside the store so removals wake `await` watchers; 0003
  is untouched. Nothing here weakens a guarantee; the change strengthens
  0004's sentence from "mostly true" to true.
