# Webhooks

[← README](../../README.md) · [Guide index](README.md)

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
