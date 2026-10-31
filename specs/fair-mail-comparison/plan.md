# Implementation plan — fair mail comparison

Date: 2026-10-28. Implements [spec.md](spec.md) and proposed
[0091](../../docs/decisions/0091-fair-mail-comparison.md).

## Technical context

Root is one Rust 2024 crate with real-protocol tests and three existing benches.
The unpublished standalone evals crate has its own workspace/lock; accepted 0055
requires its dependency versions to exist in the root lock. Existing owners are
runner, fixture/mail, mcp/oracle, baseline, tasks, results, provenance, artifacts
and report. Use existing locked Tokio/reqwest/serde/tempfile/libc for the root
driver. No new crate/service. Competitor binaries are pinned experiment inputs,
not dependencies smuggled past license review.

No plan template exists in this checkout; the available spec template describes
another workspace layout. This plan supplies technical context, constitution check,
structure, research, data model, contracts, quickstart and risks within the four
authorized chief artifacts.

## Constitution check

Preserve accepted 0001–0005, 0020 proof placement, 0050/0072 pinned executed proof,
0055/0063/0071/0075/0076, 0056 unsigned commits and the publication boundary. No
historical spec rewrite or server fast path, gate bypass, coverage exclusion,
allowlist widening, extra root member, routing rewrite or engine re-pin.

The chief commits only spec/plan/tasks and proposed 0091. Implementation T1 adds
one proposed index row; any inventory failure until then remains outstanding.
0091 is next after this candidate's 0090, not a global reservation. Parallel
numbering and 0090 require operator reconciliation. Compatible work proceeds;
any contradiction with an accepted ruling waits for the operator.

## Research and prerequisite ledger

| Item | Evidence and next action |
|---|---|
| Framing | Four packet checksums, exact captured issue, intake and both positions read. No fresh GitHub observation or web-search capability. No tracked links into ignored evidence. |
| Eval build | Conflict markers in results.rs, artifacts.rs, provenance.rs and identity/provenance tests. Coordinate repair in current owners preserving strict v2, traps, privacy, lifecycle and every assertion; no wholesale legacy-side selection. |
| Coordination | #201 full-suite host-wide proof remains absent. A parent-test lock is insufficient. Require independent processes/checkouts and default-threaded full-suite receipts, with #201 macOS CI scope. |
| Baseline/MCP | Genuine #151 baseline, six-client authenticated smoke and final #152/#153 reconciliation remain outstanding. Current mcp.rs assumes 2025-03-26 text results. Record observed negotiated revisions/output, not assumed full conformance. |
| Competitors | Implementation must capture official versioned repository/release/license/API/setup sources and exact artifacts. This design claims no verified versions/capabilities. Unavailable sources/artifacts block collection, not independent code work. |
| Delivery | Inherited HIGH review-integrity references are incomplete in framing; obtain originals for protected review without inventing findings. #219 stays separately tracked. Local measurements replace neither pinned published engine nor protected review proof. |

Record research in docs/evals/README.md and the source lock/protocol before
measurement. Exact upstream API/MCP mapping must be reviewed before agent scores.
#151 currently plans 16 tasks, six declarations, five repetitions and 32 controls;
the actual genuine frozen baseline determines the comparison product. Planned
cardinality is not execution evidence.

## Project structure

| Files | Responsibility |
|---|---|
| benches/mail_compare.rs; benches/mail_compare/{protocol,wire,records,targets}.rs | One executable with concrete targets, bounded wire clients/lifecycle, protocol/record validation and arithmetic; independently buildable without evals. |
| benches/mail_compare/mailpit_mcp.rs | One thin upstream adapter implementation with local executable mode, later launched by existing eval owner; no fixture/scorer/admin state. |
| Cargo.toml | Register harness=false bench; preserve workspace/package includes. No new dependency planned. |
| tests/mail_compare.rs; tests/common/ | Real owned-server and evidence/lifecycle tests exercising bench modules with independent expected results. Missing competitors fail the explicit comparison suite. |
| scripts/compare-mail.sh | Thin preflight/dispatch wrapper, no second supervisor; delivery modes never launch paid work. |
| docs/evals/README.md, protocol.json, sources.lock.json | Method, source/license pins, prerequisites/status, reproduction and frozen protocol before measured samples. |
| docs/evals/runs/<run-id>/ | Only actual admitted safe manifest/raw/report/cost/index artifacts, no fake baseline/final placeholders. |
| evals/src/{runner,identity,provenance,artifacts,results}.rs | Reconcile conflicts; concrete target choice and comparison typing/retention within existing ownership; native v2 remains unchanged. |
| evals/src/{mail,fixture,mcp,oracle}.rs | Explicit transport/control-client mapping without business scenario or scorer changes. |
| evals/src/comparison.rs; main.rs, lib.rs, report.rs, baseline.rs | Explicit comparison envelope/plan validation and entry point; reuse baseline orchestration, scoring and arithmetic, no alternate runner. |
| evals/tests/comparison.rs and existing proof/runner/identity/provenance/boundary/scoring/report/baseline/wrappers suites | Real integrated control/refusal/replay proofs and all existing assertion preservation. |
| README.md, evals/README.md, docs/decisions/README.md | Reproduction, eventual evidence-citing claims, proposed decision inventory. |

No comparison change to root src/ is planned. Bench code is experiment infrastructure,
not production code relocated outside coverage. If reusable production behavior is
needed, place it under src/ with full exact coverage. Preserve the independently
tested standalone eval boundary and root-locked dependency versions.

## Implementation and frozen profiles

Build local wire/adapter modes without importing evals. First verify exact upstream
pins and perform untimed setup/correctness probes. Then commit a complete protocol.
The following are proposed starting profiles, subject to source-verified setup and
a committed revision before measurement, never tuning after seeing comparative scores:

- Smoke: one repetition, 12 authored plain/HTML/multipart messages per target plus
  present/future/empty waits. Label smoke, not headline performance evidence.
- Throughput: six repetitions in the six permutations of target order, 100 warmup
  messages in separate owned state, 3,000 measured messages at concurrency 1/10/100.
  Fresh and reused SMTP connections are separate profiles. Closed-loop workers
  have one outstanding delivery; record scheduling delay, claim no open-loop rate.
- Stress: same bounded corpus/count and six-order design at 1,000 sessions. Refusal/
  saturation stays in results; no unbounded load, retry or host tuning.
- Wait per target/order repetition: 100 present, 100 future, 20 empty attempts.
  Present/future deadline 30,000 ms; empty deadline 1,000 ms. These are method limits,
  not promised latency. Poll interval/streaming must be source-justified and frozen
  during research; missing choice refuses collection.
- Resource starting profile: equal 2 CPU/2 GiB service budget, same bounded controller
  budget, file-descriptor ceiling, process/session limits and finite profile/cleanup
  deadlines. Record and verify enforcement through the selected runtime. Unsupported
  enforcement refuses that profile; other environments need a separate committed
  profile, never unenforced limits described as enforced.
- Storage: matched memory-only primary profile only if all pinned targets actually
  support it; otherwise declare the actual profile and unsupported equivalence.
  Exact-count retention accommodates the entire workload. Durable/cap-churn results
  are separate, configured and tested, not inferred.

The protocol resolves every remaining numeric ceiling, corpus byte hash, seed,
wait mechanism and runtime before launch; the validator refuses placeholders.
Verify identities/fidelity outside the timed send window and separately time verified
completion. Use controller monotonic timestamps, never cross-process wall-clock
subtraction. Keep all profiles/configurations separate in reports.

Implement only reviewed general upstream API translations. The wrapper dispatches
an executable lifecycle owner; avoid unmanaged shell background children. The eval
runner later owns adapter and server through its existing process-group cleanup.

After compatible eval repair, add concrete target selection to existing execution.
Share internal execution/scoring facts and strict common validators; comparison
emission is an explicit new mode. Fixture transport changes only connection/settings
and documented identity selection. Unsupported chaos or isolation remains unsupported,
not a replacement task or fabricated observation. Baseline Swarmail mode is preserved.

## Data model and contracts

New records are closed/versioned typed data with duplicate/unknown-field refusal,
numeric/size/depth/path bounds and required fields. Existing catalogue/snapshot/result
business version stays 2.

| Record | Required content |
|---|---|
| Protocol v1 | Complete products/applicability and evidence; task/scorer/adapter/MCP hashes, corpus/seed/time, counts/order/profiles, resources/storage/caps/deadlines, statistics, controls and baseline/final association. |
| Source lock v1 | Concrete target, official source/document/release identities, artifact/platform digests, license/runtime/version and public fixed launch settings. |
| Manifest v1 | Unique run/mode, committed protocol/lock hashes, executable bytes/known-or-unknown source, harness/adapter revision/dirty facts, OS/kernel/CPU/runtime/toolchain, resources, planned coordinates and prelaunch time. Recheck identities at launch/finalization. |
| Wire sample v1 | Run/profile/target/repetition/sample, message ID, typed SMTP disposition, monotonic events/units, bytes/counts/API requests, observation/error/timeout/censoring, verification/cleanup facts. Absent events have finite unavailable reasons. |
| Comparison trial envelope v1 | Explicit comparison_schema_version discriminator, common v2 business execution/provenance facts, Swarmail/Mailpit target union with truthful settings, adapter identity, method hash and coordinates. No fictitious native TrialResult for Mailpit. |
| Index/report v1 | Full planned/executed/unsupported/unstarted accounting, artifact hashes/lengths, cleanup/completeness, configuration strata, unchanged eval formulas, wire quantiles and failures. |
| Cost evidence | Actual accounting source/basis/currency/period, safe usage/charge provenance, pricing identity where used, unavailable reasons; estimates separately labeled. |

Refactor common internal facts only as needed. Native readers still reject foreign
envelopes and historical/mixed v1 rows. Comparison readers reuse snapshot/scorer and
common strict validation, then validate target-specific settings. Replay validates
all bytes/hashes/coordinates before scoring and never executes saved commands.
No migration, default concurrency or ninth business failure class.

Keep existing success over pass+fail with 95% Wilson interval, passes/attempts,
infrastructure counts and non-error wall-time formulas. Applicability coverage is
additional accounting. Wire nearest-rank quantiles show censored counts separately.
Retain incomplete raw data even when full-report admission refuses.

## Quickstart and command behavior

Interfaces to implement, not existing commands or executed receipts:

```bash
bash scripts/compare-mail.sh local --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json --output /private/new-local-run
bash scripts/compare-mail.sh controls --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json --output /private/new-control-run
bash scripts/compare-mail.sh evidence --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json --baseline /private/genuine-baseline --final /private/genuine-final --receipts /private/prerequisites.json --output /private/new-comparison
```

From documented preinstalled clean prerequisites, each is one root-level command.
Build offline/locked once, verify pins, dispatch owned execution and derive reports.
No arguments prints usage and launches nothing. Local builds root bench only.
Controls checks eval prerequisites and adds free full-product controls. Evidence
validates genuine authorized input/receipt sets before local collection/derivation;
it never launches paid agents. Private path values are absent from public records.

For a future separately authorized operator, the same entry point also exposes
measure --baseline /private/genuine-baseline --config /private/selected-wrappers
with the same protocol/source/receipt/output arguments. This mode validates the
genuine baseline, contracts and gates, executes fresh free controls, then delegates
the final Swarmail/Mailpit product to existing eval/baseline owners and runs the
wire workflow and dossier derivation. No second scheduler, baseline overwrite or
implicit paid default. The operator must separately authorize this invocation;
this delivery never executes real paid selections. Its dry-run neither opens
private credential files nor launches trials. Prove the orchestration with existing
free inert command fixtures, not authenticated providers. Thus one entry point
supports both safe replay and the later authorized full measurement workflow.

Exit 0: selected mode completed declared checks/accounting. Exit 1: complete measured
business/workload failures with valid report. Exit 2: prerequisite/operational/
interrupt/cleanup/accounting failure; it dominates. Expected empty-wait timeouts and
declared null failures count as successful control checks. Mode/complete artifact
validation, not exit code alone, admits full evidence.

## Verification

Run every gate in the single [house inventory](../../docs/house-rules.md#gates--part-of-every-change),
locked wherever applicable. Verify engine bytes against tooling.json and shims against
the pinned manifest; execute pinned coverage/selftest, publish/seal and delivery
proof scripts. Require exact zero missed lines/branches/functions and fail-closed
native-tls absence (unrelated cargo-tree failure is not proof). Verify installs nothing.

Also run root burst/p2/rate/mcp/compat and all existing persistence/store/webhook/SMTP/
TLS regressions, cargo bench --locked --no-run and cargo bench --locked, standalone
eval fmt/locked all-target clippy/full default-threaded tests, and one-command real
three-target benchmarks plus integrated controls. New-client tests use real owned
servers, authored MIME and bounded invalid/oversized local responses, cancellation
and cleanup; no attack reproduction or exploitation tooling.

Run full suites even when a prerequisite is expected to fail, retaining actual failure.
CI owns charter network/RustSec/chart/Docker/platform/port-80 proofs; #201 owns macOS
eval and shared-host coordination proof. Local measurements prove neither engine
behavior nor protected review.

Preserve requested Terra 6.1 intake/Opus 5.5 implementation and inherited five-role
mixed panel: distinct Astra correctness/guarantees, independent Claude Fable, official
DSH Flash security and Spark GLM-5.3 Flash tests; protected read-only boundaries,
worst verdict, no unauthorized fallback, hard security hold. This design dispatches
no extra reviewers or route changes.

## Risks

| Risk | Mitigation and remaining limit |
|---|---|
| Eval conflict syntax or missing coordination | Continue independent root work; compose accepted behavior and all tests, require actual #201/full-suite receipts. Integrated evidence stays blocked. |
| Capability gaps shrink comparison | Freeze full catalogue and source-backed applicability; separate full/common denominators. Unsupported semantics keep full same-suite AC open. |
| Biased adapter | General source-verified operations, justified waits/pagination and measured overhead, real identity/parity controls. |
| False target provenance | Explicit comparison union/envelope and native-reader refusal, same scorer/common validators, proposed contract decision. |
| Client saturation/storage mismatch | Record controller scheduling/resources, counterbalance order, separate profiles/count failures; machine-local numbers stay machine-local. |
| Death/orphans/partial artifacts | One owner, actual reap/drain, incomplete retention and stopped admission; forced death requires operator cleanup confirmation before evidence resumes. |
| Sensitive raw output | Closed environment, discarded bounded bytes, finite fields, private no-clobber artifacts and safe rendering; no transcript escape hatch. |
| Missing baseline/cost | Unavailable status, no placeholder/estimated actuals or agent numbers; full completion remains open. |
| Parallel contracts/number collisions | Keep 0090/0091 proposed, coordinate reservations and operator reconciliation without rewriting history. |
| HIGH review-integrity findings/#219 | Carry original references, no reproductions/access expansion/closure inference; published pinned engine and protected review remain required. |
| Proof complexity | Reuse concrete owners/dependencies; do not remove failures, weaken assertions or relocate production to escape coverage. |

## Design-stage verification observations — 2026-10-28

These checks were actually attempted against the draft on base 4919b31; they are
not implementation or full-story success. Engine SHA-256 matched tooling.json
and every public shim matched its pinned hash. Root formatting and locked clippy,
pinned bundle compilation, coverage selftest, publish-guard proof, seal-branch
proof and license check passed. The locked/offline native-tls inverse tree failed
specifically because that package is absent, satisfying the absence check.

Root cargo test failed in the two decision-inventory tests because proposed 0091
has no index row. The chief's exact four-file restriction leaves the index addition
to implementation T1. The instrumented coverage run failed at those same tests;
it did not produce a passing exact-coverage verdict. This is an outstanding gate,
not an exemption or assertion that production coverage is complete.

Standalone eval fmt, locked clippy and full tests were invoked and failed at
artifacts.rs's existing unclosed/mismatched delimiters amid unresolved conflict
syntax. No filtered suite, private-lock proof or historical pass replaces those
failures. Gate command/status logs are retained in run-local chief evidence;
CI-owned proofs, full #201 coordination, real competitor measurements, model
trials and protected review were not performed by this design seat.
