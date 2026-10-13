# 0017 — The codex review seat's delivery sandbox

- **Status:** accepted (operator delivery repair, 2026-10-13 — the same
  standing as 0013's panel directive; the human operator may revert via
  this ADR)
- **Date:** 2026-10-13
- **Preserves:** [0013](0013-four-model-review-panel.md) — the panel mapping
  stands unchanged, including the `glm-flash` fallback; this repairs one
  member's delivery, not its verdict or its lens. No frozen guarantee is
  touched.

## Context

Run `close-the-p1-persistence-finding-15dd8e6b` parked
`awaiting_operator` in review: the `guarantees` member — codex adapter,
`gpt-6-astra` per 0013 — failed three attempts with
`seat wrote no result file (the result contract was not met)`. The rollout
shows the opposite of a refusal to work: 44 work checkpoints, ~523k input
tokens, and a complete verdict delivered as the seat's final message. What
the seat could not do is write `.forge/results/<effect>-guarantees.json`.

The mechanism: the office declares no `tools.sandbox`, the seat declares no
hands, and a review seat is a gate under the `harness` boundary — so the
composed cold argv carried no `-s` class and codex-cli filled the gap with
its own default, `read-only` (the session's own `<permissions
instructions>`: "`sandbox_mode` is `read-only`"). `adapters/codex.json`
already recorded the measurement this predicts: "the workspace is writable
only under workspace-write".

Two in-repo shapes were evaluated and neither lands today:

- **`tools.sandbox: "workspace-write"` on the office** — refused at
  compile: a sandbox class without hands has no engine path that expresses
  it ("recorded and not delivered", design D5.3).
- **`hands` on the office** (the read-only gate fragment with the
  last-message door) — compiles only for providers that can put hands in
  the box; dsh declares hands unsupported, so the office's `glm-flash`
  fallback dies at compile. The fallback is 0013's own term.

## Decision

1. **The delivery environment grants the reach.** A swarmail delivery
   launch sources the dsh credentials, exports `PATH` with the staged
   brokkr, and exports `CODEX_HOME` at a scoped home
   (`.forge/codex-home`, gitignored) whose `config.toml` sets
   `sandbox_mode = "workspace-write"` for codex spawns. The seat's own
   default (read-only) is a harness default, not a constitution ruling;
   the operator's launch environment is where harness defaults belong.
2. **The office's charter states the discipline, not the mechanism.**
   `charters/review-guarantees.md` tells the seat the one write its
   delivery needs is its own result file and nothing else in the tree is
   touched. The class a future engine path can express (decision 0065
   slice one's lowering) replaces the environment grant without touching
   the charter's conduct clauses.
3. **A pinned run is repaired at the operator layer, never by editing
   what it pins.** Resume refuses digest drift; the parked run was
   re-armed with `brokkr operator retry` and resumed under the scoped
   home. The repair is empirical: the retried member delivered, the panel
   completed (`REVIEW-RESIDUAL-OK`), and the run closed `SHIP-COMPLETE`
   at `f2b12d0` with a clean worktree.

## Consequences

- The codex-routed member now delivers under the delivery recipe; the
  other three members (dsh and claude routes) were never affected.
- A reviewer holds tree-write reach it is chartered not to use. The
  journal records every seat checkpoint, the review notes the verdict's
  `fixes_applied: false`, and the ship gate verifies the clean close-out —
  an abusing seat is detectable after the fact.
- A fresh clone that skips the recipe re-learns this failure the way this
  run did; the ADR and the recipe are the pointer. When the engine gains a
  path that expresses a class without hands, the office should declare it
  and the `CODEX_HOME` grant should be dropped.
