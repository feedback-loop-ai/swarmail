# 0098 — Flash correctness and guarantees members; a four-hour review deadline

- **Status:** accepted (operator, 2026-10-29 — "accept the decisions",
  given in chat).
- **Date:** 2026-10-29 (operator direction given 2026-10-28)
- **Amends:** [0058](0058-codex-review-routing.md), only the correctness and
  guarantees model assignments of Careful (`codex-careful`) and High-risk
  (`codex-fast` design/engine); and the review seat's deadline in the
  [0057](0057-codex-delivery-recipes.md) recipes `codex`, `codex-careful` and
  `codex-fast`. Proposed [0090](0090-mixed-provider-routing.md) is affected
  only because `mixed-opus-terra` inherits careful's review seat unchanged.
  Proposed [0081](0081-glm-flash-seats.md)'s tests assignment is unchanged.
- **Source:** operator directions to the operator's assistant, 2026-10-28:
  “pin correctness and guarantees to dsh with dsh flash on deepseek
  platform” and “make the timeout 4x longer, we are not looking for speed,
  but for efficiency and parallelism”. This record accepts no decision.

## Context and evidence

The full five-member panel (Careful, fast design/engine, and mixed through
inheritance) shares one review-seat deadline of 3600 s; the panel effect
ends when it expires, settled members or not. The engine's composition rule
at the pinned source base (`crates/brokkr-runtime/src/bundle/assemble.rs`,
`seat_limits`, brokkr 731189f) makes an explicit seat limit win over any
office limit, and a panel member may declare none, so the seat's
`limits.timeout_seconds` is the only deadline a panel member has.

Dated journal observations (`.forge/forge.db`, read-only, 2026-10-29):

- **Tests member at the deadline.** In parked runs `…-4f13ffbd`,
  `…-4cf400ed`, `…-c4703b16` and `…-fced9656` the review effect went
  `effect/indeterminate` with “panel members ["tests"] could not establish
  completion” exactly 3600 s after `effect/started` (for example 16:25:25Z →
  17:25:25Z on 2026-10-28 in `4f13ffbd`), with the GLM-5.3-Flash member still
  checkpointing tool turns. That is a deadline kill, not a crash or an
  adverse verdict; the other four members had settled. The operator's
  assistant counted eight such tests-member indeterminacies among parked runs.
- **Spark capacity (dated observation, not a contract).** A read-only GET of
  the Spark server's `/health` and `/metrics` by the operator's assistant,
  2026-10-28 ≈22:55Z: backend `tensorfold`, served context 512000, at most
  six concurrent streams; 2,807 lifetime requests with mean model-turn
  latency ≈52 s (146,894 s / 2,807), mean time to first token ≈8.5 s,
  ≈10% of turns over 120 s, prompt-cache hit ≈94%. The dsh route caps the
  window at 200k with compaction at 80% and 32768 max tokens, with no
  reasoning-effort control. A tests review of ~70 or more turns cannot fit
  3600 s at that mean; 14400 s fits about 275.
- **Astra members.** In parked runs `…-af74a948`, `…-b7b03105`, `…-57f99bd9`,
  `…-22e90c39` and `…-e6597bb7` the two Codex Astra members went
  indeterminate together (“panel members ["correctness", "guarantees"]
  could not establish completion”) while security, tests and independent
  settled. These ended 24–55 minutes after `effect/started`, before the
  deadline, so they are not timeout evidence; the route change for them is
  the operator's direction, not a measured cure.

## Proposed ruling

1. **Correctness and guarantees take security's route.** In Careful and
   High-risk (and so in `mixed-opus-terra`), `codex-review-astra-correctness`
   and `codex-review-guarantees` select dsh `flash` → `deepseek-flash` on the
   `deepseek-official` route at `high` effort — exactly the
   `codex-review-security` office's model and effort — as single-model
   chains with no fallback. Each keeps its own overlay of its own base
   office (`review-correctness`, `review-guarantees`), its own charter,
   invocation and result. Three of five members now share one model family;
   five roles still do not mean five independent families, and all five
   results remain required.
2. **The office name `codex-review-astra-correctness` stays.** It names a
   role slot referenced by both careful and both fast full-risk panels,
   the Rust and Python route proofs, prior journals' pinned manifests and
   historical specs. Renaming would ripple through all of those for a label,
   while the resolved route (provider, model, concrete pin, effort) is what
   the proofs pin. The name is historical; the route is the fact.
3. **Review deadline 14400 s.** The review seat of `codex`, `codex-careful`
   and `codex-fast` (all strategies, since the seat-level limit covers every
   selected case) moves from `{max_attempts: 3, timeout_seconds: 3600}` to
   `{max_attempts: 3, timeout_seconds: 14400}`. `mixed-opus-terra` inherits
   it. Intake, triage, design, implementation, verify and ship limits, the
   legacy `self` review deadline and every office limit are unchanged. The
   aim is throughput across parallel runs, not latency: a member that is
   still working is given room to finish instead of being killed.
4. **Unchanged:** Standard correctness (Fable), security (Flash), tests
   (Spark GLM, effort not applicable), independent Fable, fast-feature
   correctness (Qwen Max with the sole pre-result Sonnet dispatch fallback)
   and the chore reviewer; DeepSeek Pro remains forbidden; protected review,
   worst-verdict aggregation, security-only hold, explicit scope/fixes and
   safety facts, unknown-fact refusal, bounded returns, candidate binding,
   and every deterministic gate.

## Read-only conduct and the dsh adapter

`adapters/dsh.json` declares `tool_permissions` and `mcp` unsupported and is
not relaxed here. A dsh member has no native tool confinement the engine can
enforce. Codex Astra ran under codex-cli's sandbox class (which the delivery
environment sets to `workspace-write` so the result file can be written,
[0017](0017-codex-review-seat-delivery-sandbox.md)); that native layer is
now absent for these two members, exactly as it already is for security.
Read-only conduct is therefore a charter obligation — the correctness and
guarantees charters already state it, including that the only permitted
write is the member's own result file — and the engine's candidate binding
(head and tree recorded at start and end of the review effect) detects tree
changes. This record claims no equivalent sandbox enforcement.

## Consequences and costs

- **Worst-case time.** Three attempts at 14400 s bound one review phase at
  twelve hours. A hung member now holds a run four times longer before it
  parks, and a provider outage is discovered later. That is the stated
  trade: completion over speed.
- **Capacity bound, not fixed here.** Spark serves at most six concurrent
  streams; under 0057 up to eight delivery runs may execute at once, so tests
  members beyond six queue on the server and spend their deadline waiting.
  The longer deadline absorbs some queueing; it does not create capacity.
- **Provider concentration.** Correctness, guarantees and security now all
  depend on the official DeepSeek platform and its credential; one outage
  stops three members, with no substitution (no fallback is authorized).
  Correlated model blind spots across those three roles are accepted by the
  direction; independent Fable remains the cross-family correctness reading.
- **Cost.** Paid Codex Astra review calls are replaced by paid DeepSeek
  Flash calls; no claim of model-quality equivalence is made.

## Proof

`tests/delivery_recipes.rs` pins both offices to `flash`/`high` equal to the
security office, refuses one-fault drifts (Astra kept, fallback, effort,
hands, merged charter, security drift), and pins the 14400 s review limit for
the three recipes beside the unchanged implement, triage and `self` limits.
`tests/codex_review_routing.py` resolves every recipe through the pinned
engine and expects `('dsh', ['flash'])` for both members in Careful, fast
design/engine and mixed, with concrete `deepseek-flash` on
`deepseek-official`; the pre-change Astra route is a rejected negative
control. Compilation proves structure only; a live run must still obtain
five real results before the route is called delivered. Seat limits are not
part of the compiled manifest, so the deadline is proven at source.

Bundle descriptions, `bundles/README.md`, `agents/README.md`,
`bundles/mixed-opus-terra/README.md` and the routing paragraph of
`docs/house-rules.md` are updated. Accepted 0058 receives a dated pointer;
its body is unchanged. Historical specs that name Astra or Sonnet members
describe the routes of their own time and are not rewritten. No push, merge,
tag, publication, issue closure or decision acceptance follows.
