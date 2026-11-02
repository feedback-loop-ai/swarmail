# One sending identity, many agents: recipient routing

[← README](../../README.md) · [Guide index](README.md)

The AUTH username only isolates parties that can each authenticate under
their own name. An application configured with a single SMTP login fills one
inbox however many agents drive it. Start the server with a route domain and
the recipient chooses the inbox instead (proposed
[0112](../decisions/0112-recipient-inbox-routing.md)):

```
swarmail serve --route-domain swarmail.test      # or SWARMAIL_ROUTE_DOMAIN
```

An envelope recipient `<local>@<label>.swarmail.test` is stored in inbox
`<label>`, lowercased, whatever the AUTH username is. The precedence is the
routed recipient, then the AUTH username, then `default`. Only `RCPT TO` is
read — no header chooses an inbox — and the local part is free, so one inbox
may own many addresses. A recipient under the domain that names no inbox
(`agent@swarmail.test`, `a@x.y.swarmail.test`, a bad character or a label over
63 characters) is refused `550 5.1.3` rather than silently merged into the AUTH
inbox. One transaction delivers to one routed inbox: a `RCPT` naming a second
one is answered `452 4.5.3`, the transaction stays open, and a sender that
retries the deferred recipient in its own transaction reaches both inboxes.
Recipients outside the domain have no vote and are recorded as before. A server
started without the flag behaves exactly as above.

An agent gets its private inbox from **`swarmail_create_inbox`** (no
arguments): it returns `{"inbox": "<32 hex>", "address": "agent@<inbox>.<route-domain>"}`.
Point the application's mail at that address, then wait on or search that
inbox by name; mail for other addresses never appears there. Minting allocates
nothing — an inbox exists once its first mail arrives, and a wait on the minted
name before then works. On a server without `--route-domain` the tool answers an
`isError` result naming the flag. This is addressing, not authentication: every
read surface still accepts any inbox name (loopback posture, decision 0004).
