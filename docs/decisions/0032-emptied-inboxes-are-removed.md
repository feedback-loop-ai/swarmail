# 0032 — An emptied inbox is gone: one rule across delete, clear and restore

- **Status:** accepted (operator, 2026-10-16 — "accept all")
- **Date:** 2026-10-16
- **Files:** `src/store.rs` (the `delete` apply arm), `src/pop3.rs`
  (`inbox_exists` follows the listing automatically), `tests/persistence.rs`,
  `tests/pop3.rs`. `src/persist.rs` needs **no** change: restore already
  rebuilds entries only for inboxes with rows.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) (the removal
  is the existing synchronous apply arm; no await enters insert),
  [0002](0002-awaits-subscribe-before-checking.md) (the waiter's
  per-iteration re-subscribe on `Closed` is miss-free — the release
  moment does not move),
  [0021](0021-watcher-channel-lifetime.md) (the watcher channel is
  released with its inbox's mail — same condition, same moment, unchanged;
  this decision changes only the *entry's* lifetime),
  [0003](0003-oldest-only-pruning.md) (untouched), 0004 (a maildrop is
  an inbox; the first mail still creates one).

## §1 The rule

Today three paths disagree about an inbox whose last mail was deleted
individually: `delete` leaves an empty `VecDeque` (the inbox stays
listed with count 0 and POP3 `USER` accepts it), `clear`'s drain removes
the entry, and a restart drops it (restore rebuilds entries only for
inboxes with rows). Live and restored state differ.

> **An inbox with no mail does not exist.** `delete`, `clear`, `clear_all`
> and restore follow one rule: removing an inbox's last mail removes the
> inbox — from the listing, from the REST/UI surfaces, and from POP3's
> maildrop existence. `USER` on a vanished inbox answers exactly what
> `USER` on a never-seen inbox answers.

Removal is chosen over persistence. An empty inbox carries no mail; its
only observables are a listing row and a POP3 answer, and keeping
emptiness across a restart means a tombstone table, a `meta` row, or a
restore-side special case — a whole persistence vocabulary to preserve
an empty container. The asymmetry the issue reports is *deleted*, not
moved.

## §2 Why this is a public-contract change worth a decision

The observable answers flip: a listing that showed the emptied inbox
with count 0 stops listing it, and POP3 `USER <that-inbox>` stops
answering as if the maildrop existed. The issue itself offers both doors
(removed like `clear`, or kept across restart); this decision records
*which* door and why, so the flip is owned rather than discovered. The
operator accepts it with the batch or refuses it and the batch keeps
today's delete-side behavior — but then `clear` and restore are the
paths that must change instead, because one rule is the point.

## §3 Why the watcher machinery is already correct for it

The `delete` apply arm already releases the watcher channel at the exact
moment the inbox's mail is gone (`list.is_empty()`,
[0021](0021-watcher-channel-lifetime.md)); this decision moves the
container removal to that same condition and moment (the `DashMap` guard
is dropped before the `remove`). A waiter parked on the channel answers
`Closed`, re-subscribes, re-scans, and reports the truth — miss-free by
0002's per-iteration discipline. The issue-#14 `watcher_count` probe
(baseline restored after the emptying) is exactly this rule's probe, and
it extends to the reopened store. **Cost accepted:** an agent that
authenticates POP3 between emptying and its next delivery sees the
maildrop close and reopen; that is the honest sequence, and the first
new mail recreates the inbox as 0004 always did.

## §4 What this decision does not do

- It does not persist empty inboxes the other way — no tombstone table,
  no `meta` row, no on-disk format change, no migration matrix.
- It does not change eviction (0003): nothing here removes a mail; it
  removes the empty container a mail is not in.
- It does not touch `await` semantics (0002) or the wake ring (0021):
  the release condition and moment are today's; only the entry's
  lifetime converges with them.
- It does not alter any refusal sentence; `USER`'s failure text for a
  vanished inbox is whatever the never-seen arm already says.
