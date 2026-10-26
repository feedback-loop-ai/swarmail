# Implementation plan — HTML client compatibility

[Specification](spec.md), [proposed 0090](../../docs/decisions/0090-html-client-compatibility.md).
This is design, not evidence of implemented findings.

## Technical context

One Rust 2024 crate with mail-parser, serde/serde_json, Tokio and existing
bounded lazy deliverability service. The requested plan-template.md is absent
from `.specify/templates/`; this plan supplies its technical context,
constitution check, project structure, research, data model, contracts and
quickstart directly. No extra design artifacts or dependencies are needed.

## Constitution check

Preserve 0001 synchronous storage before 250, 0002 subscribe before scan, 0003
oldest-only eviction, 0005 ingest-only stored extraction and 0031 sender-honest
fields. The operator's #251 direction authorizes this separate derived report;
0085 remains proposed. Compatible implementation may proceed under 0045 §8;
no accepted ruling is amended or accepted by this seat. Keep 0085's cache,
worker ownership, exact predicates, cancellation and sanitized failures.
Tests follow 0020: deterministic tests inline, only concrete cold diagnostic
paths in owner-declared cfg(test) harnesses, real servers for protocol proof.
No coverage exclusions, allowlist widening, routing edits or new service owner.

The final chief commit contains exactly spec.md, plan.md, tasks.md and proposed
0090. A separate prerequisite commit supplies 0090's proposed index row: the
repository inventory tests require it immediately, so deferral to implementation
is invalid. Do not edit historical specs or
accepted records. Run-local receipts stay outside specs, so HEAD remains the
single design commit and all design files remain clean.

## Research and resolved choices

Both contrarian positions, verified issue packet, tracked intake, decision index,
0001–0005 as applicable, 0031/0040 and proposed 0085 ground the spec. The missing
run-local task directory does not replace the checked-in intake. Catalogue
references are authored guidance, not fetched client-support records. The spec's
adjudication owns exact choices and costs; implementation must not substitute
a different clipping unit, byte threshold or image model silently.

Current `collect` has the synthetic demo and `Collector::push/finish` already
owns ordering and bounds. `Inputs` already binds version/config to each Weak
identity. MIME support and JSON are existing dependencies; `/src/**` and
`/tests/**` package includes cover the proposed catalogue and fixtures. The
extractor's private Regions walker lacks the attributes/CSS interface needed
here; do not refactor ingest parsing. Its bounded `decode_refs` can become
crate-private unchanged for reuse; retain all existing entity/progress tests.
Inspect the pinned mail-parser APIs locally for body types, structural children,
filename/disposition and encoding-error flags before implementing the walker.

## Project structure and files

| Path | Planned work |
|---|---|
| `src/deliverability.rs` | Declare private HTML module, wire collector, bind aggregate identity to embedded data, retain demo/service behavior; update version expectations and owner tests. |
| `src/deliverability/html.rs` | Closed catalogue types/validator, lazy initialization, bounded MIME walk and HTML/CSS scan; no cache or transport owner. Inline deterministic tests. |
| `src/deliverability/html-rules.json` | Seven rows, source summaries/provenance, selector parameters, limits and aggregate identity. |
| `src/extract.rs` | At most expose existing bounded entity-decoder helper crate-privately; no semantic/tokenizer changes. |
| `tests/fixtures/html-compatibility.json` | Reviewed catalogue snapshot/version, row-paired HTML/MIME cases and generated boundary recipes with exact expectations. |
| `tests/deliverability.rs`, `tests/stdio.rs` | Extend real-server parity, predicates, restart and failure witnesses with HTML. |
| Existing deliverability/store owner tests | Keep cache/cancellation/ready-only/watcher/prune/ingest invariants; use HTML input in relevant cases. Cold-path harness only if 0020 requires it. |
| `README.md`, `src/static/llms.txt`, report descriptions in `src/static/openapi.json` | Replace demo-only explanations with versioned finite HTML rules, known gaps, limits and quickstart; schema stays unchanged. |
| `docs/decisions/README.md` | Verify the already-added proposed 0090 index row; do not change 0085 or accepted statuses. |
| SMTP, persistence, model, webhook, routing, Cargo.toml/Cargo.lock/deny.toml | Preservation owners; no planned production edits. |

## Data model and implementation approach

1. Define closed serde catalogue/selector structs. Use a Result-valued immutable
   once-cell initialized only from the lazy evaluator; pure parse/validate accepts
   bytes for tests. Static selectors cover CSS property/value, element, missing
   attribute, image-only, decoded bytes and remote small image. Validate every
   field that influences behavior. Keep fixed strings in data, not match arms.
2. Keep a compile-time `RULE_SET_ID` constant matched to catalogue version by
   mandatory tests, retaining exact Inputs identity without parsing on ingest.
   No new runtime configuration surface. Test a fixture version/selector through
   pure evaluation; public data is immutable.
3. Preflight raw length, retain the demo header rule, then parse MIME once.
   Iteratively map structural children to paths, apply subtree exclusions and
   verify post-parse part/depth and decoded-byte budgets. Use genuine typed HTML
   bodies; never synthesizing body accessors or Email.html as a complete MIME view.
   Detect eligible-part decode errors through existing parser metadata; pin any
   tolerated decoder replacement in tests instead of claiming strict losslessness.
4. Scan each eligible body with source indices and bounded nesting counters.
   Emit text/attributes/style declarations to local accumulators; decode once
   after boundaries. Preserve string/comment/parenthesis awareness in CSS without
   layout or identifier-escape interpretation. Assert monotonic scan work with
   per-invocation counters in tests, never global statistics or timing assertions.
5. Maintain fixed-size per-rule match flags for each part. Finish all bounded
   scanning before emitting its one contribution/rule; then push into the single
   Collector. Any later failure discards the whole evaluation. Existing finish
   canonicalizes and checks encoded report bytes. Retain successful-result-only
   caching and fixed sanitized errors.
6. Extend existing protocol/owner fixtures, update current version expectations
   coherently and living docs, verify the proposed index row, then run all gates. Do not replace synthetic
   regression tests that still prove header/qualification behavior.

## Contracts and proof map

The spec's Contract owns literal IDs/hints, data provenance, MIME eligibility,
lexical grammar, failure boundaries and work bounds. No new public type is
introduced. The existing REST/MCP/UI transport shapes and Store wait logic are
preservation constraints, not parallel implementations. AC1–AC5 map to catalogue
and pure scanner tests; AC6 to real HTTP/SMTP/stdio/restart; AC7 to existing
owner/barrier and burst/p2/rate tests; AC8 to docs, index, packaging and gates.
Fixtures assert target-rule absence/presence and the full expected report where
other rules also match; an image-only pixel legitimately has multiple findings.

## Quickstart and validation

After implementation, deliver ordinary HTML SMTP mail with
`<p>Hello</p><img src="cid:logo"><div style="display:flex">Offer</div>`.
Fetch its existing message ID, then GET `/api/v1/messages/{id}/deliverability`.
Expect swarmail-html/1 with alt-missing and display-flex warnings at root `[]`.
Add `alt=""` and change `display:flex` to `display:block` to remove both.
MCP `swarmail_get_deliverability_report` and the message page show the same
findings. An exact assertion with threshold warning excludes the first message;
threshold error includes it. Empty inbox waits still require at least one mail.

Run offline, with no installations. Focus first on `cargo test --locked --lib deliverability`,
`cargo test --locked --test deliverability` and the existing stdio, burst, p2
and rate suites. Then execute the complete single
[house inventory](../../docs/house-rules.md#gates--part-of-every-change): fmt,
locked clippy/workspace test, bundle compile, exact coverage and selftest,
publish-guard proof, seal proof and license check. Verify candidate engine digest
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` and use
`.forge/bin/brokkr compile --bundle bundles/self`. Verify shim hashes, then run
canonical bundle gate scripts under 0050. Also run design-check/selftest,
delivery-proof-test and fail-closed native-tls absence. Save command/exits/log
hashes tied to commit/tree in run-local evidence; rerun fresh on implementation.
CI/operator-only proofs are assigned in AC8, never simulated or claimed here.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Source guidance ages or is mistaken for empirical certification | Explicit authored provenance and narrow client matrix, fixed risk wording; no live-version assurances. Review/version catalogue updates. |
| Parser scope produces false negatives/positives | Source-aware lexer and fixtures; document conditional-comment/CSS-escape/visibility gaps and small-image false positives. No universal clean claim. |
| Malformed or hostile input appears clean | Explicit malformed/decode/budget failures, complete-result-only publication, real predicate failure proof. |
| Allocation occurs before part/depth validation | Raw 8 MiB preflight plus decoded and structural bounds; inspect pinned parser locally. These are workload caps, not an RSS SLA. |
| Work interferes with ingestion | Preserve existing four worker permits and no store guard during analysis; prove ingest proceeds while evaluations are held. Shared pool contention remains. |
| Larger accepted mail cannot be reported | Report operational failure separately from clipping warning; document both thresholds and test each boundary. SMTP/storage limits remain their existing contract. |
| New scanner drifts from ingest | Separate analysis model with unchanged stored bytes/fields; only reuse existing entity decoder unchanged. |
| Version changes leave stale cache or expectations | Catalogue/version snapshot fixture, full Inputs key and cold/warm/restart parity tests. |
| Excess proof surface under exact coverage | One private module and closed selectors, no generic interpreter; every branch has deterministic fixtures and full exact gate. |
| Design receipt confused with implementation completion | Checkboxes remain open; only final design check decides designed, later implementation executes behavior proofs and fresh gates. |
