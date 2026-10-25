# Plan: portable SIGTERM delivery proof

## Technical context

One Rust 2024 crate, existing Tokio networking and a real child-process binary
suite. The active code baseline is `94b8ae15f0b3515d5860266f0988a2a4b75f15b0`;
`f6a83e6` is design entry after continuation intake. Linux execution is local;
macOS is CI-owned. The defect is the reuse-enabled bind expectation and its exclusion claim.
No new dependency, schema, public API, configuration or production behavior.

No plan template exists under this candidate's `.specify/`; this plan supplies
the required technical context, constitution check, structure, research, data
model, contracts, quickstart and risks directly. No separate research, model
or contract file is needed for this single test fixture.

## Constitution check

Preserve frozen 0001–0003, ingest extraction 0005 and real protocol isolation.
Keep the wire test in `tests/binary.rs` under 0020; use observed events and
bounded waits under 0018. Preserve 0041/0048 ledger arithmetic, warning causes,
redaction, shutdown order, retry count and budgets. No cfg seam, coverage
exclusion, harness relocation or denominator change. Use pinned proofs (0050),
unsigned commits (0056) and unchanged candidate recipe routing (0057/0058).
No semantic amendment or decision acceptance is part of this work.

## Project structure and files

| Path | Role and permitted change |
| --- | --- |
| `specs/residual-test-proofs/{spec,plan,tasks}.md` | Exactly the chief's three design-commit files; reconcile current scope and retain dated provenance. |
| `specs/residual-test-proofs/intake-macos-continuation.md` | Read-only current framing and historical intake receipts. |
| `tests/binary.rs` | Implementation edits only the named SIGTERM test's contender check and directly related comments; setup, refusal probe and final drop stay unchanged. |
| `Dockerfile`, `tests/common/mod.rs`, `tests/webhook.rs`, `src/api.rs` | Preserve reviewed #191 work byte-for-byte against `94b8ae1`. |
| `.forge/epic192/issue191/` | Ignored evidence and new gate receipts, never staged. |
| `.brokkr/state/design/positions/` | Ignored panel evidence, never staged. |

All other code, Cargo files, decisions, gates, tooling and routing remain
preserved. No new source file or shared socket abstraction.

## Research and resolved choices

Checksum-verified `review-78a81fc6.md` identifies an unconditional
reuse-enabled `AddrInUse` assertion that is valid on Linux but fails on BSD/XNU
against this established endpoint. Its independent finding 2 limits macOS
exclusion against parallel reuse-enabled listeners. The connected-client target
and exact refusal probe remain sound by the supplied reasoning; actual macOS
execution belongs to CI. No new platform experiment is needed for design.

Both current positions recommend removing the reuse-enabled path and keeping
one reuse-disabled contender. Adopt that choice. Reject platform-specific bind
success/failure expectations, which demonstrate kernel policy without improving
the shutdown proof. The spec records the loss of the stronger exclusion claim
and the remaining interference risk. Keep the inline fixture and every existing
observation; no helper, allocator or production seam is justified.

## Implementation approach

1. Verify packet checksums and engine identity; record HEAD, platform and clean
   status. Compare implementation scope to `94b8ae1`; preserve earlier fixes.
2. Preserve the existing temporary loopback listener, explicitly bound
   reuse-disabled IPv4 client, bounded connect/accept and accepted peer. These
   are already implemented; no fixture reconstruction is authorized.
3. Preserve the client's `local_addr()` as `refused`, dropped setup listener
   and both retained stream locals through all assertions. Correct the opening
   comment: lifetime excludes reuse-disabled binders, but not necessarily
   reuse-enabled macOS listeners, including parallel Tokio listeners. Do not
   read, write, half-close, shut down or release the pair early.
4. Preserve the exact bounded `TcpStream::connect(refused)` refusal probe,
   webhook address, SMTP, stalled-request and exhaustion observations, SIGTERM,
   EOF/reap, clean exit, message formatting, exact ledger and redaction checks.
5. Replace the final `[false, true]` loop with one fresh IPv4 `TcpSocket`.
   Explicitly set reuse false, require `bind(refused)` to fail with exactly
   `AddrInUse`, and never listen. Correct its comment/diagnostic to describe
   only reuse-disabled exclusion. Keep `drop((reservation, peer))` last.
   No singleton loop, platform conditional or replacement reuse-enabled check.
6. Run the exact focused test (one selected pass) and full binary suite. Inspect
   the diff for only the contender/comment changes and compare protected files.
   Cross-check active spec/plan/tasks and comments for honest macOS limits.
   Setup refusal is a point observation; final exclusion does not establish
   every retry's failure stage or continuous protection from all listeners.
7. Run all local gates on the repaired code, retain fresh candidate-bound
   receipts and commit scoped implementation unsigned. Independent verify/review
   and CI's macOS execution remain required. Design-baseline receipts do not
   close any implementation task.

## Data model and contracts

No application model or wire-contract change. Test state is one ephemeral setup
listener, one explicitly bound outgoing connection, its accepted peer and the
client's local SocketAddr. Lifecycle: bind → connect → accept → release setup
listener → probe refusal → configure real webhook → SMTP → observe request and
exhaustion → SIGTERM → EOF/reap → assert ledger/redaction → assert reuse-disabled
bind rejection → release pair. All setup awaits are bounded. Ownership is local,
not global mutable state, port-number guessing or TIME_WAIT dependence. Retained
streams establish lifetime and the final bind check observes reuse-disabled
exclusion only. They do not establish exclusion of reuse-enabled binders or
listeners on macOS, including parallel Tokio listeners, or fully close that
reuse race.

The final ledger remains delivered=0, dropped=0, lagged=0, exhausted=1,
aborted=1. The default-level stop event appears once, with exactly one exhaustion
warning containing a cause and no test credentials. SMTP acceptance remains
independent of webhook success. This proves orderly shutdown accounting, not
crash delivery durability or a response after forced process termination.

## Quickstart and verification

```bash
(cd .forge/epic192/issue191 && sha256sum --check --strict SHA256SUMS)
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo test --locked --test binary sigterm_with_a_delivery_in_flight_prints_the_final_counts -- --exact
cargo test --locked --test binary
```

Require exactly one selected passing test in the focused invocation. Record the
candidate, Linux platform, commands and exits. Compare the engine digest above
with `bundles/self/tooling.json` before running it. Run the entire
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
```

The charter remains the inventory authority. Verify checks public shims and
executes the corresponding pinned `bundles/self/scripts/` copies offline,
installing nothing. Require zero missed lines, branches and functions and a
successful coverage exit; a printed summary followed by timeout is insufficient.
Also run pinned `design-check.sh selftest` and `delivery-proof-test.sh` as in
the current verify seat. Bundle compile must have no policy-sweep warning.

Run `cargo tree --workspace --locked --offline --invert native-tls`, requiring a
nonzero exit and an exact diagnostic line that package ID specification
`native-tls` did not match any packages (Cargo's backtick or single-quote
spelling, as recorded in intake). Success means the forbidden package is
present; any different error fails proof. Do not equate arbitrary Cargo failure
with absence. Store logs in ignored evidence with completed exits.

After the chief's unsigned three-file commit, run
`bash bundles/self/scripts/design-check.sh check .` and verify HEAD touches only
those three paths in one spec directory with a clean worktree. The sequence's
final check still owns its result; no further commit follows the chief commit.
Baseline gates exercise existing Linux code only. The later repair must repeat
its required proofs. CI owns actual macOS, chart/history, Docker smoke, RustSec,
privileged port-80 and release-only checks; the operator owns live settings,
sealing and publication authority. None is claimed locally.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Platform behavior is overclaimed again. | Keep the exact refusal and reuse-disabled bind check; CI owns macOS execution. No local pass certifies macOS or all concurrent schedules. |
| A reuse-enabled macOS listener can overlap the target. | Retain the pair and reuse-disabled bind check, but explicitly accept the limited proof: neither this check nor setup refusal excludes reuse-enabled binders/listeners, including parallel Tokio listeners. Existing assertions do not detect every interference or prove every retry fails at connect. Full race closure is outside this task; separately scope any CI-observed collision. |
| Wrong endpoint accidentally selects a listener. | Derive target only from client's local address; exact refusal probe fails if connect succeeds. |
| Added waits turn aborted=1 into exhausted=2. | Prompt refusal, existing deadlines/budgets and unchanged event ordering; do not accept connect timeouts or add sleeps. Existing extreme scheduling sensitivity is not a new latency guarantee. |
| A keeper or half-close loses ownership. | Two local RAII streams, no keeper task or shutdown; explicit release after all checks. |
| Extra resources or setup fail. | Two retained descriptors and a temporary listener, ephemeral loopback ports and bounded setup. Resource exhaustion fails visibly; no retries or allocator framework. |
| Design reopens reviewed fixes. | Compare protected files and unchanged binary regions to `94b8ae1`; preserve earlier snapshot proof and separate webhook test. |
| Historical receipts conceal unfinished repair. | Fresh candidate-bound logs, unchecked new tasks, distinction between baseline gates and repaired-code proof; macOS remains CI-owned. |
