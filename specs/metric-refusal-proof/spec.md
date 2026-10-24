# Metric refusal proof — issue #202 continuation

Date: 2026-10-24. Baseline: `62e6496`. Epic #192, issue #202.

Finish the accepted decoded request-metric implementation and supply the
missing executable evidence for a refused control-bearing request. The
previous recovery repaired documentation; its prescribed test did not observe
the transient request row. The retained security review explicitly reports a
low security residual, with no fixes applied and no spec defect. It does not
demonstrate injection, and it does not clear the residual.

## Authority and scope

[Accepted 0065](../../docs/decisions/0065-decoded-inbox-metrics.md) is the
existing authority, not a new decision declaration. This continuation makes
no new semantic ruling, so it creates no proposed ADR. Preserve that record,
all decision statuses, and reservations #180=0062, #181=0063, #174=0064 and
#202=0065. Do not accept, renumber or rewrite decisions.

The [decoded-inbox specification](../decoded-inbox-metrics/spec.md), including
AC1–AC6 and its dated AC1/AC5 clarification, remains the runtime contract.
This package supplies the current implementation/proof sequence. The source
prohibition in the prior [metric recovery](../metric-recovery/spec.md) and
runtime T9's final sentence describe that documentation-only recovery, not
this continuation. This task now authorizes the bounded implementation below.
Preserve the seven-file repair at `62e6496`, dated receipts and historical
clarifications. Reconcile obsolete present-tense instructions in the owning
runtime artifacts during implementation; preserve their dated history.

Select the existing inbox segment first, then percent-decode exactly once
with strict UTF-8. Success uses the whole decoded string; invalid UTF-8 uses
the whole original segment, even when other escapes were valid. Preserve
case, Unicode, padding, literal plus and malformed-percent treatment.
`%252F` becomes `%2F`; `%2F` is segment data. Keep route recognition,
extractor/admission refusals, no-await-registration, middleware order and
RAII accounting lifetimes unchanged. Keep metric names/escaping, MCP keys,
all-request totals and the cumulative 4096-name cap/reserved LF overflow.
The live request registry does not acquire that cumulative cap.

Valid aliases intentionally coalesce. Refused `%FF` and admitted `%25FF` can
both account under `%FF`; request metrics are neither admission nor commit
receipts. Consumers must use decoded selectors. No dual compatibility rows,
lossy/partial decoding, normalization, render-only transformation, new metric,
configuration, dependency, reset, storage schema or delivery-policy change.

Preserve synchronous accepted storage, subscribe-before-scan, oldest-only
pruning and ingest extraction. Storage, SMTP, webhook, TLS and chaos failure
boundaries remain unchanged; this feature makes no new durability claim.

## Panel synthesis and rulings

Adopt both positions' existing-helper strict decoder and immutable guard key.
Simplicity wins on production scope: robustness's wider failure analysis is a
preservation checklist, not authority for adjacent repairs. Accept the linear
scan/allocation and selector migration rather than add an identity layer.

On observation timing, choose robustness's hold **after the real handler
returns its refusal and before accounting guards drop**. Reject simplicity's
acceptable pre-handler-only alternative for this proof: that interval cannot
yet witness completed admission refusal. The chosen approach costs a small
post-handler test seam and response ownership across a test-only await; it
proves refusal and live accounting in the same interval.

Choose a request-owned one-shot installed through a test-only outer router
layer and consumed at the middleware owner, targeted to the exact exclusive
raw URI. Prefer this over simplicity's thread-local installation precedent:
thread-local setup can be smaller, but binds correctness to runtime/thread
placement and can accidentally be consumed by another request. The chosen
cost is a small test-only extension carrier and setup layer. It adds no
shipping route, flag or public hook. Keep accounting in the production owner.

Adopt simplicity's narrow shared test parser instead of duplication or a
parser dependency, with robustness's independent CR/NUL fixture and retained
malformed controls. Under 0020, seam access alone does not justify exclusion:
branch-stable tests remain inline; concrete cold timeout/failure diagnostics
justify an owner-declared harness. Accept that harness only for those paths.

Use exclusive names and an executable-wide cap-headroom audit first. Reject
unconditional process isolation; pay child startup/supervision only if the
audit cannot prove headroom. A fresh in-process server or a momentary cap
check does not provide isolation. No global resets, suite serialization or
shared-total assertions can replace per-name proof.

The runtime specification remains the evidence owner; this required design
package links to it instead of becoming an alternate metric contract.
Reject a general amendment validator, gate rewrite or another documentation
recovery. Preserve the disclosed finite semantic-pointer audit limitation.

## Acceptance Criteria

1. **AC1 — Pin real refusals before changing runtime.** Capture exact status
   and raw/text response bytes for exclusive-prefix `%0D%00`, `%FF`, and
   mixed valid-escape/invalid-UTF-8 awaits over real HTTP. Pin observations,
   including the control refusal's existing 400/message; do not assume an
   extractor JSON envelope. The same assertions pass afterward. Raw, decoded
   control and would-be lossy identities gain no active/cumulative await rows.
2. **AC2 — Prove the selected identity.** The owning AC1 helper matrix passes
   for recognized, unrelated, missing and empty-component routes, ASCII,
   space, Unicode, padding, plus, slash data, double encoding, literal,
   incomplete and nonhex percent, invalid UTF-8 and mixed invalid input.
   Both strict decoding outcomes execute; mixed invalid input retains its
   complete original segment. No other production behavior changes.
3. **AC3 — Prove ordinary decoded accounting and lifecycle.** Two exclusive
   space/Unicode encoded awaits park concurrently with demonstrated headroom.
   Once both active rows are observed, immediately assert one complete parsed
   stable scrape has exactly one value-1 decoded row per inbox in request,
   active-await and cumulative-await families, with no encoded aliases.
   Do not poll until the HTTP-row assertion passes. Seed through encoded
   routes, join successful `matched == 1` responses, then prove both live
   rows disappear and each cumulative row remains 1. Restoring only the old
   raw helper fails the specific decoded-row assertion; restored code passes.
4. **AC4 — Close the missing transient observation.** Target an exclusive
   prefixed CR/NUL await on a real production router over an ephemeral TCP
   listener. Hold after its actual refusal response exists, with request and
   total guards still alive and no registry lock held. During the reached/
   release handshake, parse one complete real scrape independently: exactly
   one request sample at value 1 has the exact decoded prefix plus CR/NUL;
   the raw encoded alias and any fixture-attributable extra/injected samples
   are absent. No active/cumulative await registration exists before, during
   or after. Release, join the baseline-identical refusal, and observe request
   row removal. Old-helper red must be an explicit wrong-label assertion,
   never a timeout/compile error; restored helper must pass. A final scrape,
   renderer call, admitted hostile name or unread client is insufficient.
5. **AC5 — Make the oracle and fixture trustworthy.** Independent parser
   fixtures recover literal CR/NUL exactly and continue to reject illegal
   escapes, raw LF and malformed syntax. No trimming/sanitizing controls or
   renderer-derived expected strings. Both wire scenarios have bounded HTTP,
   readiness, release, join and shutdown operations and failure-safe task/
   server ownership. The hold cannot intercept metrics/unrelated requests;
   dropped release or cancelled owners cannot strand it. Names and headroom
   hold under normal parallel execution, without resets or sleeps as sync.
6. **AC6 — Preserve authority and report proof honestly.** Owning runtime
   tasks/receipts map whole proofs to actual commands, candidate and outcomes;
   baseline, mutation-red and restored-green evidence remain distinguishable.
   Historical receipts and decision files/index stay intact. Review the
   0065-to-0043 scope directly: the finite `store_contract_docs` amendment
   list does not prove that pointer. Selector guidance matches asserted keys.
   No task is checked on the strength of a prescription or test name.
7. **AC7 — Verify and obtain protected review.** Targeted API, library API
   (including the new harness) and document tests pass, as do all fresh local
   [house-rule gates](../../docs/house-rules.md#gates--part-of-every-change),
   exact zero-missed coverage, its selftest, pinned tooling/shim checks and
   fail-closed native-tls absence. Preserve existing hostile-label, totals,
   admission, cap/overflow and guard lifecycle proofs. CI-owned checks remain
   attributed to CI. Under accepted [0058](../../docs/decisions/0058-codex-review-routing.md),
   the engine requires five distinct protected results bound to the final
   candidate: Astra correctness, official DeepSeek Flash security, Sonnet 5.5
   tests, Astra guarantees and independent Fable 5.1. No DeepSeek Pro or
   unauthorized fallback. Preserve recovered `has_security_residual=true`
   and `max_residual_severity=low` unless evidence proves the recorded input
   wrong; only actual protected disposition can resolve it. True security
   cannot admit a residual under 0050. See the dated continuation account in
   [the owning runtime plan](../decoded-inbox-metrics/plan.md).
   No push, merge or issue close.

A strict-scrape conflict must be returned with concrete evidence for the
appropriate ruling/review. It never licenses silently changing accepted
escaping, validation or decoding to make the test pass. This design records
obligations; it does not claim the feature or security review complete.
