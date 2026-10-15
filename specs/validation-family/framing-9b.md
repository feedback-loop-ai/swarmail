# Run 9b — intake framing: the validation family's protected re-entry (#46 #40 #38 #41 #47)

## Goal (one sentence)

Re-run the protected phases — verify → review → ship — on the repaired
tree at `e50a58b`: confirm the committed spec artifact set still holds,
prove the prior review's residuals are repaired by gates that can go red,
and put the validation family before a fresh review panel, without
re-arguing the ruled shape and without touching the code unless a defect
is found (any edit re-runs the full gate set).

## What this re-entry is (and is not)

- **Design is not re-opened.** The spec is committed
  (`specs/validation-family/`), decision 0029 is `proposed` and indexed
  (`docs/decisions/README.md` row 0029), and the deterministic design
  check passes on it. The ruled shape is not re-argued; the design
  phase's only job is to confirm the artifact contract still holds
  (criteria 1 below).
- **Implement is a confirmation, not a build.** The work exists
  (8bb0cb0 → 22039c4 → 4daa332 → e50a58b). Implement confirms
  tree == spec == repair; it touches nothing unless a defect is found.
- **Verify, review, ship run as usual.** Landing is the operator's: PR,
  both duplicate CI runs green, squash. Seats do not close issues or
  edit `agents/`, `adapters/`, `bundles/` (operator notes to
  `.forge/run9b-operator-edits.md`).

## Frozen-guarantee proximity (0001–0003)

- Sits near **0001** and **0002** but weakens neither. Every refusal in
  the repaired batch — the `arguments` container check, the `limit`
  parity, the wire-text validation — fires *before any effect*: before a
  subscribe (0002's refuse-before-subscribe posture) and before any
  insert (0001's accept == stored). `src/store.rs`/`src/persist.rs`
  stay at zero diff (0025/0029's boundary; 0029 §Preserves pins it).
- **0003** is untouched — no pruning path is in scope.
- No frozen guarantee is amended and no new decision is required: 0029
  (`proposed`, operator-mandated) is the authority on the ruled shape.
  The re-entry itself makes no semantic change, so it needs no decision
  document — that is exactly why it is `resolved` and not `blocked`.

## Acceptance criteria (checkable claims)

1. **The artifact contract still holds.** `specs/validation-family/`
   carries `spec.md`, `plan.md`, `tasks.md` in the shape
   `scripts/design-check.sh` enforces (acceptance section, checkbox
   task items, risks section); `docs/decisions/0029-*.md` exists with
   `Status: proposed` and a row in `docs/decisions/README.md`; the
   design phase's re-run of `scripts/design-check.sh` on its prompt
   files `designed` with no findings.
2. **The named HIGH repair is proven, not asserted.** `src/mcp.rs`
   validates the `arguments` container before any staging (the check
   sits beside the `params` pointer read); a `tools/call` whose
   `arguments` is an array, a scalar or an explicit `null` refuses
   `-32602` naming the field and its domain — and
   `swarmail_clear_inbox` with `"arguments": []` does **not** clear
   `default`. Red-able claim: `cargo test --test mcp
   a_malformed_arguments_container_refuses_before_any_effect` passes at
   HEAD and drives the absent-arguments arm too (`tests/mcp.rs`
   ~1459–1497). Deleting the check goes red.
3. **`limit` parity is restored on await/assert.** REST `await`/assert
   parse a supplied `limit` (and `since_ms`) exactly as `list` does;
   `openapi.json` documents the full filter set; a malformed supplied
   value refuses 400 with the shared sentence. `cargo test --test api`
   proves it against the real HTTP leg.
4. **The LOWs are closed in the tree, not in the journal.** The
   per-target bound-drop warn names the refused target; the decision
   index carries the 0029 row; README names the envelope's refusal
   codes; `llms.txt` states the webhook bound is per-URL and names the
   refused target. Checkable by reading the tree at HEAD; the review
   panel weights them.
5. **The ANSI guard (4daa332) is not regressed.** The serve startup
   line is the `smtp=`/`http=` contract and parses in any capture —
   plain when piped, ANSI only when the target stream can render it —
   in serve and mcp both. `cargo test --test binary` (the binary suite)
   and `tests/stdio.rs` hold the line.
6. **All house gates are green at HEAD**, including the exact
   coverage gate (100/100/100, missed == 0; the floor may rise, never
   fall) — commands below.
7. **Review rules on the tree at HEAD** against the ruled spec, the
   accepted decisions and proposed 0029. The prior panel's findings
   (run 9's journal) are repaired at `e50a58b`: the panel verifies the
   repairs and does **not** re-litigate the pre-repair tree. New
   findings rule as always; a security-hold is a hard stop.

## Verify gate

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
#   targeted: cargo test --test mcp --test stdio --test api \
#                  --test webhook --test smtp_edges --test binary
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh            # exact 100/100/100 — floor may rise, never fall
cargo deny check licenses
```

Verify seat additionally: the piped serve startup line must parse in any
capture (0029 §4, enforced by 4daa332 — do not regress the ANSI guard);
network denied; under the usual budget. SIGBUS → disk-backed TMPDIR.

## Blocked check

Nothing in the re-entry contradicts a frozen guarantee: it changes no
semantics, amends no decision, and the one decision in play (0029) is
already committed `proposed` — the lawful path the operator mandated.
**Resolved.**
