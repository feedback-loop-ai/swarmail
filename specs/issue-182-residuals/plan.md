# Implementation plan — issue 182 residuals

[spec.md](spec.md) owns AC1–AC8 and panel dispositions;
[tasks.md](tasks.md) pairs the implementation order with proof.

## Technical context

One Rust crate with existing Tokio/Axum wire tests. This patch changes two
published guides and test code only. No new runtime data, dependencies,
features, storage schema, API or resource bounds. Existing phrase checks
normalize whitespace but use case-sensitive substrings, which lets an
obsolete forbidden phrase certify nothing and lets a phrase elsewhere
satisfy the wrong claim. The existing Unicode test covers Über/über, while
ß/ss is only a comment. The seed constructor already refuses hostile fields.

The repository has `.specify/templates/spec-template.md` but no plan template
in its tracked inventory. This plan supplies the required technical context,
constitution check, project structure, research, data model, contracts and
quickstart directly; no template or auxiliary artifact is added.

## Constitution check

Preserve accepted 0001–0005, synchronous Store insertion and queryability
before SMTP 250, subscribe-before-check waits, oldest-only eviction and
ingest-only extraction. Preserve all accepted refusal envelopes, thread key
semantics and POP3 snapshot behavior. No proposed amendment is necessary.
Inline deterministic tests stay measured under 0020; no seams, sleeps,
random IDs, clock-dependent fixtures or cold search/diagnostic branches are
introduced. The unchanged house gate inventory and exact coverage apply.
Unsigned commits only; never push. Recorded `engine` classification retains
0057's configured independent review route and hard security hold, without
provider fallback or author-model substitution. No reviewer is launched by
this design-only seat.

## Project structure and files touched

| File | Implementation change | Proof |
| --- | --- | --- |
| `README.md` | Qualify existing MCP introductory rule with both exceptions; add precise seed refusal sentence with 0010 authority and call-id scope. Preserve other issue claims. | Issue-182 checks and defect controls; existing hostile-fixture wire test. |
| `src/static/llms.txt` | Matching seed/timeout qualification and seed refusal sentence, with notification-safe HTTP wording. | Same applicable document checks and controls. |
| `tests/store_contract_docs.rs` | Repair only the five issue-182 checks; small local predicates plus in-memory defect controls. | Valid originals pass; each independent defect fails its owning predicate; capitalization/wrapping controls preserve verdicts. |
| `src/threads.rs` | Add one regression inside existing inline `#[cfg(test)] mod tests`. | Actual grouping, membership, counts and unequal returned keys; existing Unicode test stays. |

The chief's separate commit touches exactly
`specs/issue-182-residuals/{spec,plan,tasks}.md`. Run-local positions and
receipts remain ignored. Runtime functions in `src/smtp.rs`, `src/mcp.rs`,
`src/threads.rs`, the wire test `tests/mcp.rs`, decisions and gate/recipe files
are inspection or execution inputs, not implementation edit targets.

## Research and resolved choices

Both positions and intake were read against baseline `72f489a`.
`build_seed_email` in `src/smtp.rs` rejects CR/LF/NUL before constructing the
mail; `src/mcp.rs` maps its failure to `Ok(error_result(reason))` after string
argument parsing. The real-server hostile-envelope regression already checks
the three fields across REST and MCP, with no insertion. Reuse it, rather
than introducing a second validation owner or duplicating its matrix.

`src/threads.rs` already provides `mail`, `group` and `ids_of`; `group`
invokes `group_threads`. Use two chain-less mails at 100 and 200 with
subjects `ß` and `ss`. Assert length before direct indexing; newest-first
order puts the 200 fixture first. Assert count and exact membership of each,
then compare their actual returned keys. Expected `thread_key` assertions
are optional supplementary evidence, never the entire key regression.

For prose, require the operation, condition and result together in the
owning passage, rather than tokens anywhere in the document. Use a small
issue-182-local predicate/checking function returning a boolean or diagnostic
result that can be asserted for both real text and defects without expected
panics. Normalize whitespace and ASCII case locally; keep literal Unicode
subjects intact. Allow document-specific exact clauses where README and
llms.txt wording differs; do not change global helpers or create a DSL.
A useful implementation is a few named predicate functions, not a parser.

## Implementation approach

1. Update the two MCP passages together. Explain the seed fixture exception
   adjacent to the timeout exception, and qualify the earlier general rule
   in both places. Cite accepted 0010 without suggesting it is amended.
   Keep call-with-id HTTP scope and notification silence explicit. Keep
   wrong argument types/keys distinct from malformed string field content.
2. Add `the_subject_fallback_keeps_sharp_s_and_ss_distinct` beside the
   existing Unicode test, reusing helpers and deterministic direct-index
   assertions. Leave production bytes and the Über/über proof unchanged.
3. Repair the five documentation tests locally. Require coherent clauses
   for AC1–AC5. Remove ineffective old negatives rather than merely
   lowercasing forbidden substrings that also occur in qualified prose.
4. Add AC6's independent in-memory defects through the same predicates.
   Start each control from the corrected original, assert replacement
   changed it, and assert the owning predicate rejects it. Distinguish
   removing the introductory exception from removing its detailed sentence.
   Separately remove field reason and no-insertion clauses. Check valid and
   defective ASCII-case/whitespace variants; recheck originals afterwards.
   No production source mutation, external mutation runner or dependency.
5. Run focused proofs, inspect the diff, then run every house gate and the
   native-tls absence proof. Bind logs to the actual candidate and commands;
   baseline evidence is not a pass for later implementation. Keep failures
   visible; do not widen scope to fix unrelated code or weaken a gate.
6. Hand the unchanged classification to configured verify/review. Correctness,
   security, guarantees and tests remain independently reviewed by their
   selected providers, including only official DeepSeek Flash when selected.

## Data model

No production model change. Test data comprises two existing Email fixtures
(no threading headers) and immutable original/modified document strings.
A defect case identifies its document, owning predicate and one clause edit;
there is no persisted fixture format, mutation history or application state.

## Contracts

AC1–AC3 preserve MCP protocol errors, tool results and REST extractor errors
as separate existing contracts. AC4 preserves subject fallback identity and
actual returned thread keys; it makes no universal Unicode/collision claim.
AC5 preserves PASS-time numbering and live-store lookup skew. AC6 proves the
named documentation defects are detectable, not arbitrary contradictory
English anywhere in either guide. Independent review covers that remaining
editorial risk. Search folding is not authority to alter thread identity.

## Quickstart and validation

From the repository root, implement only the four-file patch, then run:

```bash
cargo test --locked --test store_contract_docs issue_182
cargo test --locked --lib threads::tests::the_subject_fallback_keeps_sharp_s_and_ss_distinct
cargo test --locked --lib threads::tests
cargo test --locked --test mcp hostile_envelope_fields_are_refused_on_both_surfaces
cargo test --locked --test store_contract_docs
```

Follow the complete [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
with the recorded digest-matched Brokkr, pinned verification copies and shim
checks. Preserve CI-only proof ownership. No local command authorizes release.
For this chief commit only, preflight with
`bash bundles/self/scripts/design-check.sh check .`; the engine's subsequent
immutable pinned check remains authoritative. HEAD must touch exactly one
spec directory, contain the three nonempty clean files and remain the last
chief commit. No new decision declaration is needed.

## Risks

| Risk | Mitigation and retained cost |
| --- | --- |
| A general argument rule still contradicts a correct seed sentence. | Check the introduction and sentence independently; remove each separately in controls. |
| Fixture-content wording swallows wrong types, keys or notifications. | Bind conditions to the named string fields and identified calls; preserve ordinary argument and notification clauses and existing wire proof. |
| Phrase checks certify unrelated prose or capitalization. | Scope coherent clauses, normalize locally and prove defects/case variants through the same predicates. Editorial coupling remains intentional. |
| A hash-only test misses actual merging or key conflation. | Assert returned conversation count, membership and unequal returned keys; fixed timestamps make indexing deterministic. |
| A new inline predicate introduces a coverage cold path. | Reuse existing helpers and direct assertions; keep document diagnostic controls in the integration file; run the exact coverage gate without exclusions. |
| Broad robustness work expands a residual repair. | Four-file boundary and final diff audit; no new runtime behavior or infrastructure. New evidence requiring scope expansion returns through triage. |
| Baseline passes get mistaken for delivered fixes. | Keep implementation checkboxes open until final-candidate proof; preserve command/commit provenance and required independent review. |
