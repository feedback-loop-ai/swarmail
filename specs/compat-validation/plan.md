# Implementation plan: compat and validation repairs

Implements [spec.md](spec.md) and proposed
[0044](../../docs/decisions/0044-compat-validation.md). This is a design
handoff, not an implementation receipt.

## Technical context

One Rust crate; Tokio, axum, serde/serde_json, regex, mail-parser, rusqlite,
DashMap and futures-util are already available. No dependency or allowlist
change is required. HTTP API, MCP, UI and compat share Store; SMTP and seeds
share build_email. Store::offload already implements awaited blocking work
with established panic behavior. Persistence already has additive column
migration, a single projection constant and ordered restore by seq/rowid.

There is no `.specify/templates/plan-template.md` in this checkout. This plan
supplies technical context, constitution check, project structure, research,
data model, contracts and quickstart directly. No extra research/data-model
files are required by this four-artifact design commission.

## Constitution check

- 0001: capture the sender before insert and persist it inside the existing
  commit-before-visibility mutation. Do not add a second write or async gap.
- 0002: staging refuses before subscribe; admitted waits keep subscribe then
  scan. Concurrent batches change scheduling, not the Store wait algorithm.
- 0003: latest is a read accessor, never a reordered deque or mutation-lock
  operation. Oldest-only pruning and restore order remain unchanged.
- 0005/0031/0040: extraction remains ingest-time; snippets and subject keys
  are derived views without writes, historical reparse or new extraction.
- 0011/0025/0029/0030: preserve delete selectors, existing error envelopes,
  validation ownership, staged compat kind handling and the 250-item cap.
  0044 proposes the scoped semantic changes; do not edit accepted ADRs.
- 0020/0035: real servers for wire proofs; deterministic inline tests by
  default; a sibling cfg(test) harness only for concrete cold failure paths.
  No sleeps as synchronization, global mutable test gates, mid-band chaos,
  wall-clock validity assertions, exclusions or coverage-floor changes.
- Preserve single-writer sidecar/5 s busy policy, webhook routing and ledger,
  stdio concurrency/classification, extraction pins, Origin/Host guards,
  cumulative awaits registry and `_uncounted` folding.

## Project structure and files touched

| Paths | Responsibility |
|---|---|
| src/model.rs | Add envelope_from; own shared inbox validation and optional-string deserialization helpers beside data types. |
| src/smtp.rs | Capture envelope before header fallback; retain shared seed builder; apply blank-name validation at username admission if not already equivalent. |
| src/pop3.rs | Reuse blank-name validator at username admission; preserve failed-auth state behavior. |
| src/persist.rs, src/store/persist_tests.rs | Add nullable column, insert/projection/decode; old-file migration and existing boundary/restart proofs. |
| src/store.rs | One read-only latest accessor; preserve all mutation/filter semantics. Local controlled-order tests. |
| src/api.rs | Handled Bytes rejection, strict query structs, shared inbox-page shape, native admission checks, seed string staging. |
| src/chaos.rs | Strict present-string deserialization of error; per-event null clear unchanged. |
| src/mcp.rs | Classifier/dispatch, join_all, descriptor-derived key refusal, search offset, shared inbox validator and schemas. |
| src/stdio.rs | Update bridge consumers/comments/tests only as required by shared classifier; no new bridge protocol owner. |
| src/ui.rs | Inbox pagination/links and blank-name path refusal, retaining escaping/CSP. |
| src/compat.rs | Scoped latest resolver, validated containing-query staging, envelope accessor, bounded snippets, borrowed MailHog serialization/capped buffer, complete page offload, short-circuit matching and unread key. |
| src/threads.rs | Extend strip_marker with character indices; collapse whitespace in subject fallback. |
| tests/api.rs, tests/mcp.rs, tests/stdio.rs, tests/ui.rs | Real transport validation, pagination, notification/classifier and concurrent batch proof. |
| tests/compat.rs, tests/persistence.rs, tests/smtp_edges.rs, tests/pop3.rs | Scoped latest, wire shapes, sender divergence/null path, migration/restart and username admission. |
| src/compat/page_work_tests.rs | Only if the offload proof needs the specified independent cleanup watchdog/cold failure arm; declare cfg(test) in its owner. |
| benches/read_stall.rs | Add simultaneous compat page/greeting measurements and document isolated peak-memory procedure; retain current bench paths. |
| README.md, src/static/openapi.json, src/static/llms.txt | Publish actual limits, nullable envelope provenance, recipient predicate, strict staging, latest refusal, compat differences and thread fallback. |
| tests/store_contract_docs.rs, tests/listener_contract_docs.rs | Extend relevant doc pins; do not build a second general doc framework. |
| docs/decisions/README.md | Index proposed 0044 during implementation; not part of this design-only commit. |
| specs/compat-validation/{spec,plan,tasks}.md, docs/decisions/0044-compat-validation.md | Exactly the design artifacts; later dated clarifications/proof receipts update their owning requirements. |

No new production module is needed. Existing Email construction fixtures
may also require the additive field in tests and benches; each must choose
known envelope vs legacy unknown deliberately, not invent a blanket default.
No Cargo/lockfile/bundle/gate edits or changes to accepted decisions are planned.

## Research and resolved choices

Source inspection confirms `received_ms` is sampled at parse start, while
deque order is commit order. Therefore max over deque tails is incorrect
even within one inbox. Global latest already uses the (received_ms, id)
ordering; retain it with O(n) comparison and O(1) retained winners. A
DashMap iteration is not a global atomic snapshot; do not promise one.

0025 permits null only for per-event ChaosConfig Option<ChaosRule>, not its
error string. 0029 already rejects a typed Args layer; existing descriptor
properties provide the unknown-key vocabulary without another registry.
0039 explicitly preserves sequential server batches; 0044 must amend that
sentence as well as the missing-ID classifier rule. The cumulative awaits
registry folds names rather than rejecting requests; keep its tests, do not
write a nonexistent cap-refusal test.

MailHog's duplicated MIME fields are required wire shape. Borrowing removes
intermediate copies; actual-byte counting handles escaping and field growth
without a fragile projected-size multiplier. A byte-regex conversion would
change searches for U+FFFD against malformed raw bytes, so retain the string
regex in this repair. Search may still scan the full store and allocate one
temporary raw string; snippet prefixes are not search truncation.

## Data model and migration

`Email.envelope_from: Option<String>` is serialized as an additive nullable
field. None means unknown, Some(empty) a known null reverse path; both
persist distinctly. Add `envelope_from TEXT` to fresh SCHEMA and
ADDED_COLUMNS, COLUMNS, INSERT bindings and row_to_email together. Adjust
column-position comments and fixtures. Do not add a default, UPDATE backfill,
schema-version framework, index or alternate connection. Migration stays
idempotent under the existing lock and PRAGMA policy.

build_email records its from_envelope argument before the existing fallback
can consume it. build_seed_email already passes its staged/defaulted sender
through that builder; verify both transports rather than adding a second
seed policy. A compat helper returns known envelope (including empty) or
legacy header address. kind=from uses that predicate locally; it does not
reuse native Filter.from after changing its meaning. Mailpit header From
remains header From even when ReturnPath differs.

## Implementation approach

### Validation and batch execution

Use a handled Bytes extractor rejection at mcp_endpoint, preserving axum's
streaming body limit and outer guards. Map size rejection to the exact
spec envelope, parse bytes with from_slice, and keep error_object as owner.
Update classify and handle together: test non-object before missing ID;
invalid batch members contribute a null placeholder to expected replies.
Use join_all on admitted handle futures and filter notification None after
completion. Do not spawn per-member tasks. Bridge success and transport
error paths must use the same updated Envelope interpretation.

At tool_call, validate the arguments container first, find the existing
descriptor by tool name, then check keys against inputSchema.properties
before tool effects. Unknown tool retains its prior code; set_chaos keeps
its pinned wording. Publish additionalProperties false for every descriptor,
including tools without inputs. Preserve existing typed readers and bound
checks. Offset uses checked usize conversion, skip then take, without
offset+limit arithmetic; total is unpaged.

Apply deny_unknown_fields to the native flat query structs and new page/
latest shapes. Keep compat ListParams/SearchParams's established staging;
do not opportunistically reject upstream-only query keys there. Native
QueryRejection mapping must keep JSON ApiError and name the bad key. A shared
present-string deserializer uses String::deserialize and wraps Some;
serde default handles only genuine omission. Make errors name the field,
using small field wrappers if serde's default message lacks the name.
Use it for seed optional strings and ChaosRule.error, never the per-event
Option<ChaosRule>. Existing validators still own string content/domain.

One pure inbox validator in model.rs rejects name.trim().is_empty(), returning
the shared sentence `inbox must not be blank`. Call it at API/MCP/compat/UI
admission, shared seed construction and protocol username admission. Adapt
protocol refusals to their established SMTP/POP3 envelopes; never alter
session state before successful validation. Do not make Store::insert
fallible for names or rename valid whitespace-bearing strings. Omitted
selectors use documented defaults; explicit blank selectors never do.

Page inbox discovery after validating query size, preserving array output,
name sort and counts. UI navigation escapes attributes and encodes names as
before. For limit=0 show the empty page plus a default-page link; positive
limits use checked/saturating bounds without wrap or zero-progress links.
Tests pin empty beyond-end behavior without pretending the whole store is
empty or providing invalid next links.

### Latest and compat search

Add Store::latest(inbox: Option<&str>) or equivalent: inspect each message
under its short-lived shard read guard, retaining only an Arc winner by
(received_ms, id). Do not clone an entire page or sort. No mutation guard.
One compat resolver stages latest query/inbox, offloads the scan, then keeps
the winner through rendering. Ordinary ID requests retain Store::get.

Inside shared search staging trim once, validate existing kind before
scanning, and apply the spec's token-prefix rule only for containing mode.
Recognize tokens by whitespace, inspect only a leading ASCII identifier and
its colon, exempt ://; name the actual field. No compiled field parser or
list of upstream field names. Preserve the established HTTP shape of existing
kind/query errors and JSON shape of page-cap errors. Turn contains into
short-circuit matching with raw last and update its false rationale.

### Snippets, borrowed pages and bounded serialization

Use a single 8192-byte source-prefix constant for text and HTML; cut down to
is_char_boundary. Collapse whitespace without collecting all words and stop
after the 201st collapsed character. For HTML render only the bounded prefix;
the rendered result is bounded by that input, not by the whole body. Track
source truncation separately from output truncation. No stored snippets.

Keep page Arc owners alive through serialization. Small Serialize views
borrow Email fields, bodies, recipients and both MIME locations. A per-item
Serialize implementation or serialize_with creates and drops the lossy raw
Cow within that item's serialization; do not precollect Cows for the page.
Do not build a Value tree containing bodies or duplicate native Email values.

Serialize directly with serde_json::to_writer into a local capped buffer.
Before each write, compare incoming length against remaining bytes using
subtraction/checked arithmetic; do not append part of an overflowing chunk.
Keep logical bytes <= budget and explicit buffer growth capped at budget
(allocator overhead is outside this logical guarantee). Return a typed local
over-budget cause and translate it to the specified HTTP 400; no general
error framework. Nothing is sent until serialization completes. Exercise
write/flush behavior and refusal with small injected internal budgets; the
public constant remains fixed. Do not try to calculate an exact smaller
fitting limit, or claim a singleton always fits. If a genuine serialization
failure other than the cap is possible, preserve an honest server error and
test its reachable source rather than mislabel it as a client cap violation.

Offload each full compat list/search scan and response build. Validate scalar
parameters before scans, pass owned request state and Arc<Store>, obtain
owned Arcs under read guards inside the closure, then drop guards before
render/serialize. Return final JSON bytes to the async handler, not Json<T>
that would serialize again on the worker. Reuse offload panic propagation.

### Threads

Use char_indices and delimiter len_utf8 for marker boundaries, not byte+1
on a full-width colon. Match the finite marker with Unicode to_lowercase;
allow Unicode whitespace before/after delimiters. Every successful strip
consumes characters. Collapse the remaining whitespace in normalize_subject,
keeping original subject storage and thread_node's header-first branches.
Numbered markers remain nonmatches and retain tests documenting that fact.

## Contracts and proof design

AC1–AC4 use real HTTP and real stdio subprocesses. Use raw streaming HTTP for
invalid UTF-8 and chunked over-limit bodies so Content-Length cannot perform
all the proof. Assert code/status/content type/IDs, execution counts and lack
of side effects. Keep object notification semantics unchanged. Observe wait
readiness through per-inbox counters or instance-owned seams; never sleep.
Configure every wait at the 60 s maximum and establish that all register
before any delivery releases them, then deliver and join. Combined with the
existing per-wait deadline and cap tests, this proves overlapping rather
than additive wait intervals without a timing threshold. Do not use paused
Tokio time to claim a deadline proof: Store currently uses std::time::Instant
for its deadline. Use an independent failure bound for regressions that
would otherwise hang; a deadline for test failure is not a latency assertion.

AC5 uses fresh local stores with intentionally inverted received_ms values
and stable IDs. Wire tests prove inbox isolation and matching raw/headers;
deterministic accessor tests prove comparator semantics separately from time.

AC6 uses a test-only inspection count passed through snippet helpers, not
elapsed-time measurements. It must count examined source bytes and prove
long all-whitespace input does not scan to the end. It has no global state.

AC7's gate is instance-owned and cfg(test)-only. Run a real server on a
current-thread runtime; park the blocking closure after signaling entry.
The test observes SMTP greeting and health before release. A separate
supervisor OS thread provides the failure deadline and always releases the
gate on failure so a mutant running the build on the async worker cannot
deadlock the entire suite. This cold path justifies the named sibling
compat harness if used. Test code still covers production lines. Every
seam must be installed by at least one test and drop/cleanup must join work.

AC8 pins shape independently with golden JSON values on small fixtures,
checks budget exact/over boundaries with escaping, and drives a real page
past 64 MiB. Measure before/after in an isolated process with identical
stored fixtures; capture baseline RSS, peak RSS, emitted size or named
refusal, and actual concurrent SMTP greeting samples. Benchmarks are
evidence, not hard latency assertions. Retained store memory is separated
from incremental page allocation; no global memory promise is inferred.

AC9–AC12 include real SMTP without header-envelope equality, known empty
reverse paths, seeds and repeated persistent restarts. Build a populated
old-schema fixture from the previous explicit schema, not from the new
SCHEMA constant, so migration really runs. Compare IDs, ordering, counters,
raw, old unknown fields and new known fields across reopen. Re-run the full
restore-plan and pragma/lock tests. Assert subject header keys exactly,
not merely equal thread counts that could hide merging two chains.

For each child retain a red-before/green-after command and behavioral
assertion. Tests can first run on unchanged production code or against a
minimal restored faulty behavior after fixture compilation has been adapted.
Compilation errors, timeouts without cleanup, and coverage alone are not
proof. Restore every mutant before gates. Store verbose receipts run-locally
and concise outcomes next to the corresponding acceptance criterion.

## Quickstart and validation

Implementation runs on a feature branch and does not close issues. Start
with the owning targeted suites, then these six required gates:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --locked --test api --test mcp --test stdio --test compat --test ui --test persistence
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
cargo deny check licenses
```

No TMPDIR override; coverage must be exact nonzero 100/100/100. Do not
silently rewrite formatting in unrelated files; review any format output.
Run the established read-stall benchmark with before/after fixtures and
record measurements. Leave no mutants, harness exclusions or generated
assets staged. Product docs and decision index change with implementation.

For the design seat, check the four nonempty artifacts, explicit Acceptance
Criteria/Risks headings, ordered checkboxes with proof, relative links and
proposed 0044. Stage and GPG-sign exactly those four paths with an imperative
subject and a why body; never push. If Git metadata or the toolchain is
blocked by the sandbox, preserve a patch and disclose the exact refusal;
do not relocate Git metadata or report a commit or green gates that did
not occur. The result file reports drafted and the actual delivery state.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Null reverse path becomes a fabricated header sender. | Distinguish None and Some(empty), test fresh SMTP and restart, fallback only for historical None. |
| Migration changes restore/commit behavior. | Existing additive mechanism/connection only; old-schema population, idempotent reopen, query-plan, busy and sidecar tests. |
| A deque-tail optimization chooses an older parse-start stamp. | Scan every scoped item by the documented comparator; inverted-order and tie fixtures. |
| Batch concurrency changes destructive effect order. | Document unordered effects and require client sequencing; preserve reply order and notification execution. |
| Registry mistaken for a concurrency guard. | Pin cumulative folding unchanged; document per-batch bound and aggregate residual. |
| Schema-derived key checking drifts from readers. | Per-tool descriptor/staging tests including valid advertised keys and rejected sentinel keys. |
| New refusals break formerly permissive callers. | Publish named field, null, query and budget rules with migration examples. |
| Budget estimator undercounts escaping or repeated fields. | Count actual writer bytes; exact-boundary/escape/duplicate-MIME tests, no estimate. |
| One huge item cannot fit at any positive page size. | Named refusal suggests raw-by-ID as well as smaller limits; no false fit guarantee. |
| Lossy raw temporary or concurrent requests still consume memory. | Item-local Cow, bounded output, explicit aggregate residual and isolated memory measurements. |
| Offload moves serialization back to worker accidentally. | Return finished bytes; gate covers response build through serialization and uses real SMTP/health. |
| Offload regression deadlocks the current-thread test itself. | Independent supervisor releases gate on a real failure deadline; cleanup joins server/work. |
| Blocking-pool contention and full-store scan cost remain. | Bound previews/output, preserve no-mutation-lock reads, record load measurements without promising global latency. |
| Prefix validation refuses literal colon words. | Exact lexical rule and :// exception documented; no hidden closed list or quoting promise. |
| Unicode marker/prefix slice panics. | Character-boundary cuts and full-width/Cyrillic/Chinese/NBSP/edge-case tables. |
| Orphan keys change while explicit chains accidentally merge. | Exact header-key/chain assertions and documented orphan re-key behavior. |
| Coverage pressure hides cold branches or test code. | Follow 0020 and gate by effect; only concrete cold-path harnesses, no exclusions. |
