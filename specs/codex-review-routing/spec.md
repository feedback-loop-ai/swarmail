# Codex review routing

- **Created:** 2026-10-24
- **Status:** Draft design; implementation and verification remain outstanding.
- **Scope:** swarmail delivery recipes only.
- **Source:** operator directive and intake for run `update-the-codex-delivery-recipe-c79c3f2d`.

Decision: 0058 — [Exact amendment to Codex review routing](../../docs/decisions/0058-codex-review-routing.md) (proposed).

## Feature and rationale

Resolve the operator's requested reviewer roles without changing who implements,
how triage chooses a lane, or which proofs authorize delivery. Standard means
`bundles/codex`. High-risk means the existing `design` and `engine` strategies
of `bundles/codex-fast`; it is not a fourth recipe or a new strategy.
`bundles/codex-careful` uses the same five review roles as High-risk and keeps
Codex Sol xhigh implementation. Standard intentionally drops its dedicated
guarantees member; correctness still checks all applicable accepted rulings.

The current four-role composition and Careful's inherited Standard review do
not implement this directive. Replacing their shared correctness office alone
would also remove Qwen from fast feature. Separate role offices and exact
resolved composition checks prevent those accidental changes.

### Required matrix

A dash means the role is absent, not optional. All listed members are gate-class
and read-only, except for writing their own distinct result files.

| Recipe / fast strategy | correctness | security | tests | guarantees | independent |
| --- | --- | --- | --- | --- | --- |
| Standard (`codex`) | Claude Fable 5.1 | official DeepSeek Flash | Claude Sonnet 5.5 | — | — |
| High-risk (`codex-fast`: `design`, `engine`) | Codex Astra | official DeepSeek Flash | Claude Sonnet 5.5 | Codex Astra | Claude Fable 5.1 |
| Careful (`codex-careful`) | Codex Astra | official DeepSeek Flash | Claude Sonnet 5.5 | Codex Astra | Claude Fable 5.1 |
| Fast `feature` | DashScope Qwen Max; Sonnet 5.5 only on authorized dispatch failure | official DeepSeek Flash | — | — | — |
| Fast `chore` | One existing official Flash reviewer covers correctness, security, tests and guarantees | included in combined reviewer | included | included | — |

Exact aliases resolve through existing adapters: `fable` to
`claude/claude-fable-5-1`, `sonnet` to `claude/claude-sonnet-5-5`, `astra` to
`codex/gpt-6-astra`, `qwen-max` through dsh to `dashscope/qwen3.8-max`, and
`flash` through dsh to `deepseek-official` / `deepseek-flash`.
DeepSeek Pro is never a selectable route, inherited model or fallback.
DashScope retains provider-default effort without an effort flag. Reviewer
routes that support effort use high; existing work/design effort choices stay
unchanged, including Careful's Sol xhigh implementation.

## Synthesis and explicit rulings

Both run-local positions were read: `.brokkr/state/design/positions/simplicity.md`
and `robustness.md`, each identifies this run. They are evidence, not artifacts
to commit or substitutes for review results.

1. **Adopt simplicity's existing-lane topology; reject robustness's additional
   `codex-high-risk` recipe.** High-risk is explicitly fast `design`/`engine`.
   Careful overrides review instead of inheriting Standard's smaller panel.
   Cost: three explicit five-member declarations can drift, and there is no
   standalone High-risk launch name. Exact resolved equality tests pay that
   cost; a new recipe, discovery entry and CI enumeration buy no required lane.
2. **Adopt robustness's distinct independent office, using simplicity's tiny
   inheritance alternative.** `codex-review-independent` extends the Fable
   correctness office and inherits its whole-diff correctness/fit charter.
   It is a separately dispatched member with a separately required verdict.
   Reject a copied charter, the generic single-seat reviewer (which can own
   security-hold), and merely reusing the same office identity in the panel.
   Cost: one small alias file and deliberate overlap with correctness; the
   existing charter already checks criteria, tests, guarantees and excess scope.
3. **Adopt robustness's behavioral proof burden.** Compiler acceptance and a
   two-element model array are insufficient evidence of fallback safety or
   aggregation. Use the recorded engine's dispatcher/evaluator with controlled
   outcomes. Reject a second resolver, fallback wrapper, health registry or
   policy DSL. Cost: provider outages still stop most routes; an unsupported
   dispatch boundary blocks the affected implementation proof, not this design.
4. **Adopt both positions' preservation boundary.** Keep distinct Astra
   correctness and guarantees offices, charters, calls and results. Five roles
   do not mean five model families. Fable provides a different reader, not a
   claim that correlated errors are eliminated. Robustness's storage, waiter,
   pruning, webhook, chaos, byte-framing and TLS examples remain existing review
   obligations; reject adding server repairs or new runtime promises here.
5. **Reconcile amendment provenance.** This semantic change gets new proposed
   decision 0058, as required by the decision discipline and design validator.
   Downstream documentation appends only a dated pointer/addendum to 0057,
   preserving its historical accepted body and identifying 0058 as proposed
   until the operator accepts it. The present directive is the source of the
   narrow fallback requirement; no historical acceptance date or receipt is
   invented. This seat does not accept a ruling or grant broader policy changes.

## User scenarios

- An operator selects Standard: exactly Fable correctness, Flash security and
  Sonnet tests review the verified candidate. There is no inherited fourth role.
- Triage classifies a change as `engine` or `design`, or the operator selects
  Careful: all five required roles return separately, including both Astra
  roles and independent Fable. A missing fifth verdict cannot become clean.
- Triage selects `feature`: Qwen correctness and Flash security run. Qwen
  dispatch failure may select Sonnet; a valid adverse Qwen review does not.
  A failed Flash review cannot be replaced. `chore` retains its combined reviewer.

## Contracts and boundaries

Fallback is exclusive to fast-feature correctness, ordered Qwen then Sonnet.
A supported dispatch failure before a valid review result may advance that
chain. A valid clean, residual or other completed verdict never triggers
substitution. Missing, null, wrong-typed or otherwise invalid review facts are
not safe facts and do not authorize retrying with another model to erase them.
Failure of Sonnet stops without shipping; there is no third model. Record the
attempted route, dispatch failure category and selected fallback in existing
run evidence, without credentials. If the pinned engine cannot enforce this
boundary, identify the concrete gap and return it for a bounded engine repair
and tooling-identity review; do not silently broaden the exception or repin.

Panel security alone owns `security-hold`; the existing single chore reviewer
keeps combined authority. Every member supplies explicit `fixes_applied: false`
and boolean `spec_defect`, with valid residual safety facts when applicable.
Worst-verdict aggregation, unknown-fact refusal, bounded scope/oversized returns,
verified-candidate binding and distinct run/member result identities remain.
No missing or unknown strategy chooses a default. Review cannot edit the
candidate. Verify, ship and final design checking remain inherited exec proofs.

No production Rust, dependencies, license allowances, gate algorithms, public
shims, tooling pins, providers, credentials, release policy, journal topology or
concurrency limits change. Legacy `self` work and review routing remain.
Accepted 0001–0005 remain binding: synchronous acceptance/queryability,
subscribe-before-check, oldest-only pruning, real-protocol tests and extraction
at ingest. This amendment adds no durability, webhook-delivery, exactly-once,
unbounded concurrency or new timeout promise. No push, merge or issue closure.

## Acceptance Criteria

- **AC1 — Standard:** engine-resolved `codex` review has exactly correctness,
  security and tests, on the exact matrix routes, all gate-class, with the
  existing aggregate, result vocabulary, typed inputs, limits and timeout.
- **AC2 — Five roles:** resolved Careful and both fast full-risk selectors have
  exactly the five matrix roles and identical role/route assignments. Astra
  correctness and guarantees retain distinct offices, owning charters, dispatch
  identities and results; independent Fable has its own office and result.
- **AC3 — Authors and triage:** every Codex recipe implementer resolves only to
  Codex; Careful remains Sol xhigh. Existing intake/design efforts and fast
  initial triage, strategy vocabulary, direct chore/feature paths, full design
  paths, escalation and no-default refusal remain intact. Counts are 1/2/5/5.
- **AC4 — Fast routes:** chore retains its one combined Flash reviewer. Feature
  has exactly dedicated Qwen correctness and official Flash security; its
  resolved correctness chain is exactly `[qwen-max, sonnet]`. Every other Codex
  reviewer chain is single-model. Legacy `self` chains are unchanged.
- **AC5 — Dispatch:** real pinned dispatcher tests show Qwen dispatch failure
  selects Sonnet, Qwen success and adverse verdicts do not, malformed result
  facts cannot be laundered by fallback, Flash failure never substitutes, and
  fallback exhaustion refuses shipping. Evidence identifies routes and failure
  categories without secrets. Offline controls are not claimed as live-provider
  compatibility evidence; any unavailable execution proof stays outstanding.
- **AC6 — Route fidelity:** resolved aliases/provider selection match the matrix,
  DashScope emits no effort flag, other reviewer routes use high, and no selected
  office or inherited/fallback chain can reach Pro or an unapproved route.
- **AC7 — Protected policy:** actual engine aggregation/evaluation over affected
  1/2/3/5-member shapes preserves hard security hold, above-medium/security
  residual refusal, missing/null/wrong-typed fact refusal, scope-defect returns
  and bounded exhaustion. A fifth member's residual changes the aggregate; a
  missing result never becomes clean. All members remain read-only and security
  hold authority remains correctly scoped.
- **AC8 — Proof ownership:** before/after resolved policy, inherited exec seats,
  design sequence, script origins, tooling identity and candidate binding agree
  outside directed review changes. Wrong/stale candidate and cross-run/member
  results refuse; no model or copied child sequence replaces a shell gate.
- **AC9 — Regression strength:** negative controls fail for missing/extra roles,
  swapped security route, merged Astra roles, missing independent verdict,
  unauthorized fallback, Pro, non-Codex implementation and unknown strategy.
  Replace obsolete four-unique-model and Careful-inherits-Standard assertions
  with exact resolved expectations rather than removing their protection.
- **AC10 — Documentation:** current summaries state the matrix and High-risk
  mapping; 0057's historical body is preserved with a dated scoped amendment
  pointer, and the decision index matches each record's actual status. Only the
  operator can accept 0058. No unrelated decision or recipe expansion lands.
- **AC11 — Validation:** self, codex, codex-careful and codex-fast compile with
  the checksum-identified recorded engine. Implementation/verify leave the
  entire [house-rules gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
  clean, including exact coverage and selftest, publish/seal proofs and the
  fail-closed native-tls absence proof; CI retains its listed additional proofs.
  Structural tests, behavioral engine proofs and live-provider evidence are
  reported separately. No green claim is inferred from this design draft.
