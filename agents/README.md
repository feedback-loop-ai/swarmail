# Agents — the council

The seat library (decision 0016): one definition per agent — description,
charter, model chain, tool allowance, limits. `brokkr agents show <name>`
reads one back; `brokkr compile --bundle .` refuses a bundle that
references a seat the library cannot resolve.

The machine is the hybrid (decision 0009, *proposed*): a contrarian
design panel argues before any code, a chief synthesizes and commits the
spec, a deterministic script rules the artifacts; verify and ship are
hermetic exec gates; review is a four-member panel (decision 0013) —
correctness, security, guarantees and tests, one independent model route
each — whose worst verdict rules and whose `security-hold` is the
operator's alone.

| Agent | Seat | Charter |
|---|---|---|
| `intake` | intake | [charters/intake.md](charters/intake.md) |
| `position-simplicity` | design:positions | [charters/position-simplicity.md](charters/position-simplicity.md) |
| `position-robustness` | design:positions | [charters/position-robustness.md](charters/position-robustness.md) |
| `chief-architect` | design:chief | [charters/chief-architect.md](charters/chief-architect.md) |
| `implementer` | implement | [charters/implementer.md](charters/implementer.md) |
| `review-correctness` | review:correctness | [charters/review-correctness.md](charters/review-correctness.md) |
| `review-security` | review:security | [charters/review-security.md](charters/review-security.md) |
| `review-guarantees` | review:guarantees | [charters/review-guarantees.md](charters/review-guarantees.md) |
| `review-tests` | review:tests | [charters/review-tests.md](charters/review-tests.md) |
| `muninn` | standing overseer (no phase) | [charters/muninn.md](charters/muninn.md) |

The phase machine is [`policy.json`](../policy.json); the `design`
sequence's deterministic check is its final exec step, the pinned
[`bundles/self/scripts/design-check.sh`](../bundles/self/scripts/design-check.sh)
(only its `designed` advances; a phase-level `drafted` stops);
the seat wiring is [`bundle.json`](../bundle.json). Panel members and
sequence steps carry no `limits` of their own — the seat above them owns
the budget, and the compiler refuses a declaration it would discard.
The retired starter reviewer seat (`reviewer.json` + its charter) is kept
as record until decision 0009 is accepted, then removed.
