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
| [0010](0010-exact-count-asserts-and-the-shared-seed.md) | Exact-count asserts and the one shared seed | accepted (ratified at epic #13 wave-4 intake, 2026-10-11) |
| [0011](0011-shim-delete-contract.md) | The shim DELETE contract: an explicit selector or a 400 | **proposed** (awaiting operator) |
| [0012](0012-commit-failure-refusal-and-the-mutation-boundary.md) | A failed commit refuses, and the memory half of a mutation is one unit (amended by 0014, 0015) | **proposed** (awaiting operator) | (Close the three P1 storage-consistency findings of issue #31) |
| [0013](0013-four-model-review-panel.md) | A four-model review panel (qwen-max, opus, astra, fable) | accepted (operator, 2026-10-13) |
| [0014](0014-one-ordered-mutation-boundary.md) | One ordered mutation boundary: commit before visibility | **proposed** (awaiting operator) | (Close the three P1 storage-consistency findings of issue #31) |
| [0015](0015-boundary-failure-contract.md) | The boundary's failure contract: refusal timing, a partial POP3 prefix, poison and the second writer (amends 0012) | **proposed** (awaiting operator) | (Close the three P1 storage-consistency findings of issue #31) |
| [0016](0016-lag-invariant.md) | The live view lags the commit: the boundary's invariant, disk exhaustion as refusal, and a loud busy bound (amends 0014, 0015) | **proposed** (awaiting operator) | (Close the three P1 storage-consistency findings of issue #31) |
| [0017](0017-codex-review-seat-delivery-sandbox.md) | The codex review office's delivery sandbox | accepted (operator delivery repair, 2026-10-13) |
| [0018](0018-shutdown-observed-waits-and-explicit-bounds.md) | Shutdown-observed waits, explicit wire bounds, and the thread fallback's exact scope (amends 0008; amended by 0019, 0024) | **proposed** (awaiting operator) | (Close the six post-P1 findings: lifecycle, contracts, surfaces) |
| [0019](0019-release-tag-integrity.md) | Release-tag integrity: the version publishes only from its own ref, and never twice (amends 0018 §6) | **proposed** (awaiting operator) | (Close the release-tag integrity residual of REVIEW-RESIDUAL-SECURITY) |
| [0020](0020-concurrency-tests-and-the-coverage-gate.md) | Concurrency regression tests and the exact coverage gate: the harness moves, the bar does not (filed as 0011) | **proposed** (awaiting operator) | (Land decision 0020: move persist_tests to the harness, tripwire reserved names) |
| [0021](0021-watcher-channel-lifetime.md) | Watcher channel lifetime: a wake ring released with its inbox's mail (filed as 0010) | **proposed** (awaiting operator) |
| [0022](0022-bounded-data-raw-bytes.md) | The DATA path is bounded and byte-honest; `/raw` serves the stored bytes (filed as 0011) | **proposed** (awaiting operator) |
| [0023](0023-off-worker-commits.md) | Commits run off the async workers: the disk leg is ordering, not thread identity (filed as 0010) | **proposed** (awaiting operator) |
| [0024](0024-shutdown-refusal-at-the-boundary.md) | The boundary refuses to start a commit once stopping: one shutdown policy for mutations (amends 0018) | **proposed** (awaiting operator) | (Close issue #32's slices 1, 5 and 6) |
| [0025](0025-shared-domain-validation.md) | Shared domain validation: a supplied malformed value refuses, an omitted optional value keeps its default (amends 0018) | **proposed** (awaiting operator) | (Close issue #32's slice 2) |
| [0026](0026-deployment-boundary-peers-tls-sizing.md) | The deployment boundary admits peers, mounts TLS, and sizes its own restore (amends 0008) | accepted (operator, 2026-10-14) |
| [0027](0027-living-listeners-bounded-sessions.md) | Living listeners and bounded sessions: the accept loops survive, health tells the truth, shutdown arrives on time (amends 0018/0022) | **proposed** (awaiting operator) | (Close issues #36, #37) |
| [0028](0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md) | Wire-honest POP3 octets, AUTH refusals that change nothing, and the snapshot-not-lock maildrop (amends 0004) | **proposed** (awaiting operator) | (Close issues #42, #43, #44, #45) |

Decisions 0001–0003 are the frozen guarantees: review refuses changes that
weaken them regardless of test outcomes. See
[`docs/house-rules.md`](../house-rules.md) for how they are enforced.
