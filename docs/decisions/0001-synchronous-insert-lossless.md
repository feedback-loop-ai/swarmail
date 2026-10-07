# 0001 — Synchronous insert: accept == stored

**Status**: accepted (original design, v0.1 — the founding constraint)

## Ruling

`Store::insert` is a synchronous call on the SMTP session's critical path.
The server writes the parsed email into the per-inbox map **before** sending
the `250` reply to `DATA`. There is no queue, no background flush, no
eventual-consistency window: when the client sees `250`, the message is
already queryable over REST, MCP and the UI.

## Why

The whole point of a test mail sink is that what you sent is what you can
assert on. MailCrab documents message loss above ~100 msg/s because its
websocket fan-out queue decouples acceptance from storage; MailSlurper's
session leaks lost mail outright. A mock that lies about delivery is worse
than no mock — tests pass against mail that never existed.

## Consequences

- Throughput is bounded by the store, not a queue — hence the criterion
  bench on the pure ingest path (~630k mails/s/core) to prove the bound is
  not the bottleneck.
- `tests/rate.rs` asserts the exact count (5000/5000) over 100 concurrent
  fresh sessions; any regression here fails CI.
- Fan-out consumers (webhooks, watchers) observe the store *after* insert
  and may lag; they may never delay acceptance (decision 0005's ingest
  posture).
