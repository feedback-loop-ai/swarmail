# Implementation plan — HTML review repair

[Specification](spec.md), [proposed 0107](../../docs/decisions/0107-html-review-repair.md).

## Technical context

One Rust 2024 crate; existing serde/mail-parser/Tokio, finite HTML/CSS scanner
and Store-owned lazy report service. No new dependency/workspace member.
The plan template is absent from `.specify/templates/`; this plan supplies its
technical context, constitution check, structure, research, data model,
contracts, quickstart and risks directly.

## Constitution check

0001 storage before 250, 0002 subscribe-before-scan, 0003 oldest-only pruning,
0004 isolated inboxes and 0005 ingest extraction remain constraints. No SMTP,
Store mutation, persistence or fan-out production repair is planned. The #251
direction/0045 §8 allow compatible work; 0085/0106/0107 remain proposed. Accepted
records and historical specs stay unchanged. Exact coverage/0020 apply to work
instrumentation too.

The prerequisite index-only commit repairs 0106 table membership and reserves
0107. The last chief commit contains exactly this plan, spec.md, tasks.md and
0107, touching one spec directory. Run-local receipts/results stay ignored.
Implementation adds dated proposed pointers to 0085/0106 without accepting them.

## Research and resolved choices

Both positions, verified packet, tracked intake, decisions and actual collector/
decoder/composition ground the design. Scan::css currently pushes declaration
state for every non-media brace, irrespective of value ownership. Work adds
cursor distance, hiding helper revisits. collect_with parses zero bytes, which
fails although SMTP accepts them. collect already runs demo, HTML and test
fixture contributions before finish: retain that order. RefScanStats observes
reference windows, not total work; extend accounting without changing its meaning.

Numeric coefficients are an implementation deliverable: derive them from the
instrumented operation census before measuring fixtures. A guessed design
constant is not proof. The acceptance boundary is fixed; coefficients cannot
be fitted or enlarged merely to admit a rescanning regression. No network
research or new library is needed.

## Project structure and files

| Path | Work |
|---|---|
| `src/deliverability/html.rs` | Bounded CSS contexts, literal URLs, exact-empty branch, operation accounting and inline regressions. |
| `src/extract.rs` | Narrow crate-private counted decoder access/field as needed; preserve legacy exact statistics and semantics, no duplicate decoder. |
| `src/deliverability.rs` | Version, composition/limits/cache expectations/tests; retain collect/finish/service ownership. |
| `src/deliverability/html-rules.json` | Aggregate ID only; existing rule data/provenance remains. |
| `tests/fixtures/html-compatibility.json` | Full v2 expectations and paired cases; retain seven-rule/MIME/boundary recipes. |
| `tests/deliverability.rs`, `tests/stdio.rs` | Real empty SMTP, CSS/URL, report/UI/predicate parity, distinct failures and version expectations. |
| Existing deliverability owner/concurrency tests | Preserve retry/identity/config/permits/cancellation/wait/deadline/snapshot proofs using existing barriers. |
| `README.md`, `src/static/llms.txt`, `src/static/openapi.json` | Current version, exact-empty exception, finite lexical behavior and honest proof limits. |
| `docs/decisions/README.md` | Prerequisite table/index repair, then verify both rows and proposed statuses. |
| `docs/decisions/0085-deliverability-report.md`, `docs/decisions/0106-html-client-compatibility.md` | Dated implementation pointers to proposed 0107 explaining v2; preserve historical text/status. |
| `specs/html-review-repair/implementation.md` (later phase) | Operation census/C/K derivation, complete finding-to-test ledger, proof identities and outstanding CI/operator obligations. |
| Run-local evidence | Defect red/green logs, temporary mutation diff/hash, restored-source hash, gate commands/exits/hashes and HEAD/tree/engine binding; never fabricate journal rows. |

No production module is planned. A declared owner-local *_tests.rs is allowed
only for a concrete cold diagnostic under 0020: explain it before moving any
test. Branch-stable assertions stay inline. Wire tests use real servers and
per-test AUTH inboxes, no private seams or mocks.

## Data model and implementation approach

1. Use small bounded frames for rule lists, declarations and component values
   with matching delimiters. Preserve property/value state across nesting. Keep
   opaque presence or original spans so removal cannot fabricate literals and
   property-only selectors see nonempty complex values. One CSS depth budget
   covers blocks/braces/parentheses/brackets/URLs; preserve media and suppression.
2. Detect literal URL tokens before ordinary comment/component handling; follow
   the spec's quoted/unquoted, whitespace, escape and invalid-tail grammar.
   Consume the correct close and restore the owner. Match boundaries, not names
   embedded in another identifier. No URL fetch/normalization is needed.
3. Add exact-empty success inside collect_with after catalogue/raw validation,
   before MIME parse. Leave demo, other/fixture collectors and finish intact.
   Existing Inputs fixture contributions prove retained composition and overflow.
4. Thread local work through real helpers and counted decoder access. Charge
   reads/comparisons at operations. Document conservative library charges with
   bounded inspected prefixes/lengths, including failures, copies/normalization,
   attribute hashing and repeated matching. State excluded allocator/hash
   collision/MIME internals. Retain progress fields separately. Justify integer
   range under input caps; overflow must not produce a false passing count.
5. Before work-fixture execution, record operation/charge/max repetitions and
   contribution to numeric C/K. Compose source bytes, decoder lookahead/window/
   table, 6/5 expansion, catalogue, depth and part caps; state per-part/message
   bounds. Predetermined multiple sizes cover successes/failures. Choose control
   sizes analytically so repeated suffixes exceed the fixed bound.
6. Run temporary real counted rescanning through an actual helper. Preserve the
   same-test bound failure with progress passing, restore exact source and rerun
   green. Ship no fault mode or second scanner. Also show behavioral CSS/URL/
   empty reds on original semantics, separate from version-expectation failures.
7. Change v2 identity, reports and living docs together. Preserve validation and
   data-change outcome tests, seven-rule pairs, MIME paths/budgets. Add dated
   proposed pointers; never rewrite old specs or decision acceptance.
8. Run focused suites then all gates on committed implementation. Bind logs to
   HEAD/tree and digest-checked candidate-local engine. Failures remain open;
   later passing commands cannot erase them. Fresh independent review follows.

## Contracts and proof map

Spec AC1/4 own lexical behavior, AC2 work/falsification, AC3 empty transport and
composition, AC5 complete disposition, AC6 version/data/docs, AC7 ownership/bounds
and AC8 fresh proof. No public endpoint/type/config knob is added. REST/MCP
text-block/stdio/UI consume one typed report. Exact asserts and minimum-one
waits retain the common qualifier/wait owner. Reuse complete JSON expectations
and server helpers; never synthesize headers onto empty mail.

## Quickstart and validation

After repair, send HTML containing
`<p style="--unused:{position:absolute};background:url(https://x.test/*/a.png)">Hello</p>`
through SMTP, see 250, select its ID, then GET
`/api/v1/messages/{id}/deliverability`: exactly v2/no findings. Add
`;position:absolute` within style: existing warning at root [], same MCP/UI
finding and exclusion at warning threshold. In a separate inbox send immediate
DATA termination: read zero /raw bytes and v2 empty report. Main integration must
also run its bulk/authentication collectors honestly.

Run offline, installing nothing. Focused commands include locked lib
deliverability/extract, integration deliverability/stdio, the existing
smtp_edges empty-DATA regression and burst/p2/rate. Run commands separately and
retain every exit. The authoritative inventory is
[house rules](../../docs/house-rules.md#gates--part-of-every-change).
Verify `.forge/bin/brokkr` against tooling.json (digest
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f) and all
bundles/self/scripts/shims.sha256 entries. Compile bundles/self with that local
binary. Run fmt, locked workspace clippy/tests, canonical bundle coverage and
selftest, publish-guard-test, seal-branch-test and cargo deny licenses. Verify
uses pinned canonical scripts; candidate shims are integrity-checked only.
Also require design-check/selftest, delivery-proof-test, package listing with
catalogue/fixture inclusion and fail-closed native-tls absence: only the exact
missing-package outcome proves absence; other Cargo errors fail. Executable
commands are in the linked intake. Inspect exact counts, not rounded percentages.

CI/operator proof owners remain as AC8 states. Keep main/later eval/PR integration
outside this candidate; after this run the combined candidate requires fresh
gates and independent review, specifically bulk/unknown-auth empty-mail outcomes.
No push, merge, publication or acceptance is authorized.

## Risks

| Risk | Mitigation and cost |
|---|---|
| False-warning repair hides real declarations | Explicit ownership, full paired reports, complex outer values, following declarations and media/selector controls; pay delimiter coverage. |
| URL handling alters surrounding parsing | Pin finite grammar/invalid tails and ordinary comments; strict malformed outcomes stay visible. |
| Work counter renames progress | Helper/decoder charges, census audit and real counted rescanning control; claim only declared model. |
| Conservative charges become quadratic or fitted | Prefix-aware successful searches; derive coefficients before measurements, choose control sizes analytically. |
| Counting affects ingest | Narrow shared-decoder access, local integers, unchanged semantics/exact stats, extraction and burst/rate proofs; do not promise optimization removes overhead. |
| Empty success bypasses composition | HTML-only return, fixture contribution/overflow proof, later bulk/auth integration tests. |
| Hostile MIME allocates before lexical proof | Raw preflight plus structural/decoded limits; MIME internals/RSS/latency remain unproved, never truncate. |
| Cancellation/restart/pruning returns stale/partial data | Preserve worker-held permits, Weak keys, snapshots, uncached failures and deterministic barriers; restart recomputes. |
| Contention, notification loss or retry after commit | Preserve bounded existing executor and post-commit best-effort webhook path. Four workers promise no fairness; crash after durable commit before reply can yield sender retry. No broader lifecycle repair. |
| Exact coverage encourages hidden branches | 0020 placement, exercised test-only seams, full gate/selftest, no denominator shrink or permanent mutation mode. |
| Design mistaken for repaired acceptance | Open tasks/ledger; structural design check is not behavioral proof. Prior green gates cannot close missing ACs. |
