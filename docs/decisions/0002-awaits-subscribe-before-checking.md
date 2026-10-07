# 0002 — `await` subscribes before it checks

**Status**: accepted (original design, v0.1)

## Ruling

Every wait-for-mail primitive (`GET …/await`, MCP `swarmail_wait_for_email`)
registers its per-inbox watcher **before** scanning the existing store. The
order is: subscribe → scan → deadline loop. Never scan → subscribe.

## Why

Scanning first opens a race: a mail delivered between the scan and the
subscribe is seen by neither, and the waiter misses a message that is
already stored — the exact flaky-test failure mode sleep-polling produces.
Subscribe-first makes the race structurally impossible: the scan can only
find what arrived before it, and the watcher catches everything after.

## Consequences

- Waiters are push-based; no polling interval exists to tune.
- The watcher channel must be registered before the initial read in every
  new wait path — reviewers check this ordering explicitly (house rules).
- Duplicate wakeups are allowed (a waiter may wake and re-scan); missing a
  wakeup is not.
