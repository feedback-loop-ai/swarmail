# Metric review continuation — issue #202

Date: 2026-10-24. Epic #192, issue #202. Design baseline:
`995661dc4bfaf7f4a7b96ae1385f67727138f730`, tree
`87f6f8bff7db29c754a3d5a5ed94b6bc1f61c2bb`.

## Feature and rationale

Continue the stopped decoded-inbox-metric review with an explicit residual
account, corrected current review instructions, and fresh candidate-bound
proof. The decoder and refused-request regression already exist. The missing
outcome is trustworthy continuation evidence and protected review, not another
metric implementation. Historical receipts certify their recorded candidates;
they do not certify this checkout or dispose of its security residual.

The runtime contract remains [decoded inbox metrics](../decoded-inbox-metrics/spec.md)
and [accepted 0065](../../docs/decisions/0065-decoded-inbox-metrics.md).
[0058](../../docs/decisions/0058-codex-review-routing.md) supplies the current
engine review matrix; [0050](../../docs/decisions/0050-delivery-gates.md)
supplies evidence and residual-admission rules. These are authority references.
This design changes no public contract, frozen guarantee or ruling and proposes
no ADR. Any later remedy contradicting accepted identity, escaping or refusal
semantics needs a separate proposed amendment and operator acceptance before
implementation under 0045 §8. Acceptance of 0065 is not security clearance.

## Recovered evidence and disposition

Source: `.forge/epic192/recovered-review-evidence.json`. Both complete panels,
all eight member notes and the five copied original result files were read.
All five source-file SHA-256 values match the manifest; this proves copy
integrity, not independent authentication of the source journal.

| Historical panel | Exact event | Recorded outcome |
| --- | --- | --- |
| `epic-192-issue-202-record-the-op-6b55bb9e` | 581; `a0d79d66efad4f680c3f1e4f3690c7803227605d4b26fbd42ab91a221b186717` | residual; all four members residual |
| `continue-epic-192-issue-202-usin-39c8c4ef` | 796; `d6640e1b01d6c3d364af29f1b062bbef0e629b518a9a1d432aa5484d41fdf9d8` | residual; correctness residual, guarantees/security/tests clean |

Both aggregate inputs are exactly `fixes_applied=false`,
`has_security_residual=true`, `max_residual_severity=low`, `spec_defect=false`.
The later panel names historical candidate `cca967b` / tree `981ccc32`.
No inspected evidence proves either aggregate malformed. Retain true/low;
a clean member or conformant local parse does not prove universal scraper
compatibility. Historical member inputs also remain intact, including the
first tests member's info/nonsecurity assessment; do not normalize members
to their aggregate or rewrite originals. Do not fabricate journal events.

| Notes accounted for | Ruling for this continuation |
| --- | --- |
| First correctness: paragraph separation and receipt location; first guarantees: stale tasks/receipt placement | Historical repair result `4e062518-31d5-4134-b6cb-8dd887b2c7f3.json` records `62e6496`. Check repaired current artifacts; preserve the receipt and avoid repeating completed repairs. |
| First security: refused decoded controls need observation during live accounting | Existing post-response held real-router scrape supplies the prescribed regression. Re-execute it; retain the low security concern. |
| First tests: finite amendment audit; first guarantees: draft provenance; correctness/guarantees: unrelated 0017 number ambiguity; correctness/tests: GIT_CONFIG environment failure | Retain the disclosed 0065-to-0043 manual-review obligation. Inventory/status tests do not prove that semantic pointer. No unrelated ADR, numbering or environment-history rewrite. |
| Later correctness: literal CR/NUL may disrupt stricter external scrapers; security: no demonstrated exposition injection | Both can be true. Carry true/low to fresh protected review; do not relabel the concern informational or nonsecurity to advance. |
| Later guarantees and tests: accepted decoder, guard lifetimes and real-wire proof conform | Preserve implementation and existing proofs; obtain current execution and review rather than reuse their verdicts. |
| Later correctness: actual harness/parser names and broad CR/NUL sample filter | Keep actual names. The filter can false-fail against a future hostile fixture; no present false pass or conflicting fixture is evidenced. Apply the conditional isolation rule below only when required. |

## Panel synthesis and scope ruling

Adopt simplicity's existing helper, one middleware owner and one owning runtime
receipt. Adopt robustness's exact residual provenance, proof timing, cancellation
limits and five-result handoff. Their runtime recommendations agree; there is
no invented semantic compromise between them.

The positions differ in emphasis, not in an incompatible runtime proposal.
Choose simplicity's minimal implementation scope and robustness's explicit
failure-boundary audit. Both already make process isolation conditional.
Reject unconditional isolation and new parser infrastructure as unnecessary
alternatives, not as a demand the robustness position made. Keep the broad
control filter unless a new concurrent hostile fixture or a reproduced
headroom/interference failure invalidates ownership. Then require explicit
fixture attribution plus exact before/during samples under controlled
ownership, or a separate test process; never delete the injection assertion.
Deferring isolation accepts a possible future false failure and another
fixture audit; unconditional isolation would add process machinery without
an evidenced benefit today.

Retain robustness's failure limits in the plan and review without creating
another runtime contract. This costs a documented audit and fresh full gates,
while simplicity's single evidence owner avoids duplicate receipts. Reject
broad storage, SMTP, webhook, TLS, shutdown and chaos changes: their failure
analysis constrains preservation, not new repair scope. There is no genuine
panel conflict requiring a semantic compromise or a losing panel ruling.

Keep one-pass strict selected-segment decoding and whole-raw invalid-UTF-8
fallback, immutable RAII keys and pre-validation request accounting. Preserve
route selection, all-request totals, refusal bytes, no-await-registration,
family names, exactly backslash/quote/LF escaping, MCP identities and the
4096 cumulative-await-name cap with reserved LF overflow. Live request rows
are bounded by concurrent activity, not that cap. Accept existing alias
coalescing, `%FF` refused / `%25FF` admitted collision, linear allocation,
transient literal controls and non-atomic cross-family snapshots as contract
costs; this statement neither clears security nor authorizes shipping.
No sanitization, new refusal, lossy/repeated/whole-path decoding, compatibility
aliases, reset, dependency, metric, configuration or public test hook.

## Acceptance Criteria

1. **AC1 — Provenance.** A dated continuation account in the owning runtime
   plan maps all eight notes to current evidence or an explicit unresolved
   limitation, preserves both exact event identities/inputs and the five
   hash-matched originals, and distinguishes historical candidates from the
   current candidate. True/low remains recorded unless concrete evidence proves
   the original input wrong; local parser success alone cannot do so.
2. **AC2 — Current handoff.** Current metric-refusal-proof AC7, T9 and handoff
   prose require five distinct results under accepted 0058: Astra correctness,
   official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees and
   independent Fable 5.1. Keep dated four-member receipts as history with a
   dated clarification. No Pro, fallback, adapter or delivery-policy edit.
3. **AC3 — Decoder and lifecycle.** Existing helper fixtures pass for route
   selection/empty components, Unicode/case/padding, plus, malformed percent,
   `%2F`, `%252F`, `%FF` and mixed valid/invalid escapes with whole-raw fallback.
   Both strict UTF-8 arms execute. Two real encoded awaits yield all six exact
   decoded value-1 rows on the same parsed scrape while parked, no raw aliases,
   two matched==1 joins, live-row cleanup and cumulative retention. Preserve
   existing old-helper assertion-red/restored-green evidence as historical;
   repeat mutation only if relevant changes or evidence warrant it.
4. **AC4 — Refusal and independence.** Real invalid/mixed UTF-8 refusals retain
   exact status/body and no await registrations. The existing real-router hold
   occurs after the actual 400 exists, before guards drop, with no registry
   lock held. A full independent scrape observes exactly one value-1 decoded
   CR/NUL request row, no raw alias/injected sample and no fixture await rows
   before/during/after. Release yields identical refusal bytes and row removal.
   Preserve dropped-observer/release and aborted-owner proofs, without claiming
   those alone prove metric cleanup on arbitrary connection cancellation.
   Parser positive/negative controls and existing RAII tests remain green.
5. **AC5 — Isolation and scope.** Exclusive fixture identities and an explicit
   per-executable cumulative-name upper bound below 4096 remain valid under
   parallel execution. No sleep synchronization, registry reset, global totals
   or global serialization. Retain cfg(test)-only seams and 0020 cold-path
   placement. Frozen accept==stored, subscribe-before-scan, oldest-only pruning
   and ingest extraction, accepted decisions/statuses/reservations, dependency
   policy and exact coverage denominator remain unchanged.
6. **AC6 — Fresh proof.** Targeted suites and every local gate in the authoritative
   house-rules inventory execute with recorded command, exit, candidate identity
   and limitations. Verify pinned tool/script/shim identity and fail-closed
   native-tls absence; exact coverage has zero missed lines/branches/functions
   and its selftest passes. CI-owned proofs remain attributed to CI. A failed,
   missing or timed-out gate is outstanding, never inherited from history.
7. **AC7 — Protected disposition.** Fresh verify and all five protected results
   bind to the final candidate. Preserve the hard security hold and unknown-fact
   refusal. While security remains true, low severity cannot admit a residual
   under 0050. If accepted semantics and security disposition cannot be reconciled,
   leave delivery stopped and surface the exact conflict for the operator;
   do not fabricate clearance. Drafting, dispatch and passing tests cannot
   complete this criterion. No push, merge or issue close.

8. **AC8 — Reproduced gate mismatch.** Fresh design validation found
   `tests/delivery_recipes.rs::fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers`
   expecting Sol/high at lines 371–372 while the existing fast implementer,
   neighboring regression and house rules require Luna/xhigh. Downstream may
   reconcile only those stale expected model/effort values, retaining checks
   over all four strategy classes, exact model-array equality, no-default
   selection and the complete reviewer matrix. The targeted delivery-recipes
   suite, full workspace tests and exact coverage must then pass. Do not
   change the configured route, adapter, policy or tooling to satisfy the old
   assertion. This observed test-contract discrepancy is additional to, and
   cannot clear, the recovered true/low security residual.
