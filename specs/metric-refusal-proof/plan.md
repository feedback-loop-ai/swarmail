# Plan: metric refusal proof

Implements [spec.md](spec.md) from baseline `62e6496`. The checkout has no
`.specify/templates/plan-template.md`; its required sections appear below.

## Technical context

Single Rust 2024 crate, Tokio, Axum 0.8 and existing direct percent-encoding
2.3.2. `src/api.rs::inbox_of_path` still returns a raw segment;
`count_in_flight` takes label/total guards before `next.run`. The handler
extracts decoded `Path<String>`, admits it, then calls `park_await`.
`prom_label` escapes backslash, quote and LF; CR/NUL pass literally.
`tests/api.rs` already owns the independent emitted-subset parser.

## Constitution check

[0065](../../docs/decisions/0065-decoded-inbox-metrics.md) already authorizes
this narrow implementation of the public metric contract. No new amendment
is needed or proposed. Preserve frozen 0001–0003, ingest extraction 0005,
0046 admission and 0047 cap/overflow. Apply
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
to seams and cold paths. No dependency/allowlist, feature, coverage exclusion,
gate, recipe or policy edits. Follow 0050's pinned proofs and actual executed
evidence, 0056 unsigned commits, and 0057's full protected review route.

## Project structure and files touched

The chief commits exactly `specs/metric-refusal-proof/{spec,plan,tasks}.md`.
Positions and logs remain ignored. Downstream implementation is bounded to:

| File | Responsibility |
|---|---|
| `src/api.rs` | Existing helper/comment/inline matrix; cfg(test)-only request hold at the accounting owner; declare the cold-path harness if needed. |
| `tests/api.rs` | Ordinary concurrent encoded awaits and separate invalid-UTF-8/refusal baseline assertions; parser consumers and independent parser fixtures. |
| `tests/common/prometheus.rs` | If required by the two test binaries, extract only the existing parser and Sample type into test-only support. |
| `src/api/metric_refusal_tests.rs` | Owner-local real-HTTP held-refusal test when concrete timeout/failure diagnostics require a harness; no production code here. |
| `README.md` | Only missing decoded-selector/collision guidance; preserve existing correct text. |
| `specs/decoded-inbox-metrics/{spec,plan,tasks}.md` | Current scope reconciliation, dated commands/results, full paired-proof task status and unresolved review. |
| `specs/metric-refusal-proof/{plan,tasks}.md` | Continuation progress linked to owning evidence, without duplicating receipts. |

No edits to decisions/index, historical recovery/recording artifacts, source
outside the API owner, Cargo files, renderer, admission, registries or MCP.
The request-extension installer and server setup remain test-only. Sharing
the parser does not move all common helpers or existing inline test modules.

## Research and resolved choices

Read both run-local positions, current intake and previous recovery framing,
accepted authorities, `62e6496`'s seven-file diff, retained security JSON and
runtime tasks. The existing T9 prescribes evidence but has never executed it;
T2–T8 are also outstanding. Prior green documentation gates do not prove the
runtime feature. The paragraph and receipt corrections already landed.

No live refusal or registry-headroom proof has been performed by design.
Implementation must capture the real bytes before changing the helper and
inventory maximum distinct cumulative registrations in each affected test
executable, including other concurrently running tests. If the upper bound
cannot prove room below 4096, use a separate test process. A library-test
child may own its cfg(test) seam; do not export the seam in the shipping
binary to obtain isolation. Parser acceptance proves the emitted subset,
not compatibility with every monitoring product.

## Data model and implementation approach

Keep `Option<String>` and split/filter/match selection. Inside the recognized
arm use `percent_encoding::percent_decode_str(inbox).decode_utf8()`, owning
the decoded value on success and the whole original segment on error. Guard
keys are chosen at acquisition and immutable through RAII drop. No persistent
model, new identity abstraction or normalization is introduced.

For the hold, a cfg(test)-only outer layer matches one exact raw URI and
attaches a request-owned carrier for reached/release one-shots. Consume its
one-shot capability once. `count_in_flight` removes the extension before
passing the request onward, retains the carrier locally, awaits `next.run`,
then signals reached and awaits release before returning the unchanged
response. Both guards remain lexical owners through this interval. The
scrape path never installs the extension; no registry mutex spans the hold.
Keep the uninstrumented production flow semantically identical. Do not add
handler validation or move middleware to expose the interval.

Dropped release is a release condition, not an indefinite wait. Cover both
installed and absent seam paths and any measured cleanup arms. Use bounded
abort/join ownership on errors; keep server shutdown within a deadline and
runtime teardown ownership for remaining connections. Do not claim a client
abort proves immediate server-handler cancellation. If the concrete cleanup
code has cold diagnostics, put that wire test in the named cfg(test) harness;
otherwise it stays measured inline. The ordinary public-wire test remains
in `tests/api.rs`, which cannot install a library cfg(test) seam.

## Contracts and proof sequence

1. Pin raw/text refusals first (spec AC1). Independent fixture expectations
   describe observed bytes rather than assume Axum returns JSON.
2. Reuse the grammar parser via test support if both binaries need it. Add
   exact CR/NUL and retained malformed controls before trusting the scrape.
3. Audit exclusive names/headroom. Keep two empty inbox waits stationary,
   observe both active rows, assert decoded request rows immediately, then
   seed/join/observe cleanup (AC3). Families remain separately sampled.
4. Install the request-specific hold for the control refusal (AC4). Check
   fixture-related await rows before, during and after; inspect one full
   scrape during the reached handshake. Match the complete label/sample set
   attributable to the fixture, not a substring or global row count. Unrelated
   concurrent tests may legitimately contribute samples.
5. Demonstrate both behavioral reds using only the old raw helper. Record
   exact assertion output and command. Restore the helper and rerun green;
   no mutant may survive into gates or a commit. Keep the separate invalid-
   UTF-8 proof and whole-raw mixed-input helper assertion.
6. Reconcile owning docs: retain historical receipts verbatim, date the prior
   recovery's source restriction as historical, and make the current T9 scope
   and test sequence unambiguous. Check a task only after its whole proof.

## Quickstart and validation

Run `cargo test --locked --test api`, `cargo test --locked --lib api` and
`cargo test --locked --test store_contract_docs`. Ensure the harness module
name is selected by the library API filter. Retain old/new assertion logs,
refusal bytes and fixture audit in the owning runtime plan's dated receipt;
link detailed ignored logs without treating their existence as success.

Run every command in the authoritative
[house-rule inventory](../../docs/house-rules.md#gates--part-of-every-change)
with offline locked dependencies and no installs. Check candidate-local
`.forge/bin/brokkr` against `bundles/self/tooling.json` before compile. Use
pinned bundle proof scripts with shim integrity, exact coverage and selftest,
publish/seal proofs, licenses and fail-closed native-tls absence. A failed
or timed-out invocation is not a pass. CI retains chart history/selftest,
real-Docker, network advisory, tag release-rate and real port-80 proofs.

Fresh eventual-candidate verification and the engine's full independent
correctness/security/guarantees/tests review remain downstream. Keep the
security hold and provider policy intact; dispatch is not review success.
The chief's last commit must touch only this spec directory; run the pinned
`design-check.sh check` against HEAD without writing the engine check result.
The chief writes only its assigned `drafted` result after honest validation.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Decoder changes structure or invents a lossy identity | Select before decode, one strict pass, whole-raw fallback and exact edge matrix. |
| Refusal row vanishes before observation | Post-handler reached/release hold under the live guards, real full scrape and cleanup assertion; accept a test-only carrier. |
| Instrumentation changes ownership or leaks into shipping code | Request-owned cfg(test) extension, exact URI match, existing RAII ownership, no public hook or feature; measured arm execution. |
| Parser hides controls or producer defects | Independent literal CR/NUL fixtures, unchanged grammar/negative controls and full exact sample assertions. Return actual conflicts for review. |
| Parallel tests exhaust static cap | Executable-wide upper-bound audit, exclusive names and conditional process isolation; no momentary headroom claim. |
| Deadlines leave handlers or tasks running | Bounded reached/release/HTTP/join/stop, dropped-release handling, owned cleanup and runtime teardown; no sleeps as synchronization. |
| New harness hides production or convenient inline branches | 0020 cold-path justification and cfg(test) declaration; exact gate, no module migration or coverage exclusions. |
| Gates become a false security disposition | Separate witnessed evidence, final candidate gates and actual independent security ruling; retain the residual until review. |
| Documentation drifts into a second contract | Runtime spec owns semantics/evidence; continuation links it, reconciles current instructions and preserves dated history/reservations. |
| Broader guarantees are accidentally weakened | Scope review plus existing real-protocol suites; no store/SMTP/webhook/TLS/extraction changes or new guarantees. |

## Chief design validation — 2026-10-24

Fresh checks ran against `62e6496` plus this three-file design draft with
`CARGO_NET_OFFLINE=true`. All 16 recorded checks exited zero: shim integrity;
targeted API (64 tests), library API (31 tests) and document (19 tests) suites;
formatting; locked all-target Clippy with warnings denied; locked workspace
tests; digest-matched brokkr compile; exact coverage and its selftest; design
validator selftest; delivery tamper-boundary proof; publish-guard proof;
seal-branch keyless proof; licenses; and fail-closed native-tls absence from
a successfully resolved, nonempty locked dependency tree.

The candidate-local brokkr SHA-256 matched tooling.json:
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Compile reported 76 valuations, 14 groups and zero unruled valuations, with
no policy-sweep warning. Coverage reported exact 100% lines, branches and
functions (zero missed); its evidence is `target/coverage/run.anr5hc`.
Publish proof retained 51 mutation-red/restored-green receipts; seal proof
passed seven cases and ten mutants. Relative links and required document
structure passed, and decisions/index remain byte-identical to baseline.

Commands, durations, exit codes and logs are under the ignored directory
`.brokkr/state/design/metric-refusal-proof-checks/`. The workspace call timed
out awaiting its response after 300 seconds; the runner continued. Subsequent
inspection found `current.txt` equal to `finished`, all 16 exits zero, and
corroborating logs. The response timeout itself was never counted as a pass.
Only this validation receipt was appended after execution; final whitespace,
path and committed-artifact checks follow it.

These are design-candidate checks of the existing runtime, not evidence that
the new decoding, baseline-refusal captures or held-refusal regressions have
been implemented. Runtime tasks remain unchecked. Fresh eventual-candidate
verification, CI-owned proofs and actual protected independent review/security
disposition remain downstream obligations.
