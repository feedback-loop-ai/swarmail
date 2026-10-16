# Plan — verify and review the fidelity batch: the record's completion

Technical context, constitution check, project structure, the amendment
set's mechanics, and the risks. The design (WHAT/WHY) is
[spec.md](spec.md); the ordered work is [tasks.md](tasks.md).

## Technical context

- **Tree:** HEAD `47fa0e2`, clean; 470 passed, 0 failed, 1 ignored
  (re-run first-hand this session, `cargo test --workspace --locked`).
  Single crate; production under `src/`, real-protocol suites under
  `tests/`; the delivery constitution is `bundles/self/` (brokkr-compiled).
- **The pass is a record repair.** The batch is committed and
  review-repaired; this run proves it. The only executable delta design
  commits is two count-neutral assertion tightenings inside existing
  tests (A4, A5). Everything else is sentences in owning artifacts.
- **The gates are the proof vehicle**, with the coverage gate's
  `TMPDIR=/home/valentin/scratch/cov-tmp` mandatory (the gate dies on
  `/tmp` tmpfs — run 11's predecessor stopped on exactly this). No mocks:
  every proving test speaks the real protocol against a real server.

## Constitution check

- **0001 (accept == stored):** untouched — the amendment set adds no
  await, no schema, no route; `Store::insert` stays synchronous. V4's
  inspection confirms no await entered the persist path.
- **0002 (subscribe before check):** untouched by this pass.
- **0003 (oldest-only pruning):** untouched in code; the receipt's cap
  bullet is corrected to state what 0003 already does (accept-and-evict).
- **0020 (test disciplines):** both test edits are assertions inside
  existing tests — no new test, no new seam, no cold path, no sleep, no
  wall clock, chaos untouched. Harness-name reservation and the coverage
  gate are unaffected: no file is added or renamed under `src/`.
- **Coverage:** no production line changes, so the measured denominator
  and numerator are unchanged; the gate must stay exact 100/100/100
  (missed == 0). A red gate here means an environment problem (TMPDIR),
  not a code problem — diagnose before touching anything.
- **Decisions:** none minted; 0031/0032/0033 stay `proposed` and indexed.
  The completion commit touches no decision file.
- **Dependencies:** none added; `cargo deny check licenses` must stay
  green with the existing allowlist.

## Project structure — files this pass touches

| File | Change |
|---|---|
| `specs/fidelity-prove-ship-receipt/spec.md` | A1 (cap-race bullet), A2 (hostile-DATA bullet), A3 (#56 pin clause) — the record's three false sentences corrected in place |
| `specs/fidelity-closures/spec.md` | A4b — AC-55.2's `total >= 1` tightened to `total == 1` |
| `tests/compat.rs` | A4a — the envelope leg's `>= 1` becomes `== 1` + `items.len() == 1` |
| `tests/ui.rs` | A5 — the guard-clause assertion added to `the_live_shell_guards_paints_with_a_token` |
| `specs/fidelity-verify-review-receipt/{spec,plan,tasks}.md` | this receipt — committed by design |

No other file. `src/` is untouched; `Cargo.toml`, `Cargo.lock`,
`deny.toml` untouched.

## The amendment set — mechanics

- **A1 + A2 + A3** are `edit` calls against the receipt of record, keyed
  to text anchors (the bullets are quoted in [spec.md](spec.md) §The
  amendment set), not line numbers — the three edits shift each other's
  lines. Each replacement is verified against code first-hand this
  session: store accepts and evicts (`src/store.rs` `pop_front` + the
  multi-evict catch-up loop; `insert_prunes_oldest_beyond_cap` at
  src/store.rs:864), DATA is allowance-bounded with `552 5.3.4` at the
  terminator (src/smtp.rs:962–:980) and `MAX_COMMAND_LINE = 4096`
  answered `500 5.5.2` (src/smtp.rs:54, :518), and `group_threads` runs
  per view request (src/api.rs:1014, src/feed.rs:54; 0018 §5 "the view
  is derived per request").
- **A4a** replaces one `assert!(... >= 1, ...)` with two exact asserts;
  the fixture delivers exactly one mail to a fresh server
  (`an_envelope_only_recipient_surfaces_in_every_compat_shape`,
  tests/compat.rs:1222), so `== 1` is the 0010-discipline truth, not a
  stricter guess.
- **A5** adds one `assert!(script.contains("seen !== paintSeq"), ...)`
  after the existing `paintSeq++` assertion — the guard exists at
  `src/static/ui.js:81` (`if (seen !== paintSeq) return;`), so the test
  goes green immediately and goes red exactly when the guard is deleted.
- **A4b** is one wording change in the design of record; the owning
  artifact is where the answer lives (the record-honesty precedent).

## Gates (the run's standing form)

```bash
source .brokkr/state/env.sh   # completes the git config pair; sets cargo PATH
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked          # 470 passed, 0 failed, 1 ignored
brokkr compile --bundle bundles/self
TMPDIR=/home/valentin/scratch/cov-tmp bash scripts/coverage-gate.sh   # exact 100/100/100
cargo deny check licenses
```

Per-issue re-proofs inside `cargo test` (V3): `--test api`, `--lib
extract` (#39); `--test persistence --test pop3 --lib persist` (#54);
`--test compat` (#55); `--test ui --lib threads` (#56); doc inspection at
HEAD (#57).

## Risks

- **The completion commit moves HEAD past `47fa0e2` mid-arc.** *Risk:* a
  phase reading V1 verbatim ("clean at `47fa0e2`") calls the tree dirty.
  *Mitigation:* V1's parenthetical reading is written into the receipt —
  clean at the completion commit, which is `47fa0e2` plus exactly the
  amendment set; AC-A.6 pins the commit's whole diff. The precedent
  (editing a committed spec mid-arc) is priced twice over in the record.
- **A seat re-finds the receipt's false sentences after refusal.** *Risk:*
  one more loop for the same five edits, plus a ship carrying a record
  that contradicts its own repair message. *Mitigation:* this is the risk
  of *not* doing the amendment set — the completion is the cheap path;
  the spec says so explicitly.
- **`total == 1` over-tightens if the fixture ever gains a mail.** *Risk:*
  a future leg added to `an_envelope_only_recipient_…` breaks the exact
  count. *Mitigation:* that is the point — 0010's exact-count discipline
  makes a future fixture change *declare itself*; the criterion (A4b)
  and the test (A4a) are tightened together so neither drifts.
- **The `seen !== paintSeq` assertion is a string pin on a shipped
  asset.** *Risk:* a future refactor of `ui.js` that preserves the
  guard's behavior but not its spelling goes red. *Mitigation:* the pin
  is on the guard clause itself — the load-bearing line — and the test's
  doc comment already records that the guard is asserted in the served
  asset; a renamed guard is a one-line test update with review provenance,
  the same shape as the existing `paintSeq`/`paintSeq++` pins.
- **The coverage gate dies on tmpfs.** *Risk:* an operator-shaped failure
  misread as a code failure. *Mitigation:* the `TMPDIR` is mandatory in
  every gate task and in V2; it is disk-backed (290G free).
- **Review holds on the `html_to_text` residual or the 0033 hole.**
  *Risk:* a hold re-opens design work in a confirmation pass.
  *Mitigation:* both residuals are recorded with their follow-ups named
  and their accepted costs priced (rulings 2 and 5); the 11d security
  verdict on `html_to_text` is shippable and on record; 0033's acceptance
  is the operator's by constitution. A hold is review's prerogative — the
  record is what makes it cheap if it comes.
- **The record's sentence edits drift from the code they describe.**
  *Risk:* the same disease, minted fresh by this pass. *Mitigation:*
  every replacement sentence was verified against the code first-hand
  before being written (the mechanics section cites the proof sites),
  and AC-A.1/A.2 tie each sentence to a grep-able code fact.
