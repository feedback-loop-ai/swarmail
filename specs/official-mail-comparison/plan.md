# Plan — official-artifact mail comparison

Date: 2026-10-28. Design for [spec.md](spec.md) and proposed
[0092](../../docs/decisions/0092-official-mail-comparison.md).

## Technical context

Continue stopped candidate 4a116dbc2ab0b8851eefb4ce8c2a8804dc619f01. The root Rust
crate already has the independent mail_compare bench, seven modules, fourteen
comparison tests, one shell entry point and draft docs/evals protocol/source lock.
Use its locked Tokio/reqwest/serde/tempfile/libc dependencies. No new crate, SDK,
feature matrix, service or license allowance. The standalone evals workspace has
its own lock, with versions already in the root lock under accepted 0055; it still
contains conflict syntax. Root buildability must not depend on evals integration.

No plan template exists at `.specify/templates/plan-template.md`; this plan supplies
technical context, constitution, structure, research, data model, contracts,
quickstart and risks in the authorized artifacts. No additional design files.

## Constitution check

Preserve 0001–0005, 0020 test placement, 0050/0072 executed pinned proof, 0055/0063/
0071/0073/0075/0076 and 0056 unsigned commits. Existing store/SMTP/wait/retention/
extraction logic is outside planned changes. Any demonstrated boundary defect gets
a focused regression under the existing owner, not a comparison bypass. Production
code stays measured: zero missed lines/branches/functions, no coverage exclusions
or new root workspace member. No dependency widening or install/network repair.

0092 is a proposed experiment-contract supplement, not accepted authority.
Compatible work proceeds; a contradiction with accepted semantics waits for the
operator. The chief commits exactly spec/plan/tasks/0092. Implementation T1 adds
0092's index row; 0091's already exists. Inventory failure before that is explicit.
Historical specs and accepted decisions remain unchanged. This design changes no
route; Terra intake, Opus implementation and the five independent protected review
results remain required. No further agents are dispatched by this chief.

## Research and decision log

| Fact | Resolution before measurement |
|---|---|
| Eight supplied file hashes and executable/version checks passed | Bind exact artifact/document/metadata hashes, release/platform/license and capture basis in source-lock v2. Hashes alone do not prove capabilities. |
| Mailpit upstream archive digest; MailHog only local digest | Closed digest-basis discriminator; require metadata equality only for upstream-metadata basis, require explicit absent upstream digest otherwise. No false signature claim. |
| develop/master target_commitish | Keep full release commit unresolved until primary evidence exists. Never substitute branch, API version or invented commit. |
| Tagged Swagger attribution and setup/stream sources incomplete | Record host-provided attribution with exact captured bytes; resolve acquisition URL/revision and missing setup/wait sources. No web capability or automatic downloader. |
| Mailpit matching count/read side effect | Decode list/search separately; real tests with more nonmatches than matches and unread searches before/after read. |
| MailHog v2 schema/runtime mismatch and absent v1 detail source | Preserve both facts; test observed items/ID fields, obtain pinned primary v1 attribution before collecting dependent results. |
| Limits | Both specs default pages to 50; prove actual behavior, don't call our 250 ceiling an upstream maximum. Inspect CLI help/settings and prove retention adequate for cumulative residency. |
| Resource/storage mismatch | No memory equivalence from private SQLite. Freeze actual per-target backend, compression, caps and enforcement facts in v2; separate matched/enforced versus explicitly unmatched/unenforced profiles. |
| Eval source conflicts | Reconcile current owners and all assertions in coordination with #148/#201; no wholesale side selection. Root-local work can proceed meanwhile. |
| Genuine evidence | #151 baseline, six-client authenticated smoke, final #152/#153 contract reconciliation, actual costs/final trials remain absent. No stand-ins become measurements. |
| Delivery | Original HIGH finding references, #219, CI, published pinned-engine proof and five protected results remain outstanding. Local engine hash match proves only selected bytes. |

Archive and executable bytes remain run-local external inputs; do not commit the
large binaries. Commit safe source/license metadata and attributed primary extracts
needed for reproduction under docs/evals during implementation, including retained
MIT notices. Preserve historical unverified observations as dated history while
replacing obsolete current-status sentences deliberately.

## Project structure and owners

| Path | Change |
|---|---|
| benches/mail_compare.rs | Add explicit adapter CLI dispatch; preserve independent check/run/admit and bounded argument handling. |
| benches/mail_compare/protocol.rs | Strict source-lock/protocol v2 fields and profile admission; no automatic v1 migration. |
| benches/mail_compare/targets.rs | Concrete list/search/read decoders, owned readiness, cumulative cap validation inputs and retained async cleanup. |
| benches/mail_compare/wire.rs | Reuse no-proxy/no-redirect bounded HTTP; add only necessary reusable operation budget/encoding helpers. |
| benches/mail_compare/mailpit_mcp.rs | One new cohesive adapter: closed MCP schemas, translation, queue/cancel/deadline ownership; no server supervisor. |
| benches/mail_compare/{runner,records,report,stats}.rs | Propagate complete budgets, attribution/configuration and cleanup/loss facts; keep sole arithmetic owners. |
| tests/mail_compare.rs | Extend real-binary translation/MCP/fairness/lifecycle/admission proofs; independent expected bodies and arithmetic. |
| scripts/compare-mail.sh | Thin dispatch/preflight, preserve explicit local/controls/evidence/measure modes and paid-launch refusals. |
| docs/evals/{README.md,sources.lock.json,protocol.json} | Source attribution, method freeze, prerequisite ledger, reproduction and honest current status. |
| docs/evals/runs/<run-id>/ | Only genuine admitted safe data/index/report/costs; no invented baseline/final placeholders. |
| evals/src/{artifacts,provenance,results}.rs; evals/tests/{identity,provenance}.rs | Coordinate conflict repair without discarding either accepted behavior or tests. |
| evals/src/{runner,identity,mail,fixture,mcp,oracle,comparison,main,lib,report,baseline}.rs | After repair, concrete comparison target/envelope through existing execution, sole scorer, controls/admission/report owners; comparison.rs only if one cohesive module is needed. |
| evals/tests/comparison.rs and existing proof/runner/boundary/scoring/report/baseline/wrappers suites | Free external-path controls, strict native/comparison separation, replay and retained privacy/coordination assertions. |
| docs/decisions/README.md; evals/README.md; README.md | Proposed 0092 index and reproduction; numeric agent claims only after genuine dossier admission. |

No Cargo dependency or root src change is planned. Bench placement is for experiment
infrastructure, never a way to hide production behavior from exact coverage.

## Data model and compatibility

Keep concrete target enums; no backend registry or parallel schema family per server.
Source-lock v2 records required artifact kind, asset/archive digest, executable digest,
digest basis (upstream_release_metadata or locally_computed), release/platform,
metadata hash, full source revision, source/license/API/setup/wait document identities
(URL, revision, captured digest, acquisition basis/date), launch settings and observed
capabilities. Missing facts remain typed unresolved reasons, never arbitrary dummy
strings or verified flags. A verified collection entry requires all mandatory facts.
The existing v1 bytes remain historical and need their matching historical reader;
new collection rejects them rather than implicitly upgrading their meaning.

Protocol v2 binds lock/adapter/schema/corpus identities, complete requested catalogue
and applicability, target-specific storage/cap/compression, resource enforcement,
operation budgets, counts/order/warmups, wait mechanism/interval and statistics.
Each frozen run binds exact protocol/lock bytes and commit. Add required identity
fields to run records explicitly with a record version change if their shape changes;
retain old runs/readers, never reinterpret old native eval v2 or pool incompatible runs.

Wire rows keep one disposition per planned attempt and independent observation
facts. Index finalization requires full coordinate reconciliation and observed cleanup.
Use signed monotonic offsets, null unavailable events/empty quantiles with finite
reasons, timeout counts, all failed/unstarted rows and configuration strata. Native
eval v2 fields/scorer remain unchanged; comparison envelope uses the 0091 explicit
discriminator and target union, reusing strict common validation and replay. No ninth
business failure, fabricated Swarmail metadata, scorer fork or historical migration.

## Adapter contract

Local mode: `mail_compare mailpit-mcp --endpoint http://127.0.0.1:PORT` with a
validated source/method configuration argument added by the existing CLI owner.
Endpoint is a literal assigned loopback socket; reject credentials, arbitrary paths,
non-loopback/DNS authority, proxies and redirects. Eval owner starts/owns server and
adapter. Adapter owns only its transport, stdin/stdout and active requests.

Negotiate MCP `2025-03-26`, initialized notification, tools/list and tools/call.
Use standard JSON-RPC id correlation and JSON-serialized results in text content
blocks. Unsupported revision/method and malformed closed arguments refuse safely.
Notifications do not fabricate replies. Allow tools/call cancellation by request id;
EOF closes admission, cancels active work and drains within the cleanup budget.
Results/errors must not contain raw diagnostic bytes, rejected input or private paths.

| Tool | Closed arguments and meaning |
|---|---|
| mailpit_list | Optional start (default 0), limit (default 50, 1..250); returns one ordinary upstream page with start/count, mailbox total and message identities/summaries. |
| mailpit_search | Required query (1..4096 UTF-8 bytes), optional start/limit; returns one page with matching count distinct from mailbox total. No fixture-aware query rewriting. |
| mailpit_read | Required id (1..512 UTF-8 bytes, bounded encoded path segment); ordinary subject/headers/Text/HTML as source-supported, with documented marks-read effect. No synthesized extraction or attachments. |
| mailpit_wait | Required query and timeout_ms (1..30000); immediate search then justified bounded rescans for matching ordinary messages, returning found or timeout only after complete scans. Missing wait/source justification refuses admission; incomplete scan/transport failure is an error. |

Default start is bounded to 0..100000; larger input refuses locally, not a claim
about upstream limits. Initial client ceilings: 64 KiB per newline-delimited JSON-RPC
input, eight pending tool calls, one active tool call, 1 MiB serialized reply, 10 s
per HTTP request, 16 MiB per HTTP body and aggregate operation bytes, 100 pages per scan and
1000 HTTP requests across the operation, 30 s operation deadline, 10 s cleanup/output bound.
These are engineering ceilings to commit in protocol v2, not upstream performance
or capability promises. All public limits appear in tools schemas/method docs.
Never silently truncate bodies into successful fidelity results; output overflow
returns a finite bounded error. Flood/queue overflow refuses without unbounded tasks.

Keep the input/control reader runnable while one tool call waits so cancellation
and EOF can be observed. Share the minimum budget/decoder helpers with targets.rs;
do not add a general event bus. Use existing MIME parsing for faithful MailHog
ordinary body access only where needed, with authored expected bytes as the oracle.

A common-client polling proposal is 100 ms after each complete scan, including an
immediate first scan and using one absolute deadline. Freeze the same interval on
all common-client targets and record its overhead. This is provisional until primary
wait/stream review and settings are recorded; do not mark the best/adapter profile
ready from Swagger omission. If a verified stream is needed, revise and commit the
method before collection, prove subscription readiness before scan and rescan on gaps.
Final client/model revision/output compatibility must still be independently observed.

## Implementation sequence and proofs

Follow T1–T14 in tasks.md. First close source attribution and perform untimed owned
probes; then implement concrete decoders/budgets and lifecycle repairs. Build the
adapter with independently expected real SMTP→Mailpit HTTP→MCP tests. A real
server, not Swarmail compatibility routes or a mock, proves the acceptance path.
Bounded malformed local responders may prove refusal paths only.

Exercise search with nonmatching messages, more matches than a page, literal
encoded inputs, read side effects, plain/HTML/multipart bodies and competing AUTH/
address identities. Prove bounds, cancellation, EOF, unknown fields, malformed data,
nonprogress/duplicates, truncated pages and cleanup. Cross identity acceptance
cannot establish isolation: report unsupported catalogue cells without emulation.

Prove successful bind ownership on the selected platform and refuse a deliberately
preoccupied test listener using owned benign processes; never interact with arbitrary
host listeners. Preserve asynchronous owner through startup cancellation and cleanup.
Tests observe direct-child reap, group/listener cessation, retained incomplete prefix,
no final index before cleanup and surviving unrelated owned test fixtures. Process
groups provide cooperative ownership; do not implement hostile-host attestation.

Freeze the complete method before measured runs. Retain existing six permutations,
100 warmup messages, 3000 measured messages per throughput repetition, fresh/reused
1/10/100 sessions, separate bounded 1000-session stress, and 100 present/100 future/
20 empty waits per declared repetition. Existing send/verify bounds (300 s/60 s)
and wait bounds (30 s, empty 1 s) remain method limits. Warmup can use separate
state only as declared; cumulative residency includes every message sharing state.
No resource/storage tuning after observing scores. Keep the full requested profile
ledger even when only a separately declared machine-local unmatched profile runs.

Retain 2 CPU/2 GiB as the requested enforced service profile. Actual sandbox has no
proved enforcement; do not mark it enforced from an open-file limit. If unavailable,
leave that profile open and freeze a distinct unenforced local profile with actual
host limits, bounded controller work and no comparative resource-equivalence claim.
Memory, durable and capped profiles remain separate. No need to build an orchestration
platform to conceal unavailable prerequisites.

After coordinated eval repair, integrate exact target typing through current owners,
run every applicable external oracle/null control with unchanged tasks/difficulty/
seven traps and fixture projections, and preserve sequential host admission through
finalization. Require actual #148/#201 multi-process/checkout/full-suite proof.
Implement eventual evidence/measure admission and inert witnesses; do not run paid
selections or invent final trials, baseline, cost, review or CI evidence.

## Quickstart and command behavior

Design-stage source checks, actually available:

```bash
sha256sum -c .forge/evidence/issue163-official-sources/SHA256SUMS
.forge/bin/mailpit version --no-release-check
.forge/bin/mailhog -version
cargo test --locked --test mail_compare
cargo bench --locked --no-run
```

Implementation target after complete frozen inputs (not a current success claim):

```bash
bash scripts/compare-mail.sh local --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json --mailpit .forge/bin/mailpit --mailhog .forge/bin/mailhog --output /private/fresh-local-run
bash scripts/compare-mail.sh controls --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json --output /private/fresh-controls
bash scripts/compare-mail.sh evidence --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json --baseline /private/genuine-baseline --final /private/genuine-final --receipts /private/prerequisites.json --output /private/fresh-dossier
```

Use a new private output leaf; documented paths are placeholders selected by the
operator, never literals to create as evidence. No install/download fallback.
Current local dispatch uses smoke and cannot substitute for frozen measured workflow;
implementation must expose the mode distinction and refuse incomplete pins. Controls,
evidence and operator-only measure still name blocked prerequisites and launch nothing
until implemented/admitted. Eventual measure reuses existing eval owners and requires
separate authorization; delivery tests it only with free inert commands. Dry-run opens
no credential config and launches no trial. Preserve exit precedence 2 (infrastructure/
cleanup/admission) over 1 (valid measured failure) over 0 (declared checks satisfied).
Expected empty timeouts/null control failures are successful control expectations.

## Verification

Run the complete single [house inventory](../../docs/house-rules.md#gates--part-of-every-change),
locked where required. Validate pinned engine/shim/patch bytes, use the pinned bundle
copies of proof scripts, exact coverage and selftest, publish/seal proofs, license
check and fail-closed native-tls absence. A cargo-tree error proves absence only when
it specifically says native-tls matches no package; all other errors fail the proof.
Also run locked benchmark build/full benches, real comparison tests, and standalone
eval fmt, locked all-target clippy and full default-threaded tests. Retain actual
failures rather than skipping expected conflicts. Preserve real burst/p2/rate/MCP/
compat/store/persistence/webhook/chaos/TLS regressions. No sleeps as synchronization
or tight latency pass thresholds; generate calendar-sensitive test fixtures per run.
CI owns the charter's network/platform/chart/Docker/port-80 proofs; #201 owns its
full coordination/macOS receipts. Design structural validation is a separate check.

## Risks

| Risk | Mitigation and remaining cost |
|---|---|
| Document/runtime mismatch or missing full source revision | Bind both evidence kinds, refuse dependent collection and retain the gap. Untimed work can continue without manufactured attribution. |
| Search pagination silently loses matches | Distinct matching totals, progress/duplicate checks, stable post-send scans and aggregate budgets; incomplete is an error. |
| Adapter favors tasks | General tools only, unchanged fixture/scorer, ordinary bodies, independent real-wire controls and all unsupported cells retained. |
| Wait selection hides upstream facilities | Primary stream review and separate native/common/adapter labels; provisional profiles stay blocked. |
| Retention/resource/storage bias | Cumulative residency budget, explicit launch verification and separate actual profiles; no unenforced or SQLite-to-memory equivalence claim. |
| Port races or cancellation orphan work | Corroborated owned bind, retained async owner, observed reap/drain/cessation and finalization fence; cooperative-host limits stay explicit. |
| HTTP/MCP/output growth | Bounded bytes, pages, requests, pending/active work and absolute deadlines; preserve reader responsiveness during waits. |
| Privacy/partial evidence | Closed environments, safe finite diagnostics and exclusive private artifacts; final index only after validation/cleanup, forced death stays incomplete. |
| Eval conflict/coordination loss | Compose all accepted behaviors/tests in existing owners; require full actual #148/#201 receipts, no filtered/private-lock substitute. |
| Baseline/cost/contracts absent | Preserve external prerequisites and prohibit substitutes/claims; no paid work in delivery. |
| Proposed index/numbering and review integrity | Implementation adds exact index row; operator reconciles/accepts; carry HIGH/#219, CI and five protected reviews without invented closure. |
| Scope/coverage growth | One adapter module and existing owners/locks; no production relocation, generic platform or dependency expansion. |

## Design-stage verification observations — 2026-10-28

These are draft-tree observations on the stopped candidate plus these design files,
not implementation acceptance or a claim that all candidate gates are clean.
Run-local command/status logs are retained under `.brokkr/state/design/chief-verification/`.

| Executed check | Observed result |
|---|---|
| Root fmt; workspace/all-target locked clippy | Passed. |
| Root workspace locked tests | Failed in two store_contract_docs decision-inventory tests: proposed 0092 has no index row. |
| Digest-verified pinned brokkr compile | Passed; binary and all three tooling patch hashes matched tooling.json. |
| Pinned shim digest checks | Passed. |
| Pinned exact coverage gate | Failed at the same two inventory tests during instrumented execution; no passing exact coverage verdict. |
| Pinned coverage selftest | Passed. |
| Pinned publish-guard proof | Passed. |
| Pinned seal-branch keyless proof | Passed. |
| cargo deny check licenses | Passed. |
| Locked/offline native-tls absence | Passed: inverse tree refused specifically because native-tls matched no package, not an unrelated Cargo error. |
| Locked bench build | Passed for all bench executables. |
| Locked full cargo bench | Incomplete: combined workspace call timed out; a separate bounded rerun exited 124 at 240 seconds, reaching read_stall. Partial timings are not accepted comparative evidence. |
| cargo test --locked --test mail_compare | Passed all 14 existing real-Swarmail tests, including the bounded 1,000-session test. This is not a new three-target/adapter proof. |
| Standalone eval fmt, locked clippy and full tests | All failed on existing conflict/delimiter syntax in eval sources; no standalone/full-coordination pass. |

The exact four-file chief restriction leaves the 0092 index addition to T1; the
failed inventory/coverage gates are outstanding, not exempted. T13 must rerun the
whole inventory on implementation and finish full benchmarks. CI-owned proofs,
#148/#201 coordination, actual adapter/competitor measurements, paid model trials,
genuine dossier/costs and protected review were not performed by this chief.

One plan write was refused because the workspace command exceeded 16,384 characters;
split bounded writes succeeded. The combined proof call returned a 300-second
response timeout while work continued until its call bound; retained per-command
status files distinguish completed proofs from interrupted full benchmarks. The
separate full-benchmark rerun used an explicit TERM/KILL timeout and retained exit
124. No tool refusal or timeout is reported as a pass. No native-shell writes.
