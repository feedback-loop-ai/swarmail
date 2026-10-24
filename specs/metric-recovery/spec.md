# Recover the metric amendment handoff

Epic #192 / issue #202. Recovery design, 2026-10-24.
Baseline: `faef809` after concluded run
`epic-192-issue-202-record-the-op-6b55bb9e`.
Current run: `epic-192-issue-202-recovery-afte-390dd9cc`.

## Feature and rationale

Repair the acceptance-recording documentation so the next runtime implementer
receives an intact historical boundary, truthful task evidence and an explicit
refused-control regression obligation. The retained review has three LOW
residuals, including an explicitly witnessed security residual. One blank
line alone cannot discharge them. This design implements no runtime decoding.

Accepted [0065](../../docs/decisions/0065-decoded-inbox-metrics.md) is authority,
not a new declaration. It already settles select-segment-first, percent-decode
once, strict UTF-8 success and whole-original-segment fallback on invalid UTF-8.
This recovery changes no public contract or ruling and needs no new decision.
Do not accept or renumber anything; preserve #180=0062, #181=0063, #174=0064
and #202=0065, including reservations with no local decision file.

## Evidence and exact residuals

The intake at `.brokkr/state/tasks/epic-192-issue-202-recovery-afte-390dd9cc.md`
and both `.brokkr/state/design/positions/{simplicity,robustness}.md` were read.
The four retained `.forge/results/27f3a39b-e0b0-486e-8f99-8163807e0013-*.json`
review results are historical evidence, never fresh verification.

1. Correctness: the sentence ending "implementation or test completion."
   runs directly into "No authentication, CORS policy ..." in
   [HTTP-plane scope boundaries](../http-plane-edges/spec.md). Insert one
   blank line, preserving both paragraphs' wording and order.
2. Guarantees: [recording tasks](../record-metric-amendment/tasks.md) T1–T7
   remain unchecked, while the implementation receipt was appended to the
   decoded-metrics plan instead of the recording plan. Add a dated recovery
   receipt to the owning recording plan and reconcile each whole paired proof.
   Preserve historical receipts, including the old failed design gates.
3. Security: request accounting precedes admission. Future decoding therefore
   exposes refused CR/NUL in transient request labels under existing escaping.
   The admitted-label regression does not observe this refused-request window.
   Add a falsifiable, still-unexecuted obligation to the runtime spec, plan and
   tasks. No demonstrated injection or security clearance is inferred here.

The tests INFO finding remains disclosed: `tests/store_contract_docs.rs` uses
an audited finite `AMENDMENTS` list; its inventory/status tests cover 0065,
but its amendment-pair test does not establish the 0065-to-0043 marker's
semantics. Direct review remains required. Accepted 0017's unrelated
historical 0065 reference stays untouched.

## Panel rulings and costs

Adopt simplicity's six-existing-Markdown-file repair and existing parser/test
ownership. Reject an amendment-validator redesign, new parser, unconditional
process harness or permanent Rust test merely for a paragraph separator.
A focused baseline-negative/candidate-positive assertion is sufficient for
that separator. The cost is explicit manual semantic/link review rather than
claiming automated amendment coverage that does not exist.

Adopt robustness's deterministic transient observation, complete-scrape parsing,
headroom audit and evidence ownership requirements. Reject any interpretation
of simplicity that reduces recovery to formatting or treats completed-refusal
scrapes as transient-label proof. The cost is a more demanding future fixture,
possibly a test-only middleware seam; the benefit is a regression that can
actually observe the security-relevant window. The panels agree on the bounded
recovery architecture and decoding contract; there is no competing identity
rule to average.

Choose conditional process isolation over always spawning a server process:
first establish exclusive names and cumulative-cap headroom for the whole
executable. Isolation is required if that cannot be proved. This saves routine
process supervision but costs a fixture audit; overflow cannot replace a
required named sample. Reject moving accounting after admission, changing
escaping, sanitizing labels, adding aliases or registry resets. Their apparent
simplicity would lose refused-request ownership or alter accepted 0065/0043.

Accept the documented raw-fallback collision (`%FF` refused versus `%25FF`
admitted) and literal-control exposure. Request metrics are activity, not
admission or storage receipts. The request family is separate from cumulative
await overflow: `%0A` is not a cross-family collision, and the 4096 cumulative
name cap does not bound concurrent request labels. Robustness's wider storage,
webhook, chaos and TLS analysis supplies preservation constraints only.

## Acceptance Criteria

1. **AC1 — Paragraph integrity.** The HTTP-plane spec has exactly the missing
   blank-line insertion at the identified boundary relative to `faef809`.
   A focused assertion fails on that baseline and passes on the repaired
   candidate; both paragraph texts and order remain byte-identical.
2. **AC2 — Owning evidence and truthful state.** The recording plan has a
   dated recovery receipt distinguishing historical candidate/reviews from
   fresh commands, outcomes and unresolved obligations. Every checked T1–T7
   has its whole paired proof mapped to evidence; incomplete tasks stay open.
   T8 requires actual independent review evidence and disposition, never just
   dispatch. Both existing decoded-plan receipts remain verbatim. A dated
   clarification accounts explicitly for AC1's authorized blank line when
   checking historical bodies; no blanket whitespace normalization is allowed.
3. **AC3 — Falsifiable future refused-control proof.** Append dated owning
   clarification to decoded-metrics AC1/AC5 and corresponding plan/tasks:
   use an exclusive prefix plus `%0D%00`; capture and pin the current real-HTTP
   control refusal's exact status and raw/text body before runtime change
   (the existing contract is 400, `inbox must not contain control characters`).
   While a deterministic hold keeps the request-accounting guard alive, one
   strict complete scrape must recover the exact decoded CR/NUL label with
   exactly one matching request sample of value 1, no encoded alias and no
   injected sample. Show no active await or cumulative registration increment
   for that identity. Release, join the identical refusal, then observe request
   row removal and continued absence of await registration. Use real HTTP,
   exclusive names, proved cap headroom, bounded operations and failure-safe
   cleanup. Preserve the separate invalid-UTF-8 raw/text compatibility proof.
   These remain future obligations, not results of recovery.
4. **AC4 — Existing authority and scope.** Change no accepted decision,
   decision index, reserved lane, source, dependency, gate or recipe. Runtime
   decoded-metrics T2–T8 remain unchecked. Preserve middleware/guard lifetimes,
   refusal behavior, metric families/escaping/totals, MCP identity, 0047's
   cap/LF overflow, synchronous accept==stored, subscribe-before-scan,
   oldest-only pruning and ingest extraction. No new decision or acceptance.
5. **AC5 — Documentary proof.** `cargo test --locked --test store_contract_docs`
   passes with its existing refusal controls. Resolve affected relative links,
   compare historical receipts/decision bodies to the recorded baselines, check
   the exact allowed path inventory and run `git diff --check`. Retain the
   finite amendment-audit limitation and direct semantic review. If a current
   contract test changes, its owning real-protocol suite and meaningful
   failure-sensitive control must pass; future decoded behavior is not a
   current-contract test.
6. **AC6 — Delivery and review.** The eventual implementation candidate passes
   the complete current [house-rule local gates](../../docs/house-rules.md#gates--part-of-every-change),
   including exact coverage/selftest, publish/seal proofs, digest-matched
   compile and fail-closed native-tls absence. Keep CI-only proofs with CI.
   Preserve triage's engine route and four independent lenses: Qwen correctness,
   Claude security, official DeepSeek Flash guarantees (never Pro), GLM 5.3
   tests with the task-authorized dispatch-only Haiku fallback. Fallback is
   only for dispatch failure, never replacement of a substantive verdict.
   Protected review and security hold remain; only actual independent review
   can dispose of the witnessed security residual. No push, merge or issue close.

## Scope boundary

The chief commits only this directory's spec, plan and tasks. Downstream
implementation repairs the six existing files listed in the plan. Documentation
and tests are the outer task scope, but no new Rust test is required for these
prose repairs. Any necessary runtime/source/contract change is a scope defect
to return, not permission to expand recovery. A design-check pass establishes
artifact structure, not implementation, gates or security clearance.
