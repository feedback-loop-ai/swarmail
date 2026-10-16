# Plan — the prove-ship receipt at `791f83d` (`fidelity-prove-ship-receipt`)

## Technical context

Run 11's design phase re-runs a third time, after the operator's
five-residual repair landed at `791f83d` (one tests MEDIUM — the
inline-adjacent html-leg pin — and four documentation LOWs). The framing
is explicit: this run proves and ships the committed tree; its edits are
not redone or re-argued. This pass is therefore a **record pass** like
its predecessor, with one addition the predecessor did not have: the
robustness seat named a failure mode no seat had named (one data file,
two servers), and it earns a ruling. No production code, no schema, no
route, no dependency is touched by this pass — the only executable delta
of the whole batch stays the repair's own pin test.

The repo layout this pass relies on:

- `specs/fidelity-closures/` — the design of record; amended in exactly
  two spots (the pin's name, the null-surface truth).
- `specs/fidelity-closures-prove-ship/` — the contract that binds
  verify, review and ship; amended in exactly five spots (the count
  twice, the pin, the Base tree, the surface truth).
- `specs/fidelity-record-honesty/` — the predecessor receipt; amended in
  exactly one spot (AC-1's live count, proof clause kept truthful).
- `specs/fidelity-prove-ship-receipt/` — this pass's synthesis (new).
- `docs/decisions/0033-one-server-per-data-file.md` — new, **proposed**,
  deliberately unimplemented; the operator's to accept and land.
- `docs/decisions/README.md` — **not** edited: the 0031/0032 index rows
  were added by the implement seat's #57 repair (`24df038`), not by the
  chief that filed the decisions; 0033's row lands with the operator's
  landing commit, keeping the index truthful against the tree at all
  times.
- `.brokkr/state/` — gitignored run-local evidence; the position files,
  the journal and the result contract stay uncommitted.
- `.brokkr/state/env.sh` — the run environment (cargo on PATH;
  `commit.gpgsign=false`, since pinentry cannot prompt here — the same
  condition the operator recorded on the unsigned repairs).

## Constitution check

- **0001 (accept == stored):** untouched — no `src/` change. 0033, if
  accepted, strengthens it by construction (one writer per file is what
  the lossless commit already assumes) and is specified to take its lock
  synchronously at open, before anything binds.
- **0002 (subscribe before check) / 0003 (oldest-only pruning):**
  untouched; no watcher or eviction vocabulary appears in this pass's
  diff. 0033's §1 cites 0003 as a *beneficiary* (prune plans assume one
  writer), not a change.
- **0005 / 0031 (extraction at ingest; sender-honest bodies):** untouched
  by the tree; the two null-surface edits bring the *specs* to the
  sentence the operator's own 0031 amendment already records.
- **0014 (per-mutation refusal) / 0018 / 0025 / 0029 / 0030:** untouched;
  the refusal posture is cited, not changed. 0033 adds a startup refusal
  in the same class (named error, non-zero exit, fails loudly at the
  boundary it enters).
- **0020 (test writing disciplines):** no new test lands in this pass;
  0033's §3 specifies its future test as inline, deterministic,
  in-process (flock is per-open-file-description), no sleep, no
  wall-clock — and states the existing second-connection tests are
  untouched, which is the reason the mechanism is the sidecar and not
  EXCLUSIVE.
- **0023 (off-worker commits):** untouched; the writer-stall proof
  family is explicitly protected by the mechanism ruling.
- **Gates:** the full suite was re-run this session at `791f83d`
  (470 passed, 0 failed, 1 ignored — AC-G.1's count proof);
  `cargo fmt --check` and `brokkr compile --bundle bundles/self` are
  re-run for the receipt; clippy and `cargo deny check licenses` are
  unaffected by a markdown-only pass and stand as recorded by the
  operator; the coverage gate is **not** re-run (contract red line 10:
  never twice without need; the TMPDIR rule is absolute).

## Project structure

The batch's shape is unchanged and re-verified: one declared dependency
(`percent-encoding`), three render sites, `REF_MAX` a named local, the
suite at 470. This pass's commit adds three synthesis artifacts and one
decision document, and edits the four owning spots of the run's record.

## Approach

1. **Verify first, mint nothing unverified.** The count, the pin, the
   compat mappings, the three render sites, the missing `locking_mode`,
   and the amendment sites were all checked first-hand this session
   before this spec was written (AC-T.1–T.3, Ruling 3's bill).
2. **Apply the amendment set** to the owning artifacts — five spots in
   the contract (spec + tasks), two in the design of record, one in the
   record-honesty receipt — exactly as §The amendment set lists, nothing
   else.
3. **File 0033** (`docs/decisions/0033-one-server-per-data-file.md`,
   status `proposed`) and reference it from the spec; do **not** touch
   the index, `src/`, `README.md`, `Cargo.toml`, or `deny.toml`.
4. **Commit the synthesis and the amendments as one commit** in the
   repo's style — imperative subject, a body that says why, never push.
   GPG signing is unavailable in this environment; the commit body
   records that honestly, as every unsigned commit in this arc has.
5. **Leave the evidence alone.** `.brokkr/state/` is gitignored: the
   position files, the journal and the result contract stay uncommitted.
6. **Hand off.** Verify proves the amended contract (its task list is
   unchanged except the count and the pin's name); review reads red
   lines 1–14 plus this pass's 15–16; ship closes hermetically with 0031,
   0032 and 0033 leaving review `proposed`, evidence attached,
   acceptance remaining the operator's act. No successor seat
   re-derives the design of record — the do-not-redo rule binds.

## Files touched

| File | Change |
|---|---|
| `specs/fidelity-closures-prove-ship/spec.md` | 4 edits: Base tree sentence, AC-G.1 470, AC-39.4 pin name + narrowing clause, AC-39.5 surface truth |
| `specs/fidelity-closures-prove-ship/tasks.md` | 1 edit: task 1 470 |
| `specs/fidelity-closures/spec.md` | 2 edits: AC-39.4 pin name, AC-39.6 surface truth |
| `specs/fidelity-record-honesty/spec.md` | 1 edit: AC-1 470 with a truthful proof clause |
| `specs/fidelity-prove-ship-receipt/spec.md` | new — this synthesis |
| `specs/fidelity-prove-ship-receipt/plan.md` | new — this plan |
| `specs/fidelity-prove-ship-receipt/tasks.md` | new — proof-paired checklist |
| `docs/decisions/0033-one-server-per-data-file.md` | new — proposed, unimplemented |

## Risks

- **Filing a decision whose code does not exist could read as the record
  promising what the tree does not do — the #57 disease in the machine's
  own artifact.** *Mitigation:* 0033 states its own status in the
  header ("not yet implemented"), the spec's failure-mode list and
  AC-S.1/S.2 carry the honest current-tree sentence, and the decision
  forbids landing the sentence without the refusal. Nothing in the tree
  or the record claims a refusal that is not there.
- **The two-servers ruling could be read as this run demanding new
  production work** — contradicting the framing's fence. *Mitigation:*
  the ruling is explicitly routed to the operator (the arc's every
  repair is an operator landing), the remaining phases are not asked to
  prove or build it, and AC-V.3 pins the commit shape so no seat
  "helpfully" lands it.
- **Editing committed artifacts mid-run reads as rewriting history.**
  *Mitigation:* the spec-kit return dialect makes the owning artifact
  the correct home for a corrected claim (the record-honesty pass's
  Ruling 2, applied twice now); the edits are additive record-honesty —
  no binding is weakened — and git history preserves the prior state.
- **The 470 count drifts again** if a later repair adds a test.
  *Mitigation:* the count is re-proved by verify's own run (contract
  task 1), which records the suite's actual summary; this pass verified
  it first-hand (470/0/1), not from the position file.
- **A successor seat reads AC-39.5/39.6's "empty string" and "fixes" the
  compat shapes to carry nulls** (a fourth render-site-class change
  behind the review's back). *Mitigation:* the amendment text cites the
  exact mapping lines and 0031 §2's "honest wires" sentence; red line 3
  (wire octets are the contract) plus review's line-level pass would
  hold it, and the change would need its own decision.
- **A successor seat implements 0033 the other way** (`PRAGMA
  locking_mode = EXCLUSIVE`) because the position named it. *Mitigation:*
  0033 §2 rules the mechanism with the full bill (the 0001 proof family
  at `tests/common/mod.rs::hold_write_lock`, `tests/lifecycle.rs:447/
  568/505/621/849`, the vandal at `src/persist.rs:341`); the refused
  alternative is named with its cost, so choosing it means re-opening
  the decision, not skimming the diff.
- **The sidecar lock file could surprise a future test** that globs the
  data directory. *Mitigation:* the decision names the file, its
  inertness, and the no-unlink rule; `stored_seqs`-style helpers remove
  the db sidecars, not glob the directory.
- **The commit cannot be GPG-signed** in this environment. *Mitigation:*
  recorded in the commit body exactly as the operator's own repairs
  recorded it; re-signing remains the operator's act on signed history.
- **The deterministic check rules on the newest `specs/<slug>/`
  directory.** *Mitigation:* this pass's directory is the newest at
  commit time, carries the acceptance-criteria heading, the risks
  section and the proof-paired checkboxes, and is committed clean before
  the check runs.
