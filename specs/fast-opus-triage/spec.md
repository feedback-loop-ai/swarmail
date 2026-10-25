# Fast triage on Opus medium

Date: 2026-10-25. Run: `apply-the-operator-directed-fast-ff1692a8`.
Phase: chief design synthesis; implementation and delivery review remain.
Baseline: `d7e615d5e94ac28aa84d5a825c95abf1cd436997`.

## Feature and rationale

Route only `codex-fast` triage to Claude Opus 5.5 through the existing `opus`
key at medium effort, with no fallback. The operator selected the model that
classifies risk before implementation. Preserve the classification rules,
execution bounds and every other route. No model-quality improvement is claimed.

Authority is accepted [0057](../../docs/decisions/0057-codex-delivery-recipes.md)
as narrowly amended by the operator's 2026-10-24 directive, and accepted
[0058](../../docs/decisions/0058-codex-review-routing.md) for reviews. The
operator explicitly requires a dated clarification appended to 0057, retaining
accepted status and historical text. This specific directive supersedes the
generic requirement to draft a newly numbered proposal. No decision is declared
by this spec, no number is allocated, and this seat accepts nothing further.

## Evidence and framing

Both `.brokkr/state/design/positions/simplicity.md` and `robustness.md` were
read. The requested `.brokkr/state/tasks/` directory is absent. The current
framing is the committed [intake](intake.md), matching this run; it takes
precedence over the packet's historical `prior-intake.md` and earlier design.
Do not manufacture task copies or journal history.

The packet README, operator-rulings, prior patch and prior intake verify against
`.forge/epic192/opus-triage/SHA256SUMS`, checked from that directory. The local
`.forge/bin/brokkr` digest is
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, matching
`bundles/self/tooling.json`. Current resolution confirms Sol/medium triage,
three attempts and 3600 seconds. The Claude adapter already supports Opus 5.5
and medium. Main already pins Luna/xhigh implementation; earlier references to
stale Sol/high implementation tests are obsolete. The known cross-attempt
engine defect is outside this amendment. No GitHub, CI, journal or sibling
worktree evidence is needed.

## Panel synthesis and rulings

1. **Adopt simplicity's single owner.** Change the existing office's model and
   complete effort map; retain its name, inheritance and adapter. Reject new
   offices, fallback machinery, retry policies, providers and dependencies.
   Cost: the historical `codex-triage` name remains and an Opus outage can stop
   triage under existing failure behavior. No Sol rescue is authorized.
2. **Adopt robustness's resolved proof.** A source-only two-field assertion
   cannot prove dispatch identity or inherited fallback. Check the actual
   recipe selection, resolved chain, provider, concrete mapping and effective
   effort against independent literal expectations. Retain the existing test
   suites; reject a general matrix framework or unrelated engine work. The
   smallest-edit position loses on proof breadth: extra equality controls and
   full verification cost are accepted. Both positions support this bounded
   reconciliation.
3. **Rule for robustness on current charter accuracy.** Along with the model,
   correct its stale four-reviewer descriptions to the accepted five-role
   design/engine panel. Simplicity's preference to leave this wording alone
   loses because it instructs the new triager incorrectly. Cost: two nearby
   documentation corrections; classification predicates remain unchanged.
   Do not rewrite 0058, unrelated historical text, or the decision index.
4. **Bound preservation proofs to existing routes.** Add fast triage and
   Standard/Careful intake plus absent-triage assertions to the Python matrix;
   retain existing work/reviewer guards and explicitly compare self's existing
   route baseline rather than treating its skipped comparison as proof.
   Reject wholesale replacement with the stale patch's framework. Cost:
   maintaining a few explicit baseline expectations, not new runtime code.
5. **Reject runtime hardening and engine repair.** Robustness's storage,
   webhook, hostile-DATA, chaos, TLS and concurrency concerns justify retaining
   the full-risk classification and existing real-server gates. They create no
   new implementation scope. Keep the cross-attempt fixture and assertion
   exactly as on main. Cost: the full routing proof can remain red at that
   known control; a separate passing route matrix cannot make it green.
6. **Keep evidence categories distinct.** Exact routing, controlled dispatcher
   proofs, house gates and actual five-role reviews establish different facts.
   Neither this design nor a controlled fixture is a provider review. Cost:
   delivery still needs the existing independent reviews and honest gate results.

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


## Dated clarification — current-base reconciliation (2026-10-25)

The preceding dated entry records the previous chief's instruction and remains
historical. This run's explicit instruction is to commit all requested work
before returning. It supersedes that entry's ship-only implementation timing;
implementation must commit its authorized changes before handing off for
verification. The chief commits exactly this directory's spec.md, plan.md and
tasks.md and leaves that commit last. The intake is already tracked here and
stays unchanged. This revision replaces stale-base findings and removes any
requirement to repair the cross-attempt engine defect. It preserves the control
and its honest result. No policy, engine pin or decision number changes follow.

## Required route contract

| Surface | Exact preserved or requested route |
| --- | --- |
| Fast triage | Office `codex-triage`; provider `claude`; chain `[opus]`; concrete `claude-opus-5-5`; medium; no fallback |
| Standard / Careful triage | Absent in both recipes |
| Standard intake / implementation | `codex-intake`: Sol medium / `codex-implementer`: Sol high |
| Careful intake / implementation | `codex-careful-intake`: Sol high / `codex-careful-implementer`: Sol xhigh |
| Fast intake / implementation | Intake absent; Luna xhigh in chore, feature, design and engine |
| Shared design, including self | Sol high simplicity; Astra high robustness and chief; inherited deterministic check |
| Standard review | Fable correctness; official Flash security; Sonnet tests |
| Careful and fast design/engine review | Separate Astra correctness, official Flash security, Sonnet tests, Astra guarantees and independent Fable |
| Fast chore review | One combined official Flash reviewer |
| Fast feature review | Qwen Max correctness, with only existing pre-session Qwen-to-Sonnet fallback; official Flash security |
| Legacy self | GLM intake/implementation; correctness `[qwen-max, glm-flash]`, security `[opus, glm-flash]`, guarantees `[astra, glm-flash]`, tests `[fable, glm-flash]`; shared design and proof ownership unchanged |

Review identities/efforts remain as pinned by accepted 0058: Claude Fable 5.1
and Sonnet 5.5, Codex Astra, DashScope Qwen 3.8 Max and official DeepSeek Flash.
DashScope has no explicit effort; the other reviewer routes retain high.
Self's legacy fallback chains remain; Codex reviewer chains are singleton except
feature correctness. DeepSeek Pro remains forbidden.

## Acceptance Criteria

- **AC1 — Exact fast triage:** source models equal `["opus"]` and the entire
  effort map equals `{"opus":"medium"}`. Real recipe/office resolution equals
  the complete triage row above, including a singleton chain and existing
  concrete Claude mapping. No obsolete Sol effort or alternate survives.
- **AC2 — Route isolation:** all other configuration equals the baseline.
  Tests preserve the work/review rows above, Standard/Careful absent triage,
  fast absent intake, triage inheritance, workspace hands/masks, limits,
  classification vocabulary, no-default selectors and bounded returns.
  Charter edits change model/count wording only, not classification rules.
- **AC3 — Documentation and authority:** append a dated clarification to 0057
  recording the operator directive of 2026-10-24 and explicitly superseding
  only its fast-triage Sol/medium statement. Existing bytes remain an unchanged
  prefix and accepted status remains. Update the triage charter, agents and
  bundles READMEs and house rules consistently. No decision/index allocation,
  renumbering or new acceptance occurs; the packet's number ledger stands.
- **AC4 — Effective-route regression proof:** extend `tests/delivery_recipes.rs`
  and `tests/codex_review_routing.py::resolved_matrix` with exact triage and
  preserved intake/absence checks. Missing triage, wrong provider, wrong alias,
  wrong concrete model, wrong effort and extra fallback are rejected by the
  same comparison used on real resolved data. Preserve existing reviewer and
  Luna guards; no stale expectation is restored.
- **AC5 — Excluded work stays excluded:** engine binary/pin/patch stack,
  adapters, lockfile/dependencies, gates, policies, bundle selectors, 0058,
  decision index and application code are unchanged. The existing Python
  cross-attempt fixture, assertion and engine behavior remain as on main.
  No source-receipt bypass or unrelated CI change is imported from the patch.
- **AC6 — Local evidence is complete and honest:** execute every house gate,
  pinned verifier selftests and fail-closed native-TLS absence proof against
  the committed candidate, with the specified engine. Record commands, exits,
  logs and commit/tree identities; exact coverage remains missed == 0 for
  lines, branches and functions with its selftest. Four compiles and the
  focused recipe and resolved-matrix proofs must pass. Run the full routing
  proof unchanged and record its cross-attempt result separately. A failure
  stays red and bars an all-gates-clean claim; this criterion neither requires
  an engine fix nor waives the delivery policy.
- **AC7 — Committed handoff:** this chief's last commit touches exactly this
  directory's spec.md, plan.md and tasks.md; all three are nonempty, clean,
  committed and pass the pinned design checker. Position files remain ignored.
  Implementation commits before verification, unsigned. No push, merge, tag,
  publication, issue closure or decision acceptance is authorized.

## Delivery ownership and limits

These local criteria do not require live-provider, macOS, namespace, network or
live-GitHub proofs. The delivery engine owns actual accepted-0058 five-role
review on the verified candidate: separate Astra correctness, Flash security,
Sonnet tests, Astra guarantees and independent Fable results. Controlled Python
participants cannot satisfy it. Changed candidate trees require renewed
applicable verification/review. Missing reviews or hard holds prevent delivery.

CI owns the chart contract/history/selftest, Docker smoke, RustSec advisory
audit, tag-only rate proof and privileged port-80 exchange under the house
inventory. The operator owns later sealing/publication. This chief adds no
local acceptance criteria for their external evidence.

Frozen accept == stored, subscribe-before-check, oldest-only pruning and
accepted ingest-time extraction remain unchanged. No production behavior,
benchmark, schema or API change is justified. A `drafted` result records this
committed design; the sequence's final check alone rules design advancement.
