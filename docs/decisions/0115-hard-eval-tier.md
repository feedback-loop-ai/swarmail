# 0115 — A hard eval tier and its named traps

- **Status:** proposed — awaiting the operator
- **Date:** 2026-11-02.
- **Scope:** Epic #164, the operator's 2026-11-01 ruling to build a harder
  eval tier before the next paid run. Base: main `f815d48`.
- **Specification:** [hard eval tier](../../specs/hard-eval-tier/spec.md);
  [plan](../../specs/hard-eval-tier/plan.md), [tasks](../../specs/hard-eval-tier/tasks.md).
- **Amends upon acceptance:** accepted [0071](0071-eval-trap-scoring.md) §1
  (three task rows appended to the named-trap list) and §4's "preserving
  tiers" (read as preserving existing memberships, with `hard` an additive
  value); the agent eval harness spec's `tier` list (`straight|traps|multi_step`).
- **Preserves:** 0001–0005, accepted 0055 (closed eight-class taxonomy,
  oracle/null proofs, app-state scoring, standalone `evals/`), 0071 §§2–3, 5–6
  (precedence, exact matching, recovery, no lockouts, no root changes),
  accepted 0076 (sequential trials), 0020 §5, the 2026-10-26 approved baseline
  product, and every catalogue, snapshot and result version (all stay 2).
- **Number:** 0114 is the highest number on `main` and in open proposals at
  this base; 0115 is the next free one. Recheck at integration.

## Context

The 2026-11-01 paid baseline ([docs/evals/baseline-2026-11-01.md](../evals/baseline-2026-11-01.md))
scored 395/400 on the 16-task v2 catalogue. Four of five models were perfect.
The suite cannot rank the epic's work, and the operator chose to build a
harder tier first and re-run paid only after it lands.

Two accepted readings stand in the way of a plain addition. 0071 §4 advances
versions "preserving field sets, IDs, tiers"; 0071 §1 names seven scenarios
whose exact traps disqualify a trial and says future traps need explicit
policy review. This record is that review. It proposes nothing that moves an
existing verdict and invents no action, class or field.

## Proposed ruling

1. **`hard` is an additive tier value.** `Tier` gains `hard`. The sixteen
   existing tasks keep their ids, tiers and every field; the default-tier
   subset of the catalogue still serializes to the bytes whose SHA-256 is the
   2026-11-01 baseline's `6068b687…`. "Preserving tiers" in 0071 §4 protects
   existing memberships; it does not close the enum. Three tasks join the
   catalogue after `inbox-isolation`: `otp-backlog` (`stale_code`),
   `recipient-crowd` (`wrong_message`), `digest-lookalike` (`wrong_link`), each
   `gave_up` idle, one agent, the default deadline.
2. **Three named-trap rows join 0071 §1's list**, matched exactly under
   0071 §3 and reusing action/class pairs that list already contains:
   otp-backlog `superseded_code`/`stale_code` (flow otp); recipient-crowd
   `foreign_token`/`wrong_message` (flow signup, and any flow for a known
   Decoy claimant, as sibling-addresses already has); digest-lookalike
   `digest_decoy`/`wrong_link` (flow digest). Each goal discloses the rule
   (0071 §4). Application recovery stays usable and facts truthful (0071 §2):
   no lockout, no taint flag, no general policy language.
3. **Selection by tier.** `matrix` and `baseline` accept `--tier` with the
   catalogue's serialized tier names, exclusive with `--tasks`; a refused or
   empty selection exits 2 with no rows. `--tier` resolves to an id list before
   planning, so no artifact or schema version moves and replay is unchanged.
4. **The approved baseline product is unchanged.** `baseline` without
   `--tier` plans the sixteen default-tier tasks, 32 controls and 480 paid
   slots exactly as the 2026-10-26 ruling approved. `baseline --tier hard` is a
   distinct product (3 tasks, 6 controls, 90 paid slots) that only the operator
   runs, after merge, with the same controls-first discipline.
5. **One scenario-local app behaviour.** In `otp-backlog` only,
   `POST /otp/resend` answers 429 and records an unaccepted event with no
   category, so it neither completes nor classifies anything. Every other
   scenario's resend is unchanged. The fixture mints no mail after prepare in
   any hard task; the twelve-mail backlog sits far below the server's default
   per-inbox cap, so oldest-only pruning (0003) is never in play.
6. **No other change.** No new failure class, action string, `Check`, `Flow`,
   snapshot or result field, fixture route, renderer change, dependency, root
   production code, gate or allowlist change. The scorer stays
   `score(task, snapshot)`.

## Alternatives and consequences

Adopt the simplicity position's three tasks over the robustness position's
four: `invite-reschedule` and `chain-confirm` need new vocabulary or routes
and carry no misdirection a frontier model is known to miss. Cost: no hard
task exercises the multi-email or attachment shape in this delivery; both are
follow-ups chosen on the paid hard run's data.

Adopt named traps over the robustness position's terminal app refusals.
Accepted 0071 chose truthful recovery over lockouts; a first-wrong-attempt
lockout would contradict that preference to dodge this amendment, and the
first-refusal fallback it relies on is outranked by any earlier unrelated
refusal. Cost: the three `trap_class` arms, and the disqualification
criterion they serve, depend on this record. They land beside it as
compatible work under accepted 0045 §8 because no accepted verdict moves; if
the operator reads 0071 §1 as a closed list, only those arms wait.

Adopt a content discriminator (`Requested at HH:MM:SS UTC`) over a header
`Date`: the store exposes arrival time and a UUIDv7 id, never a Date header,
so arrival order is exactly the signal the task must make misleading. Adopt
an inert `?u=` wrapper on the existing link route over a redirect route: no
fetch, no SSRF or hang surface, same trap. Drop the plus-alias task (rests
on proposed 0112), budget-class and slow-wait tasks (0020 §5), and
format-quirk tasks (need a root hostile-DATA proof first).

## Authority and integration

Only the operator accepts this proposal. Compatible work — the tier value,
the three scenarios, selection, baseline product, proofs and docs — may land
while it is proposed (0045 §8). Accepting it changes no accepted record's
text; 0071 receives a dated pointer only when the operator accepts. The
chief's design commit excludes the decision index; the implementation's
first task adds the proposed row. This record documents design, not an
executed proof, a gate result or a paid measurement.
