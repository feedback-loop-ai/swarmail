# Repair the eval secrecy and publication proofs

Issue: Epic #192 / #234. Phase: design. Implementation and its causal proof remain outstanding.

## Feature and rationale

Close the two named MEDIUM test debts left by #200 without changing production behavior or weakening an assertion. The existing entropy tests replace the constructor's private RNG, so they cannot detect a constructor that derives secrets from a public seed. The publication-failure test uses business snapshot version 1, which current validation refuses before any injected write failure. Passing either test today does not establish the intended claim.

Add an independent production-constructor experiment while preserving the controlled experiments. Update the publication fixture to the current snapshot constant and prove that each intended injection actually ran. Permit exactly one prerequisite repair: import `std::io::Write as _` inside the existing Linux-only `evals/src/runner.rs::test_probes`, whose missing trait currently prevents standalone eval tests from compiling.

## Authority and scope

The issue README and operator rulings under `.forge/epic192/issue234/` were verified against their packet-relative SHA256SUMS on 2026-10-25. The README records no prior #234 outcomes. Intake is `.brokkr/state/tasks/epic-192-issue-234-tighten-200-s-c0b38faf.md`; both design positions are under `.brokkr/state/design/positions/`. These are run-local evidence, not committed design dependencies.

Accepted [0055](../../docs/decisions/0055-agent-eval-harness.md) requires standalone eval evidence. Accepted [0071](../../docs/decisions/0071-eval-trap-scoring.md) governs current business snapshot version 2. [0063](../../docs/decisions/0063-eval-provenance.md) remains proposed; this repair neither accepts it nor changes its contracts. No new decision is required: only tests of existing behavior change. Preserve synchronous accept/store, subscribe-before-check, oldest-only pruning, AUTH inbox routing and ingest extraction (0001–0005).

Implementation source scope is existing test modules in `evals/src/fixture.rs`, `evals/src/artifacts.rs` and the single runner test import. Preserve production code, dependencies/locks, schema readers, scoring, envelope versions, coverage denominator, gates, engine pin, recipe routing and decision statuses. Design/delivery records may document the evidence. No new entropy abstraction, production seam, integration harness, network dependency, namespace work or retry mechanism is authorized.

## Panel synthesis and explicit losses

Adopt both positions' preservation of every deterministic control, canonical private projection, actual constructor/planner use, current schema constant and precise compilation prerequisite. Adopt simplicity's small test-local setup extraction and rejection of RNG interception or generic entropy infrastructure.

Rule for robustness on the sampling conflict: use four independent pairs per catalogue task, compare the two ordered vectors of private projections separately for each task. Reject simplicity's single catalogue-wide inequality. A healthy random task must not conceal a task whose entire secret projection has become deterministic. Cost of this ruling: eight local fixture plans per task and a negligible nonzero collision risk instead of the smallest possible test. Neither approach certifies every field independently or proves OS entropy quality. Four matching pairs of a lone six-digit OTP have probability approximately 1/(900000^4), assuming independent uniform draws; no individual OTP inequality and no retry-until-green are allowed.

Rule for robustness on publication: retain the schema correction and both original assertions, and additionally require a local injection witness plus the exact injected error. Reject simplicity's literal-only repair because a future early validation error could again satisfy the original assertions. Cost: two small assertions and a local boolean, without changing the publisher or failure mechanism. Already-published partial files may remain; absence of a completion marker is the existing contract, not atomic rollback.

Reject broader infrastructure and scenario expansion: the named failure boundaries need in-memory fixture planning and the existing real file publisher. Existing real-protocol suites continue to prove protocol behavior; this change makes no new persistence, TLS, webhook, hostile-host or concurrency guarantee.

## Acceptance Criteria

1. `entropy_proofs::production_construction_keeps_secrets_independent_of_public_seed` visits every catalogue task. For each of four fresh pairs, both instances use production `Private::with_inputs`, the same explicit public seed, fixture timestamp, task, trial ID, app URL and equivalent account data. Neither path replaces, reseeds, clones or manually consumes `private_rng` before normal `plan()`.
2. Each planned fixture has actual secret material, not merely a nonempty JSON object or null-password array. Reuse the sorted `private_projection` of tokens/OTPs, ticket IDs/challenges, invite UIDs and passwords. Require the expected secret-bearing collections for OTP, reset, support and invite scenarios. For each task, the two vectors of four projections differ; differences in fresh IDs, clocks, public state or wire formatting cannot satisfy this assertion.
3. The new focused test passes with the unchanged production constructor. A temporary constructor mutation deriving `private_rng` from the explicit public seed makes that same test fail specifically at private-projection inequality. Record the command and red/green result; restore the exact original initializer and rerun green. Compilation, setup, schema or tool failures do not count as the negative control. Commit no mutation.
4. Preserve all existing assertions and deterministic setup in `every_fixture_private_class_uses_independent_entropy_not_public_seed` and `changing_secret_consumption_cannot_change_public_message_suffixes_or_schedule`: equal private projections across differing public seeds, snapshot exclusion of all currently checked secret classes, and unchanged public headers/invite schedule when secret consumption changes.
5. `every_publication_stage_refuses_without_a_completion_marker` uses `crate::state::SNAPSHOT_SCHEMA`. For each of `result.json`, `state.json`, `diagnostics.log`, `metadata.json`, retain a fresh temporary root, the original error assertion and absent-metadata assertion; additionally assert the chosen callback stage was reached and the error equals `injected write failure`. Non-target stages still use real `publish`. Business v1 remains refused and version-1 artifact/replay envelopes stay unchanged.
6. The final implementation diff is confined to the scoped tests, exact test import and necessary delivery records. Offline locked standalone eval fmt, clippy, focused tests and full suite pass. All local house/pinned gates in the plan pass on committed work, including exact zero-missed coverage and its selftest; root green alone is insufficient. Existing assertions, thresholds and gate policies remain intact.

CI owns macOS eval evidence through #201, privileged port-80 exchange, chart/history checks, Docker smoke, network RustSec and tag-only release proof. The operator owns live GitHub settings and sealing; #221 owns namespaces. None is an additional local acceptance criterion. No push, merge, tag, publication, issue closure or decision acceptance is authorized.
