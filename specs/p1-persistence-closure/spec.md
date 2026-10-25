# Spec — issue #31, the P1 storage-consistency slice, run 3: the boundary stands, its invariant is stated, its busy bound is loud

Issue [feedback-loop-ai/swarmail#31](https://github.com/feedback-loop-ai/swarmail#31)
(*"Story: Close persistence, lifecycle and deployment gaps from the architecture
reassessment"*, reviewed commit `5b09843`). This run delivers **only** the
storage-consistency slice; lifecycle, adapter offloading, the Helm chart, the
contract-agreement and the CI findings are the other run. Intake framing:
`.forge/intake/issue31-p1-persistence-framing.md`
(lineage: `…-efc41bdd.md`).
The panel's positions —
`.forge/design/positions/simplicity.md` and
`.forge/design/positions/robustness.md`
(both `pass`) — are run-local evidence, gitignored.

Decision: **0016** —
[docs/decisions/0016-lag-invariant.md](../../docs/decisions/0016-lag-invariant.md)
(status `proposed` — acceptance is the operator's). It confirms 0014/0015
without reopening them, and its in-place amendments to those two proposed
decisions (0014 §8, 0015 §2) are landing tasks of this run (AC2), not
committed here.

## The situation — nothing is red, so nothing is rebuilt

The reviewed commit `5b09843` stopped describing the tree two runs ago:
`3b158cd` implements 0014's one ordered mutation boundary (the `mutation`
lock spanning plan → commit → apply at `src/store.rs:363/:514/:547/:577`,
commit before visibility, refusal with zero trace, fan-out after apply) and
`832b3fd` lands 0015's failure contract. Run 2's spec closed the failure
contract's documents; run 3's panel verified rather than trusted — and
re-verified in this synthesis: `git diff 5b09843..HEAD -- src/persist.rs
Cargo.toml Cargo.lock` is **empty**; the eleven refusal call sites reuse
0012 §2's fixed text; the C1 probe
(`sixteen_threads_cannot_leave_memory_and_disk_in_two_orders`) was re-run
**green** on HEAD `19cc49d` during this design (0.59 s), and the ten
real-server wire legs of `tests/persistence.rs` were re-run **green** (5.9 s).
**No criterion is red.** This run proves and closes; it does not redesign.
A red found later by verify reopens the council — it is never fixed silently.

What run 3 closes is the last of the truth-telling work, the three gaps both
seats measured in the documents rather than the code:

1. the **lag invariant** — the property the boundary actually buys — is
   stated nowhere;
2. **disk exhaustion** (ENOSPC/EDQUOT) is answered by the refusal contract
   in code and claimed in no document, and the size-cap refusal is recorded
   nowhere;
3. the **~5 s busy bound** — 0015 §2's load-bearing constant — is enforced
   by nothing: its true location is rusqlite's open-time default, invisible
   in this tree, so a dependency bump rewrites every surface's failure
   timing silently.

## Ruling — how the positions were reconciled

The seats converge on upholding the incumbent; they diverge on where the
confirmation's ink goes and on one instrument. This spec rules rather than
averages.

**Ruling 1 — the confirmation is a ruling of record in the decisions, not
only in this spec (robustness's channel argument beats simplicity's
spec-only shape).** Simplicity's ideal outcome is a confirmation spec whose
entire added vocabulary is one sentence — *the mutation lock now spans the
commit* — carried with the refusal list and the anti-cut list. Adopted — the
vocabulary stays exactly that small. But the confirmation lives in
**0016** (committed, `proposed`), not only here: the house rules make the
decisions the authority intake seats and reviewers read, and a confirmation
that lives only in a run-local spec binds no future run. **Cost of the
losing side (spec-only):** none in code — it is one more committed page —
and the gain is that the next run's framing meets the invariant in the index
instead of rediscovering it.

**Ruling 2 — the lag invariant is stated once, as 0014 §8 (robustness's
mandatory item; adopted verbatim in substance).** *The live view lags the
commit; it never leads it.* Insert: memory ⊆ disk at every instant — the
ghost (visible mail whose commit did not land) is unrepresentable. Delete /
clear / clear-all: memory ⊇ disk at every instant — the resurrection (a
success reply the restart copy contradicts) is unrepresentable. The one tear
is poison's (a mid-apply panic), bounded by the boundary and healed by
restart (0015 §3). The full paragraph lives in
[0016 §2](../../docs/decisions/0016-lag-invariant.md) and lands in 0014 §8;
this spec does not carry a second copy that can drift.

**Ruling 3 — disk exhaustion rides the refusal contract; the size cap is
refused (robustness's claim; simplicity's economy).** A full or failing disk
is an ordinary commit error: ENOSPC/EDQUOT answers the fixed-text refusal
(451/503/`isError`/`-ERR`) on every surface after the same bounded stall,
with zero new code — the docs claim it. **A size cap and a `552` are
refused**: a new promise, a new wire string, and a new ordering question
(cap-check before or inside the boundary?) with no red criterion under it;
the counter-move is deployment-owned (a volume quota). **Cost of the losing
side (a size cap):** named in 0016 §3 — a hostile peer *can* fill the disk,
and the system answers `451` until an operator intervenes; bounded,
convergent, honest.

**Ruling 4 — the busy bound is pinned by a fact tripwire (the genuine
conflict; ruled for robustness, with simplicity's freeze clarified).**
Robustness asks for an inline test asserting the opened connection's
`PRAGMA busy_timeout` equals the dependency default; simplicity's stance
would add nothing to `persist.rs`, whose zero-line diff 0014 §5 records as a
virtue. **Adopted**, because the alternative is the silent rewrite 0015 §2
itself names: a dependency bump that moves the default rewrites every
surface's refusal timing and nothing in the tree objects. The tripwire is a
*fact*, never a wall-clock assertion (0020 rule 5), branch-stable, and adds
zero production lines — 0014 §5's freeze guards the mirror's *behavior*, and
0015 already edited a comment there by exactly that reasoning; test-only
lines in the measured inline module shrink no denominator. Two legs, per the
house standard that an instrument must prove it can fire: the green leg
(`the_opened_connection_keeps_the_dependency_busy_default`) and a
`#[should_panic]` leg that overrides the pragma on a throwaway connection
and proves the tripwire trips
(`the_busy_default_tripwire_fires_when_the_bound_is_overridden`). **Cost of
the losing side (no tripwire):** the bound stays prose-only; the red state
is a dependency bump nobody notices until a stalled disk answers `451` at a
different delay than every document claims. **Cost of adoption:** the test
couples to the dependency default (5000, rusqlite 0.32.1
`inner_connection.rs:119`), so a legitimate default change fails the suite —
which is the tripwire working: it fires the decision-document obligation
0015 §2 already imposes.

**Ruling 5 — simplicity's refusal list and anti-cut list stand as review
checks, and its successor discipline is inherited.** No new `src/` file, no
new type, no new dependency, no new wire string, no counter, no epoch
scheme, no feature flag, no retry loop inside the store, no
`insert_checked`; the load-bearing lines (the plan phase's post-counters,
`drop(peeked)`, `pop_front` eviction, the contracted `pre_mirror_gate` seam,
the two distinct probes) must not be cosmetically shaved. The successor
slice *reuses* the boundary — compat bulk delete is `delete_async` per id
under the existing offload, never a batch bypass; atomic `QUIT` keeps its
owner in the adapter-consistency run. The starvation canary stays
`tests/burst.rs` + `benches/ingest.rs`: a floor drop there is a red
criterion, not noise.

## Standing refusal list (carried, not re-litigated)

The per-surface refusal table (SMTP `451 4.3.0 storage commit failed`,
REST/compat `503`, MCP `isError: true`, POP3 `-ERR`) lives in 0012 §2 as
amended and 0015 §1; this run rewrites none of it — referencing, not
re-stating, is how drift is avoided. Refused for this run and every run
until a red criterion says otherwise: a bounded writer / write-behind queue
(0023 §3), an undo/rollback engine (nothing runs on refusal — there is
nothing to roll back), per-inbox locks (they cannot span `clear_all` or the
global index), disk-first reordering, a degraded-mode latch, a
`persist_failures_total` counter, a size cap / `552` (Ruling 3), poison
self-exit (lifecycle-run owned), a second-writer open guard
(deployment-run owned), synthetic commit-stage chaos (injection stays real:
triggers and lock-holders on the real data file), and any edit to
`persist.rs`'s mirror behavior (0014 §5).

## Frozen guarantees — posture

| Guarantee | Verdict |
|---|---|
| **0001** accept == stored | **Unchanged, still strengthened.** No behavior line moves in this run; the lag invariant states why `250 ⇒ stored` holds and when the refusal answers instead. |
| **0002** await subscribes before it checks | Untouched. `wait_for` is not edited. |
| **0003** pruning is oldest-only | Untouched. No eviction path changes; counters are events of the commit (0014 §4). |
| **0023**, **0020**, **0012**, **0014**, **0015** (proposed) | Built upon. 0014 and 0015 are amended **in place** by this run's tasks with markers naming 0016 — legitimate because both still await the operator (0014's own precedent). No accepted decision is edited. |
| Exact 100/100/100 gate | Held. The only `src/` change is two tests in persist.rs's existing inline module; every tripwire line executes on every run; the denominator does not shrink (no harness file, no `coverage(off)`). |

## Acceptance Criteria

- **AC1 — No criterion is red; the boundary stands unmodified (C1).** The
  16-thread barrier-released probe over a real data file with
  `max_per_inbox = 1` keeps live `list`/`get`/`count` and a fresh
  `Store::open` in agreement — same ids, same per-inbox order, same
  counters, nothing over cap after restart. *Proved by:*
  `GIT_CONFIG_COUNT=0 cargo test --lib sixteen_threads_cannot_leave_memory_and_disk_in_two_orders`
  — green this session on `19cc49d`; the adopting run re-runs it. Any red
  reopens the council.
- **AC2 — The invariant and the claims live where the house reads them
  (C5).** 0014 carries §8, the lag invariant, with an *(Added by 0016 …)*
  marker; 0015 §2 carries the ENOSPC-as-refusal claim, the size-cap refusal
  and the tripwire clause under an *(Amended by 0016 …)* marker; 0016 is
  committed with status `proposed`; `docs/decisions/README.md` lists 0016;
  no accepted decision is edited. *Proved by:* reading the diffs in review —
  today's silence is the red state (greps for "lags the commit"/"never leads"
  and for an ENOSPC claim in 0015 return nothing on HEAD).
- **AC3 — The busy bound is loud (0015 §2 made checkable).**
  `the_opened_connection_keeps_the_dependency_busy_default` opens a real
  `Persist` and asserts `PRAGMA busy_timeout` == 5000 with the dependency
  citation; `the_busy_default_tripwire_fires_when_the_bound_is_overridden`
  (`#[should_panic]`) overrides the pragma on a throwaway connection and
  proves the tripwire trips. Both live in persist.rs's existing inline
  `#[cfg(test)] mod tests`; no sleeps, no wall-clock validity assertions,
  no new file. *Proved by:*
  `GIT_CONFIG_COUNT=0 cargo test --lib busy_default` — both green; the green
  leg's red state is any override reaching the server's own connection.
- **AC4 — Zero behavior lines (C6).** The `src/` diff of the adopting run
  is exactly the two tripwire tests; no new `src/` file, type, dependency,
  wire string or counter; `Cargo.toml`/`Cargo.lock` unchanged. *Proved by:*
  `git diff --stat` review plus
  `GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh` — exact 100%
  lines/branches/functions, missed == 0, the floor not lowered.
- **AC5 — The slice's own criteria stay green (C2/C3/C4).** The refused
  insert is invisible on every surface, failed deletes/clears refuse and
  never resurrect, and counters stay exact: the ten real-server wire legs of
  `tests/persistence.rs` and the POP3/compat/MCP legs pass.
  *Proved by:* `cargo test` (subsumes
  `--test persistence --test compat --test mcp --test pop3`).
- **AC6 — The house gates pass (C6).** `cargo fmt --all -- --check`;
  `cargo clippy --all-targets -- -D warnings`; `cargo test` (real servers,
  no mocks); `brokkr compile --bundle bundles/self`;
  `GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh` (exact 100/100/100);
  `cargo deny check licenses`. *Proved by:* the gate runs themselves.

## Verify gate

```bash
GIT_CONFIG_COUNT=0 cargo test --lib sixteen_threads_cannot_leave_memory_and_disk_in_two_orders  # AC1
GIT_CONFIG_COUNT=0 cargo test --lib busy_default                                                # AC3
cargo test                                            # AC5 (+ AC1/AC3 subsumed)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
brokkr compile --bundle bundles/self
GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh      # AC4: exact 100/100/100
cargo deny check licenses                             # AC6
```

The filtered invocations are named because they are the criteria's
instruments; `cargo test` subsumes them. A red anywhere reopens the council.
