# Implementation plan — eval privacy

Implements [spec.md](spec.md) and proposed
[0062](../../docs/decisions/0062-eval-privacy.md). Design only; future commands
and unchecked tasks are not execution receipts.

## Technical context

Existing standalone Rust 2024 evals workspace, Tokio processes, reqwest, Serde,
Clap and tempfile, Unix process groups. Reuse existing dependencies/locks.
Root exact coverage does not measure evals under 0055; separate proof is mandatory.
No plan template exists here (only a spec template). Required research, data
model, contracts and quickstart are embedded here to retain the three-file dialect.

## Constitution check and phase boundary

Preserve 0001–0005, 0020 test discipline, 0055 lifecycle/results and 0056 unsigned
commits. No root source/manifests/locks, dependencies, gates or recipe changes.
A discovered accepted-ruling contradiction waits for specific operator acceptance.

Chief commits exactly spec/plan/tasks and proposed 0062. The decision index is
therefore not part of this commit. Root test
`store_contract_docs::every_decision_file_has_exactly_one_resolving_index_row`
requires a 0062 row: implementation T1 owns it. Do not claim clean root gates
at this four-file-only boundary or suppress that prerequisite. No index-only
commit follows chief during design. Pinned deterministic validation owns the
design verdict; chief returns only drafted.

## Research and resolved questions

Both positions and intake identify independent inheritance/output leaks. Source
review confirms runner.rs's raw Log tails, Admin error bodies, path-bearing
metadata and unbounded announcement channel, plus main.rs's matrix inheritance,
raw common failure rendering and default Clap/panic output. Cargo inherits stderr.
Existing helpers rely on inherited EVAL_TEST_* and inspect only EVAL_*; replace
them before relying on absence assertions.

Rule for robustness on cleared preparation and bounded typed announcements; for
simplicity on omitting output and keeping existing module owners. The spec fixes
exact runtime/toolchain maps and explicit-input limits. No new CLI/schema is needed.

## Project structure and files

| File | Change |
|---|---|
| evals/src/runner.rs | Shared environment construction, OS-string resolver, directory ownership, safe errors/facts, zero raw retention, bounded announcements/Cargo parsing, safe metadata. |
| evals/src/main.rs | Fallible safe CLI parsing and entry panic hook; matrix resolves programs once, clears workers and drains bounded row protocol. |
| evals/tests/common/mod.rs | Test-owned wrappers/configuration, complete actual-environment probes; no production test-variable exceptions. |
| evals/tests/runner.rs | Environment, lookup/runtime/auth, preparation, independent output, buffer/lifecycle controls. |
| evals/tests/report.rs | Matrix role/publication/cardinality/failure and report compatibility controls. |
| evals/tests/proof.rs and evals/tests/fixture.rs | Adapt helper controls where necessary; preserve real protocol coverage. |
| evals/README.md | Exact maps, preparation inputs, authentication migration and privacy limits. |
| specs/agent-eval-harness/spec.md and plan.md | Dated clarification replacing inheritance/capture/path-metadata promises; preserve unrelated requirements and prior clarification history. |
| docs/decisions/README.md | One proposed 0062 index row before implementation gates. |
| This design and proposed 0062 | Correct discovered ambiguity and record real proof disposition without acceptance claims. |

Inspect oracle.rs/fixture.rs consumers; do not change business logic to accommodate
launch mistakes. No new production module/crate. Adapt other inherited helper test
call sites only when compiler/test evidence requires it.

## Data model and contracts

Keep TrialRequest, TrialResult and secret-free Snapshot semantics. Use small
internal enums/constants for role/stage/category and optional numeric status facts.
Never attach arbitrary String causes to a publishable fact. Map underlying errors
at their operation boundary. Existing diagnostics storage may hold only safely
rendered facts, capped at 64 KiB plus a fixed truncation notice. metadata.swarmail_bin
becomes null; no replacement path or caller-derived provenance string.

Resolved commands hold OS-string absolute programs and unchanged remaining argv.
Resolve at original caller cwd/PATH before changing either. Matrix passes resolved
programs to workers. A directory owner holds per-child home/temp/XDG paths under
0700 scratch in /tmp through draining/cleanup. Apply env_clear before generated
values. No global set_var/remove_var, inherited test controls or secret dictionary.

Preparation resolves tool executables and named locator defaults before private
HOME creation, then constructs the spec's cleared Cargo map. Parse the executable
from Cargo JSON; never guess target/debug or copy preparation variables to trials.
Drain both Cargo streams, retaining no compiler diagnostics.

### Finite output handling

Use 8192-byte read buffers. Readiness lines cap at 8192 bytes: after overflow,
discard through newline before parsing again. The producer parses the owning
service's announcement, validates loopback endpoints and sends at most the first
valid typed tuple through a one-slot/oneshot mechanism. It never awaits a receiver.
No raw-line queue, no readiness channel for agents; keep existing startup deadline
and real HTTP/SMTP health checks.

Cargo records cap at 1 MiB. Discard oversized records through newline, mark failure,
continue concurrent draining and retain only the required matching executable
artifact. Matrix buffers at most one 1 MiB row; reject extra/non-row output or
overflow safely, validate schema and expected task/agent association, and discard
stderr concurrently. No output() accumulation. The spec names the large-label
compatibility cost. These bounds do not describe child allocations or total build
duration; this task adds no new Cargo timeout or broad lifecycle redesign.

Do not change matrix to in-process trials, group ownership, scoring or shutdown
scheduling. Install a safe panic hook once in main, never in concurrent library
trials. Join/error publication also suppresses payloads; hook suppression alone
is not the publication boundary.

### Publication audit

Trace stdout JSON, stderr, diagnostics.log, metadata/result/state JSON and temporary
writes. Trace Admin/MCP/task errors reaching them without changing predicates.
Replace raw interpolation before formatting, including early returns. Use fallible
Clap parse; generated help/version print normally, invalid arguments yield constant
categories. Keep artifact failure's existing result/scoring behavior; emit safe
stage/category and do not imply whole-bundle atomicity or artifacts after every
early error. Matrix never inherits stderr even from the known executable; failures
retain the existing harness_error row/cardinality and numeric/category evidence.

Preserve explicit labels as attribution; document their non-secret requirement.
Do not scan all parent values or rewrite structured state/counts.

## Regression design and proof ordering

First replace inherited helper modes/markers/barriers with test-owned wrapper
configuration or literal synthetic arguments. Capture the launched environment
before a wrapper sets its own helper variables. Inspect full raw OS key/value
membership and require unique execution markers and correct generated values.
For fixture/matrix/current-executable roles, use test-owned launch wrappers or
owner-local test seams observing the actual constructed command, without public
override knobs or mock mail services. Pair constructor assertions with real
subprocess observations; unrelated environment reconstruction is not proof.

Set synthetic parent canaries on individual Command.env, never globally. Include
arbitrary names, non-UTF-8 and synthetic PATH directories. Output tests receive
canaries through independent files/arguments. Scan both CLI streams and every
final/dot-temporary artifact; assert useful categories and outcomes too, so a
helper which never runs cannot pass. Synthetic file authentication proves supported
wrapper behavior without real credentials.

Use barriers/deadlines for pre/post-announcement floods, overflow, noisy agents,
failure paths, matrix and Cargo cases. Functional SMTP/HTTP/MCP tests retain real
servers; controlled subprocess emitters test only launch/stream boundaries. Inline
stable owner-local tests are default; cold-path harnesses follow 0020. No sleep as
synchronization, global env races, probabilistic chaos or checked-in TLS expiry.

Record separate mutation-red/restored-green controls for role inheritance, raw tail
publication and path/error interpolation. Restore mutations before final suites.
Evidence names commands, source/test identities, exits and limits using synthetic
values; never copy historical green verdicts into current proof.

## Quickstart and validation

Default target example from repository root:

```bash
cargo build --offline --locked --bin swarmail
cargo build --offline --locked --manifest-path evals/Cargo.toml
SWARMAIL_BIN="$PWD/target/debug/swarmail" evals/target/debug/swarmail-evals trial --task signup --agent oracle
SWARMAIL_BIN="$PWD/target/debug/swarmail" evals/target/debug/swarmail-evals trial --task signup --agent custom -- /absolute/caller-owned-agent-wrapper
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked
cargo test --manifest-path evals/Cargo.toml --locked --test proof
```

Custom targets use their actual Cargo-reported executable paths. README also
shows automatic preparation and explicit wrapper configuration with synthetic
placeholders, never key values in arguments. Keep existing proof target.

Before implementation completion run every gate in
[the sole house inventory](../../docs/house-rules.md#gates--part-of-every-change),
including exact coverage and selftest, publish/seal proofs, licenses, fail-closed
native-tls absence and exact recorded brokkr compile. Use dispatched pinned proof
resources with checked shims/tooling under 0050. Candidate scripts or this plan
cannot certify themselves. Evals remains separate mandatory evidence reproduced
by reviewers. Missing execution/tooling stays outstanding; no installation in
verify, lowered floor or skipped gate represented as green.

The engine keeps hermetic verification, four independent reviews/security hold
and ship. No push/merge. CI-only chart/history/selftest, Docker, advisory, port-80
and tag/rate/publication obligations remain attributed to CI. This design phase
neither runs nor claims implementation gates.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Closed HOME/PATH breaks ambient auth/custom runtimes. | Exact maps, boundary lookup, synthetic file-auth wrapper proof; deliberate migration without fallback. |
| Cargo cannot discover local toolchain/cache. | Named locators, system/rustup and custom-target tests, safe failure/prebuilt route; local file configuration remains trusted. |
| Omission removes useful failure clues. | Safe role/stage/category and numeric facts, assert their usefulness; accept loss of transcripts. |
| Output flood blocks startup or grows memory. | Fixed buffers/limits, first-valid typed slot, independent drains, overflow tests and structural bound inspection. |
| Panic/Clap/library formatting bypasses policy. | Entry hook, fallible parsing, operation-boundary mapping and independently supplied bad values. |
| Helper controls vanish and tests pass vacuously. | Move configuration, require execution/whole-map assertions, independent output and mutation controls. |
| Concurrent trials share state or environment. | Per-command maps/owned dirs, no global mutation; overlapping real isolation proof. |
| Explicit labels confused with hidden environment. | State limits and preserve attribution; no substring corruption of snapshots/counts. |
| ADR has no index row at design boundary. | T1 owns index addition; no clean-root claim for this four-file draft. |
| Incidental cleanup changes scoring/frozen behavior. | Existing owners and real eval/root suites; no production/gate changes. |
