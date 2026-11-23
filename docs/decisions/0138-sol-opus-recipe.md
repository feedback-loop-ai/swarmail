# 0138 — Sol implements and Opus reviews in `sol-opus`

- **Status:** accepted (operator, 2026-11-23, given in chat).
- **Date:** 2026-11-23
- **Scope:** one additional recipe under [0099](0099-claude-careful-recipe.md).
  Amends [0100](0100-one-brokkr-folder.md) §2 to select the new default.
  The new Sol and Opus offices use high effort in this lane.
- **Source:** operator directions and rollout choice, 2026-11-23 (one misspelling corrected in square brackets).

  > can we swap sol 6.2 as implementer everywhere, here, ripper, and agents-server, and opus 5.5 as reviewer?

  > so [commission] next batches of work that way, as I see most of the tokens go to opus, and we have ample limits on codex tonight

  > and swarmail and mrunkam

  > Open a PR in each

## Context

The operator has available Codex capacity and wants the next batches to use
Sol 6.2 for implementation and Opus 5.5 for review. Swarmail's default lane
was the zero-Codex `claude-careful`, which assigns implementation to Sonnet
5.5 and review to Fable, Flash, GLM and Opus. The new lane uses the same
careful process with the requested crew.

## Ruling

1. **`.brokkr/recipes/sol-opus` extends `claude-careful`.** Its overrides
   are the description, `implement` and `review`. The two overridden seats
   copy the parent's records with the office assignments below. Each seat
   retains its 14400 s timeout and three attempts. Intake, design, policy,
   verify and ship are inherited, including protected review. The review
   aggregate requires five separate gate-class results, and the security
   member alone owns `security-hold`. Each model chain has one entry.

   | Seat | Office | Adapter | Model → concrete | Effort | Hands |
   |---|---|---|---|---|---|
   | Intake | `claude-intake` | claude | `haiku55` → `claude-haiku-5-5` | high | workspace |
   | Design: simplicity | `claude-position-simplicity` | claude | `opus` → `claude-opus-5-5` | medium | workspace |
   | Design: robustness | `dsh-position-robustness` | dsh | `flash` → `deepseek-flash` (`deepseek-official`) | high | none |
   | Design: chief | `claude-chief-architect` | claude | `fable` → `claude-fable-5-2` | medium | workspace |
   | Design: check | `claude-careful`'s copy of self's validator | exec | — | — | offline workspace |
   | Implement | `sol-implementer` | codex | `sol` → `gpt-6.2-sol` | high | workspace |
   | Review: correctness | `opus-review-correctness` | claude | `opus` → `claude-opus-5-5` | high | none (read-only) |
   | Review: security | `opus-review-security` | claude | `opus` → `claude-opus-5-5` | high | none (read-only) |
   | Review: tests | `opus-review-tests` | claude | `opus` → `claude-opus-5-5` | high | none (read-only) |
   | Review: guarantees | `opus-review-guarantees` | claude | `opus` → `claude-opus-5-5` | high | none (read-only) |
   | Review: independent | `claude-review-independent` | claude | `opus` → `claude-opus-5-5` | medium | none (read-only) |

2. **Offices.** `sol-implementer` overlays `implementer` and copies
   `codex-careful-implementer`'s `hands` and `replaces` declarations. The
   Codex adapter has no tool-name restriction, so the native shell is
   read-only and writes go through the boxed workspace tool. The workspace
   allows network access, masks `credentials` and `credentials.toml` in the
   Cargo overlay and mounts `~/.rustup` read-only. Each new reviewer directly
   overlays its corresponding `review-*` base and inherits its charter.
3. **Default.** `.brokkr/bundle.json` extends `sol-opus` by name. A run with
   no `--recipe` selects that lane. `claude-careful` and the other shelf
   recipes remain available by name.

## The design seat and its validator

The inherited design seat retains `claude-careful` as its script origin.
The boxed check resolves the validator and its two helpers from the
byte-identical copies described in 0099. The `sol-opus` directory carries
only `bundle.json`.

## Known risks and costs

All five reviewers share the Claude login, provider capacity and model.
A Claude outage can block the full panel, and the separate charters can
still yield correlated misses. Each required verdict still comes from a
separate review invocation.

## Proof

`tests/delivery_recipes.rs` pins the folder default, the two seat overrides
against `claude-careful`, the office overlays and model aliases, and the
contents of the recipe directory. `tests/codex_review_routing.py` checks
the resolved chain, office, concrete pin, effort and hands with the recorded
engine. The verify and CI recipe sweeps include `sol-opus` through the shelf
glob. These checks cover route structure. Provider access and review outcomes
require a live delivery.
