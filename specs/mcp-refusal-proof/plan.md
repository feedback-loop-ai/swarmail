# Implementation plan — MCP refusal proof

Implements [spec.md](spec.md) without changing runtime semantics.

## Technical context

Single Rust crate using the existing Tokio/Axum/serde_json stack and real SMTP,
HTTP, SQLite and stdio test infrastructure. No dependencies or new production
module. `.specify/templates/plan-template.md` is absent; this plan supplies the
required context, constitution check, structure, research, data model, contracts,
quickstart and risks directly. Chief commits only spec.md, plan.md and tasks.md.

## Constitution check

Preserve accepted 0001–0005, 0020 placement, 0043 guards, 0045 proposal/gate
ownership, 0048 webhook/REST independence and 0057/0058 protected routing.
Lazy report analysis never recomputes stored extraction or delays SMTP storage.
No changes to store transactions, watcher ordering, pruning, best-effort webhook
budgets, generated TLS validity fixtures, chaos admission or commit ownership.
Keep inline branch-stable tests measured; cold timeout/panic diagnostics remain
in the declared cfg(test) owner harness. Public API tests stay under tests/.
Private access alone is not grounds for moving an inline test out of coverage.
No tests run attacks against review, widen access or execute standalone evals.

## Research and chosen approach

The existing success formatter owns one captured Value. Error formatting already
omits structuredContent, and report dispatch maps AnalysisError through it.
There is no demonstrated runtime defect to repair. Existing Held observes real
worker entries and releases on Drop; its ten-second diagnostic is a failure
bound, never synchronization. Use that seam without modifying service limits.

The modern historical case list lacks explicit null report IDs and malformed
containers. Add independent table cases beside Wire, not to the independently
captured historical inputs or outputs. Wire already starts the real Binary
against a loopback server, so no new transport abstraction is warranted.

Unavailable HTTP currently checks selected fields; bridge equality alone could
match two malformed results. Replace that proof with a literal complete expected
envelope on each adapter. Invalid timeout assertions need the same strengthening.
Notification tests must account for steady-state concurrency: ping is not a
completion fence. Binary::finish closes input and drains output; use that to
prove no trailing notification replies, then inspect the server's stored count.
If observing effects before EOF, use bounded observed state, not sleep-as-sync.

The existing router constructs its own Arc<ToolRate> inside /mcp. Direct context
exhaustion does not access that instance. The integration Binary harness cannot
access the cfg(test) Held seam. Record these boundaries as missing wire-rate and
binary-unavailable proof. Do not infer closure from production inspection,
substitute a test router, burst against a one-second clock, guess an executable
path or introduce runtime test controls. Any later genuine runtime defect or
need for a new seam is documented and reframed before broadening this design.

## Implementation sequence

1. Verify evidence/tooling manifests and freeze protected historical path hashes.
   Record starting HEAD/tree and clean status. Map every T8 row to the spec's
   exact evidence classes. Capture focused baseline test exits.
2. Add `modern_report_argument_refusals` in structured.rs using Wire::start.
   Enumerate omission, {}, null/numeric ID, null/array/scalar container and
   unknown key with independent expected literal envelopes and unique request
   IDs. Include explicit-null JSON-RPC ID as a request control. Finish every
   spawned child and stop each isolated server. Do not derive expected messages
   from production helper functions or mutate historical fixtures.
3. Strengthen held-worker unavailable equality and existing wait-timeout equality.
   Preserve real worker acknowledgments, SMTP/queryability and post-hold mutations.
   Keep the existing unavailable test and add a separate held-server notification
   case if needed to avoid extending its watchdog across unnecessary work.
4. Add `modern_notification_refusal_contract` in structured.rs for both modern
   revisions and both HTTP/binary adapters: admitted seed, malformed-report
   notification, correlated ping and exact final transcript/effect checks. Use
   fresh inbox/server state; HTTP checks actual empty body rather than a parser's
   Null fallback. Binary EOF drains outstanding work before exact effect counts.
5. Add `report_unavailable_notifications_keep_bridge_silent` in the existing
   deliverability owner harness using Held and actual bridge-function/HTTP. Prove
   empty 202, no notification lines, ping correlation and EOF completion. Keep
   all workers held until refusals finish, then release/join them and stop the
   server, including panic-safe cleanup. No spawned-binary claim follows.
6. Rerun focused and retained T8 witnesses. Record final status per row in this
   supplement, including explicit rate/process gaps. Keep earlier specs and their
   unchecked T8 untouched. Document any test failure precisely, never mark a row
   green merely because another row uses the same helper.
7. Commit tracked implementation/evidence unsigned, then execute every house gate
   plus framed supplemental proofs on that clean HEAD. Run the inventory even
   after a failure; retain exits/logs. If fixing in-scope failures changes HEAD,
   bind new receipts accordingly. Engine verify/review own subsequent verdicts.

## Project structure and files touched

| Path | Planned role |
| --- | --- |
| specs/mcp-refusal-proof/spec.md, plan.md, tasks.md | New design; later current supplement status, without rewriting previous specs |
| specs/mcp-refusal-proof/implementation.md | Later implementation-only witness/receipt index and explicit remaining gaps; absent from chief commit |
| tests/mcp_revisions/structured.rs | Modern argument/notification matrices and complete timeout envelopes, existing Wire/Binary usage |
| tests/mcp_revisions.rs | Existing Binary helper only if a minimal test-only drain operation is required; no transport framework |
| src/deliverability/concurrency_tests.rs | Complete unavailable envelopes and separate held-worker notification witness |

Read-only proof owners include src/mcp.rs, src/api.rs, src/stdio.rs,
src/store.rs, tests/stdio.rs, tests/deliverability.rs and other invariant suites.
No production edit, Cargo/deny change, bundle/gate modification or ADR/index edit
is planned. Historical specs/mcp-report-output, mcp-structured-output and
mcp-version-negotiation; all decisions; tests/fixtures/mcp-structured-output and
supplied run evidence remain byte-identical. Run-local logs stay ignored.

## Data model and contracts

No stored or public data model changes. The test ledger records case, revision,
adapter class, request identity, literal expectation, effect/silence expectation,
witness name, command/exit and candidate HEAD/tree. This is evidence metadata,
not a runtime registry or second formatter. Shared helpers may transport data,
but expected results are independently specified.

Argument failures keep the correlated JSON-RPC error arm (-32602, no result).
Report not-found/failed/unavailable, storage, invalid wait timeout and shutdown
keep the text-only tool result with isError:true. Successful timeout remains
successful. Notifications omit request ID and produce no reply; id:null is a
request. Modern success text and structuredContent remain from one captured Value.
No full-conformance, availability, exactly-once or retry guarantee is introduced.

## Quickstart and validation

Use workspace hands, installed tools and offline Cargo. Before running brokkr,
compare `.forge/bin/brokkr` to `brokkr.binary_sha256` in
bundles/self/tooling.json (currently
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`).
Check public shims with the pinned shims.sha256. Never execute a mismatched
binary or replace missing tools/download evidence during verification.

```bash
export CARGO_NET_OFFLINE=true
cargo test --locked report_workers_do_not_hold_store_guards
cargo test --locked --test mcp_revisions
cargo test --locked --test stdio
cargo test --locked --test deliverability
```

The single authoritative full inventory is
[house rules](../../docs/house-rules.md#gates--part-of-every-change).
Execute format, locked workspace clippy with warnings denied, locked workspace
tests, digest-matched `.forge/bin/brokkr compile --bundle bundles/self`, exact
coverage and its selftest, publish guard proof, keyless seal proof and licenses.
Use the pinned bundle script owners under 0050; the later verifier uses its own
pinned closure offline. Require literal zero missed lines/branches/functions.
Also execute `bash bundles/self/scripts/design-check.sh selftest` and
`bash bundles/self/scripts/delivery-proof-test.sh`, and verify shims with
`sha256sum --check --strict bundles/self/scripts/shims.sha256`. Prove native-tls
absence via `cargo tree --locked -i native-tls`: only its specific no-package
match diagnostic establishes absence, not an arbitrary nonzero exit.

Retain exact command/status, commit/tree, dirty-state check and log hashes.
Chief runs the committed-artifact check with
`bash bundles/self/scripts/design-check.sh check .`; the sequence's final pinned
check still owns advancement. A local gate receipt is not that engine verdict.
CI retains the inventory's platform, network, chart, Docker and real port-80
proofs; tag-only rate/publication proof is separate. No new ownership claims.

## Risks

- **False all-T8 completion:** keep explicit unproved binary-unavailable and
  shared-router-rate rows; preserve historical unchecked T8 and member findings.
- **Two equal malformed envelopes:** independently pin complete literal responses,
  including jsonrpc, ID, single text block, isError and absent structuredContent.
- **Concurrent notification races:** EOF draining/observed effects, exact transcript
  cardinality and per-ID correlation; ping alone is not a mutation barrier.
- **Held-worker watchdog on slow CI:** use fresh focused cases, avoid redundant
  exchanges, keep Drop release and worker joins. Do not raise production limits
  or add global mutexes/sleep synchronization to hide scheduling problems.
- **Wrong fixture provenance:** freeze independent captures and distinguish
  directly inserted malformed rows from SMTP delivery, typed reserved forms,
  real bridge function and spawned executable evidence.
- **Coverage evasion:** existing cold-path harness placement stays justified;
  production and inline tests remain fully measured, no new exclusions.
- **Inherited security and integration debt:** retain HIGH and indeterminate
  findings, require actual parallel OPTIONS review and combined-candidate proofs,
  preserve #151/#201/client/CI/seal prerequisites and protected verdict ownership.
- **Scope creep from a failing test:** record the exact defect; only compatible
  scoped work proceeds. A changed public/ruling contract requires a separately
  proposed decision and any required operator acceptance, not a silent test fix.
