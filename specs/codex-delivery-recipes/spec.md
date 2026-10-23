# Codex delivery recipes

Created: 2026-10-23
Branch: `epic192-codex-recipes`
Status: design draft; delivery evidence remains outstanding
Scope: operator-authorized recipe bootstrap before epic #192

## Feature and rationale

Deliver the already-authored `codex` and `codex-careful` recipes as tested,
reviewable, unsigned commits. Codex authors the work; four independent model
routes review it after the existing hermetic verifier. The operator can
choose ordinary or higher-effort implementation without replacing the realm's
policy, proof ownership, or mail guarantees. This bootstrap enables later
epic work; it resolves no epic runtime finding.

The routing authority for this framed two-recipe bootstrap is already accepted
[0057](../../docs/decisions/0057-codex-delivery-recipes.md), recording the
operator's 2026-10-23 directive. This design implements that ruling without
adding semantics, so it declares no new proposed decision. The existing 0057
file is an authorized uncommitted implementation input: implementation must
commit it; chief commits only the three design files. Preserve
[0007](../../docs/decisions/0007-hermetic-gates-single-model.md),
[0009](../../docs/decisions/0009-agent-council-hybrid.md),
[0013](../../docs/decisions/0013-four-model-review-panel.md),
[0017](../../docs/decisions/0017-codex-review-seat-delivery-sandbox.md),
[0050](../../docs/decisions/0050-delivery-gates.md), and
[0056](../../docs/decisions/0056-signing-policy.md) as amended by 0057.
Accepted historical bodies remain intact; corrections use dated addenda.

## Council synthesis

Read both `.brokkr/state/design/positions/simplicity.md` and `robustness.md`;
their `.forge/design/positions/` copies are byte-identical. The requested
`.brokkr/state/tasks/` directory is absent. The actual framing is
`.forge/intake/deliver-the-operator-requested-c-f2f37969.md`, AC1–AC8.
These ignored paths identify evidence, not committed dependencies. The
following rulings carry the substantive synthesis into this committed spec.

1. **Adopt simplicity's scope and inheritance.** Keep `codex -> self` and
   `codex-careful -> codex`, singleton office overlays, one policy and the
   complete inherited design/verify/ship seats. Reject copied sequences,
   proof scripts, policies, a new scheduler and runtime repairs. Robustness's
   SMTP, pruning, await, webhook, chaos, DATA and TLS examples explain what
   delivery protects; they authorize no new runtime features or separate
   runtime test campaign here. The cost is unresolved epic residuals.
2. **Adopt robustness's execution boundary over a config-only finish.**
   Structural tests and compilation cannot prove mounted script origin,
   actual policy evaluation, live provider selection, result delivery or
   ledger generation. Require those executions and all inherited gates.
   Reject treating the cheapest passing checks as delivery. The simplicity
   cost is slower gates, paid independent review and possible refusal despite
   a small diff. Evidence costs win over reducing proof to configuration.
3. **Rule for bounded evidence.** Both recipes compile and their assignments
   are tested; every seat dispatched by this delivery needs its actual
   receipt. Adopt simplicity's refusal to demand a second paid full delivery
   solely for the careful effort variant. Reject a broader interpretation of
   robustness requiring every variant or eight runs here. The cost is openly
   unproved live behavior for unexercised variants, never a blanket claim of
   live verification or eight successful concurrent deliveries.
4. **Adopt independent review without fallback.** Provider failure leaves
   work outstanding. Reject replacing an independent reviewer with the author
   or DeepSeek Pro, softening security holds or inferring safe residual facts.
   Availability loses to the operator's review requirement. Dsh read-only
   conduct remains a charter obligation with detection, not native tool
   confinement; a sandbox redesign is outside this bootstrap.
5. **Rule for frozen inputs over convenient repairs.** Review and later
   commit existing authorized bytes without changing pinned bundles, agents,
   adapters, charters, tooling or the run pin. Add needed non-pinned tests/docs
   only. Required pinned repairs become findings for an operator-prepared new
   run. The cost is a new run; explanatory prose cannot repair execution.
6. **Reconcile the evidence-path conflict without rewriting evidence.**
   Preserve both position copies; new evidence/results/journals use `.forge/`.
   Frozen charters require the legacy position path, conflicting with 0057's
   instruction not to create state-only root `.brokkr/`. Chief observed
   successful compile/default lookup in this workspace after panel output;
   that does not settle host launch/resume or fresh-clone behavior. Reject
   deleting evidence, fabricating adopter configuration or calling the mirror
   a discovery fix. Record charter repair for a future pin and prove dispatch.

## Requirements

| Seat | `codex` | `codex-careful` |
|---|---|---|
| Intake | Sol medium | Sol high |
| Simplicity | Sol high | inherited Sol high |
| Robustness and chief | Astra high | inherited Astra high |
| Implementation | Sol high | Sol xhigh |
| Correctness | DashScope `qwen3.8-max`, provider-default effort | inherited |
| Security | Claude Opus high | inherited |
| Guarantees | official DeepSeek `deepseek-flash` high | inherited |
| Tests | DashScope `glm-5.3`, provider-default effort | inherited |
| Design check, verify, ship | inherited offline exec seats from `self` | inherited |

Sol resolves to `gpt-6.1-sol`, Astra to `gpt-6-astra`, Opus to
`claude-opus-5-5`. New offices have exactly one model, retain base charters
and have no fallback. `self` retains GLM intake/implementation and its
historical Qwen/Opus/Astra/Fable review chains; its shared design council is
the intentional 0057 amendment.

Keep boxed workspace writes, masked Cargo credentials, read-only rustup and
offline verify/ship. Credentials are named, never recorded as values. Preserve
the protected four-lens panel, worst-verdict aggregation, security-only
ownership of `security-hold` and typed residual admission. Unknown facts
never authorize shipping. Keep `recipes -> bundles`, the standalone realm
map, explicit `.forge/` journals and `.forge/ledger/<run>.md`. Recover from
the real journal under the same pin; never reconstruct historical rulings.
Missing or partial result files are incomplete attempts.

## Acceptance Criteria

- **AC1 — Design (intake AC1).** Chief HEAD touches exactly
  `specs/codex-delivery-recipes/{spec,plan,tasks}.md`, all committed, nonempty
  and clean. Both positions are reconciled above. The actual final pinned
  design check validates that HEAD and prerequisites. Chief returns
  `drafted`; only the deterministic check produces the phase verdict.
- **AC2 — Composition (intake AC2).** Recorded-engine compilation of `self`,
  `codex` and `codex-careful` succeeds without policy-sweep warnings. Recipe
  and constitution tests enforce inheritance, protected policy, whole base
  design/verify/ship origin and historical self routes. Actual dispatch
  confirms inherited proof origin; static assertions alone cannot prove it.
- **AC3 — Work routes (intake AC3).** Tests pin every model/effort above,
  singleton chains, inherited charters, workspace hands, credential masks and
  read-only toolchain. No pinned bytes or pin change after run start. Receipts
  distinguish actual routes from assignments and name unexercised variants.
- **AC4 — Review/policy (intake AC4).** All four reviewers actually dispatch
  on the authorized provider/model routes and deliver engine-consumed results,
  including official Flash, never Pro. Read-only conduct and tree stability
  hold. The recorded engine's evaluator proves clean-review routes,
  worst-verdict aggregation, hard security holds, explicit-safe residual
  admission, above-medium refusal and absent/null/wrong-typed safety refusal.
  String assertions or a locally reimplemented policy are not execution proof.
- **AC5 — Discovery/paths (intake AC5).** Default named lookup resolves both
  recipes to canonical bundle definitions and agrees with explicit compile.
  The realm resolves without sibling checkouts and uses `.forge/` journals.
  Actual ship writes a nonempty `.forge/ledger/<run>.md` for this run, naming
  its candidate head and genuine journal evidence. Host discovery/dispatch
  limitations are reported; position mirrors are not compatibility proof.
- **AC6 — Gates (intake AC6).** Launched `.forge/bin/brokkr` matches frozen
  tooling by SHA-256, not version alone. Every
  [house gate](../../docs/house-rules.md#gates--part-of-every-change) passes
  on the final candidate with locked Cargo inputs. Actual offline verify
  executes the inherited transitive pinned proofs: checked shims, exact
  zero-missed lines/branches/functions, coverage/design selftests, tamper
  boundary, publish/seal proofs, licenses and fail-closed native-tls absence.
  CI-only proofs retain their owner and are not claimed as locally executed.
- **AC7 — Receipts (intake AC7).** Evidence identifies command/dispatch,
  candidate, engine/pin, run/attempt, actual provider/model, exit/verdict and
  result/log location without secrets. Structural checks, compile, evaluator,
  actual gates and live routes are reported separately. Provider/gate failure,
  missing results or required pinned repair remains outstanding and prevents
  completion by assertion.
- **AC8 — Delivery/isolation (intake AC8).** Implementation reviews and
  commits all authorized bootstrap files and necessary non-pinned tests/docs
  unsigned with rationale on `epic192-codex-recipes`. No push, merge,
  dependency widening, pin edit, accepted-body rewrite or epic runtime fix.
  Documentation requires at most eight runs with distinct branches, worktrees,
  journals, evidence, writable targets and private cache overlays, reserved
  overlapping files and integration gates. This bootstrap launches none of
  the eight workstreams. Task states follow phase evidence; close-out requires
  a clean tree and head matching the real ledger.

## Concurrent scope observation — 2026-10-23

During chief drafting, a preservation check detected new `codex-fast`
bundle/offices/charters and external changes to 0057, the 0009 addendum,
the decision index and house rules. They appeared after the intake/panel and
chief's baseline. Chief neither authored nor validated those changes and
preserves them without staging them. Further external drift appeared in
recipe tests, CI and README files, including `bundles/README.md`; actual pin
membership and original scope must be checked, not assumed. References to
0057 here apply to the
original two-recipe authority (rulings 1–5); newly appearing triage/fast-lane
rulings are not part of this design or authorization to reduce this run's
four-member review. The implementation inventory is the original framed
bootstrap, not every file matching a broad `codex-*` glob. Reconcile concurrent
scope/pin drift with the operator's run preparation before claiming delivery;
required changes to pinned inputs need a new run. Do not rewrite or discard
another writer's files to make this attempt pass.

## Preserved behavior and exclusions

Preserve 0001 accept == stored, 0002 subscribe-before-check, 0003 oldest-only
pruning and 0005 ingest-time extraction. Add no production Rust, dependency,
workspace member, feature matrix, coverage exclusion or second proof owner.
Retain real-protocol suites and exact coverage without denominator changes.

This bootstrap promises neither exactly-once SMTP nor durable webhooks,
unlimited retention, global memory bounds, stronger dsh confinement or eight
simultaneous heavy builds. Existing limits are not scope for silent repairs.
