# Tasks — the post-P1 findings closure

Ordered. Every task names the test that proves it; no task is done until its
proving test runs green against the change and red against its absence (or,
for prove-only tasks, until the proof is demonstrated). The boundary files
(`src/store.rs`, `src/persist.rs`) are never opened.

- [ ] Delete the subject-union in `src/threads.rs` pass 1 and fix its
  comment — explicit chains authoritative, subject fallback orphan-only.
  *Proving test*: the inline threads tests — two same-subject chains → two
  threads; a bridge mail → one thread keyed by the surviving anchor with the
  retired key absent; chain + subject-only mail with equal subjects → apart;
  two chain-less same-subject mails → together (all four in
  `src/threads.rs` `#[cfg(test)]`).
- [ ] Add the harvest to both accept loops (`src/smtp.rs`, `src/pop3.rs`):
  `while sessions.try_join_next().is_some() {}` at the top of each
  iteration, with the `#[cfg(test)]` `REAPED` seam written by the loop.
  *Proving test*: the inline seam probes — real `serve` on a loopback
  listener, two successive connection batches, the counter tracked behind a
  real deadline (one probe per module).
- [ ] Add `MAX_AWAIT_TIMEOUT_MS` and `shutdown_signal` to `src/api.rs`;
  refuse over-cap `timeout_ms` with `400` before subscribing; `select!` the
  REST await against the signal; move the `total` count after the wait in
  both arms. *Proving test*: `tests/api.rs` — the 60001 → `400` probe, the
  60000-passes probe, the parked-await-then-stop → `503` leg, and the
  stale-total leg (deliver mid-wait, assert `matches` and post-wait
  `total`).
- [ ] Give `McpContext` the `shutdown` receiver (one construction site) and
  rework the `src/mcp.rs` wait arm: the same cap refusal, the same select,
  `total` in both results. *Proving test*: `tests/mcp.rs` — the parked
  wait-then-stop → `isError("shutting down")` leg, the 60001 → isError
  probe, and the wait result's `total`.
- [ ] Validate `swarmail_set_chaos` in `src/mcp.rs` (presence →
  `u8::try_from` → `<= 100`, named refusals; `delay_ms` wrong-typed →
  named error) and publish the schema bounds. *Proving test*: `tests/mcp.rs`
  — 256 → named error, `"high"` → named error, 150 → named error, 50 → rule
  set, `delay_ms: "soon"` → named error.
- [ ] Add the wire legs to `tests/lifecycle.rs`: stop-with-parked-REST-await
  (503 + budget), stop-with-parked-MCP-wait (isError + budget), and
  stop-under-held-lock (the busy burn does not hold stop; nothing
  half-lands). *Proving test*: those legs themselves — red on HEAD (stop
  held for the remaining timeout), green after task 3/4.
- [ ] Add the compat bulk-delete regression to `tests/compat.rs`:
  `hold_write_lock` + 10ms timer canary + multi-id `DELETE` → first failing
  id refuses `503` within the busy bound + margin, committed ids stay
  committed, the canary stays timely. *Proving test*: that leg (green
  against today's already-offloaded loop — it is a proof, not a rebuild).
- [ ] Enforce the chart contract: the render-time replica fail with its
  rationale comment, `strategy: Recreate` unconditionally, and
  `service.annotations` in `metadata.annotations`. *Proving test*:
  `scripts/chart-verify.sh` — the wrapped asserts, the Recreate default
  assert, the annotations assert, and the `--set replicaCount=2` negative
  render leg.
- [ ] Gate publication in `.github/workflows/ci.yml`: `needs:` the five gate
  jobs on both publish jobs; version tags only from `v*` refs; `:main` +
  `:sha-<sha>` from main. *Proving test*: `actionlint` (if present) plus a
  rendered-workflow inspection committed in the PR description — the
  conditions are declarative and verified by review + the next release.
- [ ] Sync the contract documents: `src/static/openapi.json` (drop
  `Email.session`; await `from`/`since_ms`/`maximum`/400/503; seed 503),
  `src/static/llms.txt` and `README.md` (cap, 503, budget, thread fallback
  scope, merge-key mobility clause). *Proving test*: `cargo test` (the
  embedded documents compile), the doc-content probes in `tests/api.rs`
  asserting the served OpenAPI now carries the new fields and no `session`.
- [ ] Amend ADR 0008 in place (§4/§5: the render guard, Recreate always) and
  write `docs/decisions/0018-shutdown-observed-waits-and-explicit-bounds.md`
  (status **proposed**) with its `docs/decisions/README.md` index row.
  *Proving test*: the decision file exists with `- **Status:** proposed`,
  referenced by `specs/post-p1-lifecycle-contract-closure/spec.md`.
- [ ] Run the full gate: `GIT_CONFIG_COUNT=0 cargo fmt --all -- --check`,
  `GIT_CONFIG_COUNT=0 cargo clippy --all-targets -- -D warnings`,
  `GIT_CONFIG_COUNT=0 cargo test`,
  `GIT_CONFIG_COUNT=0 .forge/bin/brokkr compile --bundle bundles/self`,
  `GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh`,
  `GIT_CONFIG_COUNT=0 cargo deny check licenses`, and the empty
  `git diff HEAD --stat -- src/store.rs src/persist.rs`. *Proving test*:
  the gates themselves; the coverage gate is the arbiter of the seams and
  the new arms.
