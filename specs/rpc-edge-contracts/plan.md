# Implementation plan — RPC edge contracts

Implements [spec.md](spec.md) and proposed
[0062](../../docs/decisions/0062-rpc-edge-contracts.md).
This is a design handoff, not a gate or implementation receipt.

## Technical context

One Rust crate, Tokio, serde_json and existing real-process/wire suites.
No dependency, license, schema, public configuration or installation is needed.
The checkout has no plan template; its required sections appear here.
Base HEAD is recipe commit 450bef65c581b3a4791cf94dbc145912814bd6ed directly over
df8630f69a390fc6a35e1dfc8c54f2f40dd7e624. Retain both.

The exact candidate engine was verified before exploration:
`/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-180-luna2/.forge/bin/brokkr`,
SHA-256 `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Use this absolute executable for every engine/compile check, never PATH.
Keep candidate-local build targets and engine-managed shared journal
`/home/valentin/source/feedback-loop-ai/swarmail/.forge/forge.db`; do not
write invented journal evidence. Luna xhigh and review assignments remain.

## Constitution check

Preserve 0001–0005, 0020 test placement, 0029 staging/bridge contracts,
0039 line concurrency, 0044 batch semantics, 0047 Host/local-size behavior,
0050 pinned proofs and 0056 unsigned commits. The supplied operator directive
authorizes only the exact ID domain; 0062's proposal is not blanket acceptance.
Append the authorized dated 0047 addendum during implementation. No store/
persistence/SMTP/webhook implementation change, gate bypass, floor reduction,
dependency widening, policy/recipe edit, push or merge.

## Research and resolved questions

Both positions and intake were read. Source confirms short bodies currently
fall through parse_response, supplied IDs are cloned unconditionally, and
arg_usize casts u64 to usize. The refused-port binary test does not execute
the warning; Host already uses original authority; the timeout test follows
the test module's closing brace. These are inspections, not executed reds.

0061 is the highest existing decision; 0062 is the next sequential free
number, leaving earlier lanes' gaps alone. The no-new-ADR proposal loses to
the chief artifact contract. Proposed 0062 and the subsequent 0047 addendum
distinguish supplied operator authority from document acceptance. The index
tests require an implementation row; do not hide this interim design failure.

An opened directory supplied as child stdin is the selected Unix error
fixture to validate on Linux/macOS. EOF is not an I/O error. Actual 32-bit
runner availability and privileged port-80 execution remain evidence questions.

## Project structure and files

| File | Scoped implementation |
|---|---|
| src/stdio.rs | Length comparison, production-used request-format helper, parser/Host/correlation tests, move stray timeout test into existing module. |
| src/mcp.rs | One private ID predicate, reply normalization/pre-dispatch refusal, checked usize conversion, classifier/staging tests and comments. |
| tests/stdio.rs | Real short-response peer, recovery, notification/batch/correlation and valid upstream controls. |
| tests/mcp.rs | Real HTTP ID table, mixed batches, observable no-effect tests and target-specific overflow call. |
| tests/binary.rs | Bad-stdin warning and real stdio ID parity; retain protocol-only and ignored port-80 proofs. |
| docs/decisions/0047-storage-rpc.md | Append only the dated supplied operator amendment to §3. |
| docs/decisions/README.md | Add exactly the proposed 0062 index row in implementation. |
| specs/rpc-edge-contracts/{spec,plan,tasks}.md | Handoff and later truthful progress/clarifications. |
| docs/decisions/0062-rpc-edge-contracts.md | Proposed semantic record; acceptance remains operator-owned. |

src/main.rs initialization is a proof boundary to inspect, not a planned
behavior edit. Update directly affected current product contract text only
if it contains an unconditional ID-echo claim. Preserve accepted historical
text and old completed task records. A sibling Rust harness requires a
concrete cold failure path under 0020; no default relocation of measured
tests. No CI refactor is planned.

## Data model and contracts

No persisted data changes. Envelope remains Silent, Single(Value),
Batch(Vec<Value>) and InvalidBatch. reply_id returns Some(null) for invalid
supplied IDs; None remains exactly missing-ID plus string-method notification.
A local predicate admits Value::String, Number and Null. handle checks
supplied IDs immediately after confirming an object, before method/tool
dispatch; invalid ID returns -32600/null.

Preserve existing cap/header/status checks. For a parsed length n, compare
available body length before decoding: short means a truncation error
(prefer expected/received counts without body data); otherwise slice to n.
Absent/unrecognized length keeps raw-tail behavior. No allocation by n.

Extract only the request-format expression into a private helper used
unchanged by rpc_post, passing original authority; dialing uses dial_target.
Tests exercise the helper whose result rpc_post actually writes.

Use usize::try_from in staging, preferably composing as_u64 and conversion
into one Option/Result chain with the existing named refusal. No production
cfg branch or simulated-width API. Target-specific tests may use
cfg(target_pointer_width = "32"); overflow test data is u64, never an
overflowing usize expression. Machine-size acceptance belongs at staging;
tool domain limits remain stricter.

## Implementation and proof sequence

1. Reconcile authority/index first: dated 0047 addendum with supplied directive
   provenance and proposed 0062 index row. Run document-integrity tests.
2. Capture behavioral reds against recipe-base short-body and invalid-ID
   behavior with isolated fixtures/patches and exact source identity. Apply
   local repairs and real-wire regressions.
3. Add warning-route and production Host tests. Prove sensitivity by disabling/
   rerouting tracing and substituting dial_target in Host in temporary local
   mutations; restore exact source bytes before continuing.
4. Apply checked staging and ordinary controls, then actual 32-bit tests if
   installed target/runner exists. Record missing evidence; never substitute
   a cross-build or install in offline verify.
5. Move the timeout test without broad relocation, prove discovery/execution,
   and handle any concrete cold path only under 0020.
6. Run targeted suites, full gates and external proof collection. Every new
   line/branch/function stays measured at exact 100%. Use fresh state,
   ephemeral ordinary ports and child-local environment. No sleeps as sync,
   global subscriber mutation, probabilistic chaos, mocks or calendar-based
   validity assertions. Transport failure never promises mutation rollback.

## Quickstart and validation

From the candidate root, after implementation:
```bash
cargo test --locked --test store_contract_docs
cargo test --locked --lib mcp::tests
cargo test --locked --lib stdio::tests
cargo test --locked --test mcp
cargo test --locked --test stdio
cargo test --locked --test binary
/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-180-luna2/.forge/bin/brokkr compile --bundle bundles/self
```

Use `cargo test --locked --target <installed-32-bit-triple> ...` with the
actual configured runner for staging and MCP overflow witnesses; record
expanded commands, names, target and results. The placeholder is not evidence.

The full gate inventory is [house rules](../../docs/house-rules.md#gates--part-of-every-change).
Use pinned script copies after verifying shims; preserve --locked, coverage
selftest, publish/seal proofs, licenses and fail-closed native-tls absence.
The design check is `bash bundles/self/scripts/design-check.sh check .`
against the design commit; the sequence independently executes its pinned
copy. Structural validation is not product verification.

CI owns chart contract/history/selftest, real Docker smoke, RustSec audit,
tag-only release-rate-5k and privileged Linux/macOS port-80 proof. Linux/
macOS bad-stdin executions and actual 32-bit runtime evidence must also be
identified. Missing receipts stay open; no release/PR/host evidence is inferred.

## Risks

| Risk | Mitigation and accepted limit |
|---|---|
| Normalization conceals an invalid mutation. | Guard handle before dispatch; assert unchanged state plus effectful control. |
| Predicate drifts at a caller. | Test HTTP execution, local bridge failure/oversize correlation and ordered mixed batches. |
| Framing masks a plausible upstream error. | Deliberately choose framing first; forward only complete errors. New -32000 still means uncertain server effects. |
| Parser expands into HTTP conformance work. | Keep malformed/duplicate lengths, chunked coding and read-to-close limitations explicit. |
| Bad stdin is EOF/platform-dependent. | Require actual warning, stderr, empty stdout, exit 1 and bounded completion on both hosts; name absent proof. |
| Formatter proof overclaims default dialing. | Production helper plus mutation witness; independent real port-80 CI exchange. |
| usize overflow is cold on 64-bit. | Unconditional checked conversion and actual 32-bit runtime, no fake widths or denominator reductions. |
| Timeout fixture conflicts with exact coverage. | Move only the required test; identify concrete cold paths before 0020 harness use, never exclude/lower coverage. |
| Proposed ADR fails interim index integrity. | Exact four-file chief commit; first implementation task indexes it. No claim of full-gate success at design time. |

## Design-phase validation — 2026-10-24

Executed against the draft artifacts: formatting, locked all-target clippy,
pinned absolute-engine self-bundle compilation, shim hashes, coverage selftest,
pinned publish/seal guard proofs, license check and fail-closed native-tls
absence passed. The focused document suite and full locked workspace test run
failed two document-index tests naming `0062-rpc-edge-contracts.md: no index row`.
Coverage refused because its instrumented test run hit the same failures; no
coverage percentage or full-gate pass is claimed. The first implementation
task owns the required index row, outside this chief's four-file commit.

Run-local command/exit receipts and logs are in
`.forge/design-rpc-edge-checks/`; the coverage failure log is
`target/coverage/run.WkRCro/run.log`. The workspace call exceeded its client's
300-second response timeout, but the command completed and its complete
results.json and logs were read back through workspace. These are design-tree
observations, not final implementation verification or CI/32-bit/host proof.

## Implementation phase record — 2026-10-24

The dated operator amendment to 0047 §3 and proposed 0062 index row are in
place. `src/mcp.rs` shares scalar-ID validation between reply classification
and pre-dispatch refusal, and stages `usize` through `usize::try_from`.
`src/stdio.rs` rejects short declared bodies before decoding and builds the
request with the original Host authority. Regression coverage now includes
real HTTP/stdio invalid-ID behavior, batch positions and no-effects, a short
response peer with recovery, exact Host construction, and a failing-stdin
binary that observes stderr. The timeout test now resides inside its existing
`cfg(test)` module. Baseline-red and mutation-sensitivity receipts are stored
under `.forge/results/`.

The complete locked workspace test suite, fmt, clippy, self and codex-fast
pinned compiles, coverage selftest, publish/seal proofs, licenses, shim hashes
and fail-closed `native-tls` proof pass. Exact coverage remains red only at
`src/mcp.rs:285`: 13078/13079 lines and 1444/1445 functions, because the
32-bit-only `usize` conversion error arm cannot execute on this x86_64 host.
The installed `i686-unknown-linux-musl` target cannot compile SQLite here
without missing `musl-gcc`; the host also lacks i686 multilib headers. No
32-bit execution or coverage pass is claimed.

External evidence still required: supported-host macOS stderr execution, the
privileged real port-80 exchange, CI chart/Docker/RustSec/tag-release proofs,
and an actual 32-bit runtime. Decision 0062 remains proposed; its record and
this implementation do not constitute operator acceptance.
