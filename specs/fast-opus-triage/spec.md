# Fast triage on Opus medium

Date: 2026-10-24. Run: `apply-the-operator-directed-tria-fcd757d9`.
Phase: chief design synthesis; implementation and delivery proofs are outstanding.

## Feature and rationale

Change only the `codex-fast` triage model route to Claude Opus 5.5, using the
existing `opus` key at medium effort with no fallback. The operator selected
this model for the risk classification that determines which implementation
and review lane a task enters. Preserve every existing classification rule,
execution bound and downstream route. This is a normative recipe amendment,
so it receives the full design and five-role review path despite its small diff.

Authority is accepted [0057](../../docs/decisions/0057-codex-delivery-recipes.md)
as narrowly amended by the current operator directive, and accepted
[0058](../../docs/decisions/0058-codex-review-routing.md) for reviews. The directive
requires an appended dated clarification to 0057, not a new numbered proposal.
It expressly supersedes the earlier “Triage stays on Sol / medium” sentence;
0057 remains accepted and its historical text remains intact. Neither this spec
nor the clarification grants authority to accept unrelated or future decisions.

## Evidence and synthesis rulings

Read both run-local positions, `.brokkr/state/design/positions/simplicity.md`
and `robustness.md`. The requested `.brokkr/state/tasks/` directory is absent;
the matching intake is `specs/codex-fast-opus-triage/intake.md`. Preserve that
untracked evidence in place; do not manufacture a legacy task or journal entry.
Baseline HEAD at inspection is `352b1cb`. The design uses a separate slug so
its three artifacts do not require committing or hiding that intake evidence.

1. **Adopt simplicity's single-owner change.** Update the existing triage
   office, retain its name and inheritance, and reuse the Claude adapter.
   Reject new offices, aliases, route registries, fallback machinery, engine
   changes and dependencies. Cost: the `codex-triage` name remains historical,
   and Opus unavailability can stop a run instead of selecting another model.
2. **Adopt robustness's exact preservation proof.** Resolved routes, concrete
   model identities, efforts and entire role sets must match explicit expected
   values. A two-field edit with string-presence checks is insufficient.
   Simplicity also supports these bounded tests; the real tension is proof
   breadth, not the requested route. Rule for full preservation assertions in
   the existing tests. Cost to the smallest-edit position: more test changes
   and full verification latency, without a new validation subsystem.
3. **Reconcile documentation scope.** Correct the triage charter's model and
   its stale four-reviewer claims, current recipe/agent summaries, and nearby
   claims that 0058 is proposed. Keep classification predicates and all
   historical rulings. Reject rewriting unrelated 0058 history or cleaning
   the duplicated index row. Cost: historical superseded prose remains and
   readers must follow dated clarifications; current instructions stay truthful.
4. **Bound robustness's runtime concerns to preservation.** Synchronous
   acceptance, subscribe-before-check, oldest-only pruning and ingest-time
   extraction remain mandatory. Existing protocol, persistence, webhook,
   chaos and TLS proofs remain required. Reject new SMTP, store, webhook or
   certificate work in this routing amendment. Cost: any reproduced existing
   failure blocks delivery and needs separately authorized repair; it cannot
   be hidden or treated as fixed by this configuration change.
5. **Keep proof categories separate.** Static and engine-controlled tests
   establish routing and refusal behavior; only five actual provider reviews
   establish the requested review evidence. Neither these positions nor
   compilation substitutes for those calls. Cost: provider availability and
   five separate results remain delivery prerequisites, including two Astra
   invocations rather than one result reused for two roles.

## Dated clarification — chief commit authority (2026-10-24)

Both positions flag the inherited ship-only instruction against the chief's
committed-HEAD requirement. For this retry, the direct chief-phase instruction
explicitly requires staging and committing exactly the design artifacts and
leaving that commit last. Apply that specific authorization to the three-file
design commit; retain ship-only timing for implementation. Reject the prior
draft's instruction to stop with uncommitted artifacts: it reproduces the
reported HEAD failure and cannot satisfy this phase's requested output. Cost:
there is one design-only author commit before ship. This is a scope ruling for
this requested chief action, not an amendment of repository-wide delivery policy
or permission to claim gates/review complete. No validator or journal changes.

## Required route contract

| Surface | Exact requirement |
| --- | --- |
| Fast triage | `codex-triage`; Claude; singleton `[opus]`; `claude-opus-5-5`; medium |
| Fast implementation | Codex `[luna]`, xhigh for chore, feature, design and engine |
| Standard intake / implementation | Codex `[sol]`, medium / high |
| Careful intake / implementation | Codex `[sol]`, high / xhigh |
| Shared design, including self | Sol high simplicity; Astra high robustness; Astra high chief; inherited deterministic check |
| Standard review | Fable correctness; official Flash security; Sonnet tests |
| Careful and fast design/engine review | Separate Astra correctness, official Flash security, Sonnet tests, Astra guarantees and independent Fable |
| Fast chore review | One combined official Flash reviewer |
| Fast feature review | Qwen Max correctness, with only authorized pre-session Qwen-to-Sonnet fallback; official Flash security |
| Legacy self | GLM intake/implementation; existing correctness `[qwen-max, glm-flash]`, security `[opus, glm-flash]`, guarantees `[astra, glm-flash]`, tests `[fable, glm-flash]`; unchanged shared design and proof ownership |

Review model identities remain Claude `claude-fable-5-1` and
`claude-sonnet-5-5`, Codex `gpt-6-astra`, DashScope `dashscope/qwen3.8-max`,
and `deepseek-flash` through `deepseek-official`. Codex reviewer chains are
singleton except the feature-correctness exception. Preserve existing legacy
self chains; do not apply the Codex singleton rule to them. Reviewers retain
high effort where specified and no explicit effort on the DashScope route.
DeepSeek Pro is forbidden in selected, inherited and fallback chains.

## Acceptance Criteria

- **AC1 — Exact triage route:** resolved fast triage equals the full tuple in
  the table. The office has exactly `models: ["opus"]` and
  `efforts: {"opus": "medium"}`, with no obsolete Sol effort entry. The
  existing adapter supplies the concrete identity; no adapter changes occur.
- **AC2 — Refusal and scope unchanged:** unavailable/unsupported dispatch,
  timeout, unusable classification, absent strategy or unknown strategy cannot
  authorize another work lane or model. Preserve existing attempts, timeout,
  bounded returns, five result values, sensitive-task classification, hands,
  credential masks, charter ownership and no-default selectors. No retry
  counter is reset. Model/count corrections change no classification predicate.
- **AC3 — Accepted amendment:** append a 2026-10-24 clarification to 0057
  citing this directive/run, replacing only the earlier fast-triage ruling
  with Opus 5.5/medium and no fallback. Preserve its historical body,
  accepted status, Luna/xhigh ruling and 0058 authority. Current charter,
  `bundles/README.md` and `agents/README.md` agree with this route and five-role
  full-risk review. Historical specs need no rewriting.
- **AC4 — Numbering untouched:** no new decision/proposal or index edit;
  no renumbering or duplicate records. Preserve reservations: #180 owns 0062;
  #200 0063; #174 0064; #202 0065; #195 0069; #181 0070; #198 0071.
- **AC5 — Work routes preserved:** exact tests compare all work rows above,
  including every fast strategy, Standard/Careful intake and implementation,
  shared design, and legacy self. Inherited design/verify/ship proof origin,
  selectors, policies, engine identity and sandbox constraints remain unchanged.
- **AC6 — Review contract preserved:** exact matrices equal the review rows
  above, including office identities, concrete provider destinations and
  efforts. Preserve distinct role results, worst-verdict aggregation, protected
  review, security's hard hold, unknown-fact refusal and candidate binding.
  A valid verdict, residual or malformed facts never authorizes fallback.
- **AC7 — Tests can reject drift:** extend `tests/delivery_recipes.rs` and
  `tests/codex_review_routing.py`; replace stale fast Sol/high assumptions with
  Luna/xhigh and remove the Python proof's skipped self comparison. Wrong triage
  provider, model, effort and extra fallback fail the same equality predicate
  used for the real tuple. Missing/extra/wrong reviewers, forbidden fallback,
  Pro, malformed facts and foreign-attempt results retain refusal controls.
- **AC8 — Complete verification:** all gates in the single house-rule inventory,
  verifier selftests and native-TLS absence proof pass on the candidate, with
  exact 100% line/branch/function coverage. Four recipe compiles are warning-free;
  resolved route and existing engine-backed boundary proofs pass. Save commands,
  outcomes, candidate commit/tree and recorded engine digest. No skipped,
  unavailable or failing proof counts as passed; CI-only proofs retain CI ownership.
- **AC9 — Actual review:** after verification, obtain five separate actual
  provider invocations and results on that same candidate: Astra correctness,
  official Flash security, Sonnet tests, Astra guarantees and independent Fable.
  Record run/attempt/member identity and aggregate verdict. Missing roles,
  provider failures or a security hold prevent shipping. Candidate changes
  require renewed applicable verification and review.
- **AC10 — Delivery boundary:** no push, merge, tag, publication or issue
  closure. The chief makes the specifically requested unsigned design commit,
  touching exactly this directory's spec.md, plan.md and tasks.md, and leaves
  it at HEAD for the final design check. Implementation commits remain restricted
  to ship. Position files and intake evidence are not staged. The design commit
  is not evidence that implementation, full gates or actual review succeeded.

## Non-goals and completion limits

No application behavior, dependency/lockfile/license policy, adapter, engine
pin, bundle selector, gate script, review charter or decision-index change is
required. Do not restore an earlier self design route. The no-fallback choice
accepts reduced availability; no throughput or provider-quality improvement is
claimed. Intake's historical coverage and foreign-attempt failures are leads
for reproduction, not newly proven failures or discharged obligations.

A chief `drafted` result records committed design artifacts. The final pinned
check independently owns design advancement; full gates and live review remain
downstream obligations on the implementation candidate. Neither design text nor
its structural check certifies feature delivery.
