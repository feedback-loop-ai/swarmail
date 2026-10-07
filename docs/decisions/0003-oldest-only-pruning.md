# 0003 — Pruning is oldest-only, never mid-delivery

**Status**: accepted (original design, v0.1)

## Ruling

The per-inbox cap (`SWARMAIL_MAX_PER_INBOX`, default 100000) evicts the
**oldest** messages when exceeded. Eviction happens synchronously inside
`insert`, after the new message is stored, and only ever removes entries
older than the incoming one. A delivery in flight is never a prune
candidate, and `await` watchers holding message ids re-scan rather than
deref.

## Why

Unbounded memory is the other way test mail sinks die: a long agent session
fills the heap. But eviction must never make the store lie — a test that
asserted on mail, then found it gone mid-run, is the same failure as loss.
Oldest-only keeps the invariant honest: anything a client could have seen
and acted on stays visible for the whole window in which it plausibly
matters.

## Consequences

- `count` assertions remain exact under churn only for mail younger than
  the cap — the rate test disables the cap (`max_per_inbox = 0`) to assert
  5000/5000 exactly.
- Pruning cost is amortized into the ingest bench (1.58 µs/mail includes
  the cap bookkeeping at `max_per_inbox = 0`; the capped path is the
  default in production and covered by `tests/burst.rs`).
