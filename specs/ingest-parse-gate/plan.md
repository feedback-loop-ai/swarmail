# Plan: parse admission

Spec: [spec.md](spec.md). Decision: [0124](../../docs/decisions/0124-parse-admission.md)
(accepted (operator, 2026-11-06); it landed as compatible work under 0045 §8).

## Technical context

- `Store` owns the parse gate: `parse_gate: Arc<tokio::sync::Semaphore>`,
  built in `Store::new` (so `Store::open` inherits it) with
  `parse_width() = available_parallelism().map(NonZeroUsize::get).unwrap_or(1)`.
- `Store::parse_permit(self: &Arc<Self>) -> OwnedSemaphorePermit` awaits a
  slot. It is not a second offload helper: callers keep calling the one
  `Store::offload` and move the permit into its closure.
- Call sites: `src/smtp.rs` DATA arm (permit taken after `park_delivery`,
  before the test-only submission signal and the `offload`), `src/api.rs`
  `seed_inbox`. `insert_async` and every other mutation are untouched.
- Test seams, `#[cfg(test)]` only (0020 §4): `set_parse_width_for_test`
  (narrow the gate before the store is shared) and a waits counter
  (`parse_waits_for_test`) that lets a test know a session has reached the
  gate without sleeping.

## Design choices

- **Asynchronous acquire**, so no pool thread parks on the gate.
- **Parse only, release before the commit**: the boundary and persist mutex
  already order commits; holding a permit across an fsync would starve the next
  parse.
- **Width = usable cores**: the probe shows a looser cap buys nothing. The
  quota-aware std call makes `--cpus=2` mean 2 on any host. No flag.
- **Rejected:** fusing parse and insert (measured no effect); an allocator
  (new dependency); `max_blocking_threads` (bounds every pool user, parked
  work included); an async mutex before `insert` (the lock is not the cost).

## Risks

- A hostile large body holds a slot for its parse; `cores` of them delay other
  parses instead of competing for the CPU. No lock is held through it.
- A test that blocks every pool thread while a DATA session waits for a permit
  would wait behind the gate; the existing suites pass unchanged.
- The musl container numbers are not measured here (no Docker, no musl C
  toolchain); stated in the decision and owed by the dispatcher.
