# 0071 — Named eval traps invalidate a trial

- **Status:** accepted (operator, 2026-10-24).
- **Date:** 2026-10-24.
- **Issues:** Epic #192, issue #198.
- **Specification:** [Eval trap scoring](../../specs/eval-trap-scoring/spec.md).
- **Amends upon acceptance:** [0055](0055-agent-eval-harness.md) §§1, 2 and 9 (catalogue/snapshot/result revision and named trap precedence).
- **Preserves:** 0001–0005, closed failure taxonomy, isolation, runner lifecycle, root dependency/coverage policies and protected review.
- **Number:** The operator assigned 0071 to issue #198, preserving issue #200 at 0063 and assigning issue #181 to 0070. Other reserved numbers remain unchanged.

## Context

0055 permits a satisfied goal to pass after wrong actions. For tasks about
choosing among planted alternatives, this admits try-all strategies. The
fixture can honestly report both decoys and successful confirmation while the
scorer returns pass. Helpers that stop after a wrong action do not expose it.

Changing precedence or secretly destroying the app goal after a decoy both
amend the accepted contract. This proposal authorizes neither before operator
acceptance, under 0045 §8 and the charter.

## Proposed ruling

1. Seven scenarios acquire trial-wide disqualification from frozen events:
   confirm-decoys decoy_link/wrong_link; sibling-addresses
   foreign_token/wrong_message; i18n-subject decoy_message/wrong_message;
   stale-otp superseded_code/stale_code; magic-login decoy_link/wrong_link;
   entity-url decoy_link or marketing_mangled/wrong_link; digest-forty
   digest_decoy/wrong_link. Exact flows and attribution are normative in the
   linked spec. Ordinary records require a provisioned Agent. Sibling's
   known-Decoy claimant foreign-token refusals count with any recorded flow,
   including null: identity refusal can precede token lookup.
2. Infrastructure/invalid state precedes live deadline, then earliest
   matching trap precedes goal scoring whether before or after the goal.
   Without a trap retain goal, isolation exception, rejected-action and idle
   fallback. Application recovery remains usable; facts and HTTP behavior
   remain truthful. Required reset replay is positive evidence.
   Expired-but-not-superseded OTP and non-target tasks retain recovery.
3. Keep one scorer, existing validation and mutex/freeze boundary. Match
   exact action/status/class/flow/claim; otherwise structurally valid
   mismatches are nonmatches, not new harness errors. No taint flag, general
   policy language or claim of snapshot authentication on an unconfined host.
4. Disclose the rule in affected goals. Advance existing catalogue, snapshot
   and trial-result versions together to 2, preserving field sets, IDs, tiers,
   eight classes, raw exits and report arithmetic. V2 check refuses v1 state;
   v2 report refuses v1/mixed rows. Historical replay uses the old source/
   binary. No automatic relabeling, migration, dual scorer or pooled rates.
5. Prove completed external naive commands over real SMTP/MCP/fixture HTTP:
   zero child exits, healthy deliveries, exact trap events and true recovered
   goals. Retained completed cases must agree with actual check CLI. Confirm
   enumeration uses both orders; digest proves forty links with 39 decoys.
   Preserve oracle/null, isolation, reset, deadline and infrastructure controls.
   Baseline red means an actual unwanted pass, not tool/version failure.
6. No root production, manifests/locks, dependency allowlist, coverage or gate
   amendment. Independent evals evidence and every pinned charter verification/
   review obligation remain required. CI-only claims need actual receipts.

## Alternatives and consequences

Adopt both positions' event policy and truthful recovery over lockouts or
fail-on-any-rejection. A genuine mistaken click now loses a targeted trial
despite recovery; explicit instructions make that a declared experiment.

Choose robustness's supersession-only rule over simplicity's expiry trap.
Cost: expiry avoidance is not measured. Choose robustness's explicit v2 over
simplicity's documentation-only revision provenance. Cost: consumer updates
and archived historical replay, in exchange for refusing silent report mixing.

Choose simplicity's exact matching over robustness's broader malformed-event
refusal regime. Cost: a structurally valid edited mismatch may disappear from
trap matching. Accepted unconfined execution does not promise adversarial
artifact integrity. Keep existing validation, producer tests and the
known-Decoy null-flow exception so honest fixture evidence counts.

Choose bounded independent controls over exhaustive permutations, model
sampling or a public naive-agent framework. Costs are a reviewed fixed
scenario match and extra real-process tests; future traps need explicit
policy review.

## Authority and integration

Only the operator accepts this proposal. Compatible index/proof work may
proceed; incompatible scoring/prompts/versions wait. Accepted 0055 stays
unchanged. Original eval documents receive dated active amendments only
when authorized implementation makes v2 current.

The chief's four-file limit excludes the decision index. The next
implementation/integration commit must add its proposed row and pass
inventory/status tests before whole-gate success. That known gap is not a
waiver. This decision records design, not implementation, executed red/green
proof, gate success or feature completion.
