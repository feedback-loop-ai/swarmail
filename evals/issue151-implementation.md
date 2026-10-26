# Issue #151 continuation — implementation and proof receipt

Date: 2026-10-26. Run: epic-164-issue-151-part-1-contin-7f07ebde.
Phase verdict: **blocked** by the two out-of-scope documentation failures below.
The launching wrappers and offline functional proofs are committed and available
for review. There is no whole-feature green or exact-coverage verdict.

Implementation commits: 6c7d4b4 (launches and declared surfaces), 29b3162
(serializable input admission), 7e4e7f7 (report arithmetic golden preservation),
ec6214f (semantic private-record refusals). This receipt adds documentation only.

## Evidence and implementation

The issue151 packet passed SHA256SUMS from its own directory, including README
be74941231e61014f3f4caa0803713901a876d28a91f514b2e29febde76d5df9
and operator-rulings
fee1252df8b6a3bb8b3693155fc5f94d123f2e5488652b744112e3d8160718e1.
The versioned contracts distinguish help/config parsing, nearby-version live
measurements and inference. This delivery invoked no provider CLI or paid agent.
The [wrapper guide](wrappers.md) owns setup, schemas, recipes and limitations.

The implementation replaces both missing-contract refusal sites. Claude and
Codex receive unchanged prompt bytes on stdin; dsh receives one literal bounded
argument and a selected route/model overlay. Fixed builders configure Swarmail
HTTP MCP and the identity-bound fixture helper. Claude uses explicit measured
MCP tool names, guarded against real tools/list; Codex uses individual TOML
keys; dsh uses insert rows and the captured provider/compaction constants.

Private selection/credential reads use bounded no-follow descriptors and closed
typed schemas. Only the selected family credential is opened. Credential-bearing
files are exclusively created 0600 under private HOME, with 0700 directories.
Claude uses the reduced OAuth file; Codex replaces the genuine refresh token
with an inert placeholder; dsh exports only its selected route key. Source
records remain unchanged. CLI stdout/stderr are null from launch. The isolated
helper exec clears inherited credentials again; a protocol acknowledgement and
/proc observation prove the effective helper environment.

Canonical tool surfaces are required in every current row, bound through worker
contexts, refusal/unstarted constructors, baseline accounting, strict readers,
retention and reports. Reports disclose family asymmetry and retain distinct
surfaces. The historical arithmetic fixture is unchanged: new metadata is
asserted separately, then every original report field still compares exactly.
Catalogue/snapshot/result business versions and scoring remain unchanged.

Unix exec preserves the existing runner group and ownership. Real stand-ins
prove normal/nonzero exit, deadline, SIGTERM and SIGINT for all three families,
with acknowledged children/grandchildren resistant to TERM. Actual cessation,
direct-child reaping and private-state disposal precede completion. A temporary
cleanup bypass produced the expected test exit 101 at “descendants alive at
publication”; the original cleanup was restored before committing. No bypass
is shipped. Existing ownership, admission and wire proofs remain intact.

Baseline still validates fresh controls and retained bindings before paid
admission. It now checks the credential-free selection once after controls,
before admitting hundreds of slots; it opens no credential there. A reduced
local proof passes fresh real controls, launches all six selected stand-in
cells, validates retained accounting and produces their failure report. This
proof leaves the product's fixed full plan unchanged.

The carried queued-signal regression now acknowledges delivery through separate
test-only observers before releasing its hold. The production receivers remain
unconsumed, and the original zero-time/no-launch row assertions remain. It
passes in the final full suite, together with the existing lifecycle controls.

## Observed verification

All commands ran offline with CARGO_NET_OFFLINE=true and installed nothing.
The candidate-local engine digest is exactly
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.
Public shim digests passed; pinned proof owners were executed. The normative
inventory remains [house rules](../docs/house-rules.md#gates--part-of-every-change).
The table is this run's receipt, not a replacement inventory.

| Observed check | Exit / result |
|---|---|
| Root fmt | 0 |
| Root locked clippy, all targets | 0 |
| Root locked workspace tests | 101: wrapper proposal (now 0083) missing its index row |
| Pinned local brokkr compile, bundles/self | 0; 76 valuations, zero unruled |
| Exact coverage gate | 1: instrumented tests fail on the unindexed wrapper proposal (now 0083); no coverage verdict |
| Coverage selftest | 0 |
| Publish guard proof | 0 |
| Keyless seal proof | 0 |
| cargo deny check licenses | 0 |
| Fail-closed native-tls absence | 0 |
| Pinned design selftest / delivery tamper proof | 0 / 0 |
| Standalone eval fmt / locked all-target clippy | 0 / 0 |
| Final standalone locked full suite | 101: boundary authority link; 51 library, 19 binary and 3 baseline tests passed first |
| Wrappers target | 0; all 15 tests, including 15 cleanup scenarios |
| Baseline target | 0; three tests, repeated successfully in final full suite |
| Fixture / identity / real oracle-null proof targets | 0 / 0 / 0; 17 / 5 / 6 tests |
| Provenance / report / runner / scoring targets | 0 / 0 / 0 / 0; 27 / 18 / 37 / 18 tests |
| Actual baseline dry-run | 0; 16 tasks, 6 models, 32 controls, 480 paid slots, five repetitions, concurrency 1, 600000 ms |

The actual dry-run created no output directory, read no selection/credential,
and launched no trial. Its invocation used the documented private placeholder
and /tmp/issue151-dry-run-only. Detailed commands, stdout/stderr, exits and the
negative control are retained as run-local files under .forge/implementation151
(plain provenance text, not tracked Markdown links).

Root gate observations were made at implementation commit 6c7d4b4. Subsequent
commits change only evals; root production, gate owners, manifests, license
policy, bundle and routing inputs are identical. Final standalone full suite,
clippy and the 15-test wrapper target ran with ec6214f committed and clean.
Only this documentation receipt follows those observations.

Earlier diagnostic attempts remain in the run-local logs. The first full eval
attempt exceeded the tool window while repeating malformed selections through
the paid product; post-control selection validation now refuses once. An early
new cleanup test incorrectly assumed interrupted rows always retain state.json;
it now checks the existing typed present/absent-state contract. The report
golden mismatch was repaired without changing its arithmetic fixture. None of
those earlier failed/timed attempts is treated as a green receipt.

## Required owner repairs and remaining proof

1. Root tests/store_contract_docs.rs fails
   every_decision_file_has_exactly_one_resolving_index_row and
   the_inventory_refuses_missing_dangling_duplicate_and_malformed_entries:
   The wrapper proposal (now docs/decisions/0083-cli-wrapper-launch.md) had no index row in
   docs/decisions/README.md. The instrumented coverage run fails for the same
   reason. The existing [0083 delivery boundary](../docs/decisions/0083-cli-wrapper-launch.md#delivery-boundary)
   and [design plan](../specs/cli-wrapper-launch/plan.md) assign index integration
   outside this implementation scope. Its recorded status is **proposed**.
2. The full eval suite fails
   boundary::sequential_contracts_and_decision_links_are_honest at
   evals/tests/boundary.rs:85: specs/agent-eval-harness/spec.md links to absent
   .forge/epic192/issue199/operator-rulings.md. #243 owns the tracked authority
   and link repair, as recorded in the continuation intake and design. No packet
   was fabricated and the assertion was preserved.

The current task limits edits to evals and its documentation. These failures
are not waived or hidden, and no complete result is reported. After the owners
repair them, the required gates must be rerun before a green verdict.

The operator owns first authenticated smoke for each of the six approved
models, exact-version tool inventory/HTTP MCP behavior (especially the live dsh
HTTP row), the paid baseline, publication/reprioritization and sealing. Costs,
tokens and provider transcripts are unmeasured. CI owns its charter checks and
#201's macOS eval proof; #221 owns namespace isolation. Every row remains
unconfined. No model fallback, effort override, dependency, runtime guarantee,
recipe route, decision acceptance, push, merge, tag or publication was added.

## Dated integration ownership — 2026-10-26

This is the prior continuation's receipt; its blocked verdict and command exits
remain historical. The [integration task](../specs/wrapper-integration/spec.md)
now authorizes the local-main merge and the formerly excluded index repair.
Its checksum-verified operator ledger allocates this proposal to 0083, which
remains proposed. References above use that current identity; they do not turn
the earlier missing-index or absent-link failures into passes. Main's #243
repair supplies tracked authority without fabricating an ignored packet.
The combined committed candidate requires fresh local proof of every gate.
