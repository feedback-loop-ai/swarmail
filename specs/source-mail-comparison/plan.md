# Plan — source-attributed mail comparison

Date: 2026-10-29. Implements [spec.md](spec.md) and proposed
[0093](../../docs/decisions/0093-source-mail-comparison.md).

## Technical context

Continue 14217f7 through the existing root mail_compare bench/adapter and standalone
evals workspace. Reuse locked Tokio, reqwest, serde, libc and tempfile. No new crate,
dependency, allowlist widening, feature matrix or production fast path. Root bench
buildability remains independent of evals. `.specify/templates/plan-template.md` is
absent; this plan supplies the required sections within the authorized artifacts.

## Constitution check

Preserve 0001–0005: synchronous accept/store, subscribe-before-check, oldest-only
pruning, AUTH inbox identity and ingest extraction. Preserve 0014 plan→commit→apply
and its dated corrections: capped commit/apply can have different live/disk sets;
lock-free reads are not whole-store transactions; SQLite's busy handler is not an
end-to-end deadline. Keep 0020 test placement, 0050/0072 proof, 0055/0063/0071 eval
semantics, 0073 lifecycle, 0075 privacy, 0076 shared admission and 0056 signing.

0093 changes experiment contracts and stays proposed alongside 0090–0092. Compatible
work proceeds; contradictions with accepted rulings wait for operator acceptance.
No route/history/numbering change. Chief's four-file allowlist excludes the index;
T1 adds its single proposed row before full-gate success. This gap is not waived.
Engine/panel own advancement and reviews; chief dispatches no further agents.

Preserve existing crash/cap/persistence/chaos/DATA/MIME/TLS/webhook regressions.
Lost acknowledgement after commit is ambiguous; memory-only process death is not
restart durability. Hung webhook delivery cannot block stored acceptance;
notifications remain best effort, not an outbox. Keep complete DATA termination,
byte bounds, ingest extraction and probability-0/100 tests. Session stress is not
a global memory guarantee; idle bounds do not bound trickle lifetime. Generate TLS
validity fixtures per run and keep validation enabled; plaintext says TLS unmeasured.
No new cross-product fault campaign, storage rewrite or production scope is planned.

## Research and implementation decisions

| Inspected fact | Required response |
|---|---|
| Checked tag chains and source bodies | Import safe primary bytes into existing sources directories with immutable attribution, metadata hashes and notices. No build/authenticated-acquisition inference. |
| Pin.source_revision means built-from | Strict lock v3; preserve v2 bytes/interpretation and separate tagged source from unknown build association. |
| Stream subset and READMEs | Record notifications; retain missing route/producer/EventSource and MailHog v1 bodies. No invented acquisition or optimum wait. |
| protocol_version is 1 despite draft-2 name | Implement actual v2 and freeze separate honest profiles. |
| reserve_port drops reservation; ready checks child/banner/HTTP | Corroborate both sockets against launched identity or refuse. |
| shutdown takes child before awaits; timeout consumes drain handle | Keep async owner/handles until terminal observation; inspect nested join results. |
| scan uses fixed page stride | Reject short nonfinal pages; final reconciliation needs quiescence and independent IDs. |
| adapter Wait replaces deadline and stops early | Compose ceiling/cutoff/response allowance as specified below. |
| wrapper integrated modes always refuse | Implement real owner delegation and independent prerequisite checks, not stubs. |
| evals conflict syntax | Compose both accepted behaviors/assertions before integration; full suite and shared coordination remain required. |

The chief checked all supplied source/artifact/engine hashes described in spec.md
and read the entire stopped event. No network acquisition, competitor experiment,
paid trial or protected review was performed. Historical receipts remain historical.

## Project structure and owners

| Files | Intended changes |
|---|---|
| benches/mail_compare/protocol.rs | Lock v3/protocol v2, source chains, scoped admission and historical interpretation. |
| benches/mail_compare/targets.rs | Owned readiness, async lifecycle, complete bounded scans, quiescent ID reconciliation and faithful decoding. |
| benches/mail_compare/wire.rs | Minimal shared encoding/budget helpers; preserve no-proxy/no-redirect and response bounds. |
| benches/mail_compare/mailpit_mcp.rs | Existing four tools, cutoff/budgets, cancellation and only justified final-client correlation/batch work. |
| benches/mail_compare/{runner,records,report,stats}.rs; benches/mail_compare.rs | Owner orchestration, full coordinates/accounting, identity checks, mode dispatch and existing arithmetic. |
| tests/mail_compare.rs | Source/refusal, real three-target/MCP, benign lifecycle, independent bodies/IDs/statistics and mode tests. |
| docs/evals/{sources.lock.json,protocol.json,README.md}; docs/evals/sources/{mailpit,mailhog}/ | Captured safe primary bytes, contracts, freeze/reproduction and single evidence ledger. No binaries. |
| docs/evals/history/ | Byte-identical superseded v2 lock/protocol snapshots and historical reader provenance; preserve v1 history. |
| scripts/compare-mail.sh | Thin working local/controls/evidence/measure dispatch. |
| evals/src/{results,artifacts,provenance}.rs; evals/tests/{identity,provenance}.rs | Reconcile conflict syntax without discarding assertions or accepted behavior. |
| evals/src/{runner,identity,mail,fixture,mcp,oracle,main,lib,report,baseline}.rs | Concrete comparison target/envelope through existing execution, sole scorer, retention, replay and admission. |
| evals/src/comparison.rs; evals/tests/comparison.rs | Only if a cohesive typing/admission module is needed; no duplicate runner/scorer. |
| Existing evals/tests suites | Retain proof/runner/boundary/scoring/report/baseline/wrappers, privacy, cleanup, provenance and coordination tests; add comparison controls. |
| docs/evals/runs/<run-id>/; README.md; evals/README.md | Genuine safe admitted evidence, losses, costs and citations when available. |
| docs/decisions/README.md | T1 adds exactly one proposed 0093 row and checks existing statuses. |

Root src, Cargo manifests/locks, coverage, gates and routes have no planned changes.
Bench placement owns experiments; it cannot hide production behavior from coverage.

## Data model and compatibility

Keep concrete target enums and current typed facts/unavailable reasons. Lock v3
requires tagged_source and binary_build_provenance independently. Tagged source
records repository/tag/object/full commit and exact captured ref/tag/tree identities.
Documents bind immutable URL/revision, kind, path, length/digest and acquisition
basis. Extend only needed document kinds and reuse confined path validation. A tree
path is not reviewed source. The source lock stays the sole authority; preserve
capture manifests as evidence, not a competing mutable configuration.

No build association is the explicit unresolved state, never inferred from tag or
target_commitish. Admission checks the declared claim: official-artifact observations
may use attributed source context plus this limitation; built-from/equivalence claims
refuse. Missing operation semantics still block dependent collection. Archive v2
bytes with matching interpretation; collection rejects historical locks rather than
automatically upgrading. Explicitly version any changed run envelope and preserve
historical readers/bytes. Native eval v2 never changes meaning.

Protocol v2 freezes full requested product/applicability, actual per-target backend,
compression/retention, resource enforcement mechanism or unenforced facts, settings,
corpus/seed, counts/order/warmup/sessions, observation methods, all budgets and schema/
lock identities. Commit before collection; manifest binds bytes and commit. Changed
method means new identity/reason. Requested unavailable profiles remain entries, not
fabricated trial rows. Verify cumulative residency, including warmups/reused state.

Wire records preserve exclusive planned-send dispositions and independent identity-
linked visibility/duplicate/eviction/timeout facts, signed offsets, windows, scheduling
and costs. Final index follows full accounting/cleanup/identity checks; diagnostic
partial reports never become complete. Eval comparison envelopes discriminate target
settings and reuse strict common native-v2 facts/scorer/replay. Native readers reject
comparison/legacy/mixed rows. Comparison readers reject foreign/missing/duplicate/
truncated coordinates/settings/hashes. No fake Swarmail metadata or ninth class.

## Contracts and algorithms

### Owned targets and scans

One async worker owns child, process identity/group, drains and scratch from partial
startup through finalization. Parent owns stop/join and has no shorter cleanup timer.
Subscribe before spawn. Reuse eval's proven ownership shape without importing evals
or extracting a crate. For fixed competitor ports on this Linux-dependent driver,
corroborate both listening sockets against launched PID/start identity through bounded
supported process/socket observation; unknown identity/access or competing binds refuse.
Native port-zero announced addresses retain owned readiness. Never adopt an endpoint,
retry until bind success or clean arbitrary host processes.

Use one five-second TERM grace then five-second post-KILL observation policy with
concurrent drains, direct waits, group membership/identity and listener cessation.
Keep handles through awaits and inspect nested errors. Cancellation cannot discard
cleanup ownership. Unknown/expired cleanup stops admission and withholds final index;
Drop is fallback only. Forced death requires independent cleanup confirmation. Remove
only owned scratch after cessation. Bounds exclude arbitrary kernel scheduling.

One scan budget spans all requests/pages/bytes. Validate echoed offset, matching total,
full nonfinal pages, nonzero progress, unique IDs and all maxima. Reject short nonfinal
pages rather than skipping by fixed stride. Final exact-count checks drain owned
senders, exclude undeclared mutation and independently reconcile expected identities,
ambiguity and evictions. Live stable totals mean a page walk, not a snapshot under
churn. Missing cursor/generation means atomic snapshot remains unsupported.

### MCP and wait bounds

Retain local MCP 2025-03-26 and JSON text blocks; final #151/#152/#153 clients separately
prove negotiation/output/batch requirements. Preserve native batch behavior. Keep
closed list/search/read/wait, ordinary bodies, marks-read effect, IDs and notification
silence. Page default 50/range 1..250, start 0..100000, query 1..4096 UTF-8 bytes,
id 1..512 bytes, timeout 1..30000 ms are adapter bounds, not upstream maxima.

Freeze 64 KiB input lines, eight pending/one active tool call, 1 MiB replies, 10 s HTTP
request bound, 16 MiB per-response and aggregate operation bytes, 100 pages per scan,
1,000 requests per operation, 30 s operation and 10 s output bound. Overflow errors
never truncate faithful success. Validate literal owned loopback endpoints without
credentials/arbitrary paths; preserve no proxies/redirects. Keep control reader live. Expired queued work refuses without issuing HTTP.

At tool-call admission t0 (including queue time), set O=t0+operation_limit,
D=t0+requested_wait and
R=min(O,D+request_limit); never replace O. Immediate scan, then post-completion
100 ms intervals while before D. Each exchange is clamped to its per-request bound
and R; totals span every scan. Sleep only until min(next poll,D), cancellation-aware.
Found requires a valid complete scan observed at/before D. Responses after D are
late/censored, never on-time successes. Bounded adapter timeout is returned only
at/after D after valid completed scans, and means no match observed under this method.
Incomplete in-flight scan, exhausted budget, cancellation or transport failure is
error/incomplete. No scans start at/after D; allowance cannot extend observation
eligibility. For 30 s wait and 30 s ceiling, R=O, so there is no extra 10 s. Bind
actual schema/bounds to the method. Controlled clocks/local finite responders prove
arithmetic; real servers prove acceptance paths. No tight latency pass threshold.

### Integration and modes

T7 composes both sides of conflicts and all accepted assertions. Full default-threaded
suite plus real shared-host independent-process/checkout admission proof is mandatory;
private locks/filtering/compilation cannot satisfy #148/#201. Retain concurrency one
per effective UID sharing host /tmp through cleanup/finalization; no new scheduler or
stale-lock recovery. Within-trial multi-agent controls remain separate and preserved.

T8 uses identical fixture tasks, public inputs, difficulty and scorer with declared
connection/tool differences. T9 external oracle/null traverses both real targets;
retain exact idle failures and seven traps. Unsupported AUTH isolation/chaos remains
visible, never emulated. Controls do not prove real-model compatibility or baseline.

Wrapper stays preflight/dispatch. Local explicitly distinguishes smoke from frozen
wire collection; document any changed smoke default. Controls delegates integrated
free evals. Evidence validates genuine baseline/final/control/cost/coordination/
contract/receipt inputs before dependent collection and report regeneration. Measure
requires separate authorization and existing eval owners; delivery tests only inert
full-product order/refusals. Dry-run opens no credentials and launches no trial.
Missing prerequisites name the phase and cannot downgrade to local success. Keep
2 > 1 > 0 exit precedence; expected empty waits/null failures are valid controls.

## Quickstart and verification

After implementation, installed digest-checked inputs and the frozen protocol:

```bash
cargo test --locked --test mail_compare
MAIL_COMPARE_MAILPIT="$PWD/.forge/bin/mailpit" MAIL_COMPARE_MAILHOG="$PWD/.forge/bin/mailhog" cargo test --locked --test mail_compare -- --ignored
bash scripts/compare-mail.sh local --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json --mailpit .forge/bin/mailpit --mailhog .forge/bin/mailhog --output /private/new-local-run
bash scripts/compare-mail.sh controls --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json --output /private/new-controls
bash scripts/compare-mail.sh evidence --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json --baseline /private/genuine-baseline --final /private/genuine-finals --receipts /private/prerequisites.json --output /private/new-dossier
```

Paths are caller-selected fresh private leaves, not fixtures to fabricate. Current
integrated modes still refuse and local is smoke; these examples specify the target
behavior, not present success. No install/download/paid fallback. Measure is not run
here; future dry-run and free inert witnesses prove no-launch/no-credential-read.
Genuine paid selections require separate authorization.

Execute the entire single [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
with --locked as specified. Verify tooling.json brokkr/patch digests and public shim
integrity, then use pinned proof copies, exact coverage plus selftest, publish/seal,
licenses and fail-closed native-tls absence. Cargo-tree errors other than explicit
no-matching-package cannot prove absence. Intake also requires pinned design-check
selftest/delivery-proof-test, full locked bench build/run, real comparison and
burst/p2/rate. Run standalone eval fmt, all-target locked clippy and full default-
threaded locked tests, controls and coordination. No filtering to hide conflicts.
Real servers prove acceptance; finite local responders prove malformed/budget
refusals only. Preserve 0020 placement and generate validity fixtures per run.

CI owns platform, RustSec, Helm/chart-history, Docker and real port-80 proofs named
in the charter; #201 supplies coordination/macOS receipts. Verify/panel produce
candidate-bound engine and five independent review results. Source/hash checks,
structural design validation and historical passes cannot substitute for these.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Tagged source becomes false build claim | Lock v3 separates facts and rejects false basis; build association stays unknown. |
| Missing route/EventSource/v1 bodies | Retain gaps and block dependent profiles; no invented acquisition/best-wait label. |
| Equal-count churn defeats page completeness | Final quiescence plus independent IDs; live walks disclaim snapshots. |
| Deadline repair alters wait meaning | Frozen cutoff/ceiling/allowance and boundary tests; late/budget work remains visible. |
| Caps or storage/resources bias results | Cumulative residency, actual launch checks and separate strata; parity can stay open. |
| Cancellation/output pressure loses ownership | Surviving worker, drain/nested-join checks and observed cessation; uncertainty stops admission. |
| Host/process identity cannot be proved | Conservative refusal; no hostile same-uid guarantee or broader permissions. |
| Eval repair drops behavior/coordination | Compose both sides, full default-threaded suite and cross-checkout receipts; no private locks. |
| Adapter makes tasks artificially solvable | Ordinary tools, unchanged scorer/public inputs and all unsupported cells retained. |
| Missing paid costs/baseline/contracts | Build validation/reporting now, leave genuine dossier/claims open; unknown spend unavailable. |
| Evidence leaks or partial output looks final | Closed environments/private exclusive artifacts, finite fields, index after cleanup. |
| Index/number collision and HIGH residual | T1 local proposed row, operator reconciles; original HIGH/#219 reach actual protected review. |
| Scope and exact coverage growth | Existing owners/dependencies, focused tests; no exclusions or weaker gates. |

## Design-stage evidence boundary

Consistency checks passed as recorded in spec.md. Four-file restriction leaves the
0093 index row to T1; full-gate success cannot precede it. Standalone eval sources
still contain conflict syntax. Any design-stage gate output describes this draft
only, not implementation acceptance. Candidate/CI/panel receipts remain required;
the engine's subsequent pinned design-check rules this design attempt.


## Executed draft checks — 2026-10-29

Run-local logs/statuses are under `.brokkr/state/design/chief-verification/`.
These are observations of the design worktree, not implementation acceptance or
engine verification. No historical receipt is reused as a current pass.

| Executed check | Observed outcome |
|---|---|
| Root format; workspace/all-target locked Clippy | Passed. |
| Root workspace locked tests | Incomplete: explicit 120 s bound, exit 124. No full-suite pass. |
| Focused store_contract_docs suite | 26 passed, 2 failed: 0093 has no index row. T1 owns the authorized later index addition. |
| Pinned exact coverage | Failed during instrumented tests on those same two inventory assertions; no passing coverage verdict. |
| Pinned coverage selftest | Passed. |
| Digest-checked brokkr compile; shim checks | Passed; policy sweep reports 76 valuations over 14 groups, zero unruled. No engine repair or review verdict inferred. |
| Pinned publish-guard/seal-branch proofs | Passed. |
| Pinned design-check selftest/delivery-proof-test | Passed. |
| cargo deny licenses; fail-closed native-tls absence | Passed. |
| Full cargo bench --locked attempt | Incomplete during compilation: explicit 120 s bound, exit 124. No benchmark completion or comparative score claim. |
| Standalone eval fmt, locked Clippy and full locked tests | Failed on inherited conflict/delimiter syntax (exits 1/101/101). No coordination/readiness pass. |
| Artifact review | Relative links resolve; inherited AC1–AC16 preserved byte-for-byte; fourteen ordered unchecked tasks each have a test; historical tracked files unchanged. |

All house gates were attempted; failures/timeouts remain outstanding. T13 must run
the complete inventory on implementation after T1 repairs the index and T7 repairs
evals. Actual competitor/full profiles, controls, final client/paid/cost evidence,
CI and protected review remain unperformed by this chief. The root timeout and
benchmark timeout do not lower required test/workload bounds. Several over-size
workspace write requests were refused before execution; split bounded writes
succeeded. All writes, commits and the engine result use the workspace tool.
