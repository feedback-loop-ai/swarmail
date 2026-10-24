# Agents — the council

The seat library (decision 0009): one definition per agent — description,
charter, model chain, tool allowance, limits. `brokkr agents show <name>`
reads one back; `brokkr compile --bundle bundles/self` refuses a bundle
that references a seat the library cannot resolve.

The machine is the hybrid (decision 0009, *accepted*): a contrarian
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

The phase machine is [`bundles/self/policy.json`](../bundles/self/policy.json); the `design`
sequence's deterministic check is its final exec step, the pinned
[`bundles/self/scripts/design-check.sh`](../bundles/self/scripts/design-check.sh)
(only its `designed` advances; a phase-level `drafted` stops);
the seat wiring is [`bundles/self/bundle.json`](../bundles/self/bundle.json). Panel members and
sequence steps carry no `limits` of their own — the seat above them owns
the budget, and the compiler refuses a declaration it would discard.
The single-seat reviewer (`reviewer.json` + its charter) remains unwired
in `bundles/self/bundle.json`. It now supplies the base checks for the
`codex-bug-reviewer` overlay on the fast recipe's small-bug route, with
an explicit read-only/result contract and scope-defect reporting.

Decision [0057](../docs/decisions/0057-codex-delivery-recipes.md) adds
`codex-*` office overlays, preserving these charters while selecting the
operator's Codex work models and independent external reviewers. The shared
design council now uses those Codex offices. See the
[recipe tables](../bundles/README.md) for exact chains, efforts and launch
commands. The `self` intake/implement and review offices above retain their
original chains. The operator-directed amendment in proposed
[0058](../docs/decisions/0058-codex-review-routing.md) assigns Fable correctness,
Flash security and Sonnet tests to Standard; Careful and fast full-risk lanes
add distinct Astra correctness/guarantees and independent Fable offices.
`codex-review-independent` and `codex-review-correctness` both directly inherit
the full correctness base charter; the engine permits only one overlay level.
Independent Fable has its own required verdict.
All reviewer chains are single-model except dedicated fast-feature correctness:
Qwen Max then Sonnet 5.5 on pre-session dispatch failure only. No completed
verdict or malformed facts trigger substitution. DeepSeek Pro is forbidden.

`codex-fast` adds `codex-triage`, `codex-fast-implementer` and
`codex-bug-reviewer`: triage records scope, the implementer returns broader
work for reclassification, and the single reviewer owns correctness and
security together. Broader fixes use dedicated Qwen correctness and official
Flash security offices; full design and engine work require all five roles.
Only the security member (or combined chore reviewer) owns security hold.

## Correctness charter resolution for Codex recipes

The tracked library uses `agents/codex-review-correctness.json` as the
Codex overlay. It extends `agents/review-correctness.json`, selects the
singleton `fable` route that accepted 0058 assigns to standard correctness, and
resolves `agents/charters/review-correctness.md`, whose tracked digest is
`8785a54a83e88cf254213afe11f230e8e10c70a18f2f147df197e40cfe59a0cb`.
Before 0058 the overlay selected `qwen-max`, and the charter digest in the
#172 candidate of 2026-10-24 was
`215d6e38a66cbaf16ed7b76f1742a68b48a8e42bb2deeb1e57ec53a099ddb2fa`.
The required compile target for this realm is
`brokkr compile --bundle bundles/self`.

Reproduce the candidate-local resolution and digest check with:

~~~bash
.forge/bin/brokkr agents show codex-review-correctness
sha256sum agents/charters/review-correctness.md
.forge/bin/brokkr compile --bundle bundles/self
~~~

At that candidate, `.brokkr/agents` was absent and the recorded engine help
documented `.brokkr/agents` as the preferred local library, falling back to
the tracked `agents/` library. The agents show output's charter digest
matched the tracked file. This is evidence for that candidate's resolution;
it does not establish which historical host copy was loaded or prove a live
review dispatch.

At the actual review launch, resolve the correctness agent again and compare
its overlay/base, singleton route, charter path and digest with the tracked
library. Repeat this for every independent reviewer required by the selected
recipe, including the official DeepSeek Flash route. If the engine resolves
a different copy or any digest differs, resolve or escalate that divergence
before claiming the reviewer was used. Do not create a `.brokkr/agents`
shadow library to repair provenance.
