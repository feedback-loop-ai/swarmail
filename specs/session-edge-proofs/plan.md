# Implementation plan: integrated session edge proofs

Implements [spec.md](spec.md); ordered paired proofs are in [tasks.md](tasks.md).

## Execution posture and technical context

Retain the six proofs present at starting HEAD
`930bf534cb18e05f4c85e28eed3473297d159743`. This chief commit revises only
the existing spec.md, plan.md and tasks.md. Subsequent implementation means
auditing and executing the retained tests, with zero source edits unless a
demonstrated fixture defect requires a narrow repair. A production regression
returns to triage with its transcript and the full review panel.

One Rust crate uses the existing locked Tokio/Axum stack, serde_json,
rusqlite and tempfile fixtures. Raw TCP clients and HTTP/SQLite helpers live
in `tests/common/mod.rs`. The integrated base supplies the owner-local POP3
`ListenerProbe`, one-shot accept fault, backoff watch and held retry readiness.
`AppState`/`api::serve` already share the listener's health flag.

There is no `.specify/templates/plan-template.md` in this checkout. This plan
retains the repository's required template sections below; research, data
model, contracts and quickstart stay inline to respect the three-file design.

## Constitution check

Prove accepted contracts without changing them: synchronous Store::insert,
awaited storage before successful DATA 250, subscribe-before-check,
oldest-only pruning, inbox isolation and extraction at ingest. No new ADR
is warranted. Preserve the existing runtime admission/reap/backoff/task
ownership and all accepted SMTP/POP3 amendments cited by the spec.

Public wire proofs remain in `tests/`; the branch-stable owner-local health
proof stays inside the existing measured POP3 test module. No sibling
harness, module declaration outside it, extra support path, production
semantic change, new dependency, feature flag or coverage exclusion is
permitted. The commission is narrower than 0020's general cold-path option.
If a necessary repair cannot satisfy both scope and exact coverage, retain
the failure and return to triage; do not weaken cleanup or assertions.

Full route, protected review, hard security hold, unsigned author commits
and no push/merge remain binding. Any contradiction of an accepted contract
would first require a proposed amendment and operator ruling; this proof-only
plan grants no such implementation authority.

## Project structure and files touched

| File | Allowed work |
|---|---|
| `specs/session-edge-proofs/{spec,plan,tasks}.md` | This design's three committed artifacts; reconcile current scope, ancestry, proof and handoff requirements. |
| `tests/smtp_edges.rs` | Retain AC1/AC3/AC4/AC5 and exact_data, accepted_witness, consumed_after_451 helpers; repair only demonstrated fixture defects. |
| `tests/common/mod.rs` | Retain bounded untrimmed reply_raw and existing real server/HTTP/SQLite fixtures; preserve timeout ownership. |
| `tests/pop3.rs` | Retain selective partial-QUIT/RSET TOP/NOOP assertions and exact surviving content for AC6. |
| `src/pop3.rs` | AC2 only, strictly inside existing cfg(test) mod tests; reuse the current probe and unconditional cleanup. |

No extra tracked paths. `tests/persistence.rs`, SMTP/store/persist/API/webhook
runtime, existing sibling harnesses, bundles, dependency locks and product
docs are preservation references, not edit targets. Run-local transcripts
and receipts stay in ignored run state and never become surrogate reviews.

## Research and evidence baseline

Chief inspection confirmed the clean starting HEAD/tree/local base in the
spec, the seven-file branch diff and 129 POP3 additions inside the test
module. Local remote-tracking identity does not prove network freshness.
The six tests already encode the required traces: recipient-free DATA
continues the original envelope; independent chaos and SQLite DATA refusals
consume it; cancellation allows default delivery then named retry; valid
transaction AUTH preserves identity; partial QUIT supplies a genuinely gone
TOP slot; owner-observed backoff reaches the real HTTP router. This is source
inspection, not a green test or fabricated baseline-red claim.

The previous run's ancestry/receipts are historical only. Both positions
support retaining the concrete fixtures. Robustness adds a necessary
structural review: inspect that SMTP awaits insert_async's whole synchronous
mutation before 250. Immediate HTTP queries alone cannot prove that ordering
under every schedule. Retain fresh persistence-suite and full-suite evidence;
no new crash harness or webhook requirement follows.

## Data model and ownership

No schema or persistent model changes. SMTP witnesses carry socket/session,
authenticated inbox, envelope sender/recipients, raw bytes, returned ID and
exact inbox ID set. Keep header From distinct from envelope sender, and
refused content/recipients distinct from recovery. Default-routing tests own
an isolated server/store; use sufficient or unlimited capacity.

POP3 keeps existing snapshot numbers, pending DELE marks and committed gone
slots. Selective SQLite refusal targets a later ID so a prior deletion truly
commits; RSET clears only remaining marks. Do not add a lock or state enum.

Fixtures own ephemeral ports, temporary databases, triggers, tasks, shutdown
senders and owner-local probe. Subscribe before fault arming; hold retry
readiness until HTTP observation. Release/abort Drop cleanup covers unwind
and the scenario deadline; success explicitly stops and joins. Abort on
failure is not a completed join receipt, and EOF is not task completion.

## Contracts and implementation approach

1. Audit `reply_raw` without changing trimmed/multiline callers. Exact fixed
   SMTP lines include CRLF and the PLAIN prompt's trailing space. Drive the
   real 354 and terminator, parse one nonempty returned ID, immediately GET
   message and raw endpoints once, then assert metadata, exact bytes and
   singleton inventory. Account once for send's CRLF and transparency.
2. Keep AC1 in `sequencing_errors_need_mail_then_rcpt`: after need-RCPT,
   RCPT/DATA complete without new MAIL/RSET/greeting. Keep two AC3 tests:
   acknowledged DATA-only probability-100 chaos (delay zero), and a real
   tempfile SQLite insert trigger. Each exact 451 is followed on that socket
   by need-MAIL for RCPT/DATA, NOOP, empty inbox, fault removal and distinct
   accepted recovery witness. No mock Store, busy-timer surrogate or reconnect.
3. Keep AC4's literal prompt/cancel/NOOP, default delivery, later valid AUTH
   and named delivery with disjoint inventories. AC5 uses valid three-field
   PLAIN bytes, exact transaction-in-progress 503, unchanged inbox inventory
   and original-envelope/default-routing completion. Malformed base64 remains
   separately covered and cannot prove precedence.
4. Keep AC6's real selective-delete flows: marked TOP error, exact failed
   QUIT, gone TOP error before/after RSET, immediate NOOP after each TOP
   refusal, exact surviving suffix at its original number, HTTP/disk prefix
   absence and retry behavior. Retain external-clear coverage separately.
5. Keep AC2 beside its probe: real POP3 greeting/USER/PASS/NOOP and HTTP 200
   baseline; subscribe, arm one accept error, wake with a real connection,
   observe held backoff, then real GET returns exact HTTP status and parsed
   JSON equality. SMTP remains healthy and shutdown inactive. Release, close,
   stop and join. No direct flag mutation, handler call, additional observer
   or new recovery cycle. JSON member ordering and dynamic healthy uptime
   are not part of the contract.

Bound I/O and the whole scenario to detect hangs, never to assert protocol
latency. No sleep synchronization, mid-band chaos, retry-until-visible reads,
global mutable injection state, ignored tests or assertion weakening.

## Quickstart and verification

Resolve HEAD/tree/base in each execution environment, inspect scope and
cleanliness, and retain command/exit/test-count evidence against that final
candidate. Require the recorded integrated starting commit as ancestry,
not the previous run's commits. Check the tracked `recipes -> bundles`
symlink and tooling identity. The required executable is:

```bash
export PATH="/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-188-integration/.forge/bin:$PATH"
export CARGO_NET_OFFLINE=true
sha256sum /home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-188-integration/.forge/bin/brokkr
cargo test --locked --test smtp_edges
cargo test --locked --test pop3
cargo test --locked --test persistence
cargo test --locked --lib pop3_degradation_reaches_http_healthz
```

Require digest `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`,
also recorded in tooling.json; version alone is insufficient. Propagate that
PATH to every brokkr-dependent gate, or invoke the absolute executable.
Require nonzero executed counts (one for the filtered library proof).

Run every command in the sole authoritative
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
with its locked flags, zero missed lines/branches/functions, coverage
selftest, publish-guard and seal-branch proofs and license check. Record
fail-closed native-tls absence: `cargo tree --locked -i native-tls` success
means present and fails; nonzero is acceptable only for the explicit
package-not-found diagnostic. Cache/metadata/tool errors do not prove absence.

Actual hermetic offline verify installs nothing, re-establishes binary/shim
identity and runs pinned proof code, including design-check selftest and
delivery tamper-boundary proof. Missing tools/caches remain outstanding.
Local `bash bundles/self/scripts/design-check.sh check .` and its selftest
are design preflights only; the sequence's pinned final check owns its
verdict. Local gates cannot replace hermetic dispatch.

Obtain four actual independent reviews on the exact final candidate: Qwen
Max correctness, Claude Opus security, official DeepSeek Flash guarantees,
DashScope GLM-5.3 tests. Never DeepSeek Pro or fallback. Refresh affected
proofs/reviews after repairs; provider failure or security hold is not a
pass. These are downstream work, not chief-seat dispatches or receipts.

Root owns final integration and applicable CI-only house obligations: Helm
chart contract/history/selftest, real Docker smoke, network RustSec audit,
real port-80 exchange and tag-context release-rate/publisher checks. If
integration changes the candidate, rerun required evidence there. There is
no separate #177-first landing prerequisite. This lane never pushes, merges,
tags, publishes or claims epic closure.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Exact error hides lost envelope or changed identity | Same-socket continuation, exact accepted-ID/raw/envelope/inventory witnesses; retain longer explicit traces. |
| Validity or precedence confused by malformed AUTH | Encode known valid three-field PLAIN and compare inventories; keep malformed coverage separate. |
| Trimmed prompt or stale TOP output passes | Preserve raw SMTP spaces/CRLF and immediate exact POP3 NOOP after each refusal. |
| Health recovers before HTTP or wrong owner is sampled | Subscribe before arm; observe actual owner backoff while retry is held; no polling or flag assignment. |
| Failure leaks held tasks or test trigger | Existing scoped guards release/abort and remove triggers; successful path joins. No failure-join claim. |
| Necessary cold arm conflicts with measured inline scope | Return to triage with evidence; no sibling harness or lower coverage. This may stop delivery. |
| Pruning/parallelism erase a valid witness | Dedicated default store, private inbox/database/probe, adequate capacity and ephemeral ports. |
| SQLite failure proves wrong 451 or only a pending deletion | Exact cause and true DATA boundary; selective later-ID delete refusal plus HTTP/disk gone-prefix proof. |
| Stale candidate, binary or provider receipts substitute for work | Bind each execution and review to actual final identity; verify digest/shims in hermetic environment. Accept tool/provider latency and unresolved failures. |
| Production regression expands scope | Preserve failing transcript and re-triage with full panel before repair. |
| Integration changes verified content | Root refreshes required gates/reviews for the integrated candidate and records CI-only evidence separately. |
| Serial traces overclaim robustness | Retain structural ordering review, persistence/full-suite proofs and limits below. |

## Failure-model limits retained from robustness

A persistent commit may survive a lost SMTP acknowledgement; retry may
duplicate mail. Task cancellation does not imply rollback of admitted
blocking work. Immediate HTTP is necessary visibility evidence but not an
exhaustive schedule proof or crash-durability proof. Existing persistence
and CI stop/kill tests retain those responsibilities. Memory-only restart
and finite retention remain intentional limits.

Webhooks stay best-effort after storage, with no durable replay or acceptance
backpressure; per-sink limits are not a global memory ceiling. POP3 snapshot
skew and partial-delete semantics remain accepted. Subscribe-before-check,
ordered pruning and ingest extraction are preserved, not redesigned.
Existing hostile-byte, cap/burst, TLS and generated certificate proofs stay
in the full suite; six plaintext traces are not exhaustive transport,
resource, arbitrary-task-cancellation or certificate-renewal assurance.
