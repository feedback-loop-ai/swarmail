# Codex delivery recipes

Created: 2026-10-23
Reframed: 2026-10-23, run `finish-and-deliver-the-operator--e326f5c1`
Reconciled: 2026-10-23, run `finish-and-deliver-the-operator--64fd181b`
Reconciled: 2026-10-23, run `finish-and-deliver-the-operator--bbd50333`
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
positions for run `bbd50333` and
`.brokkr/state/tasks/finish-and-deliver-the-operator--bbd50333.md`.
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
   faster; paid review, refusal probes and possible delivery failure remain
   necessary. Neither position requests universal extra paid deliveries.
4. **Rule for simplicity on proof breadth.** Reuse valid receipts and the
   exact-core harness after checking provenance. Reject a second full paid
   delivery solely for careful's effort change, and eight epic deliveries as
   bootstrap acceptance. Actual fast 1/2/4 selection and aggregation require
   matched evidence, including existing final-binary fixtures. The cost is explicitly unexercised live behavior for variants
   not dispatched; static assignment is never live proof.
5. **Adopt robustness's fact and candidate boundaries in their existing owners.**
   Patch 0002 now preserves unknown scope/fixes and witnessed true flags;
   patch 0003 binds verification to journal-owned HEAD/tree and rejects
   inconsistent clean results at singleton consumption too. The previous
   source-level blocker is historical, not the present implementation.
   Reconcile matching final-engine evidence before reuse; require remaining
   dispatch proofs. Reject a second evaluator, candidate sidecar or late shell
   repair of admitted facts. The cost is provenance work and another stopped
   bootstrap if a necessary pinned defect is demonstrated.
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
For panels, Opus owns security holds; worst verdict wins. Every review-to-ship
allow in self, codex, careful and fast requires explicit `spec_defect: false`
and `fixes_applied: false`.
Known scope defects from unchanged fast review return to triage on chore,
feature, design and engine; self/codex/careful refuse a known scope defect
without inventing a triage transition. Residual returns also require explicit
non-security, at-most-medium debt.
Security holds and above-medium debt retain stop precedence. A clean result
with known true fixes and explicit false scope can select the
supported reverification rule (full recipes and fast feature/design/engine,
not fast chore), subject to candidate binding. A committed reviewer edit
violates that binding even if the result reports false fixes and Git is clean.
The rule neither grants write authority nor bypasses the measured witness.
Missing or malformed facts must not become safe by aggregation. Unknown scope
is not inferable from a clean tree; a reviewer can commit an edit and leave
that tree clean. Preserve witnessed scope/fixes and bind review to the verified
candidate's HEAD and tree, as well as checking worktree/index cleanliness.

## Fact preservation and candidate identity

Scope and fixes are optional booleans: a witnessed true dominates false and
unknown; otherwise unknown prevents a false aggregate. Missing, null and
wrong-typed fields cannot manufacture permission. Residual security/severity
unknowns remain unknown, witnessed security and high severity survive joins,
and explicit clean results carrying security or above-none debt refuse even
in singleton review. Explicit non-security residuals of valid at-most-medium
severity remain allowed with false scope and false fixes. Zero unsafe admission
must not silently become a new zero-debt contract.

Actual advancing verification records `verified_candidate` with HEAD and tree
in engine-owned journal inputs. Protected review and ship compare against
that witness, refusing missing/spoofed witnesses, committed reviewer drift,
dirty index/worktree, unreadable Git and untracked files, including when
`status.showUntrackedFiles=no`. Restart retains the witness. Preserve the
existing gate HEAD-span refusal as an additional boundary. Neither a seat
receipt, a read-only charter nor a clean status can replace measured identity.
The candidate protection applies to these recipes with a verify phase; exec
fixtures lacking verification do not prove this boundary.

The current evidence is bounded: 124 final-binary CLI member cases and twelve
verify/review candidate controls execute no providers; 102,110 exact-core
policy evaluations prove evaluator behavior, not dispatch. Reuse only after
matching source, binary and relevant inputs. Remaining 1/2/4 selection,
restart/result-association and final-engine ship obligations must be resolved
before delivery, with positive controls and refusals at the owning boundary.
Do not repeat unchanged proofs merely to spend time or call fixtures product
delivery. A demonstrated required pinned repair stops this run for a new pin;
no alternate executable, digest waiver, hidden source edit or wrapper bypass.

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
  script/tooling bytes match this run's pin. Digest-matched `.forge/bin/brokkr`
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
  Fast returns known scope defects on all four strategies; full non-triage
  recipes refuse them. Missing proof or an admitted known defect blocks delivery.
- **AC4 — Safe review.** Execute the recorded core evaluator across valid
  strategies, unknown selectors, all valid severities, true/false and
  absent/null/wrong-typed safety, scope and fixes facts. Record first-match
  rule IDs/destinations, above-medium precedence, explicit-safe debt and final
  unsafe refusal. Match or execute the 1/2/4 clean/residual member matrix:
  absent inputs, missing/null/wrong-typed scope/fixes, true scope/fixes,
  conflicting members and explicit-false controls. No omitted/malformed facts
  can become false, witnessed defects/edits cannot disappear, and one hold dominates
  clean members. Explicit clean security or above-none residuals refuse,
  including singleton results. Incomplete results cannot pass. Require zero
  unsafe residual admission or security-to-scope laundering. A required pinned repair blocks
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
  refusal probes establish that dirty tree/index, changed HEAD/tree, hidden
  untracked files and empty, missing or mismatched ledger cannot reach done,
  alongside an unchanged successful control. The older ship receipt is not
  final-engine evidence. Engine binding and shell close-out have separate
  owners. The repaired script emits shipped only for matching ledger/HEAD
  and a clean tree; exit 2 on a
  discrepancy must remain a real engine refusal. Ready is preliminary, never
  delivery success. Prove final dispatch; labels or notes are insufficient.
  Restart preserves pin, run identity, counters, attempt/result association and the
  journal-owned verified HEAD/tree witness. Missing/spoofed witnesses and
  dirty/untracked/unreadable Git refuse; committed reviewer edits refuse even
  with clean status. Missing/partial/stale/misassociated results never advance.
- **AC8 — Evidence and delivery.** Receipts name candidate, engine/pin,
  run/attempt, command/provider, exit/verdict and result/log location without
  secrets. Validate core-probe provenance and input identities before reuse;
  match the current three-patch build/activation chain, resolved tables and
  evidence hashes, or rerun the existing harness for changed inputs. The
  refreshed policy receipt matches current identity; old counts and digest
  complaints are historical. Preserve old receipts and source baseline reds.
  The patch helper must reject wrong HEAD, dirty tracked tree/index, invalid
  hashes/paths and malformed/duplicate JSON before patch mutation; CI applies
  the declared ordered bytes and runs actual engine regressions before build.
  Its nine real-Git tests prove buffering/order/refusals, not global rollback
  after a later apply failure or cleanliness of untracked source files.
  Commit implementation-owned evidence/task edits before final verify; later
  receipts remain ignored. Missing proof, provider/gate failure, sweep warning,
  dirty shipping tree or required pinned repair is outstanding. Preserve
  stopped-run history and the recovered immutable audit snapshot; never
  resume or mutate that snapshot, mutate remote history, fabricate old rulings
  or supersede any finding without matching evidence and authority. Verified
  export/import already adopted 67 recovered runs and three stopped bootstraps
  (70 before this run); import is not acceptance, resume or discharge.
- **AC9 — Bounded bootstrap and isolation.** No production runtime,
  dependency/allowlist, coverage-denominator or accepted-body changes. Commit
  authorized recipe work unsigned, never push/merge. Document up to eight
  isolated Git stores/branches/worktrees/results/evidence/build/coverage/cache
  overlays, ports/inboxes/data and reserved 16-CPU groups. All future lanes
  share canonical `.forge/forge.db`, visible in ordinary `brokkr tui`; no
  relocation or alias of an open journal/WAL. Reserve overlapping files and
  rerun integration gates. Reserved wave1/A–H launches wait until bootstrap
  actually completes. Their committed recipe baseline stays immutable; later
  gate repairs use separate writable candidate scripts and execute repaired
  proofs. Eight successes require eight actual outcomes, not directories.

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
and ship findings are not superseded by a draft. The next stopped run
`finish-and-deliver-the-operator--64fd181b` committed `cdbd579` and its design
check passed; that run identified the scope/fixes gap in the then-active
one-patch composite. The one-patch binary identity and blocker in that dated
revision describe historical inputs, not the current engine. Their full
bytes and findings remain in Git and the retained run evidence.

## Final-composite clarification — 2026-10-23

This new run `finish-and-deliver-the-operator--bbd50333` uses executable
`.forge/bin/brokkr`, SHA-256
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Source identity is exact base `597f4868e3fb971b62e6a368a2d8aa23271f4531`
PLUS the ordered checksum-pinned 0001 residual-facts, 0002 scope/fixes and
clean-inconsistency, and 0003 verified-candidate patches in
`bundles/self/tooling.json`. Composite diff SHA-256 is
`49611f8d62c495467acdc4cb3f77c1e17bc5148339aaa3d48830e25214f3ac6b`.
Never describe it as a clean-base build or select an archived executable
because version 0.12.0 matches. The normal PATH engine must match too.
The plan records all patch digests and evidence limits. The build receipt's
staged-not-activated wording predates `.forge/epic192/active-engine.json`;
that later record activates the same bytes. Preserve both, without changing
pinned provenance prose during this run.

The recovered `.forge/epic192/history/agents-server-forge.snapshot.db` stays
immutable, as does the remote original. Verified adoption and re-export
retain byte-identical streams/manifests. The 67 recovered runs plus three
stopped bootstraps total 70 before this run; this chief has not recounted the
journal. Counts are context, never proof of discharged findings. The later
canonical-journal clarification in 0057 governs earlier per-lane-journal
wording. Recorded corrected-0051 acceptance and the specific 0047 JSON-RPC ID
amendment in `.forge/epic192/operator-rulings.json` remain later-lane adoption/
implementation, with no publication authority and no bootstrap runtime work.

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
