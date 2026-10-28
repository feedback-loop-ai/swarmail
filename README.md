<p align="center"><img src="assets/logo.svg" width="132" alt="Swarmail — an open envelope under the feedback-loop pulse: two resting dots and one live one"></p>

# Swarmail

**Most mail mocks are built for a human looking at a web UI. Swarmail is built for a program waiting on mail.**

[![CI](https://github.com/feedback-loop-ai/swarmail/actions/workflows/ci.yml/badge.svg)](https://github.com/feedback-loop-ai/swarmail/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/feedback-loop-ai/swarmail?label=release&color=blue)](https://github.com/feedback-loop-ai/swarmail/releases/latest)
[![license: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![coverage · literal 100%](https://img.shields.io/badge/coverage%20%C2%B7%20literal%20100%25-brightgreen)](scripts/coverage-gate.sh)
[![deps · permissive-only](https://img.shields.io/badge/deps%20%C2%B7%20permissive--only-brightgreen)](deny.toml)
[![platforms](https://img.shields.io/badge/linux%20%C2%B7%20macos-blue)](.github/workflows/ci.yml)
[![rust](https://img.shields.io/badge/rust-stable-orange)](Cargo.toml)

**Email infrastructure for tests and agents.** One Rust binary: a lossless
SMTP mock with per-test inboxes, chaos engineering, push-based waiting,
native MCP for AI agents, and a live web UI — swallowing mail faster than
your tests can produce it.

*The swarm never drops a letter.*

> **Accept is the truth.** An SMTP `250` means the mail is queryable — and,
> with a data file, already on disk. There is no async gap where mail can
> vanish, because there is no async gap.

[The landing page](docs/index.html) · machine-readable surfaces:
[`/openapi.json`](http://localhost:8025/openapi.json) ·
[`/llms.txt`](http://localhost:8025/llms.txt) · `/metrics` · `/healthz`

## Contents

- [The name](#the-name)
- [What it does](#what-it-does)
- [The guarantees](#the-guarantees)
- [60-second quickstart](#60-second-quickstart)
- [Kubernetes (Helm)](#kubernetes-helm)
- [The surfaces](#the-surfaces)
  - [SMTP — accept is the truth](#smtp--accept-is-the-truth)
  - [The REST API — await, don't poll](#the-rest-api--await-dont-poll)
  - [MCP — agents drive the mail](#mcp--agents-drive-the-mail)
  - [POP3 — the same store, RFC 1939](#pop3--the-same-store-rfc-1939)
  - [MailHog / Mailpit shims](#mailhog--mailpit-shims)
  - [Threads and the live UI](#threads-and-the-live-ui)
  - [Webhooks](#webhooks)
  - [Chaos — break SMTP on purpose](#chaos--break-smtp-on-purpose)
  - [Persistence — the restart copy](#persistence--the-restart-copy)
- [Benchmarks](#benchmarks)
- [Repo layout](#repo-layout)
- [The decision culture](#the-decision-culture)
- [Testing & coverage](#testing--coverage)
- [Roadmap](#roadmap)
- [License](#license)

## The name

A **swarm** is a thousand independent agents with no coordinator and no
single point of loss. That is exactly what a test fleet looks like: fifty
parallel CI jobs, three AI agents, one flaky retry loop — every one of them
convinced it is the only mail user in the world. Swarmail's whole contract
is that each of them gets its own inbox (the AUTH username *is* the inbox)
and that the swarm, as a whole, never drops a letter.

The name is also the promise: mail *in* swarm quantities — 5000 mails over
100 concurrent fresh sessions, exact — and mail *out* to whoever waits for
it: a test's `await`, an agent's `swarmail_wait_for_email`, a webhook sink,
or a human watching the inbox update live.

**The binary is `swarmail`.** No services, no database to babysit, no Node
runtime — a static binary and a scratch container about its size.

## What it does

You point any SMTP client at it — Ory Kratos's courier, Nodemailer, a bare
`smtplib` — and mail lands in a queryable store with everything the
assertion will need already extracted: the links, the OTP-like codes, the
thread headers. Tests wait with `await` (push-based, subscribes before it
checks); agents drive the same store over MCP; existing MailHog/Mailpit
clients work through the shims; a human watches the inbox update itself.

```
docker run --rm --stop-timeout 30 -p 1025:1025 -p 1110:1110 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:main
swarmail serve                 # or: cargo run -- serve — same ports on localhost
swarmail mcp                   # the stdio MCP bridge
swarmail gen-cert --domain localhost --out ./certs
```

Thirteen MCP tools, one REST API, two wire protocols in (SMTP, POP3), one
compatibility layer (MailHog/Mailpit), one observability story
(`/metrics`, `/healthz`, `/openapi.json`, `/llms.txt`) — and a web UI with
zero frontend dependencies.

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
covered, `/mcp` and the UI included; requests without an `Origin` — curl,
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
identity as the await rows ([decision 0065](docs/decisions/0065-decoded-inbox-metrics.md)).
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
[0047](docs/decisions/0047-storage-rpc.md), issue #130).

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
[0044](docs/decisions/0044-compat-validation.md)) — and every
single-message response: Mailpit's `/message/{id}`, `/plain` (both
spellings), `/raw`, `/headers`, the three `latest` routes, MailHog's
`/download` and the native `GET /api/v1/messages/{id}[/raw]` are projected,
rendered and serialized to finished bytes on the pool (decision
[0049](docs/decisions/0049-extraction-compat.md)) — so a 50 MiB mail read
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

## The guarantees

These are the frozen guarantees ([decision records](docs/decisions/README.md)
bind them; review refuses a change that weakens one, no matter how green
the tests are):

1. **Accept == stored** ([0001](docs/decisions/0001-synchronous-insert-lossless.md)).
   `Store::insert` is synchronous; an SMTP `250` means the mail is queryable,
   and with `--data-file` it is committed with fsync-per-accept before the
   reply leaves.
2. **`await` subscribes before it checks** ([0002](docs/decisions/0002-awaits-subscribe-before-checking.md)).
   Nothing is missed while connecting; no sleep-polling exists anywhere in
   the product.
3. **Pruning is oldest-only, never mid-delivery**
   ([0003](docs/decisions/0003-oldest-only-pruning.md)). The per-inbox cap
   (default 100k, `0` = unlimited) evicts the oldest mail only.
4. **The AUTH username is the inbox**
   ([0004](docs/decisions/0004-per-test-inboxes-via-smtp-auth.md)). Free
   parallel isolation for every test and agent; no config to route it.
5. **Extraction happens at ingest, not query time**
   ([0005](docs/decisions/0005-extraction-at-ingest.md)). Links, codes and
   thread headers are parsed once, when the mail is accepted; queries
   re-parse nothing.

## 60-second quickstart

One binary, three ports, no config. **Docker:**

```
docker run --rm --stop-timeout 30 -p 1025:1025 -p 1110:1110 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:main
```

`--rm` takes the container's anonymous `/data` volume with it: the image
declares `VOLUME /data`, so a run without `--rm` leaves one volume behind
per container even after `docker rm -f`. `--stop-timeout 30` gives the
graceful drain room (see *Stopping* below).

`:main` is the continuous build of the main branch — mutable, so pin
`:sha-<commit>` (or build locally) when you need a reproducible image.
`:v0.1.0` was never published, and `:0.1.0` is the 2026-10-12 first
build; the next release cut publishes both `:v<version>` and `:<version>`.

**Persistence in Docker:** set `SWARMAIL_DATA_FILE` and mount a named
volume at `/data`. The image ships `/data` owned by its runtime uid 65532,
so a fresh named volume is writable (`:main` carries it; the 2026-10-12
`:0.1.0` build predates it):

```
docker run --rm --stop-timeout 30 -p 1025:1025 -p 1110:1110 -p 8025:8025 \
  -v swarmail-data:/data -e SWARMAIL_DATA_FILE=/data/swarmail.db \
  ghcr.io/feedback-loop-ai/swarmail:main
```

The named volume outlives the container — that is the point — so remove
it yourself when the data is no longer wanted: `docker volume rm
swarmail-data`. A volume an older image created is owned by root;
`chown 65532:65532` its contents once, or start from a fresh volume.

**From source:** `cargo run -- serve` (Rust stable; everything else is
vendored), or `cargo install --locked --path .` from a checkout — the
crate is not published on crates.io.

**Then loop.** This transcript is real — a stock `smtplib` delivering, the
REST surface answering:

```
$ curl -X POST localhost:8025/api/v1/inboxes/quickstart/seed \
    -H 'content-type: application/json' \
    -d '{"from":"noreply@x.io","to":"ada@x.io","subject":"Verify your account",
         "text":"Welcome! Verify at https://x.io/verify?token=abc123 — code 424242 is your fallback."}'
{"id":"01a1261e-…","inbox":"quickstart", …,
 "links":["https://x.io/verify?token=abc123"], "codes":["424242"]}      # extraction ran

$ python3 - <<'EOF'                                    # a real client, really delivered
import smtplib
s = smtplib.SMTP('localhost', 1025, timeout=10)
s.sendmail('kratos@x.io', ['ada@x.io'],
           "From: kratos@x.io\r\nTo: ada@x.io\r\nSubject: Reset your password\r\n\r\n"
           "Reset: https://x.io/reset?token=xyz789 or use 998877.")
s.quit()
EOF

$ curl "localhost:8025/api/v1/inboxes/quickstart/await?to=ada@x.io&count=1&timeout_ms=5000"
{
  "matched": 1,
  "total": 1,
  "emails": [ { "subject": "Reset your password",
                "links": ["https://x.io/reset?token=xyz789"],
                "codes": ["998877"] } ]                 # no regex over HTML — fields
}
```

`await` blocks until the mail arrives (or the timeout answers `matched: 0`)
— push-based, no polling. Fixtures without SMTP: `…/seed` runs the full
parse + extraction pipeline. Per-test isolation needs no config: the AUTH
username names the inbox.

**One SMTP connection serves one inbox.** A session's first successful
AUTH fixes its inbox; a second AUTH on the same connection answers `503`
and changes nothing (decisions 0028 and 0034). Python's `smtplib` treats that `503`
as "already authenticated" and does not raise, so `s.login('a', 'x')`
followed by `s.login('b', 'x')` on one `SMTP` object files *both* tests'
mail under `a`. Open a fresh connection per inbox (a new `smtplib.SMTP`
per test); only a successful STARTTLS resets the identity on an open
connection.

Environment knobs: `SWARMAIL_SMTP_LISTEN` (default `127.0.0.1:1025`),
`SWARMAIL_POP3_LISTEN` (`127.0.0.1:1110`), `SWARMAIL_HTTP_LISTEN`
(`127.0.0.1:8025` — the image and the chart set `0.0.0.0`, see the
posture above), `SWARMAIL_MAX_PER_INBOX` (`100000`, `0` = unlimited),
`SWARMAIL_URL` (the MCP stdio bridge's upstream, default
`http://127.0.0.1:8025`),
`SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY`,
`SWARMAIL_TLS_HANDSHAKE_TIMEOUT_MS` (`10000`), `SWARMAIL_DATA_FILE`.
Logging: `RUST_LOG` (env-filter syntax; default
`swarmail=info,tower_http=off`). `serve` and `gen-cert` log to stdout —
`gen-cert` also prints its `wrote <cert> and <key>` line there — and
write a refusal (`swarmail: <reason>`, exit 1) to stderr. Only `mcp`
logs to stderr, because its stdout is the JSON-RPC protocol and nothing
else (decision 0029 §4).

**Stopping:** `SIGTERM` (Docker's default stop signal, and the image's
declared `STOPSIGNAL`) or `SIGINT` starts the graceful drain, which may
take up to its **10 s** deadline (decision 0027 §7). A bare `docker stop`
also waits only 10 s before `SIGKILL`, so its grace meets the drain only
at the knife's edge (0027 §8): a kill that lands there turns the drain's
tail into crash recovery — every mail already answered `250` is
committed, anything not yet answered is refused, so the loss is grace,
never correctness. Give it room: run with `--stop-timeout 30` as above,
or stop with `docker stop -t 30`. Kubernetes' default 30 s grace already
clears the drain. A second signal during the drain exits 70 at once.

## Kubernetes (Helm)

A shared namespace mailbox, an ephemeral per-run instance, an agent-sidecar —
the chart packages the same binary with no configuration language of its own
([decision 0008](docs/decisions/0008-kubernetes-deployment-contract.md),
*accepted*):

```
git clone https://github.com/feedback-loop-ai/swarmail && cd swarmail
helm install swarmail ./charts/swarmail --set image.tag=main --set image.pullPolicy=Always \
  --namespace swarmail --create-namespace
helm test swarmail -n swarmail        # the /healthz hook, from a curl pod
```

Install from a checkout for now. The only published chart, OCI `0.1.0`,
predates the startupProbe, the TLS settings, `networkPolicy.ingressPeers`
and the listen configuration, and the publish guard never republishes a
version (decision 0019). The tree's chart is `0.1.2`, prepared but not
published (`0.1.1` was prepared and never published either). Its
`appVersion` image, `:0.1.0`, is the 2026-10-12 build that predates the
current server, so the command sets `image.tag=main` explicitly — and
`image.pullPolicy=Always`, because `:main` moves: under the chart's default
`IfNotPresent` a node that cached an older `:main` would keep running it.

**Releases publish the image before the chart.** A `v*` tag publishes only
when it is exactly `v<Cargo.toml version>` for the image and exactly
`v<Chart.yaml version>` for the chart, and the chart's `appVersion` must
equal Cargo's version; the chart job needs the image job's success and
checks that the image it names is published. So a tag that matches only
the chart — `v0.1.2` on today's tree, whose crate is `0.1.0` — publishes
neither. Preparing a chart version is not a release: CI requires a chart
version bump for every chart edit (`scripts/chart-verify.sh`), and the
release itself is the operator's aligned tag. The two pushes are not one
transaction: if the chart push fails after the image pushed, the image
stays published and a rerun refuses it as a republish (decision 0019) —
recovery is a deliberate registry act by the operator.

The chart adds nothing the binary does not read — configuration is exactly
the env contract, passed through `config: {SWARMAIL_MAX_PER_INBOX: "0"}`
or `envExtra`. One Service carries all three ports, so one DNS name serves
every client; the optional Ingress exposes only the HTTP surface (SMTP and
POP3 are plain TCP). The contract worth knowing before you scale it — you
cannot: the chart refuses a `replicaCount` above 1 at render time, naming
the contract ([0008 §4](docs/decisions/0008-kubernetes-deployment-contract.md)):

| Value (default) | What it changes |
|---|---|
| `replicaCount` (`1`) | must stay 1 — a second pod would fork the mailboxes or mount the same RWO claim twice; anything above 1 fails `helm template`. The strategy is **Recreate at every setting** ([0008 §5](docs/decisions/0008-kubernetes-deployment-contract.md)): a rolling surge would run two pods behind one Service, and the store is single-replica by design ([0001](docs/decisions/0001-synchronous-insert-lossless.md)) |
| `image.repository` / `image.tag` / `image.pullPolicy` (`ghcr.io/feedback-loop-ai/swarmail` / `""` = the chart's `appVersion` / `IfNotPresent`) | the image; set `pullPolicy: Always` with a moving tag such as `main` |
| `config` (the three binds, `0.0.0.0:1025` / `0.0.0.0:1110` / `0.0.0.0:8025`) | `SWARMAIL_*` env entries, rendered as given — the binds are stated because the binary's own defaults are loopback ([0030 §1](docs/decisions/0030-loopback-plane-bounded-results.md)); add any other knob, e.g. `SWARMAIL_MAX_PER_INBOX` (Helm merges it beside the binds) |
| `envExtra` (`[]`) | raw env entries passed through verbatim (`valueFrom` and friends) |
| `service.type` / `service.ports` (`ClusterIP`, `1025`/`1110`/`8025`) | the Service; the ports are the Service side, the container always listens on 1025/1110/8025 |
| `persistence.enabled` (`false`) | SQLite on a PVC mounted at `/data` (`SWARMAIL_DATA_FILE=/data/swarmail.db`); `existingClaim` (`""`), `storageClass` (`""`), `accessModes` (`[ReadWriteOnce]`), `size` (`1Gi`) |
| `startupProbe.periodSeconds` / `.timeoutSeconds` / `.failureThreshold` (`10` / `2` / `90`) | the restore window before the kubelet restarts the pod — period × threshold, 15 minutes by default, because the data file restores fully before the first bind ([0026 §3](docs/decisions/0026-deployment-boundary-peers-tls-sizing.md)) |
| `probes.initialDelaySeconds` / `.periodSeconds` / `.timeoutSeconds` / `.failureThreshold` (`2` / `10` / `2` / `3`) | the liveness and readiness probes on `/healthz` |
| `tls.enabled` / `tls.secretName` (`false` / `""`) | mounts an operator-supplied Secret's `tls.crt`/`tls.key` read-only at `/etc/swarmail/tls` and sets `SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY`, so SMTP offers STARTTLS; the chart mints nothing, and `enabled` without a `secretName` fails `helm template` by name ([0026 §2](docs/decisions/0026-deployment-boundary-peers-tls-sizing.md)) |
| `ingress.enabled` (`false`) | the UI/API/MCP surface on your ingress class (`className`, `host` `swarmail.local`, `tls`, `annotations`); SMTP/POP3 stay cluster-internal |
| `serviceMonitor.enabled` (`false`) | a prometheus-operator ServiceMonitor on `/metrics` (`interval` `30s`, `labels`) — **fails closed with a named refusal** if the CRDs are absent |
| `networkPolicy.enabled` / `networkPolicy.ingressPeers` (`false` / `[]`) | ingress on the three ports from the same namespace only; `ingressPeers` adds `from:` peers (`namespaceSelector`, `podSelector`, `ipBlock`) verbatim to that one rule ([0026 §1](docs/decisions/0026-deployment-boundary-peers-tls-sizing.md)); the policy is Ingress-only |
| `resources` (requests `50m` CPU / `32Mi`, limit `256Mi` memory) | size `limits.memory` from observed use: memory is not globally bounded (0038) |
| `tests.enabled` (`true`) | the `helm test` `/healthz` hook pod |
| `podAnnotations`, `podLabels`, `imagePullSecrets`, `nodeSelector`, `tolerations`, `affinity`, `nameOverride`, `fullnameOverride` | the usual scheduling and naming passthroughs (all empty by default) |

`helm template` output is valid plain manifests — Kustomize and raw-YAML
teams are first-class, not gated behind the chart. The chart is verified
offline by `scripts/chart-verify.sh` (lint + both profiles + contract
assertions), which the CI `chart-verify` job runs on every push and PR,
together with its chart-version bump check; publishing is the separate,
tag-only `chart-publish` job.

## The surfaces

### SMTP — accept is the truth

`:1025` — EHLO, AUTH PLAIN/LOGIN (accept-any; **username = inbox**),
PIPELINING, 8BITMIME, SMTPUTF8, SIZE, a 50 MiB cap. Every mail is inserted
synchronously on `DATA` completion — the `250` *is* the durability
contract, not an eventual promise. 8BITMIME is true: a DATA body is stored
as the exact octets it arrived as, and `/api/v1/messages/{id}/raw` serves
those same bytes back as `message/rfc822`. A message over the cap is
refused whole — `552 5.3.4 Message too big`, nothing stored, nothing
truncated — with the cap binding memory where the bytes arrive, not just
the accumulator; a protocol line over 4096 bytes — POP3's own line cap —
is drained and answered `500 5.5.2 Line too long`, and the session
continues.

**The session boundaries are exact**
([0034](docs/decisions/0034-data-terminator-boundary.md), *accepted*).
DATA ends only on CRLF `.` CRLF — a `.` CRLF line behind a bare-LF line is
payload, stored with its dot, and so is a `.` bare-LF line; a terminator
straight after the `354` is the empty message. An over-cap message drains
to that real boundary before its one `552`. A sender whose lines end in bare
LF only must still close with a CRLF-ended line before the dot, or the
session idles out with nothing stored. MAIL needs `FROM:<path>` and RCPT
needs `TO:<path>` — keyword case-insensitive, whitespace after the colon
allowed, angle brackets required; a bracketless, inverted or glued-on path
is `501`, `MAIL FROM:<>` is the null sender and `RCPT TO:<>` is refused.
A path that only looks like one is `501` too
([0046](docs/decisions/0046-smtp-pop3-edges.md), *accepted*): blank after
trimming whitespace (`< >`, `<\t>`, an ideographic space), or carrying any
control character — only the literal `<>` is null. Paths are otherwise
opaque and kept verbatim: local names (`<alice>`) and UTF-8 addresses are
kept, no `@` or DNS check is made. Two RFC 5321 forms are deliberately not
parsed: a quoted local part containing `>` (`<"a>b"@x.io>`) ends at its
first `>` and is refused, and a source route (`<@hop1,@hop2:bob@y.io>`) is
not stripped — the whole route is stored as the address. MAIL accepts `SIZE=<digits>` (a
declaration above the cap, however many digits, is `552` at MAIL; the real
bytes are still counted at DATA), `BODY=7BIT|8BITMIME` and bare `SMTPUTF8`;
a malformed or repeated one is `501`, any other MAIL parameter is `555`, and
RCPT takes no parameters (`555`). Refused MAIL and RCPT commands leave the
open transaction exactly as it was — they are checked before chaos rolls —
and HELO/EHLO reset the transaction like RSET (HELO answers one line). PLAIN
must be exactly `authzid NUL authcid NUL password`; only the authcid names
the inbox, the authzid is ignored and any password — empty included — is
accepted. The username (PLAIN's authcid, LOGIN's username) is the inbox
name verbatim, padding included, and a blank one or one carrying a control
character — a PLAIN-shaped `\0user\0password` sent to LOGIN included — is
`501 5.5.2` with nothing assigned. One AUTH succeeds per session (`503` after that, even as
`default`), surviving RSET and greetings; only a successful STARTTLS starts
afresh, clearing identity and envelope. A bare `AUTH` or surplus tokens is
`501`, an unknown mechanism `504`. Deliberately lenient: MAIL before any
greeting is accepted, and so are greetings with no hostname. A second AUTH
is the trap for clients that reuse one connection across inboxes — see
*One SMTP connection serves one inbox* in the quickstart.

**Commands are UTF-8; DATA is octets.** A command line — or an AUTH
continuation line — whose raw bytes are not valid UTF-8 ends the session
with **no reply**: the connection simply closes, and an open transaction is
dropped with it (a base64 AUTH value that *decodes* to invalid UTF-8 is a
`501`, as above). Non-ASCII mail content belongs inside DATA, which is
stored byte for byte; a non-ASCII address in MAIL/RCPT must be UTF-8.

**The listener never dies on a bad accept.** A transient `accept()` error
flags the listener degraded and backs the loop off (1 s); the flag clears
when the next accept is observed waiting or succeeds — an idle recovered
listener goes green without a peer, while back-to-back failures stay red
across the retry (the backoff clock alone never clears it). Waiting is an
observation, not a promise that a later accept succeeds. `/healthz` says
so: `200 {"status":"ok"}` when no listener is flagged, `503
{"status":"degraded","degraded":["smtp", ...]}` naming the flagged
surfaces, and `503 {"status":"stopping"}` for a request that reaches the
handler after the stop has fired — that state wins over everything else.
Sessions are bounded where the wire admits them: more than 256 concurrent
SMTP or POP3 sessions is refused before any greeting (`421 4.7.0 Too many
connections, closing transmission channel` / `-ERR too many connections`);
completed sessions are reaped right after each accept, before the cap is
checked, so a slot freed while the listener waited is offered to the very
next peer. More than 100 recipients on one mail is refused `452 4.5.3 Too
many recipients` before anything is appended, and an idle session is
answered `421 4.4.2 idle timeout, closing transmission channel` (POP3:
`-ERR idle timeout`) and closed — SMTP 5 min, POP3 10 min, each its own
compiled default, never armed across a commit (a mid-DATA expiry stores
nothing, exactly as if the peer had hung up). A session that fails — a reply
write that expires included — drops its connection at once, without a
graceful close that could wait on the same stalled peer; a session that ended
well gets one more idle window for its close (TLS `close_notify`), so a
STARTTLS peer that stops reading frees its slot at the deadline. Every
accepted SMTP and POP3 socket sets `TCP_NODELAY` before its first byte, so a
pipelined group's replies are not held for the client's delayed ACK.

**The idle bounds are per read and per write, not a session lifetime**
(decision 0036, accepted). One incomplete read or write is cut off at the
bound, but a peer that completes a command or a DATA line inside each
window earns another window, indefinitely: there is no session lifetime,
no DATA lifetime and no minimum transfer rate. At the defaults,
complete-line tricklers can occupy all 256 SMTP slots indefinitely, and
each DATA connection can retain a payload buffer approaching 50 MiB —
about 12.5 GiB across 256 connections before allocation, parsing and
other overhead (an exposure estimate, not an RSS ceiling); an over-limit
DATA drain has no total time bound either while complete lines keep
arriving. POP3 command trickles can hold its 256 slots the same way. A POP3
reply has no whole-reply deadline either: its bound applies to each socket
write, and every write that moves bytes starts a fresh window, so a reader
whose socket accepts some bytes of each next write within its window
receives a 50 MiB RETR in full, and keeps its slot and that reply's buffer
for as long as it takes — at the defaults, 256 such replies are about
12.5 GiB of raw-message-scale exposure, not an RSS ceiling; a write that
moves nothing for a whole window cuts the reader off (decision 0046,
accepted).
Progress means bytes the local socket accepted, not bytes the client's
application consumed, so a reader that stopped is seen only once its
socket buffers fill. Command reads keep their whole-command bound and the
cap's refusal stays one bounded write. The
cap and the idle bounds are no availability guarantee against a
deliberately slow client.

**The stop is wired to both signals and has a documented exit vocabulary.**
`SIGINT` and `SIGTERM` take the same path: the first starts the graceful
drain (`shutting down` on stdout, the 10 s drain deadline raced against
graceful serve). HTTP graceful shutdown closes admission: a new probe
after that gets connection refusal, not a `stopping` body; an already
accepted connection gets `503 {"status":"stopping"}` only if its request
reaches the handler after the stop, and may close without a response. A
second signal during the drain is the operator's
explicit "now" — a forced `exit(70)`, never a disposition accident. The
exit codes are **0** graceful, **70** forced; the image declares
`STOPSIGNAL SIGTERM` so `docker stop` rides the graceful path.

**STARTTLS** is real: with `--tls-cert`/`--tls-key` (or the env pairs), EHLO
advertises `STARTTLS`, the session upgrades with rustls/ring, and the
session restarts per RFC 3207 §4.2 — plaintext pipelining before the
handshake is discarded, every further command is TLS-only. A stalled
handshake is dropped after `--tls-handshake-timeout-ms` (default 10 s).
The value is not range-checked: `0` (or `SWARMAIL_TLS_HANDSHAKE_TIMEOUT_MS=0`)
is accepted at startup, and then every STARTTLS handshake fails — the
client gets `220 2.0.0 Ready to start TLS` and the connection is dropped
before any handshake can finish, so use a positive value. Plaintext
sessions answer to the per-I/O idle bounds above (5 min SMTP,
10 min POP3, decisions 0027 and 0036). Without a cert the
plaintext listener is byte-identical to the one without the flag.

```
$ swarmail gen-cert --domain localhost --out ./certs    # cert.pem + key.pem, 0600
$ swarmail serve --tls-cert ./certs/cert.pem --tls-key ./certs/key.pem
```

### The REST API — await, don't poll

`:8025` — the native API. The verbs a test actually needs:

| Verb | What it does |
|---|---|
| `GET /api/v1/inboxes` | the live inbox list, sorted by name — `limit` (default 100, ≤ 250, refused above) and `offset` page it |
| `GET/DELETE /api/v1/inboxes/{inbox}/messages` | list newest-first; wipe the inbox |
| `GET /api/v1/inboxes/{inbox}/await` | **block until arrival** — `to`/`from`/`subject`/`since_ms` filters, `count` (minimum one, even `count=0`), `limit`/`count` ≤ 250 (refused, never clamped), `timeout_ms` (≤ 60000: refused with 400 above, never clamped; a stopping server answers 503 `shutting down` instead of parking) |
| `GET /api/v1/inboxes/{inbox}/assert` | exact-count assertion — `count=0` asserts an empty filter; never clamped — 200 or a named failure |
| `GET /api/v1/inboxes/{inbox}/threads[/{key}]` | conversations, oldest first, stable keys |
| `GET /api/v1/inboxes/{inbox}/feed` | SSE: one snapshot, then one event per accepted mail |
| `POST /api/v1/inboxes/{inbox}/seed` | a fixture without SMTP — full parse + extraction; `html` preserved verbatim (supersedes `text`), CR/LF/NUL envelope fields refused |
| `GET /api/v1/messages/{id}` | one full mail |
| `GET /api/v1/messages/{id}/raw` | the RFC 5322 source as `message/rfc822` |
| `DELETE /api/v1/messages/{id}` | remove one mail |
| `GET /api/v1/chaos` | inspect active chaos rules |
| `DELETE /api/v1/chaos` | clear active chaos rules |
| `GET /api/v1/webhooks` | inspect redacted targets |
| `DELETE /api/v1/webhooks` | clear webhook targets |
| `DELETE /api/v1/delete-all` | clear every inbox |

Every mail carries `text`, `html`, `links`, `codes` — extracted at ingest —
and `envelope_from`, the SMTP `MAIL FROM` reverse path: `""` for a bounce's
null sender `<>`, `null` only for mail stored before swarmail kept it (it is
never backfilled from the header `From`).
`codes` holds at most the first **64 distinct** codes, in extraction order
(decision [0049](docs/decisions/0049-extraction-compat.md)): a
text part and an HTML part that are direct siblings in one
`multipart/alternative` contribute the text's codes (the HTML's only when
the text has none); otherwise both bodies contribute in source order, so a
mailing-list footer after an HTML-only mail cannot hide its code. Mail
stored before the cap keeps its longer array verbatim — nothing is
re-extracted or truncated on restore. `links` is uncapped (bounded only by
the body): a quoted attribute's link ends at the attribute's own closing
quote, so `href="…?name=O&#39;Brien&amp;t=1"` stays whole, and balanced
parentheses, IPv6 brackets and interior commas are kept while trailing
sentence punctuation and unmatched closers are trimmed.
`GET /api/v1/inboxes/{inbox}/count` is the exact-count read;
`/openapi.json` serves the whole contract (3.1).

**Request bodies are capped at 2 MiB.** Every REST body — the seed, the
webhook PUT, chaos, the shims' `DELETE /api/v1/messages` — is read through
axum's default limit of 2,097,152 bytes. One byte more is refused before
parsing with axum's own plain-text `413` (`text/plain`, `Failed to buffer
the request body: length limit exceeded`), not the JSON `error` envelope,
and nothing is stored or applied. A larger fixture goes in over SMTP,
whose cap is 50 MiB. `POST /mcp` has the same 2,097,152-byte limit but
answers in JSON-RPC: HTTP 413 with a null-id `-32600` naming the limit
(see MCP below).

The filters are exact about what they match. `to` is a case-insensitive
substring of **any recipient** — the `To` and `Cc` headers *and* the SMTP
envelope recipients — so a Bcc'd address matches, and `assert?to=…&count=0`
is not a "this address is in no header" check. `from` is the header `From`
address. Closed native query rules are method-specific: the existing
list/count/await/assert/thread/feed routes accept only their documented
keys; an unknown key is a `400` naming the route and key. These ten native
operations take no query keys: `DELETE /api/v1/inboxes/{inbox}/messages`,
`DELETE /api/v1/delete-all`, `GET` and `DELETE /api/v1/messages/{id}`,
`GET /api/v1/messages/{id}/raw`, `GET` and `DELETE /api/v1/chaos`,
`GET` and `DELETE /api/v1/webhooks`, and
`POST /api/v1/inboxes/{inbox}/seed`. Unsupported selectors (for example
`?subject=keep`, `?inbox=a`, or `?event=data`) and typos receive a native
JSON `400` naming the method, route and decoded key before the operation
reads or changes data/configuration. An empty-valued or bare key also
refuses; no query and a literal empty query keep their current behavior.
Seed's JSON syntax/media/size/data-shape errors retain precedence over a bad
query; a deserializable but domain-invalid seed body is checked after query
admission. Use the body-based `DELETE /api/v1/messages` selector to remove
specific ids. The Mailpit/MailHog list and search shims keep their upstream
tolerance of unknown keys — `/api/v1/messages?inbx=` is a typo that answers
the whole store — while the `latest` shims refuse them. This rule does not
change compat aliases, selective DELETE, chaos/webhook PUT, health, metrics,
UI or document routes.
The seed and webhook bodies are closed too (`subjet`, `secrt` refuse by
name). In JSON bodies an omitted optional field takes its default while a
present `null` is refused with the field named — on the seed's `from`,
`subject`, `text` and `html`, a webhook's `inbox`, `secret` and `ca_pem`,
and on a chaos rule's `error` alike (only a whole chaos event set to
`null` means "clear it"). Seed and webhook JSON data errors are `400` with
a JSON `error` (decision 0048): unknown keys, missing fields, wrong types,
nulls and wrong top-level shapes. Malformed JSON syntax stays the extractor's
plain-text `400`, missing/wrong media type its plain-text `415`, and
oversize its plain-text `413`. Chaos JSON data errors stay the extractor's
`422`. One inbox-name rule holds on every
surface — REST, MCP, UI, the shims, webhook inbox filters, SMTP AUTH and
POP3 `USER`: a blank name (empty after trimming whitespace) is refused, and
so is one carrying any control character (`inbox must not contain control
characters`); any other name is kept exactly as given, padding included, so
`" pad "` names the same inbox everywhere and never the inbox `pad`.
Previously stored rows are not renamed: a control-bearing name stored
before this rule still lists in `/api/v1/inboxes`, but its by-name routes
refuse it; its mail still shows in the unscoped `GET /api/v1/messages`
and is removed by id (`DELETE /api/v1/messages/{id}`, or `DELETE
/api/v1/messages` with an `ids` list). Nothing promises such a name ages
out.

### MCP — agents drive the mail

```
$ curl -s -X POST localhost:8025/mcp -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{…}}'
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26",
  "serverInfo":{"name":"swarmail","version":"0.1.0"}, …}}

$ curl -s -X POST localhost:8025/mcp -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":2,"method":"tools/call",
         "params":{"name":"swarmail_wait_for_email",
                   "arguments":{"inbox":"quickstart","to":"ada@x.io","count":1,"timeout_ms":2000}}}'
{"jsonrpc":"2.0","id":2,"result":{"isError":false,
  "content":[{"type":"text","text":"{\"emails\":[{…\"codes\":[\"424242\"]…}]}"}]}}
```

Thirteen tools: `swarmail_search_emails`, `swarmail_get_email`,
`swarmail_get_latest_email`, `swarmail_delete_email`, `swarmail_clear_inbox`,
`swarmail_clear_all`, **`swarmail_wait_for_email`** (blocks until arrival),
`swarmail_extract_links`, `swarmail_extract_codes`, `swarmail_seed_email`,
`swarmail_set_chaos`, `swarmail_clear_chaos`, `swarmail_get_deliverability_report`. Except for malformed seed fixture
content and the supplied
`timeout_ms` refusal described below, every tool validates its supplied
arguments at the envelope: a wrong-typed or malformed value — the
`arguments` container included — returns JSON-RPC `-32602` in HTTP 200
for a call with an `id`, naming the field and its domain, before any
effect, and so does an argument key the tool does not publish
(`{"inbx": "zz"}` can never clear `default`; every schema says
`additionalProperties: false`); an unknown tool name is `-32601`; a JSON
parse failure or invalid UTF-8 is HTTP 400 `-32700`; a body over the
2,097,152-byte limit is HTTP 413 `-32600`; a value that is not an object
(`5`, `null`, `"x"`, a nested array) is `-32600` with a null id — once per
such member in a batch; an object without an `id` is a notification only
when its `method` is a string: it runs and stays silent, whatever the
method names. Any other `id`-less object — `{}`, `{"foo":"boo"}`,
`{"method":1}` — is `-32600` with a null id, alone or as a batch member
(decision [0047](docs/decisions/0047-storage-rpc.md), issue #132).
`swarmail_search_emails` pages with `offset` past
its 250 limit. The members of one batch (at most 16) run concurrently:
sixteen waits share one wait interval rather than queueing, replies keep
member order, and the members' effects have **no** order — send dependent
mutations as separate requests.

An elapsed `swarmail_wait_for_email` timeout is a normal tool result:
`isError: false`, with `matched`, `total` and `timed_out: true` in its text
content. On `swarmail_wait_for_email`, a supplied `timeout_ms` must be an
integer from 0 through 60000; malformed values and values above 60000
return an HTTP-200 tool result with `isError: true` and refusal text, not
a JSON-RPC `error` object.
`swarmail_seed_email` refuses `CR/LF/NUL` in its `to`, `from` or
`subject` fixture strings before insertion under decision
[0010 §2](docs/decisions/0010-exact-count-asserts-and-the-shared-seed.md);
for a call with an `id`, it returns HTTP 200 with a tool result
`isError: true` whose text names the offending field and its reason;
no message is inserted. Wrong-typed seed arguments and unknown keys
remain JSON-RPC `-32602`; id-less notifications stay silent.

Tool failures such as `shutting down` return
`isError: true` with an error message in text content.
An unknown MCP argument key returns JSON-RPC `-32602` in HTTP 200 for a call
with an `id`, before any effect. Other protocol and
argument refusals use a JSON-RPC `error` envelope as described above.

Wire it up:

```jsonc
// .mcp.json — HTTP transport
{ "mcpServers": { "swarmail": { "type": "http", "url": "http://localhost:8025/mcp" } } }
// or stdio subprocess:
{ "mcpServers": { "swarmail": { "command": "swarmail", "args": ["mcp"] } } }
```

`swarmail mcp` speaks JSON-RPC over stdin/stdout and bridges to
`SWARMAIL_URL` (default `http://127.0.0.1:8025`). The HTTP surface is
Streamable JSON-RPC at `POST /mcp`. A URL without a port dials HTTP's port
80 and sends its `Host` exactly as spelled; a malformed port is refused
(decision [0047](docs/decisions/0047-storage-rpc.md), issue #133).

The bridge answers each line as `POST /mcp` would (decision
[0039](docs/decisions/0039-stdio-concurrency.md), updated by
[0044](docs/decisions/0044-compat-validation.md)). A batch with requests
gets one array line; a notification, or a batch of notifications only,
gets nothing; a line that is valid JSON but not an object — or an `id`-less
object without a string `method` — is an invalid request and gets the
server's null-id `-32600`. A line whose body is over the server's
2,097,152-byte limit (counted in bytes, without its line ending) is never
sent: the bridge answers `-32600` in the server's own 413 wording with the
line's own ids — one error, or one array in member order for a batch —
and nothing for a notification. Up to 16 lines are in flight at once, so a wait never holds
up the seed that satisfies it. Replies arrive as they complete, so order
holds only within a batch. A client that needs two mutations in order
should await the first reply. A line that is not UTF-8 or not JSON gets a
local `-32700` with a null id and never reaches the server. If the server
cannot be reached, each request id — and each non-object member, as a
null id — gets a `-32000` error, and a rejected batch gets one null-id
error. The outcome of such a request is unknown,
and nothing is retried. At EOF the bridge waits for every request in
flight to finish, then exits 0. It exits 1 if stdin fails (after waiting)
or if stdout fails (immediately). Known gaps: no length cap on reading or
parsing a single input line (the body limit bounds what is sent, not what
is held); a stalled stdout reader stalls the bridge; sixteen parked
waits delay a seventeenth line; a server or proxy with a lower body limit
than the bridge's still answers its own null-id refusal.

### POP3 — the same store, RFC 1939

`:1110` — USER, PASS, STAT, LIST, UIDL, RETR, DELE, NOOP, RSET, TOP, CAPA,
QUIT. The maildrop **is** a swarmail inbox: `USER proj-42` names the inbox
exactly like SMTP AUTH does, so mail delivered over SMTP reads back over
POP3 — including mail that arrived before the session opened. The maildrop
is a snapshot taken at successful `PASS`, not a lock: numbering stays stable,
and new arrivals appear in the next session. Listings read the snapshot;
`RETR`, `TOP` and `DELE` consult the live store, so mail removed elsewhere
can remain listed but these commands answer `-ERR no such message`
(decision [0028 §3](docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md)).
The name is everything after `USER` and its one space, verbatim (only the
line's CRLF is
removed, so an extra CR is part of the name and refused); a command that is
not UTF-8 ends the session unanswered. UIDLs are the
swarmail ids; octet counts are the payload the client keeps — every line
CRLF-terminated, the transparency dots RETR adds not counted (RFC 1939
§11), the terminating `.` not counted — derived from the stored payload by
the same traversal RETR writes with, never stored a second time (decision
[0037](docs/decisions/0037-pop3-size-accounting.md), amending
0028 §1). DELE marks in-session and applies
on QUIT — the update stage; a DELE of mail removed elsewhere answers
`-ERR no such message`, like RETR; RSET unmarks; a dropped connection
leaves the maildrop untouched. Plaintext only: no APOP, no STLS — a test-harness
surface behind the same trust boundary as the SMTP listener.

### MailHog / Mailpit shims

Point an existing client at the HTTP port and it works — the two shapes
test suites actually hit, answered from one store:

| Endpoint | Behavior |
|---|---|
| `GET /api/v1/messages` | Mailpit's summary envelope, newest first — the receive time, then id, as `latest` uses, scoped or not; `?inbox=` scopes, `?limit=`/`?start=` page; each `Snippet` is built from at most the first 8192 bytes of its body |
| `GET /api/v1/message/{id}` | Mailpit's full message shape (`Text`, `HTML`, `ReturnPath`, …) |
| `GET /api/v1/message/latest`, `…/latest/raw`, `…/latest/headers` | the newest mail — the maximum receive time, then id — in Mailpit's message shape, as its raw source, or as its header map; `?inbox=` scopes it to one inbox (404 when that inbox is empty, whatever the others hold), no `?inbox=` means the whole store; a nonblank `?query=` is a `400` — search, then fetch the id |
| `GET /api/v1/message/{id}/plain`, `GET /api/v1/messages/{id}/plain` | the text part as `text/plain; charset=utf-8` — html-only mail is rendered to text at request time |
| `DELETE /api/v1/messages` | Mailpit's `{"ids": [...]}` delete (`IDs` accepted alike) — the named ids and nothing else; every other body (absent, empty, unparseable, `{"ids": []}`) is a `400` that deletes nothing — including MailHog's bodyless wipe (decision 0011) |
| `DELETE /api/v1/delete-all` | swarmail's own explicit wipe of every inbox — not an upstream route |
| `GET /api/v1/search` | Mailpit search: `kind=from\|to\|subject\|containing`, `?query=` required; the envelope's `messages_unread` is the match count |
| `GET /api/v2/messages` | MailHog v2 `{total, count, start, items}` with `data.Message` shapes; a page whose JSON would pass 64 MiB is refused whole by name |
| `GET /api/v2/search` | MailHog search: `kind=from\|to\|containing` present and one of the three, as upstream requires; a bare `400` on an unknown kind, an absent kind or an empty query; `?limit=`/`?start=` page |
| `GET /api/v1/messages/{id}/download`, `GET /api/v1/message/{id}/raw` | the RFC 5322 source as `message/rfc822` |

**Deliberate omissions** — swarmail has no counterpart and a shim would
lie: attachments and inline images (the fields exist, the arrays are
empty), read/unread state, tags (always `[]`), the send API (mail arrives
by SMTP), Mailpit's `mime/part` routes, `latest` on the plain/attachment
routes, and the `to:`-style query prefixes: in a containing search, every
whitespace-separated word that opens with a field name and a colon
(`to:`, ` from:alice`, `plan subject:x`, an unknown `foo:`) answers a named
`400` — the prefix grammar is not implemented; swarmail takes `kind=`.
Both shims remove every double quote from each word for refusal recognition
only, then look past one leading `(` and one exclusion `-` or `!`
(decision 0061, accepted 2026-10-26). Thus `-to:x`, `!from:alice`,
`"to:x"`, `-"subject:x"`, `(to:x)`, `"-to:x"`, `"!from:alice"`,
`"-subject:a b"`, `to":x` and `"to":x` refuse by the field's name instead of
silently matching nothing. A URL's `://` is exempt, so `https://host/path`
(and `-https://host/path`) stays a literal search, as do `-word` and
`"a phrase"`, but other colon words (`mailto:x`, `"note:x"`) refuse too.
Accepted queries execute with their original quotes and markers; no quoting,
grouping or negation grammar is implemented. Supported explicit-kind values
stay literal (Mailpit's `subject` extension remains outside MailHog's kinds).
swarmail's own single-message route is
`/api/v1/messages/{id}`; the Mailpit-style singular `/api/v1/message/{id}`
is the shim.

**Known differences** — the shapes are close but not byte-identical, and
the divergence is the honest part: a malformed `limit=`/`start=` answers
axum's plain-text `400` (not Mailpit's JSON error envelope — matching it
means a custom query extractor for one status-code shape); a blank
`?inbox=` is a `400`, never the whole store. The sender the shims report —
MailHog `From`, `Raw.From` and `kind=from`, Mailpit `ReturnPath`, the
synthesized `Return-Path` — is the SMTP envelope sender, as upstream's is
(`""` and `<>` for a bounce); only mail stored before swarmail kept the
envelope falls back to the header `From`. Mailpit's `From` and the native
`from` filter stay on the header. Every compat list and search, scoped by
`?inbox=` or not, and `latest` order by the receive time, then the id —
so at a stable store `messages?inbox=x&limit=1` is `latest?inbox=x`. The
receive time is stamped when parsing starts, so a large mail accepted after
a small one can sort behind it; the native inbox list, POP3 numbering and
pruning keep acceptance order, and can disagree with the compat views.
`latest` does not read Mailpit's `tz`: `?tz=` is a `400` naming it, like any
other unknown key there.

### Threads and the live UI

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

### Webhooks

```
$ curl -X PUT localhost:8025/api/v1/webhooks -H 'content-type: application/json' -d \
    '[{"url": "http://127.0.0.1:9999/hooks", "secret": "s3cret"}]'
```

Every accepted email is POSTed as `{"event":"received","email":{…}}` with
the secret in `X-Swarmail-Secret` — queued, retried (100 ms → 1.6 s, four
attempts), inbox-filtered, and never awaited by SMTP acceptance. `http://`
targets stay plain; `https://` targets are TLS with verification on — hand
a self-signed sink its own CA through the target's optional `ca_pem`.
Clients pinned to a `ca_pem` are cached per PEM under a fixed bound of 32
(`swarmail::webhook::CA_CLIENT_CAP`); the LRU build is evicted and the next
delivery rebuilds it. `swarmail_webhook_ca_cache_{entries,builds_total,
evictions_total}` on `/metrics` lets you watch the bound hold. `builds_total`
counts successful delivery-side constructions, including duplicate builds
that lose a publication race or finish after a waiter is cancelled; API
validation/projection and cache hits do not count.

A target is validated when it is PUT, not at delivery. Its keys are
exactly `url`, `inbox`, `secret` and `ca_pem` — an unknown key such as
`secrt` is refused, and an optional one is either omitted or a string,
never `null` (omit `inbox` for every inbox). The URL must parse
with an `http`/`https` scheme and a host to dial (`localhost:9000` is read
as *scheme* `localhost` and refused — the misconfiguration that would
otherwise surface only at delivery); every certificate in `ca_pem` — at
least one, a bundle's later roots included — must load into the delivery
TLS client itself, built the way delivery builds it (decision 0048: this
proves the roots load, not that a later handshake succeeds); and the
`secret` must be an HTTP header value
the delivery client accepts — no control characters but tab, no leading
or trailing space or tab — of at most 4096 octets. A refusal is a 400
naming the defect (never echoing the secret, the URL or any wrong-typed
value: a body-shape error names the path and the JSON kind) and applies
nothing: a list
with one bad target leaves the previous list in place. `GET
/api/v1/webhooks` and the PUT echo redact the secret and strip URL
userinfo (`http://user:pw@host/x` is shown as `http://host/x`; delivery
still sends the credentials as Basic authorization). `ca_pem` is projected
to canonical, validated certificate blocks only: unrelated PEM sections
and prose are omitted, while certificate DER, order and duplicates are
preserved. A supplied bundle that cannot be fully projected is shown as an
empty string; an absent value stays `null`. Installed input and admission
are unchanged, including ignored unrelated sections. Query strings and
fragments are shown as configured — keep credentials out of them. PUT is the
only writer; an operator retains their own payload — the echo is diagnostic,
not a body to PUT back.

Delivery is best-effort with the loss made observable — there is no durable
outbox: a mail whose fan-out cannot fire is never re-fired after a restart.
At most 32 deliveries are in flight **per target**
(`swarmail::webhook::WEBHOOK_DELIVERY_CAP`, keyed by the target's
normalized URL): a stalled sink holds only its own target's slots, so it
cannot consume another target's delivery slots. CA admission is shared across targets. A mail whose target's slots are all
taken has that fan-out refused at the bound, counted once in
`swarmail_webhook_dropped_total`, and never retried; the first refusal
per target warns, and further refusals within 60 s of that warning are
counted without a line. A mail's payload is serialized once and shared by
every target and retry, so extra targets cost references, not copies of
the mail. A target that accepts
and never answers wedges its delivery for the dispatcher's whole budget —
about 22.1 s for network attempts and backoff (the cold-client build and
CA admission/build time are outside that budget) — so a stalled slot holds
the bound for at least that portion, then the delivery is
counted in `swarmail_webhook_exhausted_total`, and its final refusal is
logged at the default level: one `warn` naming the target URL and the last
delivery error (no per-attempt spam). Server-managed CA work uses two process-wide dedicated native workers with
no pending queue and a 64 MiB logical charged-job budget (#220). GET projection,
PUT complete validation/projection and custom-CA delivery cache misses share
nonblocking admission. Held CA jobs cannot consume the Tokio blocking slots used
by SMTP parsing and Store commits. This provides no OS scheduling or latency
guarantee. The two threads stay resident until process exit, across server stops
and runtime replacement; stop does not drain builders. Cancelled running jobs and
retained results stay charged until disposal. Two stuck jobs require a restart.

GET and PUT return native-envelope 503 `webhook CA preparation unavailable` on
capacity/byte exhaustion, contention, closed scope or failed worker startup.
Admitted worker failure retains GET 500 `webhook response preparation failed` and
PUT 500 `webhook configuration preparation failed`; invalid PUT remains 400.
Refused/failed/cancelled PUT keeps the old list. First successful reservation wins:
there are no class quotas or fairness guarantee. A refused delivery miss spends
an ordinary attempt and may exhaust all four attempts under GET pressure.
Oversized valid input can refuse, and an oversized directly installed snapshot
can keep returning GET 503 until replaced/cleared. The HTTP body limit is unchanged.

The logical tariff charges job-owned input/output/copies, parser/root/client
representations and metadata with explicit weights (decision 0078). It is not an
allocator or RSS ceiling. HTTP extraction, installed configuration, pre-existing
dispatcher snapshots/shared payloads, transferred responses/cache, transport,
thread stacks and synchronous embedder validate/redacted calls are outside it.
The five ledger counters —
`swarmail_webhook_{delivered,dropped,lagged,exhausted,aborted}_total` — are
cumulative since process start (they reset on restart, they are not
persisted), and a shutdown counts its backlog of undelivered notifications
as aborted instead of dropping them silently. That count is made after the
HTTP plane has stopped, so `/metrics` cannot show it: on a graceful stop
the dispatcher logs one default-level `webhook dispatcher stopped` line
with all five final totals (`delivered= dropped= lagged= exhausted=
aborted=`) on stdout.

### Chaos — break SMTP on purpose

```
$ curl -X PUT localhost:8025/api/v1/chaos -H 'content-type: application/json' -d \
    '{"data": {"probability": 100, "error": "451 4.3.0 tempfail", "delay_ms": 250}}'

$ python3 -c "
import smtplib
s = smtplib.SMTP('localhost', 1025, timeout=10)
s.sendmail('a@x.io', ['b@x.io'], 'Subject: t\r\n\r\nbody')"
…
smtplib.SMTPDataError: (451, b'4.3.0 tempfail')              # the injected line, verbatim

$ curl -X DELETE localhost:8025/api/v1/chaos                  # recovery is a test too
```

Chaos is per-stage — `connect`, `mail_from`, `rcpt`, `data` — each with its
own probability, error line and delay. Set it, run the retry suite, clear
it. MCP: `swarmail_set_chaos` / `swarmail_clear_chaos`. Wire values are
never narrowed: `probability` must be an integer 0..=100 — present, a true
integer, within the ceiling — each stage refusing by name, and a
wrong-typed `delay_ms` is refused while an absent one honestly means "no
delay" (decision 0018). A supplied `error` must be a reply line: a single
line starting `4xx`/`5xx`, at most 510 octets (RFC 5321 §4.5.3.1.2's 512
less the CRLF `write_line` appends) — a `2xx` error is refused, because a
rule that would answer success while the mail is not stored is a lie the
wire cannot tell. A supplied `delay_ms` is an integer 0..=300000, the
stage-timer scale — a chaos stall is a wire condition, not an unbounded
hang. Refused values answer 400 with the rule's own sentence (the same
words on the MCP plane), and a rejected PUT applies nothing.

A fired rule always applies its delay, then always fails its stage — a
delay-only rule (probability set, no `error`) is a slow failure, never a
harmless slow-down (issue #17):

| Stage | Delay-only rule (no `error`) | With `error` |
|---|---|---|
| `connect` | wait, then close with no line at all — neither banner nor error | wait, then the given line replaces the banner |
| `mail_from` | wait, then `451 4.3.0 chaos` | wait, then the given line |
| `rcpt` | wait, then `451 4.3.0 chaos` | wait, then the given line |
| `data` | wait, then `451 4.3.0 chaos` — the mail is **not** stored | wait, then the given line — the mail is **not** stored |

So an omitted `error` means *the stage's default failure*, not *no failure*
— turn chaos off with `probability: 0` or by clearing the config. A
present `"error": null` is refused like any non-string (decision 0044 §3);
an unset line is omitted from the answer, so a fetched config PUTs back. Two
orderings are pinned: a genuine over-limit `552 5.3.4` is answered before
the data chaos rule is consulted (resource failures precede injected
ones), and a data rule's delay applies after the body has been received
and before the reply — the transfer itself is never slowed.

### Persistence — the restart copy

By default the store is memory — fast, and gone when the process exits.
With a data file, every insert, delete and clear is committed to bundled
SQLite (WAL, fsync-per-accept) and the full state — inboxes, messages with
raw bytes, the `emails_inserted`/`emails_dropped` counters — is restored
before the server answers anything:

```
swarmail serve --data-file /var/lib/swarmail/mail.db   # or SWARMAIL_DATA_FILE=…
```

Restored mail does not re-fire webhooks or waiters — it is
already-delivered state, not a new delivery.

**One server per data file** — a second process opening the same file
refuses at startup (a `<data-file>.lock` advisory lock held for the
process's lifetime). It exits non-zero with `the data file is already in
use by another server: <path>` before binding anything (decision
[0033](docs/decisions/0033-one-server-per-data-file.md)). The lock dies with
the process: a stopped or killed server's file is immediately reusable, and
no manual unlock ever exists — the `mail.db.lock` file left beside the data
file is inert and is never deleted. A replacement started while its
predecessor is still draining is refused until that predecessor exits;
retrying belongs to the supervisor. Restart reloads what is on disk; it does
not repair a file two servers already wrote together before this lock
existed (decision [0042](docs/decisions/0042-single-writer-lock.md)).

The refusal follows the *file*, not its name: besides the sidecar, the
server holds an identity guard,
`/tmp/swarmail-db-locks-<owner uid>/<device>-<inode>.lock`, named by the
data file's owner, device and inode — which every symlink and hard link of
the file shares, in any directory — so a second server reaching it through
an alias is refused the same way (decision
[0047](docs/decisions/0047-storage-rpc.md), issue #129). The root is always
`/tmp`, whatever `TMPDIR` says. The server never locks the SQLite database
or its `-wal`/`-shm` files itself: on macOS and the BSDs such a lock
conflicts with SQLite's own. The owner directory must be private (0700) to
the data file's owner and is never repaired; an unsafe directory or guard
refuses startup. Guard files are inert after the server stops and are never
deleted by it; exclude `/tmp/swarmail-db-locks-*` from age-based `/tmp`
cleanup while any server runs, and clear it only when none does. A data
file the server creates is no wider than SQLite's own 0644 (subject to
umask); an existing one keeps its mode and bytes. The guarantee covers
cooperating current servers running as the data file's owner, on a local
filesystem with working `flock`, sharing one `/tmp` (separate containers
with private `/tmp` mounts do not share guards; another user's server may be
refused with an I/O error rather than "in use"). Still **not** refused: an
older, sidecar-only swarmail reaching the file through a different alias
(the same path still collides on the sidecar); a writer that takes no lock
at all; replacing the file or its directory under a live server; single-file
bind mounts (unproven); and network filesystems, whose lock semantics vary.
Keep one configured path across
restarts — after a crash, SQLite looks for the `-wal` file beside the name
it was given, so restarting through another hard link's name would miss
the crashed server's last commits.

A missing ordinary data path is created. A dangling symlink refuses startup
as `OpenError::Io` for the supplied path with `ENOENT`; the symlink remains
unchanged and its missing target is not created. Relative data paths and
paths through a symlinked parent are admitted. When cooperating servers
share the stable namespace, either spelling of the same file is refused as
`InUse` while an owner holds it, with the rival's supplied spelling in the
error; it can be opened after that owner drops. These spelling checks prove
the existing ownership behavior without attributing the refusal to one
particular guard.

**Memory is not globally bounded** (decision
[0038](docs/decisions/0038-store-memory-and-timestamps.md)).
`SWARMAIL_MAX_PER_INBOX` caps messages per inbox — `0` disables it — and is
not a global byte or RSS cap: the inbox count is unlimited, and all
persisted rows restore before bind without application of that cap. A
resident mail costs its raw bytes, decoded bodies and extracted fields, its
index entries, any copy a watcher still holds, and the buffers of concurrent
requests; the data file's size on disk is not its size in RAM. Each distinct
awaited inbox name keeps a watcher channel — about 780 B per name observed
on one build, not a fixed ceiling — until that inbox is cleared or emptied
or the process restarts; awaiting the same name again reuses it. This is
separate from the 4,096-name cap on the await-metrics registry.

**`received_at` / `received_ms` are parse-start stamps**: sampled when
`build_email` begins, before parsing and before acceptance. They are not
the sender's `Date` and not a commit sequence, and need not sort in
acceptance order across sessions or clock steps — a large mail can be
stamped before a small one that is accepted first. Per-inbox order, POP3
numbering, oldest-only pruning and the data file's restore order all follow
acceptance. Restart is exercised by
`tests/persistence.rs`: the exact state comes back, and a clear racing a
250-answered insert never loses the answered mail.

## Benchmarks

| Measurement | Result | How |
|---|---|---|
| Parse + extract | **1.28 µs/mail** (~780k/s/core) | `cargo bench` · criterion median |
| + store insert | **1.58 µs/mail** (~630k/s/core) | `cargo bench` · criterion median |
| + store insert at the default cap, steady state | **~2.1 µs/mail** (~470k/s/core) — memory-only, no `--data-file` | `cargo bench` · criterion median (`ingest/store/insert@cap-steady`) |
| End-to-end SMTP ingest | **5000/5000 exact**, zero loss | `cargo test --release --test rate -- --ignored --nocapture` (100 concurrent fresh sessions; session-latency-bound — scales with cores) |

Reproduce with `cargo bench` (results land in `target/criterion`). The
cap-steady row is the shipped default configuration
(`SWARMAIL_MAX_PER_INBOX` = 100 000): a store prefilled to exactly the cap,
every measured insert evicting exactly one oldest mail — the shape the
accept path runs in production. The uncapped rows above describe an
unbounded store, and before the deque eviction landed, the same cap-steady
bench measured ~13.7 µs/mail at this cap: the front-drain shift, not the
parse, was the cost. The capped query paths publish in the same run
(`ingest/threads/view@cap`, `ingest/store/count@cap`,
`ingest/store/count@cap-filtered`).

## Repo layout

| Path | What it is |
|---|---|
| `src/smtp.rs`, `src/pop3.rs` | the two wire protocols, hand-rolled on tokio |
| `src/store.rs`, `src/persist.rs` | the synchronous store and the SQLite restart copy |
| `src/extract.rs`, `src/threads.rs` | ingest-time extraction and conversation grouping |
| `src/api.rs`, `src/compat.rs` | the native REST API and the MailHog/Mailpit shims |
| `src/mcp.rs`, `src/stdio.rs` | the MCP HTTP surface and the stdio bridge |
| `src/chaos.rs`, `src/webhook.rs`, `src/tls.rs` | the failure injector, the dispatcher, STARTTLS |
| `src/static/` | the embedded UI, `/openapi.json`, `/llms.txt` |
| [`docs/decisions/`](docs/decisions/) | the constitution: numbered rulings, indexed |
| [`docs/house-rules.md`](docs/house-rules.md) | the realm charter — frozen guarantees, gates, dependency policy |
| [`assets/logo.svg`](assets/logo.svg) | the brand mark, and `social-preview.png`, the repository's link card |
| [`scripts/coverage-gate.sh`](scripts/coverage-gate.sh) | the exact-coverage gate: literal 100% line/branch/function, or refusal |
| [`charts/swarmail/`](charts/swarmail/) | the Helm chart — verified offline by [`scripts/chart-verify.sh`](scripts/chart-verify.sh), published as OCI |

## The decision culture

Every semantic change is a numbered ruling in
[`docs/decisions/`](docs/decisions/), kept in full and cited by number in
the code that enforces it — the guarantees above are exactly those
citations. An implementer may write a decision, but only with status
`proposed`; acceptance is the operator's, recorded in the file. A ruling is
never edited into a different meaning — a new number supersedes it and says
so. That is why the README, the error messages and the tests can all cite
"decision 0001" and mean the same paragraph.

The realm charter is [`docs/house-rules.md`](docs/house-rules.md); agents
start at [`AGENTS.md`](AGENTS.md).

## Testing & coverage

**Every integration test runs against real servers speaking the real
protocol** — no mocks in the loop. The bar for a human contribution is the
bar the machine is already held to. **The gate list is
[`docs/house-rules.md` § Gates](docs/house-rules.md#gates--part-of-every-change)** — the one
inventory, `--locked` builds, the coverage selftest, the publish-guard proof
and the CI-only proofs included; this README keeps no copy of it. CI
([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs it as these
jobs:

| CI job | What it runs |
|---|---|
| `quality` | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, the publish guard's proof, the shim digests and the design check's selftest |
| `test` (ubuntu + macOS) | `cargo test --workspace --locked`, then the ignored port-80 bridge proof |
| `coverage` | the coverage gate's selftest, the exact gate itself and the delivery-proof test; uploads `target/coverage/` |
| `deny` | `cargo deny check licenses` and the fail-closed `native-tls` absence proof |
| `dependency-audit` | the RustSec advisory audit (network-backed, CI-only) |
| `brokkr-compile` | applies the checksummed engine patches, proves review aggregation, builds the recorded brokkr, and compiles self plus all three Codex recipes |
| `chart-verify` | `scripts/chart-verify.sh`: its selftest, the chart-version bump check against history, and lint + both profiles + contract assertions |
| `docker-smoke` | real Docker: the built image as its own uid keeps accepted mail across a graceful stop and a hard kill on a named volume, `--rm` leaks no anonymous volume, and no `target/` reaches the build context |
| `release-rate-5k` (tags only) | `cargo test --release --locked --test rate -- --ignored --nocapture`, the 5000/5000 proof |
| `docker`, `chart-publish` | publication only, after the jobs above succeed (main and `v*` tags for the image, `v*` tags for the chart) |

The coverage gate is not a percentage to trend upwards. It demands literal
integer equality — `covered == count` — over **lines, branches and
functions**: exactly 100/100/100, every missed count zero, on every run
(the counts themselves grow with the code, so none is quoted here). It
refuses `#[coverage(off)]` outright so production code cannot shrink its
own denominator, and writes the full lcov report and summary under
`target/coverage/` — kept when red, so the burn-down starts from evidence,
not from a number. There is no threshold to lower.

```bash
cargo test                                                    # the suite (seconds)
cargo test --release --test rate -- --ignored --nocapture    # the 5k guarantee
cargo bench                                                   # ingest path → target/criterion
bash scripts/coverage-gate.sh                                 # the floor gate
```

| Suite | What it proves |
|---|---|
| `tests/p2.rs` | the P2 end-to-end: MCP JSON-RPC, the seed API's parse+extraction pipeline, webhook delivery with secret and retry targets |
| `tests/smtp_edges.rs` | every protocol verb, refusal, AUTH form, DATA limit, dot-unstuffing, group addresses, chaos on connect/MAIL FROM/RCPT/DATA; the 0034 boundaries — mixed terminators, path and parameter matrices, greeting resets, validation before chaos, strict PLAIN and one AUTH per session |
| `tests/rate.rs` | the 5k losslessness guarantee + burst |
| `tests/burst.rs` | 1000 mails / 50 conns exact-count, per-inbox isolation, cap eviction, chaos rejection + recovery |
| `tests/bulk_sender.rs` | Source-citing bulk rules, original header grammar, genuine text bodies, untrusted auth unknowns and inclusive analysis limits (proposed 0089) |
| `tests/deliverability.rs` | Lazy typed report, REST/MCP parity, threshold assertions/waits, UI states, persistence, pruning and served contract documentation (proposed 0085) |
| `tests/api.rs` | every REST route, happy + error branches |
| `tests/config_cli.rs` | the CLI/env config surface, the independent SMTP/POP3 idle defaults, the rendered `serve --help` |
| `tests/listener_contract_docs.rs` | the listener contract as documented: README, the controlling 0027 clarification and 0036 state the per-I/O bounds, the unbounded complete-line trickle residual and its costs, and the conditional `stopping` answer |
| `tests/mcp.rs` | every tool, JSON-RPC protocol errors, `isError` results |
| `tests/ui.rs` | every page, escaped (injection-proof) rendering |
| `tests/webhook.rs` | delivery with secret, pathless target, the retry budget, https over a real TLS server, the CA client cache bound |
| `tests/stdio.rs` | the stdio JSON-RPC bridge over in-process duplex pipes |
| `tests/starttls.rs` | STARTTLS: byte-identical plaintext listener, real rustls upgrade, RFC 3207 restart (identity and envelope reset), refused certs, stalled handshakes, malformed PEM refusal, gen-cert file handling |
| `tests/lifecycle.rs` | graceful stop: serve futures return, ports release, open feeds and stalled webhook deliveries cannot hold stop hostage, interrupted SMTP/POP3 transactions apply no partial changes, parked REST/MCP awaits answer shutdown inside the stop budget, three live seed requests observed under a held SQLite write lock refuse with 503 and store nothing, and fan-out in flight at drain is counted |
| `src/lifecycle_tests.rs` | `src/lifecycle_tests.rs::stalled_body_selects_drain_deadline` retains an unfinished real HTTP body and asserts the selected drain-deadline arm emitted its receipt; `src/lifecycle_tests.rs::graceful_http_completion_emits_no_deadline` is the negative control requiring no deadline receipt |
| `src/api.rs` | `tests::a_healthz_inside_its_handler_at_the_stop_is_answered_503_stopping` proves the handler-entered `503 stopping` response; the drain deadline ends the serve wait without promising every detached socket is closed |
| `src/store/persist_tests.rs` | `an_in_flight_commit_neither_holds_stop_nor_half_lands` proves the admitted-commit stop obligation beside the gate that holds it |
| `tests/binary.rs` | the shipped binary: SIGINT and SIGTERM → clean exit, SIGINT with seeds queued on a held write lock exits inside the budget and stores nothing, a second signal during the drain exits 70, a mail committed before SIGTERM is queryable after restart, mcp EOF → 0 |
| `tests/pop3.rs` | the POP3 surface: every verb happy + refused, multi-line replies, deletion-on-QUIT, its own idle bound wired independently of SMTP's |
| `tests/compat.rs` | the shims: Mailpit + MailHog envelopes, searches, deletes, 404/400 branches |
| `tests/persistence.rs` | restart restores exact state; a racing clear never loses 250-answered mail |
| `tests/sqlite_locks.rs` | a refused second server reaching the data file through an alias leaves the owner's SQLite locks intact — observed from a separate raw-SQLite child process (0047 §1) |
| `tests/store_contract_docs.rs` | the store, maildrop and compat contracts as documented — README, llms.txt, the chart comment and the served schema, their numbers built from the running constants — and every decision index row agreeing with its record's status |
| `tests/delivery_constitution.rs` | structural pins on `bundles/self/` (0050): the review-residual stops, the design sequence ending in the pinned check, the verify seat's pinned gates and shim check; and the documented install examples and CI's chart-history and docker-smoke wiring (#141) |
| `tests/delivery_recipes.rs` | Codex recipe discovery, triage routing and single/double/full review, inherited shell gates, work-seat tools, exact routes and independent review without Pro or author-model fallback (0057); [recipe tables and launch commands](bundles/README.md) |
| `tests/tooling_patches.rs` | the actual tooling-patch helper against real Git repositories: source identity, clean state, strict records, safe paths, all checksums before mutation and ordered patch application |
| unit | extraction, filters, chaos gating, model plumbing, POP3 line protocol, compat shapes |

For triage-first bug delivery, use the [Codex recipe guide](bundles/README.md).
Normal `brokkr tui` reads the canonical `.forge/forge.db`; isolated deliveries
name that shared journal explicitly. Each lane needs its own Git store,
branch, results, build and coverage outputs, and private cache overlays.
The prepared eight-lane layout reserves 16 CPUs per lane; throttle heavy
gates when host capacity requires it. Reserve overlapping files before launch.

Keep a live lane's committed recipe baseline immutable. A candidate gate repair
uses separate writable candidate scripts and must execute the repaired proof;
editing the pinned baseline invalidates the run. Compilation, local dispatch
fixtures and provider smoke prove different boundaries. Delivery still needs
committed-candidate hermetic verification, protected independent review and
the genuine ledger close-out. The operator seals the verified local branch
before publication or merge; agent runs never push or merge.

## Roadmap

- [x] STARTTLS + self-signed cert generation
- [x] HTTPS webhook targets (reqwest + rustls)
- [x] SQLite persistence (`--data-file` / `SWARMAIL_DATA_FILE`)
- [x] POP3 server; MailHog/Mailpit API compat shims
- [x] Exact 100/100/100 coverage contract (lines, branches, functions) in the gate and CI
- [x] MCP: HTTP + stdio, 13 tools
- [x] Kubernetes: the Helm chart (OCI-published, decision 0008 *accepted*)
- [ ] crates.io publish — the package is publish-ready (see *Publishing*);
  the operator runs `cargo publish --locked` with their token

## Publishing

The crate is **not published on crates.io** — no version has ever been
uploaded, and nothing in CI uploads one. What ships is the container image
(`ghcr.io/feedback-loop-ai/swarmail`) and the OCI Helm chart; from source,
install with `cargo install --locked --path .` from a checkout. The
package is prepared for a first upload should the operator choose to make
one: the packaged file set is curated in `Cargo.toml` (`include`) — the
sources, the embedded static assets, the real-protocol test suites, the
criterion bench, the WHATWG entity snapshot, both license texts and the
README, not the delivery constitution, the brokkr seats or the CI
workflow.

```bash
cargo publish --dry-run --locked   # proves the package builds; uploads nothing
```

An upload would be the operator's act (`cargo login <token>` with a
never-committed token, then `cargo publish --locked`, which publishes
exactly the dependency graph `Cargo.lock` pins); until one happens, no
document may claim the crate is on crates.io.

**Version policy:** a version bump is the operator's release act — bump
`version` in `Cargo.toml`, refresh `Cargo.lock` (`cargo check` keeps it in
lockstep, so `--locked` stays publishable), append
`docs/releases/vX.Y.Z.md`, and sign the tag (see *Releases* in
[CONTRIBUTING](CONTRIBUTING.md)). Shipped release notes are
**append-only, with dated errata**: what a release shipped and measured is
never rewritten; a later correction is added as a dated *Correction* beside
the claim (or in the notes' "Errata and provenance" section) naming its
issue and commit, and work landed after a tag is recorded in dated
"Unreleased" sections until the next cut. `docs/releases/v0.1.0.md` has
been extended this way since its tag; its errata section lists each edit
and the batch that made it.

## License

Dual licensed under either of:

- **Apache License, Version 2.0** — [LICENSE-APACHE](./LICENSE-APACHE)
- **MIT license** — [LICENSE-MIT](./LICENSE-MIT)

at your option (SPDX: `MIT OR Apache-2.0`) — the same convention as the Rust
ecosystem itself. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional
terms or conditions.

The WHATWG HTML entities data and derived table are licensed under CC-BY-4.0; attribution and transformation details are in [NOTICE](./NOTICE).

## Deliverability reports


GET /api/v1/messages/{id}/deliverability takes no query keys and returns
{rule_set_id, findings:[{rule_id,severity,location,fix_hint}]}. Severity is
info < warning < error. Locations are tagged header {name,occurrence:null|u64},
mime_part {path:[u64]} or link {part_path:[u64],byte_offset:u64}; link locators
are transfer-decoded source-byte coordinates, not URLs. Reports are sorted by
rule, typed location, severity and hint, with exact duplicates removed.
swarmail-bulk/1 adds seven source-citing bulk-sender checklist rules:
one_click, message_id and date are errors; plain_text is warning-level presence
advice. DMARC policy presence, From alignment and passing aligned DKIM-or-SPF
are independent checks. Their error-level *.not_evaluated.v1 findings explicitly
say "Not evaluated:" until trusted observations exist. Sender-written
Authentication-Results and DKIM-Signature never supply evidence. #255 owns that
verification and strict/relaxed alignment; #253/#255 own expiry and refresh.
Production therefore excludes every message at every report predicate threshold;
exact zero assertions still work, while waits still require a qualifying message.

Headers use original bytes and complete singleton values, with root lowercase
names and occurrence null/0/1 for absent/invalid/duplicate. Message-ID and Date
support current RFC 5322 syntax and surrounding comments/folding; obsolete-only
forms are unsupported by this checklist, not universally invalid Internet mail.
Date validates calendar and weekday, numeric zones and optional seconds (including
60), without a clock/freshness check. Message-ID checks syntax, not uniqueness.
One-click requires a complete URI list containing HTTPS and the exact case-sensitive
List-Unsubscribe-Post: List-Unsubscribe=One-Click value. This structural check
proves neither signed-header DKIM coverage nor endpoint operation; no URI is fetched.
Text advice checks genuine outer sender text/plain, including empty/default/encoded
bodies and nested inline multiparts, excluding attachments/ancestry and embedded
message bodies (including multipart/digest defaults). It proves neither content
equivalence nor an unconditional provider mandate. HTML is never converted here.
Hints cite https://support.google.com/mail/answer/81126 and
https://senders.yahooinc.com/best-practices/ plus applicable RFC 8058, 5322, 2046
or 7489 sections. These are fixed published references, not live verification or
provider certification. The proposed rule record is decision 0089.

The synthetic demo.marker.v1 remains: the first root X-Swarmail-Demo field,
unfolded and space/tab trimmed, selects info|warning|error and emits the fixed
hint "Remove the X-Swarmail-Demo header." Disabling the demo leaves bulk rules
active. Empty findings do not imply deliverability or authentication success.
Analysis is lazy from stored bytes, off SMTP; extraction stays at ingest.
The existing message page renders the same report without polling.

swarmail_get_deliverability_report requires string id and admits no other keys.
On current MCP 2025-03-26 it returns report JSON in one text block, isError=false.
Missing mail and failed/unavailable analysis use isError=true; malformed arguments
use -32602. No report outputSchema/structuredContent yet; that small follow-up
lands when #153 lands, independently of this delivery.

REST assert/await and MCP wait_for_email admit optional no_findings_at_or_above
(info|warning|error). It qualifies ordinary-filter candidates before paging;
failed reports abort, never count as clean. Exact assert counts its snapshot,
including zero, and never subscribes. Example: assert?count=0&no_findings_at_or_above=warning
holds when zero candidates qualify; it is not a universal inbox health assertion.
Wait still requires at least one, even count=0, and keeps ordinary totals/order.
Zero timeout uses completed cache entries only: cold/busy required reports return
unavailable. An incomplete scan at deadline is unavailable; a complete insufficient
scan keeps the usual timeout with its exact count. Shutdown keeps its existing error.
The option is refused on list/count; absent means existing behavior.

Cache: 128 Weak-identity slots per Store, exact immutable inputs and rule version,
four admitted blocking jobs retained through cancellation. Up to 256 findings and
65536 compact JSON bytes per cached report. Inclusive metadata limits are
8388608 raw bytes, 65536 per header block (including separator), 262144 aggregate
header bytes, 256 outer entities including root, and depth 32 (root zero).
Complete bounded MIME inspection includes closing boundaries and attached
containers; missing separators, ambiguous/unsupported metadata, encoded multipart,
truncation or exhausted budgets fail without partial success. Embedded messages
stay opaque. No body transfer decoding, SMTP admission change or stored-field
rewrite follows. REST: 404 missing, 503 "deliverability analysis unavailable" (capacity
or deadline) / "deliverability analysis failed" (analysis/size), native JSON error.
Selected pre-removal reads may finish; fresh lookups cannot resurrect removed mail.
Cache loss at restart costs recomputation. These are cache/job bounds, not total
memory or latency promises. #253 owns DNS observation/policy/expiry/wakeup semantics;
no DNS/authentication verification, spam scorer, external service or new dependency
ships here. The cache key includes every effective authentication fact and the
immutable policy/observation identity, alongside selected Weak mail identity.
