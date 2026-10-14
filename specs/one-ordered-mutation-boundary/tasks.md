# Tasks — issue #31 P1 slice: one ordered mutation boundary

Ordered. Each task names the test that proves it. Decisions
[0014](../../docs/decisions/0014-one-ordered-mutation-boundary.md) (new) and
[0012 as amended](../../docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md)
are the rulings; both stay `proposed` for the operator. Scope guard: **only**
the storage-consistency slice — no lifecycle, offloading, chart,
contract-agreement or CI work; `src/persist.rs`, `src/smtp.rs`,
`src/ui.rs` and `src/stdio.rs` change by **zero lines**; no new `src/` file,
type or dependency; the exact 100/100/100 gate is never relaxed. Concurrency
tests follow 0020: the probe inline, cold failure paths in the harness,
wire-shaped races in `tests/`.

- [ ] 1. Write the refusal suite **red first**. In
        `tests/common/mod.rs` add `refusing_trigger(db, kind)` — a real
        `CREATE TRIGGER … BEGIN SELECT RAISE(ABORT, 'refused'); END` on the
        real data file (`INSERT`-rejecting and `DELETE`-rejecting kinds).
        Rewrite `src/store/persist_tests.rs`'s
        `a_failing_data_file_never_blocks_the_store` to the zero-trace
        refusal: `fail_writes()`, channels subscribed first, then
        `insert(...).is_err()` ⇒ `count == 0`, `get` is `None`, counters
        unchanged, both channels `try_recv`-empty, `inboxes()` mints no
        phantom entry, and committed mail is not evicted; then
        `delete`/`clear`/`clear_all` each return `Err` with memory and disk
        preserved. In `tests/persistence.rs`, flip the two blocked-commit
        legs' memory-truth blocks to absence (`matched = 0` via `/assert`,
        `total == 0`, the committed-first mail still there) and add the
        `BEFORE INSERT`-trigger leg (cap = 1, one committed mail, refused
        send, no eviction, restart shows only committed-first, trigger
        dropped ⇒ retry `250`).
        — *Proved by:* `cargo test --lib
        a_failing_data_file_never_blocks_the_store` and
        `cargo test --test persistence refusal` **fail on the current tree**
        — today it answers the refusal and still serves the mail (AC1 red,
        AC2 red).

- [ ] 2. Land the failure-propagation contract in `src/store.rs`:
        `delete` commits `record_delete(id)?` then applies the memory
        removal; `clear`/`clear_all` snapshot ids, commit
        `record_deletes(&ids)?`, then drain; all three become
        `Result<bool/usize, rusqlite::Error>` (async wrappers inherit via
        `offload`). Map the eight call sites: `src/api.rs` `delete_message`
        → `503 storage commit failed` (`404` stays for absent), `clear_inbox`
        → `503`; `src/compat.rs` `delete_messages` moves to `delete_async`
        and → `503`, delete-all → `503`; `src/mcp.rs` →
        `error_result("storage commit failed")`; `src/pop3.rs` `QUIT` →
        first `Err` answers `-ERR storage commit failed`, no sign-off,
        session stays in TRANSACTION with marks unapplied. Mechanical
        `.unwrap()` ripple across the inline-test call sites; **delete** the
        two `pre_mirror_gate` re-entry race tests (their window is doomed);
        the gate and its two pins stay at the old timing until task 4.
        — *Proved by:* task 1's harness test is green for its
        delete/clear/clear_all arms; `cargo test --test persistence
        a_blocked_commit` passes; `cargo test --test pop3 --test compat
        --test mcp` passes (their failure legs land in task 5).

- [ ] 3. Write the 16-thread barrier probe **red** into the inline
        `#[cfg(test)]` mod of `src/store.rs` (0020 rule 1 — warm branches
        only): a fresh `Store::open(1, &real_data_file)` per round, 16
        barrier-released threads on public `Store::insert` across two
        inboxes plus `delete`/`clear`/`clear_all` racer roles, several
        rounds, joined per round; then assert live `list`/`get`/`count`
        equal a fresh `Store::open` on the same file — same per-inbox ID
        sets, same per-inbox ordering, same counters — and no inbox exceeds
        the cap after restart.
        — *Proved by:* `cargo test --lib <probe>` **fails on the tree with
        the commit still outside the boundary** (the issue's finding-1
        repro); record the red three times before task 4. If no red is
        producible in a dense run, stop and investigate — never tune the
        assertion.

- [ ] 4. Reorder `insert` to plan → commit → apply inside the boundary
        widened across the commit (0014 §1-§2): peek the overflow ids by
        iterator (never `entry().or_default()` — no phantom inbox),
        `record_insert(email, &pruned_ids, inserted+1, dropped+overflow)?`,
        then on `Ok` push/prune/index/counter-bump; fan-out stays after
        apply, outside the commit. Move the `pre_mirror_gate` firing to
        pre-commit **inside** the boundary under the no-re-entry contract
        (0014 §6); port the await-ordering pin's closure to block-and-release
        (no store re-entry — it would deadlock); the panic-symmetry pin
        keeps its shape. Update `record`'s log wording: refused, nothing
        stored, committed state preserved.
        — *Proved by:* task 3's probe is green (AC1) and task 1's refusal
        suite is fully green (AC2), including absence on `get`/`list`/
        `count`/`assert`/`await`/POP3/UI and committed-first un-evicted;
        the two ported pins pass (`cargo test --lib
        a_panicking_mutation_re_raises_in_the_awaiting_caller
        the_awaited_mutation_does_not_resolve_while_its_commit_is_in_flight`,
        names per landing); the clear-race wire leg
        (`tests/persistence.rs:291`) still passes unmodified.

- [ ] 5. Add the remaining wire legs, red first where an adapter still lies:
        `tests/mcp.rs` (delete/clear/clear_all under the trigger →
        `isError: true`, state preserved, no resurrection after restart);
        `tests/compat.rs` (mailpit delete + delete-all → `503`); 
        `tests/pop3.rs` (`DELE` mark, `QUIT` under the trigger → `-ERR`,
        `LIST` still shows the mail, session accepts further commands, `RSET`
        then trigger-dropped `QUIT` → `+OK` and gone);
        `tests/persistence.rs` — the no-webhook leg (`destination_sink`
        receives nothing across a refused insert), the `await` leg
        (long-poll times out `matched = 0` while the refused mail is in
        flight), the UI leg (the refused mail absent from the rendered
        page), and the counter-refusal leg appended to
        `counters_keep_counting_across_restarts`.
        — *Proved by:* the legs **fail on the current tree** where they
        assert today's success claims (`200 {"deleted":true}`, `+OK`,
        counters incremented) and pass after tasks 2/4 (AC2, AC3, AC4).

- [ ] 6. Amend decision 0012 **in place** (it is still `proposed`): §4's
        ghost reversed to the zero-trace refusal, §5's logged swallow
        reversed to propagated failure with the per-surface table, §6's
        "adds nothing across `record`" replaced by the
        boundary-across-commit rule (fixed lock order, no-re-entry seam
        contract, the at-least-once crash window); record the
        `benches/ingest.rs` steady-state number from task 7. Add the
        missing 0012 row to `docs/decisions/README.md` (flagging the two
        missing 0011 rows to review). Retire the mirror-image debt note at
        `tests/persistence.rs:401-407` — the boundary closes it.
        — *Proved by:* the review/docs criterion — the amended §6 no longer
        contains the separate-serialization claim, the index lists 0012,
        the note no longer calls the race open debt; `grep` is the test.

- [ ] 7. Run every gate and record the cost: `cargo fmt --all -- --check`;
        `cargo clippy --all-targets -- -D warnings`; `cargo test` (real
        servers, no mocks); `brokkr compile --bundle bundles/self`;
        `GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh` (exact
        100/100/100, missed == 0 — every new production line must be
        executed by some test, else add the leg, never an exclusion);
        `cargo deny check licenses`; `cargo test --test burst`;
        `cargo bench --bench ingest -- --quick` for the number task 6
        records.
        — *Proved by:* all eight commands exit 0 on the final tree (AC6);
        `git show --stat` shows no new `src/` file and no new dependency.

- [ ] 8. Leave the tree reviewable: both decision documents carry status
        `proposed` (acceptance is the operator's), the spec's verify gate
        reproduces from a clean checkout, and the position files remain
        uncommitted under `.forge/`.
        — *Proved by:* `git status --short` shows no stray staged files;
        `git show --stat` names exactly the spec set and decision 0014.
