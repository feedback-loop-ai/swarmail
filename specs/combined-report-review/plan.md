# Plan — Combined report review

[Specification](spec.md) · [Ordered tasks](tasks.md).

## Technical context

One production Rust crate with existing serde/mail-parser, plus a standalone eval
workspace. Preserve static collector composition, Service and Store's wait owner.
No dependency, schema, rule, version, executor, persistence or route change is
planned. Add necessary durable proofs and narrowly compatible repairs of actual
findings. Semantic changes return to design; historical specs remain unchanged.

No plan template exists in `.specify/templates/`; this document provides its
required sections directly. The intake task directory is absent: use the supplied
task, recorded feature and intake result explicitly. Initial spec and plan writes
exceeded the workspace tool's 16,384-character command limit and were refused;
shorter writes followed. Those refusals produced no artifacts or clearance.

## Constitution check

Preserve 0001–0005 and accepted amendments: synchronous acceptance/storage,
subscribe-before-scan including resubscription, oldest-only pruning, isolated
inboxes and ingest extraction. Follow 0020 placement: branch-stable tests inline,
cold diagnostics in declared sibling harnesses, wire contracts in integration
tests. Use barriers/deadlines, chaos poles, fresh inboxes and generated certificate
fixtures; no sleep synchronization, mocked protocols or coverage exclusions.

Proposed 0085/0089/0090/0091 already describe the behavior. No new semantic ruling
or ADR is selected. Report 0081 provenance without changing status. Preserve
0058 protection and 0050 candidate binding. The explicit chief instruction permits
exactly three unsigned design files before ship despite the general ship-only
commit restriction; it authorizes no other commit or publication. All workspace
writes, Git mutations and the phase result use the boxed tool. The sequence check
owns the design verdict; downstream protected phases own verification/review/ship.

## Research and settled approach

Read both full positions, intake result, checksummed original packet, complete
stopped-run audit, decision index, 0001–0005, 0058, 0081 and proposed
0085/0089/0090/0091. Inspected full change inventories and selected collector/cache,
CSS/work, decoder-census, report/empty/CSS tests, mixed overlay and eval owners.
Direct comparisons establish historical restoration and index structure at the
starting candidate. The engine digest matches tooling.json. These observations
are design grounding, not exhaustive semantic review or gate results.

Retain existing owners, strengthen complete CSS expectations, map every empty
control, and require a fresh actual-read negative control under the fixed bound.
No generic fixture builder, proof runner or evidence schema is needed. Census
adequacy, full merge correctness, any missing durable cells, coordinated evals,
hermetic proof and actual independent results remain downstream obligations.

## Project structure and files

| Paths | Treatment |
|---|---|
| specs/combined-report-review/{spec,plan,tasks}.md | Exactly the chief's tracked changes and commit contents. |
| tests/deliverability.rs | Strengthen independent complete CSS expectations; map/extend empty controls, sanitized MCP errors and final raw/message retention. Keep 15-finding oracle. |
| tests/stdio.rs; tests/bulk_sender.rs; tests/fixtures/html-compatibility.json | Run/inspect adapter, auth and catalogue proofs; change only demonstrated missing assertions. |
| src/deliverability.rs; src/deliverability/html.rs; src/deliverability/html-rules.json | Audit admission/finish, immutable Inputs/auth, cache/worker ownership, catalogue, CSS/work; use existing inline tests for necessary local additions. No speculative production refactor. |
| src/extract.rs; src/deliverability/concurrency_tests.rs | Assess counted decoder/output parity; retain declared concurrency harness and actual seam installation. |
| src/store.rs; src/smtp.rs; src/persist.rs; src/model.rs; webhook modules | Review accepted invariants, cancellation, post-storage fan-out and shutdown; run existing proofs. |
| tests/burst.rs; tests/p2.rs; tests/rate.rs; remaining root suites | Run full regression scope; default ignored rate execution is no tag-rate proof. |
| evals/src/{artifacts,provenance,results,wrappers}.rs; evals/tests/{boundary,identity,provenance}.rs; evals/Cargo.toml; evals/README.md | Review both merge sides, deletions, v2 readers, privacy/tool/trial semantics and comments; full suite after coordination. |
| README.md; src/static/{llms.txt,openapi.json}; docs/decisions/README.md and 0081/0085/0089/0090/0091 | Verify served wording, identity/status provenance; preserve history/status. Current-doc repairs only for actual contradictions. |
| agents/mixed-{terra-intake,opus-implementer}.json; adapters/codex.json; bundles/mixed-opus-terra/ and inherited offices | Assess effective identities/protection; no route or fallback change. |
| tests/delivery_recipes.rs; tests/delivery_constitution.rs; .github/workflows/ci.yml; bundles/self/; scripts/; Cargo.toml; Cargo.lock; deny.toml | Prove routing, workflow comments, pin/shim/gate/dependency integrity; never weaken gates. |
| Ignored evidence and designated phase result | Existing genuine records only; no synthetic journal/reviewer/verified-candidate/ship events. |

Git's full changed-path inventory, including the empty capability-probe file and
historical eval ledger, controls scope beyond this table. Record actual merge
parents. Equality with one side or removal of conflict markers does not establish
that deleted assertions were redundant.

## Data model and contracts

Report remains `{rule_set_id, findings}` with typed coordinates, fixed hints and
canonical ordering/deduplication. Combined html/3 is distinct from historical
HTML-only html/2. Full immutable Inputs and validated Weak mail identity govern
cache matches. Header admission fails on empty raw before bulk/HTML; HTML's own
empty branch only contributes nothing. Finish precedes success publication.
No database or wire model changes are planned.

Use existing evidence formats: criterion/finding, exact source/assertion, HEAD/tree,
command/exit/log digest and pending reason. Distinguish passed, failed, refused
and not executed. Audit hash equality identifies bytes without independent
source authentication; retain the limitation. Narrative cannot create typed
clearance. Every threshold and count 0/1 requires these distinctions:

| State/operation | Expected outcome |
|---|---|
| Stored zero raw, exact REST assert | Fixed analysis-failed 503, including exact zero. |
| Stored zero raw, positive-deadline REST/MCP wait | Analysis failed: REST 503, MCP isError/fixed text; count zero means minimum one. |
| Stored zero raw, cold zero-time REST/MCP wait | Unavailable; submit no cold work. Previous failed evaluations do not create a ready success. |
| Truly empty inbox, exact assert | Zero succeeds; one mismatches; no required analysis. |
| Truly empty inbox, REST/MCP wait | Existing timeout/zero matches for zero and one counts, at zero and positive deadlines. |

Map existing tests before adding duplicate cells. Preserve trusted-input/evaluator
positive controls because public Unknown errors mask HTML warning discrimination.
Check sanitized payloads, not only flags, and keep unchanged raw/message data.

## Implementation sequence

1. Bind the candidate/checksums and create a run-local AC/finding map, including
   every original finding and R1–R6. Review full original HTML/repair, both
   integration sides and follow-up, including non-HTML paths.
2. Recheck historical bytes/prefix, one-table index and every living/served
   html/3 owner versus intentional html/2 history. Record both 0081 provenance
   facts. Historical specs' stale proof claims stay historical.
3. Extend CSS wire pairs with explicit bulk and HTML expectations independent
   of responses. Reuse small local literal construction, not a generic builder.
   Keep catalogue snapshot negatives and the 15-finding oracle. Map each empty
   control and sanitized payload; strengthen missing cells in existing suites.
4. Review CSS/URL/helpers/decoder against the census; run focused tests. Use a
   disposable candidate-bound copy for actual counted repeated reads. Capture
   mutation/source/binary/test/log hashes, progress success and unchanged-bound
   failure; restore byte-exactly and rerun green. Never mutate the final candidate,
   fabricate increments, raise C/K or commit a hook. Preserve distinct historical
   experiments and numerical outcomes as attributed executions.
5. Run bulk/auth/cache/boundary/atomic-failure and guarantee proofs. Retain actual
   failures before narrow compatible fixes. Semantic contradictions return to
   design. Rebind final evidence after any change.
6. Run full local gates and separately coordinated evals. Missing prerequisites
   remain pending; no repeated unchanged passing suites without cause. Root tests
   prove neither standalone evals nor hermetic verification.
7. Hand exact candidate/evidence to engine-owned hermetic verify then protected
   review. This sole design seat launches no replacement panel. Policy determines
   advancement; preserve every actual residual, unknown and refusal.

## Validation and quickstart

The only gate inventory is
[house rules § Gates](../../docs/house-rules.md#gates--part-of-every-change).
Use offline locked resolution, no installs, and absolute candidate-local
`.forge/bin/brokkr`, SHA-256 checked against tooling.json (currently
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`).
Digest equality identifies bytes, not reproducible build provenance.

Run all inventory commands: fmt, locked workspace/all-target clippy with warnings
denied, locked full tests, pinned-engine bundle compile, exact coverage and
selftest, publish/seal proofs, licenses and fail-closed native-tls absence.
Check shims against the pinned manifest; execute engine-mounted proof owners,
not public shims. PIN_DIR means the actual engine mount, normally
`/runtime/bundle/scripts`, absent here. Candidate-copy execution can be an
observation but cannot emit a hermetic verified_candidate. Record command, exit,
log digest and candidate identity. Exact coverage requires integer missed==0
for lines, branches and functions, with selftest passing.

Also run pinned design-check selftest and delivery-proof-test.sh, existing
recipe/constitution proofs, package inclusion inspection, mixed bundle and
inherited route compilation. Policy-sweep warnings refuse even at exit zero.
Successful `cargo tree --locked -i native-tls` means presence and fails; absence
requires nonzero plus the specific package-not-found diagnostic, not another
Cargo error. Focused commands supplement full gates:

```bash
cargo test --locked --lib deliverability
cargo test --locked --lib inspected_work_has_a_fixed_linear_bound_separate_from_progress
cargo test --locked --test deliverability
cargo test --locked --test bulk_sender
cargo test --locked --test stdio report_stdio_text_contract
cargo test --locked --test burst
cargo test --locked --test p2
cargo test --locked --test rate
cargo test --locked --test delivery_recipes
cargo test --locked --test delivery_constitution
```

Before standalone evals, obtain authorized same-user suite-wide nonoverlap and
cleanup coordination including other runs. This box cannot observe host processes;
a free trial lock proves neither suite completion nor descendant cleanup. Never
bypass/unlink admission, hold a trial reservation around the suite or retry
through contention. If coordination is unavailable, leave AC10 unresolved and
continue independent work. No new lock/watcher framework or paid trial.

After coordination, build the root executable offline/locked and select its path
from Cargo's artifact report, honoring CARGO_TARGET_DIR. Copy to an isolated
run-local capture, verify both binary digests, record source HEAD/tree/build
command and set SWARMAIL_BIN to the absolute capture. Never inherit a stale
binary or allow a concurrent rebuild to replace it. Then run:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked
```

Record suite interval/cleanup evidence; retain refusals. Local stand-ins do not
prove paid-provider interoperability or confinement. Resolve mixed-opus-terra
to Terra `gpt-6.1-terra` intake and Claude `claude-opus-5-5` implementation with
bounded workspace hands. Require inherited separate `gpt-6-astra` correctness
and guarantees, `deepseek-flash` via `deepseek-official` security, local
`spark-glm/GLM-5.3-Flash-EXL3` tests via dsh with absent effort, and independent
`claude-fable-5-1`. Compilation proves routing intent; candidate-bound actual
invocations/results prove review. Preserve gate/read-only offices, charters,
limits, worst verdict, unknown-fact refusal and security-owned hard hold. No
DeepSeek Pro, author substitution or outage fallback.

For design handoff, check the three staged files, whitespace and relative links;
commit unsigned with imperative title/rationale body and leave it last. Run
available design-check check/selftest as structural prechecks; the engine-pinned
sequence check owns the verdict. Full gates, evals and protected review remain
subsequent obligations, not claims of drafted. Write exactly result/inputs/notes
to the designated phase result.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Design/repair changes candidate identity | Record baseline/descendant and bind final proofs anew. |
| Common wrong report passes parity | Independent complete expectations plus catalogue snapshot; explicit fixture maintenance. |
| Unknown masks HTML threshold errors | Trusted-input/evaluator positive controls alongside public Unknown proofs. |
| Empty shortcut conceals failure | Full state/threshold/count mapping, sanitized errors, retention and retry proofs. |
| Work census omits helpers or overclaims CPU bound | Independent census, fixed-bound actual-read control, explicit exclusions; stop on unknown adequacy. |
| Mutation contaminates delivery | Disposable copy, hashes, byte-exact restore and green; no committed hook. |
| Merge deletes eval guarantees | Full both-parent semantic review and separate standalone suite. |
| Eval overlap/orphans | Authorized suite-wide window and cleanup evidence; honest deferral. |
| Missing mount/resources misclassified | Preserve failure classification; candidate copies cannot issue verified_candidate. |
| Historical evidence/status treated as current clearance | Attribute audit and 0081 discrepancy; preserve history/proposed statuses. |
| Outage or invalid result removes protection | Separate mandatory offices; keep refusals/unknowns and hard hold; no substitutions. |
| Advanced main/#260 invalidates proof | Fresh candidate binding, verification and review after final integration. |
| Shared-pool/storage limits misunderstood | Preserve ownership/burst/persistence/webhook proofs; no stronger latency/replay/retention promise. |
