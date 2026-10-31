# Implementation record — near misses on a timed-out wait

Companion to [spec.md](spec.md), [plan.md](plan.md) and [tasks.md](tasks.md).
Decision [0106](../../docs/decisions/0106-near-miss-timeouts.md) stays
*proposed*; this work is compatible under 0045 §8 and accepts nothing.

## Binding (T1)

Built on the design commit `f8393f6` (tree `7d591bb`), whose parent is main
`d892acb`. `src/store.rs`, `src/smtp.rs`, both wait owners (`wait_for`,
`wait_for_report`, `wait_focused`), every captured March fixture and
everything under `evals/tests/data/` have no diff. The captured March
comparisons in `tests/mcp_revisions/structured.rs` pass unchanged.

## What landed

- `src/mcp/near_miss.rs` (new): the three caps, `clip` (via
  `floor_char_boundary`), `recipients`, `Fields::missed` (single-field
  projections of the caller's `Filter` through the one `Filter::matches`),
  bigram `nearest`, `derive` and the closed `schema()`.
- `src/mcp/focused.rs`: `List` shared (`with_cap`, default unchanged); the
  focused timeout attaches the object; a second closed `timed_out` variant.
- `src/mcp.rs`: the report wait's timeout arm attaches the object; a third
  closed `oneOf` variant for `swarmail_wait_for_email`.
- One departure from the plan's sketch: `near_misses` first takes the cheap
  `Store::count` and answers `None` on an empty or missing inbox without
  leaving the poll. `tests/mcp.rs::zero_timeout_finishes_on_the_first_dispatch_poll`
  pins that a zero-deadline wait over an empty inbox finishes on the
  dispatch's first poll, and an unconditional `Store::offload` broke it.
- The schema carries no `maxLength`: the repository's finite evaluator
  refuses that keyword (`evaluator_selftest`), so the 256-byte bound is the
  producer's, proven by the inline clipping tests.
- Tests: 13 inline in `near_miss.rs`; wire shape, empty-inbox bytes and the
  AUTH tenant boundary in `tests/mcp.rs`; focused timeouts in
  `tests/focused_waits.rs`; revisions, transports and schema negatives in
  `tests/mcp_revisions/structured.rs`; exact-shape updates in
  `tests/mcp_revisions/focused.rs`; the three scripted controls in
  `evals/tests/proof.rs` (`near_miss_controls_recover_after_a_timed_out_wait`).
- README, `src/static/llms.txt` and `evals/README.md` describe the member and
  state the metric as not measured.

## Gate receipts (locked, offline)

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` (root and `evals/`) | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` (root and `evals/`) | clean |
| `cargo test --workspace --locked` | all pass |
| `cargo test --locked` in `evals/` | all pass, including the new proof |
| `bash scripts/coverage-gate.sh` | exact 100/100/100: lines 19563/19563, branches 1340/1340, functions 1997/1997 |
| `bash scripts/coverage-gate.sh selftest` | pass |
| `bash scripts/publish-guard-test.sh` | OK |
| `bash scripts/seal-branch-test.sh` | 7 cases and 10 mutants passed |
| `cargo deny check licenses` | licenses ok; `native-tls` absent from the tree |
| `brokkr compile --bundle .brokkr/recipes/self` | **not run**: the pinned brokkr binary is not provisioned in this seat's sandbox and nothing may be installed. No file under `.brokkr/` changed. |

CI-owned contexts (helm chart, docker smoke, RustSec audit, macOS tests,
the port-80 bridge exchange, the pinned bundle job) are owed to CI.

## Not measured

The rate of recovery after a failed wait. It needs paid live-model trials,
which this run does not hold. The scripted controls prove recovery is
mechanically possible from the timed-out answer alone and that acting on
`nearest_subject` in the sibling trap still loses as `wrong_message`.
Slow-delivery's goal already prescribes the release, so recovery there is not
attributable to this feature.
