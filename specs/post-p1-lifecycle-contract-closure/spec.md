# The post-P1 findings closure: lifecycle, wire bounds, and the surface truth

Issue: #31 (post-P1 findings 1–6 · design run `close-the-remaining-post-P1-find-4a39f25f`)
Decision: **0018** — [shutdown-observed waits and explicit bounds](../../docs/decisions/0018-shutdown-observed-waits-and-explicit-bounds.md) (status *proposed*, awaiting the operator).

The P1 run (`specs/p1-persistence-closure/`) closed the storage-consistency
findings and left the boundary reused, never bypassed. Six findings remained
open — the JoinSet harvest, waits that ignore shutdown, unbounded wire values,
the silent MCP casts, thread chains welded together by their subjects, and
publication that can replace released bytes. Nothing here is red in a way a
gate can see today: every finding is a *missing* contract — an accumulating
set, a parked wait, an unadvertised refusal, a merge that never happens on the
wire. So this spec's verification posture is the P1 run's: **prove, don't
rebuild**, and where a contract is genuinely absent, write the smallest honest
one into the file that already owns it.

`store.rs` and `persist.rs` end this run at **zero diff from HEAD**. The P1
spec already ruled the successor slice reuses the boundary — compat bulk
delete is `delete_async` per id under the existing offload, *"never a batch
bypass"* — and both positions' strongest ideas (handler-level select, template
guards) keep that promise. Any design in this space that touches the boundary
has misread the framing.

## Rulings

Each ruling names the position that wins, what the losing position costs, and
the exact test that proves it. Position files: `.forge/design/positions/{simplicity,robustness}.md`.

**R1 — The JoinSet harvest: reap ready completions, never park the accept
loop** *(both positions converge on the line; they split on test placement —
simplicity wins)*.

`smtp.rs` and `pop3.rs` spawn sessions into a `JoinSet` and join only in the
shutdown arm, so completed entries accumulate with the cumulative connection
count. The fix is one line at the top of each accept-loop iteration:
`while sessions.try_join_next().is_some() {}` — non-blocking, never stalls
the accept path, never aborts a live session. The shutdown arm stays
byte-identical: `abort_all` + explicit drain; no session socket outlives the
serve loop; a session mid-DATA has no commit to land and gets no reply. Three
refused "fixes": a drain grace period on stop (an ack race at the exact
moment the operator believes the process stopped), an abort-on-timer leak
detector (it would bound a *running* stalled peer — the plaintext idle posture
is deliberate: "the peer goes away" is the bound), and harvesting only inside
the shutdown arm (the bug itself).

Test placement: **inline** (`src/smtp.rs`, `src/pop3.rs` `#[cfg(test)]`
modules). The harvest is hot-path presence — every connection completes — so
decision 0011 rule 1 applies: a branch-stable, deterministic probe driving the
real `pub async fn serve` on a real loopback listener, no mocks. The probe
installs a `#[cfg(test)]`-only private `REAPED: AtomicUsize` seam written by
the harvest line itself (rule 4: every seam keeps the test that installs it),
opens and closes a first connection, polls the seam to advance behind a **real
deadline** (the `spin_until` discipline — never a sleep), then a second
batch, and asserts the reap tracks completions. With the bug the counter never
advances and the deadline expires: red today, green tomorrow, measured on
every run. Cost of robustness's rule-2 harness placement
(`src/<module>/<name>_tests.rs`): refused — the probe needs no cold failure
path, and inline keeps the harvest line inside the 100/100/100 tribute, which
is precisely the exact gate's vacuity demand.

**R2 — Waits observe shutdown at the handler, on the watch the feed already
uses** *(simplicity wins the shape; robustness wins the discipline)*.

`await_messages` parks in `store.wait_for` while `stop()` waits for it — the
SIGINT repro: stop held for the remaining client timeout. The fix races the
wait against the existing `AppState.shutdown` watch at the handler: the same
receiver the SSE feed consumes, one source of "stopping", no second
cancellation channel. `McpContext` gains `shutdown` (one field, one
construction site — `mcp_endpoint`; no test constructs `McpContext`
directly) and the `swarmail_wait_for_email` arm selects identically. The
answer is `503` with the fixed text `shutting down` in REST and an
`error_result("shutting down")` in MCP — the run's one new wire string, in
the vocabulary both positions agree on. Not `408`: that lies that the client's
deadline elapsed. Not `200`: a "found" answer from a dying server lies about
what the caller may do next. `WaitOutcome` gains no variant; the store's
signature does not move.

The robustness discipline is adopted verbatim: **never a single entry check**
— a check-once-at-entry misses a stop that lands mid-wait and stop hangs for
the remaining timeout again. The shared primitive is
`pub(crate) async fn shutdown_signal(mut rx: watch::Receiver<bool>)`: at once
when the stop has already fired (`borrow_and_update`), else on the next
change — 0002's per-iteration re-check applied to the stop watch. It is
written branch-free of cold arms (sender-drop resolves as stopping, honestly —
the last sender is gone only when the server is going away) so the exact
coverage gate has no unreachable line to refuse, and the already-stopped case
answers without parking. Cost of robustness's in-store
`WaitOutcome::Shutdown` (losing side): refused — it reopens the boundary this
run must not reopen, churns a nine-call-site signature, and mints a second
"stopping" source to re-merge. Cost accepted: the store alone does not observe
shutdown; both wait surfaces are adapters today, and 0018 records that any
future wait surface must race the same watch at its adapter.

**R3 — `timeout_ms` is refused above the cap, never clamped** *(robustness
wins; simplicity's clamp loses)*.

`AwaitQuery::timeout` and the MCP parse are uncapped: `timeout_ms=86400000`
pins a task for a day. One `pub(crate) const MAX_AWAIT_TIMEOUT_MS: u64 =
60_000` in `api.rs` (the await's owner — not a CLI flag, not a knob) bounds
both. Above the cap: REST `400` with the fixed text `timeout_ms must be at
most 60000` *before subscribing* (a refused request never parks); MCP
`error_result` with the same text. OpenAPI `timeout_ms` gains `maximum:
60000` and documents the 400; the MCP input schema gains the bound so a
compliant client refuses at the tool layer. Cost of the clamp (the losing
side, named): a clamp is a narrowing on a wire value — the same disease as
the `as u8` casts this run removes — and a client that asked for an hour and
got 60 seconds silently is a lie by omission whose eventual `408` poisons
the timeout status's meaning. The refusal is loud; the breaking change is
the point.

**R4 — MCP `swarmail_set_chaos` validates; no narrowing casts on wire
values** *(robustness wins; simplicity's table doesn't contest it)*.

`probability` is `a.get("probability").and_then(as_u64).unwrap_or(0) as u8`:
missing, wrong-typed and 256 all become 0 — chaos silently off when the
operator asked for chaos. The arm refuses: presence and integer type via
`ok_or`, the 0..=255 range via `u8::try_from`, the 0..=100 contract via an
explicit upper-bound check — every refusal names the constraint
(`probability must be an integer between 0 and 100`), every arm of the
validation is warm under the wire probes (256, a string, 150, and a
successful 50). `delay_ms`: wrong-typed is an error naming the constraint;
absent defaults to 0 — honest "no delay". The input schema publishes
`minimum: 0, maximum: 100` on probability and the integer bound on
`delay_ms`. REST chaos needs nothing: `ChaosRule.probability: u8` already
rejects 256 at serde.

**R5 — Await totals: counted after the wait, in both arms** *(simplicity
wins; robustness's one-scan loses)*.

`api.rs` captures `total` before the wait, so a response can carry fresh
matches beside a pre-wait total. The count moves after `wait_for` returns,
into both arms; `matched` stays `want` (the requested count — correct).
MCP wait results gain the same `total` so both surfaces report the same
shape (the AC's fields-agree clause). Residual, documented in 0018 and the
README: the count is taken at response construction, not atomically with the
wait's last scan — a pair can still disagree by the inserts landing in that
window, bounded by one insert, invisible to any deterministic test, and the
honest reading of "the response's observed state". Cost of the one-scan
(losing side, named): refused — it edits `wait_for`/`WaitOutcome` inside the
boundary at zero-diff, and ripples a signature across its call sites to buy
snapshot-atomicity no deterministic test can distinguish from the documented
residual. A future run that wants the stronger invariant owes it a red
criterion and its own decision.

**R6 — Threads: explicit chains are authoritative; the subject fallback is
orphan-only; merges pay in docs, not alias state** *(both positions
converge; the deletion is simplicity's, the merge-key contract is
robustness's)*.

Pass 1 of `group_threads` welds every id-bearing mail's subject node into its
id-component — a transitive bridge that merges two unrelated chains whose
mails share a subject, on nothing but a string. The union is deleted (pure
deletion; the pass-1 comment is fixed with it). After: a mail with ids
threads by its References **even when the parent is absent from the inbox**
(absence does not authorize subject merging); a chain-less mail lands on its
subject node and groups with other orphans of the same normalized subject;
two explicit chains with equal subjects stay apart; two chain-less
same-subject mails still group (the fallback's kept, pinned behavior).
Subject affinity may group orphans; it may never override wire evidence.

A bridge mail referencing two existing threads merges them — honest; the
wire says one conversation — and retires one advertised key. The surviving
key is the merged thread's anchor (the existing derivation: oldest first
mail, `receive_ms` then id) — deterministic, already what `Thread.key`
computes. **No alias state**: the view is derived per request; a key→key map
is a table, a migration and a new invalidation invariant bought for a
presentation concern, and it would claim a stability the `VIEW_CAP` window
already denies (the view shifts as mail prunes). Every other pre-merge key
answers a loud `404`, never a wrong thread. Cost (named, operator-facing): a
client holding a pre-merge key sees 404 until it re-lists — loud,
recoverable, documented. Cost of refusing the merge instead: RFC 5256-style
threading is the wire's truth; refusing it would keep two conversations
split by an accident of the first mail's subject.

**R7 — Chart: the cap is enforced at render, the strategy is Recreate
always, annotations are annotations** *(robustness wins Recreate-always and
the render fail; simplicity wins "no values.schema.json")*.

- `replicaCount > 1` fails the render with a named refusal (the
  ServiceMonitor `{{- fail }}` precedent) plus a template comment stating
  why the cap exists (0008 §4: two serving stores behind one Service fork
  the mailboxes). `values.yaml` keeps `replicaCount: 1` — the only legal
  value; the guard is the enforcement. No `values.schema.json` (one
  invariant, three template lines, zero new files).
- The strategy is **Recreate for every profile**: the persistence-conditional
  is deleted and the default-profile assert changes with it. A RollingUpdate
  surge runs two serving stores behind one Service during the upgrade — the
  same lie the replica cap forbids, for memory and disk profiles alike.
  ADR 0008 (proposed) is amended in place (§4/§5) as a landing task of this
  run; its consequences paragraph already prices the restart-length gap.
- `service.annotations` moves out of `metadata.labels` into
  `metadata.annotations` — annotations are not labels.
- `scripts/chart-verify.sh` wraps its two bare discarded comparisons (the
  ServiceMonitor path and the Ingress path) in real `assert`s, asserts the
  Recreate default, asserts the annotations, and gains the negative leg: a
  `--set replicaCount=2` render must fail with the named refusal. The
  deliberate-mutation proofs that the asserts can fail are one-off verify-seat
  demonstrations, not a permanent broken-value matrix (refused).

**R8 — Publication: the gate and the bytes are one run, and version tags
publish only from tag refs** *(robustness wins; simplicity's registry probe
loses)*.

Both publish jobs gain
`needs: [quality, test, coverage, deny, dependency-audit]` — publication is
the last thing that happens in a run whose gates are green in **that** run;
a chart is verified against a binary built from the same tree. Version
tags (`:<Cargo.toml version>`) publish **only from `v*` tag refs**; `main`
pushes `:main` and `:sha-<sha>` only; the docker and chart publish steps
gate on `github.ref == 'refs/heads/main' || startsWith(github.ref,
'refs/tags/v')`. Cost of the registry-probe guard (losing side, named): it
races concurrent publishers and drags registry credentials into a job that a
workflow-level `if` enforces without them. Cost of winning: releases now
require pushing a git tag — an operator ritual change, licensed by the AC's
immutability demand, documented in the release notes. With `Cargo.toml` at
`0.1.0`, the probe alternative would send every main rebuild against the
same tag and go chronically red — the ritual is cheaper than the probe.

**R9 — The contract documents describe the server that ships** *(robustness
wins the framing; simplicity wins the minimal diff)*.

`openapi.json`: `Email` loses the removed `session` field; the await path
gains `from` and `since_ms` (the filters the server already honors), the
`timeout_ms` maximum, the 400 and the 503; the seed path gains the 503.
`mcp.rs` schemas: the wait tool gains `since_ms` and the timeout bound; the
chaos tool gains its bounds (R4). `llms.txt` and the README match: the await
cap, the 503, the thread fallback's exact scope and the merge-key clause
(the README's key-mobility list gains the merge as its fourth cause), the
stop budget. The disease is spec rot: a generated client that models the
spec meets a 503 it was told cannot exist and fails to deserialize it, and
an unadvertised filter produces *intermittent* awaits — the flakiest failure
mode a harness can export. Every refusal the server can emit on these
surfaces is documented on them.

**R10 — Compat bulk delete: prove, don't rebuild** *(simplicity wins, and
the P1 spec's successor-slice clause is the standing order)*.

Finding 2 is closed in code (`3b158cd`): `delete_messages` already offloads
per id through `delete_async` — the boundary is reused, not bypassed. The
run adds the wire regression the AC's second sentence never had:
`tests/compat.rs` holds the write lock (`hold_write_lock` — a real second
connection's `BEGIN IMMEDIATE`), issues a multi-id `DELETE` while a 10ms
timer canary ticks, and asserts the first failing id refuses `503` (0015's
contract: committed ids stay committed) within the busy bound plus margin —
proving the loop never serializes N×5s burns and the HTTP runtime stays
timely under the offload. `delete_many_async` is refused: a public-contract
change (all-or-nothing versus 0015's first-failure-503) with no red
criterion under this run's AC. Cost (named): a large bulk delete under
flapping contention trickles, each id bounded by the busy bound until the
first refusal ends it — documented as the boundary's shape, not a defect.

**R11 — The stop budget is stated, and an in-flight commit neither holds
stop nor half-lands** *(robustness's posture, adopted; the budget is the
operator-facing number both positions imply)*.

With R2, stop completes within a documented budget — **the SQLite busy bound
(5000ms) plus a scheduling margin (5000ms) = 10 seconds** — independent of
any client's wait timeout and of any commit's busy burn. A commit already in
flight at stop is not cancelled: the blocking pool runs it to completion; it
lands after the socket died (the mail is in the store, the client saw EOF,
not `250` — retry is lossless) or refuses; nothing half-lands. Aborting a
commit to hit a tighter budget is how a `250` stops meaning anything.
Proven by the lifecycle legs: stop-with-parked-await (budget + 503) and
stop-under-held-lock (stop returns well within budget while a commit burns
its busy timeout).

## Refused in this run (the anti-cut list)

1. **A session-supervisor module or shared harvest abstraction** — two
   one-line loops; a third listener is the refactor signal.
2. **A cancellation token, `ShutdownGuard`, or `wait_for_cancellable`** —
   the watch exists; the select is six lines in the handler.
3. **A configurable await cap** — one const; a knob doubles the test matrix
   for a bound no operator asked to tune.
4. **New status codes or response shapes for shutdown** — `503` + one
   string; the `408` arms keep their meaning.
5. **A batch/transactional bulk delete** — the per-id offloaded loop *is*
   the boundary (0014); a second mutation order is the thing 0014 exists to
   prevent.
6. **`values.schema.json`** — one invariant, three template lines.
7. **A thread-key alias map** — persistent state for a per-request view.
8. **Reopening the persistence boundary** — `store.rs`/`persist.rs` end at
   zero diff; any slice that touches them for these findings has misread
   the framing.
9. **New dependencies** — `Cargo.lock` stays untouched; the CI guard uses
   tooling the runner already has.
10. **Deliberate-broken-value CI matrices for chart-verify** — the negative
    render leg is permanent; the mutation proofs are one-off.
11. **A "shutting down" `WaitOutcome` variant** — the handler answers before
    an outcome is minted; `error_result` is the MCP vocabulary.
12. **Clamping any wire value** — refusals are the bound's honest form.

## Acceptance Criteria

1. **Completed session entries do not accumulate.** After successive
   connection batches against either listener, the accept loop has reaped
   every completed entry before its next accept — proven by the inline seam
   probes in `src/smtp.rs` and `src/pop3.rs` (real `serve`, real loopback
   connections, real deadlines), with the existing `tests/lifecycle.rs`
   mid-DATA legs still green (the shutdown arm is untouched).
2. **A parked await cannot hold stop hostage.** With a 60s REST await parked
   on an empty inbox and an MCP wait parked the same way, `stop()` completes
   within the documented 10s budget; the parked requests answer `503` /
   `error_result` with the fixed text `shutting down`; a request arriving
   after the stop answers without parking. `WaitOutcome` is unchanged and
   `src/store.rs` ends at zero diff.
3. **Wire bounds are explicit and tested.** `timeout_ms` above 60000 → REST
   `400` / MCP `isError`, each naming the maximum, before any subscribe;
   `timeout_ms` within 1..=60000 (and the 5000 default) behaves as today;
   OpenAPI and the MCP schema publish `maximum: 60000`. MCP
   `swarmail_set_chaos` refuses missing, wrong-typed and out-of-range
   `probability` (0..=100) and wrong-typed `delay_ms` with named error
   results; 0..=100 still sets the rule; no narrowing cast remains on any
   wire value; the published schemas carry the bounds.
4. **Await totals describe the response's observed state.** Delivering mail
   while a wait is pending yields matches and a `total` counted after the
   wait, in both the 200 and 408 arms; the MCP wait results carry the same
   `total`; the residual (one insert's race) is documented in 0018 and the
   README.
5. **The contract documents match the server.** `openapi.json` drops
   `Email.session`, advertises `from`/`since_ms` on the await path, the
   `timeout_ms` maximum, the await 400 and 503, and the seed 503; the MCP
   wait schema advertises `since_ms` and the bound; `llms.txt` and README
   agree; every refusal on these surfaces is documented on them.
6. **Explicit chains are authoritative.** Two chains sharing a subject stay
   apart; a subject-only mail never joins a chain; a reply threads by its
   References even when the parent is absent; chain-less same-subject mail
   still groups; a bridge merge yields one thread keyed by the surviving
   anchor and the retired key answers a loud 404; no alias state exists
   (no new tables, maps or migrations); the README thread prose states
   exactly this.
7. **The chart enforces its own contract.** `replicaCount=2` fails the
   render with a named refusal; the strategy is Recreate in both profiles;
   `service.annotations` land in `metadata.annotations`; chart-verify
   asserts all of it (bare comparisons wrapped) and stays green; ADR 0008
   is amended in place with the cap's rationale as a template comment.
8. **Publication is gated and immutable.** Both publish jobs `needs:` the
   five gate jobs; version-tagged images and charts publish only from `v*`
   tag refs; `main` pushes `:main` and `:sha-<sha>` only; a published
   version is never replaced by a branch build.
9. **The compat bulk delete is proven.** Under a held write lock, a
   multi-id `DELETE` refuses on the first failing id with `503` within the
   busy bound plus margin, committed ids stay committed, a 10ms timer
   canary stays timely throughout, and no `delete_many_async` exists.
10. **The boundary is untouched and the gates hold.** `git diff HEAD --
    src/store.rs src/persist.rs` is empty at the end of the run;
    `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
    warnings`, `cargo test`, `brokkr compile --bundle bundles/self`,
    `bash scripts/coverage-gate.sh` and `cargo deny check licenses` are all
    green.

## Verify gate

From a clean tree, with the harness's broken `GIT_CONFIG_KEY_0` sidestepped:

```sh
GIT_CONFIG_COUNT=0 cargo fmt --all -- --check
GIT_CONFIG_COUNT=0 cargo clippy --all-targets -- -D warnings
GIT_CONFIG_COUNT=0 cargo test
GIT_CONFIG_COUNT=0 .forge/bin/brokkr compile --bundle bundles/self
GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh
GIT_CONFIG_COUNT=0 cargo deny check licenses
GIT_CONFIG_COUNT=0 git diff HEAD --stat -- src/store.rs src/persist.rs   # empty
helm template charts/swarmail --set replicaCount=2 >/dev/null            # must fail
GIT_CONFIG_COUNT=0 bash scripts/chart-verify.sh
```

The coverage gate is the arbiter of R1–R5: every new production line runs on
every test invocation, and no `#[cfg(test)]` seam exists without the probe
that exercises it.
