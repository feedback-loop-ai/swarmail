# Plan: metric parser proof

Implements [spec.md](spec.md). The available `.specify/templates/` has no plan
template; the required sections are supplied here. Only this directory's
spec.md, plan.md and tasks.md belong in the chief's design commit.

## Operator clarification — 2026-10-24

The checksummed operator ruling replaces former R1 and upstream-parser AC3
with in-repo conformance proof. No upstream tool discovery, install, capture
adapter or parser provenance task remains. Historical receipts below and in
other metric plans remain dated history. This plan's research, contracts,
quickstart and tasks are revised together; they do not require the retired
proof. No runtime semantics or accepted decision is changed.

## Technical context

Single Rust 2024 crate using Tokio/Axum. `src/api.rs::inbox_of_path` selects
then percent-decodes one segment strictly; invalid UTF-8 retains the full raw
segment. `count_in_flight` owns guards through `next.run` and the cfg(test)
response hold. `render_rows` calls `prom_label`, escaping exactly backslash,
quote and LF. `tests/common/metrics.rs::parse_prometheus` reads the complete
emitted subset character by character without calling the encoder.

The gap is fixture completeness and independent byte assertions, not a known
runtime defect. Additional `/count` requests use no await registrations.
The existing real-TCP helper keeps body bytes but discards headers; add only
local completeness/framing assertions needed to prove those bytes represent
the complete body. Require observed Content-Length to equal body length and
refuse unsupported transfer framing; do not build a generic HTTP stack.

## Constitution check

Preserve 0065, 0043, 0046 admission and 0047 cap/overflow. Frozen 0001–0003
and ingest extraction under 0005 remain unchanged. Under 0020, branch-stable
helper fixtures stay inline; cold timeout diagnostics stay in the existing
owner-declared cfg(test) refusal harness. No new seam, dependency, lockfile,
license allowance, gate, floor, tooling pin, recipe or accepted-decision edit.
No new ADR is needed for this compatible proof. A concrete apparent semantic
conflict is reported with exact evidence; it does not authorize changing the
settled identity or inventing an amendment.

Storage commit-before-visibility/reply, oldest-only retention, wait watcher
ordering, pre-insert DATA/chaos refusals and ingest extraction remain boundaries.
Webhooks remain bounded best effort, not a durable outbox. No certificate
expiry policy or wall-clock fixture is introduced. Metrics remain process-local
activity and do not prove durable delivery or cross-family atomicity.

## Project structure and files touched

| Path | Planned work |
| --- | --- |
| `tests/api.rs` | Extend `the_strict_prometheus_reader_accepts_the_format_and_nothing_else`; retain the two-await and invalid-UTF-8 regressions. |
| `src/api/refusal_tests.rs` | Extend the existing sequential hold owner with explicit URI/decoded/wire fixtures, complete raw body assertion, strict parsing, exact sample attribution and release cleanup. |
| `tests/common/metrics.rs` | Reuse unchanged unless a concrete oracle defect requires a narrow correction with its own controls; no second grammar. |
| `src/api.rs` | Inspect decoder/renderer/guards and execute existing inline fixtures. Add branch-stable test cases only if a concrete gap demands them; no planned production change. |
| `specs/metric-parser-proof/{spec,plan,tasks}.md` | Current coherent proof contract; check implementation boxes only with evidence. |
| `specs/decoded-inbox-metrics/plan.md` | Downstream dated implementation receipt with commands, candidate binding, fixture/headroom audit and red/green evidence, preserving old receipts. |
| Earlier decoded/refusal/continuation artifacts | Downstream dated pointers only if a current instruction conflicts with this replacement; preserve historical receipts and completed work. |

No new capture archive/schema or harness file is planned. The design commit
must touch exactly these three owning design files; downstream documentation
reconciliation is separately scoped to the implementation candidate. The
prior historical receipt's run-local runner is absent in this checkout; it
is not a prerequisite. Use the available canonical gate commands directly.

## Research and synthesis

Read both positions, current intake, verified operator packet, decisions index,
0065, 0043, 0020 and routing authority; inspect the actual decoder/renderer,
shared strict reader and hold owner. The spec explicitly chooses the full
finite C1/bidi matrix over representative-only coverage. Source review shows
bidi formatting scalars must not be assumed to satisfy `char::is_control`.
Fixture expected status/body follows the existing validator, not a new policy.
No external parser, journal authentication or universal compatibility claim.

## Data model and contracts

Use a small test-local fixture table: exclusive ASCII case ID/prefix, explicit
percent-encoded segment, independently specified decoded UTF-8 string,
expected escaped label/sample bytes, expected status and exact response body.
Use Rust literal code-point escapes for reviewable C1/bidi source and explicit
percent spellings for request input. Do not derive expectations using the
production percent decoder/renderer or generate a second encoder. All 32 C1
scalars and the spec's 12 bidi scalars are finite data, not new abstractions.
Include individual escape cases, backslash-n versus LF, CR/NUL pair and one
mixed case. Compare `as_bytes()`; print escaped/hex diagnostics only on failure.

The proof has two independent comparisons: raw observed sample bytes against
literal expected exposition, and full-reader sample labels against expected
UTF-8 bytes. Whole-body parsing rejects malformed suffixes; exact fixture row
sets reject extra syntactically valid samples. Never trim body/labels, use
Unicode line splitting, or compare terminal appearance. Keep raw body bytes
until both assertions have run. A local emitted-subset reader is the promised
oracle; Content-Type negotiation by external scrapers is outside this proof.

## Implementation sequence

1. Extend the existing strict reader's positive/negative fixture test. Keep
   the existing grammar unless a reproducible oracle defect warrants repair.
2. Retain the CR/NUL await refusal. Add sequential held `/count` fixtures
   in the same owner using the existing exact-URI installer and request-owned
   reached/release handoff. A real response must exist before observation.
3. For each fixture observe before, held and released states over real TCP.
   Assert complete HTTP framing/body, exact wire bytes, full parse, exactly
   one attributable request row at value 1, no raw alias or extra attributed
   family/row, unchanged status/body and no await registration for refusals.
   Release, join and rescrape for disappearance. Preserve bounded task/server
   ownership on failures and endpoint drops; do not hold a registry lock.
4. Replace the old global CR/NUL filter with exclusive fixture attribution
   when adding the matrix, retaining injection/cardinality assertions. Audit
   both test executables' cumulative registrations against 4096, including
   parallel tests; `/count` adds zero. Fresh stores do not reset static metrics.
   Isolate only if demonstrated interference defeats the bounded approach.
5. Run the existing same-scrape space/non-ASCII lifecycle, invalid/mixed UTF-8,
   helper route/plus/slash/double-decoding and hold drop/cancellation tests.
   Add no redundant second implementation of these proofs.
6. Demonstrate sensitivity with a deliberately wrong expected escaped byte or
   identity: the relevant targeted test must fail its exact assertion, then
   pass after restoration. Keep mutation logs separate from final receipts;
   production behavior need not be mutated. Preserve historical old-helper
   sensitivity as history, not a freshly repeated experiment.
7. Record one dated implementation receipt and reconcile only conflicting
   current guidance. Freeze the implementation candidate, run all gates,
   and leave review disposition to the existing protected engine route.

## Quickstart and validation

```bash
cargo test --locked --test api
cargo test --locked --lib api
cargo test --locked --lib api::refusal_tests
cargo test --locked --test store_contract_docs
cargo test --locked --test delivery_recipes
```

Run every command in [the house inventory](../../docs/house-rules.md#gates--part-of-every-change),
using `.forge/bin/brokkr compile --bundle bundles/self` after matching SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`
to the operator pin and `bundles/self/tooling.json`. Check public shims against
`bundles/self/scripts/shims.sha256`; canonical bundle proof scripts supply
coverage, selftest, publish and seal proofs. Verify also retains its design
selftest and delivery tamper proof. Prove native-tls absent fail-closed using
a successfully resolved nonempty locked tree and the specific package-not-found
inverse query, rejecting unrelated query errors. Install nothing.

The chief checks links and whitespace, commits exactly the three design files
unsigned, leaves that commit last, runs local house gates, and runs
`bash bundles/self/scripts/design-check.sh check .`. Logs are run-local with
actual HEAD/tree. The engine's pinned check owns its final verdict; the chief
reports only `drafted`. Design gate passes cannot check implementation tasks.
CI/operator-only proofs and protected review remain with their owners as
specified in the spec; no locally unprovable criterion is added.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Circular oracle or permissive parse hides encoding loss | Literal independent wire/decoded expectations, negative syntax controls, full parse and exact row sets; local conformance only. |
| One combined case conceals a scalar omission | Individual full finite matrix plus mixed case; more fixture data and HTTP operations. |
| CR/NUL/bidi transformed during capture | Keep byte vectors; exact framing/length and `as_bytes()` comparisons; no trim, Debug encoder or Unicode line split. |
| Bidi accidentally treated as refused controls | Preserve actual validator predicate and response bytes; no production validation change. |
| Transient request row disappears before scrape | Existing post-response hold, reached/release handshake and bounded joined ownership; no sleeps. |
| Global registry interference or cap exhaustion | Exclusive prefix, exact attribution and executable-wide upper bound; conditional isolation only on evidence. |
| Passing tests erase inherited security concerns | Preserve true/low and require existing five separate protected results; no author clearance. |
| Unnecessary scope or coverage shrinkage | Existing owners only, cold diagnostics under 0020, no production refactor or exclusion; exact zero-missed gates. |
| Historical receipt mistaken for current proof | Dated clarification, unchanged historical text and fresh HEAD/tree-bound logs. |
| A gate fails in this environment | Record exact command/diagnostic and leave proof outstanding; no pin/floor weakening or historical-pass substitution. |

## Historical chief validation receipt — 2026-10-24

The following prior receipt is retained verbatim as history. Its upstream R1
references describe the retired proof, and its gate results do not certify
this revision. Current verification records are separate and candidate-bound.


Executed the existing offline gate runner against baseline
`6696a48e7271b7aaaea8bdebc525be19cd24fbf3`, tree
`d34bfadbbf8602f20847fec1d003a52e445d8a1f`, plus the three uncommitted design
files. No source/test/decision/policy changed. The runner's ordinary Git diff
hash excludes untracked drafts; their separate SHA-256 inventory is
`.brokkr/state/design/metric-parser-proof-checks/draft-identity.json`.
This receipt and the adapter-reproducibility clarification were added after
those baseline checks; this is not final implementation verification.

Exact invocation:

```bash
env CHECK_ROUND=chief-parser-proof python3 .forge/epic192/metric-review-continuation-implement/run-checks.py
```

The existing runner selects a new evidence subdirectory, leaving prior
receipts untouched. Fresh commands, exits and individual logs are under
`.forge/epic192/metric-review-continuation-implement/chief-parser-proof/`.
The workspace tool response timed out after 300 seconds; subsequent direct
inspection found `current.txt` equal to `finished`, all 16 `results.json`
entries at exit 0 and matching successful logs. The response timeout is not
the evidence of success; those completed execution records are.

Passed: targeted integration API, library API, held-refusal, document-contract
and delivery-recipe suites; formatting; locked all-target Clippy with warnings
denied; full locked workspace tests; digest-bound bundle compile (76
valuations, 14 groups, zero unruled); exact coverage and its selftest;
publish-guard proof (51 mutation-red/restored-green receipts); seal proof
(7 cases, 10 mutants); licenses and fail-closed native-tls absence. All seven
public shim hashes matched. The brokkr binary digest was
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`,
matching `bundles/self/tooling.json`. Canonical bundle proof scripts ran;
public dispatch shims were checked, not used to choose the proof algorithm.

Coverage in `target/coverage/run.FQHqyi` was exactly 12,981/12,981 lines,
764/764 branches and 1,437/1,437 functions, with zero missed. These passes
preserve existing behavior; they did not execute the new upstream matrix,
resolve R1, clear true/low or supply a fresh protected panel. CI-only proofs
remain unclaimed. Relative artifact links and literal-control checks passed;
staged whitespace and committed HEAD structure are checked at commit time.
