# Network posture, metrics and bounded pages

[← README](../../README.md) · [Guide index](README.md)

**The binds are loopback by default, and the loopback plane refuses
cross-origin pages and rebound hosts.** All three listeners default to
`127.0.0.1` — a mail mock serves the machine it runs on — and the image
and the chart state `0.0.0.0` explicitly, because a container that
trusted its defaults would never publish a port. While the HTTP listener
is loopback-bound, two headers are checked before the router. **Host**:
a request whose `Host` is not a loopback spelling of the bound HTTP port
is answered `403` — this is the DNS-rebinding defense, because a rebound
page sends *same-origin* requests (and same-origin GET/HEADs carry no
`Origin` at all) while its `Host` line still spells the attacker's
hostname. **Origin**: a present `Origin` that is not a loopback spelling
is answered `403` — the one header a browser sets honestly, so a
cross-site drive is named and refused. Every method and every route is
covered, `/mcp` and the UI included; the opt-in MailSlurper listener
(`--mailslurper-listen`, off by default) wears the same guard keyed to its
own port; requests without an `Origin` — curl,
agents, the stdio bridge, every test harness — pass untouched (their
`Host` is the loopback spelling they dialed). A `Host` without a port
means `80`, the http default, so a port-80 loopback bind admits
`Host: localhost`; a portless loopback `Origin` is still refused, so a
browser write to a port-80 loopback bind must spell `:80` in its origin
(a limitation decision 0043 §1 keeps on purpose).

**An off-loopback bind trusts the network, not the browser.** An operator
who binds `0.0.0.0` (the image and the chart do) states the trust
themselves: the `Host` allowlist is inactive, because there is no loopback
plane to rebind to. The `Origin` leg stays active (decision 0043): a
present `Origin` must name the request's own `Host` authority — scheme
`http` or `https`, the same host (case-insensitive) and the same port,
where an omitted port on either side means the Origin scheme's default
(80 or 443) — or the request is answered `403` (`cross-origin requests
must match the Host authority`) before any handler runs. So a page at
`https://evil.example` can no longer POST `text/plain` to `/mcp` and arm
chaos, while curl, MCP SDK clients and the stdio bridge, which send no
`Origin`, are untouched. Behind a TLS-terminating reverse proxy, forward
the browser's original `Host` authority unchanged, including a
nondefault external port: `https://mail.example` with `Host:
mail.example` (or `mail.example:443`) passes, a proxy that rewrites
`Host` to the backend's own address, or strips an external `:8443`,
makes every browser request with an `Origin` refuse. `Forwarded` and
`X-Forwarded-*` are never consulted. What an off-loopback bind still
trusts: any network peer can reach it, a direct client can omit or forge
`Origin`, and DNS rebinding is not prevented without the loopback `Host`
allowlist — this is a header check against cross-site browser pages, not
authentication.

**One opt-in outbound path: the link reachability probe.** Swarmail makes no
outbound connection for mail content by default (webhooks are the other
operator-configured sender). Proposed decision 0121 adds one more, and it needs two
explicit steps: the operator starts the server with `--link-reachability
public|loopback` (`SWARMAIL_LINK_REACHABILITY`, default `off`), and a caller
asks with `GET /api/v1/messages/{id}/deliverability?reachability=true`. SMTP,
the message page, MCP, `assert` and `await` never probe. `public` contacts
only addresses the default-deny policy proves globally routable; `loopback` also contacts `127.0.0.0/8` and
`::1`, which means **mail content can cause GET requests to local ports** of
the host (headers only, no body read, no credentials, fixed User-Agent). Use it
on a development machine, not on a host that runs sensitive local services
you would not expose to a mail's author. The address policy is
**default-deny** (basis: the IANA special-purpose registries, read 2026-11-04):
IPv6 is contacted only inside `2000::/3`, minus `2001::/23`, `2001:db8::/32`,
`2002::/16` and `3fff::/20`, so site-local `fec0::/10`, NAT64, 6to4, Teredo and
every other address outside `2000::/3` are refused; IPv4 is contacted only
outside the registry's non-global blocks (private, link-local including the
cloud metadata address 169.254.169.254, CGNAT, multicast, reserved and the
others listed in the guide). An address that embeds IPv4 is refused, never
decoded. The check holds on every redirect hop, and the server pins each
connection to the address it validated. Residual: a block IANA later allocates
inside `2000::/3` is contacted until the table is edited. Name resolution uses
the OS resolver on tokio's blocking pool, which cannot be cancelled, so a
dropped probe's lookup runs on until the OS gives up; the per-`Service` caps
bound it and it costs latency, never a `250`. The bounds are in
[Deliverability reports](deliverability.md#opt-in-reachability).

**Forwarded ports refuse on a loopback bind.** `ssh -L 9000:localhost:8025`,
VS Code and devcontainer port forwarding and GitHub Codespaces all
deliver the browser's `Host: localhost:9000` (or a forwarding hostname)
to a listener bound to `127.0.0.1:8025`; the port (or host) is not the
bound one, so the loopback guard answers `403`. The remedy is a choice
the operator owns (decision 0030): bind off loopback, for example
`SWARMAIL_HTTP_LISTEN=0.0.0.0:8025`, which exposes the plane to the
network and keeps the Origin-to-Host check above active. There is no
port-forward bypass and no proxy allowlist.

**`/metrics` counts its own scrape.** `swarmail_http_requests_in_flight`
counts every request inside the router, refusals and the scrape itself
included, so a quiet server reads `1`, one parked request plus the scrape
reads `2` — subtract the observer for `0`/`1` (decision 0043 §5). The
per-inbox readiness rows (`{inbox="…"}`) escape label values exactly as
the Prometheus text format defines — `\\`, `\"` and `\n` — and pass every
other character, tab and Unicode included, literally.

Inbox-scoped HTTP request rows use the same selected, once-decoded inbox
identity as the await rows ([decision 0065](../decisions/0065-decoded-inbox-metrics.md)).
For example, requests to `/api/v1/inboxes/name%20with%20spaces/await` use
`inbox="name with spaces"` in all three families; encoded aliases coalesce
under that selector. The segment is selected before decoding, so `%2F` is
label data. Strict UTF-8 decoding falls back to the complete original segment
for invalid bytes. A refused request can therefore have request activity
without an await registration or stored mail, and `%FF` can share a metric
label with a valid literal-percent `%25FF` name.

`swarmail_awaits_registered_total` keeps one row per awaited inbox name for
the first 4,096 names. A name first seen after that has no row of its own:
its waits are counted, exactly, in the overflow sample
`swarmail_awaits_registered_total{inbox="\n"}` — a label of one line feed,
which no inbox name can be — while names tracked before the cap keep
counting in their own rows. The cap limits attribution, never which waits
are admitted, and the unlabeled total is the sum of all labeled rows in the
same scrape. Dashboards that selected the old overflow label
`inbox="_uncounted"` must move to `inbox="\n"`; `_uncounted` is now an
ordinary inbox with its own row (decision
[0047](../decisions/0047-storage-rpc.md), issue #130).

There is no auth on any surface, by design: the loopback
posture is the boundary, and a server with secrets would need auth —
this one holds test mail, and its knobs are local (`set-chaos`
included). Webhooks dial out only to addresses an operator explicitly
configures — directly: environment proxies (`HTTP_PROXY`, `HTTPS_PROXY`,
`ALL_PROXY`) are ignored and redirects are never followed (decision
0041 §2) — **and that is the whole boundary**: an arbitrary or
internal address is accepted at registration and dialed from this
process, so on a network with sensitive internal services the operator
should spell loopback or otherwise-sanctioned targets (no allowlist, no
auth on the registration API — the posture is the trust statement, and
the SSRF exposure of an internal-target registration is recorded in
decision 0030 §5 rather than papered over). The delivery budget is
`WEBHOOK_DELIVERY_CAP` (32) **per normalized target URL** — many
configured targets multiply the ceiling, and there is no global
backstop across targets; nothing ever dials an address learned from
mail content.

**The pages are bounded.** Every result page — the REST list, `await`,
the MailHog/Mailpit shims, and every MCP search/get-latest/wait — is
capped at **250 items** (`limit`/`count` above the cap is refused with a
400 on REST and the shims, a `-32602` error over MCP: named, never
clamped). `await`'s exact-count `assert` keeps its unclamped semantics —
its `limit` is parsed for parity with `list` but inert, because the
assert answer is a count, not a page. The inbox listing
(`/api/v1/inboxes` and the UI index) pages too: `limit` (default 100, at
most 250, `0` an empty page) and an uncapped `offset`. SMTP request parsing
runs off the async workers on the blocking pool, and so does the whole of
every compat list and search build — scan, filter, page, snippet render and
JSON serialization (decision
[0044](../decisions/0044-compat-validation.md)) — and every
single-message response: Mailpit's `/message/{id}`, `/plain` (both
spellings), `/raw`, `/headers`, the three `latest` routes, MailHog's
`/download` and the native `GET /api/v1/messages/{id}[/raw]` are projected,
rendered and serialized to finished bytes on the pool (decision
[0049](../decisions/0049-extraction-compat.md)) — so a 50 MiB mail read
through those routes cannot stall the accept loop or the await wakes it
gates. The MCP `swarmail_seed_email` seed build, the REST list/await pages,
the UI's message page and the SSE thread-view serialization still run
inline on the async workers: a 50 MiB mail read through them *can* delay
an SMTP greeting, and that residual is named here rather than hidden. The
pool is shared with ingest, so offloading is not an admission bound
either. A Mailpit snippet reads at
most an 8192-byte prefix of its body; a MailHog v2 page refuses whole once
its JSON would pass 64 MiB (`compat page exceeds byte budget 67108864;
reduce limit or fetch raw by id`). These bound responses, not memory: the
stored mail, a full-scope search's candidates and concurrent requests still
cost what they cost.
