# Implementation plan: deterministic timing proofs

Implements [spec.md](spec.md). This is a design handoff, not implementation
or gate evidence.

## Technical context

One Rust crate, edition 2024. Tokio 1.53.2 and Axum 0.8.9 are already locked;
futures-util, watch/oneshot/Notify, public API router/AppState and MCP
dispatch/McpContext are available. Tokio test-util is not enabled and Tower
is not a direct dependency. Add neither. SMTP and POP3 each own an actual
JoinSet and one shutdown future. The existing HeldCommit harness supplies
the admission/release barrier for 9.6.

The repository has no .specify/templates/plan-template.md. This plan uses
the mandated technical context, constitution check, project structure,
research, data model, contracts, quickstart and risks sections, as the
existing listener-recovery plan does. Research/model/contracts stay here;
the design commit contains only the three requested artifacts.

## Constitution check

Conform to accepted 0020 placement and 0035/0050 exact-coverage rules:
branch-stable listener proofs remain inline, public-entry/wire tests stay
in their integration files, and persistence stays in its existing harness.
No feature flag, ignored test, production scheduler hook, coverage exclusion
or harness migration. Installed and no-observer arms must both execute.

Production ordering and budgets remain unchanged. No timer surrounds
insert, commit or POP3 deletion; no helper reaps sessions for serve.
The release build retains the same accept/reap/cap/abort-and-drain owners.
No new semantic ADR is needed; contradictory semantics or a new file/seam
outside the allowlist require an explicit scope report first. The design
gate is structural, not a proof of these guarantees.

## Project structure and files touched

| File | Change and proof ownership |
|---|---|
| tests/api.rs | 9.1 wire/refusal metrics and supplemental router polls; 9.5 fixture-derived boundaries and independent limit test. |
| tests/mcp.rs | All three 9.1 elapsed sites, cumulative refusal observations, public dispatch polls and exact wire results. |
| tests/stdio.rs | 9.2 owned silent peer with request/release rendezvous and cleanup. |
| tests/smtp_edges.rs | 9.3 real dialogue to DATA, held peer, exact helper-timeout outcome and owned cleanup. |
| tests/common/mod.rs | Only if required: narrow named diagnostic/event helpers for these tests. Prefer existing metric parsing and timeout unchanged. No general cleanup. |
| src/smtp.rs | Owner-local cfg(test) reap observations, backoff readiness seam and inline proofs. Production mechanics unchanged in final code. |
| src/pop3.rs | Corresponding reaping/backoff proof, including the measured pending/ready replacement for the actual missed paths. |
| src/store/persist_tests.rs | Existing 9.6 proof: remove elapsed duplication, explicitly check held state and single fanout. No G import. |
| specs/epic192-deterministic-timing-proofs/{spec,plan,tasks}.md | Design, subsequent dated clarification and finding-by-finding evidence index. |

Everything else is read-only context. Temporary recorded mechanism mutants
in allowed files are restored before any final check or commit.

## Research and evidence baseline

The chief inspected both positions, intake, charter, accepted concurrency,
storage/lifecycle rulings, source owners and retained receipt. Source
inspection establishes these facts, without claiming repaired-test execution:

- Both admission loops increment global REAPED. Both across-batches helpers
  start a new server per call. Harvest is immediately before cap evaluation;
  existing SPAWNED AbortHandles and parked-accept choreography supply the
  needed actual-task completion observation.
- Backoff selects shutdown against sleep. The degraded flag is written
  before this select, not a hold on its timer.
- Store::wait_for returns zero before its receive/timer await. REST awaits
  it directly; MCP dispatch reaches it through the real argument parser.
  Requests and empty stores can be fully constructed before polling.
- Locked Axum ServiceExt<R> extends Service<R>. A local generic function
  bounded by axum::ServiceExt<Request<Body>> can invoke poll_ready/call
  without importing Tower. Use the actual router, middleware, GET query
  and empty Body; no private handler seam.
- smtp_send awaits the named DATA read before constructing/sending a body.
  read_line_named distinguishes read-error and timeout panic strings.
  That precise diagnostic exposes the owner without a new callback.
- Persistence already waits for admitted gate entry, stops under
  STOP_BUDGET, checks no pre-release effect and releases once. The duplicate
  elapsed assertion adds no proof.

Baseline SHA-256, recomputed by the chief:

| File | SHA-256 |
|---|---|
| tests/api.rs | c2c6cb2eca73ac64a263cddc944c96d623fcd3d77ac52c8b13a15ecc1f27df92 |
| tests/mcp.rs | 193ceff63c527365929c1b4cb3e210259a6117b86cc0d172a9d02183dfd7a36f |
| tests/stdio.rs | 6a8463de3af19f27c482532620842b17aacfa1b1d41bb67ff6365d02778e7942 |
| tests/smtp_edges.rs | da884fd6996ef877bcd83a21262ac1c30d396ba319db345da81d3bf45aa21a4f |
| tests/common/mod.rs | 4f24e2dfd54d8a56d816c473d43eb353169fbfeb9e27b6f5b0f1012f88aeb5b2 |
| src/smtp.rs | ec17519585b515d3ce45beffb2a1722421ecd6520b29136bb2799129a7f11e0b |
| src/pop3.rs | a7a1cf1aff4ba8efb9d531e6bb0b452b5126d56ecaa9464b2de46cb37ed6a898 |
| src/store/persist_tests.rs | aadce209520d98421c2eacc541322184cba3d83caaa283e60c09571175af22a3 |

Retained LCOV SHA-256:
fe57f46356bf0a4f4350b640df033015d3459d57cfa04c6db6c8d03869bcd43a.
Record/compare a whole-directory hash manifest of the retained evidence
before/after implementation; keep the new manifest outside that directory.
Never apply UNVERIFIED-G2-context.patch. Source matches are provenance,
not proof that global credit caused run.i4vhyb.

## Data model and observation ownership

No persisted schema or public type changes.

Each protocol has a small local cfg(test) probe, carried in a Tokio
task-local scope around the particular serve future and cloned once at
serve entry. Later callbacks use that captured handle, not ambient
thread-local lookup. Finite per-test state holds admission/park observations,
actual spawned AbortHandles, local cumulative harvest count and optional
backoff latch state. A watch channel or correctly registered notification
wakes observers; subscribe/register before checking. The observer never
joins, aborts, completes or removes a session for the production owner and
never contributes to the cap calculation.

Remove REAPED as evidence. Existing unrelated thread-local injection helpers
may remain for their current tests; the new multi-owner proofs cannot use
them for attribution. Route accept-error injection through the captured
installing owner's probe for these proofs. Ordinary servers install no probe.
No global reset, port-to-owner map or suite-wide lock.

Each peer fixture owns a socket task, request-observed event and release
sender. Dropping the sender releases the receiver on failure. On normal
paths release/join; on diagnostic failure abort/join remaining async work
before reporting the named unmet event. Use existing guards where appropriate,
not a generic cleanup subsystem. HeldCommit keeps its Drop release.

## Contracts and implementation sequence

### 1. Public wait progress and refusal attribution

Build fresh AppState/McpContext from public fields/defaults with an empty
Store, distinct inbox and live false shutdown watch sender. For REST create
a fully available GET Request<Body> through api::router; use the ServiceExt
generic-bound bridge to call the real service. For MCP use mcp::dispatch
with complete JSON bytes. Pin the actual request future and call
Future::poll once using existing futures-util waker utilities (or an
enclosing poll_fn returning the Poll value). No socket, body stream, spawn
or yield intervenes between request construction and inspection.

Explicit zero must return Ready with the correct response; then decode its
ready body. Fresh omission and positive-timeout futures must demonstrate
Pending before stop or a controlled matching insert resolves them. Retain
the shutdown sender so dropped-sender shutdown cannot masquerade as zero.
These supplemental production-entry calls do not replace real HTTP tests.

Check access/behavior first. If middleware introduces unavoidable suspension
unrelated to waiting, report the precise missing observation (resolved
timeout/first park in src/api.rs or src/mcp.rs) before expansion. Never
poll-until-Ready, check only Store::wait_for, add a dependency or revert to
elapsed timing to conceal that problem.

For each real refusal test use a dedicated inbox, first issue a legal zero
wait and require its cumulative metric row to exist and advance once.
Snapshot the same row around every malformed/over-cap call; require zero
delta. Keep exact refusal and seeded non-consumption checks. Missing
attribution at the 4096-name cap fails the precondition; global/overflow
totals cannot substitute. Current affected integration suites have no
deliberate 4096-name saturation fixture; unit registry tests run separately.

### 2. Peer-held protocol deadlines

Stdio: replace only the sleeping fixture. Read through HTTP headers and
the declared request body; validate request/id and signal observed. Drive
bridge and peer concurrently, collect request observation before validating
the bridge result, keep the peer silent until validation, then release/join.
Preserve injected exchange deadline and exact -32000/read timeout contract.
Distinct generous watchdogs name request-not-observed/result-not-produced.

SMTP: perform real command/reply dialogue, recognizing EHLO and envelope
commands before DATA. Signal DATA observed; hold without 354 until
smtp_send reports its own panic. Compare the complete expected
smtp_send: the DATA reply: no reply within 30s diagnostic (derive duration
formatting from HELPER_DEADLINE). Keep the single timeout owner/defaults;
a generous outer fence allows the real 30-second contract to run.
Outer expiry is named mechanism failure, never success. Retain the client
handle so timeout cannot detach it; abort/join it and release/join the peer
on failure. Correct the old body-arrival comment.

### 3. Reaping on one owner

For each protocol start a cap-two listener with fresh probe and stop channel.
Greet A/B, hold both sessions live, and prove a third peer receives the
existing full-cap refusal. Observe this owner's accept Pending. Pin its
wait for local harvest count 1 and poll it to Pending while neither A nor
B can complete.

Complete A by real protocol, observe its actual AbortHandle finished,
then connect C exactly once. Its first reply must be a greeting; harvest
count must be exactly 1, recorded before the cap decision. The same pinned
observation resolves Ready.

With B/C live, pin the count-3 wait and establish Pending. Complete B/C
while accept is parked, observe both handles finished, then connect D once.
Require first-reply admission and count 3 (delta 2) on the SAME serve owner.
This covers empty/nonempty harvest and pending/ready observations without
retry or timing luck.

Run a second instrumented listener while the first owner's sessions are
held. Complete/harvest work there; require the first count unchanged and
its wait still Pending. Distinct probes supply actual owner attribution.
Reuse established protocol completion choreography, replacing ambient owner
assumptions only where needed. Stop/join both listeners and release clients.
EOF, end-task callbacks and session counters are not Tokio task completion.

### 4. Backoff at the actual select operand

Keep the existing select and shutdown arm. Under cfg(test), the optional
captured probe supplies a readiness latch to that backoff operand. The
installing test holds it Pending until explicit release; its first pending
poll records entry and wakes the test. The latch delays only test-operand
readiness: it does not call shutdown or execute a copied select/abort/drain.

The uninstalled test path awaits ACCEPT_BACKOFF sleep; non-test code retains
that sleep with no probe/latch. Prefer a small local async wrapper whose
installed branch awaits the release receiver then the same timer, with
pending observation on the release wait. A latch around serve itself is
wrong: a sleep-before-shutdown mutant could bypass it.

Greet a live session, install one owner-specific accept error, wake accept,
await actual latch Pending, then send stop while the latch is closed.
Require serve return, finished session handle and closed socket before
release. A separate installed-path case releases the latch and exercises
retry/timer completion; existing recovery tests cover no-observer timer/
health paths. Keep cap, health and drain assertions; no replacement 500ms
window or other elapsed ceiling.

### 5. Timestamp and persistence fixtures

Use the seed's returned/listed received_ms and id. Query all three REST
surfaces at equality and checked_add(1), asserting exact inclusion/exclusion.
Keep malformed refusals. Omit since_ms from the separate later limit case;
it tests another contract and cannot assume increasing parse-start stamps.

In existing 9.6 remove Instant/elapsed only where unused, inspect HeldCommit's
flag after stop and before release, keep memory/disk/counter/listener/no-250
checks, release once, then require one fanout and matching committed row.
Keep admission gate and STOP_BUDGET fence. No new Store seam or G helpers.

## Falsification and evidence

Run each temporary mutant separately, compile successfully, observe the
named behavioral failure, restore saved exact bytes and verify hashes
before targeted restored green. No mutant is committed or used by final
gates. Minimum source-mutant matrix:

| Mechanism | Mutant and required red |
|---|---|
| Both listener harvests | Remove admission harvest or leave it only in shutdown: first post-completion admission/local count fails before stop. |
| Both repeated harvests | Harvest only the first completion batch: second exact count/admission fails. |
| Observation isolation | Restore shared cross-server credit: unrelated-owner completion violates unchanged count/Pending. |
| Both backoff owners | Await held backoff before checking shutdown: named stop-before-release diagnostic fails; cleanup releases the hold. |
| SMTP helper | Bypass actual read_line_named timeout around read_line: the DATA timeout is absent; watchdog reports failure and cleans up, never passing as the expected panic. |
| Persistence acceptance | In src/smtp.rs acknowledge DATA before awaiting held insertion: no-250 assertion fails while release is closed. |

For zero/default, malformed/valid and since equality/+1 record paired
public-entry/input observations as discrimination controls, separately from
source mutants. The positions' additional parser/refusal-registration/filter
source mutants require src/api.rs, src/mcp.rs or src/store.rs, outside this
lane. That concrete scope extension is not authorized; report it before
attempting those experiments. Never label input controls source mutations.

New evidence stays separate from the historical failure, with per-finding
source hashes, commands/exit codes, control diffs and red/restored logs.
A failed watchdog names the unreached event. Preserve every coverage failure;
selective reruns cannot erase one.

## Quickstart and verification

Root owns launch, CPU assignment and the existing journal. This seat starts
no run. Implementation uses the private target and root-assigned CPU group.
Do not modify other worktrees, refs, journal history, baseline or pins.

After the design commit preflight with
bash bundles/self/scripts/design-check.sh check . and its selftest.
The sequence's final check runs the pinned copy and alone owns the design
verdict; candidate-copy preflight is not dispatched proof.

Affected suites:

    cargo test --locked --test api
    cargo test --locked --test mcp
    cargo test --locked --test stdio
    cargo test --locked --test smtp_edges
    cargo test --locked --lib the_accept_loop_reaps_completed_sessions_across_batches
    cargo test --locked --lib a_stop_inside_a_backoff
    cargo test --locked --lib an_in_flight_commit_neither_holds_stop_nor_half_lands

If names change, update filters and record actual test counts; zero tests
is not a pass. Then run the full authoritative
[house inventory](../../docs/house-rules.md#gates--part-of-every-change),
including locked commands, exact coverage/selftest, publish guard, keyless
seal proof and licenses. Verify brokkr's pinned tooling identity before
compile; no unsafe sweep warning. Also execute wrapper digest validation,
design-check selftest, delivery-proof-test and native-tls absence with
fail-closed Cargo command status.

Hermetic verify uses pinned scripts and checks public shims rather than
running candidate-controlled algorithms. Require actual configured Qwen Max,
Opus, official DeepSeek Flash and GLM 5.3 reviews and actual verify/ship in
engine order. This design dispatches none. CI-only chart/history/selftest,
Docker, RustSec, port-80 and tag-rate evidence remains separately labeled.
Unsigned author commits only; never push or claim whole-epic completion.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Another server supplies credit | Capture probe once per serve; isolate both owners in one test with exact unchanged/delta assertions. |
| EOF precedes task completion | Observe actual AbortHandle.is_finished before the one offered admission. |
| Notification precedes an already-expired timer | Installed latch holds readiness at the actual operand until release; no degraded-only inference. |
| Single poll sees unrelated middleware suspension | Complete public requests, fresh stores, live shutdown sender, paired positive Pending control; report unexpected suspension before expanding scope. |
| Metric row silently overflows | Pre-establish exact row with successful registration; missing attribution fails, global totals never substitute. |
| Timeout is really peer EOF or outer expiry | Hold peer; require exact owner-specific timeout envelope/panic. |
| Held work leaks or hangs failures | Owned handles, release/drop guards, abort/join diagnostic cleanup, HeldCommit Drop release. |
| Inline helper adds cold branches/dead seams | Branch-stable proofs and installed/uninstalled coverage; report concrete placement need without denominator workarounds. |
| Tied/regressing stamps break order | Data-derived boundary; limit proof independent of since. |
| Stopped socket mistaken for rollback | No effect while held; exact late commit after release under 0024. |
| Broader controls breach scope | Allowed-owner mutants only; disclose parser/filter source-control access limits. |
| Green rerun erases red or substitutes for delivery | Immutable receipts, source/evidence map, failed-gate history and separate host/hermetic/CI labels; downstream work stays outstanding. |
