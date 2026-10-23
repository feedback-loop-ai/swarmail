# Deterministic protocol and concurrency proofs

Date: 2026-10-23
Status: draft for implementation
Branch: epic192/next-177
Baseline: b576466d055592d5b143848250b10f55106120a6
Issue: #177, all six findings, plus the actual POP3 reaping coverage failure.
Route: engine, full design and four independent review lenses.

## Feature and rationale

Make the affected tests distinguish correct protocol/lifecycle behavior from
broken behavior by observed events, ownership and ordering. A loaded runner
must not turn an otherwise correct result into a failure merely because it
exceeded a sub-deadline elapsed-time assertion. Conversely, deleting those
assertions must not turn a test into an eventual-response check that accepts
a defaulted zero timeout, an unbounded helper read, delayed shutdown or
shutdown-only session reaping.

The retained host run at c32ba255c2b2fd8fe6f643e94b518f0c200efa9f passed its
tests but failed exact coverage: 12,446 total / 12,444 covered lines,
724 / 723 branches, 1,379 / 1,379 functions. POP3 lines 1334–1335 and the
false outcome at 1332 were missed. The receipt, source copies and run.i4vhyb
LCOV under .forge/epic192/actual-pop3-coverage-flake remain immutable evidence.
This was a host check, not candidate hermetic verification.

Both listeners currently increment a global test REAPED counter. Each call
to the across-batches helper creates a new listener and compares the global
counter with an absolute threshold. Other listeners can supply its credit;
two calls do not establish two batches on one JoinSet. This is a
source-supported failure mode, not an experimental diagnosis of that
historical execution. The replacement must make cross-owner credit
impossible and exercise both pending and ready observations deliberately.

## Authority and semantic boundary

This is a repair of tests and exercised cfg(test)-only listener
instrumentation. It proposes no public contract or ruling change, so it
declares no new decision and allocates no ADR number. The accepted authorities
are [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0025](../../docs/decisions/0025-shared-domain-validation.md),
[0036](../../docs/decisions/0036-smtp-trickle-lifetime.md),
[0038](../../docs/decisions/0038-store-memory-and-timestamps.md),
[0039](../../docs/decisions/0039-stdio-concurrency.md),
[0046](../../docs/decisions/0046-smtp-pop3-edges.md) and
[0050](../../docs/decisions/0050-delivery-gates.md), including their
controlling amendments.

Preserve synchronous insert and commit/visibility before 250 (0001,
0014, 0023), subscribe-before-check (0002), oldest-only pruning (0003),
ingest extraction (0005), the live-view lag invariant (0016) and
[0024's admitted-commit policy](../../docs/decisions/0024-shutdown-refusal-at-the-boundary.md).
Keep one shutdown watch, abort-and-drain ownership, admission-position
harvest, health/refusal semantics, all drain/stop/busy/idle/transport
budgets, and the ordinary one-second accept backoff. Tests cannot authorize
a semantic change. If implementation needs one, report the concrete need
and coordinate a proposed decision before broadening; never self-accept.

## Council synthesis and rulings

The chief read both run-local positions and the intake framing for
epic192-next-177-deterministic-p-521a6ac3. Their recommendations are resolved
as follows; the plan is normative about the chosen mechanisms.

| Question | Ruling and cost |
|---|---|
| Small local proof versus broader infrastructure | Adopt simplicity's local changes and robustness's causal proof standard. Reject a general event bus, global registry/reset, suite serialization, production scheduler API, new helper module or dependencies. Cost: a little protocol-local duplication and explicit cleanup. |
| Reaping | Adopt both positions' per-serve owner, actual task-completion barrier and repeated batches on one JoinSet. Require robustness's second-owner isolation and exact batch deltas. Reject preserving the retry loop simply to cover its old lines. Cost: more real socket choreography; the new pending/ready proof remains inline and measured. |
| Zero timeout access | Adopt simplicity's existing public router/dispatch entry points as supplemental single-poll proofs; robustness is right that payloads, unrelated health requests and a Store-only zero wait cannot prove adapter forwarding. Existing axum ServiceExt exposes the Service supertrait through a generic bound, so REST needs no Tower dependency or private handler access. Cost: fixtures for AppState/McpContext and a public-entry proof alongside the real wire legs. First-poll feasibility must be checked before implementation proceeds to unrelated work. |
| Backoff | Rule for the explicitly installed cfg(test)-only readiness latch at the actual backoff operand, as both positions ultimately require. A passive notification or degraded flag cannot hold a real timer pending. This is within the requested test instrumentation scope: no non-test scheduler hook or release behavior changes. Cost: installed/uninstalled seam coverage and a separate release-path exercise. |
| SMTP helper timeout | Choose the existing 30-second read deadline and exact timeout diagnostic plus a DATA-observed/peer-release handshake. Reject an additional timeout-event API or configurable helper deadline unless a concrete access defect is demonstrated. Cost: this real timeout test still spends its actual deadline; runtime optimization is outside #177. |
| Negative controls | Require actual harvest, backoff, DATA-read-timeout and premature-ack mechanism mutants within the named files, with exact restoration. Adopt zero/default, refusal-registration and inclusive-boundary input controls through the actual public entries. Reject silently editing out-of-scope API/MCP/Store owners merely to add more mutation experiments. Cost: no claim of parser/filter source-mutation proof without separately coordinated scope; public-entry discrimination remains mandatory. |
| Placement and coverage | Adopt both positions' inline, branch-stable listener proofs under 0020. Reject harness relocation, ignored tests, exclusions, reduced denominator, gate normalization and retry-for-green. A concrete unavoidable cold path needing another file must be reported before scope expands. |

## Acceptance Criteria

- **AC1 — Complete scope and timing discipline.** All six findings below,
  the additional MCP over-cap elapsed check, and both sibling reaping proofs
  are addressed. No affected correctness assertion compares elapsed real
  time to a ceiling; no sleep synchronizes a proof. Real contract timeouts
  remain exercised. Generous watchdogs bound failure diagnostics and never
  count as the expected protocol result.

- **AC2 — 9.1 refusals.** Real REST/MCP malformed and over-cap requests
  retain their existing status/envelope/text and non-consumption checks.
  Each refused call has zero delta in its exclusive inbox's cumulative
  swarmail_awaits_registered_total row. A legal wait first proves that the
  same row exists and advances exactly once. Absence at registry saturation
  fails attribution; global/overflow deltas and active-gauge-only checks
  cannot substitute. Existing unrelated tests cannot supply the row's work.

- **AC3 — 9.1 zero.** Real HTTP REST returns 408 with matched=0 and MCP
  returns timed_out=true with matched=0 for explicit zero on empty inboxes.
  Supplemental calls through the production REST router and MCP dispatch
  receive fully available request data and yield the correct response on
  one synchronous poll of their actual request futures. An omitted timeout
  and a legal positive timeout on a fresh empty inbox instead demonstrate
  Pending; stop or a controlled matching insert resolves and cleans them
  up. This checks the adapters, not a copied parser or only Store::wait_for.
  Zero's wire call registers once and leaves no active wait.

- **AC4 — 9.2 stdio.** The real peer observes the complete request, signals
  that event and holds its socket until explicitly released after the
  bridge reports the same id, error -32000 and exact message read timeout.
  The five-second sleeping fixture is gone. Peer task and bridge work are
  joined; cancellation/failure releases or aborts and joins owned tasks.
  Early close, malformed response or a diagnostic watchdog is not a pass.

- **AC5 — 9.3 DATA reply.** A real scripted peer observes greeting/EHLO/
  envelope/DATA in order, withholds 354 and remains held through the helper
  outcome. The actual smtp_send/read_line_named timeout produces the exact
  named DATA-read timeout panic, not a read-error or outer-timeout result.
  No body is sent before 354. Removing the actual timeout wrapper makes
  the named mechanism assertion fail; restoration passes. Existing helper
  callers retain their defaults and diagnostics.

- **AC6 — 9.4 backoff.** In each protocol a greeted live session is owned
  by the tested serve future. One accept error reaches that owner's real
  backoff select; the installed test operand is observed Pending and remains
  held. Stop then completes through the actual abort-and-drain arm while
  retry readiness is still unreleased, with the live task finished and its
  socket closed. No 500ms or replacement elapsed ceiling remains. Moving
  shutdown behind the held backoff fails stop-before-release. Both ordinary
  timer behavior and installed latch release are exercised.

- **AC7 — 9.5 timestamps.** Derive since_ms from the first stored fixture's
  actual received_ms. Equality includes its identity; checked stamp+1
  excludes it consistently on list, await and assert, with exact counts and
  existing malformed refusals. The separate count=2/limit=1 case does not
  assume the second parse-start stamp is newer. No now-plus/minus margin,
  inter-seed sleep or monotonic timestamp promise is introduced.

- **AC8 — 9.6 admitted persistence.** In the existing
  an_in_flight_commit_neither_holds_stop_nor_half_lands test, gate entry
  precedes stop and the HeldCommit release flag remains closed after stop
  completes. While held: no DATA 250, closed listeners, no added live mail,
  counter, fanout or disk effect. Explicit release then produces exactly
  one matching committed mail in memory/disk, one counter increment and one
  notification. Remove the redundant elapsed assertion; preserve the
  existing STOP_BUDGET diagnostic fence and failure-safe release. A
  premature DATA-ack mutant fails before release; restored code passes.

- **AC9 — Reaping ownership and batches.** Each listener proof uses one
  fresh observation object captured once by one actual serve future and
  observes at least two batches on that same owner, including a batch of
  two completions. Confirm actual AbortHandle completion while accept is
  parked, then offer one next connection and require admission on its first
  reply. Require exact harvest deltas before the cap decision; retain a
  genuinely full-cap refusal. A different server's completed/harvested
  sessions leave the first owner's observation unchanged. No global
  REAPED, baseline subtraction, reset or retry-until-admitted proof remains.

- **AC10 — Non-vacuous measured outcomes.** Before releasing each held
  batch, poll the pinned owner-local reap wait and assert Pending; after
  actual completion and admission require Ready on that same wait. Actual
  empty and nonempty harvest cases run. The second batch advances beyond
  the first. Removing admission harvest, making harvest shutdown-only, or
  harvesting only the first batch fails the appropriate owner/batch
  assertion for both protocols. Replacement inline lines and branches,
  including the old false/retry obligation's replacement, execute in the
  exact coverage report.

- **AC11 — Evidence and restoration.** Preserve the historical failed
  receipt and its hashes. For each mandatory mechanism control record the
  exact diff, source hashes, command, named behavioral assertion/red log,
  restored hashes and targeted green. A compiler error, arbitrary assertion
  change, unrelated-server credit, outer-timeout-as-success or repeated
  run-until-green is not proof. Record input controls separately from
  source mutants. Final source hashes and finding-by-finding evidence map
  are required; no G patch or private helper imports.

- **AC12 — Delivery.** Every gate in the authoritative
  [house inventory](../../docs/house-rules.md#gates--part-of-every-change)
  passes with locked commands where specified, literal 100% lines/branches/
  functions, coverage selftest, publish and keyless seal proofs, pinned
  compile without unsafe sweep warnings and licenses. Also require checked
  wrapper digests, design-check selftest, delivery tamper proof and
  fail-closed native-tls absence. The chief's last commit contains exactly
  this directory's spec.md, plan.md and tasks.md; the pinned final design
  step owns its verdict. Downstream require configured DashScope Qwen Max
  correctness, Claude Opus security, official DeepSeek Flash guarantees
  (never Pro), DashScope GLM 5.3 tests, and actual hermetic verify/ship.
  No fallback, new launch, push, merge, tag or publication is authorized.

## Scope, failure limits and handoff

Implementation touches only tests/api.rs, tests/mcp.rs, tests/stdio.rs,
tests/smtp_edges.rs; narrow necessary helpers in tests/common/mod.rs;
inline proofs and cfg(test)-only accept/reap/backoff instrumentation in
src/smtp.rs and src/pop3.rs; the existing 9.6 test in
src/store/persist_tests.rs; and this spec directory. No other G source,
Store/Persist semantics, gate/script, dependency, policy, recipe, charter,
adapter, tooling or dispatch-pin edits are part of this feature.

The accepted crash window is still commit without a delivered 250, with
possible duplicate on retry, never an acknowledged absent commit. A hung
webhook cannot delay mutation/250 and gains no delivery guarantee. Caps
still prune by insertion order; hostile complete-line tricklers keep their
documented indefinite slot/buffer exposure. Chaos remains at the poles
and cannot create a half-applied refusal. TLS fixture/calendar work is
outside scope; do not weaken verification to make a test pass.

Diagnostic deadlines can still expire on a severely starved runner. Such
a result is an unfinished/failed proof, not evidence of wrong semantics
and not a waiver. Host checks, actual hermetic seat checks and CI-only
chart/history/selftest, Docker, RustSec, port-80 and tag-rate evidence must
be reported separately. This design does not implement #177, close the
epic, accept an ADR or claim any downstream gate/review/ship ran.

## Clarifications — 2026-10-23

The reserved slug is epic192-deterministic-timing-proofs. The owner of the
muted SMTP test is the helper's read of 354, not the server's final 250.
The single-poll adapter proof supplements real protocol tests. Test-only
backoff readiness control is explicitly selected here within the allowed
listener instrumentation; it must be absent from non-test builds. The
historical POP3 failure remains a failure after any future green run.
