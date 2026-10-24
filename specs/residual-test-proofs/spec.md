# Issue #191 residual test proofs

Status: Drafted design; execution evidence and delivery verification remain pending.
Date: 2026-10-24.
Scope: Epic #192, issue #191, all five supplied INFO findings (23.1–23.5).
Inspection baseline: `db0b51757f04673c5439b5c8c03851a6c932b9d9`.
Integrated comparison baseline: `34eb99f20d102914a7c9d53ed6b701ba90a1af7d`.

## Feature and rationale

Close the five findings by preserving the existing narrow fixes and proving
that their tests discriminate the defects they name. Dockerfile already names
AC52.2. Commit `db0b517` already corrected helper prose, removed the unused
metric wrapper, retained the refused target's port and added a snapshot source
assertion. Inspection supports these changes; it is not test execution.
The smallest additional implementation is therefore zero lines.

The previous released port allowed a competing test to take the target. The
previous idle-map output assertion accepted separate locks for the total and
rows. These were gaps in regression evidence, not demonstrated production
failures. Truthful helper prose also protects reviewers from confusing raw
Host/Origin fixtures or pre-trigger counter observations with other semantics.

Reuse this existing `residual-test-proofs` directory as the single owning
design. Retained implementation relative to the integrated baseline is limited
to `tests/common/mod.rs` documentation/unused wrapper, the refusal fixture in
`tests/webhook.rs`, and the inline snapshot test in `src/api.rs`. Dockerfile
is a preservation check. No new production behavior, API, dependency, gate,
policy, accepted-decision edit or unrelated cleanup is authorized.

## Authority and classification

Retain intake's `engine` classification: socket lifetime and snapshot locking
are concurrency concerns under [0057 §7](../../docs/decisions/0057-codex-delivery-recipes.md).
Keep the policy-selected full route, protected review and hard security hold;
small INFO findings do not authorize a cheaper route. This design does not
choose the engine's next phase or dispatch additional seats.

Preserve synchronous accept/store, subscribe-before-check, oldest-only pruning
and extraction at ingest (0001–0005). Accepted
[0047 §2](../../docs/decisions/0047-storage-rpc.md) already requires one snapshot
for cumulative total and rows, including LF overflow. Accepted
[0041](../../docs/decisions/0041-webhook-boundary.md) and
[0048](../../docs/decisions/0048-rest-webhook-validation.md) preserve independent
webhook delivery, offline PUT validation and retry/ledger behavior.
[0043](../../docs/decisions/0043-mcp-origin-boundary.md) is authority for the
Host/Origin test boundary. Follow [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
for deterministic measured inline tests and [0050](../../docs/decisions/0050-delivery-gates.md)
for proof ownership. This is evidence repair, with no semantic amendment to a
frozen guarantee, public contract or ruling; no new ADR is warranted.

## Panel synthesis and rulings

This synthesis reads BOTH `.brokkr/state/design/positions/simplicity.md` and
`robustness.md`, plus the current intake
`.brokkr/state/tasks/epic-192-issue-191-satisfy-all-f-98b59273.md` and prior
framing. These remain ignored evidence, not committed deliverables.

1. **Adopt simplicity's zero-additional-edit default and robustness's proof
   requirement together.** Both current positions agree on preserving the
   existing implementation. Neither claims executed acceptance. Reject a
   cosmetic patch merely to create work; reject closure based on inspection
   alone. Cost: mutation, platform and gate work remains despite no new code.
2. **Adopt both positions' narrow helper ownership.** Preserve the corrected
   paragraphs and wrapper deletion, verbatim Host/Origin bytes, deadlines,
   pre-trigger counter baselines and labeled gauge polls. Reject helper merging,
   optional Host defaults, normalization and polling redesign. The cost of
   retaining separate helpers is smaller than reopening security fixtures.
3. **Adopt continuous non-listening port ownership and the bounded real
   refusal probe.** Reject bind-and-drop, a listener, fixed ports, retries,
   skips or platform fallbacks. They either restore the ownership race or
   substitute a TLS/read failure for connection refusal. Cost: one held socket
   descriptor, one probe and actual Linux/macOS execution evidence. Ordinary
   supported-platform ownership is the boundary; no adversarial socket service
   or global allocator is justified.
4. **Rule for the complete-function source assertion plus exact output.**
   Both current positions support CRLF normalization and outer trim only.
   Reject token counting, permissive internal normalization, probabilistic
   writers, runtime lock wrappers, callbacks, parser dependencies and new test
   seams. The rejected runtime-instrumentation alternative would better count
   arbitrary acquisitions but adds ownership and coverage obligations beyond
   this task. The chosen proof loses that generality and intentionally fails
   on benign internal formatting/refactoring. It pins this tiny reviewed body;
   it does not certify arbitrary callee internals. Require a compiling split-lock
   mutation to fail the structural assertion while the output assertion passes.
5. **Adopt robustness's contract limits without enlarging implementation.**
   Exact stored counts before and after observing exhaustion prove queryability
   at those moments, not that SMTP temporally beat retry exhaustion. Storage
   ordering, crash semantics, pruning, DATA handling, extraction and TLS trust
   remain unchanged. Reject an outbox, new timing guarantee or TLS bypass.
   A real wider failure triggers escalation, not preventive architecture work.

There is no unresolved substantive conflict between the current positions.
The previous design's optional second-bind and tolerant-normalization debate
is historical; neither is a recommendation of this panel. The rulings above
choose concrete costs rather than averaging incompatible mechanisms.

## Acceptance Criteria

- **AC1 — 23.1, Dockerfile attribution.** Dockerfile retains the AC52.2
  digest-pinning comment and the identical builder digest. Its bytes equal the
  integrated comparison baseline. No image refresh follows from this finding.
- **AC2 — 23.2, truthful helper documentation.** `http_send_text` owns the
  raw-text description, `HELPER_DEADLINE` owns deadline prose,
  `http_send_text_with_headers` owns extra-header/Origin prose, and
  `send_with_host` has exactly one mandatory verbatim Host description with no
  optional/default claim. Inspection and existing consumers show unchanged
  signatures, request bytes and deadlines.
- **AC3 — 23.3, unused wrapper only.** Repeat an exact identifier search across
  repository source, imports/re-exports and relevant hidden/non-Rust files,
  excluding VCS/build/run evidence. Distinguish documentary references. Confirm
  no executable `wait_for_metric` declaration or caller remains and compare its
  deletion with the integrated baseline. Do not recreate it to delete it again.
  Preserve `metric_now`, `wait_for_metric_from`, `wait_for_metric_row` and all
  live callers. If a new deletion becomes necessary, repeat caller search
  immediately beforehand; a live caller requires scope reconciliation.
- **AC4 — 23.4, owned refusal and exact observations.** The webhook test holds
  an IPv4 Tokio socket bound to `127.0.0.1:0`, reuse disabled, never listening,
  from setup through the final assertion. A real connection bounded by
  `HELPER_DEADLINE` must return `ConnectionRefused`; success, timeout or another
  error fails. Keep PUT, pre-mail exhaustion baseline, SMTP, exact stored count
  one, one new exhaustion, exact stored count one, then explicit drop, in that
  order. RAII covers unwind. The direct probe does not affect the webhook
  ledger. No sleep synchronization, retry, early drop or fallback. Execute on
  Linux and macOS; unavailable evidence remains outstanding.
- **AC5 — 23.5, snapshot discriminator.** The existing measured inline test
  retains its local registry and exact total six, LF overflow two, `a` two,
  `b` one and `c` one. Extract the entire production `cumulative_rows` body
  before the inline test module using mandatory signature and `render_rows`
  boundaries; missing anchors fail, and the expected literal cannot match
  itself. Compare with only CRLF normalization and outer trim. The assertion
  rejects an added lock, early guard release or delegated replacement of this
  body. Require current-pass / compiling sum-unlock-relock-render-fail /
  exactly-restored-pass evidence. The mutant must pass the rendered-output
  assertion and fail specifically at the source assertion; a compiler error or
  unrelated failure is insufficient. Never retain or commit the mutant.
- **AC6 — scope and escalation.** Allow zero additional implementation edits.
  The retained implementation diff from `34eb99f` is only the three named
  test/docs regions; runtime `cumulative_rows` and all other production behavior
  remain unchanged. Dockerfile, Cargo files, accepted decisions, gates and
  policy remain unchanged. Keep branch-stable assertions inline and measured.
  A need for runtime changes, caller migration, a new seam/cold-path harness,
  incompatible socket semantics, dependency or unrelated edits returns for
  scope escalation before widening implementation.
- **AC7 — executed verification.** Exercise the affected real-server API,
  lifecycle, binary, webhook and MCP consumers. Before delivery completion,
  leave every gate in [the sole house inventory](../../docs/house-rules.md#gates--part-of-every-change)
  clean, with locked builds, exact zero-missed coverage, its selftest, pinned
  proof/tool ownership, publish/seal proofs, license check and fail-closed
  native-tls absence. Keep CI-only proofs and unavailable platform evidence
  distinct from local passes. Preserve protected review/security hold. Commit
  unsigned; never push, merge, tag or publish.

## Dated clarification — 2026-10-24, current synthesis

The original `0298c31` design framed four residual repairs prospectively at
`34eb99f`. Implementation `db0b517` is now the inspected baseline, and this
run explicitly includes the already-correct Dockerfile criterion. This revision
supersedes stale prospective wording without asserting any historical task or
execution succeeded. The original design remains available in Git history;
prior framing is untouched. All implementation/proof tasks remain unchecked
until their actual receipts exist.

## Design handoff

The chief commit touches exactly this spec, [plan](plan.md) and [tasks](tasks.md)
in one directory. No position file or result JSON is committed. The final
pinned deterministic check owns the design verdict. Drafting and structural
validation establish no runtime, platform or full-gate pass.
