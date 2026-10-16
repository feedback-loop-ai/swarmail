# Plan — the protected re-entry (HOW)

- **Run:** run 10b · **Feature slug:** `protected-reentry` · **Base:**
  `a4ff22c` (clean tree). Companion to
  [spec.md](spec.md) (WHAT/WHY) and
  [tasks.md](tasks.md) (the ordered, test-paired tasks).
- **Predecessor:** [off-worker-parses-loopback-posture/plan.md](../off-worker-parses-loopback-posture/plan.md)
  remains the ruled shape's HOW; this plan governs only the re-entry's
  phases and its defect path.

## Technical context

Run 10b re-runs the protected phases over a tree whose family is already
implemented (f9cde28) and repaired (a4ff22c). No phase here designs
machinery; each phase consumes the last one's verdict:

- **implement** — a confirmation seat. It reads the tree against the
  ruled spec's acceptance criteria and the repair's diff, re-runs the
  named pins, and writes nothing unless a confirmation goes red. The
  defect path (spec.md, ruled conflict #2) is the only lawful write:
  smallest local fix, its wire pin in the same commit, the full gate set
  re-run. A decision document is owed only if a defect touches a frozen
  guarantee or the 0030 posture itself — otherwise the repair is a
  finding's consequence, not a ruling's amendment.
- **verify** — hermetic exec seat; the box denies network. The six gates,
  the targeted red-green filters below, and the framing's two
  environment notes (SIGBUS → disk-backed `TMPDIR`; the piped startup
  line must parse in any capture). No bench re-run: `benches/read_stall.rs`
  records measured numbers for the operator to rule on (0023's
  discipline), and re-measuring is not an acceptance line of this run.
- **review** — correctness + security members judge HEAD. Their inputs
  are the ruled spec, the amended 0030 (`proposed`), and this run's
  weighting section; their findings rule as always, and the review gate
  is constitutionally protected — a security hold stops the run.
- **ship** — the operator's: PR, both duplicate CI runs green, squash.
  Nothing in this plan touches `agents/`, `adapters/`, `bundles/`.

## Constitution check — the frozen guarantees, re-verified in code, most recently at HEAD `95d2823`

- **0001 (accept == stored):** the DATA terminator consumes the envelope
  before any outcome (`src/smtp.rs:968-976`) and the 250 is written only
  after the commit outcome is known (`:1033-1050`); `Store::insert` stays
  synchronous; the offloaded parse runs *before* the mutation boundary
  and no commit path gained a branch. The `451 4.3.0 storage commit
  failed` arm (`:1045`) is the commit-failure arm (0012), not a
  parse-panic arm — the parse-panic residual is 0030 §6's, by 0023 §5's
  panic symmetry (the session task unwinds; the peer sees the socket
  close). Two panel sentences ride this bullet (spec § Adopted at the
  re-entry panel): a crash between the commit and the 250 stores the
  mail and answers nothing — the peer's retry duplicates and dedup is
  the sender's (message-id) job, because `250 ⇒ queryable` is
  one-directional by design; and the tree's absence of `fsync` is
  correct only because `src/persist.rs:86-87` pins
  `journal_mode = WAL` with `synchronous = FULL` — accept==stored is
  only as durable as that pragma, and anyone touching `persist.rs`
  re-derives 0001's disk leg or refuses the edit.
- **0002 (await subscribes before it checks):** `wait_for` registers its
  watcher before it scans; the `post_subscribe_gate` seam keeps the
  subscribe→scan window interleavable under test, including the `Closed`
  re-subscribe arm. Untouched by the family.
- **0003 (oldest-only pruning):** overflow is derived at PLAN from a peek
  and applied at APPLY with `pop_front` inside the mutation mutex; the
  inserting mail is never its own prune candidate. `src/persist.rs` at
  zero diff for the whole family; `src/store.rs` at the one-word
  visibility change.
- **0005 (extraction at ingest):** extraction runs inside the offloaded
  block, still before the 250 — never at query time.
- **0029 §4 (the startup line's law):** ANSI only when the target stream
  renders it; `tests/binary.rs`'s `startup_addrs` (`:50-54`) is the
  parse-in-any-capture pin.

## Project structure

No new files under `src/` and no new production path. The spec dir gains
two committed artifacts — the intake framing copies `framing-10b.md` and
`framing-10b-successor.md`, each riding its synthesis commit per the
validation-family precedent (the design-check probe counts untracked
files in the spec dir). The only
production files this run may touch, each only through its defect path,
and each with the pin that proves it:

| File | Defect it could owe | Proving pin |
|---|---|---|
| `src/api.rs` (the guard, `:307-392`) | An admitted non-loopback spelling; a route reachable before the middleware; an inert-arm inversion | The three rebind legs in `tests/api.rs` (`:1295`, `:1349`, `:1431`) |
| `src/compat.rs` (`search_hog`, `:247-275`) | A kind that rides through with different semantics | `mailhog_v2_lists_and_searches_with_upstream_shapes` (`tests/compat.rs:536-540`) |
| `src/store.rs` / `src/persist.rs` | Any commit-path change (none expected — the boundary is pinned) | AC1.3's diffs + `cargo test --workspace` |
| `src/static/openapi.json`, `README.md`, `src/static/llms.txt` | A claim the wire cannot observe; a drift from the parser | AC4.2 + the served schema pinned to the parser |
| `docs/decisions/0030-*.md` | Nothing now — folds at acceptance | AC1.2 |

Everything else is out of scope: no new module, no new dependency, no new
seam, no new env var, no `[[bench]]`, no confirmation-test twin.

## Research / data model / contracts / quickstart

None new — the re-entry designs no data model and no contract. The
authoritative text is 0030 §1–§6 (the posture, the 250 cap, the fold,
the SSRF disclosure, the refused residuals) and the ruled spec's ACs.
The re-entry's quickstart is the gate set:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh     # exact 100/100/100; SIGBUS → disk-backed TMPDIR
cargo deny check licenses
```

Targeted red-green filters for the repair's own surface:
`cargo test --test api --test compat --test binary --test config_cli`,
plus `cargo test --lib` for the awaits-gauge inline pins.

## The review weighting (what review reads before judging)

1. The four-sentence ledger (spec.md § The re-entry's ledger): exotic
   spellings refuse closed; the guard is keyed to the actually-bound
   port and any future bind change must carry that fact through; the
   loopback default is a network boundary, not a tenant boundary; the
   Origin parse's leniency cannot widen and why. Plus the two adopted
   0001-durability sentences (spec.md § Adopted at the re-entry panel):
   the commit→250 window costs a duplicate on retry and dedup is the
   sender's; the WAL/FULL pragma is 0001's disk leg and any
   `persist.rs` edit re-derives it or is refused. Plus the third
   panel's record sentences (spec.md § Adopted at the third panel): a
   `250` is true when written — how long it stays true is the
   operator's cap; a poisoned boundary dies loudly, never quietly
   retries; and the lazy-registration rule — arm every
   crash/termination surface before the wire starts answering — read
   for new signals, listeners and spawns the way 0001 is read. Plus
   the fourth panel's items (spec.md § Adopted at the fourth panel):
   the corrected webhook delivery sentence (retried within the budget;
   losses counted; never replayed across a restart), the
   serialized-boundary-as-throughput-ceiling sentence, and the priced
   400-arm wire residual — review's to take as a finding, the ~8-line
   `tests/compat.rs` test pre-priced beside it
   (`GET /api/v1/search?query=` + 250 000 `k`s → `400`).
2. The 451/parse-arm distinction (`smtp.rs:1045` is 0012's
   commit-failure arm; the parse-panic residual is 0030 §6's, trigger
   named).
3. The no-re-litigation rule: the pre-repair findings are repaired with
   their own tests; re-opening one is churn with a gate-set price, not a
   finding. New findings rule as always.
4. Every endorsed claim is one the wire can observe (the awaits-gauge
   repair is the template): assert the refusal that can be proven, drop
   the metric leg that cannot. Landing-note phrasing about the loopback
   default scopes it to network-reachable clients; cross-session
   isolation stays 0004's story.

## Risks

1. **Scope creep in a security re-entry** — every seat's temptation to
   also harden (a refusal counter, an allowlist knob, CSP headers, an
   SMTP-plane extension). *Mitigation:* AC5's zero-delta contract and
   the ruled refusal — the ledger is sentences, not machinery; review
   refuses unpriced hardening, and a defect-driven fix obeys the house
   order (delete the unnecessary path, one owner, restore proven
   behavior, only then the smallest local guard) with its pin.
2. **The edit price misread as license** — every edit re-runs the six
   gates; a cosmetic rename burns the budget for zero observable.
   *Mitigation:* the two declined renames are recorded in the spec
   (AC5.4) and re-declined here; edits are defect-driven only.
3. **The design-check verdict is env- and artifact-dependent** — the
   committed-spec probe swallows git errors (`git status --porcelain ...
   2>/dev/null`), so a broken git env reads as "no uncommitted changes"
   (run 9b's recorded finding; still present at HEAD); and the same
   probe counts uncommitted files in the spec dir, which tripped three
   times on the intake framing copies — untracked at the first
   synthesis, modified at the second (the intake's own update
   reflecting `81409d4`), and untracked again as the successor copy
   (`framing-10b-successor.md`) at this one. *Mitigation:* run under a
   working git env (`GIT_CONFIG_KEY_0` exported — needed a fourth time
   this session) and commit each framing copy per the
   validation-family precedent (the successor copy rides this
   synthesis commit); the env-vacuity finding
   stays an operator note (`.brokkr/state/run10b-operator-edits.md`),
   never a mid-judgment edit to the checker that judges the phase. A
   design seat does not edit its own judge.
4. **Review re-litigates the pre-repair tree** — the cheapest way to
   look rigorous while burning a gate set on settled findings.
   *Mitigation:* weighting item 3; the review inputs cite the repair
   commit and its pins, and the framing's rule is restated in spec.md's
   AC4.
5. **A new finding lands in the guard mid-re-entry** — e.g. an input
   shape that slips `authority_is_loopback_of` *into* an admission.
   *Mitigation:* the defect path is pre-priced (fix + wire pin + full
   gates); the failing direction is named; a decision is owed only for a
   posture-level change. Fail-closed margins (trim, ASCII lowercase,
   bracket handling) already refuse every deviation the parser cannot
   prove.
6. **False confirmation** — a re-entry that "passes" because nothing was
   actually exercised. *Mitigation:* every task in tasks.md is paired
   with the named pin that would go red if its fact broke; AC2 pins are
   executed by this run's verify, not cited from memory.
7. **Signing** — the seat cannot reach the operator's key (pinentry
   times out) and the run env pins `commit.gpgsign=false`; the design
   commit goes unsigned like the operator's own a4ff22c and the run-9b
   precedent c5f2a1f, recorded in its body. Landing re-signs at the
   operator's squash.
