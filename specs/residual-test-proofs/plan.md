# Plan: portable SIGTERM delivery proof

## Technical context

One Rust 2024 crate, existing Tokio networking and a real child-process binary
suite. The active code baseline is `90eb2a0333a751576c94d82474e4410040d68d43`;
`fb4c6ea` is design entry after intake. Linux execution is local; macOS is
CI-owned. The defect is a fixture TCP state assumption, not dispatcher code.
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
| `specs/residual-test-proofs/intake-macos.md` | Read-only current framing and historical intake receipts. |
| `tests/binary.rs` | Implementation edits only the named SIGTERM test's fixture, ownership checks, comments and final drop. |
| `Dockerfile`, `tests/common/mod.rs`, `tests/webhook.rs`, `src/api.rs` | Preserve reviewed #191 work byte-for-byte against `90eb2a0`. |
| `.forge/epic192/issue191/` | Ignored evidence and new gate receipts, never staged. |
| `.brokkr/state/design/positions/` | Ignored panel evidence, never staged. |

All other code, Cargo files, decisions, gates, tooling and routing remain
preserved. No new source file or shared socket abstraction.

## Research and resolved choices

Checksum-verified local evidence identifies repeatable macOS timeout for the
bound/unconnected target. Both panel positions recommend a held established
connection and report supporting Linux socket experiments. Neither establishes
Rust regression success or macOS portability. The proposed state is an
exclusively bound client connected to one peer: an unrelated incoming SYN has
no matching established four-tuple and no listener at that local endpoint.
The existing exact refusal test makes this assumption executable on each OS.

Adopt robustness's checks with competing address reuse both disabled and enabled;
prefer simplicity's inline setup, no background keeper and early release of the
setup listener. The two streams own the endpoint through the test. The spec
records each ruling and its cost. Accepting timeouts risks consuming the stalled
delivery's own retry lifetime; accept/close changes failure stage; bind/drop
restores the original race. None is a fallback if CI rejects this fixture.

## Implementation approach

1. Verify the evidence checksums and engine identity; record HEAD, platform and
   initial clean status. Compare scope to `90eb2a0`. Preserve all earlier fixes.
2. In the named test, create a Tokio IPv4 loopback listener on port zero. Create
   `TcpSocket::new_v4()`, call `set_reuseaddr(false)` and explicitly bind to
   `127.0.0.1:0`. Connect this socket to the listener under `HELPER_DEADLINE`,
   then accept the peer under the same deadline discipline. Connect can finish
   into the listener backlog before accept is polled; no spawned task is needed.
3. Read the connected client's `local_addr()` into `refused`. Retain the client
   stream and accepted peer as ordinary owned locals. Drop the setup listener
   after accept; it has a different address. No read/write/shutdown/half-close
   or drop/rebind of the pair is allowed before all final assertions.
4. Preserve the existing deadline-bounded `TcpStream::connect(refused)` probe
   and exact `ConnectionRefused` assertion; update only its fixture wording.
   Keep webhook URL, SMTP delivery, stalled-request synchronization, metric
   observation, SIGTERM, child cleanup, final totals and redaction checks.
5. After existing final assertions, loop over `[false, true]`: create a fresh
   IPv4 TCP socket, set its reuse flag to that value and require bind to
   `refused` to fail with `AddrInUse`. Never listen on a contender. Let each
   contender drop, then explicitly drop the retained stream pair last.
   RAII releases every local resource on panic; no detached task or leak.
6. Run the exact focused test, checking that one test ran, then the full binary
   suite. Review the diff and protected-file equality. Preserve all assertions
   and deadlines; a setup/bind/refusal failure requires repair, not retry loops.
7. Run every local house gate on final code, save fresh receipts and commit the
   scoped implementation unsigned. Hand off to independent verify/review and
   CI's macOS test. Design-baseline receipts do not close implementation tasks.

## Data model and contracts

No application model or wire-contract change. Test state is one ephemeral setup
listener, one explicitly bound outgoing connection, its accepted peer and the
client's local SocketAddr. Lifecycle: bind → connect → accept → release setup
listener → probe refusal → configure real webhook → SMTP → observe request and
exhaustion → SIGTERM → EOF/reap → assert ledger/redaction → assert exclusive
bind → release pair. All setup awaits are bounded. Socket ownership is local,
not global mutable state, port-number guessing or TIME_WAIT dependence.

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
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
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
| Established-client behavior differs on macOS. | Keep exact real refusal and ownership assertions; CI must execute them. Local experiments are feasibility only. A CI failure requires a fixture repair. |
| A parallel listener can claim the target. | Explicit reuse-disabled bind, uninterrupted retained pair, final contender binds with reuse off/on; inspect lifetime and never drop/rebind. |
| Wrong endpoint accidentally selects a listener. | Derive target only from client's local address; exact refusal probe fails if connect succeeds. |
| Added waits turn aborted=1 into exhausted=2. | Prompt refusal, existing deadlines/budgets and unchanged event ordering; do not accept connect timeouts or add sleeps. Existing extreme scheduling sensitivity is not a new latency guarantee. |
| A keeper or half-close loses ownership. | Two local RAII streams, no keeper task or shutdown; explicit release after all checks. |
| Extra resources or setup fail. | Two retained descriptors and a temporary listener, ephemeral loopback ports and bounded setup. Resource exhaustion fails visibly; no retries or allocator framework. |
| Design reopens reviewed fixes. | Compare protected files and unchanged binary regions to `90eb2a0`; preserve earlier snapshot proof and separate webhook test. |
| Historical receipts conceal unfinished repair. | Fresh candidate-bound logs, unchecked new tasks, distinction between baseline gates and repaired-code proof; macOS remains CI-owned. |
