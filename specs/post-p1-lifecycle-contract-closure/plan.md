# Plan — the post-P1 findings closure

HOW for [spec.md](spec.md). Ordering is bottom-up by blast radius: pure
functions first, then adapters, then wire tests, then packaging and
publication, then docs. Every step carries its proving test (see
[tasks.md](tasks.md)); the boundary files are never opened.

## 1. Threads — the deletion (pure, isolated)

`src/threads.rs` pass 1: delete `components.union(first, subject_node)` and
fix the pass-1 comment (the component is the explicit-chain ids only).
`thread_node`'s orphan fallback is untouched — chain-less mail still lands on
`subject:<normalized>`; `thread_key` and `Thread.key` (anchor = oldest first
mail, `receive_ms` then id) are untouched — the anchor derivation is already
the surviving-key contract after a merge.

Inline tests (the existing `#[cfg(test)]` module; `group_threads` is pure, no
seam):

- two chains whose mails share a normalized subject → **two** threads (red
  today — the union merges them for the wrong reason);
- a bridge mail referencing both chains → **one** thread whose `key` equals
  the surviving anchor's hash; the retired anchor's key is absent from the
  view (red today);
- a chain mail and a subject-only mail with equal subjects → **apart** (red
  today);
- two chain-less mails with the same subject → **together** (green — pins the
  fallback's kept behavior against over-deletion).

No alias state: nothing keyed per request, no store change, no migration.

## 2. Adapters — the wait contract

`src/api.rs`:

- `pub(crate) const MAX_AWAIT_TIMEOUT_MS: u64 = 60_000;`
- `pub(crate) async fn shutdown_signal(mut rx: watch::Receiver<bool>)`:
  branch-free of cold arms —
  ```rust
  if !*rx.borrow_and_update() {
      let _ = rx.changed().await;
  }
  ```
  At once when the stop has already fired (`borrow_and_update`), else on the
  change; a dropped sender resolves as stopping (honest: the last sender is
  gone only when the server is going away). No cold line for the exact gate;
  no dependence on clone/version semantics.
- `await_messages`: refuse `timeout_ms > MAX_AWAIT_TIMEOUT_MS` with `400`
  `timeout_ms must be at most 60000` **before** subscribing; then
  `tokio::select!` the wait against `shutdown_signal(state.shutdown.clone())`;
  the stop arm is `Err(ApiError(SERVICE_UNAVAILABLE, "shutting down".into()))`;
  the outcome arms move the `total` count **after** the wait (both arms) and
  keep `matched: want` / `matched` as today.
- `McpContext` gains `pub shutdown: watch::Receiver<bool>`; the single
  construction site (`mcp_endpoint`) passes `state.shutdown.clone()`.

`src/mcp.rs`:

- the `swarmail_wait_for_email` arm: the same cap refusal (error_result with
  the same text) and the same select against
  `shutdown_signal(ctx.shutdown.clone())`; its Found/Timeout results gain
  `total` counted the same way;
- the `swarmail_set_chaos` arm: `probability` via presence check
  (`ok_or`) → `u8::try_from` (0..=255) → explicit `> 100` refusal; the
  message is `probability must be an integer between 0 and 100`;
  `delay_ms` present-but-wrong-typed → `delay_ms must be a non-negative
  integer`, absent → 0; the input schemas gain the bounds.

`src/smtp.rs` / `src/pop3.rs` (harvest, R1):

- top of each accept-loop iteration: `while sessions.try_join_next().is_some() {}`;
- a `#[cfg(test)] static REAPED: AtomicUsize` incremented inside the loop
  body (test-only writer; the seam's only reader is the probe);
- inline probes: bind a loopback listener, `tokio::spawn(serve(...))` with a
  never-firing shutdown future and defaults for the rest, connect + close,
  `spin_until`-style poll (real deadline) until `REAPED` advances; repeat
  for a second batch. The shutdown arm and its comment are untouched.

`Cargo.toml` / `Cargo.lock`: **no changes** — no new dependencies anywhere.

## 3. Wire tests

`tests/lifecycle.rs` (reuses `start_with`, `http_json`, `spin_until`):

- `a_parked_await_does_not_hold_stop_hostage`: open the HTTP surface, fire
  `GET /await?timeout_ms=60000` on an empty inbox, then `server.stop()`
  inside `tokio::time::timeout(BUDGET, ...)` with `BUDGET = 10s`; assert the
  parked request answered `503` with the fixed text and stop completed;
- the MCP twin: park `swarmail_wait_for_email` (`timeout_ms: 60000`) over
  `POST /mcp`, stop, assert the `isError` result with the fixed text;
- `stop_does_not_wait_for_an_in_flight_commits_busy_burn`: file-backed
  store, `hold_write_lock(data_file)`, drive a delivery to its commit, stop,
  assert stop completed well inside the budget (the busy burn did not hold
  it) and nothing half-landed (a fresh open sees a consistent store).

`tests/api.rs`: cap probes (60000 passes, 60001 → `400` with the fixed
text); the stale-total leg (park an await, deliver mid-wait, assert the
response's `matches` and post-wait `total` describe the same state — green
only with the fix; red today because `total` was captured pre-wait).

`tests/mcp.rs`: chaos probes (256, `"high"`, 150 → named errors; 50 → the
rule is set and echoed; `delay_ms: "soon"` → named error); the wait cap
probe; the wait result's `total`.

`tests/compat.rs` (R10): `hold_write_lock`, a 10ms-tick timer canary task,
a multi-id `DELETE /api/v1/messages` → `503` within the busy bound + margin,
committed ids stay committed, the canary never stalls past its tick window.

## 4. Chart and verify

`charts/swarmail/templates/deployment.yaml`: the render-time fail above one
replica (printf with the refusal text and the 0008 §4 rationale in the
comment), `strategy: Recreate` unconditionally. `service.yaml`: annotations
to `metadata.annotations`. `scripts/chart-verify.sh`: wrap the two bare
comparisons in `assert`s; default-profile assert → `Recreate`; add the
`--set replicaCount=2` negative render leg (expects nonzero exit + the named
refusal); assert annotations placement.

## 5. CI

`.github/workflows/ci.yml`: `docker` and `chart` gain
`needs: [quality, test, coverage, deny, dependency-audit]`; publish steps
gate on `github.ref == 'refs/heads/main' || startsWith(github.ref,
'refs/tags/v')`; the tag branch of the tag matrix emits `:<version>` only
from tag refs; `main` emits `:main` and `:sha-${{ github.sha }}` only.

## 6. Docs and decision

`src/static/openapi.json` (R9 edits), `src/static/llms.txt`, `README.md`
(await cap + 503, thread fallback scope, merge-key mobility clause, stop
budget), ADR 0008 amended in place (§4/§5 — Recreate always, the render
guard), new `docs/decisions/0018-shutdown-observed-waits-and-explicit-bounds.md`
(status **proposed**) plus its `docs/decisions/README.md` index row.

## Risks

1. **A cold arm under the exact coverage gate.** Any `return` reachable only
   when the server is already stopped would be an uncovered line the gate
   refuses. Mitigation: `shutdown_signal` is branch-free of cold arms by
   construction; the cap and chaos refusals each have a warm wire probe per
   arm (256 / `"high"` / 150 / 50; 60000 / 60001). If the gate still names a
   line, restructure the arm — never add a `#[cfg(test)]`-only caller.
2. **The seam probe is timing-sensitive.** `REAPED` must be polled behind a
   real deadline (`spin_until`-style), never `sleep`; a CI stall surfaces as
   a deadline expiry with a clear message, not a hang. The probe drives the
   real `serve` — if its future needs a TLS or chaos value, use the crate's
   defaults; if `serve`'s signature makes that awkward, bind via the same
   constructor the module's other tests use.
3. **The stale-total leg could flake as a *false green*.** Delivering
   mid-wait must land before the wait's final scan; the leg parks on an
   empty inbox with a generous timeout, delivers, and asserts the 200 — if
   the response ever carries a pre-wait total again, `total` is 0 with
   `matches` non-empty and the assert fires. Deterministic by construction
   (subscribe-before-check, 0002).
4. **MCP `total` is a new response field.** Surfaces-agree is an explicit AC;
   the field is additive and documented in the same slice. A client asserting
   the old exact shape is a test-suite client, updated in the same run.
5. **`Recreate`-always tightens upgrades for memory-profile users.** The
   cost is priced in 0008's consequences paragraph and the template comment;
   the alternative (two stores behind one Service during a surge) is the lie
   the replica cap forbids.
6. **CI tag-ref gating changes the release ritual.** Operators must push
   `v<version>` to publish. Documented in the release notes; `:main` and
   `:sha-<sha>` remain continuous. With `Cargo.toml` at 0.1.0 the
   registry-probe alternative would go chronically red — the ritual is the
   cheaper failure mode.
7. **The compat canary could false-alarm under load.** The canary asserts a
   tick window (deadline-based, like the stalled-sink legs), not a latency
   percentile; its budget is the busy bound's margin, far above scheduling
   noise. If a runner is slow, the margin — not the assertion — is tuned.
8. **Decision 0018 is proposed.** Its acceptance is the operator's; the run
   commits the spec and the decision draft, never flips the status.
