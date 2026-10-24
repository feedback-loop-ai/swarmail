# Plan: metric review continuation

Implements [spec.md](spec.md). No plan template exists under `.specify/templates/`;
the required technical context, constitution, structure, research, data model,
contracts and quickstart are supplied here.

## Technical context

Single Rust 2024 crate with Tokio, Axum 0.8 and existing percent-encoding.
`src/api.rs::inbox_of_path` already selects before strict decoding;
`count_in_flight` owns both guards across `next.run` and the cfg(test) hold.
`prom_label` leaves CR/NUL literal under accepted escaping. Current tests are
`src/api/refusal_tests.rs`, `tests/api.rs` and `tests/common/metrics.rs`.
The implementation delta is documentation/evidence plus the reproduced stale
routing-test expectation described in AC8; no production change is planned.

## Constitution check

Preserve accepted 0065 and 0043/0046/0047; frozen 0001–0003 and extraction
0005; 0020 placement; 0050 proof ownership/residual rules; 0056 unsigned
commits; accepted 0058 reviewer identities. No semantic amendment is proposed.
No change to decisions/index or reservations #180=0062, #181=0063, #174=0064,
#202=0065. No dependency/allowlist, coverage exclusion, tooling pin, gate,
provider, policy or recipe change. Existing whole-protocol suites remain the
proof of preservation; no new production behavior is justified by the notes.

## Project structure and files touched

The chief commits exactly `specs/metric-review-continuation/{spec,plan,tasks}.md`.
Run-local positions, recovery evidence, result files and logs stay ignored.
Downstream work is bounded as follows:

| Path | Action |
| --- | --- |
| `specs/decoded-inbox-metrics/plan.md` | Single owner for dated residual mapping, commands/outcomes and fresh candidate evidence; retain prior receipts verbatim. |
| `specs/metric-refusal-proof/spec.md` | Reconcile current AC7 with 0058 and link owning evidence. |
| `specs/metric-refusal-proof/tasks.md` | Correct current T9; date clarification of historical four-lens receipt; keep review unchecked until full proof. |
| `specs/metric-refusal-proof/plan.md` | Dated current handoff and actual-file clarification; preserve historical plan/validation receipt. |
| `specs/decoded-inbox-metrics/{spec,tasks}.md` | Only if current handoff conflicts remain; preserve completed historical implementation facts. |
| `src/api.rs`, `src/api/refusal_tests.rs`, `tests/api.rs`, `tests/common/{metrics,mod}.rs` | Inspect and execute existing proofs. Edit only for a specifically reproduced compatible defect; pair any change with a meaningful regression and reverify. No speculative rename/refactor. |
| `tests/delivery_recipes.rs` | Reconcile only the reproduced selector test's Sol/high expectation to the already configured and required Luna/xhigh; preserve exact strategy and reviewer assertions (AC8). |
| `tests/store_contract_docs.rs`, `README.md`, historical recovery/recording specs | Read/check existing inventory, selector guidance and completed repairs; no planned edits. |

Do not duplicate complete receipts in this package. Its tasks link the owning
runtime plan. Any later evidence update changes candidate identity and must
be included before final candidate-bound verification/review.

## Research and resolved choices

Read both positions, both intake records, both recovered events/eight notes,
five hash-matched originals, accepted authorities and current implementation.
The missing-evidence triage escalation is superseded by supplied records;
it does not clear security. The old repair result records `62e6496` and the
later panel `cca967b/981ccc32`; neither is current HEAD. No new runtime failure
has been reproduced by synthesis. Keep the existing post-response seam and
independent grammar parser. No new parser package or generic amendment auditor.
Review the 0065-to-0043 semantic link directly because the finite amendment
list only partially covers it. Preserve stated limits and historical wording.

## Data model and contracts

No persistence, API schema, metric family or identity model changes.
`Option<String>` is selected once; guards own immutable keys until Drop.
A decoded request row reports activity before admission, not stored mail.
The process-local registries are not durable receipts. Cumulative awaits and
live request attribution have different bounds; family plus label is the key.
Two parked awaits make the six-row assertion valid without promising global
atomic snapshots. A one-sample CR/NUL parse establishes the tested grammar,
not compatibility with every external scraper.

Preservation review includes: synchronous storage commit before final SMTP
250; a crash after commit/before reply can still lead to client retry; memory
stores promise no restart persistence. Webhook fan-out stays nonblocking,
bounded and best-effort, with possible crash/exhaustion loss and no outbox.
Pruning remains within the ordered insert boundary and oldest-only. DATA
limits/chaos refusal precede insertion, extraction stays at ingest, and waits
subscribe before scanning. TLS failures remain bounded failures with verified
certificates and per-run fixtures. None warrants an adjacent implementation.

## Implementation and proof sequence

1. Recheck recovery hashes and map all notes to current artifacts, preserving
   exact historical inputs and distinguishing the current residual assessment.
2. Repair only stale current review prose; append a dated 0058 clarification
   beside historical four-member receipts. Link the single owning receipt.
3. Run existing decoder, real-wire lifecycle, held-refusal, parser and document
   suites. Audit fixture identity/cap headroom and unchanged semantic pointer.
   Do not reimplement a passing proof. If a defect appears, first capture its
   failing assertion and identify a compatible minimal fix; otherwise keep
   runtime/tests unchanged. Contract conflicts require an operator ruling.
4. Record fresh outcomes, failures and limitations in the owning plan; check
   tasks only after their entire paired proof. Keep review/security pending.
   Reconcile the newly reproduced AC8 test expectation with the existing
   Luna/xhigh contract, retaining exact assertions; run the delivery-recipes
   suite before the final full gates. This is a test correction, not a route
   or provider change.
5. Freeze the final candidate, run complete pinned verification and obtain the
   engine's five separate protected results. Any fix invalidates the prior
   candidate binding and requires the appropriate fresh verification/review.

## Quickstart and validation

From repository root, offline without installing dependencies:

```bash
cargo test --locked --test api
cargo test --locked --lib api
cargo test --locked --lib api::refusal_tests
cargo test --locked --test store_contract_docs
cargo test --locked --test delivery_recipes
```

Then execute the complete [house-rules gate inventory](../../docs/house-rules.md#gates--part-of-every-change).
The inventory is authoritative, not a copied reduced subset here. Match
`.forge/bin/brokkr` SHA-256 to `bundles/self/tooling.json` before compilation;
use pinned proof scripts and check all public shims. Prove native-tls absent
from a successfully resolved nonempty locked dependency tree, not a failed
query interpreted as absence. Keep full logs ignored with exact exit statuses.
CI owns its chart, Docker, RustSec, release-rate and real port-80 proofs.

For this design, validate links/whitespace, run required local gates and
record their actual outcomes below. Commit only the three design artifacts
unsigned, then run `bash bundles/self/scripts/design-check.sh check .`.
The engine owns the final check result; the chief writes only `drafted` to its
assigned result file. Leave the design commit at HEAD. Design checks and
baseline tests are not final implementation verification or security clearance.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Literal CR/NUL disrupts a strict external scraper | Preserve true/low, existing held proof and protected security disposition. No universal compatibility claim; an unresolved contract conflict stops delivery. |
| Clean security prose erases aggregate facts | Retain both events, member notes and typed aggregate inputs; demand concrete counterevidence before correcting a recorded input. |
| Historical receipts certify a different candidate | Record full current head/tree for fresh runs and require final engine binding. No inferred identity from similar code or commit subjects. |
| Four-role wording bypasses independent Fable | Reconcile current AC7/T9/handoff under 0058; inspect five distinct resolved identities/results, Flash official only, no fallback. |
| Speculative hardening changes accepted behavior | Default to zero runtime edits; require reproduced compatible defect and regression. Semantic changes need a separate proposed amendment/operator ruling. |
| Parallel fixtures false-fail or exhaust cumulative cap | Audit executable-wide distinct identities; retain injection assertions; conditional fixture attribution or process isolation, never resets/serialization. |
| A test observes before refusal or polls until green | Keep post-response reached/release handshake and exact single-scrape assertions; real deadlines bound failures only. |
| Cancellation proof is overstated | Separate hold release/abort proof from actual guard lifecycle tests and cleanup observations. |
| Finite amendment audit is mistaken for semantic proof | Retain explicit limitation plus direct review of accepted 0065 and 0043 marker; do not build a generic auditor. |
| Proof tools or environment are unavailable | Record actual failure/blocked gate, preserve pins, install nothing and never substitute a historical pass. |
| More evidence creates competing contracts | One owning runtime receipt, dated references elsewhere and preserved historical text. Accept documentation audit overhead. |

## Chief validation receipt — 2026-10-24

Executed offline against baseline `995661dc4bfaf7f4a7b96ae1385f67727138f730`,
tree `87f6f8bff7db29c754a3d5a5ed94b6bc1f61c2bb`, plus this uncommitted design.
The ignored directory `.brokkr/state/design/metric-review-continuation-checks/`
contains `identity.json` (including draft hashes), `tool-pin.json`,
`results.json` (commands/exits/durations) and individual logs. This receipt
and the resulting AC8 scope clarification were added after execution.

Passed: public-shim SHA-256 checks; formatting; locked workspace/all-target
Clippy with warnings denied; targeted API 66/66, library API 33/33,
held-refusal 2/2 and document 19/19 suites; pinned bundle compile (76
valuations, 14 groups, zero unruled); coverage selftest; publish-guard proof
(51 mutation-red/restored-green receipts); seal proof (7 cases/10 mutants);
license check; native-tls absence from a successful nonempty locked workspace
dependency tree; and design-validator selftest. The brokkr binary digest was
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`,
matching `bundles/self/tooling.json`. Canonical bundle scripts executed;
public shims were checked, not used to choose the verification algorithm.

Failed: `cargo test --workspace --locked` exited 101 at
`tests/delivery_recipes.rs:371`: chore model array was `["luna"]`, expected
`["sol"]`. The same test also expects high effort while the adjacent
`fast_implementation_uses_luna_at_xhigh_without_provider_fallback` requires
Luna/xhigh and passes. The coverage gate exited 1 because its instrumented
suite hit that same assertion, preserving `target/coverage/run.dLrCF6/run.log`.
There is no successful exact-coverage result for this candidate and no claim
that the full suite completed. No test, source, recipe or gate was edited by
the chief; the mismatch is in unchanged baseline files. AC8/T6 specifies the
bounded downstream repair and required full revalidation rather than weakening
or bypassing the gate. The earlier panel's historical passes are not substituted.

All 16 planned checks executed: 14 passed, two failed as recorded. Required
workspace/coverage success remains outstanding. The design may be drafted
and structurally checked without claiming delivery complete. Fresh final
candidate verification, CI-owned proofs and five protected results/security
disposition remain downstream; the original true/low facts remain unchanged.
