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
| [0008](0008-kubernetes-deployment-contract.md) | The Kubernetes deployment contract (Helm) | **proposed** (awaiting operator) |
| [0009](0009-agent-council-hybrid.md) | The agent council: a hybrid machine on the speckit dialect | **proposed** (awaiting operator) |
| [0010](0010-off-worker-commits.md) | Commits run off the async workers: the disk leg is ordering, not thread identity | **proposed** (awaiting operator) |
| [0010](0010-exact-count-asserts-and-the-shared-seed.md) | Exact-count asserts and the one shared seed | accepted (ratified at epic #13 wave-4 intake, 2026-10-11) |
| [0010](0010-watcher-channel-lifetime.md) | Watcher channel lifetime: a wake ring released with its inbox's mail | **proposed** (awaiting operator) |
| [0011](0011-concurrency-tests-and-the-coverage-gate.md) | Concurrency regression tests and the exact coverage gate: the harness moves, the bar does not | **proposed** (awaiting operator) | (Land decision 0011: move persist_tests to the harness, tripwire reserved names)
| [0012](0012-commit-failure-refusal-and-the-mutation-boundary.md) | A failed commit refuses, and the memory half of a mutation is one unit (amended by 0014) | **proposed** (awaiting operator) | (Close the three P1 storage-consistency findings of issue #31)
| [0014](0014-one-ordered-mutation-boundary.md) | One ordered mutation boundary: commit before visibility | **proposed** (awaiting operator) | (Close the three P1 storage-consistency findings of issue #31)
| [0013](0013-four-model-review-panel.md) | A four-model review panel (qwen-max, opus, astra, fable) | accepted (operator, 2026-10-13) |

Decisions 0001–0003 are the frozen guarantees: review refuses changes that
weaken them regardless of test outcomes. See
[`docs/house-rules.md`](../house-rules.md) for how they are enforced.
