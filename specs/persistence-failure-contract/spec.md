# Spec — issue #31, the P1 storage-consistency slice, run 2: the boundary stands, its failure contract is written down

Issue [feedback-loop-ai/swarmail#31](https://github.com/feedback-loop-ai/swarmail#31)
(*"Story: Close persistence, lifecycle and deployment gaps from the architecture
reassessment"*, reviewed commit `5b09843`). This run delivers **only** the
storage-consistency slice; lifecycle, adapter offloading, the Helm chart, the
contract-agreement and the CI findings are the second run. Intake framing:
[`.forge/intake/issue31-p1-persistence-framing-efc41bdd.md`](../../../.forge/intake/issue31-p1-persistence-framing-efc41bdd.md).
The panel's positions —
[`.forge/design/positions/simplicity.md`](../../../.forge/design/positions/simplicity.md) and
[`.forge/design/positions/robustness.md`](../../../.forge/design/positions/robustness.md)
(both `pass`) — are run-local evidence, gitignored.

Decision: **0015** —
[docs/decisions/0015-boundary-failure-contract.md](../../docs/decisions/0015-boundary-failure-contract.md)
(status `proposed` — acceptance is the operator's). It amends decision
[0012](../../docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md)
(proposed) §5 and §7 **in place** — legitimate because 0012 still awaits the
operator (0014's own precedent) — and the in-place amendments are landing
tasks of this run (AC5), not committed here. Decision
[0014](../../docs/decisions/0014-one-ordered-mutation-boundary.md) (proposed)
is built upon, not reopened: **no line of the boundary changes in this run.**

## The situation — the tree already carries the slice

The reviewed commit `5b09843` no longer describes the code. `3b158cd`
implements 0014: one ordered mutation boundary — the `mutation: Mutex<()>`
span widened to cover plan → commit → apply, commit before visibility,
refusal with zero trace, fan-out after apply — and `src/persist.rs` is
unchanged, as 0014 §5 demands. The council's rule for exactly this situation
is the intake framing's: *reopen the mechanism only if a criterion is red on
HEAD.* The simplicity seat ran the red-detector on HEAD — the C1 probe
(`sixteen_threads_cannot_leave_memory_and_disk_in_two_orders`) is **green**
(6 rounds × 16 barrier-released roles on a real data file) — and the framing's
own spot-checks confirm the C2/C3/C4 legs exist. **No criterion is red.**
This run therefore proves and closes; it does not redesign. A red found later
by verify reopens the council — it is never fixed silently.

What is *not* closed is the boundary's **failure contract**: the code's
behaviour on each failure is right, and the documents that carry it are wrong
in one place and silent in three.

## Ruling — how the positions were reconciled

The seats converge on upholding the incumbent and diverge on what this run
should spend its ink on: simplicity says *confirm and refuse to add*;
robustness says *uphold the mechanism, write its failure contract*. This spec
rules rather than averages.

**Adopted from both (convergent):** the one ordered mutation boundary is the
closing design for findings 1–3 and is not reopened. The standing refusal
list is carried as a check every successor must answer — no bounded writer or
write-behind queue (0010 §3), no undo/rollback engine (the ghost is
unrepresentable; there is nothing to roll back), no per-inbox locks (they
cannot span `clear_all` and re-derive finding 1), no inbox-wide
`DELETE … WHERE inbox = ?` (the per-id mirror is self-verifying), no
degraded mode, no failure counter (the refusal *is* the detector), no new
error type (`rusqlite::Error` is already public), no new `src/` file, no new
dependency, no new wire string, no `coverage(off)`, no commit-stage chaos
event (injection stays real: triggers and lock-holders on the real data
file), no feature flag toggling the sequencing, no epoch/generation scheme.
Simplicity's four "looks heavier than it is" records stand: the plan phase,
the `drop(_guard)` before fan-out, the `debug_assert` in `clear`, and the
contracted `pre_mirror_gate` seam are load-bearing and must not be
cosmetically shaved.

**Ruling 1 — the QUIT contradiction is corrected in the documents now; atomic
QUIT is refused for this run and owed to the adapter-consistency run.** This
is the one genuine conflict, and it rules on robustness's own split
recommendation. The current text is self-contradictory: `src/pop3.rs`'s
`QUIT` comment says "the update is abandoned whole: the marks are unapplied"
and three lines later admits "the already-deleted part of a partial batch
stays deleted"; 0012 §5 says "its marks unapplied" flatly. The code is right
and the prose is wrong: `QUIT` applies marks one `delete_async` at a time and
answers `-ERR` on the first failure, so a mid-loop failure leaves a committed
*prefix* — at-least-once, never a success-then-lie, every client path
convergent (retry `QUIT`, which is idempotent over the unmarked suffix, or
`RSET`, or a fresh session that shows the truth). **Adopted (mandatory):**
amend 0012 §5 and the `pop3.rs` comment to that truth. **Refused for this
run (robustness's optional hardening):** atomic `QUIT` — collect the marks,
one commit, one apply. It is a public-contract change to POP3 update
semantics plus a new store API (`delete_many`), with **no red criterion**
under it; the run-2 adapter-consistency slice is its named owner.
**Cost of the losing side:** POP3 batch updates stay at-least-once — a client
whose `QUIT` fails mid-loop must retry to converge, and a *selective* failure
leaves a committed prefix the `-ERR` does not name. Bounded, convergent,
documented: the price of proving before building.

**Ruling 2 — the busy handler enters the refusal contract, as a document
(robustness's mandatory item), with the chief's correction of its location.**
Robustness's ledger calls the 5 s bound "a PRAGMA inside `persist.rs`"; it is
not — there is no busy handler in this crate. The bound is **rusqlite's own
default**: `sqlite3_busy_timeout(db, 5000)` set by the dependency at open
(rusqlite 0.32.1, `src/inner_connection.rs:119` — the exact line 0012 §Why
already cites). `persist.rs` deliberately never overrides it. The contract
sentence this run writes into the documents: a stalled or contended data file
delays each refusal by at most ~5 s per statement, then `SQLITE_BUSY`
surfaces as the refusal (451/503/`isError`/`-ERR`) — the `451` arrives *after*
a bounded stall, not immediately and not never; the bound is deliberately not
wall-clock-asserted (0011 rule 5 — the legs prove the *refusal*, never the
timing); and whoever changes the bound — an override in `persist.rs` or a
dependency bump — rewrites the failure semantics of every surface and owes a
decision document. **Cost of adoption:** one more paragraph of frozen prose
that a future tuning change must revisit — the price of the bound being
load-bearing and invisible in our own tree.

**Ruling 3 — poison, the second writer and the refusal residue are named, not
built (robustness's doc items; simplicity's economy).** Adopted as
documentation in 0015: *poison's recovery is restart* (the restart copy is
always exactly the committed set; a self-exiting process would be more honest
than a write-panicking, read-serving zombie, but no criterion demands it —
the lifecycle run owns exit semantics if ever asked); *single-writer is a
deployment assumption* (`Persist::open` takes no exclusive lock; the advisory
open-guard belongs to the deployment run); *the refusal residue stays as 0014
states it* (a burned UUIDv7 id, a burned sequence number, an over-cap window
until the next accepted insert — pruning on refusal would evict committed
mail, which the boundary exists to protect). **Cost of refusal here:** the
second-writer hazard and the zombie window remain until the second run names
them again — both are documented, neither is silent.

**Ruling 4 — the contract gets an instrument, not just prose (chief's
addition; the house's own standard).** "A criterion without a red state under
its instrument is not a criterion." The existing QUIT leg
(`a_failed_quit_answers_err_and_keeps_the_marks`, `tests/pop3.rs:632`) uses
the all-refusing trigger, so its first delete fails and the prefix is empty —
it proves refusal, not the partial-prefix contract, exactly as robustness
observed. This run adds the missing instrument: a **selective** refusing
trigger (refuse one id, allow the rest — test-code only, joining
`tests/common/mod.rs`'s real-injection vocabulary) and a POP3 wire leg that
drives `QUIT` across a mid-loop refusal: `-ERR`, TRANSACTION retained, the
allowed ids' commits landed, the refused id on both trees, retry-after-
recovery signs off. Red on `5b09843` (which swallowed and answered `+OK`),
green on HEAD. No production line changes, so the coverage denominator is
untouched.

**Ruling 5 — the run-2 discipline is committed here, so the next run inherits
it (simplicity §6).** The second slice must *reuse* the boundary, not extend
or reopen it: the compat bulk delete (P2) is `delete_async` per id off the
async workers — the same ordered boundary, not a batch bypass and not a
second mutation path; shutdown/lifecycle work must not wrap the boundary in a
writer, queue or drain; ADR0012 §3–§6 are not re-litigated (the 0014
amendment markers are in place; the epoch/generation debt note is retired);
any proposed hardening follows the house order — delete the unnecessary path,
simplify to one owner, restore proven behaviour, then add the smallest local
guard, which in this tree is almost always the refusal the boundary already
returns.

## The failure contract — what this run writes down

| Failure | Contract (already the code's behaviour) | Where it lives after this run |
|---|---|---|
| Crash before commit | Nothing on disk, no `250` sent; the peer retries. At-least-once. | 0014 (unchanged) |
| Crash between commit and apply/`250` | Row durable, no `250`; the retry duplicates. At-least-once; the restart copy is exactly the committed set. | 0014 (unchanged) |
| Storage refusal mid-batch (POP3 `QUIT`) | Committed prefix stays committed; `-ERR`; session stays in TRANSACTION with the failing suffix unmarked; retry converges. **At-least-once — the documents stop claiming atomicity.** | **0015 §1** → 0012 §5 + `pop3.rs` comment |
| Disk stall at the boundary | Every mutation delays ≤ ~5 s per statement (rusqlite's default busy timeout, never overridden), then refuses with `SQLITE_BUSY` → 451/503/`isError`/`-ERR`. Deliberately not wall-clock-asserted. | **0015 §2** |
| Panic inside the boundary | Fail-stop via the poison `expect`; readers keep serving memory; **restart is the recovery** — the restart copy is exactly the committed set. | **0015 §3** → 0012 §7 |
| Two processes, one data file | No byte-level corruption (SQLite file locking); logical state interleaves. Single-writer is a deployment assumption; the open guard is the deployment run's. | **0015 §4** |
| Refusal residue | Burned UUIDv7 id and seq number; over-cap window until the next accepted insert; no counter moves. | 0014 (reaffirmed, **0015 §5**) |

## Frozen guarantees — posture

| Guarantee | Verdict |
|---|---|
| **0001** accept == stored | **Preserved, strengthened posture unchanged.** The refusal (451/503/`isError`/`-ERR`) is the converse arm that keeps the promise honest; 0015 writes down when that refusal arrives (after a bounded stall) and what a mid-batch refusal leaves behind. |
| **0002** await subscribes before it checks | Untouched. `wait_for` is not edited in this run. |
| **0003** pruning is oldest-only | Untouched. No eviction code path changes; the refusal residue stays exactly 0014's. |
| **0010**, **0011** (proposed) | Built upon (test-code only, plus comment/doc edits); the new leg obeys 0011 rule 5 — no sleeps, no wall-clock validity assertions. |
| **0012**, **0014** (proposed) | 0012 amended in place by this run's tasks (§5 QUIT truth, §7 poison→restart); 0014 stands untouched; 0015 is the new ruling. All three stay `proposed` for the operator. |
| Exact 100/100/100 gate | Held. The only `src/` change is a comment; the new test lives in `tests/`; the denominator does not move. |

## Acceptance Criteria

- **AC1 — One mutation order stands (finding 1, C1).** The boundary is
  unmodified on HEAD: the 16-thread barrier-released probe over a real data
  file with `max_per_inbox = 1` holds live `list`/`get`/`count` and a fresh
  `Store::open` in agreement — same per-inbox ids and order, same counters,
  no inbox over cap after restart. *Proved by:*
  `cargo test --lib sixteen_threads_cannot_leave_memory_and_disk_in_two_orders`
  — green on HEAD, red on `5b09843`. Any red reopens the council.
- **AC2 — A refused insert is invisible (finding 2, C2).** With cap = 1, one
  committed mail and a rejecting `BEFORE INSERT` trigger on the real data
  file: the next send answers `451 4.3.0 storage commit failed`; the refused
  mail is queryable through no surface; the committed-first mail is not
  evicted; neither counter moves; no watcher wakes, no webhook fires; after
  recovery the retry answers `250` once; the restart holds exactly the
  committed set. *Proved by:* `cargo test --test persistence`
  (`a_blocked_commit_answers_451_and_never_claims_250`,
  `a_blocked_commit_refuses_the_rest_seed_with_503`,
  `a_refused_insert_is_never_exposed_as_accepted_mail`) — green on HEAD, red
  on `5b09843`.
- **AC3 — A failed deletion refuses and never resurrects (finding 3, C3).**
  With a rejecting `BEFORE DELETE` trigger: native `DELETE` answers `503`
  (never `200 {"deleted":true}`); shim deletes answer `503`; MCP destructive
  tools answer `isError: true`; POP3 `QUIT` answers `-ERR` and stays in
  TRANSACTION; nothing is removed live; the restart holds the refused-deletion
  mail. **And the new partial-prefix leg:** with a *selective* trigger
  refusing one id, `QUIT` answers `-ERR`, the session stays in TRANSACTION,
  the allowed ids' commits have landed, the refused id remains live and after
  restart, and a retry `QUIT` after recovery signs off. *Proved by:*
  `cargo test --test persistence --test compat --test mcp --test pop3`
  including the new selective-trigger leg — red on `5b09843`, green on HEAD.
- **AC4 — Counters are events of the commit (C4).** A refused insert moves
  neither `emails_inserted` nor `emails_dropped`, live or after restart;
  every eviction of an accepted insert is counted exactly once; the eviction
  choice is front ids, in order. *Proved by:* `cargo test --lib` (the
  `persist_tests` harness + inline probes) and the restart-counter asserts in
  `tests/persistence.rs` (`counters_keep_counting_across_restarts`).
- **AC5 — The failure contract is written accurately (C5 extended).** 0015
  exists, status `proposed`; 0012 §5 and §7 carry the 0015 amendment markers
  in place (partial-prefix `QUIT` truth; restart-is-the-recovery); the
  `src/pop3.rs` `QUIT` comment states the partial-prefix truth and no longer
  says "abandoned whole"; the busy handler is documented as part of the
  refusal contract with its true location (rusqlite's default at open, never
  overridden in this crate, deliberately not wall-clock-asserted);
  `docs/decisions/README.md` lists 0015; no accepted decision is edited.
  *Proved by:* reading the files in review — the contradiction the current
  text carries today is the red state.
- **AC6 — The gate discipline holds (C6).** All six gates green:
  `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`;
  `cargo test` (real servers, no mocks); `brokkr compile --bundle
  bundles/self`; `bash scripts/coverage-gate.sh` (exact 100%
  lines/branches/functions, missed == 0); `cargo deny check licenses`. No new
  production line, no new dependency, no `coverage(off)`; the only `src/`
  change is a comment. *Proved by:* the gate runs themselves.

## Verify gate

```bash
GIT_CONFIG_COUNT=0 cargo test --lib sixteen_threads_cannot_leave_memory_and_disk_in_two_orders  # AC1
cargo test --test persistence                        # AC2, AC3, AC4
cargo test --test compat --test mcp --test pop3      # AC3 surfaces + the new prefix leg
cargo test                                           # everything, real servers
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
brokkr compile --bundle bundles/self
GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh     # exact 100/100/100 (AC6)
cargo deny check licenses
```

The filtered invocations are named because they are the criteria's
instruments; `cargo test` subsumes them.
