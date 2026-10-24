# Issue #191 residual test proofs

Status: Drafted design; implementation and delivery acceptance remain separate.
Date: 2026-10-24.
Scope: Epic #192 issue #191, exactly findings 23.1–23.5.
Inspected candidate: `ffca01b1e04ad1aaa7b39eb2dc5ce2cdc894fcee`.
Integrated comparison baseline: `c3d124d` (rollup-a before the three draft commits).

## Feature and rationale

Finish the test-helper and stale-comment cleanup by repairing the remaining
refused-target fixture and requiring a falsifiable snapshot regression proof.
The three earlier commits (`361af41`, `7ed846f`, `ffca01b`) are drafts, not
acceptance receipts. Current source confirms the Dockerfile already cites
AC52.2, helper prose has its correct owners, `wait_for_metric` is absent, and
the snapshot test has a complete-function assertion. The prompt's known-gap
sentence about the unused wrapper is superseded by that source inspection.

The actual remaining defect is in
`tests/binary.rs::sigterm_with_a_delivery_in_flight_prints_the_final_counts`:
a temporary listener yields an address and drops before use. Another test
can bind that address before the webhook attempts. The held-port repair in
`tests/webhook.rs` is valuable but does not fix this binary fixture.
Idle snapshot output also cannot distinguish one lock from separate locks;
the retained structural assertion must demonstrate that distinction by an
executed, compiling negative control.

Reuse this directory as the single owning design. No production behavior,
public API, dependency, gate, policy or decision change is needed. Preserve
all valid earlier repairs; add only the missing binary fixture repair and
any strictly necessary correction within the five named findings.

## Evidence and authority

Read both current positions in `.brokkr/state/design/positions/` and intake
`.brokkr/state/tasks/epic-192-issue-191-tidy-the-test-7dc86ca4.md`.
The packet `.forge/epic192/issue191/README.md` passes its packet-local
`sha256sum -c SHA256SUMS`; the candidate-local engine digest matches
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Historical outcomes, including the truncated review, are context only.
No GitHub, CI, journal or sibling-worktree evidence is required or consulted.

Preserve synchronous accept/store, subscribe-before-check, oldest-only pruning,
real isolated protocols and ingest extraction (accepted 0001–0005).
[0047 §2](../../docs/decisions/0047-storage-rpc.md) already requires a single
locked snapshot for cumulative total and rows, including LF overflow.
[0041](../../docs/decisions/0041-webhook-boundary.md) and
[0048](../../docs/decisions/0048-rest-webhook-validation.md) govern independent
webhook delivery, retry accounting, shutdown and credential-safe diagnostics.
[0043](../../docs/decisions/0043-mcp-origin-boundary.md) governs Host/Origin
fixtures. Follow [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
for measured inline tests, [0050](../../docs/decisions/0050-delivery-gates.md)
for pinned proofs, and [0056](../../docs/decisions/0056-signing-policy.md)
for unsigned author commits. These are authorities, not new declarations.
This design strengthens evidence for existing contracts; no ADR is warranted
or authorized. Keep this candidate's recipe routing under
[0057](../../docs/decisions/0057-codex-delivery-recipes.md), protected review and
security hold; the design does not select the next phase.

## Panel synthesis and rulings

Both current positions agree on the concrete repair. There is no substantive
conflict to invent; their emphasis differs in implementation cost and proof
strength. The earlier design's zero-additional-edit conclusion loses to the
actual binary ownership gap.

1. **Adopt simplicity's fixture-local repair and robustness's full lifetime.**
   Bind a non-listening IPv4 socket with reuse disabled and keep it through
   final ledger/cause/redaction assertions. Require the real bounded refusal
   probe. Reject bind-and-drop, a listening dummy, fixed ports, sleeps,
   retries-to-green, skips and platform fallbacks. Cost: one held descriptor
   and one bounded connect per fixture. A shared reservation framework loses:
   it could reduce duplicated setup, but adds a lifetime API for two short
   fixtures without improving their evidence.
2. **Adopt both positions' preservation of the helper repairs.** Keep raw
   Host/Origin bytes, deadlines and live counter/gauge helpers. Reject helper
   merging, normalization, optional Host defaults and recreating the deleted
   wrapper. Cost: similar raw-client bodies remain, avoiding new behavior in
   existing boundary tests.
3. **Rule for the complete-function source assertion and exact output.**
   Adopt both positions' narrow discriminator and robustness's executed
   mutation/restoration requirement. Reject token counting, permissive internal
   normalization, probabilistic writers, runtime lock wrappers, new seams,
   parsers and harnesses. The runtime instrumentation alternative loses its
   broader acquisition accounting; the chosen fixture cannot certify arbitrary
   callee internals or endpoint routing. Accept its maintenance cost: benign
   internal refactoring requires reviewing the literal and rerunning the
   negative control. Inspect `/metrics` and `render_rows` as supporting evidence.
4. **Adopt robustness's contract limits without broader implementation.**
   Queryability before observing exhaustion does not establish SMTP-versus-retry
   temporal order. Storage/crash/pruning/DATA/extraction/TLS behavior remains
   unchanged. Reject an outbox, new timing guarantee or trust bypass. Cost:
   this narrow cleanup offers no broader durability or resource-bound proof.
5. **Reconcile platform and draft scope using the current operator instruction.**
   The named binary repair is required; a webhook-only fix and zero-edit rule
   are rejected. Linux refusal execution is local; macOS is CI-owned when the
   PR opens, not a pending local prerequisite. The cost is that this seat cannot
   attest Darwin behavior; CI remains its authority, with no invented pass.

## Acceptance Criteria

- **AC1 — 23.1, attribution.** Dockerfile cites AC52.2 and is byte-identical
  to `c3d124d`, including the builder digest. Inspection proves this comment
  correction; no image refresh or new spelling test is needed.
- **AC2 — 23.2, helper ownership.** Raw-text prose directly documents
  `http_send_text`; deadline prose documents `HELPER_DEADLINE`; extra-header
  and Origin prose documents `http_send_text_with_headers`. Exactly one
  `send_with_host` definition has one truthful required/verbatim Host
  description. Existing signatures, request bytes and deadlines are unchanged;
  real helper consumers pass.
- **AC3 — 23.3, dead wrapper.** Exact-identifier source/import/re-export search,
  including relevant hidden/non-Rust sources and excluding VCS/build/run
  evidence, finds no executable `wait_for_metric` declaration or caller.
  Documentary mentions are allowed. Preserve `metric_now`,
  `wait_for_metric_from`, `wait_for_metric_row` and their live callers.
- **AC4 — 23.4, binary ownership.** The named binary final-counts test holds a
  Tokio IPv4 socket bound to `127.0.0.1:0`, reuse disabled, without listening.
  A separate real connect bounded by `HELPER_DEADLINE` must return exactly
  `ConnectionRefused`; success, timeout or another error fails. Hold the
  reservation through configuration, SMTP, exhaustion, SIGTERM, EOF/reap and
  every final assertion; drop explicitly afterward, with RAII on unwind.
  Preserve the stalled sink's completed-request synchronization, exactly one
  final event, delivered/dropped/lagged zero, exhausted one, aborted one,
  transport-cause checks and absence of credentials/secret. A fresh child
  justifies exhaustion baseline zero. Execute the named test on Linux;
  macOS execution is CI-owned, not pending local work.
- **AC5 — preserve the distinct webhook proof.** Its existing non-listening
  reservation, bounded exact refusal probe, pre-SMTP exhaustion baseline,
  stored-count one / one new exhaustion / stored-count one observations,
  and final drop remain. Execute its named test locally. This is supporting
  evidence and cannot substitute for AC4; its macOS leg is CI-owned.
- **AC6 — 23.5, snapshot discriminator.** Preserve the measured inline local
  registry test and exact total six, LF overflow two, `a` two, `b` one, `c` one.
  Require the complete production `cumulative_rows` function, before the test
  module, using mandatory signature and following `render_rows` anchors;
  normalize only CRLF and outer whitespace. Missing anchors fail and the
  expected literal cannot match itself. The named test passes on current
  code, fails specifically at the source assertion on a compiling
  sum/unlock/relock/render mutant after its output assertion passes, and
  passes after byte-exact restoration. Zero selected tests, compiler failure
  and unrelated failures are not proof. Never retain or commit the mutant.
- **AC7 — scope and truthful handoff.** Reconcile all three owning artifacts
  to this candidate and the binary gap. Production code, Dockerfile, Cargo,
  decisions, gates and policy remain unchanged. Keep branch-stable assertions
  inline and measured. No new helper framework, dependency or exclusions.
  A need for wider behavior, caller migration or new seams returns for scope
  evaluation rather than silently expanding this cleanup.
- **AC8 — executed local verification.** Affected real-server consumers and
  every gate in [the sole house inventory](../../docs/house-rules.md#gates--part-of-every-change)
  pass on the final repaired candidate, including locked builds, digest-matched
  `.forge/bin/brokkr`, exact zero-missed coverage and its successful exit,
  coverage selftest, publish/seal proofs, licenses and fail-closed native-tls
  absence. Historical receipts and printed summaries followed by timeout do
  not pass. CI owns macOS and the house inventory's CI-only proofs; no local
  CI success is claimed. Preserve independent review; never push, merge, tag,
  publish, close the issue or accept a decision.

## Dated clarification — 2026-10-24, earlier synthesis (historical)

The original `0298c31` design framed four residual repairs prospectively at
`34eb99f`. Implementation `db0b517` is now the inspected baseline, and this
run explicitly includes the already-correct Dockerfile criterion. This revision
supersedes stale prospective wording without asserting any historical task or
execution succeeded. The original design remains available in Git history;
prior framing is untouched. All implementation/proof tasks remain unchecked
until their actual receipts exist.

## Dated clarification — 2026-10-24, rollup-a synthesis

The preceding entry is retained provenance, not the active baseline or scope.
The active candidate is `ffca01b`, with draft changes relative to `c3d124d`.
Source disproves both the stale unused-wrapper gap and the earlier assertion
that every repair is present: the binary port is still unowned. This revision
supersedes the zero-edit restriction, webhook-only refusal criterion and local
macOS requirement. Preserve valid helper/webhook/snapshot changes and implement
AC4. Gates run during design describe the unrepaired candidate only; final
implementation, mutation and verification receipts remain required.

## Design handoff

Commit exactly this spec, [plan](plan.md) and [tasks](tasks.md), keeping the
design commit last. Positions and result JSON remain ignored. The sequence's
pinned deterministic check owns its verdict. The chief reports only `drafted`;
a committed design or green baseline gates do not establish feature completion.
