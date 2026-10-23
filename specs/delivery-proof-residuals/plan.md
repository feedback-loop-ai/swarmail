# Implementation plan — delivery proof residuals

Implements [spec.md](spec.md); proposed [0062](../../docs/decisions/0062-delivery-proof-residuals.md).

## Technical context

One Rust application crate; integration pins in `tests/delivery_constitution.rs`;
JSON bundle/policy; Bash canonical proofs under `bundles/self/scripts/`;
SHA-256 tooling already required by verify; recorded brokkr 0.12.0 base
`597f4868e3fb971b62e6a368a2d8aa23271f4531` plus three ordered patches.
Use the digest-matched candidate-local `.forge/bin/brokkr`, never a sibling
binary or an arbitrary PATH build. There is no plan template in
`.specify/templates/`; this plan supplies the required sections directly.

## Constitution check

0050 requires actual evaluator/dispatch witnesses and transitive pinned code;
0057 requires the full engine lane and protected review here. The present
Codex recipe's four review routes and a qualifying self run's historical
mapping are distinct: do not relabel or modify one to impersonate the other.
Any provider incompatibility in the self route is a precise execution block,
not permission to rewrite its recipe. No nested run is launched by chief.
0056 requires unsigned commits and no push. 0045 §8 allows compatible proof
and factual documentation work while 0062 remains proposed. The marker does
not permit semantic deviation from 0050. No new measured production code,
workspace member, dependency, exclusion or gate bypass is needed.

## Research and dependency checkpoint

Before implementation writes, the authorized root context refreshes the live
canonical `.forge/forge.db` read-only and actual active candidate diffs.
Record timestamp, journal sequence, reserved paths and candidate identities
in preflight.json. Keep #172 preflight and tooling-pin ownership outside this
issue; any necessary overlap waits for root partition/integration. Recheck
0062 numbering on integration, with coordinated links if another run takes it.
The supplied framing found no bounded-path conflict; the workspace box cannot
refresh it. Do not fabricate a local database to satisfy this checkpoint.

Establish exact patched-core source/API availability from the recorded base,
patch digests and build provenance. Retain source and harness identities; no
source path is assumed available. Historical self receipts are inspected
before scheduling providers; validate their export chain and pinned manifest.
A historical name/count or old mismatched engine proves history only. Missing
core, current ownership, export or provider inputs park the dependent work.
These are execution prerequisites, not reasons to change the intended proof.

## Approach and project structure

| Owner/path | Work |
|---|---|
| `tests/delivery_constitution.rs` | SHA-256 shim verification, fixed approved design-check/verify-seat digests, positive named call assertions and both candidate-path refusals; bounded helper functions operate on fixture bytes/paths for mutation testing. |
| `bundles/self/scripts/shims.sha256`, `design-check.sh`, `verify-seat.sh` | Read-only authorities for this issue. Canonical byte changes need root coordination; tests do not bless arbitrary bytes. |
| `bundles/self/scripts/delivery-proof-test.sh` | Retain existing component proof; extend only a demonstrated missing component fixture. Do not make it invoke verify recursively. |
| `.forge/epic192/next-171/<attempt>/` | Recorded-core harness, literal cases, isolated mutants, real engine dispatch fixture driver, logs, exports and receipts. Retain source/build/command hashes for audit. No application Cargo dependency on brokkr. |
| `specs/delivery-proof-residuals/receipts.md` | Implementation adds one compact evidence index with durable retained locations/hashes and the complete obligation map. |
| `specs/delivery-gates/{spec,plan,tasks}.md` | Append a current controlling clarification, supersede stale current signing/status/predicate instructions, preserve historical entries, link receipts and tick only proven work. |
| `docs/decisions/0050-delivery-gates.md`, `docs/decisions/README.md` | Dated known-limit addendum pointing specifically to §2 and 0062; proposed index entry. Preserve accepted historical body/status. |
| `docs/decisions/0062-delivery-proof-residuals.md` | Proposed clarification authored by chief; no self-acceptance. |

### 1. Real evaluator and aggregation

Use an existing evaluator API or a minimal run-local harness linked to the
exact recorded patched core. Resolve its actual API before writing the
adapter; no invented CLI command. Cases are literal JSON objects with absent
keys actually absent, plus independent expected triples. Extract the supported
severity vocabulary from the recorded core and assert case-table completeness.
Feed the whole compiled self policy, then current Codex policy controls where
routing differs. Run representative member-ingress tests from the pinned
aggregation patches, adding only missing malformed/absent examples in the
external source fixture. No engine binary used for delivery changes.

Run a deliberately permissive policy through the same actual evaluator and
show an unsafe expected-stop assertion fail; restore and run the whole table.
Record each expected/actual rule, next and severity, count and process exit.
A harness that does not compile or skips rows produces incomplete evidence.
Use bounded subprocesses and publish receipts by rename only after all rows
and logs are durable; preserve interrupted-attempt diagnostics separately.

### 2. Pins and targeted mutants

Invoke the already-required `sha256sum` from Rust test helpers, failing on
command error or malformed output. Compare shims to the checked canonical
manifest and exact expected member set. Store independently reviewed baseline
hash literals for design-check and verify-seat in the test, with update
comments requiring canonical diff review. Whole-script pinning catches an
identity check/call moved into a comment or `if false` body without parsing
shell; named assertions retain useful failure messages. Fixed approved hashes
are not a substitute for runtime refusal proofs.

Run each mutant against the same checking helper with a disposable directory
or byte input. Require baseline green, mutant failure for its intended guard,
and restored green, including each AC3 deletion/comment/disable variant. Do
not mutate the shared checkout or refresh expected hashes during testing.
The digest test is sensitive to benign edits; accept that maintenance cost.

### 3. Actual dispatched proofs and live self

Keep the actual-seat orchestration outside `delivery-proof-test.sh`, because
verify already invokes that script. Use isolated full candidate fixtures
satisfying verify's non-coverage prerequisites, with their own build/result
paths. The red candidate has clean formatting/compilation but a deliberately
uncovered production branch and an exit-zero public coverage shim. Invoke the
real engine's verify dispatch with pinned resources; require both diagnostics
in its failing result. Remove the uncovered branch and restore shim bytes for
the complete covered green control. Do not substitute a call to the component
script or add a candidate-controlled skip switch. Exercise missing/mismatched
engine identity through fixture inputs without changing approved tooling.

For design, keep old valid artifacts present and give the chief HEAD a missing
risks section; replace the candidate validator with a stub while dispatch uses
its original pin. Capture the actual validator refusal. A qualifying live
self run must record a genuine invalid-design failure, the policy's first retry,
corrected chief commit, final check and all later required phases to local
ledger/done. The candidate-stub refusal may be a separate dispatch control;
the successful live run restores all candidate bytes before verify.
Correction changes candidate artifacts, not the pinned algorithm or policy.
Second-failure and leaked-drafted controls use separate identified actual
engine runs; they cannot be cited as successful lifecycle evidence. No forged
phase result or journal edit creates the retry. Coordinate the bounded fault
fixture with root before the authorized execution stage starts providers.
Use the canonical shared journal specified by 0057's later clarification,
with distinct run IDs and isolated candidates/results/builds, not a new
untracked historical database. Resume only from actual recorded attempts.

### 4. Comparator documentation and old obligations

Record the existing text comparator's equal-indentation boolean substitution
and its limited significance, preferably with the existing observed diff and
actual function call in an isolated copy. The observation need not run a new
Rust build because this design selects documentation, not a refusal repair.
Retain the existing real-build debug-assert green and bare-boolean red controls
under the mandatory selftest. Do not broaden the allowance, normalize more
text, remove production debug assertions or present the textual observation
as a real-build gate result. Append a dated addendum titled for 0050 §2,
link 0062 and identify this unresolved false acceptance without changing the
normative body. Review new/current documentation for contradictory claims.

Reconcile T08, R04 and D04 separately; D04 includes all old AC1–AC13 and final
gates. Existing qualifying receipts may discharge old obligations only after
candidate/pin applicability is established. If an unrelated old proof is
missing, list the exact outstanding obligation and responsible owner; this
issue cannot truthfully claim all requested boxes checked until supplied.
Do not silently enlarge the implementation into those unrelated repairs.

## Data model and contracts

No runtime data model, public API or policy schema changes. Run-local receipt
objects contain `status` (passed, failed or incomplete), `obligations`,
`candidate` (commit/tree), `engine` (binary/base/patch identities), `bundle`
(manifest/policy hashes), `harness` (source/build hash), `command`, `exit`,
`diagnostics` (retained path/hash), and optional `run_id`, `attempt`, `sequences`
and `cases` (input/expected/actual). These are evidence files, not phase-machine
results; never introduce these fields at the top level of engine results.
The phase result contract stays exactly result/inputs/notes. No credentials
or model transcript secrets enter committed receipt summaries.

## Quickstart and verification

After the preflight checkpoint, compare `.forge/bin/brokkr` SHA-256 with
`bundles/self/tooling.json`, then run
`cargo test --locked --test delivery_constitution`. Compile using that binary
with `compile --bundle bundles/self`, capturing unsuppressed output. Execute
the recorded-core matrix, aggregation regressions and real-dispatch fixture
commands resolved in research, retaining their exact invocations; this plan
does not invent an unavailable evaluator CLI. Run canonical design-check
selftest and delivery component proof separately from actual-seat evidence.

Run every command in [the single gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
on the restored final candidate, with `--locked` where listed. Hermetic verify
runs pinned copies and proves native-tls absence fail-closed. CI retains its
chart/history/selftest, Docker, advisory, tag-rate and port-80 obligations.
An unavailable tool, dependency cache or provider leaves a named incomplete
receipt. Never install from the offline verifier, suppress warnings, lower
the floor or claim a gate passed from structural validation.

Chief commits exactly the three new design files and proposed 0062, unsigned;
HEAD remains that commit for the final check. Implementation later adds the
index/marker and evidence links. The chief's local structural check is not
the engine's final design verdict or an implementation completion claim.
The exact four-file chief boundary defers the ADR index update; until T07,
the decision-inventory tests cannot pass. Chief checks on 2026-10-24
confirmed both inventory tests fail for unindexed 0062; workspace tests and
coverage therefore fail. Formatting, Clippy, pinned compile, coverage
selftest, publish/seal proofs, licenses, native-tls absence, design selftest
and delivery component proof passed. Logs are retained under
`.forge/epic192/next-171/chief-checks/`. This is a design-stage limitation,
not a green final-tree gate verdict.

## Risks

| Risk | Mitigation and residual cost |
|---|---|
| Unknown facts become safe in aggregation | Real patched-core ingress regressions plus literal evaluator matrix; no defaults. Missing source blocks proof. |
| Test hashes bless the current arbitrary bytes | Fixed approved canonical digests and protected shim manifest; immutable expectations during red/green mutants. Review every update. |
| Text guards miss shell semantics | Whole-script identity plus actual dispatch substitution proofs. No general shell-safety claim from the two path strings. |
| Retained comparator admits unrelated release flips | Explicit dated known-limit marker; preserve normative obligation and label 3.4 documented. No claim of repaired equivalence; future repair/exception needs separate evidence and, if semantic, operator acceptance. |
| Honest debug assertions fail under naive hardening | No comparator algorithm change or Store edits in this design; retain existing green controls. |
| Live proof costs providers or stalls | Reconcile history first, identify exact missing provider/core/receipt, retain attempt state, no fallback or fabricated success. |
| Fixtures recurse or contaminate other runs | External actual-seat harness, isolated candidates/builds/results, bounded subprocesses, no verify-from-component recursion or candidate skip control. |
| Concurrent pin/preflight or decision-number changes | Root refreshes reservations; leave tooling and verify-seat runtime code alone, reconcile final hashes after authorized integration and rerun gates. |
| Old checkbox is broader than this story | Sub-obligation evidence map; missing AC1–AC13 proof blocks D04 completion without authorizing unrelated code work. |
