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

## The HTML preview and remote content

The reader's **Preview** tab, and the HTML section of the standalone page
`/ui/message/{id}`, frame a separate document,
`GET /ui/message/{id}/preview` (decision 0135). The mail's HTML is
never written into the console page itself. The frame keeps a bare `sandbox`
attribute, and the document carries its own `Content-Security-Policy`
response header with a token-free `sandbox` directive. Scripts, forms,
plugins, nested frames, popups and top navigation stay off, even when the
URL is opened directly. The console's own policy gains only
`frame-src 'self'`, so that it can load this one frame.

**Inline images.** A `cid:` reference in an `<img src>` is shown when it
names an inline (or disposition-less) PNG, JPEG, GIF or WebP part whose bytes
were kept at ingest. The preview checks that part's type, size and SHA-256
against the stored attachment entry and inlines the stored bytes as a `data:`
URI. These references stay broken:

- SVG and every other type;
- parts sent as attachments;
- missing Content-IDs, and Content-IDs that two parts share;
- references from CSS or `srcset`;
- parts whose bytes were not stored or do not match;
- every reference in a mail whose source is over 8 MiB.

A note under the frame says so. The limits are 256 KiB per image and 2 MiB of
inserted `data:` URIs per preview, charged per reference. HTML over 2 MiB, or
a preview over 4 MiB, answers `413`. At most two previews render at once; a
third request answers `503 preview busy; retry`.

**Remote content is off by default.** Remote images, stylesheets and fonts are
blocked, as in a mail client, so opening a mail does not fire its tracking
pixels; the Deliverability tab still lists them. **Load remote content**
reloads only the open mail's preview with `?remote=1`, which allows images,
stylesheets and fonts over `http:` and `https:` and nothing else. The button's
label warns that loading contacts the sender's hosts. Your browser makes
those requests, not the server: the hosts see your IP address and may receive
cookies or credentials your browser holds for them. They can also redirect,
and they can be local or private addresses. Mixed-content blocking and font
CORS may still refuse some loads. The preview itself sends no `Referer`
(`Referrer-Policy: no-referrer`), so no request names Swarmail, the console or
the mail. A font or image that a remote stylesheet requests is the stylesheet's
own request, though: browsers give it the stylesheet's URL or origin as its
`Referer` and apply the stylesheet's own referrer policy, which the preview
cannot reach. That referrer only names the remote host back to itself. The
choice lasts for that selection only:
picking another mail, or coming back later, starts with remote content off
again, and nothing is stored.

The preview also removes `<base>`, every `<meta>`, and every `<link>` that is
not an ordinary stylesheet (so no refresh, prefetch or preconnect). It drops
`href` from anchors and areas, and `ping`, `referrerpolicy` and
`attributeName` from every element. The response headers, not this
transformation, are what keep scripts and forms off.

