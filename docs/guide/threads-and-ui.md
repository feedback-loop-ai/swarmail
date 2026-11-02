# Threads and the live UI

[← README](../../README.md) · [Guide index](README.md)

`GET /api/v1/inboxes/{inbox}/threads` groups the inbox into conversations —
References/In-Reply-To/Message-ID chains when they resolve, the normalized
subject (leading reply/forward markers stripped — `Re`, `Fw`, `Fwd`, `Aw`,
`SV`, `VS`, `Antw`, `WG`, `TR`, `RV`, `Odp`, `R`, `I`, `ENC`, `RES`,
`Ответ` and `回复`, in any case, with an ASCII `:` or a full-width `：` and
whitespace before it tolerated; a numbered `Re[2]:` is *not* a marker and
stays in the fallback key — then every whitespace run collapsed to one
space and the remainder Unicode-lowercased) as the fallback for the *chain-less* mails
only: a reply threads by its References even when the parent id was
never published, because absence does not authorize a subject merge.
Unicode lowercasing is not full case folding: `ß` and `ss` remain distinct
in subject fallback keys. Each conversation carries a
stable `key`: the hash of the conversation's *first* mail's anchor (its
`Message-ID`, else its `In-Reply-To`/first `References` entry, else the
normalized subject, else its own id). A reply — even one whose message-id
sorts before the root's — can never move the URL a client was already
given.

Mail stored before this derivation is re-keyed on the next request —
threads are recomputed per request, so there is no data migration and
nothing to run; a key also moves when its conversation's oldest mail
leaves the newest-200 view window, is deleted or cap-pruned, when two
mails share a receive millisecond and the id tie-break re-orders them, or
when a bridge mail merges two chains — the merged chains retire their old
keys, which answer a loud 404; re-list to find the surviving anchor's key.
`…/feed` is the SSE stream of the full thread view: one
snapshot, then one event per accepted mail, driven by the same per-inbox
watcher that powers `await`. The human UI at `/ui/inbox/{inbox}` and
`/ui/inbox/{inbox}/thread/{key}` is a thin shell over these endpoints and
updates live — no manual refresh, zero frontend dependencies.
