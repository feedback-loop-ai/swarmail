# Plan: positioning slogan

## Technical context

One Rust crate with embedded llms.txt/OpenAPI, clap help, server-rendered UI,
a static HTML landing page, Cargo metadata and a Helm chart. Use existing
Rust integration tests, Python/PyYAML chart tooling and shell gates. No new
dependency, build step, runtime abstraction, schema or migration is needed.
The workspace has a spec template but no plan template; this document
supplies the requested dialect sections directly.

## Constitution check

Preserve synchronous accept == stored, subscribe-before-check, oldest-only
pruning and ingest-time extraction, including accepted amendments to
0001–0005. Preserve real-server testing, exact coverage and dependency
policy. The slogan adds no delivery or retention guarantee. Apply
[0050](../../docs/decisions/0050-delivery-gates.md) proof ownership and
[0056](../../docs/decisions/0056-signing-policy.md) unsigned author commits.
No ADR is needed because no accepted contract changes. Preserve candidate
bundle routing. Commit exactly the three chief artifacts, leave panel
positions run-local, and leave the design commit at HEAD for validation.

## Research and resolved choices

Authority hashes and the candidate engine digest were verified. The absent
run-local tasks directory is replaced by the existing committed intake and
verified operator packet, not guessed evidence. Both panel positions agree
on direct copy edits; the [spec](spec.md) explicitly rules their differences.

The current README has badges before its four-line one-binary paragraph.
Insert the slogan before those badges. Its comparative `faster` wording is
explicitly preserved by the task. The landing hero is `header#top` with a
`p.tagline`; move its old contents into an adjacent supporting paragraph and
use the exact slogan in the tagline. Keep existing styles unless wrapping
requires a small local correction, without frontend tooling or dependencies.

Six current `fastest` occurrences were identified in Cargo, llms.txt,
OpenAPI, config, UI and main. Search current tracked descriptions for any
others, classifying historical quotations rather than globally replacing.
The chart's folded scalar needs a decoded-value assertion and its description
change requires chart version 0.1.3. Cargo/appVersion/image stay at their
existing application version. Existing `chart_metadata` parses historical
fixtures and emits version/app; preserve that interface.

## Project structure and files touched by implementation

| Path | Change and proof owner |
| --- | --- |
| `README.md` | Bold exact lead; preserve paragraph, badges and motto; doc-contract test. |
| `docs/index.html` | Exact visible tagline and preserved supporting paragraph; hero-scoped doc test. |
| `src/static/llms.txt` | Exact summary prefix, preserved continuation; doc and real HTTP tests. |
| `Cargo.toml` | Exact package description only; compiled Cargo description assertion. |
| `charts/swarmail/Chart.yaml` | Exact folded description and chart patch bump; decoded metadata/coherence check. |
| `src/static/openapi.json` | Replace only the obsolete info-description opening; real HTTP equality and independent description checks. |
| `src/config.rs`, `src/ui.rs`, `src/main.rs` | Display/help/module-description strings only; existing help/UI and doc proofs. |
| `tests/store_contract_docs.rs` | Exact independent slogan/placement/preservation and current-claim assertions; `env!("CARGO_PKG_DESCRIPTION")` for effective package text. |
| `tests/api.rs` | Strengthen `machine_docs_are_served_from_the_binary` in its existing real-server harness. |
| `tests/ui.rs` and existing CLI/config test owner | Assert emitted header/help text using existing harnesses; do not add a server fixture. |
| `scripts/chart-verify.sh` | Small current-chart positioning mode/check, invoked by ordinary verify; reuse Python/PyYAML, preserve generic metadata/history interface. |

Update additional existing exact-text assertions only if the changed strings
require it. Historical specs, accepted rulings, Cargo.lock, storage/protocol
implementation, bundle routing and pinned gate code are preservation inputs.

## Implementation approach

1. Record base revision and preservation bytes for the README introduction,
   motto, benchmark prose and machine-document guarantee continuations.
   Use independent literals from the spec for positive copy assertions.
2. Apply direct edits in the table, including the chart patch bump and all
   active superlatives. For supplementary standalone descriptions use
   `Built for a program waiting on mail.`; retain factual context where a
   sentence is embedded in a larger description.
3. Extend `tests/store_contract_docs.rs` using existing raw/normalized readers.
   Compare the raw README lead and paragraph; isolate the actual hero before
   normalizing markup/whitespace. Do not let a duplicate elsewhere satisfy
   either placement requirement. Assert effective Cargo description via the
   Cargo-provided compiled environment value. Scope stale-word assertions
   to current descriptions, not a repository-wide historical ban.
4. Extend the real served-doc test: retain status and full parsed OpenAPI
   equality; independently require its corrected opening and preserved
   details. Compare full llms response bytes to the asset and independently
   assert the normalized opening summary. Extend existing emitted CLI/UI
   assertions for their new text and absence of the obsolete claim.
5. Add a `positioning` mode to the chart verifier, executable without Helm,
   checking the current decoded description, a chart version greater than
   baseline 0.1.2, and existing application/image coherence. Invoke the same
   check from normal verify. Keep historical `metadata` and version-history
   fixtures generic. Reuse the existing Python/PyYAML stack; add focused
   current-copy negative fixtures or temporary mutations and run this mode
   locally. The implementation uses 0.1.3 without freezing future versions.
6. Exercise focused refusal cases: changed slogan word, missing boldness,
   lead below badges, slogan moved outside hero, altered effective package
   or folded chart description, missing chart bump and reintroduced current
   superlative. Use disposable fixtures/copies or restore each edited input
   before committing. No generic mutation framework is needed.
7. Review preservation diff, commit implementation unsigned, then run all
   local house and intake proofs. Save command exits and logs as run-local
   evidence, with tested commit identity. Never substitute prior intake
   evidence for the new candidate's results.

## Data model

No new entities, persistence or configuration. The only values are static
full/short slogans, the summary prefix, package description and chart
version. They remain native literals in their owning Markdown, HTML, Rust,
JSON, TOML and YAML formats.

## Contracts

Exact words and punctuation are specified in the spec. README boldness and
position are raw-text requirements. HTML and blockquote line wrapping may
be normalized for prose comparisons; YAML must be decoded before comparing
its scalar. Cargo's compiled metadata is the package truth. The served
assets retain endpoint/status/schema behavior and their existing guarantee
qualifications. Chart parser output and history semantics stay unchanged.

## Quickstart and verification

Run offline and install nothing. Check the engine SHA-256 is
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, then:

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo test --locked --test store_contract_docs
cargo test --locked --test api machine_docs_are_served_from_the_binary
cargo test --locked --test ui
bash scripts/chart-verify.sh positioning
cargo metadata --locked --offline --no-deps --format-version 1
```

Run the existing CLI/config proof selected during implementation as well.
The new chart mode above is an implementation task, not a command available
at design time. Run every house gate from the single canonical inventory:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

These are the pinned implementations of the checked public gate shims,
per 0050. Also run the intake's fail-closed native-tls absence proof: successful
inverse-tree output refuses; only a nonzero result specifically saying the
package did not match is absence, and any other error refuses. On the chief
commit run `bash bundles/self/scripts/design-check.sh check .`; do not run
that HEAD-discovery check against a later unrelated implementation commit
and treat it as design evidence. CI/operator obligations remain as assigned
in the spec, without invented local event baselines or external reads.

## Risks

- **Copy drift:** independent exact assertions cover every owned surface;
  equality to the same source alone cannot prove the operator's wording.
- **Placement false positives:** isolate the H1 lead and actual hero element;
  demonstrate wrong-location refusals. Preserve supporting copy separately.
- **Whitespace/parser mismatch:** normalize only format-permitted prose
  whitespace and decode YAML; keep raw paragraph and boldness checks exact.
- **Historical damage:** classify search results and restrict edits to current
  product claims; leave specs and accepted records intact.
- **Metadata proof overreach:** keep historical parsing and scratch fixtures
  generic, apply new positioning only to the current chart, and explicitly
  execute its Helm-independent mode locally and in normal chart verify.
- **Implied behavioral expansion:** preserve factual qualifications and limit
  source changes to strings. Full protocol regressions still run; no new
  concurrency seams or capacity promises are introduced.
- **Longer hero:** use existing responsive flow and inspect its markup at
  narrow-width rules; a minimal local style fix is allowed if needed. Do not
  claim a browser rendering check unless one actually runs locally.
- **False green delivery:** retain real exits/logs and exact coverage plus
  selftest; report missing tools or failures by name. No gate weakening,
  fresh dependency, network installation or external proof is a workaround.
