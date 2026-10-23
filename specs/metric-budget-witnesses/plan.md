# Implementation plan: metric-budget-witnesses

## Technical context

One Rust crate using Tokio, real SMTP/HTTP/MCP integration clients and rustls
sinks. No new dependency or production behavior. Registration metrics are
static process-wide cumulative counters, whereas webhook ledgers belong to
individual servers. `tests/common::start` disables inbox retention with
`max_per_inbox: 0`. Existing list responses provide stored message identity.
The existing helpers and tests are the implementation owners.

## Constitution check

Preserve synchronous acceptance/storage, subscribe-before-scan, oldest-only
pruning and ingest extraction. Public protocol tests stay in `tests/`; the
branch-stable key test stays inline and measured under 0020. Add no runtime
seam, feature flag, exclusion, support crate or dependency allowance. Only
changed assertions and test plumbing are authorized; discovering a needed
runtime or semantic repair requires returning the scope to triage. No new
ADR is needed for this plan. The house-rules gate inventory remains the
single authoritative list; every required local and CI proof remains owed.

## Research and resolved questions

Inspection of the two positions, intake and current code establishes:

- `common::metric_row_value` returns None for absent rows, so before/after
  `unwrap_or(0)` can pass when both observations lost their instrument.
  Existing strict `next177_registered` readers provide the post-control
  pattern; their absolute setup assertions need measured baselines too.
- Valid zero-deadline waits register before scanning under accepted 0002;
  a nonmatching filter yields an immediate legal timeout even with seeded mail.
- Retry exhaustion increments after the last delivery await. Keep that
  observation before live sink loads. An atomic read alone has no finality.
- Every zero-window helper exits before accepting; its capture sender is
  owned inside the spawned listener closure. No accepted-handler clone remains
  to retain that sender, so cancellation observes the terminal release.
- The current batch fixtures reuse subjects. Index them to prove the accepted
  set, and keep their independent SMTP inputs as the expected set. Retention
  is already disabled; list pagination must be explicitly large enough.
- No plan-template file exists in this checkout's `.specify/templates/` or
  bundle resources (only the spec template is present). This plan supplies
  the requested technical context, constitution check, structure, research,
  data model, contracts and quickstart directly, without adding template files.

## Project structure and files touched

| File | Planned implementation |
|---|---|
| `tests/api.rs` | nocap positive control; strict refusal rows; baseline-based malformed-timeout control |
| `tests/mcp.rs` | nowaiter/capbox controls; observed408 baseline; repair shared local registration setup |
| `tests/webhook.rs` | five required storage witnesses; indexed batch identities; three-element sink tuples and direct loads; zero-window cancellation proof; all helper callers |
| `src/webhook.rs` | only `sink_keys_normalize_and_fall_back` assertions, inline under cfg(test) |

Runtime modules, `tests/common/mod.rs`, manifests, lockfiles, bundle recipes,
gates, accepted decisions and unrelated specs are reference-only. Keep any
readback helper concrete and local to `tests/webhook.rs`. The design commit
contains only this directory's spec.md, plan.md and tasks.md.

## Data model and contracts

There is no schema or public API change. A registration observation consists
of an exclusive inbox label and two counter snapshots. Absence is allowed
only before its positive control; every later observation requires presence.
The positive control adds one registration; each refused operation adds zero.
Audit label ownership across each test binary so concurrent tests cannot
contribute to the same delta. Preserve per-server identity for all observations.

A storage witness compares an expected SMTP fixture (recipient, subject,
text) with `/api/v1/inboxes/<inbox>/messages?limit=<sufficient-limit>`.
Assert HTTP 200, response total, returned length, full fixture identity set
and distinct nonempty ids; do not depend on response order. Singles have one
fixture. Batches use unique indexed subjects, with 50 and cap+8 fixtures
respectively. A small local assertion helper may centralize this exact
contract; it must make one request and never poll for eventual visibility.

Sink helpers retain `(SocketAddr, Receiver<Capture>, Hits)` with the existing
capture type for each transport. Cancellation is a closure witness only when
no connection was accepted; exhaustion remains the retry finality witness.
The listener lifetime and per-connection tasks are unchanged. Key equality
uses the existing parsed URL type and does not implement normalization anew.

## Implementation sequence

1. Capture baseline test evidence and verify label uniqueness. Repair the
   existing local registration setup to measure +1, allowing absence only on
   its initial scrape. Reuse it for nocap, nowaiter and capbox with a
   nonmatching zero-timeout wait and explicit successful timeout assertions.
   Snapshot the refusal baseline after that control and keep strict reads.
   Repair observed408 before-spawn baseline without disturbing readiness.
2. Add single-mail readbacks immediately after SMTP success in the HTTP 500,
   untrusted TLS and trusted TLS 500 tests. Verify the independent filtered
   fixture in its own inbox in the HTTP case. Add one complete read after each
   batch with indexed identities. Do not insert per-message REST calls into
   the capacity burst. Preserve budget snapshots and assertions; no readback
   may replace them or wait for eventual insertion. Retain existing before/
   after-exhaustion witnesses; a new post-exhaustion read is required only if
   a test claims survival across exhaustion as well as immediate visibility.
3. Remove count channels and `hits()` consistently, repair all tuple callers
   and comments, and retain the capture receivers in the zero-window test.
   Await their cancellation under bounded deadlines before the zero loads.
   Keep acceptance retries and sink lifetime unchanged, including the existing
   transport retry backoff (it is not a new synchronization sleep).
4. Extend the inline key test with equivalent spelling equality and a distinct
   path inequality; retain variant/fallback assertions. Use branch-stable
   assertions so every added measured line executes on the normal run.
5. Run targeted tests and the full affected suites. Perform each AC6 mutation
   separately against the changed tests, record command/source identity and
   intended failure, restore that mutation, then prove restored green. An
   omitted-fixture control must reach the storage identity/count assertion
   after successful acceptance, not fail merely because SMTP refused. Use a
   temporary local mutation or controlled deletion of the accepted fixture
   before the read; keep the intended readback failure explicit. No permanent
   production seam or mutation framework is justified.
6. Restore the clean implementation, run all house-rule gates and fail-closed
   native-tls proof through the required pinned discipline, then protected
   verification and four-lens review. Bind receipts to the tested source;
   distinguish inherited/failing/missing evidence from current passes.

## Quickstart and proof handoff

Before compile or any provider dispatch, verify exactly:

```bash
sha256sum /home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-178-luna1/.forge/bin/brokkr
```

Required digest:
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Stop dispatch on missing/mismatched binary; no PATH or sibling substitute.
Use that absolute binary for `compile --bundle bundles/self` and engine checks,
and this candidate's own target directory for all builds.

Focused implementation commands include:

```bash
cargo test --locked --test api the_wait_refuses_over_the_cap_registers_no_waiter_and_accepts_the_boundary
cargo test --locked --test mcp the_wait_registers_no_waiter_when_staging_refuses
cargo test --locked --test mcp the_staging_refuses_over_the_cap_and_honors_the_boundary
cargo test --locked --test mcp wait_results_carry_the_post_wait_total_in_both_arms
cargo test --locked --lib sink_keys_normalize_and_fall_back
cargo test --locked --test webhook
cargo test --locked --test api --test mcp
```

Also run the modified malformed-timeout/helper callers explicitly when
collecting their counter controls. The full gate inventory is
[house rules](../../docs/house-rules.md#gates--part-of-every-change), not a
subset of these commands. Hermetic verify executes pinned bundle copies and
checks public shims under 0050; candidate-local checks cannot replace its
actual dispatch. Design's deterministic check identifies the final design HEAD.

External evidence remains pending for CI-owned chart contract/history/bump
selftest, real-Docker persistence/context/volume smoke, RustSec audit and the
real port-80 bridge exchange. Tag-only release-rate-5k proof remains required
when publishing applies; this run authorizes no tag or publication. Do not
report whole-issue completion while required local/CI proof remains missing.

## Risks

| Risk | Mitigation and retained limit |
|---|---|
| Renamed or nonincrementing metric passes a zero check | Same-label +1 control, strict later rows and discriminating mutation |
| Concurrent writer contaminates a cumulative delta | Audit exclusive inbox labels; retain process and label identity; no reset/global lock |
| Liveness work accidentally enters the refusal delta | Establish a fresh baseline after the valid wait completes |
| Count-only or paginated read hides lost/duplicated mail | Explicit limit, total/length, complete indexed identities and distinct ids |
| Readback misses an acceptance gap by waiting for delivery | One immediate REST read after sends, before exhaustion/budget waits; never visibility polling |
| Extra work crosses capacity exhaustion horizon | One batch read, no per-mail queries; preserve exact arithmetic and report observed scheduling failures rather than widen tolerances |
| Atomic zero or four is read before it is final | Capture cancellation at zero window; new exhaustion before retry hit loads; fifth-attempt mutation |
| Helper cleanup alters TLS, redirects or capture lifetime | Exercise every caller in the full webhook suite, preserving trust, generated certs, drop-first and no-leak tests |
| Inline key test creates a coverage escape | Keep branch-stable assertions inline; exact line/branch/function gate unchanged |
| Claims exceed the witnesses | Retain crash/restart, hostile DATA, chaos and shutdown proof owners; no new durability or arbitrary-load claim |

## Evidence status

This design records requirements and inspected mechanisms, not implementation
or mutation success. Implementation receipts, all required local gates,
protected verification/review and CI proofs must be evaluated on the final
candidate. The design result is `drafted`; only the deterministic sequence
check rules on advancement.

### Design-seat baseline checks — 2026-10-24

On the inherited runtime/test source at 9b15fda, with only this design added,
format, locked Clippy, locked workspace tests, digest-verified candidate-local
Brokkr compile, coverage selftest, publish-guard proof, seal-branch keyless
proof, license check and fail-closed locked native-tls absence all passed.
Exact coverage passed on retry: 12,953/12,953 lines, 762/762 branches and
1,437/1,437 functions (`target/coverage/run.zm7uU1`). The first coverage
invocation exceeded the design seat's 120-second allowance and has no passing
verdict; its timeout is retained separately from the successful longer run.
Run-local command logs are in `.brokkr/state/design/chief-checks/` and are
uncommitted evidence. These are baseline checks, not proof of the still
unimplemented assertions or mutation controls. Final implementation gates,
actual protected verification/review and external CI remain outstanding.
