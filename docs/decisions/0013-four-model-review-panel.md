# 0013 — A four-model review panel

**Status**: accepted (operator directive, 2026-10-13 — "a bigger panel:
qwen 3.8 max, astra, opus and fable in a panel")

## Ruling

The delivery constitution's **review** seat — the protected phase — is
a four-member parallel panel, one independent judge per model route:

| Member | Lens | Abstract model | Route |
|---|---|---|---|
| `review-correctness` | correctness & fit | `qwen-max` | dsh → `dashscope/qwen3.8-max` |
| `review-security` | threat model; owns `security-hold` | `opus` | claude → `claude-opus-5-5` |
| `review-guarantees` | frozen guarantees & ADR fidelity | `astra` | codex → `gpt-6-astra` |
| `review-tests` | test realism, 0011 placement, coverage honesty | `fable` | claude → `claude-fable-5-1` |

Every member's model chain ends in `glm-flash` as the fallback route,
so a dead provider degrades to the standing route instead of stalling
the protected phase. All four members run `class: gate` — the
composition the claude and codex adapters measure as read-only.

This amends decision 0007's second hard rule ("one model route") on the
**judging axis only**. What 0007 ruled and still stands:

- `verify` and `ship` remain hermetic exec seats — no model certifies
  its own work.
- The **work axis stays on the standing route**: `intake`, the design
  panel (`position-simplicity`, `position-robustness`,
  `chief-architect`) and `implementer` run `glm-flash` on the local
  spark route, keeping the expensive, nondeterministic seat cheap and
  local (0007's "Why", unchanged).
- `adapters/dsh.json` gains the `dashscope` route (`uncontracted`
  egress, decision 0036's closed vocabulary) and the abstract
  `qwen-max` → `dashscope/qwen3.8-max` mapping; `adapters/claude.json`
  and `adapters/codex.json` are adopted verbatim from the engine's
  measured reference adapters for this machine (claude 2.1.296 serves
  fable/opus; codex 0.160.0 serves astra).
- The panel aggregate is unchanged: the worst member verdict rules;
  `security-hold` belongs to the security member alone and remains a
  hard stop.

## Why

0007 pinned one route to keep the judge capable of reading exactly what
the implementer wrote and the supply chain auditable. That rationale
governs the work axis and is preserved. The *review panel* is a
different instrument: its value is independence, and four judges on one
route share one model's blind spots — correlated confidence is how a
flawed diff gets four identical clean verdicts. Diverse routes turn the
panel into four independent readers: different tokenizers, different
training lineages, different failure modes, arguing over the same diff
under read-only charters. The realm's own history argues for this:
issue #31's findings are exactly the kind (ordering, resurrection,
adapter drift) that a single-model panel can ratify green.

Cost is the honest price: a review attempt is now four model calls
(the seat allows three attempts). The operator directs the budget; the
`qwen-max` judge on the token-plan route and the glm-flash fallbacks
keep the marginal attempt survivable.

## Consequences

- `agents/review-correctness.json` and `agents/review-security.json`
  changed model chains; `agents/review-guarantees.json` and
  `agents/review-tests.json` are new members with charters in the
  house style (read-only; `clean`/`residual` verdicts; findings named
  with file:line and decision references).
- The new routes carry credentials by name only: `DASHSCOPE_API_KEY`
  (dsh's own env file), the claude and codex CLIs' own logins. No
  secret value enters the bundle, the journal or this repository
  (decision 0012's discipline, unchanged).
- `dashscope` is declared `uncontracted` — the operator's token-plan
  endpoint is a third party 0036 has not ruled `contracted`. No review
  seat binds secrets, so egress_minimum never binds here; the
  classing is recorded so the route's posture is explicit.
- The dsh `tool_permissions: "unsupported"` / `mcp: "unsupported"`
  declaration still holds, so no model seat on dsh may declare a
  restricted tool surface — the new charters declare none. The claude
  and codex adapters express gate-class read-only composition
  natively; members on those providers get it.
- Cost accounting (`brokkr costs`) now spans four routes; the judge is
  no longer trivially the implementer's own model. This is accepted:
  the panel's job is independence, not sympathy.
- 0007's title keeps its historical meaning; this document is the
  index's pointer for the judging axis from 2026-10-13 onward.
