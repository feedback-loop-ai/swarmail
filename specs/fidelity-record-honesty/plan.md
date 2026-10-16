# Plan — record honesty at the repaired tree (`fidelity-record-honesty`)

## Technical context

Run 11's design phase re-runs after a review hold whose repair the operator
landed at `e245fae`. The tree is done: the five closures were implemented at
`24df038` (28 files, +1579/−195, 440→468 tests), the review held on one HIGH
(quadratic `parse_ref` scan) and one MEDIUM (the batch-delete 503's
atomicity lie), and the repair fixed exactly those — a named 40-character
window inside `parse_ref` and one truth-telling openapi line. This pass is
a **record pass**: no production code, no schema, no route, no dependency.
The only files it touches are markdown under `specs/`.

The repo layout this pass relies on:

- `specs/fidelity-closures/` — the design of record (do not touch).
- `specs/fidelity-closures-prove-ship/` — the contract that binds verify,
  review and ship; amended here in four places (see
  [spec.md](spec.md) §The amendments).
- `specs/fidelity-record-honesty/` — this pass's synthesis (new).
- `.brokkr/state/` — gitignored run-local evidence; the position files and
  operator notes stay uncommitted.
- `.brokkr/state/env.sh` — the run environment (cargo on PATH;
  `commit.gpgsign=false`, since pinentry cannot prompt here — the same
  condition the operator recorded on the unsigned `e245fae`).

## Constitution check

- **0001 (accept == stored):** untouched — no `src/` change; the repair
  this pass records only *strengthened* its availability (bounded scan,
  no new await).
- **0002 (subscribe before check) / 0003 (oldest-only pruning):**
  untouched; no watcher or eviction vocabulary appears in the diff.
- **0005 (extraction at ingest):** untouched; the window lives inside the
  existing ingest-side parse.
- **0014 (per-mutation refusal):** the record is brought *to* the decision
  (the 503 prose), not past it — AC-4.
- **0020 (test writing disciplines):** the pin test's placement and shape
  are re-verified (inline, output assertion, no wall-clock validity) —
  AC-2; Rulings 3 and 4 refuse the two test forms the discipline forbids.
- **0022 / 0030 §3:** untouched; the window and the cap are two local
  bounds, each in its owner.
- **Gates:** `cargo test --workspace --locked` was re-run this session for
  the count proof (AC-1: 469 passed, 0 failed); fmt/clippy/brokkr-compile
  are unaffected by a markdown-only pass and stand as recorded at
  `e245fae`; the coverage gate is **not** re-run (the contract's red line
  10: never twice without need; the TMPDIR rule is absolute); the license
  gate is unaffected by markdown.

## Approach

1. **Amend the owning contract** (spec.md AC-G.1 468→469, AC-39.2 gains
   the pin proof site, Base line notes `e245fae`; tasks.md task 1
   468→469). Nothing else in the contract changes.
2. **Commit the synthesis** (`specs/fidelity-record-honesty/{spec,plan,tasks}.md`)
   and the contract amendments as **one commit** in the repo's style —
   imperative subject, a body that says why, no push. GPG signing is
   unavailable in this environment (`env.sh` pins `commit.gpgsign=false`);
   the commit body records that honestly, as `e245fae` did.
3. **Leave the evidence alone.** `.brokkr/state/` is gitignored: the
   position files, the journal and the result contract stay uncommitted.
4. **Hand off.** Verify proves the amended contract (its task list is
   unchanged except the count); review reads the contract's red lines 1–11
   plus this pass's 12–14; ship closes hermetically. No successor seat
   re-derives the design of record — the do-not-redo rule binds.

## Files touched

| File | Change |
|---|---|
| `specs/fidelity-closures-prove-ship/spec.md` | 3 edits: AC-G.1 469, AC-39.2 pin proof site, Base sentence |
| `specs/fidelity-closures-prove-ship/tasks.md` | 1 edit: task 1 469 |
| `specs/fidelity-record-honesty/spec.md` | new — this synthesis |
| `specs/fidelity-record-honesty/plan.md` | new — this plan |
| `specs/fidelity-record-honesty/tasks.md` | new — proof-paired checklist |

## Risks

- **Editing a committed contract mid-run reads as rewriting history.**
  *Mitigation:* the spec-kit return dialect makes the owning artifact the
  correct home for a corrected claim; the edits are additive
  record-honesty (a count, a proof site, a base note) — no binding is
  weakened, and git history preserves the prior state. The commit message
  names the drift it closes.
- **The synthesis spec is mistaken for a competing design of record.**
  *Mitigation:* Ruling 1 states it mints no vocabulary and defers
  explicitly; the contract's Base sentence points review at the added red
  lines rather than replacing them.
- **The 469 count drifts again** if a later seat adds or removes a test.
  *Mitigation:* the count is re-proved by verify's own run (contract task
  1), which records the suite's actual summary; this pass's AC-1 was
  verified first-hand, not inherited from the position file.
- **A successor seat "helpfully" widens the window** (config knob, HTML5
  table, module constant) or re-adds a parse timer. *Mitigation:* red
  line 12 names all three forms; the design of record priced the window
  as a proof, not a knob.
- **A successor seat routes restore through `insert`** (e.g. to reuse
  fan-out) and silently re-fires watchers on every restart. *Mitigation:*
  red line 13 makes it a review finding; the recorded mechanism
  (`push_back` restore before bind) is the contract's own text.
- **The commit cannot be GPG-signed** in this environment. *Mitigation:*
  recorded in the commit body exactly as the operator's own repair
  recorded it; re-signing remains the operator's act on signed history.
- **The deterministic check rules on the newest `specs/<slug>/`
  directory.** *Mitigation:* this pass's directory is the newest at
  commit time, carries the acceptance/risks/checkbox structure, and is
  committed clean before the check runs.
