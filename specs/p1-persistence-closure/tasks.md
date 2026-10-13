# Tasks — the boundary stands, its invariant is stated, its busy bound is loud

Ordered; each task names the test that proves it. The only `src/` change in
the whole run is two tests in `src/persist.rs`'s existing inline module —
zero behavior lines. A red criterion at any step reopens the council (the
intake framing's rule); it is never fixed silently under this spec. Where a
command meets git, prefix `GIT_CONFIG_COUNT=0` (the harness env carries a
`GIT_CONFIG_KEY_0` that is missing; run 2's verify gate records the same
workaround).

- [ ] 1. Certify the premise before touching anything: run the red-detectors —
  `GIT_CONFIG_COUNT=0 cargo test --lib sixteen_threads_cannot_leave_memory_and_disk_in_two_orders`
  and the slice's wire legs (`cargo test --test persistence --test compat
  --test mcp --test pop3`). All green on HEAD `19cc49d` as of this design.
  **If anything is red, STOP**: the confirmation's premise is gone, the
  council reopens, and no task below runs. *Proved by:* the green outputs
  themselves (spec AC1, AC5).
- [ ] 2. Add the 0016 row to `docs/decisions/README.md` (0016 is committed
  with this spec, status `proposed`): title, one-line summary, status
  **proposed (awaiting operator)**, amends-0014/0015 note. *Proved by:* the
  row renders in the index; no accepted decision's row or text is touched
  (spec AC2).
- [ ] 3. Amend
  `docs/decisions/0014-one-ordered-mutation-boundary.md` **in place**: add
  §8, *The invariant the boundary buys* — the lag-invariant paragraph from
  [0016 §2](../../docs/decisions/0016-lag-invariant.md) — marked *(Added by
  [0016](0016-lag-invariant.md) during run 3; 0014 still awaits the
  operator)*. Nothing else in the file changes; status stays `proposed`.
  *Proved by:* review reads the diff against AC2 — today's §7-end is the
  red state (the invariant is greppable nowhere on HEAD).
- [ ] 4. Amend `docs/decisions/0015-boundary-failure-contract.md` §2 **in
  place** under an *(Amended by [0016](0016-lag-invariant.md))* marker: (a)
  disk exhaustion — ENOSPC/EDQUOT, a read-only remount — is an ordinary
  commit error answered by the fixed-text refusal on every surface after the
  same bounded stall, with zero new code; (b) a size cap and a `552` are
  refused, with the named cost (a hostile peer can fill the disk; the
  counter-move is a deployment-owned volume quota); (c) the bound is pinned
  by the fact tripwire of task 5, whose red state is any override reaching
  the server's own connection. Status stays `proposed`. *Proved by:* review
  reads the diff against AC2 — today's §2 silence about ENOSPC is the red
  state.
- [ ] 5. Write the busy-bound fact tripwire in `src/persist.rs`'s existing
  `#[cfg(test)] mod tests`: a `fn assert_dependency_busy_default(conn:
  &Connection)` that reads `PRAGMA busy_timeout` and asserts `5000` with a
  comment citing 0016 §4 / 0015 §2 and rusqlite 0.32.1
  `inner_connection.rs:119`; the green leg
  `the_opened_connection_keeps_the_dependency_busy_default` (real
  `Persist::open` via the existing `db_path` helper, cleanup like its
  siblings); and the `#[should_panic]` can-fire leg
  `the_busy_default_tripwire_fires_when_the_bound_is_overridden` (set
  `PRAGMA busy_timeout = 30000` on a throwaway `Persist`'s own connection,
  then call the helper). No sleeps, no wall-clock validity assertions, no
  new file, no seam. *Proved by:*
  `GIT_CONFIG_COUNT=0 cargo test --lib busy_default` — both legs green (AC3);
  the green leg's red state is any override reaching the server's own
  connection; the can-fire leg's red state is a tripwire that no longer
  trips.
- [ ] 6. Run the whole gate block and report it verbatim:
  `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`;
  `cargo test` (real servers, no mocks — subsumes tasks 1 and 5);
  `brokkr compile --bundle bundles/self`;
  `GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh` (exact 100%
  lines/branches/functions, missed == 0, the floor not lowered);
  `cargo deny check licenses`. *Proved by:* the gate outputs themselves
  (AC4, AC6) — with a `src/` diff of exactly the two tripwire tests.
