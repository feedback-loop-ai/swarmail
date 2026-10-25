# Implementation plan — bounded CA executor

Implements [spec.md](spec.md) under accepted 0069 and proposed
[0078](../../docs/decisions/0078-ca-executor-ownership.md). Design only.

## Technical context

Single Rust 2024 crate. Use std threads/synchronization, existing Tokio oneshots
and current reqwest/rustls/rustls-pemfile/base64 builder. No new dependency,
feature, lockfile/allowlist change, private runtime or public tuning API.
`.specify/templates/plan-template.md` is absent; the charter's required technical
context, constitution, structure, research, model, contracts and quickstart are
inline. The generic spec template's unrelated multi-repository Alkemio instructions
do not apply to this single-crate realm.

## Constitution check

Preserve frozen accept==stored, subscribe-before-check and oldest-only pruning;
keep ingest extraction and 0023/0030 awaited parse/whole mutation/panic symmetry.
No production Store/SMTP change. Accepted 0069 authorizes isolated CA admission;
0078 resolves compatible choices without acceptance or changing 0041 §5 stop order
or 0067 no-builder-drain. Index the proposal during implementation, not this
four-file design commit. Never rewrite accepted bodies.

Real protocols, generated TLS fixtures and fresh inboxes remain required. Use
0020: measured inline branch-stable tests; declared sibling harness for concrete
cold watchdog/failure paths; tests/ for public-only wire cases. Every seam is
cfg(test)-only and exercised; no feature gates/denominator exclusions. No sleeps
as synchronization, mid-band chaos or whole-suite lock. Keep bundle routing and
protected five-office review/security hold. Commit unsigned, never push.

## Research and resolved approach

webhook.rs offloads redacted_targets, prepare_targets and build_delivery_client
to shared spawn_blocking. GET clones via current() too early; delivery makes two
PEM key copies around await. The single offline builder and waiter-owned cache
publication already exist. public_ca_pem decodes every certificate, canonicalizes
and builds the reconstructed whole bundle. Retain that implementation.

RunningServer owns shutdown/drained watches and separate serve/dispatcher sets.
A private RAII owner token closes CA authority on drop without adding Drop to
RunningServer (whose stop moves fields). Close it explicitly before shutdown
send in stop. Keep it in the startup frame until return, so error legs close
it automatically. Workers/callers hold scope references, never owner tokens.

Process ownership matches the global cache and bounds restart churn. Native
threads must build clients without capturing Tokio Handle. Prove standalone
construction first; a runtime dependency is a design defect to resolve, never
permission for shared-pool fallback. W = J = 2, B = 64 MiB and spec's exact Q
are settled policy, not measurements. No latency/throughput benchmark is required.
Opaque allocations have explicit logical weights, not claimed heap maxima.

## Project structure

| Path | Planned change |
| --- | --- |
| `src/webhook/ca_executor.rs` (new private module) | Concrete executor, checked sizing, reservation/envelope/reply, process initialization and scope owner; measured inline arithmetic/state proofs. No general scheduler/public trait. |
| `src/webhook.rs` | Module declaration, borrowed GET admission, move-only PUT and charged delivery miss; typed unavailable outcome, scope plumbing; retain builder/cache/retry owners. |
| `src/api.rs` | Preserve guards/query/extractors; pass scope, map 503, consume charged replies and publish PUT under scope guard without await. |
| `src/lib.rs` | Initialize before listener admission, own/drop scope token, close at stop entry, preserve join order. |
| `src/webhook/ca_construction_tests.rs` | Existing cold-path owner for held-builder, capacity, cancellation, persistence, cache and lifecycle proofs; replace old coupling expectations with causal isolation. |
| `src/api/webhook_handler_tests.rs` | Actual GET/PUT handler cancellation/close seams keyed by server identity. |
| `src/smtp.rs` | Existing cfg(test) DATA-submission observer only if needed; no production edits. |
| `tests/webhook.rs`, `tests/api.rs` | HTTP/TLS overload/precedence and served-schema proofs; preserve complete-root/later-root/>32-root cases. |
| `README.md`, `src/static/llms.txt`, `src/static/openapi.json` | Defaults/refusal costs and CA-only isolation, each 503 and existing 500. |
| `docs/decisions/README.md` | Index 0078 as proposed during implementation; no decision acceptance. |
| `specs/bounded-ca-executor/{spec,plan,tasks}.md` | Owning corrections/proof pointers; preserve dated clarification entries. |

Existing burst/p2/persistence/lifecycle/rate suites prove unchanged production
behavior. No new framework, cache, metrics family or persistent model.

## Data model, ownership and locking

Use a concrete job kind (GET snapshot, PUT targets, delivery PEM), reply kind
(projection, raw+echo, key+client/error), one non-cloneable Reservation and a
ChargedReply. Envelope owns input, scope reference and oneshot sender; waiter
owns only receiver/cancellation. Never return uncharged raw data from await.

Process state records idle/reserved slots and sum Q under one short mutex.
Admission uses try_lock and refuses contention. Each reserved slot has one
bounded handoff mailbox/channel, never a queue behind busy work. Entry checks
receiver/scope closure, then catches job unwind, produces a charged reply and
sends; failed send drops it. A retained reply keeps its slot unavailable even
when the thread is idle. Release slot/bytes only after owned inputs/outputs drop.
No parsing/destruction under capacity state. Refund can take the short mutex;
it cannot wait for a job or trigger replacement.

Initialize once before listeners. Create both worker channels/threads before
exposing the facility. Spawn failure closes startup channels and joins only idle
threads already created; retain unavailable state, continue unrelated server
startup and refuse CA work. Success intentionally detaches handles. Test-only
factory injection covers partial failure without host resource exhaustion.

Scope has a short live/closed publication lock plus existing shutdown notification.
Admission uses try-lock; close and synchronous publication take the short lock
normally, never across await or CA work. Order: GET configuration read → scope →
capacity reservation. Drop scope/capacity guards before cloning under configuration
read, then release configuration before submission. Workers never take config/cache
guards. Publication uses scope → configuration write OR cache mutex, never scope →
configuration read; GET's scope acquisition is try-only, avoiding a blocking cycle.
No capacity guard during publication/disposal. Cache loser/eviction destruction
happens outside publication guards.

Reservation under live scope is the admission point. Closure afterward cancels
unentered work; entered work stays charged. Publication under live guard is the
synchronous effect point. Cancelled/closed owner discards charged reply, without
install/cache insertion/send. Cache hits keep normal accounting but check live
authority before authorizing the next delivery request. Already-authorized sends
may be in flight at close and retain existing abort/join behavior; no promise
about the last network byte relative to stop.

### Caller integration

GET: query first; borrow installed targets; compute Q without copies; reserve;
clone once; release read guard; submit. Do not call current() first. Charged
projection transfers synchronously to response owner after await.

PUT: extract first, size actual Vec/String capacities, reserve and move the list.
One job validates all then creates safe echo. Live handler consumes raw+echo
under scope guard, performs one set and constructs response without await.
Any earlier refusal/failure/cancellation preserves installed raw bytes.

Delivery: cache lookup by borrowed PEM; miss sizes/reserves before allocating
key. Job returns key+client. Keep charge through winner recheck, key clone,
insertion and loser disposal. Preserve map/deque mutex/LRU, successful actual
build counts and actual evictions. Thread scope through dispatcher/deliver without
changing public synchronous helpers or retry/ledger ownership.

Worker unwind maps to existing fixed API Worker/delivery construction error;
admission gets distinct Unavailable. Invalid PUT keeps value-free Invalid errors.
Catch only job execution, never Store/dispatcher. Panic hooks run before catch:
keep injections fixed/keyed and do not claim catching suppresses arbitrary logs.

## Contracts and proof design

No persistent migration. Four-field request/response schemas remain; separately
add GET/PUT 503 while preserving query/error statuses, including GET 400/403/500.
Docs disclose fixed defaults, oversized-state refusal, no fairness, process
lifetime and logical-byte exclusions. No hidden bound on synchronous embedders.

Adapt projection_capacity_case into
`all_ca_workers_held_smtp_persists_before_release` and
`cancelled_get_retains_charge_without_blocking_smtp`. Use temporary data_file,
current-thread runtime max_blocking_threads(1), unique inbox/message and actual
HTTP GETs entering both builder holds. Excess GET and valid PUT return 503.
Observe real DATA parse submission/final 250 and exact HTTP mail/body before any
release. Reopen database after releasing/stopping/dropping original Store handles.
Keep hard-kill persistence proofs: graceful reopen alone does not prove fsync
precedes 250.

For cancellation, an identity-keyed actual-handler seam aborts/joins the handler,
not just the HTTP client. Both charges stay held. Repeat SMTP/query success before
release. Watchdogs only clean up failures; success asserts unused watchdog and
active holds. A test-only source control restoring shared-pool placement must
fail before watchdog release could manufacture success; restore source and bind
negative/restored-positive receipts to exact trees. Retain old coupling receipts
as historical evidence, not contradictory active expectations.

`mixed_ca_admission_obeys_job_and_byte_bounds` observes held jobs across all three
paths, configuration churn and clone-entry observation. Inline synthetic sizing
cases cover exact B/overflow without giant allocations; real spare-capacity
Strings/Vecs cross-check ownership descriptors. Canonical-output tests cover
many small/duplicate roots and empty/invalid values without asserting private
allocator layouts. `ca_envelope_cancellation_transfers_once` holds pre-entry,
running and completed-reply transitions and proves refunds/no late effects.

`ca_scope_close_preserves_dispatcher_order` stops with an abandoned held delivery,
then releases it and proves unchanged terminal totals; separately race PUT/cache
publication against closure. `ca_workers_survive_runtime_replacement` carries
charged work across runtime teardown, starts another server/runtime and proves
refusal, finite reclamation and reuse of the same two thread identities. Partial
initialization tests can own a private test facility; process lifetime/protocol
proofs must exercise the actual global owner.

Scope serialization only to fixtures asserting uncontended admission/global
counters, following existing CACHE_TESTS. Audit other CA tests for interference;
no blind retries or whole-suite serialization to hide overload. Isolated executor
unit tests do not replace global mixed-contention/lifetime proofs.

## Quickstart and verification

Run offline with cached tools, install nothing. Verify evidence, engine sha256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` and
`bundles/self/scripts/shims.sha256`. Execute pinned scripts after shim checks.
Focused tests supplement the complete charter inventory:

```bash
export CARGO_NET_OFFLINE=true
cargo test --workspace --locked --lib webhook::ca_construction_tests
cargo test --workspace --locked --lib webhook_handler_tests
cargo test --workspace --locked --test webhook
cargo test --workspace --locked --test api
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Also run pinned verify's fail-closed native-tls absence proof: only the explicit
absent-package error from `cargo tree --locked -i native-tls` is success, never an
unrelated error. Full suite includes burst/p2/persistence/lifecycle/rate. Compile
warnings sweeping protected facts refuse even if exit zero. Coverage requires
zero missed lines, branches and functions plus passing selftest. Old receipts
and attempted commands never count as passes.

This design checks its committed four-file artifacts and runs every requested
house gate, recording actual failures in ignored receipts. It does not claim
future feature proof. The final sequence check owns advancement; implementation
and verify rerun on their committed candidate. CI/operator retain spec's external
proof ownership; no local network/namespaces acceptance criterion is added.

## Risks

| Risk | Mitigation and remaining limit |
| --- | --- |
| Early refund on cancellation | One charged envelope/reply, explicit transition tests, disposal-before-refund. |
| Close/publication deadlock or race | One synchronous effect guard, try-only admission/lock order, controlled close tests; no await/worker wait under guards. |
| Global fixture contention | Scope exact-global assertions with existing cache-test serialization and keyed barriers; mixed-contention tests assert refusal intentionally. |
| Native build needs runtime | Standalone construction/reuse test first; no Handle capture/shared-pool fallback. |
| Logical bytes mistaken for RSS | Exact weights/exclusions, arithmetic/copy and canonical-envelope tests; no opaque heap maximum claim. |
| Legitimate configurations refuse | Fixed 503 and documented direct-set permanence; clear/replace, never truncate or misclassify as malformed. |
| GET pressure loses deliveries | Explicit first-reservation policy, fixed attempt errors/final totals; only SMTP pool isolation promised. |
| Stuck worker/init failure | Honest unavailable/permanent capacity, restart recovery; no unbounded replacement or stop drain. |
| Sensitive panic diagnostics | Fixed keyed seams/sanitized assertions; no payload formatting/global hook changes. |
| Watchdog creates false progress | Assert active holds/watchdog unused before DATA/query success; persistent exact-mail and restored-shared-pool control. |
| Coverage/tool limitations | Keep new branches measured and name exact failures; never weaken gates or claim complete implementation. |
