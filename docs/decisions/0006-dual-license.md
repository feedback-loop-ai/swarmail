# 0006 — Dual license: MIT OR Apache-2.0

**Status**: accepted (operator request, 2026-10-07)

## Ruling

Swarmail is dual licensed under MIT **or** Apache-2.0, at the licensee's
option (SPDX: `MIT OR Apache-2.0`). `LICENSE-MIT` and `LICENSE-APACHE`
carry the texts; `Cargo.toml` declares the SPDX expression; the README
documents the choice.

## Why

The Rust ecosystem's own convention (rustc, cargo, tokio). Apache-2.0 adds
an explicit patent grant for corporate consumers; MIT keeps the barrier at
zero for everyone else. Either alone would exclude someone for no
engineering reason.

## Consequences

- `deny.toml` holds the dependency tree to permissive-only licenses for
  the same reason: openness that imposes nothing.
- Contributions are accepted under both licenses simultaneously (the
  standard dual-license inbound grant); the operator is the sole licensor
  of record for now.
