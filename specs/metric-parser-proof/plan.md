# Plan: metric parser proof

Implements [spec.md](spec.md). `.specify/templates/` contains no plan template;
this plan supplies the required technical context, constitution check,
project structure, research, data model, contracts and quickstart inline.

## Technical context

Single Rust 2024 crate, Tokio and Axum 0.8. The existing
`src/api.rs::inbox_of_path` selects then strictly decodes once;
`count_in_flight` owns both guards across `next.run` and the cfg(test) hold.
`prom_label` escapes backslash, quote and LF. `/metrics` returns String;
its actual HTTP media type must be captured, not assumed from the renderer.
`src/api/refusal_tests.rs::get` currently discards headers, and the shared
reader in `tests/common/metrics.rs` is repository-authored. Neither proves
upstream format selection. No production edits are planned.

## Constitution check

Preserve accepted 0065 and 0043/0046/0047, 0001–0003 and 0005, 0020 test
placement, 0050 pinned proof and residual rules, 0056 unsigned commits and
0058's five distinct offices. No decision/index/status, reserved number,
dependency, license allowlist, feature flag, coverage exclusion, gate, tool
pin, provider or delivery-policy changes. A semantic conflict follows AC6;
this compatible proof design needs no numbered proposal.

Read-only preservation review covers the ordered storage plan/commit/apply
boundary and SMTP reply ordering; crash after durable commit but before reply
may still cause retries. Webhooks stay nonblocking and bounded best-effort,
not a durable outbox or a condition of SMTP success. Pruning stays oldest-only
inside insert; failed commits do not prune. DATA size/chaos refusal stays
before insertion, extraction at ingest and waits subscribe before scanning.
Existing TLS tests use generated fixtures, not fixed expiry dates. These
boundaries justify no adjacent storage/SMTP/webhook/TLS refactor here.

## Project structure and files touched

The chief commits exactly `specs/metric-parser-proof/{spec,plan,tasks}.md`.
Positions and result files stay ignored. Downstream scope is:

| Path | Responsibility |
| --- | --- |
| `src/api/refusal_tests.rs` | Extend existing cold-path harness with the sequential scalar matrix, raw HTTP capture, exact fixture attribution and lifecycle assertions. Reuse the current hold; no new runtime seam. |
| `specs/decoded-inbox-metrics/plan.md` | Single dated evidence receipt with raw-capture digests, parser invocation/provenance, outputs, negatives, candidate identity and verification limits; preserve earlier receipts. |
| `specs/metric-parser-proof/{plan,tasks}.md` | Resolve the tooling prerequisite explicitly and record task proof links/status truthfully. No duplicate complete receipt. |
| `.forge/epic192/issue202/metric-parser-proof/` | Ignored binary captures, headers, fixture manifest, upstream adapter if needed, parser outputs and logs. Keep reproducibility information in the committed owning receipt. |
| `.forge/epic192/issue202/operator-proposal.md` | Conditional unnumbered proposal only on concrete invalid/ambiguous behavior; stop branch. |
| `src/api.rs`, `src/model.rs`, `tests/api.rs`, `tests/common/metrics.rs` | Existing owners/proof consumers to inspect and run; no planned changes to runtime or local grammar. |
| Existing refusal/continuation specs, decisions, index, README, `tests/store_contract_docs.rs`, `tests/delivery_recipes.rs` | Preservation checks; historical repairs and review obligations remain. No unrelated repair. |

Prefer no new test file. Cold deadline/cancellation diagnostics remain in the
already declared cfg(test) harness under 0020. A minimal run-local adapter
may expose upstream parse results as hex; it must not implement grammar,
normalize inputs or silently skip failures. Do not add an application/parser
dependency or vendor a monitoring project. Committed evidence must retain the
minimal adapter source or exact inline invocation, its digest and commands,
so ignored artifacts are not the only description of the experiment.

## Research and prerequisite status

Both positions agree on preservation and independent parsing. The chief rules
for simplicity's existing-file sequential implementation and robustness's
complete matrix/transport/oracle proof. The cost rulings are in the spec;
there is no authorized alternate identity to implement.

Read current intake, stopped triage/result, both full historical panels,
all five newer results and their recovered summary/ruling. All ten receipt
hashes matched in this seat; no direct journal authentication is claimed.
Inspected decoder/renderer, middleware hold, capture helper and actual model
admission (`char::is_control`). Bidi format characters are not all control
characters under that predicate. The original await refusal must remain;
additional held `/count` fixtures can exercise admitted and refused labels
without new waiter registrations.

**Open prerequisite R1: available upstream parser and exact command.**
PATH checks found no `promtool`, `prometheus` or `go`; Python lacks
`prometheus_client`. A bounded `.forge/bin`/`target` filename search found no
matching parser. The chief did not retrieve the supplied standards URL,
install a tool or run an upstream scrape experiment. No honest exact parser
invocation can be recorded yet. No guessed command stands in for it.

Before implementing the compatibility proof, identify a suitable already
available offline artifact in the execution environment and record its path,
source/version/digest, exact argv, complete-body input mechanism, observed
Content-Type selection/fallback and byte-safe output schema here and in the
owning receipt. Validate that artifact with positive and negative controls.
If none is available, stop the dependent proof and leave AC3/T1 open; report
missing tooling without calling the accepted policy invalid. Hermetic verify
installs nothing. Resolving this prerequisite is evidence work, not permission
to change pins, download dependencies or bypass the box. This drafted plan is
structurally reviewable but not an executed compatibility proof or a claim
that R1 is solved.

## Data model and contracts

No persistent or public schema changes. Proof-only records are:

- Fixture: stable exclusive ID, raw request path, decoded code points and
  UTF-8 hex, expected response status/body bytes and request sample value.
- Capture: case ID and phase (before/held/released), raw HTTP artifact digest,
  status/header bytes including media type, framing description, complete
  de-framed body path/digest/byte length, candidate HEAD/tree.
- Parser result: provenance/invocation/configuration, input digest, exit code,
  complete parsed samples with family, label names and hex values, value,
  assertion outcomes and negative-control receipts.

Store raw bytes in files, never shell variables or newline-normalizing reads.
Review views show escaped/code-point/hex text, preserving a separate untouched
capture. No console appearance is used to compare bidi labels. The observable
contract remains family plus decoded identity, not the encoded URI spelling.

## Implementation and proof sequence

1. Resolve R1 or retain an explicit blocked proof. Bind source candidate and
   all historical receipt identities before experiments. Inspect the supplied
   standards reference only through available authorized evidence; do not
   confuse a citation with observed parser behavior.
2. Extend the current HTTP helper to return raw response metadata/body bytes.
   Assert full framing: honor Content-Length or chunked transfer and reject
   incomplete/unsupported framing explicitly. For the currently expected
   length-framed String response, asserting that observed format is enough;
   do not build a generic HTTP stack without need. Preserve binary bodies.
3. Keep the original held CR/NUL await refusal. Add the specified finite
   matrix sequentially in the same test owner. Use fresh Store, port 0,
   exclusive names, reached/release signals and bounded client/server joins.
   Holding a real `/count` response suffices for the other cases. Admission
   determines actual status/body; no new rejection policy. Before/during/
   after observations and exact response assertions retain AC4.
4. Audit all fixture names in the lib executable. The existing global
   CR/NUL filter can conflict with future cases: narrow it to the owner's
   fixtures only if necessary, retaining exact names/values and checks for
   unexpected fixture-bearing rows across families. Do not reset registries
   or assert global totals. Additional `/count` cases need no cumulative
   await names; retain a documented bound comfortably below 4096. Use
   process isolation only if concrete interference defeats this approach.
5. Capture each complete held scrape and feed its identical body to the
   pinned upstream implementation, with actual media type/configuration.
   Require full parse and byte-exact identity/sample assertions over the
   parsed collection. Preserve the separate two-await six-row proof; request
   and await families do not become one atomic snapshot.
6. Execute upstream malformed-syntax negatives and comparator negatives
   (truncated/changed expected label, extra fixture row); record controlled
   assertion-red then restored-green. A wrapper returning unconditional
   success must fail controls. Do not mutate conformant production merely
   to manufacture a pre-fix defect. Retain historical old-helper proof.
7. On concrete rejection/ambiguity/identity failure, write AC6's precise
   unnumbered proposal and stop before policy-changing code. A transport
   media-type failure is not repaired silently in `/metrics`. If all cases
   conform, keep runtime unchanged and retain the smallest test/proof delta.
8. Append one dated owning receipt, then freeze the candidate, run the full
   required verification and obtain fresh five-role protected review. Every
   tracked evidence update changes the candidate: final gate/review receipts
   bind to the resulting HEAD/tree, not an earlier similar revision. Keep
   runtime and review checkboxes open until their paired proofs exist.

## Quickstart and validation

The exact upstream command is deliberately unavailable until R1 is resolved;
no command below is a substitute for AC3. Targeted existing suites are:

```bash
cargo test --locked --lib api::refusal_tests
cargo test --locked --lib api
cargo test --locked --test api
cargo test --locked --test store_contract_docs
cargo test --locked --test delivery_recipes
```

Run every command in the authoritative
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change).
Match `.forge/bin/brokkr` to `bundles/self/tooling.json`; check public shims
against `bundles/self/scripts/shims.sha256` and execute pinned proof copies.
Coverage must report missed==0 for lines, branches and functions and its
selftest must pass. A successful nonempty `cargo tree --workspace --locked
--prefix none` without native-tls plus the specific package-not-found inverse
query proves absence; network/resolution errors do not. Verify installs
nothing. CI-owned chart/history/selftest, Docker, RustSec, tag-rate and
port-80 proofs remain CI-owned; no local completion is inferred.

For the chief's three-file design, check relative links and `git diff --check`,
commit only those artifacts unsigned, and run
`bash bundles/self/scripts/design-check.sh check .` against HEAD. The engine's
pinned final check owns its verdict; the chief reports only `drafted`. Leave
that design commit last. Gate execution details are recorded below; a
structural check does not resolve R1 or the security residual.

## Risks

| Risk | Mitigation and cost |
| --- | --- |
| No upstream parser offline | R1 blocks successful proof; record absence, install nothing, preserve true/low. Cost: delivery remains pending. |
| Actual media type selects no parser | Capture headers; prove explicit supported fallback if applicable. Disclose limits; unresolved rejection takes AC6, never an unrecorded header change. |
| Raw bytes are changed by capture/reporting | Preserve files/digests and lengths; remove HTTP framing only; use hex comparison. Cost: bounded capture plumbing. |
| Local oracle repeats renderer assumptions | Upstream implementation plus malformed syntax and identity/count negatives. No second handwritten grammar. |
| One combined fixture hides a rejected scalar | Individual CR/NUL/C1/bidi cases plus combined and escape controls. Cost: finite larger matrix. |
| Bidi values look equal or control refusal is invented | Byte/code-point comparison; assert existing admission and actual response bytes. Keep readable escaped review views separate. |
| Held requests leak or race shared fixtures | Existing request-owned handshake, bounded ownership, exact attribution/headroom audit and conditional isolation. No sleeps/resets/global totals. |
| Parser accepts but truncates or adds rows | Complete parse, exact label bytes and complete fixture sample set, negative comparator controls. |
| A conformant test is mistaken for clearance | Preserve every inherited fact and require five fresh 0058 results; 0050 hard stop remains. |
| Unbounded compatibility claim | Name version, transport, media type and fallback; no all-scraper or OpenMetrics claim. |
| Sanitization silently changes accepted identity | Zero runtime change; AC6 proposal and operator decision before contradictory work. |
| Old receipts or edited task boxes certify a new tree | Append dates, retain originals, bind final commands/results to HEAD/tree; pending proofs stay unchecked. |
| Coverage or proof environment refuses | Record failing gate honestly, retain exact floor/pins and CI distinction; no historical-pass substitution. |


## Chief validation receipt — 2026-10-24

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
