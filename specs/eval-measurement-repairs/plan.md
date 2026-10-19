# Implementation plan — eval measurement repairs

Implements [spec.md](spec.md) under existing proposed
[0055](../../docs/decisions/0055-agent-eval-harness.md), without amending it.
The chief commits design artifacts only; the ordered implementation and
verification obligations remain in [tasks.md](tasks.md).

## Technical context

Standalone Rust `evals/` crate with its own workspace and lockfile. Reuse
the existing Tokio/HTTP/MCP/SMTP fixture, test helper subprocesses, snapshot
validator and result-row constructor. No new dependencies, production files,
features, root workspace wiring or report-math changes are necessary.
There is no plan template in this checkout; this plan includes the required
technical context, constitution, structure, research, model, contracts,
quickstart and risks directly.

## Constitution check

0001 synchronous accept/store, 0002 subscribe-before-check, 0003 oldest-only
pruning and 0005 ingest extraction are unchanged. Real protocol tests use
0004 inbox routing and follow 0020's fresh state, poles-only chaos and
explicit synchronization rules. Evals stays outside the root coverage report
as already ruled in 0055; no coverage exclusion or floor change is allowed.
Root gates remain byte-identical. The root gate suite does not prove evals,
so implement and reviewers must independently run its own suite until the
deferred CI wiring lands. No new public contract or decision is introduced.

## Research and resolved questions

At `4dac267`, `tasks::score` calls `validate_snapshot` and returns its errors
to `check`, whose error arm already prints harness_error and exits 2.
`runner::run_trial` instead selects budget before consulting that scorer.
The preparation check already rejects Failed/Parked; it cannot catch a
later release/resend. `fixture::record_attempt` parks an initial transient
451 and fails a transport error, while successful retry changes the final
status to Delivered. Consequently a final-status rule covers both real
failure mechanisms without rejecting the smtp-retry positive control.

The matrix currently resolves its binary before entering the product loop
and returns immediately on failure. The existing refused-row constructor
already supplies schema-valid harness errors. Keep that constructor and
traversal, carrying resolution failure into the loop. Preserve input
validation before resolution and leave unrelated lifecycle behavior alone.

The spec records the panel disagreement over `Err` versus an Ok harness-error
verdict. The chosen validator path avoids changing `check`'s exit mapping;
its diagnostic must say unscorable state/harness failure rather than falsely
calling every failed-delivery snapshot malformed.

## Project structure and files

| Path | Planned change |
|---|---|
| `evals/src/tasks.rs` | Add Failed/Parked eligibility rejection after structural validation; update validator/scorer documentation. |
| `evals/src/runner.rs` | Validate available final state before budget, preserving process errors and early preparation refusal; describe validation errors accurately. |
| `evals/src/main.rs` | Carry binary-resolution errors through matrix row accounting; broaden check error wording. |
| `evals/src/report.rs` | Formatter-only repair to the existing assertion. |
| `evals/tests/scoring.rs` | Final-status cases, decoys, satisfied/idle controls. |
| `evals/tests/fixture.rs` | Post-provision transport failure using owned real Swarmail, freeze and replay. |
| `evals/tests/runner.rs` | Post-launch Parked normal/deadline trials and retained-state replay. |
| `evals/tests/common/mod.rs` | Focused modes in the existing test-only agent helper, if needed. |
| `evals/tests/report.rs` | Missing-binary product cardinality and input refusal controls. |
| `evals/README.md` | Clarify final-status precedence, replay scope and valid-product versus invalid-input accounting. |

`evals/tests/proof.rs` and `evals/tests/boundary.rs` supply unchanged control
and dependency/workspace proofs. The only committed design files are
`specs/eval-measurement-repairs/{spec,plan,tasks}.md`. The earlier spec's
precedence remains authoritative and consistent with this clarification;
no additional design-file edit is needed for implementation.

## Data model and contracts

Keep `Snapshot`, `Delivery`, `Attempt`, `Verdict` and `TrialResult` unchanged.
The snapshot validator's `Result<(), String>` now means eligibility for
business scoring as well as structural validity. Reject the first final
Failed/Parked receipt in existing sequence order, including its seq,
identity, flow, message kind and last stage/code when available. Missing
attempt detail must not panic or make a Failed/Parked receipt eligible.
Do not serialize a new classification field or rewrite saved receipts.

Final live selection is: existing run/cleanup error; then absent or rejected
snapshot; then deadline; then `tasks::score`. The scorer keeps validation
for independent replay. Both snapshot rejection paths produce the existing
harness-error result. Do not add an unchecked scoring entry point to avoid
the ordinary path's inexpensive second scan.

Matrix resolution remains once after input validation. Log its error once,
then for each element either construct `refused_row` with labels and a fresh
ID or run the existing successful-resolution child path. Aggregate exits
unchanged (2 over 1 over 0). Do not resolve `current_exe` or launch a child
on the failed-binary path. Ordinary successful batches retain current order,
child handling and stdout ownership.

## Proof approach

Write tests first against current APIs. Pure snapshots cover Failed and
Parked independently, satisfied and idle business states, decoy receipts,
Held and recovered Delivered. Treat a scoring `Err` as the expected refusal;
CLI assertions establish the public JSON and exit-code contract.

For Failed, use the existing fixture rig: provision slow-delivery, stop/reap
its own Swarmail, call public release to completion, freeze and serialize the
actual snapshot for `check`. This tests a real transport failure without
pretending it proves the runner's final selection.

For live precedence, extend the existing external helper with a mode that
sets DATA chaos to 100 through its exported MCP URL and performs public
release/resend after provisioning. Wait for the action response so its
receipt is recorded, then clear chaos. One variant exits normally; a second
remains pending until the actual runner deadline. Retained state must show
Parked/data/451 in both cases. Use a sufficient bounded deadline and explicit
action completion; a timing miss fails the test instead of passing as an
unrelated harness error. Replay both states and inspect actual child exits.

For matrix, use a fresh temporary missing path set only on that subprocess.
Choose two valid tasks, two labels and two repeats; validate eight rows,
ordering, multiplicities, distinct IDs, null/empty statuses and exit 2.
Use an external agent marker if needed to prove no child launch. Preserve
the existing row-less invalid-input tests and report aggregation tests.

Capture red evidence from an isolated archive/copy of `4dac267` with the new
tests applied, using an absolute path to the actual baseline Swarmail binary
and a separate target directory where needed. Never reset or amend branch
history. The new tests must compile and fail on the relevant assertions,
then pass after the fix. Store logs as run-local evidence, not committed
generated artifacts. Full controls still run through real servers, no mocks.

## Quickstart and validation

First run the targeted test binaries from `evals/` with `cargo test --locked
--test scoring --test fixture --test runner --test report`. After the
repair, run `cargo fmt --all` there, review its diff and run these gates
from `evals/`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

The last command includes the proof target: 16/16 oracle passes, 16/16 null
failures with declared idle classes and concurrent isolation. Do not infer
these from root success. Run the unchanged full house inventory from root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
cargo deny check licenses
```

Use the brokkr revision/version pinned in `bundles/self/tooling.json`; install
nothing. Retain the existing fail-closed native-tls absence proof. Gate
inventory is not reduced to the resume framing's shorthand six root gates.
Record exact exits/logs and environmental blockers; do not report green when
a gate could not run. A known baseline evals format failure is a pending
repair, not permission to format production code in this design commit.

Design validation checks nonempty files, `## Acceptance Criteria`, `## Risks`,
task checkboxes and the existing relative decision link/status. Stage exactly
the three design files, sign a plain imperative commit citing #148, verify its
signature and changed paths, and confirm branch/clean tracked state. Later
implementation likewise appends signed commits without pushing or rewriting
`fd66988`/`4dac267`. The chief's required JSON result must identify the
committed paths on success or explicitly disclose any commit blocker and
uncommitted artifacts; result/position files stay gitignored.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Validator name suggests structure only. | Document scoring eligibility and broaden diagnostics; retain the small existing error API. |
| Runner and replay diverge again. | One validator rule, invoked before budget and inside score; real retained-state replay assertions. |
| Expected retry or unreleased mail becomes infrastructure. | Match only final Failed/Parked, with Held and 451-to-250 controls. |
| A server-death test passes without exercising the repair. | Separate transport fixture proof from live chaos proof; assert final Parked receipt, process completion/deadline and replay. |
| Deadline arrives before delivery is recorded. | Await public response, allow a bounded sufficient deadline and assert receipt existence; no sleep-based synchronization. |
| Agents deliberately cause infrastructure errors. | Preserve existing snapshot authority and visible harness-error counts; no intent inference or new taxonomy. |
| Matrix loop changes lose labels or repeat IDs. | Validate the full ordered 2×2×2 product and use existing row/ID constructors. |
| Tests contaminate other trials. | Own real servers and scratch paths, process-local environment overrides, chaos 0/100 and normal cleanup. |
| Gate or signing tools are unavailable in the seat. | Record the blocker; never bypass signing, weaken gates or claim implementation complete. |
| Scope collides with epic #120 batch 5. | Evals-only implementation and unchanged root source/scripts/bundle/CI; diff and boundary tests enforce the separation. |
