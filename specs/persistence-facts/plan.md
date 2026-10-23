# Implementation plan — persistence facts

## Technical context

One Rust 2024 crate; write-through SQLite through rusqlite 0.32; synchronous
Store mutation owner, awaited Tokio blocking wrappers and watch-based stop
observation. DashMap readers take shard guards but not the mutation mutex.
The task changes explanatory Markdown and Rust comments only. No new
runtime dependency, type, persistence field, API or migration is needed.
The local `.specify/templates/plan-template.md` is absent; this plan supplies
the required technical context, constitution check, structure, research,
data model, contracts, quickstart and risks directly.

## Constitution check

Preserve 0001–0005, 0014's ordered writer boundary, 0015's ordinary refusal
and poison posture, 0023's awaited offload and 0024's existing watch read.
Use 0045 §8's distinction between factual observation and normative change:
accepted ADRs receive dated addenda, never edited historical bodies or new
acceptance status. No proposed ADR is needed for this factual-only scope.
A discovered behavior/contract change returns to design; a conflicting
amendment awaits the operator. No dependency or license change is allowed.
The exact coverage floor and every house-rule gate remain obligations.

Triage already chose engine, followed by the full four-seat review:
correctness, security, guarantees on official DeepSeek Flash (never Pro or
fallback), and tests on DashScope GLM-5.3. The chief does not re-triage the
work as a chore because its patch is prose. No additional agents are needed
inside this synthesis seat.

## Integration evidence and preflight

At design start HEAD and locally recorded origin/main both equal
`45d68df37044de3360439b34c39687975dd59b84`; tracked status is clean and the
candidate is attached. No detached fast-forward applies, and no remote
freshness is claimed. Both `.forge/forge.db` and `.brokkr/state/forge.db` are
absent from the boxed view. The explicitly designated operator reservation
record `.forge/epic192/fleet-context.json` (2026-10-23T20:05:53Z) and the Git
worktree registry were inspected. Its listed reservations have no overlap
with these six implementation files or this new spec directory. Unavailable
or prunable sibling paths do not prove inactivity. No required unmerged
implementation or decision dependency is identified for this correction.

Before implementation, refresh available reservations and compare the
actual base; park on an exact conflicting path or required missing decision
rather than inferring ownership. Keep the designated snapshot's limits
visible. Check `.forge/bin/brokkr` against `bundles/self/tooling.json` before
compiling; a different PATH or sibling binary is not an acceptable substitute.

## Project structure and files touched

Design commit: exactly `specs/persistence-facts/{spec,plan,tasks}.md`.
The later implementation has this bounded diff:

| File | Required edit | Findings |
|---|---|---|
| `docs/decisions/0014-one-ordered-mutation-boundary.md` | Append dated correction of §8 and serialization consequence: cap=1 counterexample, timing scope, absent measurement | 15.1–15.3 |
| `docs/decisions/0015-boundary-failure-contract.md` | Append correction of Context item 2/Decision §2; identify inherited 0012 §Why claim via link | 15.2 |
| `docs/decisions/0016-lag-invariant.md` | Append correction of §2 and Context item 3/§§3–4; link counterexample and timing correction | 15.1–15.2 |
| `docs/decisions/0024-shutdown-refusal-at-the-boundary.md` | Append correction of Preserves/§1 watch wording and Preserves/§3/Consequences timing arithmetic | 15.2, 15.4 |
| `src/store.rs` | Stopping-field doc comment and `record()` shutdown timing comment only | 15.2, 15.4 |
| `tests/persistence.rs` | Introductory comments on the blocked SMTP and REST commit tests only | 15.5 |

`src/persist.rs`, `src/store/persist_tests.rs`, protocol adapters, 0012,
0023, the decision index and all gate code are evidence, not edit targets.
Use semantic anchors above instead of relying on stale issue line numbers.

## Research and resolved choices

`Store::insert` holds the mutation mutex through planning, `record_insert`
and memory apply. `Persist::record_insert` inserts B, deletes exact pruned
ids and writes counters in one transaction. This establishes the cap=1
counterexample without manufacturing a race test. Existing completed-state
tests are regression evidence, not proof of an instantaneous subset or
cross-map reader snapshot. Keep these proof scopes separate.

The two busy-default tests inspect PRAGMA and deliberately change it on a
throwaway connection to prove the tripwire fires. Neither measures filesystem
latency. `Store::record` calls `borrow()` in the boolean shutdown expression;
the watch read uses synchronous read locking and adds no await. No need to
replace the watch with a second flag. 0012 has no cited steady-state ingest
measurement; 0023's worker-occupancy result is different evidence.

The SMTP and REST assertions already contradict their introductory comments.
SMTP proves live/restart refusal plus prior-mail retention and later success;
REST proves live/restart absence and await matching zero, without a recovery
retry. Tests and fixtures therefore need no edits. Creating a new test solely
to mirror corrected prose would add no behavior proof.

## Data model and contracts

No data model changes. The explanatory model names persistent retained sets
M (memory) and D (disk): A initially in both; capped B commit yields M={A},
D={B}; successful apply yields M=D={B}, absent another mutation. This example
is about the normal commit/apply interval, not permission to leave failed
inserts visible or to change oldest-only eviction.

Public contract stays fixed: synchronous Store methods and awaited wrappers;
SMTP 451, REST 503, MCP isError and POP3 -ERR on existing commit-error paths;
success follows successful mutation. Notifications remain post-apply wakeups,
not durable delivery receipts. Read transactions, exactly-once retry, I/O
deadlines and durable webhook outboxes are not added. No contracts or schema
files need generating; existing authorities and the spec carry requirements.
Crash-after-commit-before-reply may retain mail without a reply; client
disappearance is not zero-trace refusal.

## Implementation and validation approach

1. Record the actual implementation base and reserve the six paths. Append
   one dated correction section per ADR with finding ids, exact affected
   sections, explicit erroneous-claim identification and relative cross-links.
   Preserve historical bytes and status. Use the required content in spec.md
   as the correction checklist, not vague replacement adjectives.
2. Correct only the four identified Rust comment locations. Review each
   edited comment against its owner or adjacent assertions; retain all code,
   expressions, guards, fixtures and assertions byte-for-byte.
3. Prove preservation with a baseline-prefix check for all four ADRs:
   `new_bytes.startswith(git_show(base:path))`, and require nonempty dated
   suffixes. Review the Rust patch with full context: every changed line
   must be a comment, and reconstruct the base by reversing just those
   comment hunks to obtain byte-identical original files. Do not rely on a
   naive regex comment stripper, which could erase string literals.
4. Run the existing proof matrix below, then all required house-rule gates.
   Record commands, outcomes and exact environmental failures in run-local
   receipts. Do not weaken a gate, install dependencies in verify, or replace
   missing runtime evidence with structural inspection. Pinned verify owns
   authoritative gate execution; CI retains its platform-only obligations.
5. Commit implementation unsigned with exactly its authorized paths, then
   follow engine verification and protected four-lens review. This is later
   work; the synthesis commit remains the last commit of this design phase.

## Quickstart and proof matrix

Run from the repository root with the locked dependency tree. The full
workspace test gate includes every unit test listed below; targeted filters
are useful diagnostics, not a reason to run it repeatedly after success.

| Requirement | Existing executable proof and limit |
|---|---|
| AC2, AC7 | `cap_pruning_is_mirrored_to_the_data_file`, `record_insert_deletes_the_pruned_rows`, `pruning_follows_acceptance_not_received_ms`; prove completed retained sets, exact eviction and acceptance order, not an every-instant reader invariant |
| AC2, AC6 | `cargo test --locked --test persistence`, especially `a_blocked_commit_answers_451_and_never_claims_250`, `a_blocked_commit_refuses_the_rest_seed_with_503`, `a_refused_insert_is_never_exposed_as_accepted_mail`; real servers/SQLite, live and restart absence |
| AC2, AC7 | `a_failing_data_file_refuses_with_zero_trace`; no apply/counters/fan-out on ordinary failure |
| AC3 | `the_opened_connection_keeps_the_dependency_busy_default`, `the_busy_default_tripwire_fires_when_the_bound_is_overridden`; default configuration and can-fire control, not timing |
| AC3, AC5 | `a_wired_stopping_store_refuses_the_commit_and_applies_nothing`, lifecycle `queued_seeds_refuse_once_the_stop_has_fired`, binary `sigint_under_queued_seeds_exits_clean_and_stores_nothing`; real shutdown/refusal under their fixtures, not universal I/O deadlines |
| AC1, AC4–AC7 | Baseline-prefix checks, six-file allowlist, contextual comment-only diff review, claim-to-source/assertion audit and relative-link existence checks |
| AC8 | Full [house-rule gate inventory](../../docs/house-rules.md#gates--part-of-every-change), pinned proof chain and fail-closed native-tls absence; no local claim for CI-only proofs |
| AC9 | `bash bundles/self/scripts/design-check.sh check .` is a local structural preflight; the subsequent pinned sequence check owns the design verdict |

Example targeted command: `cargo test --workspace --locked busy_default`.
No wall-clock performance assertion, new concurrency seam, randomized chaos,
sleep synchronization, mock server or static certificate fixture is added.
If scope changes to require tests, apply decision 0020's placement rules and
return the changed design for review first.

## Risks

- **Factual correction becomes policy amendment.** Mitigation: preserve all
  historical bytes and status; explicitly distinguish observation under
  0045 §8; require a proposed decision/operator ruling for contrary behavior.
- **A replacement claim promises snapshot atomicity.** Mitigation: include
  the cap=1 transaction order, separate writer serialization from reader
  observations, and name poison/restart limits. No new locking is permitted.
- **An apparent timeout survives nearby.** Mitigation: cover 0024's timing
  arithmetic and `record()` as well as the three persistence records; link
  the inherited 0012 statement. Configuration tests cannot certify latency.
- **Historical wording confuses readers.** Mitigation: use dated, explicit
  correction headings and section references; accept the cost of retained
  erroneous history rather than silently rewriting accepted records.
- **Scope expands into neighboring defects or benchmark work.** Mitigation:
  enforce the six-file/comment allowlist and exact five-finding map; state
  unsupported performance evidence plainly. No new benchmark or reader audit.
- **Tests overclaim recovery.** Mitigation: match each test's own assertions;
  only SMTP claims the demonstrated recovery retry. Existing tests protect
  behavior but do not validate every sentence automatically.
- **Ownership or tooling evidence is stale/unavailable.** Mitigation: refresh
  designated reservations before implementation, park on concrete overlap or
  missing required inputs, verify the local engine digest, and record blocked
  gates as outstanding. This draft is not implementation or epic completion.

## Design-phase validation — 2026-10-24

On the unchanged implementation at baseline `45d68df`, the chief ran and
passed fmt, all-targets locked clippy, the full locked workspace tests, the
real persistence suite, digest-matched local brokkr compile, exact 100/100/100
coverage, coverage selftest, publish-guard proof, seal-branch keyless proof,
license checks and fail-closed native-tls absence. The design-check selftest
and delivery tamper-boundary proof also passed. Relative artifact links and
required headings/checkboxes were checked. Command receipts and logs are
run-local under `.forge/design/persistence-facts/`; a tool response timed out,
but its completed receipts were subsequently read and verified.

These are local baseline/design checks, not the final pinned sequence verdict,
implementation completion, independent review, or CI-only proof. Later edits
still owe the implementation-bound verification and review in tasks.md.
