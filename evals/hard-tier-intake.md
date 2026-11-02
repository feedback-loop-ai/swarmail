# Hard eval tier, intake framing (epic #164, operator ruling 2026-11-01)

> **Status 2026-11-02:** this is the intake phase record and is not rewritten.
> The tier shipped three tasks, `otp-backlog`, `recipient-crowd` and
> `digest-lookalike`, not the four framed below; see
> [specs/hard-eval-tier/spec.md](../specs/hard-eval-tier/spec.md) and proposed
> [0115](../docs/decisions/0115-hard-eval-tier.md).

Run: a-harder-eval-tier-164-epic-oper-85dfc61d. Phase: intake, framing only.
Base: main f815d48. Route: claude-careful (decision 0099), zero Codex.

## Goal

Add a `hard` tier of four new tasks to the v2 catalogue. Each targets one
failure class from the closed taxonomy and is proved by oracle and null, with
its verdict read only from app state. The matrix can run the hard tier alone.
The existing 16 tasks, their ids and their fields do not change.

## Frozen guarantees

Not near 0001 to 0003. The work is confined to the standalone `evals/` crate
(0055). No server, Store, await or pruning code changes. If a candidate needs a
server change (plus-address routing, which 0112 leaves proposed, or MCP
attachment bodies), that task is dropped. The guarantee is not touched.

## Authority read

- 0055 (sections 1 to 3, 9): closed failure enum, oracle/null proofs, app-state
  scoring. 0071 section 3: exact named traps. Section 4: versions advance
  together and keep "field sets, IDs, tiers". 0076: sequential trials, no v3.
- Spec "Task format": `tier` is documented as `straight|traps|multi_step`.
- Baseline 2026-11-01: 395/400, catalogue sha256 6068b687. The suite is
  saturated, so the hard tier is the next measurement step.

## Decisions this framing takes (operator may redirect)

1. **`hard` is an additive tier value.** Existing tasks keep their tiers.
   I read 0071's "preserving tiers" as protecting existing memberships, not
   freezing the enum. The implementation lands compatible with that reading.
   The spec and 0071 amendment is a new decision document, status `proposed`,
   at the next free number, 0115 at this commit. Nothing is accepted. If the
   operator reads "preserving tiers" as a closed enum, the implementation
   waits for that decision. This is the one reading to confirm.
2. **New named traps are scoring semantics.** `superseded_invite` and the
   tracked-link decoy are added to 0071's named-trap list in the same proposed
   decision. The existing `foreign_token`, `superseded_code` and
   `digest_decoy` actions are reused where they fit.
3. **Versions stay at 2** (catalogue, snapshot, result). 0076 forbids a v3
   without need, and the change is additive. The catalogue hash changes as
   expected. Historical rows keep 6068b687.
4. **No budget-class task.** A tight deadline needs a wall-clock validity
   assertion, which 0020 section 5 forbids. A tool-call cap cannot be enforced
   across MCP. Both are rejected.
5. **`--tier` is exclusive with `--tasks`.** Combining them is refused with
   exit 2 and no rows, so the selection stays unambiguous.

## Proposed hard tasks

| id | failure_class | idle_class | Trap (the named mistake) | Scenario | Check |
|---|---|---|---|---|---|
| `otp-backlog` | stale_code | gave_up | About 12 codes for one identity from earlier generations. The current generation's mail arrives mid-backlog, not last. Arrival order and list position both mislead. Only the newest generation by Date is valid. Using any other is `superseded_code`. | new `OtpBacklog` (flow otp) | `OtpLoggedIn` |
| `invite-reschedule` | wrong_message | gave_up | The first invitation (SEQUENCE 0, time T1) is followed by a reschedule (same UID, SEQUENCE 1, time T2). Accepting with T1 is `superseded_invite`. | new `InviteReschedule` (flow invite) | `InviteAccepted`, time matched to the highest SEQUENCE |
| `digest-tracked-links` | wrong_link | gave_up | Each article link is a tracking wrapper. Two anchors show the same title. Only the one whose `u=` target is the real article is right. Anchor text alone is the trap, and the careful agent reads `u=` without opening anything. | new `DigestTracked` (flow digest) | `DigestSelected` |
| `plus-alias-confirm` | wrong_message | gave_up | Conditional. The identity is `local+signup@domain`. A bare `local@domain` mail holds a valid confirmation. Using it is `foreign_token`. Dropped if `+` identities are refused. | new `PlusAlias` (flow signup) | `Confirmed` |

Why a careful agent can pass each one: the discriminator is in the mail
itself (generation dates, SEQUENCE, the `u=` target, the exact recipient). No
task needs a guess or an external fact. Null cannot pass, because no event
fires and the idle class is declared.

## Probes before implementation (free, offline)

- MCP wait and list return Date, the decoded text/HTML body and recipient.
  `since_ms` exists in the server API. Its presence in the MCP schema is not
  yet verified (otp-backlog depends on Date ordering).
- The fixture accepts `+` local parts (plus-alias-confirm).
- The fixture HTTP can serve the `/t/` tracking route for digest-tracked-links.
- A failed probe drops that task. The others still proceed.

## Acceptance criteria (each names its red test)

A1. The first 16 catalogue entries are unchanged field for field (id, tier,
    failure_class, idle_class, goal, goal_template, scenario, check,
    agent_count, deadline_ms). Red: a pinned-copy test fails on any edit.
A2. `tasks` shows the four hard tasks with tier `hard`. The tier enum accepts
    `hard`, and an unknown tier still refuses. Red: the strict-enum test.
A3. `matrix --tier hard` emits rows only for hard ids. An unknown tier, or
    `--tier` with `--tasks`, exits 2 with no rows. Red: a non-hard row, or
    rows for a refused invocation.
A4. For each hard task, on at least 8 fixed seeds: oracle exits 0 and `check`
    passes on its state, and null exits 1 with exactly its declared idle class.
    Red: any seed where oracle fails or null's class differs.
A5. For each hard task, a scripted mistake agent (test data, not a model) that
    commits exactly its named trap fails with exactly its failure_class through
    the real trial path. Red: a wrong class, or the idle class.
A6. Hard-task verdicts come only from `score(task, snapshot)`. The scorer's
    inputs gain no mailbox or account read. Red: a boundary test on the signature.
A7. evals/README.md and run-plan.md state: 16 default plus 4 hard, the
    `--tier hard` command, the historical 6068b687 hash, and that the paid hard
    run is operator-owned. A table documents each hard task's trap, expected
    class and why a careful agent can pass. Red: a grep for each id in both docs.
A8. The proposed decision (0115 or the next free number) has status `proposed`.
    It amends the spec tier list and 0071's named traps. No accepted decision
    changes status.
A9. No network and no paid agent. Tests use loopback fixtures plus oracle, null
    and the scripted agent. No new dependency in `evals/Cargo.toml` or
    `Cargo.lock` without an operator ruling.
A10. Every house gate is green (below).

## Gates (the verify seat runs all of them, locked)

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo build --locked --bin swarmail
    cd evals && cargo fmt --all -- --check
    cd evals && cargo clippy --all-targets --locked -- -D warnings
    cd evals && SWARMAIL_BIN=<candidate swarmail> cargo test --locked
    cd evals && cargo test --locked --test proof      # plus the new hard-tier proof
    brokkr compile --bundle .brokkr/recipes/self
    bash scripts/coverage-gate.sh
    bash scripts/coverage-gate.sh selftest
    bash scripts/publish-guard-test.sh
    bash scripts/seal-branch-test.sh
    cargo deny check licenses

No root production code is expected to change. The root coverage gate does not
measure evals, so the coverage gate's green result says nothing about the new
tasks; the evals test suite is their proof.

## Non-goals

Paid or networked trials. The paid hard-tier run (operator, after merge).
Catalogue, snapshot or result version bumps. Server changes. A budget-class
task. `slow-since`, deferred to the next candidate list.
