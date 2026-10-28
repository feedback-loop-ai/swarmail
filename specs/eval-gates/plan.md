# Implementation plan — continue eval gates

## Technical context

Continue candidate `26df727` and inherited design `ae4d2bd` in this checkout.
The service is one Rust crate; `evals/` is an unpublished standalone workspace
outside root packaging and exact root coverage. Existing Tokio/libc and Unix
process groups support Linux/macOS tests. CI and pinned Bash verification use
small Python artifact selectors. No dependency or allowlist change is needed.

No plan template exists under `.specify/templates/` in this checkout. This
plan provides technical context, constitution check, research, structure,
data model, contracts, quickstart and risks directly. The three existing
artifacts remain the single design; no auxiliary research artifact is needed.

## Constitution check

This implements accepted [0055](../../docs/decisions/0055-agent-eval-harness.md)
and [0050](../../docs/decisions/0050-delivery-gates.md), without a new public
contract or ruling. No ADR is declared. Preserve accept==stored,
subscribe-before-check, oldest-only pruning, AUTH isolation, ingest extraction
and the MCP Origin/Host boundary. Production bridge limits in 0039 are not
this runner's stdout contract. No root source, manifest, lock, packaging,
license or coverage-denominator edit is authorized by this plan.

Pinned proof ownership, offline/no-install verify, every house gate, protected
review and security hold remain mandatory. Root test placement follows 0020.
0056 requires unsigned commits and later operator sealing; no push, merge,
issue closure or decision acceptance. New scope that contradicts a ruling
returns for design/operator action rather than being smuggled into a repair.

## Research and continuation evidence

The current and stopped-run intake, both continuation positions, inherited
specs and prior implementation result were read. Inspection confirms:

- CI quality already runs eval fmt and locked all-target clippy; Linux/macOS
  tests capture root artifacts and invoke the full locked eval suite.
- Canonical pinned verify mirrors that sequence offline. Root failures refuse
  before artifact selection; explicit quoted binary propagation crosses its
  command shells. Its fail result may accompany process exit zero.
- `delivery-proof-test.sh eval-dispatch` already owns isolated mutant copies,
  custom target/decoy controls and result receipts. Keep it outside the
  default nested verifier proof to avoid recursion.
- Runner readiness is optional one-shot parsed addresses; agents use log-only
  drain. Startup framing and shared log retention are bounded. The historical
  unbounded server receiver has been removed.
- `retained_server_drains_chatty_stdout_after_one_shot_readiness` waits on
  `spawn_blocking` FIFO open, then `proc.child.wait()`, without deadlines.
  Its later five-second marker deadline cannot diagnose a stall in either.
- Lock tuple negative controls and real MCP shape/decoder controls already
  exist. README and both older specs cite accepted 0055 and wired gates.
- Earlier timeout receipts, later root/eval/coverage success receipts and the
  implementation handoff have different evidentiary weight. Inspect retained
  dispatch evidence under `target/coverage/eval-dispatch-evidence` as well as
  `.brokkr/state/design/validation`; bind it to the actual candidate and pin.
  A missing receipt is outstanding proof, not evidence that code is missing.

### Dated clarification — 2026-10-24

The prior plan described adding gates, replacing the queue and using jq for
selection from baseline `dc1822a`. Those are historical instructions.
Preserve the implemented Python selectors and one-shot design, repair the
local regression, and rerun missing/invalidated evidence. No claim is made
that this local HEAD equals a freshly fetched remote main.

## Project structure and file ownership

| Owner | Continuation work |
|---|---|
| `evals/src/runner.rs` test module | Primary repair: cancellable bounded retained-server rendezvous, child wait, failure cleanup and a stalled-drainer proof. Keep runtime readiness behavior. |
| `evals/tests/runner.rs`, `evals/tests/common/mod.rs` | Reuse real process/lifecycle helpers; change only if the local regression needs them. |
| `.github/workflows/ci.yml` | Preserve existing explicit quality and both OS test wiring, artifact selector, deadlines and publication prerequisites; repair only demonstrated gaps. |
| `bundles/self/scripts/verify-seat.sh` | Preserve pinned offline sequence, artifact provenance and pin/shim refusals; repair only demonstrated gaps. |
| `bundles/self/scripts/delivery-proof-test.sh` | Reuse outer eval-dispatch controls; repair missing evidence recording locally if proved necessary. |
| `tests/delivery_constitution.rs` | Preserve structural wiring/inheritance checks as supplements to actual dispatch. |
| `evals/tests/boundary.rs`, `evals/tests/fixture.rs`, `evals/src/mcp.rs` | Run existing tuple/real-shape/decoder tests; extend only for a concrete uncovered requirement. |
| `evals/tests/proof.rs`, scorer/report/lifecycle suites | Preserve full oracle/null/isolation and infrastructure precedence controls. |
| `docs/house-rules.md` | Keep the sole authoritative gate inventory and current binary prerequisite; no second inventory. |
| `evals/README.md`, `evals/Cargo.toml`, `specs/agent-eval-harness/{spec,plan,tasks}.md`, `specs/eval-measurement-repairs/{spec,plan,tasks}.md` | Check current authority/gate/fixture/signing guidance; fix only remaining contradictions and preserve dated history. |
| `specs/eval-gates/{spec,plan,tasks}.md` | This chief commit reconciles continuation scope; later tasks record proven completion with receipts. |

These are owners, not a mandatory edit list. No rewrite of accepted 0055's
historical body or status is planned. No new shim, helper crate, workflow,
model route, public mode or broad lifecycle platform is needed. Recipe bundles
inherit canonical verify; validate inheritance instead of duplicating it.

## Implementation approach

### 1. Reconcile before editing

Record HEAD, tracked/untracked state and existing scoped changes. Map each
acceptance criterion to current implementation and receipts. Preserve the
chronology of timeout, later success and handoff claims. A usable execution
receipt identifies HEAD plus dirty-diff digest when applicable, command,
selected binary, engine/pinned proof identity, exit, result JSON and decisive
diagnostics. Do not infer successful dispatch from process exit alone.

Retain code already satisfying the contract. Do not import/reset concurrent
worktrees or repeat completed implementation because old tasks are unchecked.
Source changes invalidate affected receipts; final gates still belong to the
final candidate. Keep evidence run-local and preserve original receipts.

### 2. Repair the retained-server proof locally

Keep the child ordering: emit valid readiness, write 131072 bytes, rendezvous,
then emit its final marker. Keep `Proc` owned and alive throughout. Establish
one five-second progress deadline before rendezvous, and apply its remaining
budget to rendezvous, child completion and final-marker observation.

Prefer preserving the FIFO with a nonblocking writer open via existing Unix
`OpenOptionsExt` and `libc::O_NONBLOCK`. Retry only the expected no-reader
condition cooperatively while checking the deadline; errors fail with their
cause. Never use a blocking worker that outlives cancellation. Once the child
has opened its reader after the stdout write, send the small release token
without a blocking wait and close the writer. A cancellable owned async
rendezvous is an acceptable local alternative if it is simpler on both OSes;
it must prove the same causal crossing, not merely observe readiness.

Collect an outcome before asserting so every failure path closes control
handles, signals the owned group, escalates to kill as necessary, waits for
child reaping under an explicit cleanup budget, and aborts/joins owned drain
handles. Preserve a panic-safe group guard as fallback; do not treat a kill
signal alone as proof of reaping. Bound cleanup separately (at most ten
seconds), and give the isolated proof an outer budget of sixty seconds to
retain diagnostics. A cleanup failure remains a failed proof.

Exercise a test-only stalled drainer after readiness in an isolated control.
It must fail for the progress deadline, complete cleanup and leave no owned
child/blocking thread; the outer timeout killing the test does not satisfy
this proof. Then restore normal draining and require the same test to pass
with the final marker and retained-log assertion. No production fault flag,
new dependency, sleep-based ordering or RSS sampling. Keep existing framing,
EOF/error, receiver-drop, chatty-agent and descendant cleanup tests.

### 3. Preserve gate and boundary contracts

Keep the root Cargo test invocation with JSON artifact capture and pipefail.
Select canonical manifest/package identity, target `swarmail`, kind `bin`,
non-test profile and exactly one distinct absolute existing executable.
Preserve useful root diagnostics and refuse root failure before selection.
Override inherited `SWARMAIL_BIN`; never guess `target/debug`, use PATH or
invoke eval's manual fallback in gates. Keep paths containing spaces safe.

Run the existing lock tuple predicate against foreign name/version/source
controls. Run real MCP initialize/tool assertions and existing decoder
negative controls separately: current real-server shape is not permission
to tighten harmless-field tolerance. Preserve full real SMTP/HTTP/MCP
acceptance, 16 oracle passes, 16 null failures with declared idle classes,
and two overlapping isolation passes. Infrastructure remains harness_error.

Use the existing outer eval-dispatch mode for actual pinned fmt/clippy/test
mutants and restored green. Inspect JSON result and named failing gate as
well as exit and logs. Exercise missing/ambiguous/wrong-package/wrong-target/
harness-only/relative/non-executable artifacts and failed root tests, plus
custom target/decoy controls. Disposable artifact inputs can test selector
refusals, but do not substitute a fake engine/Cargo or modified algorithm for
actual dispatch. Keep original tamper/engine identity proofs. If proof cannot
retain sufficient receipts, correct its existing recorder, not the policy.

### 4. Reconcile current documentation and handoff

Both older specs and README must describe already accepted 0055, active gates
and the test-build prerequisite, while preserving deferred broker driver,
unconfined execution, HTTP ICS and HTTP reply limitations. Keep dated history
and superseding 0056 signing authority. The fixture exists at
`evals/src/fixture.rs` and CLI exposes `swarmail-evals fixture`.

Exact erroneous external #149 wording is unavailable in inspected materials.
Keep the operator follow-up with canonical replacement; do not fabricate a
source quote, external edit or completion. Only demonstrated repository
contradictions warrant further documentation edits.

## Data model and contracts

No persistent/public schema changes. `Proc` retains optional one-shot
`(SocketAddr, SocketAddr)` readiness; agents allocate no readiness queue.
`StartupLine` retains at most 8192 bytes including newline and a discard flag.
Overflow discards through newline; suffixes are never reparsed as new lines.
Delivery or receiver closure drops framing; draining never awaits a consumer.
`Log` retains at most 65536 bytes after append, with finite chunk/decoding
allocation overhead. These are output-retention bounds, not whole-process
memory or hostile-agent confinement promises.

Evidence records remain run-local: candidate identity/diff, pin identity,
command, selected binary, exit/result and diagnostic. This describes required
facts, not a new public result schema or proof registry.

## Quickstart and validation

From the checkout root, run the targeted runner control and eval fmt/clippy.
For full eval tests use the root-test capture/selection sequence already in
CI and pinned verify; do not manually substitute a stale executable. Targeted
commands include:

```bash
cargo test --manifest-path evals/Cargo.toml --locked retained_server_drains_chatty_stdout_after_one_shot_readiness
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
SWARMAIL_BIN="$candidate_bin" cargo test --manifest-path evals/Cargo.toml --locked
bash bundles/self/scripts/delivery-proof-test.sh eval-dispatch
```

`candidate_bin` is set only by the successful root-test artifact selection
above. The outer dispatch proof uses the recorded candidate-local engine;
normal verification runs pinned resources, not candidate-selected helpers.
Run the complete [house inventory](../../docs/house-rules.md#gates--part-of-every-change),
design selftest, delivery tamper/identity proofs and any affected recipe
compiles with the digest-matched `.forge/bin/brokkr`. Keep exact coverage,
coverage selftest, publish/seal proofs and fail-closed native-TLS absence.
Missing tools/cache or failed execution stay outstanding; verify installs
nothing. CI-owned chart/history/selftest, Docker stop/kill smoke, network
RustSec, tag rate proof and privileged port-80 checks remain distinct.

This chief stages exactly these three design artifacts, commits unsigned and
checks committed HEAD with `bundles/self/scripts/design-check.sh check`.
Leave that commit last for the sequence's pinned final check. Implementation,
full gate certification and protected review remain subsequent obligations;
drafting does not claim them complete.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Regression hangs before diagnostic or during runtime shutdown. | Nonblocking/cancellable rendezvous, progress/cleanup deadlines, cleanup before assertion and stalled-drainer red/restored-green control; local test complexity. |
| A timeout kills the proof but leaks children or masks its cause. | Record intended deadline diagnostic, explicitly reap owned children and finish tasks; outer timeout is a failed proof, not success. |
| Old receipts certify changed code or conflict with earlier timeouts. | Preserve chronology and candidate/diff/pin identity, rerun missing/invalidated proofs and final gates; added runtime. |
| Stale binary or hidden root failure gives false green. | Existing canonical artifact selection, pipefail, negative artifacts and custom-target/decoy controls. |
| Candidate helpers choose verification or proof recurses. | Pinned canonical owners, explicit outer dispatch, actual JSON verdicts and tamper/engine controls. |
| Fixing the test expands into a lifecycle or protocol redesign. | Keep runtime one-shot/log contracts and MCP tolerance; only demonstrated gaps justify edits. |
| Root green is confused with eval or CI-only coverage. | Standalone suite and complete inventory remain mandatory; retain distinct proof ownership and limits. |
| Offline prerequisites are unavailable. | Refuse and record exact missing proof; no installation or skip in verify. |
| Documentation fabricates authority or #149 completion. | Cite recorded acceptance, retain historical prose and explicit external operator handoff. |
| Concurrent eval work is overwritten. | Scoped checkout edits, deliberate overlap reconciliation, no reset/import and rerun affected proof. |
