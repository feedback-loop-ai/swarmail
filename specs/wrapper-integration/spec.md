# Integrate the finished CLI wrappers

Date: 2026-10-26. Issue #151 part 1, epic #164.
Status: design draft; implementation and delivery verification remain outstanding.

## Feature and rationale

Bring the finished wrapper candidate through `6a67f7e` together with the locally
available main tip `89581763b8b4f4ddbc1a800c7cdb37dae0f2e3ff`, preserving both
histories. Correct the wrapper proposal's allocation to **0083**, index it as
**proposed**, and retain main's #243 tracked-authority links alongside #151's
launching-wrapper documentation. Change no wrapper behavior.

The [prior implementation receipt](../../evals/issue151-implementation.md)
records 15 passing wrapper tests and a successful dry-run, but root inventory
and coverage were blocked by the unindexed proposal and the full eval suite
failed on links into ignored delivery state. A passing isolated wrapper target
cannot certify the combined candidate. The purpose is to remove those two
integration blockers and obtain fresh local evidence, not measure models.

## Authority and evidence

Read both panel positions at `.brokkr/state/design/positions/simplicity.md`
and `.brokkr/state/design/positions/robustness.md`, the earlier framing under
`.brokkr/state/tasks/`, and the current
[integration intake](../../evals/issue151-integration-intake.md). The older
framing concerns building wrappers; the current task explicitly authorizes
the previously excluded index repair and local-main integration.

All entries in `.forge/epic192/issue151/SHA256SUMS` passed from that directory.
The required README digest is
`be74941231e61014f3f4caa0803713901a876d28a91f514b2e29febde76d5df9`;
the operator-rulings digest is
`bafa819f9f9296574aa3c1f5d5f30876d16584ae03fd05c7a71e2cc72ef9579e`.
The README's prior-runs section is empty; the supplied task and tracked receipt
supply the prior outcome. No historical evidence is invented. The ledger gives
0079/0080 to #152, 0081 to routing, 0082 to #153 and 0083 to these wrappers.
That allocation does not accept 0083. No required local input is missing.

The [house charter](../../docs/house-rules.md), accepted
[0050](../../docs/decisions/0050-delivery-gates.md),
[0055](../../docs/decisions/0055-agent-eval-harness.md) and
[0056](../../docs/decisions/0056-signing-policy.md) govern proof ownership,
independent eval verification and unsigned commits. This integration introduces
no semantic contract and declares no new decision. Implementation renames the
existing wrapper proposal to `docs/decisions/0083-cli-wrapper-launch.md`,
keeping its runtime provisions and proposed status. The
[launch specification](../cli-wrapper-launch/spec.md) remains the functional
contract; its obsolete scope statements receive dated integration corrections.

## Panel synthesis and rulings

- **Adopt simplicity's bounded repair.** Use Git ancestry, the existing decision
  index, #243's tracked authority and boundary suite, and existing wrapper tests.
  Reject new link linters, registries, aliases for the collided decision path,
  credential formats, retry policies, supervisors, dependencies and routing edits.
  Cost: this delivery provides no new confinement or authenticated CLI proof;
  future recipe changes still require their own review.
- **Adopt robustness's preservation evidence.** Compare against both merge
  parents, keep upstream eval proof repairs, and rerun the complete local root
  and standalone eval inventories against committed content. Historical green
  wrapper receipts are not a substitute. Cost: a full offline proof cycle for
  a documentation integration, plus explicit ancestry/content inspection.
- **Reconcile rather than invent a conflict.** Both members want the real local
  merge and all existing gates. Robustness's crash, pruning, waiter, hostile DATA,
  webhook and certificate analysis identifies contracts to preserve, not new
  work. Simplicity's smaller implementation wins over adding runtime guards;
  robustness's stronger evidence wins over relying on the small diff alone.
  Existing loss, retry ambiguity, retention and unconfined-execution limits stay.
- **Reject simplicity's reuse-only design packaging.** This sequence requires a
  chief commit identifiable as one design. Use this separate integration spec
  instead of replacing the detailed launch contract or importing main in the
  design commit. Cost: one additional three-file design directory; later repair
  must reconcile the old live scope text without erasing dated failures.

## Acceptance Criteria

These criteria apply to the implemented integration. Drafting this design does
not claim them met. Each is locally provable; external ownership follows below.

1. **AC1 — History and conflict resolution.** The integrated HEAD has both
   `6a67f7e` and `89581763b8b4f4ddbc1a800c7cdb37dae0f2e3ff` as ancestors,
   proved by `git merge-base --is-ancestor`. Its local merge retains #243's
   tracked ruling link and documentation-boundary section in `evals/README.md`
   and #151's wrapper guidance. No unresolved conflict markers remain.
2. **AC2 — One correctly allocated proposal.** The wrapper decision has filename
   `docs/decisions/0083-cli-wrapper-launch.md`, title number 0083, exactly one
   proposed status header, corrected allocation rationale and exactly one
   resolving proposed index row. The collided wrapper filename is absent.
   Every tracked reference to that proposal, including `specs/cli-wrapper-launch`,
   `evals/wrappers.md` and `evals/issue151-implementation.md`, uses 0083.
   References to #152's allocation are not rewritten. Earlier failures keep
   their actual verdicts/exits; dated clarifications replace stale live scope
   exclusions. No accepted ruling is rewritten or proposal accepted.
3. **AC3 — Tracked documentation boundary.** The root `store_contract_docs`
   target and full standalone eval `boundary` target pass unchanged, including
   #243's scanner and negative controls. Current proposal links resolve and
   tracked Markdown links do not target ignored delivery state. Plain historical
   provenance paths may remain text. No ignored packet is fabricated to pass.
4. **AC4 — Preserved executable behavior.** Enumerated wrapper source,
   entrypoints, declarations, tests and helpers match `6a67f7e` byte-for-byte.
   All other executable changes are precisely the incoming main changes,
   checked against both parents and the merge base. All 15 wrapper tests pass
   with their assertions intact. Preserve private-file admission, environment
   clearing, managed-config refusals, fixed argv, raw-output suppression,
   declared tool surfaces, sequential admission and runner-owned cleanup.
5. **AC5 — Fresh eval evidence.** Standalone eval fmt, locked all-target clippy,
   full tests and direct boundary/wrappers/baseline/real-server proof targets
   pass. The actual baseline dry-run exits zero and reports 16 tasks, six
   models, 32 controls, 480 paid slots, five paid repetitions, concurrency 1
   and 600000 ms. Its output directory remains absent; existing baseline tests
   prove no credential reads or trial launches. No paid/provider trial runs.
6. **AC6 — All local gates.** Every local house gate and the intake's pinned
   supplemental proofs exit zero on the committed integrated candidate using
   the digest-verified `.forge/bin/brokkr` and current bundle routing. Exact
   coverage proves zero missed lines, branches and functions; its selftest
   passes. Shim integrity, publish/seal proofs, warning-free bundle compile,
   licenses, design selftest, delivery tamper proof and fail-closed native-tls
   absence are retained. No test, floor, denominator, allowlist or gate is
   weakened. Command, tested commit, exit and log are recorded; partial output
   or transport timeout is not a pass.
7. **AC7 — Delivery integrity.** Authorized integration and receipts are committed
   unsigned, with no tracked worktree/index changes left. Review the final diff
   for unchanged root runtime, dependencies, scoring, schema and recipe routing
   except exact changes already supplied by main. No push, remote merge, tag,
   publication, issue closure, sealing or decision acceptance occurs.

## Ownership and limits

The specific local merge is authorized for implementation; remote delivery is
not. This chief commits only this spec, plan and tasks and leaves the merge,
rename and index edits to implementation. The design commit must remain HEAD
until the deterministic check consumes it. Gates run at this unintegrated
stage cannot certify AC1–AC7 or erase carried failures.

Preserve frozen 0001 accept==stored, 0002 subscribe-before-check, 0003 oldest-only
pruning, 0004 real protocols and 0005 ingest extraction. The integration adds
no durable webhook outbox, exactly-once delivery, indefinite retention or
crash-proof agent cleanup. Measured family tool asymmetry and selected existing
logins are settled operator directions, not new refusal grounds.

CI owns the charter's platform, Helm/chart, Docker, network RustSec and real
port-80 evidence; #201 owns macOS eval proof and #221 owns namespaces. The
operator owns authenticated six-model smoke/live tool inventories, paid baseline,
publication/reprioritization, live settings, sealing and acceptance. Issue #151's
baseline and reprioritization remain operator work; #152/#153 stay unmerged until
that baseline exists. These limits are not extra local acceptance criteria.
