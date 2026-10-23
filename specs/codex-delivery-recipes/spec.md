# Codex delivery recipes

Created: 2026-10-23
Reframed: 2026-10-23, run `finish-and-deliver-the-operator--e326f5c1`
Reconciled: 2026-10-23, run `finish-and-deliver-the-operator--64fd181b`
Branch: `epic192-codex-recipes`
Status: design draft; implementation and delivery proof remain outstanding

## Feature and rationale

Finish the operator's existing `codex`, `codex-careful` and `codex-fast`
compositions before epic #192 repairs. Most authoring uses Codex. Triage in
`codex-fast` lets a proved, understood bug proceed directly to implementation,
full shell verification and one or two independent reviewers. Sensitive work
retains the council and four reviewers. This bootstrap is `engine` work:
changing admission machinery requires that full path regardless of diff size.

The authority is accepted
[0057](../../docs/decisions/0057-codex-delivery-recipes.md), including triage
and bounded escalation. This design implements that directive; it adds no
semantic amendment and declares no new decision. Implementation must commit
the already-authorized 0057 file and dated markers. Chief commits only this
spec, [plan](plan.md) and [tasks](tasks.md). Preserve accepted
[0007](../../docs/decisions/0007-hermetic-gates-single-model.md),
[0009](../../docs/decisions/0009-agent-council-hybrid.md),
[0013](../../docs/decisions/0013-four-model-review-panel.md),
[0017](../../docs/decisions/0017-codex-review-seat-delivery-sandbox.md),
[0050](../../docs/decisions/0050-delivery-gates.md) and
[0056](../../docs/decisions/0056-signing-policy.md), as amended by 0057.
No accepted historical body or other proposal's status changes here.

## Council synthesis

Inputs are both new `.brokkr/state/design/positions/{simplicity,robustness}.md`
positions for run `64fd181b` and
`.brokkr/state/tasks/finish-and-deliver-the-operator--64fd181b.md`.
These ignored files are run-local evidence; the following rulings are the
committed synthesis, not an average of the positions.

1. **Adopt simplicity's bounded bootstrap and inheritance.** Keep
   `codex -> self`, `codex-careful -> codex`, and `codex-fast -> codex`.
   Fast is the explicit routing overlay; careful changes authoring effort.
   Reject another scheduler, registry, evidence database, policy interpreter
   or copied proof chain. Adopt robustness's runtime examples as scope
   boundaries, not production work. The cost is unresolved epic residuals.
2. **Adopt direct bug lanes; reject a council for every known bug.** Both
   positions support the operator's 1/2/4 rule. Classification must establish
   scope through evidence, not patch size or severity. Rejecting universal
   full review costs reliance on triage judgment, bounded escalation and
   independent scope checking. Unknown work stops.
3. **Rule for robustness on proof depth.** Compiler sweeps, JSON assertions
   and core evaluation cannot prove actual selection, aggregation, pin origin,
   result consumption or clean close-out. Require execution at each distinct
   boundary. Reject a configuration-only finish even if compile is green.
   Both positions accept this obligation. The rejected shortcut would be
   faster: paid review,
   refusal probes and possible delivery failure remain necessary.
4. **Rule for simplicity on proof breadth.** Reuse valid receipts and the
   exact-core harness after checking provenance. Reject a second full paid
   delivery solely for careful's effort change, and eight epic deliveries as
   bootstrap acceptance. Actual fast 1/2/4 selection and aggregation still
   need proof. The cost is explicitly unexercised live behavior for variants
   not dispatched; static assignment is never live proof.
5. **Adopt both positions' blocking aggregation finding.** The repaired fast
   table returns known scope defects on all four strategies and requires
   explicit false scope/fixes for shipping. The ship script now refuses
   close-out discrepancies with exit 2. Those repairs do not prove member
   validation: the declared aggregate still defaults missing scope to false
   and writes false fixes unconditionally. Require exact-engine member-level
   refusal evidence before delivery. If no earlier enforcement exists, stop
   for a new pin that repairs validation/aggregation; do not edit the pin or
   weaken the requirement. The cost is another run and delayed epic repairs.
6. **Adopt independent review and explicit safe admission.** No fallback,
   DeepSeek Pro, scope laundering of security debt or missing-fact inference.
   Allow inherited tracked debt only with explicit non-security and valid
   at-most-medium severity. Reject conflating zero unsafe admission with zero
   allowed debt. Provider availability loses to independent judgment.
7. **Adopt frozen inputs and honest evidence ownership.** Review and later
   commit all authorized pending changes, including fast. Finish only
   non-pinned tests/docs. Report required pinned repairs for preparation of
   a new run. Preserve journals and legacy evidence mirrors; neither mirrors
   nor baseline logs prove discovery/delivery. A new run and slower epic
   progress are accepted costs.

## Routing and scope requirements

| Strategy | Required path and review |
|---|---|
| `chore`: isolated understood non-sensitive bug | Framing → direct implementation → all inherited shell gates → one official Flash reviewer covering correctness/tests/guarantees/security → inherited ship |
| `feature`: broader understood non-sensitive fix under an existing contract | Framing → direct implementation → all gates → Qwen correctness and Opus security → inherited ship |
| `design` / `engine`: architectural or sensitive work | Framing → full positions/chief/deterministic check → implementation → all gates → four independent lenses → inherited ship |
| `escalate`, unknown/missing/invalid classification or selector | Stop; no default cheap route |

Direct lanes own requirements in triage framing: reproduction or equivalent
concrete evidence, affected files/callers/contracts, falsifiable criteria,
classification rationale and a targeted regression. They require no council
or miniature spec package. `feature` is an engine key for an understood fix,
not permission for a new product feature. Security, concurrency, persistence,
shutdown, resource bounds, protocol/validation/extraction contracts, gates,
policy and delivery machinery require the full lane even for a two-line fix.

Implementers return `oversized` before implementing newly sensitive scope.
Current policy returns once at triage visit one; another oversized result at
visit two stops. A review return may reach triage visit three only to stop
before another implementation. No restart resets the budget, no downgrade
retries an inadequate lane, and reclassification absorbs the new evidence.
Monotonic scope is a charter obligation, not proved by counters alone.

The single Flash reviewer owns security holds as well as scope checking.
For panels, Opus owns security holds; worst verdict wins. Every fast-lane
ship allow requires explicit `spec_defect: false` and `fixes_applied: false`.
Known scope defects from unchanged review return to triage on chore, feature,
design and engine;
residual returns also require explicit non-security, at-most-medium debt.
Security holds and above-medium debt retain stop precedence. A clean panel
result with actual fixes and explicit false scope takes the supported
reverification path; inconsistent scope/fixes refuses. Neither ships directly
nor grants reviewers write authority.
Missing or malformed facts must not become safe by aggregation. Unknown scope
is not inferable from a clean tree; a reviewer can commit an edit and leave
that tree clean. Preserve witnessed scope/fixes and bind review to the verified
candidate's HEAD and tree, as well as checking worktree/index cleanliness.

## Blocking member-fact proof

The declared composite's `crates/brokkr-runtime/src/bundle/aggregate.rs`
sets `fixes_applied` to false unconditionally and joins `spec_defect` through
`as_bool().unwrap_or(false)`. Both panel positions and chief inspected this
source. The residual safety patch does not establish mandatory earlier
validation of those fields. This is a source-level gap, not a claimed fresh
end-to-end bypass reproduction.

Before implementation is certified, probe the exact active executable through
member consumption and aggregation on singleton, paired and four-member
paths. For clean and residual results, exercise absent inputs; missing, null
and wrong-typed scope/fixes; true scope; true fixes; contradictory members;
and valid explicit-false controls. A clean tree alone cannot supply a missing
scope judgment. An authoritative measured mutation fact must preserve any
witnessed edit and its candidate identity; a read-only charter is no substitute.
Unknowns must stop, a known scope defect must not ship, and actual fixes must
not be erased. Preserve hold/above-medium dominance and explicit-safe debt.

An earlier mandatory refusal may discharge this obligation with executed
receipts. Otherwise report the precise source/patch/tooling repair and stop
for an assistant-prepared new pin. No in-run pinned edit, wrapper, alternate
engine, default inference or waived criterion is permitted. Later verification,
protected review and ship remain outstanding while this boundary is unproved.

## Model and execution requirements

| Role | Required assignment |
|---|---|
| Ordinary intake / implementation | Codex Sol medium / high |
| Careful intake / implementation | Codex Sol high / xhigh |
| Fast triage / implementation | Codex Sol medium / high |
| Shared simplicity | Codex Sol high |
| Shared robustness / chief | Codex Astra high |
| Correctness | DashScope `qwen3.8-max`, no explicit effort argument |
| Security | Claude Opus high |
| Guarantees / single bug reviewer | Official-platform `deepseek-flash` high |
| Tests | DashScope `glm-5.3`, no explicit effort argument |
| Design check / verify / ship | Complete inherited offline exec seats from `self` |

Sol is `gpt-6.1-sol`, Astra is `gpt-6-astra`, Opus is
`claude-opus-5-5`. New review chains are singleton, with no author fallback
and never DeepSeek Pro. Preserve historical self GLM intake/implementation
and Qwen/Opus/Astra/Fable review chains, except accepted shared-design and
DashScope effort amendments. Keep workspace hands, masked Cargo credentials,
read-only toolchain and offline verify/ship. Dsh cannot express native tool
restriction; read-only conduct and checked tree identity are obligations,
not confinement claims. Record credential names only.

## Acceptance Criteria

- **AC1 — Current design.** Chief HEAD commits exactly the three nonempty,
  clean files in this directory, reconciling both current positions. The real
  pinned deterministic check validates HEAD and dialect prerequisites. Chief
  returns `drafted`; only the check rules design. No operator-local home
  paths, old-run scope exclusions or fabricated acceptance remain.
- **AC2 — Frozen composition.** All current authorized bootstrap bytes are
  reviewed and later committed unsigned. Bundle/agent/adapter/charter/policy/
  tooling bytes match this run's pin. Digest-matched `.forge/bin/brokkr`
  actually compiles self, codex, careful and fast with zero unruled valuations
  and no sweep warnings. Named discovery agrees with explicit composition;
  complete design/verify/ship origin remains self. A needed pin repair stops
  delivery for a new run, never an alternate engine or digest exception.
- **AC3 — Triage and scope.** Tests and real engine evidence establish the
  routing table, no unknown default, direct lanes without spec prerequisites,
  and full council for this engine change. Prove return-budget persistence,
  new strategy selection, first-return/repeated-oversized/third-visit stops,
  and refusal of unsafe scope growth. Demonstrate a supported non-shipping
  disposition for scope defects on every strategy and its aggregation.
  Missing proof or an admitted known defect blocks delivery.
- **AC4 — Safe review.** Execute the recorded core evaluator across valid
  strategies, unknown selectors, all valid severities, true/false and
  absent/null/wrong-typed safety, scope and fixes facts. Record first-match
  rule IDs/destinations, above-medium precedence, explicit-safe debt and final
  unsafe refusal. Separately execute the member-fact matrix above on 1/2/4
  paths, including valid controls: no omitted/malformed scope/fixes can become
  false, witnessed defects/edits cannot disappear, and one hold dominates
  clean members. Incomplete results cannot pass. Require zero unsafe residual
  admission or security-to-scope laundering. A required pinned repair blocks
  delivery until a new pin proves it; predicate tests alone cannot close AC4.
- **AC5 — Routes and results.** Tests pin models/efforts/hands and exact
  1/2/4 reviewer cardinality. Real engine evidence proves distinct selector/
  dispatch behaviors claimed. This bootstrap receives four live,
  engine-consumed authorized provider results with stable candidate bytes.
  Provider failure, missing result, fallback or Pro prevents completion.
  Do not claim live singleton/paired delivery or careful effort execution
  without receipts; selection probes differ from full live deliveries.
- **AC6 — Gates.** Every local
  [house gate](../../docs/house-rules.md#gates--part-of-every-change) passes
  on the final candidate with locked inputs and exact zero-missed lines,
  branches and functions. Actual inherited offline verify executes transitive
  pinned proofs, shim checks, refusal selftests, design/tamper proof,
  publish/seal proofs, licenses and fail-closed native-tls absence. Compile
  all four separately: inherited verify compiles only self. CI-only proofs
  retain their owner. Baseline logs are not final proof; manual commands
  cannot substitute for hermetic dispatch.
- **AC7 — Ship and restart.** Actual ship renders a nonempty ledger from
  this run's real journal at `.forge/ledger/<run>.md`, with candidate/head
  agreement, clean index/worktree and real engine close-out. Recorded-engine
  refusal probes establish that dirty tree/index, changed HEAD and empty,
  missing or mismatched ledger cannot reach done. The repaired script emits
  shipped only for matching ledger/HEAD and a clean tree; exit 2 on a
  discrepancy must remain a real engine refusal. Ready is preliminary, never
  delivery success. Prove final dispatch; labels or notes are insufficient. Restart
  preserves pin, run identity, counters and attempt/result association;
  missing/partial results are incomplete, never implicit success.
- **AC8 — Evidence and delivery.** Receipts name candidate, engine/pin,
  run/attempt, command/provider, exit/verdict and result/log location without
  secrets. Validate core-probe provenance and input identities before reuse;
  explicitly reconcile its stale tooling-record digest with the active
  base-plus-patch identity, or rerun the existing harness. Preserve old receipts.
  The patch helper must reject wrong HEAD, dirty tracked tree/index, invalid
  hashes/paths and malformed/duplicate JSON before patch mutation; CI applies
  the declared ordered bytes and runs actual engine regressions before build.
  Its nine real-Git tests prove buffering/order/refusals, not global rollback
  after a later apply failure or cleanliness of untracked source files.
  Commit implementation-owned evidence/task edits before final verify; later
  receipts remain ignored. Missing proof, provider/gate failure, sweep warning,
  dirty shipping tree or required pinned repair is outstanding. Preserve
  stopped-run history and the recovered immutable audit snapshot; never
  run/resume/operator/import that snapshot, mutate remote history, fabricate
  old rulings or prematurely supersede any finding.
- **AC9 — Bounded bootstrap and isolation.** No production runtime,
  dependency/allowlist, coverage-denominator or accepted-body changes. Commit
  authorized recipe work unsigned, never push/merge. Document up to eight
  isolated branches/worktrees/journals/evidence/writable targets/cache overlays,
  ports/inboxes/data files, overlap reservations and integration gates. Reserved
  wave1 and A–H batches wait until recipes are reviewable. Chief launches none;
  eight concurrent successes require eight receipts, not directories.

## Historical clarification — 2026-10-23

Stopped run `deliver-the-operator-requested-c-f2f37969` and unsigned chief
commit `7b47790` remain history. Its two-recipe framing and contemporaneous
fast-lane drift observations described that run; they are not current scope
restrictions. Evidence is archived under `.forge/epic192/bootstrap-v1`.
This new intake authorizes existing fast changes and freezes prepared bytes.
Old pre-fast logs under `.forge/epic192/bootstrap-evidence` remain baseline
only. No prior live-delivery or final-gate claim transfers to this revision.

## Repaired-source clarification — 2026-10-23

Stopped run `finish-and-deliver-the-operator--e326f5c1` and approved unsigned
spec commit `8cb8bb9` also remain history. Its actual fail-open aggregation
and ship findings are not superseded by this draft. This revision replaces
obsolete chore-only scope and discrepancy-as-shipped descriptions with the
prepared repairs, while retaining the new member-fact obligation above.

The active binary is `.forge/bin/brokkr`, SHA-256
`517c22680beaaf354f6ee3d01e4b964669ecd030a3ea4d9d4554d88e50c83bf0`.
Its identity is source base `597f4868e3fb971b62e6a368a2d8aa23271f4531`
PLUS ordered patch `bundles/self/tooling-patches/0001-preserve-review-residual-facts.patch`,
SHA-256 `5d4da2217135c5a2ca14354ae9907623cf67ff0076b078277cb77a09ad3020b5`.
Never certify the base alone as containing the repair or select the archived
base executable. Detailed proof ownership and receipt limits are in the plan.

The recovered `.forge/epic192/history/agents-server-forge.snapshot.db` is
immutable audit evidence, with snapshot receipt and `ruling-index.json`
reporting 67 runs, 24 rulings and 96 member notes. Recovery corrects the old
missing-journal limitation; it does not accept, rerun or supersede findings.

## Preserved behavior and exclusions

Preserve synchronous accept == stored, subscribe-before-check, oldest-only
pruning and ingest-time extraction. Existing real-protocol suites remain
mandatory; this bootstrap adds no unrelated runtime test campaign.
Persistent acceptance follows commit; a lost reply may cause retry ambiguity.
Memory-only storage cannot survive process loss. Webhooks remain best effort,
lawful pruning can remove older accepted mail, and per-target or per-read
limits are not global process bounds. Chaos refusals remain before acceptance;
TLS errors never authorize insecure retry. These limits neither weaken the
frozen guarantees nor authorize epic repairs in this bootstrap.
