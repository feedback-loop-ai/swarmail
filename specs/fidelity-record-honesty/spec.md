# Spec — record honesty at the repaired tree (run 11 record-honesty pass: issues #39, #54, #55, #56, #57)

- **Run:** run 11 (`brokkr-state-run11-feature-txt-52ac38b0`) · **Phase:**
  design — the chief architect synthesizing the robustness and simplicity
  positions · **Feature slug:** `fidelity-record-honesty`
- **Issues:** [#39](https://github.com/feedback-loop-ai/swarmail/issues/39)
  (extraction fidelity, P1), [#54](https://github.com/feedback-loop-ai/swarmail/issues/54)
  (restored state matches live state), [#55](https://github.com/feedback-loop-ai/swarmail/issues/55)
  (Mailpit/MailHog compat fidelity), [#56](https://github.com/feedback-loop-ai/swarmail/issues/56)
  (thread normalization + web UI), [#57](https://github.com/feedback-loop-ai/swarmail/issues/57)
  (documentation and comment drift).
- **Base:** HEAD `e245fae` — the review-hold repair ("Bound the
  entity-reference scan and tell the batch-delete truth"), operator-committed
  on top of the prove-and-ship contract `ffe97b4` and the implement commit
  `24df038`. Verified first-hand this session at `e245fae`: the full suite
  reports **469 passed, 0 failed** (the one ignored test is `tests/rate.rs`'s
  opt-in throughput smoke), and the repair is exactly
  `src/extract.rs` (+24/−1: a named local `const REF_MAX: usize = 40` window
  inside the owning `parse_ref`, plus the inline bounded-work pin test) and
  `src/static/openapi.json` (+1/−1: the batch-delete 503 now states 0014's
  per-mutation refusal). `e245fae` is unsigned; its own message records why
  (pinentry cannot prompt in this environment) — the acceptance is the
  operator's.
- **Design of record:** [specs/fidelity-closures/spec.md](../fidelity-closures/spec.md)
  (committed at `2cfe08c`, amended at `24df038`) — **unchanged by this
  pass**. Every mechanism it pinned survives the repair verbatim.
- **Contract of record:** [specs/fidelity-closures-prove-ship/spec.md](../fidelity-closures-prove-ship/spec.md)
  with its [plan](../fidelity-closures-prove-ship/plan.md) and
  [tasks](../fidelity-closures-prove-ship/tasks.md) — **amended by this
  pass** with exactly the record-honesty edits listed in §The amendments;
  its red lines 1–11 stand and this pass adds 12–14 (§Red lines).
- **Decisions:** [0031](../../docs/decisions/0031-sender-honest-stored-bodies.md)
  and [0032](../../docs/decisions/0032-emptied-inboxes-are-removed.md) —
  both still **proposed**, implemented by the tree, awaiting the operator.
  **No new decision is minted by this pass and none is needed:** the repair
  amends no standing ruling — 0014 already refused batch atomicity (the 503
  line now *says* so), and bounding the scan makes 0001's accept path more
  reliably prompt without entering an await anywhere.

## The problem

Run 11b's review held on one HIGH and one MEDIUM: `parse_ref`'s
window-less `s[1..].find(';')` rescanned to the end of the URL match for
every `&` — quadratic on a hostile body, pinning the shared blocking pool
every store mutation shares — and the Mailpit batch-delete 503 promised a
batch atomicity accepted decision 0014 never granted. The operator repaired
both at `e245fae`, and the phase machine re-ran design. The question this
pass answers: **what does the design phase owe a repair it is forbidden to
re-litigate?**

The answer, from both seats, is **record honesty — nothing else**. The
repair made the code match its own documented contract (`src/extract.rs`'s
module doc claimed "total and linear" all along) and made the doc match the
code (the 503). No new file, dependency, route, decision, helper or type;
the only new production name is one local `const`. The design of record and
the prove-and-ship contract both stand as written — except for one drift
the repair created *inside the contract itself*: it was committed at
`ffe97b4`, before the repair, and says the suite is **468** tests
(AC-G.1, tasks.md task 1) while the repaired tree carries **469**; its
AC-39.2 totality demonstration does not name the pin the HIGH's repair
added; and its Base line names a tree that is no longer the one the
remaining phases prove. The contract binds verify — a contract that
disagrees with the tree is precisely the #57 drift this run exists to
close, sitting in the very artifact that binds proof. This pass repairs
the record and re-binds the phases that remain; it mints no design
vocabulary, no criterion that was not already binding, and no production
change.

## The positions, ruled

Both seats returned **pass** and agree on the skeleton: the repair is
record-honesty rather than design, the landed shapes all survive, and the
cheapest remaining work is proof. Four things needed a chief; each is
ruled, not averaged, with the losing side's cost named.

**Ruling 1 — the substance of "no third spec" is adopted; the letter is
refused.** Simplicity §6.1: "no third spec or synthesis document … the
chief's §4 amendments are edits to the second, not a new document." The
substance is adopted wholesale: nothing below re-derives, extends or
"improves" the landed design; there is no new vocabulary, no new file in
`src/`, no new dependency, no new route, no new decision. The letter is
refused because the phase machine's contract requires the synthesis
committed as this attempt's artifacts and the deterministic check rules on
the newest `specs/<slug>/` directory alone — a synthesis that lives only in
`.brokkr/state` is run-local evidence, not the record (the same reasoning
as the contract's own Ruling 1, one pass later). **Cost of losing the
letter:** one small committed spec — this one. **Cost of the alternative:**
the attempt fails its own deterministic check, or verify and review are
bound by an uncommitted document the record cannot see.

**Ruling 2 — the drift is repaired inside the owning artifact, not
restated here.** Adopted: simplicity §4's amendment set, applied to
[specs/fidelity-closures-prove-ship/](../fidelity-closures-prove-ship/spec.md)
itself (§The amendments below). Rejected: restating corrected numbers in
this spec and leaving the contract stale — two records disagreeing about
the same demonstrations is exactly the failure #57 closes, and verify
proves against the contract, not against a successor's summary. **Cost of
the adopted path:** editing a committed artifact mid-run, which the
spec-kit return dialect ("replace obsolete text, and leave no
contradiction") makes correct — the owning artifact is where the answer
lives, and git history preserves the prior state. **Cost of the rejected
path:** permanent drift between the two committed artifacts of one run.

**Ruling 3 — robustness's wire-shaped hostile-body *latency* test is
refused; the pin test is the bounded-work detector.** Robustness #39:
"a body at the 0022 cap built of pathological `&` runs … still produces
`250` with bounded latency … if that test cannot be written without a
sleep, the implementation is the bug." Refused in the letter, adopted in
the substance. A wire test asserting "bounded latency" asserts a duration —
0020's no-wall-clock-validity discipline and the contract's red line 3
forbid exactly that; and a wire test asserting only that the `250`
eventually arrives cannot detect a quadratic path except by suite-wide
timeout — the slowest, least deterministic detector available. The honest
detector already exists and is the repair's own pin:
`src/extract.rs::an_ampersand_run_decodes_in_bounded_work` — a
500,000-`&` URL asserting **exact output** (`assert_eq!` on the extracted
links), under which a quadratic rescan hangs the suite. The wire leg that
carries a body to extraction is proven by AC-39.1's real-SMTP test; 0022
bounds the body before parse; extraction is called from one ingest site.
**Cost of losing the literal demand:** a regression introduced in the SMTP
framing *around* the extract call — not in the scan itself — would not
hang the suite; the residual surface is the plumbing to one call site,
which is review's line-level job, not a new test's.

**Ruling 4 — "restored mail must not re-fire watchers" is adopted as a red
line, refused as a suite test.** Robustness #54 demands the test. The tree
already guarantees the property by construction: `Store::open` restores
the full state before the server accepts a connection and deliberately
bypasses `insert` — the comment at `src/store.rs` ("restored mail must not
re-fire watchers or webhooks on every restart", the `push_back` loop) is
the recorded mechanism. A suite test of the negative ("the sink received
nothing") needs a wall-clock quiet period to observe an absence — 0020
refuses sleep-as-sync and the vacuity it hides, which is why no such test
exists. **Adopted:** the guarantee as red line 13 with review binding.
**Cost of refusing the test:** the no-refire property is proven by
inspection, not execution — an edit that routed restore through `insert`
or moved it after bind would fail no test; the red line makes it a review
finding instead.

**Ruling 5 — everything else both positions bind is already binding; no
new ruling is minted.** Robustness's ledger (commit-before-visibility,
try-shaped bounds everywhere, the per-mutation refusal told truthfully,
envelope-truth recipient surfaces, the `IDs` alias narrowed to spelling
under 0011, sequenced UI paint, the CSP that actually binds the preview,
`to_lowercase` rather than a folding dependency, `deny.toml` mirroring the
charter modulo the recorded CDLA allowance) and simplicity's red lines
(`REF_MAX` stays a named local, no parse timer, no HTML5 table through the
window's door, the gate stays exact and disk-backed) are carried by the
contract's red lines 1–11 verbatim — robustness's own #55.4 demands were
discharged by the design of record C3 (fixed, or README-listed as known
differences at `README.md:407–411`). Where robustness asks for more
machinery (a timer, a dedicated pool, a tombstone table, a batch-atomic
promise, a full casefold dependency), the contract's existing red lines
already refuse it. This pass adds only what the repair made new: red
lines 12–14.

## The amendments

Applied to the contract of record in the same commit as this spec — the
return dialect's "encode each answer in the owning artifact":

1. **spec.md, AC-G.1:** "the suite is 468 tests" → "the suite is 469
   tests" (verified first-hand this session: 469 passed, 0 failed, one
   opt-in ignored smoke in `tests/rate.rs`).
2. **spec.md, AC-39.2:** the proof list gains
   `src/extract.rs::an_ampersand_run_decodes_in_bounded_work` — the
   bounded-work pin the HIGH's repair added, so the repair has a recorded
   criterion (verify's `cargo test --lib extract` filter already runs it).
3. **spec.md, Base:** one sentence noting that the tree the remaining
   phases prove is `e245fae` — the review-hold repair — and pointing at
   this pass for red lines 12–14.
4. **tasks.md, task 1:** "the suite reports 468 tests" → 469.

Nothing else in the contract changes: AC-G.4's diff statement is history
about the implement commit and stays true; AC-57.3's doc-match claim is
true at both `24df038` and `e245fae` (the repair changed only prose that
had been lying about code that never had the promised behaviour); every
other proof site survives the repair verbatim — the only moved lines are
`parse_ref`'s body and the 503 description.

## The design of record and the red lines

The design is [specs/fidelity-closures/spec.md](../fidelity-closures/spec.md)
§The design — A1–A4 (#39), B1–B3 (#54), C1–C3 (#55), D1–D4 (#56), E1–E5
(#57) — as implemented at `24df038` and repaired at `e245fae`. The
contract's red lines **1–11 stand verbatim** and bind verify, review and
ship as before. Three are added by this pass:

12. **The window stays what the repair made it.** `REF_MAX` is a named
    local `const` inside `parse_ref` — never a config knob, a module-level
    constant, or a shared bounds module; the pin test stays inline in
    `src/extract.rs` and asserts output, never a duration; the window's
    vocabulary (the five XML names plus numerics) widens only with its own
    criterion and test, never by silently inflating the constant; and no
    parse timer, watchdog, or dedicated pool returns for the scan — the
    unbounded work was deleted, not promoted to supervised work.
13. **Restore stays before bind and outside `insert`.** Restored mail
    fires no watcher and no webhook (`Store::open`'s `push_back` restore,
    Ruling 4); any edit that routes restore through the mutation boundary
    or moves it after the listener binds is a review finding.
14. **The record's prose stays truthful to the accepted decisions.** The
    openapi 503 states 0014's per-mutation refusal and the idempotent
    retry; if the batch behaviour ever changes, the description changes in
    the same commit — #57's discipline applies to the run's own artifacts.

## Acceptance Criteria

This pass commits no production change; its criteria are about the record
and the repaired tree. Each names its proof. All commands run at the
repository root with `.brokkr/state/env.sh` sourced.

- **AC-1 (the count).** `cargo test --workspace --locked` reports
  **470 passed, 0 failed** (one ignored: `tests/rate.rs`'s opt-in
  throughput smoke). *Proof:* the suite run at the current base `791f83d`
  (the receipt pass re-recorded it there; this pass's own verification at
  `e245fae` reported 469 — the html-leg pin moved the count);
  re-recorded by verify under the contract's AC-G.1 / task 1 as amended.
- **AC-2 (the pin).** `cargo test --lib extract
  an_ampersand_run_decodes_in_bounded_work` passes, and the test asserts
  exact extracted output for a 500,000-`&` URL — not a duration.
  *Proof:* the inline `#[cfg(test)]` test in `src/extract.rs` (0020 rule
  1: the bounded scan is the green path, measured).
- **AC-3 (the window is named and local).** `REF_MAX: usize = 40` is a
  `const` inside `parse_ref` in `src/extract.rs`, with its reason comment,
  and is referenced nowhere else in production code. *Proof:*
  `grep -n 'REF_MAX' src/` shows the definition and its single-use site.
- **AC-4 (the 503 tells the truth).** The batch-delete 503 description in
  `src/static/openapi.json` names 0014's per-mutation refusal and the
  idempotent retry. *Proof:* the JSON inspection this session
  (`/paths//api/v1/messages/delete/responses/503/description`).
- **AC-5 (the contract no longer drifts).** The committed
  [contract](../fidelity-closures-prove-ship/spec.md) says 469 in AC-G.1
  and in tasks.md task 1, names
  `an_ampersand_run_decodes_in_bounded_work` in AC-39.2's proof list, and
  its Base line records `e245fae`. *Proof:* grep of the committed files.
- **AC-6 (the repair's shape is unchanged).**
  `git show e245fae --stat` is exactly `src/extract.rs` (24+/1−) and
  `src/static/openapi.json` (1+/1−) — nothing else. *Proof:* the git
  command; history, not the tree.
- **AC-7 (nothing new enters the tree).** This pass's commit contains
  only the synthesis artifacts (`specs/fidelity-record-honesty/`) and the
  four contract amendment lines — no `src/`, no `Cargo.toml`, no
  `deny.toml`, no decision document. *Proof:* `git show --stat HEAD`.
- **AC-8 (the standing demonstrations bind).** The contract's
  AC-39…57 proof sites and red lines 1–11 are unchanged and remain what
  verify proves and review rules against; 0031 and 0032 leave review
  `proposed` with their evidence attached, acceptance remaining the
  operator's act. *Proof:* the amended contract; the run journal.
