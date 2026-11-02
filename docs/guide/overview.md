# Overview: the name, what it does, the guarantees

[← README](../../README.md) · [Guide index](README.md)

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

Eighteen MCP tools, one REST API, two wire protocols in (SMTP, POP3), one
compatibility layer (MailHog/Mailpit), one observability story
(`/metrics`, `/healthz`, `/openapi.json`, `/llms.txt`) — and a web UI with
zero frontend dependencies.

## The guarantees

These are the frozen guarantees ([decision records](../decisions/README.md)
bind them; review refuses a change that weakens one, no matter how green
the tests are):

1. **Accept == stored** ([0001](../decisions/0001-synchronous-insert-lossless.md)).
   `Store::insert` is synchronous; an SMTP `250` means the mail is queryable,
   and with `--data-file` it is committed with fsync-per-accept before the
   reply leaves.
2. **`await` subscribes before it checks** ([0002](../decisions/0002-awaits-subscribe-before-checking.md)).
   Nothing is missed while connecting; no sleep-polling exists anywhere in
   the product.
3. **Pruning is oldest-only, never mid-delivery**
   ([0003](../decisions/0003-oldest-only-pruning.md)). The per-inbox cap
   (default 100k, `0` = unlimited) evicts the oldest mail only.
4. **The AUTH username is the inbox**
   ([0004](../decisions/0004-per-test-inboxes-via-smtp-auth.md)). Free
   parallel isolation for every test and agent; no config to route it. An
   application that sends with one AUTH identity can opt in to
   [recipient routing](recipient-routing.md)
   instead (proposed [0112](../decisions/0112-recipient-inbox-routing.md)).
5. **Extraction happens at ingest, not query time**
   ([0005](../decisions/0005-extraction-at-ingest.md)). Links, codes and
   thread headers are parsed once, when the mail is accepted; queries
   re-parse nothing.
