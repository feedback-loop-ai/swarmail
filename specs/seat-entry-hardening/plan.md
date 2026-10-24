# Implementation plan — seat entry hardening

Implements [spec.md](spec.md); task/proof pairs are in [tasks.md](tasks.md).

## Technical context

Single Rust crate, Bash seat scripts, Git NUL plumbing and jq result rendering.
Proof scripts live in `bundles/self/scripts/`; public scripts are checked fixed
shims. The candidate-local `.forge/bin/brokkr` matches the recorded binary
identity. Verify is offline and installs nothing. No new dependencies,
workspace members or runtime product changes are needed.

`.specify/templates/plan-template.md` is absent (only the spec template exists).
This plan supplies the required technical context, constitution check, project
structure, research, data model, contracts and quickstart directly.

## Constitution check

This repair preserves 0007, 0020, 0045, 0050 and 0057 and every frozen guarantee.
The final deterministic check alone advances the chief's committed design.
Maintain pinned proof ownership, engine digest checks, strict HEAD identity,
exact coverage, no mocks in product protocol suites, independent reviewers and
security hold. Shell execution fixtures belong to the existing delivery test
owner; they do not require moving production code out of coverage. Chief
commits exactly the three design artifacts unsigned and never pushes.

No policy/proof-boundary change is authorized. If research uncovers such a
requirement, retain the evidence, return to triage and draft a proposed ruling
for operator resolution as necessary; do not modify accepted decisions or
reinterpret this design as advance approval.

## Project structure and files touched

| Owner | Planned change |
|---|---|
| `bundles/self/scripts/design-check.sh` | NUL-safe discovery, checked producer/temp lifecycle, strict status diagnostic/token/ambiguity checks and existing selftest fixtures. |
| `bundles/self/scripts/verify-seat.sh` | Non-login runner, safe invocation of digest-checked resolved engine, complete existing resource preflight list. |
| `tests/delivery_constitution.rs` | Bounded runtime seat fixtures for startup, failure propagation, resource omissions and identity; retain structural pins. |
| `agents/README.md` | Effective correctness charter resolution and how to reproduce it. |
| `specs/seat-entry-hardening/{spec,plan,tasks}.md` | This committed design and later accurate evidence/task updates. |

Inspect existing `delivery-proof-test.sh`, coverage/scanner, speckit,
publish/seal scripts, shims, `tooling.json`, agent overlays and recipe wiring
as authority/callers. They need no production changes for the current design.
Do not edit bundle policy, routes, tooling identity, release configuration or
shim hashes merely to make a fixture pass. Run-local proof receipts remain
under ignored `.forge/` paths; positions stay under ignored `.brokkr/state/`.

## Research and resolved choices

Source inspection confirms `tr '\0' '\n'` destroys Git record boundaries,
`bash -lc` launches gates, and preflight omits the later delivery proof call.
Both positions support local fixes. The selected status contract is the strict
canonical grammar plus explicit diagnostic, not broader accepted typography.
Recognize the recorded alternate metadata forms only when detecting conflicting
headers; do not search arbitrary prose for status words.

Before coding, trace every static helper from the actual pinned entry, confirm
whether the actual exec child can inherit `BASH_ENV`, and establish a startup
sentinel whose login positive control works in the isolated test environment.
A temporary HOME alone is insufficient because login startup may reset it.
Use scoped child-process instrumentation or an isolated startup hook that
observes Bash's `login_shell` state; never edit host startup files or global Git
configuration. State the limits of instrumentation in the proof receipt.

Recheck `agents show codex-review-correctness` with the recorded local engine
and compare its charter digest, singleton chain and tracked inheritance.
The chief's observation is in AC5. Recheck actual launch resolution because a
future local library could shadow tracked definitions. No missing historical
copy can be reconstructed honestly from these observations.

## Implementation approach

1. In validator discovery, create one unique temporary stream file with
   `mktemp`; check the direct Git producer status before consuming bytes.
   Read with `IFS= read -r -d ''`, extract the immediate slug component and
   use exact quoted array comparisons to deduplicate. Keep invalid slugs in
   the count. Remove the stream before later validation; cover all failure
   exits without replacing the selftest's existing EXIT trap. Do not place
   NUL streams in shell variables or trust process-substitution status.
2. Extend the existing Git selftest using real commits and existing verdict
   assertions. Preserve a disposable baseline validator for the newline
   bypass demonstration; assert the baseline passes that specific fixture
   while the repaired version refuses with the intended two-slug reason.
   Use a scoped Git wrapper only to inject producer error/partial output,
   delegating all unrelated Git commands to real Git. Such failure injection
   must never become a production override.
3. Keep a small anchored metadata check for declared status. Canonical form
   and token boundary establish proposed status; count recognized header
   forms to reject duplicates/conflicts rather than accepting any matching
   line. Emit the exact spelling in every status refusal. Preserve the
   declaration/link/authority distinction and all committed/clean checks.
4. Replace login Bash with `bash -c` in the existing runner. Resolve brokkr
   once, retain digest comparison, and shell-quote that resolved path using
   the existing `%q` approach before compile invocation. Preserve the
   compound native-tls command, labels, redirects, captured output, ordering
   and result schema. Do not build a new dispatcher or environment policy.
5. Add the missing delivery proof to preflight and audit the complete static
   closure. Required entries relative to `pin_dir` are `coverage-gate.sh`,
   `publish-guard.sh`, `publish-guard-test.sh`, `design-check.sh`,
   `delivery-proof-test.sh`, `seal-branch.sh`, `seal-branch-test.sh`,
   `speckit/check-prerequisites.sh`, `speckit/common.sh`,
   `coverage-fn-scan/Cargo.toml`, `coverage-fn-scan/Cargo.lock`,
   `coverage-fn-scan/src/main.rs`, `shims.sha256` and `../tooling.json`.
   Keep missing-resource refusal before all gates and shim-integrity failure
   accumulation separate. Generated selftest helpers are not static entries.
6. Extend `tests/delivery_constitution.rs` with unique temporary candidate,
   resource and result roots, bounded subprocesses and cleanup. Execute the
   actual seat source against controlled fixture executables/resources,
   without modifying its identity logic or adding a skip switch. Provide
   complete controls before testing each omission. Stop instrumented gate
   sequences at a deliberate named failure so cargo test cannot recursively
   invoke itself or delivery-proof invoke the full verifier. Fixtures must
   assert fresh JSON keys/result/diagnostics, not the seat's exit code alone
   (a correctly written failure result may exit zero).
7. Document charter resolution in `agents/README.md`; run focused proofs,
   then the full existing inventory and actual pinned dispatch. Store the
   candidate commit, engine/resource identities, commands and outcomes in
   run-local evidence. The final engine check after chief is real design
   dispatch evidence; later verifier evidence must name its assessed
   candidate and pin, not rely on a copied script fixture.

## Data model and contracts

No application data model or persisted schema changes. Internally the changed
path stream is a temporary NUL-delimited file and the unique slug collection
is a Bash array. Temporary state is per invocation and never shared between
concurrent delivery runs. Slugs remain kebab-case; subordinate filenames retain
the existing contract.

The seat protocol remains JSON with exactly `result`, `inputs`, `notes` and
atomic publication. Chief returns `drafted`; design check emits `designed|fail`;
verify emits `pass|fail`. Existing CLI modes, gate order and pinned-helper
locations remain. Status typography is not operator authority. No alternate
result vocabulary, test-only mode, mutable resource manifest or gate bypass is
introduced.

## Validation and quickstart

From the candidate root, use the digest-matched `.forge/bin/brokkr` and the
already installed offline toolchain. Focused commands after implementation:

```bash
bash bundles/self/scripts/design-check.sh selftest
cargo test --locked --test delivery_constitution
.forge/bin/brokkr agents show codex-review-correctness
sha256sum agents/charters/review-correctness.md
.forge/bin/brokkr compile --bundle bundles/self
```

Run every command in the single house-rules gate inventory, using the pinned
copies inside actual verify, plus the existing delivery tamper proof and
fail-closed native-tls probe. No narrowed suite discharges full verification.
Compilation is not provider dispatch. Instrumented subprocess fixtures prove
seat control flow, not hermetic pin mounting. Actual engine design and verify
dispatch must supply that evidence; record a blocked proof as outstanding.

Immediately after the chief commit, `bash bundles/self/scripts/design-check.sh
check .` can locally inspect the artifacts; the sequence's pinned final exec
step still owns the design verdict. After implementation commits, HEAD no
longer names the chief design: do not weaken selection or create an unrelated
commit to force a check. Use isolated design fixtures for validator regressions
and preserve the chief-check receipt for this sequence.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| NUL/whitespace loss or masked Git failure admits the wrong design | Read NUL records, compare full strings, check producer status and use real red/green Git fixtures; accept a small temp lifecycle and linear search. |
| Partial output, cleanup or concurrent runs reuse stale evidence | Unique roots, checked creation, prompt-specific fresh results, cleanup on every owned exit and a partial-output failure fixture. |
| Login sentinel never exercised, or a resource/identity error preempts the intended failure | Mandatory login positive control, complete-resource control, assert named diagnostic and gate markers; bounded instrumentation supplements real dispatch. |
| Non-login execution overclaimed as total environment attestation | Inspect actual exec environment, document BASH_ENV limits, escalate untrusted-boundary changes; keep broader attestation outside 0050's scope. |
| Engine path interpolation executes different bytes or shell code | Invoke the resolved digest-checked executable with data quoting; exercise paths with spaces/metacharacters and preserve mismatch refusal. |
| Explicit resource list drifts or integrity failure exits too early | Per-resource removal tests and call-chain review; retain real combined tampered-shim/uncovered-line proof. |
| Strict status grammar inconveniences authors or accepts contradictory headers | Print exact required spelling, require full token and a single recognized metadata header; test alternate forms, conflicts and historical prose. |
| Runtime fixtures recursively run cargo/verify or mutate shared state | Deliberately fail before recursive stages; isolate child environment and temp roots; no production bypass or shared HOME edits. |
| Charter observation is mistaken for historical or live reviewer evidence | Label dated candidate-local observation, compare digest at validation and actual launch, preserve independent route pins and security hold. |
| Scope expands into policy/product repair | Return to triage; no accepted-rule amendment, dependency/release change or coverage relaxation under this design. |

## Design-stage validation — 2026-10-24

The chief ran the full local house-rules inventory on the documentation-only
candidate: fmt, locked all-target clippy, locked workspace tests, recorded
engine compile (zero unruled valuations), exact coverage, coverage selftest,
publish-guard proof, seal keyless proof and license check all passed. The
fail-closed native-tls absence check and existing design validator selftest
also passed; local document links and required headings/tasks were checked.
Exact coverage was 12,953/12,953 lines, 762/762 branches and 1,437/1,437
functions. Its first bounded attempt timed out; the standalone longer retry
completed successfully. Receipts are run-local under
`.forge/design-chief-checks/seat-entry-hardening/`. These are baseline checks
for this draft, not evidence that AC1–AC5 repairs, actual implementation
verify dispatch, independent reviews or CI-only proofs have been completed.
