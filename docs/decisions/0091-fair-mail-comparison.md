# 0091 — Fair comparison evidence and target-aware experiment records

- **Status:** accepted (operator, 2026-11-02 — "accept all as recommended",
  given in chat to the dispatcher).
- **Date:** 2026-10-28
- **Issue:** Epic #164 / #163
- **Specification:** [Fair mail comparison](../../specs/fair-mail-comparison/spec.md)
- **Supplements:** [0055](0055-agent-eval-harness.md) and [0063](0063-eval-provenance.md) with explicit comparison mode, adapter contract and evidence admission.
- **Preserves:** 0001–0005, 0050/0072 proof, 0056, 0071 scoring, 0075 privacy, 0076 admission and all gate/dependency policies.
- **Number:** next after this candidate's 0090; provisional, not reserved across parallel work. Only the operator reconciles numbering and acceptance.

## Context

Issue #163 requires genuine baseline/final evidence, a fair same-suite Mailpit
comparison and real throughput/wait benchmarks against Mailpit and MailHog.
Current TargetIdentity includes Swarmail-specific settings and chaos facts.
Reusing those for Mailpit invents equivalence; silently changing native strict
v2 readers undermines evidence. Local infrastructure can proceed while #148/#201,
conflicted eval sources, #151 baseline and final MCP contracts remain unresolved.

This is a new public comparison/adapter contract, not a competitor capability
claim or permission to change native guarantees. Drafting authorizes no paid
trials, publication or closure. Compatible work can proceed under 0045 §8;
contradictions with accepted rulings wait for operator acceptance.

## Proposed ruling

1. Commit the complete versioned experiment before collection. Bind official
   source/artifact/platform/license evidence, every requested cell, model/tool
   declarations, applicability, workload/seed/count/order/limits and statistics.
   Preserve all runs, failures and unstarted work. A changed method starts a
   separate run; no best-rerun selection or denominator shrinkage.
2. Reuse existing fixture, sole v2 scorer, controls, admission, lifecycle and
   report arithmetic. Add concrete Swarmail/Mailpit choice and discriminated
   comparison envelope with truthful target-specific identity/settings and
   common unchanged v2 business meaning. Keep native v2 readers/output and
   baseline behavior unchanged. Native readers reject comparison envelopes;
   comparison readers use strict common validation and the same scorer. No
   fabricated Swarmail metadata, legacy migration, v1 reader, ninth failure
   class or alternate harness.
3. Thin Mailpit MCP exposes only source-verified general upstream operations.
   Preserve body/filter/pagination/errors, bound requests and include overhead.
   Justify wait facilities before collection. No task answers, extraction/cache/
   index, synthetic isolation/chaos, fixture admin or hints. Verify capabilities
   through real owned protocols. Unsupported tasks remain full-product entries
   without fabricated trial results; report common subset separately. Unsupported
   full-suite semantics can leave the requested criterion incomplete.
4. One independently buildable benches driver uses three actual server binaries.
   Record SMTP accepts/rejects/ambiguity, visible identities, elapsed units and
   scheduling; separate caps/storage/resources/profiles. Separate native/adapter
   waits, preserve signed ack-relative offsets, samples and censored timeouts.
   No store-only/network comparison, ambiguous retry or hidden failed sample.
5. One wrapper exposes local, free-controls and evidence modes with pinned
   installed inputs and explicit missing-prerequisite failure. It starts no paid
   work in delivery modes. An explicit operator-only measure mode delegates the
   full final product to existing eval owners only after separate authorization
   and prerequisite/control admission; delivery proves it with free inert commands
   and never executes paid selections. Full evidence requires genuine authorized
   runs and all prerequisite
   receipts; local success is not story completion. #148/#201 shared-host proof,
   actual #151 baseline and contract reconciliation remain gates.
6. Preserve privacy, one cleanup owner and sequential admission through
   finalization. Raw data is safe typed measurement/allowed synthetic fixture
   state, not logs/transcripts/credentials. Exclusive private outputs retain
   incomplete prefixes; only validated post-cleanup data gets a final index.
   No resume/overwrite/invented suffix. Forced death requires confirmed cleanup
   before evidence resumes; hashes are not hostile-host authentication.
7. docs/evals ultimately holds genuine baseline/final evidence, method, actual
   costs, safe raw data and all losses. Numeric README agent claims cite exact
   denominators/configurations. Unknown cost is unavailable, not zero or estimated
   actual spend. Missing full evidence remains incomplete.
8. Carry inherited HIGH review-integrity findings and separately tracked #219
   into protected review without reproduction, invented IDs, expanded access or
   closure claims. Preserve provisional 0090, five required independent results
   and security hold. Merged source is not a published pinned engine or proof.

## Synthesis and costs

Adopt simplicity's existing eval owners, one independent wire driver and thin
command. Reject generic platforms, new crates, dashboards, duplicate scorers,
provider systems and MailHog agent eval. Cost: fixed targets and separate
benchmark/eval build boundaries.

Adopt robustness's full accounting, bounded ownership, typed manifests and
honest ambiguity/censoring over subset-only or exit-code-only reporting.
Cost: more validators, records, negative controls and visible incomplete runs.
The explicit comparison envelope costs another strict decoder while preserving
native v2 and truthful target identity.

Preserve robustness's failure concerns through existing regressions and changed
boundary proofs. Reject a durable outbox, storage rewrite, confinement system
or new cross-product fault research suite. No new crash/TLS/webhook-equivalence
claim follows; optional future profiles need their own frozen method.

Both positions reject fabricated paid evidence and infrastructure-as-completion.
Accept slower sequential runs, less raw diagnostic detail and blocked final
claims. Only genuine evidence, all gates and protected review satisfy the story.

## Proof and integration

AC1–AC16 and ordered paired tasks in the linked design are normative. Real
three-target/adapter tests, external controls, schema/replay/privacy/cleanup
refusals, independent arithmetic and evidence admission complement the complete
house inventory and standalone eval suite.

The chief commits exactly this proposed record and spec/plan/tasks.
Implementation adds its single proposed decision-index row and validates
status/link inventory; a temporary missing-index gap is explicit, not waived.
No accepted decision or historical spec is rewritten. No paid trial, push,
merge, publication, tag, issue closure, decision acceptance or engine re-pin.

## Dated addendum — 2026-11-02 (at acceptance)

The operator accepted this record on 2026-11-02. The text above is unchanged
and remains the record of what was proposed. This addendum states what has
moved since it was written, checked against `main` at `c336c95`; it rules
nothing beyond the acceptance.

- **The tooling shipped; the evidence is owed.** `44175fe` (#287) landed the
  wire driver (`benches/mail_compare.rs`), the Mailpit adapter
  (`benches/mail_compare/mailpit_mcp.rs`), `scripts/compare-mail.sh` and the
  comparison row type (`evals/src/identity.rs`).
  [`docs/evals/README.md`](../evals/README.md) records the controls (T9.a)
  as unstarted and no comparison evidence, final contract, final trial or
  cost record; its rows are open, not waived. No paid trial is authorized.
  Acceptance makes no comparison claim.
- **The #151 baseline exists.** It was published on 2026-11-01 (`f815d48`,
  [baseline](../evals/baseline-2026-11-01.md)), while `docs/evals/README.md`
  still says, under its 2026-10-30 status, that no baseline exists.

## Dated pointer — 2026-11-07 (eight targets, proposed 0130)

The text above is unchanged. Issue #340 states that on 2026-11-07 the operator
asked to bench throughput and wait latency against MailSlurper, smtp4dev,
Inbucket and GreenMail as well as Mailpit and MailHog, and records the ruling
as "#163 approved and widened". That wording is the issue's paraphrase of the
ruling, not a quotation of it. Proposed
[0130](0130-eight-target-mail-comparison.md) would widen §4's "three actual
server binaries" and its "fixed targets" to eight, by addition only: protocol
v2 and its 2026-10-30 collection stay as recorded, and protocol v3 is a
separate run with a separate denominator. This pointer changes nothing in
§§1–6; 0130 is a proposal, and only the operator accepts it.

2026-11-07: nine targets; see 0130's review-return addendum.

2026-11-08: the operator accepted [0130](0130-eight-target-mail-comparison.md) ("Accept and merge"); §4's target count is amended by addition as 0130 states.

## Dated addendum — 2026-11-07 (the operator's widening, issue #339)

This addendum only adds. The text above, and the 2026-11-02 addendum, are
unchanged and remain the record of what was proposed and accepted. It
records the operator's ruling of 2026-11-07 on #163, quoted in issue #339:
"compare with all that we have shims for". Authority is that ruling, not
this note.

- **Four targets, four agents, no GLM.** The agent-suite comparison covers
  Swarmail, Mailpit, MailHog and MailSlurper, on claude-opus, claude-sonnet,
  codex-sol and codex-luna. There is no GLM: spark is down and the
  DashScope GLM quota is out until 2026-11-22.
- **The "MailHog agent eval" rejection does not apply to this story.** The
  synthesis above rejects a "MailHog agent eval". For the agent-suite
  comparison of #339 and #163 the operator's ruling lifts that rejection,
  and `evals/src/identity.rs`'s refusal of MailHog agent rows follows it.
  Nothing else in the synthesis is lifted: one fixture and one scorer (§2),
  a thin adapter that emulates no Swarmail-only capability (§3), controls
  before any paid session (§5) and an honest cost basis (§7) all stand.
- **MailSlurper joins unchanged under 0092 §3.** It is not named in this
  record; it enters through 0092 §3's adapter contract with no new rule.
- **Contracts.** The contracts that implement the ruling (a runner bridge
  for adapters, batches, a source-lock v4, a frozen product record and the
  wrapper's new scopes) are in proposed
  [0129](0129-agent-suite-comparison.md). The ruling is recorded here; 0129
  records only what implements it. No paid session is authorized, and the
  planned 240 sessions are confirmed by the operator separately.
- **MailSlurper's official evidence (§1).** For MailSlurper in this story, §1's
  official evidence is met by the official source plus the labelled
  source-built artifact, under the operator's 2026-11-07 ruling recorded in
  [0129](0129-agent-suite-comparison.md)'s second addendum: the published
  linux executable does not start, and the comparison's MailSlurper is a cgo
  build of the pinned release source, labelled "built from source".

2026-11-08: accepted [0130](0130-eight-target-mail-comparison.md) pinned the source-built MailSlurper as its own lock row, `mailslurper-source-build`, in its source lock v4; the "source-lock v4" and the "labelled source-built artifact" in the 2026-11-07 addendum above are read as that row, and proposed [0129](0129-agent-suite-comparison.md)'s 2026-11-08 addendum records the integration. Nothing above changes.
