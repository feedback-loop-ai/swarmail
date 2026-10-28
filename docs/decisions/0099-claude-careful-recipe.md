# 0099 — A zero-Codex careful recipe: `claude-careful`

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-29 (operator direction given 2026-10-28 and 2026-10-29)
- **Scope:** one additional recipe under [0057](0057-codex-delivery-recipes.md)
  and [0058](0058-codex-review-routing.md). It reuses the tests route of
  proposed [0081](0081-glm-flash-seats.md) and the security, guarantees and
  deadline rulings of proposed
  [0098](0098-review-panel-flash-and-deadline.md). No existing recipe, office
  route, decision body or decision status changes.
- **Source:** operator directions to the operator's assistant: “propose me a
  claude-centric recipe, with dsh ds flash on ds platform and spark glm 5.3
  flash in the mix”; then “haiku 5.5 intake”, “we can try Sonnet 5.5 as
  implementer”, “reviewer seats split by opus / fable / glm 5.3 / ds flash”;
  then the design simplicity position on Opus 5.5 rather than GLM. This
  record accepts no decision.

## Context

Codex credits are exhausted and must not be spent. Every existing recipe
hires Codex somewhere: `self`'s shared design crew is Codex Sol/Astra
(0057), and `codex`, `codex-careful`, `codex-fast` and `mixed-opus-terra`
inherit it; their intake and implementation offices are Codex too, except
mixed's Opus implementer. The direction is a careful-shaped lane whose
composed route names no Codex model at all, built from Claude seats, paid
official DeepSeek Flash and free local Spark GLM-5.3-Flash.

## Proposed ruling

1. **`bundles/claude-careful` extends `codex-careful`** and overrides exactly
   the description and the `intake`, `design`, `implement` and `review`
   seats. Policy, protected review, security hold, results, inputs and the
   review-panel aggregate are careful's; `verify` and `ship` stay inherited
   exec gates. Every chain is a single model with no fallback: Flash never
   substitutes, a Spark or Claude outage never authorizes a paid
   substitution, and DeepSeek Pro remains forbidden.

   | Seat | Office | Adapter | Model → concrete | Effort | Hands |
   |---|---|---|---|---|---|
   | Intake | `claude-intake` | claude | `haiku55` → `claude-haiku-5-5` | high | workspace |
   | Design: simplicity | `claude-position-simplicity` | claude | `opus` → `claude-opus-5-5` | high | workspace |
   | Design: robustness | `dsh-position-robustness` | dsh | `flash` → `deepseek-flash` (`deepseek-official`) | high | none |
   | Design: chief | `claude-chief-architect` | claude | `fable` → `claude-fable-5-1` | high | workspace |
   | Design: check | self's validator | exec | — | — | offline workspace |
   | Implement | `claude-implementer` | claude | `sonnet` → `claude-sonnet-5-5` | high | workspace |
   | Review: correctness | `codex-review-correctness` | claude | `fable` → `claude-fable-5-1` | high | none (read-only) |
   | Review: security | `codex-review-security` | dsh | `flash` → `deepseek-flash` | high | none (read-only) |
   | Review: guarantees | `codex-review-guarantees` | dsh | `flash` → `deepseek-flash` | high | none (read-only) |
   | Review: tests | `codex-review-tests` | dsh | `glm-flash` → `spark-glm/GLM-5.3-Flash-EXL3` (local) | not applicable | none (read-only) |
   | Review: independent | `claude-review-independent` | claude | `opus` → `claude-opus-5-5` | high | none (read-only) |

   Workspace hands are careful intake's exactly: network on, `~/.cargo`
   overlay with `credentials` and `credentials.toml` masked, `~/.rustup`
   read-only. Tests effort is an explicit empty office map and an absent
   compiled effort, as 0081 rules.
2. **Offices.** New overlays, each of one base office with that office's
   charter: `claude-intake` (of `intake`), `claude-position-simplicity` (of
   `position-simplicity`), `dsh-position-robustness` (of
   `position-robustness`), `claude-chief-architect` (of `chief-architect`),
   `claude-implementer` (of `implementer`) and `claude-review-independent`
   (of `review-correctness`, like `codex-review-independent`, since the
   engine admits one overlay level). Correctness, security, guarantees and
   tests reuse the existing `codex-review-*` offices unchanged; their names
   are role slots, not providers. No charter is written or changed.
   `claude-position-simplicity` takes workspace hands because the
   `codex-position-simplicity` overlay it replaces declares them; the dsh
   robustness position declares none (rule 4).
3. **Adapter alias.** `adapters/claude.json` gains `haiku55 →
   claude-haiku-5-5`. The existing `haiku → claude-haiku-4-5-20251001`
   alias keeps its meaning. `haiku55` is not added to `judges`: the
   intake seat is a work seat, and the pinned engine resolved and compiled
   it without a judge entry. `adapters/dsh.json` and every capability
   declaration are unchanged.
4. **Robustness position without hands.** dsh declares `mcp` and
   `tool_permissions` unsupported, and the pinned engine refuses hands it
   cannot box (0081: “the provider declares hands unsupported, so the
   agent's hands cannot be put in the box”). The base `position-*` offices
   always selected dsh `glm-flash` without hands, and the pinned engine
   admits this overlay: `agents show` resolves `flash`/dsh `ok`, and the
   compiled manifest gives the site no hands and no boundary entry, as for
   every dsh reviewer. The position charter still requires the member to
   write its run-local position file under `.brokkr/state/design/positions/`
   and its own result file; dsh writes those through its own unconfined
   tools. That is a charter obligation, as for dsh reviewers' result files
   (0098), and the design check and later gates rule only committed
   artifacts. No equivalent sandbox enforcement is claimed.
5. **Limits.** The implement seat declares `{max_attempts: 3,
   timeout_seconds: 14400}` (careful: 7200 s); the review seat keeps
   careful's 14400 s. Intake inherits the `intake` office's 3600 s; the
   design seat keeps `self`'s `{3, 14400}`.

## The design seat and its validator

The pinned engine (brokkr 0.12.0, source base 731189f) has no agent-only
override of a sequence step. `override.seats` replaces a whole seat;
`override.cases` and `override.limits` apply only to selector cases and
limits. A seat's `./` command resolves against the layer that wrote it
(`expand_command`; composition records `seat_origin`), and a boxed exec
dispatch binds only that layer's root read-only at `/runtime/bundle`
(`hands_command`). A `..` component is refused by the pinned-script rule,
and a symlink out of the bound root would not resolve inside the box. This
is why 0057 changed `self`'s shared design crew instead of overriding it,
and why the existing recipes inherit the design seat whole.

`claude-careful` must replace the design crew to remove Codex, so it
redefines the design seat: `self`'s seat verbatim with its three agent
names changed. Its final `check` step then resolves against
`bundles/claude-careful/`, so the leaf carries byte-identical copies of
`self`'s `scripts/design-check.sh`, `scripts/speckit/check-prerequisites.sh`
and `scripts/speckit/common.sh` — the validator and the two helpers it
resolves beside itself. The compiled manifest pins them under the same
digests as `self`'s copies, the copy's own selftest passes, and a source
test refuses any byte drift or any other file in the leaf. The public shim
`scripts/design-check.sh` still dispatches to `self`'s copy; CI and the
verify seat keep running that one.

Without the copy the recipe still compiles — boxed exec scripts are not
admission-checked at compile time; a probe compile with no `scripts/`
directory exited 0 — but every design check would find no validator at
run time. The alternative, re-pointing `self`'s shared design crew, would
change the design route of all five existing recipes and was not directed.
Which of these the realm keeps is the operator's call; this record proposes
the copy.

## Known risks and costs

- **Haiku framing quality.** Haiku 5.5 is untested on this realm's intake
  charter. Watch for intake parks or framings the design panel rejects; the
  stated remedy is to move intake to Sonnet 5.5, a one-office change.
- **Sonnet implementation** is likewise untested here at 14400 s.
- **dsh design positions are untested live.** The base offices compiled
  with dsh positions, but no run in this journal has exercised a dsh
  position writing its position file under the current engine.
- **Spark capacity.** Spark serves at most six concurrent streams (0098's
  dated observation). With simplicity on Opus, GLM appears only on the
  tests review member: one Spark seat per run, so eight concurrent runs can
  still queue two tests members.
- **Claude concentration and cost.** Six of ten model seats — intake,
  simplicity, chief, implementation, correctness and independent review —
  depend on the host's Claude login and its rate limits; one Claude outage
  stops most of the run, with no substitution. Two Claude reviewers
  (Fable and Opus) are the most expensive members of the panel.
- **Flash concentration.** Official DeepSeek Flash carries the robustness
  position, security and guarantees: three seats on one paid provider and
  credential. The operator's review split “opus / fable / glm 5.3 / ds
  flash” names four models for five roles, so Flash takes two of them;
  correlated blind spots between security and guarantees are accepted.
  DeepSeek Pro remains forbidden.
- **Structural note.** A boxed Claude workspace seat holds only the MCP
  tools (`adapters/claude.json`, measurement U0 C10), so the Terra-style
  intake failure — the native filesystem is read-only and the result is not
  written — is not reachable on the Claude work seats; they write through
  the boxed workspace tool. This does not hold for the hands-less dsh
  position (rule 4).
- **Adapter digest.** Adding the alias changes `adapters/claude.json`
  (sha256 `11de5787…` → `ff1aeaf1…`) and so the claude `adapter_digest`
  in every new compile that hires a Claude office (for `codex-careful`'s
  independent member, `f02693d8…` → `b4f6dfc0…`). Manifests already
  pinned keep their recorded digest; a parked run resumed from a checkout
  that contains this change would compile against the new adapter. The
  frozen `.forge/bundles-0098` checkout is not touched.
- **Validator copy.** Two pinned copies must move together; a change to
  `self`'s validator now also requires the same bytes in the leaf, which
  the source test forces.

## Proof

`tests/delivery_recipes.rs` pins the recipe shape, every seat record
against careful's or `self`'s, all ten offices (base, single model,
effort or its absence, hands, concrete pin), the 14400 s implement and
review limits, the byte-identical validator and an extends-chain walk that
finds no Codex model in any composed office. One-fault mutants are refused
by the same comparison: inherited Codex design, Codex position, a Codex
model, Haiku 4.5, the 4.5 pin under `haiku55`, a moved historical `haiku`,
fallback chains, effort on GLM, dsh hands, a GLM robustness position, a
hands-less simplicity position, unmasked credentials, Fable independent,
Flash correctness, the 7200 s and 3600 s limits, a candidate-copy design
check, a copied policy, a Codex alias collision and DeepSeek Pro.
`tests/codex_review_routing.py` resolves `claude-careful` through the
pinned engine (`recipes show`, `agents show`) and compares the ten
effective rows — provider, chain, concrete pin, adapter route, office and
compiled effort, resolution status, charter, office limits, hands and
boundary — plus the composed provider set (`claude`, `dsh`, `exec`), every
concrete pin any site reads, the source limits and the validator digests;
everything outside the model offices equals careful field for field. CI
compiles `claude-careful` with the other five recipes.

Compilation and resolution prove structure, not provider access, model
quality or a completed review. A live run must still obtain five real
review results before the recipe is called delivered. No push, merge,
tag, publication, run, provider probe or decision acceptance follows.
