# Tasks — epic #13 wave 9 (#12): benchmark capped storage, stop over-allocating on counts and limited views

Ordered; each task is paired with the test that proves it. Tasks 1–4 are independent of
the fixes (the bench measures the *current* tree honestly — its steady-state canary passes
pre-fix, which is what makes the `before` column trustworthy). Tasks 5–8 are the fixes;
9–10 close the wave. The six gates run clean after every task that touches `src/`.

- [ ] 1. Add the `prefill(cap)` helper and the steady-state capped ingest bench
      `store/insert@cap-steady` to `benches/ingest.rs` (existing `ingest` group): hardcode
      the cap `100_000` with a comment tying it to `src/config.rs:46`; prefill outside
      `b.iter`; measure inserts from `i = cap` onward; after sampling assert the sampled
      inbox length is exactly the cap and the `emails_dropped` delta equals the
      `emails_inserted` delta (panic with a drift-naming message otherwise — the canary is
      documented as proving the measurement's honesty, not the fix). Leave the uncapped
      `store/insert` and `build_email/parse+extract` benches byte-identical.
      *Proves AC1, AC2.* Test: `cargo bench` runs all ids with no `probe` group and the
      asserts pass on the pre-fix tree; record the `before` numbers for the PR body.
- [ ] 2. Add the capped query benches `threads/view@cap`, `store/count@cap` (default
      filter) and `store/count@cap-filtered` (a needle filter matching nothing — the
      pure-waste regime) on the same prefilled store; doc comments name the regime and say
      memory-only, "no `--data-file`".
      *Proves AC3.* Test: `cargo bench` publishes all three ids; record the `before`
      numbers (~1.20 ms view, ~1.12 ms count; the filtered row is the regime's first ever
      measurement).
- [ ] 3. Swap the per-inbox container to `VecDeque<Arc<Email>>` in `src/store.rs`:
      `push_back` in `insert` and in `Store::open`'s restore (insertion order, front =
      oldest), a general `pop_front` eviction loop (preserving `overflow > 1`) under the
      same shard write guard, and the `drain_inbox` signature. `list`, `list_all`,
      `delete`'s `retain`, `clear`/`clear_all` and `inboxes()` compile unchanged.
      *Proves AC6 (mechanism).* Test: the existing settlement tests stay green —
      `insert_prunes_oldest_beyond_cap` (cap 2), `cap_pruning_is_mirrored_to_the_data_file`,
      `totals_counters_are_readable`, `tests/burst.rs` (exact counts under churn) — plus
      task 4's new pin.
- [ ] 4. Pin the restore-over-cap catch-up: a data file written under a higher cap,
      reopened under a lower one, trimmed exactly to the lower cap by the first insert,
      with `emails_dropped` counting the excess and order intact (oldest evicted first).
      *Proves AC6 (edge).* Test: new unit test in `src/store.rs`'s test module
      (`restore_over_a_lowered_cap_trims_on_the_first_insert`), exercising the general
      `pop_front` loop's `overflow > 1` branch.
- [ ] 5. Rewrite `Store::count` (`src/store.rs:390-392`) to a clone-free scan with the same
      signature — `list.iter().filter(|e| filter.matches(e)).count()`, `None => 0` — no
      `Arc` clones, no `Vec`.
      *Proves AC4.* Test: existing count tests (`get_delete_and_clear_edge_cases`,
      `inboxes_are_isolated`) stay green plus a new pin
      (`count_matches_list_len_for_a_non_trivial_filter`): seeded inbox,
      `count(inbox, filter) == list(inbox, filter).len()` for a needle filter that
      matches a subset — decision 0010's exactness, now allocation-free.
- [ ] 6. Add `list_bounded(inbox, filter, cap)` beside `list` in `src/store.rs`
      (newest-first, filter applied before the bound, doc comment stating the order) and
      rewire `threads::view` to `list_bounded(inbox, &Filter::default(), VIEW_CAP)`.
      *Proves AC5.* Test: new unit tests in `src/store.rs` — cap respected with the newest
      kept; filter-then-take across the boundary (a filtered inbox where the newest `cap`
      *matches* — not the newest mails filtered down — equals `list(...).take(cap)`);
      missing inbox → empty. Existing `the_view_caps_at_the_newest_view_cap_emails` and
      `the_view_of_an_unknown_inbox_is_empty` stay green.
- [ ] 7. Change `EmailList.emails` to `Vec<Arc<Email>>` and delete the `(*e).clone()` map
      in `list_messages` (`src/api.rs`), taking `total` off the match Vec before the move.
      *Proves AC7 (mechanism).* Test: new unit pin in `src/api.rs` — the serialized page
      is byte-identical to the `Vec<Email>` shape and carries no `raw` key; all existing
      HTTP tests (`tests/api.rs`: `first_id`'s listing, `list_all_orders_desc_with_a_deterministic_tie_break`,
      the compat and shim suites) stay green, proving the wire unchanged.
- [ ] 8. Re-run the committed benches on the fixed tree and write the honest result: the
      after numbers into the PR-body table; if any fix shows no measurable win, skip it and
      report its number instead. Update the README benchmark row with the steady-state
      capped figure beside the retained uncapped control, labeled memory-only,
      "no `--data-file`".
      *Proves AC9.* Test: `cargo bench` before/after table in the PR body; the README row
      matches the committed bench's after numbers (review-checked); the uncapped row is
      unchanged.
- [ ] 9. Verify the untouched surfaces and the closed list: `wait_for`, `Filter`,
      `list_all`, the Mailpit shim, `AwaitResponse`, the persist path — `git diff` names
      only the four files plus README; the probe group is absent.
      *Proves AC10, AC1 (no probe ships).* Test: `tests/p2.rs`, `tests/rate.rs`,
      `tests/compat.rs`, `tests/persistence.rs` green unchanged; `git diff --stat` shows
      the expected four files + README.
- [ ] 10. Full gate pass: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
      warnings`, `cargo test`, `brokkr compile --bundle .`, `bash scripts/coverage-gate.sh`
      (exact 100/100/100 — every new production line executed by its test, no
      `coverage(off)`), `cargo deny check licenses` (no dependency added).
      *Proves AC8.* Test: the six gates, run in order, all green.
