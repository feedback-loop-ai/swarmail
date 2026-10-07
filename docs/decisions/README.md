# Decision records

Accepted rulings on why swarmail is shaped the way it is. Semantic changes
to a ruling land as a **new** decision with status `proposed`; only the
operator accepts one. Never edit an accepted decision — a correction
amends it with a dated addendum, the way xbox-inference records them.

| # | Decision | Status |
|---|---|---|
| [0001](0001-synchronous-insert-lossless.md) | Synchronous insert: accept == stored | accepted (v0.1 design) |
| [0002](0002-awaits-subscribe-before-checking.md) | `await` subscribes before it checks | accepted (v0.1 design) |
| [0003](0003-oldest-only-pruning.md) | Pruning is oldest-only, never mid-delivery | accepted (v0.1 design) |
| [0004](0004-per-test-inboxes-via-smtp-auth.md) | Per-test inboxes via SMTP AUTH username | accepted (v0.1 design) |
| [0005](0005-extraction-at-ingest.md) | Extraction happens at ingest, not query time | accepted (v0.1 design) |
| [0006](0006-dual-license.md) | Dual license: MIT OR Apache-2.0 | accepted (operator, 2026-10-07) |
| [0007](0007-hermetic-gates-single-model.md) | Hermetic gates + single-model delivery bundle | accepted (operator, 2026-10-07) |

Decisions 0001–0003 are the frozen guarantees: review refuses changes that
weaken them regardless of test outcomes. See
[`docs/house-rules.md`](../house-rules.md) for how they are enforced.
