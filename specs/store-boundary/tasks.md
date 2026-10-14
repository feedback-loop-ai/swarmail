# Tasks — epic #13: one Store seam, two contracts

Ordered. Each task names the test that proves it. Decision
[0012](../../docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md)
is the ruling (status `proposed` — the operator accepts); this list is its
landing sequence. Scope guard: **no** refusal is added for
`delete`/`clear`/`clear_all` (their `record` calls become `let _ =`,
nothing else), **no** new `src/` item, type, dependency or test file is
created, and the two named debts (refused-delete resurrection;
mirror-ordering/epoch) stay out.

- [ ] 1. Rewrite the degraded test **first**, red against the current tree:
        in `src/store/persist_tests.rs`, replace
        `a_failing_data_file_never_blocks_the_store`'s body — install
        `fail_writes()`, `subscribe("box")` + `subscribe_all()` before the
        insert, assert `store.insert(email("f1", "box", "a@x.io")).is_err()`,
        keep the memory-truth block (`count("box") == 1`, counters) and the
        existing delete/clear/clear_all lines, and assert both channels are
        empty (`try_recv`) after the refused insert.
        — *Proved by:* `cargo test --lib a_failing_data_file_never_blocks_the_store`
        **fails on the unfixed store** (it returns the mail today) with the
        `is_err` assertion named in the failure (AC1, AC4, AC5 red).

- [ ] 2. Land contract #1 in `src/store.rs`: `record` returns
        `rusqlite::Result<()>` (log stays, then the `Err` propagates);
        `insert` becomes `Result<Arc<Email>, rusqlite::Error>` with `?` at
        the `record("insert", ...)` call (`:362`) — memory phase and
        counter before it, watcher + `global_tx` sends after it, untouched;
        `insert_async` inherits the `Result` through `offload`;
        `delete`/`clear`/`clear_all` take `let _ = self.record(...)` at
        `:473`/`:493`/`:521` and change no other behavior.
        — *Proved by:* task 1's test is now green (the `Err`, the memory
        truth and the quiet channels all asserted by it); the existing
        clear-race tests still pass; `cargo clippy --all-targets` shows the
        remaining `must_use` sites (the ripple, task 6).

- [ ] 3. Write the 8-thread barrier probe into the inline `#[cfg(test)]`
        `tests` mod of `src/store.rs` (0020 rule 1 — warm branches only):
        ~50 rounds, each a fresh `Store::new(0)`, eight threads — 3× insert
        on `A`, 1× delete on `A`, 1× clear on `A`, 2× insert on `B`,
        1× `clear_all` — joined per round; per round assert, for every
        inbox, `count(inbox) == |retrievable ids|`, that no accepted id is
        lost, and that no thread panicked through `join`.
        — *Proved by:* `cargo test --lib <probe>` **fails on the current
        tree** — record the red (the `count`/`get` disagreement and/or the
        `:464` expect panic) before task 4 lands; if no red is producible
        in a dense run, stop and investigate, never tune the assertion
        (AC7 red).

- [ ] 4. Land contract #2: one `mutation: std::sync::Mutex<()>` field on
        `Store`; acquire/release around the memory phase of `insert`
        (entry-guard block through `index.insert` + counter), `delete`
        (index remove through retain + watcher release) and
        `clear`/`clear_all` (the drains); release **before**
        `run_pre_mirror_gate()` and before `record`; guards acquired with
        `.expect("store mutation lock poisoned")`. No lock across the
        commit, an `await`, or a gate callback; readers untouched.
        — *Proved by:* task 3's probe is green under every schedule
        (AC7); the two clear-race tests
        (`a_clear_racing_an_insert_cannot_erase_the_inserts_row`,
        `a_clear_all_racing_an_insert_...`) pass **unmodified** — they are
        the deadlock guard for the release-before-gate ordering.

- [ ] 5. Add the forcing helper and the wire refusal legs, **red first**:
        `tests/common/mod.rs` gains `hold_write_lock(db) -> WriteLock` (a
        second real `rusqlite::Connection` on the data file holding
        `BEGIN IMMEDIATE` until drop). Legs: in `tests/persistence.rs` —
        SMTP `DATA` under the lock asserts a reply starting `451` with no
        `250` substring, the refused mail is still queryable and absent
        from a restart copy, then drop the lock and retry the same `DATA`
        → `250`; REST seed under the lock → `503` with the fixed body; in
        `tests/mcp.rs` — MCP seed under the lock → `isError: true`.
        — *Proved by:* the legs **fail on the current tree** (today's
        unconditional `250` / `200` / ok) — run them before task 6 and
        record the red (AC2, AC3, AC9 red).

- [ ] 6. Land the three caller refusals: `src/smtp.rs` DATA path matches
        the awaited result — `250 2.0.0 OK: stored as {id}` on `Ok`,
        exactly `451 4.3.0 storage commit failed` on `Err` (`:461-462`);
        `src/api.rs` seed maps the `Err` to
        `ApiError(StatusCode::SERVICE_UNAVAILABLE, "storage commit
        failed")` (`:454`); `src/mcp.rs` seed maps it to
        `error_result("storage commit failed")` (`:202`).
        — *Proved by:* task 5's three legs go green, including the
        retry-250 leg (the transient argument for `451`); the REST and MCP
        legs also assert the memory-truth side (the refused mail is still
        listed) (AC2, AC3).

- [ ] 7. The mechanical ripple, nothing more: `benches/ingest.rs`
        (`:36`, `:78`, `:117`) and `benches/writer_stall.rs` (`:124`,
        `:166`, `:182`) take the `Result` with a per-site `.expect(...)`
        naming the leg (memory-only stores cannot refuse); the ~50 inline
        `#[cfg(test)]` call sites across `src/` take `let _ =` (or
        `.unwrap()` where the test cares). No signature, shape or behavior
        change beyond `Result`.
        — *Proved by:* `cargo clippy --all-targets -- -D warnings` is
        clean (the `#[must_use]` warning is the ripple's completion
        detector) and `cargo bench --bench ingest` compiles and runs with
        the `insert@cap-steady` canary asserting `len == cap` and
        `dropped == inserted` (AC8, AC10).

- [ ] 8. Leave every gate green on the final tree and commit exactly the
        diff surface — `src/store.rs`, `src/smtp.rs`, `src/api.rs`,
        `src/mcp.rs`, `src/store/persist_tests.rs`,
        `tests/common/mod.rs`, `tests/persistence.rs`, `tests/mcp.rs`,
        `benches/ingest.rs`, `benches/writer_stall.rs` (the spec, plan,
        tasks and decision 0012 are committed by the chief):
        `cargo fmt --all -- --check`; `cargo clippy --all-targets --
        -D warnings`; `cargo test`; `brokkr compile --bundle .`;
        `GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh` (exact
        100/100/100, missed == 0); `cargo deny check licenses`.
        Confirm the scope guard in the diff: no refusal shape for
        `delete`/`clear`/`clear_all`, no new `src/` item or dependency,
        no `coverage(off)`, and the wire legs speak real protocol against
        real servers with no mocks.
        — *Proved by:* all six commands exit 0; `git show --stat` lists
        exactly the ten code files; `grep -rn 'coverage(off)' src/` is
        empty; the coverage summary shows every new production line
        (record's `Err` return, the `?`, the fan-out skip, the four lock
        regions, the three refusal arms) executed by its task's test
        (AC9, AC10).
