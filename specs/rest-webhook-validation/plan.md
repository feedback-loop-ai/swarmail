# Implementation plan — REST and webhook validation

Implements [spec.md](spec.md) and proposed
[0048](../../docs/decisions/0048-rest-webhook-validation.md).
This is a design handoff, not an implementation or gate receipt.

## Technical context

Single Rust crate; existing axum/serde request parsing, reqwest with rustls,
rustls-pemfile, Tokio and tracing are sufficient. No dependencies, features,
license allowances, persisted fields or configuration knobs change. The
checkout contains a spec template but no plan template; this plan supplies
the required technical context, constitution, structure, research, data model,
contracts and quickstart sections directly, following the existing spec plans.

`set_webhooks` already validates every target before `Webhooks::set`.
`WebhookTarget::validate` calls the inbox validator (batch 1) and a PEM helper
that currently takes only the first certificate. `client_for` holds the
bounded process-global CA cache and performs the missing TLS build. Count
reuses `ListQuery`; assert reuses `AwaitQuery`; the three view handlers ignore
queries. `Json<T>` currently rejects data errors with axum's 422.

## Constitution check

Preserve synchronous commit-before-acceptance, subscribe-before-scan,
oldest-only pruning and ingest extraction. No runtime edits to store,
persistence, SMTP/POP3, extraction, stdio, charts, bundle policy or publication
guards are needed. A strictly test-only observation at an existing owner is
permitted if necessary to prove refusal-before-subscribe; no production seam.
Retain all earlier-batch protections named in the spec, including the identity
sidecar's macOS repair and the metric-attribution cap.

0048 remains proposed and names the accepted 0025/0029 exception explicitly.
Do not silently remove 0030's inert assert limit or treat the intake's status
labels as authority. Use real servers, no mocked protocol loop; follow 0020:
deterministic unit tests stay inline, cold failure paths alone justify a
declared sibling harness, and public-wire tests live in `tests/`. No sleeps
as synchronization, clock-dependent certificate validity assertions, shared
mutable test globals, coverage exclusions or denominator reductions.

## Research and resolved questions

Both positions and all issue evidence were read. Current main is `ffded10`;
`2a325c9` is an ancestor. Decisions 0001–0047 exist, so 0048 is next. The
matching intake is `batch3-intake-framing.md` under `.brokkr/state/epic120/`;
the tasks directory contains only older coverage framing. The spec's
synthesis table resolves disagreements without importing factual errors:
0041 explicitly disclaims configuration echo round trips, 0030 explicitly
preserves assert limit, and the registry cap limits metric labels only.

The CA authority is the enabled delivery stack's client build, not
`Certificate::from_der` alone. Iterating all certificates followed by one
client build per target exercises root loading while keeping the existing
boundary policy. No separate ASN.1 parser or certificate-time check is needed.

Use typed `JsonRejection` variants for the local 400 exception. Matching
serde diagnostic text to distinguish null/unknown/type errors would be brittle
and leave ambiguous 422 cases. Map the entire `JsonDataError` class and keep
other variants' existing responses. Verify diagnostic paths do not echo
supplied values; use a field/path-only diagnostic if the default source text
exposes a credential-bearing value. Do not normalize chaos or all extractors.

## Project structure and files

| File | Intended work |
|---|---|
| `src/webhook.rs` | Complete uncached CA builder; reuse from validation and cache miss; close target deserialization with existing present-string helper; bounded error-chain formatting; explicit tracing message; inline deterministic proofs. |
| `src/api.rs` | Closed SeedRequest, CountQuery, AssertQuery and empty view query; local JSON data-rejection mapping for two handlers; docs and parser/schema unit pins. |
| `src/model.rs` | Reuse `present_string` and `validate_inbox`; no new production rule expected. |
| `tests/webhook.rs` | Real invalid-CA/body refusals and full-state preservation; generated later-root HTTPS; existing redaction/direct-delivery/retry/cache proofs. |
| `tests/api.rs` | Seed no-effect, closed query, JSON status matrix, successful filter/assert/view behavior and served contract proofs. |
| `tests/binary.rs` | Stop-message position and cause-chain/redaction; isolate global cache observations in a child process if needed. |
| `tests/common/mod.rs` | Only necessary real-server/process fixture support and bounded cleanup. |
| `src/static/openapi.json`, `src/static/llms.txt`, `README.md` | Exact accepted keys/statuses, CA loadability, input/output null distinction, compat tolerance and non-replayable echoes. |
| `docs/decisions/README.md` | Index 0048 as proposed during implementation. Older records need at most clearly dated proposal pointers; no accepted text rewrite. |

Only `specs/rest-webhook-validation/{spec,plan,tasks}.md` and
`docs/decisions/0048-rest-webhook-validation.md` belong in the design commit.
Keep product documentation and its executable contract tests with implementation.

## Implementation approach

1. Establish the authorized branch from current main containing `2a325c9`;
   preserve earlier batches and any operator changes. Verify the decision
   number and GPG configuration. Sign the four design artifacts before runtime
   changes. The design environment refused fetch and branch creation because
   `.git` is read-only; resume Git operations in an environment permitting
   them. Do not redirect Git metadata or create an unsigned substitute.
2. Replace the first-only CA path with `build_ca_client(pem)` in the same
   module. Iterate certificates to exhaustion, reject no certificates, add
   each root to `client_builder()` and finish `build()`. Keep one named
   `ca_pem_error` family. Validation builds then drops one temporary client
   per target. `client_for` delegates only its existing cache-miss build;
   hits, eviction order, entry cap and counters remain owned there. Keep
   validation outside both cache and configuration locks, with no dial.
3. Add a small local source-chain formatter used by CA builder diagnostics
   and delivery transport errors. Retain the outer Display, append up to
   eight source causes, and stop there to bound traversal. Run delivery
   errors through `without_url()` first. Test no-source, nested and truncated
   chains deterministically inline, plus a real transport cause without
   OS-specific full-string matching. Existing cause-less HTTP status errors
   remain meaningful and unchanged.
4. Close WebhookTarget and SeedRequest. Apply `default` plus existing
   `present_string` to the three optional webhook strings; preserve Serialize
   behavior. Change only the seed and webhook handlers to receive/examine
   typed JSON extraction results. Map data errors to native 400; forward
   non-data rejection responses unchanged. Share a small mapping helper
   locally if needed, not a global middleware/validation framework. Ensure
   no offload, store mutation or target replacement precedes successful parse.
5. Split count and assert into flat closed structs and one empty closed
   struct for views. Keep `Filter` conversion, signed `since_ms`, exact count
   defaults and the inert usize limit; remove obsolete cross-owner methods
   only when their functionality has moved. No serde flatten abstraction.
   Parse RawQuery after inbox admission and before view/read/subscription.
   Refused feed requests never invoke `feed_stream`; valid feed ordering stays.
6. Put `DISPATCHER_STOPPED` in explicit tracing message position, preserving
   its constant and the source-mapping comment. Do not alter dispatcher
   lifecycle, counts, per-mail exhaustion warnings or retry timing.
7. Update fixtures that encoded omission as null. Response tests inspect
   JSON as a response shape; do not deserialize redacted echoes into the now
   stricter request type or alter responses just to accommodate fixtures.
   OpenAPI uses separate request/response schemas where necessary; this does
   not require runtime DTO duplication. Document exact query sets and the
   two local data-error mappings; qualify the strict-input sentence explicitly.
8. Collect each behavioral red/green receipt and finish the full gate
   inventory. Commit signed implementation with the issue-to-test mapping.
   Do not push, open a PR, merge or close issues.

## Data model and contracts

No persistent data model changes. Target storage remains a list of current
WebhookTarget values with one lock-protected replacement. Delivery snapshots
remain per mail; a concurrent PUT does not tear a snapshot. Validation adds
work before the swap without changing success durability (configuration is
in-memory only). Unrelated in-flight deliveries may change their own metrics;
the no-effect guarantee means PUT validation itself performs no cache mutation.

CountQuery has four optional filter fields. AssertQuery adds optional usize
count and limit; omission yields exact count 1, limit is parsed but unused.
The empty view query has no fields. Seed's required `to` and optional strings
retain their existing domains. Webhook request optional strings are non-null
when present; response optional strings retain nullable output. Refusals use
existing route/domain error envelopes except the explicitly scoped data-error
400 amendment. Store acceptance, query visibility and extraction do not change.

## Proof design

Pair proposed test names in [tasks.md](tasks.md) with issue and AC identifiers;
equivalent names are fine only when the final commit records the actual map.
Install a prior target carrying credentials, snapshot `srv.webhooks.current()`
where public access already exists, submit each malformed body, compare every
stored field and verify a real delivery still uses the prior secret/auth.
This proves more than redacted GET equality. Exercise a bad target after a
good target and a bad certificate after a good certificate independently.

Generate CA fixtures per run. Keep verification on, serve a real HTTPS target
whose chain uses the second CA, and wait on actual received requests. Prove
no PUT dial through a controlled listener with observation bounded by the
completed request/cleanup, or a test-only observation at the actual send path
with a positive control. Do not infer absence of network work from fast return.
Run cache isolation in a dedicated process or with an existing isolated cache
seam, including an LRU victim probe; do not compare noisy shared global totals
across parallel tests or reset process-global counters.

For refused-feed ordering, combine real 400/no-SSE wire proof with a
deterministic inline observation of subscription entry using an existing
per-instance seam, adding a cfg(test)-only seam only if needed. Include an
accepted-feed positive control so the observation can fail. Do not assert
against cumulative awaits metrics: they do not measure feed subscriptions.

Prove input-schema agreement using independent expected key sets, actual
requests and served JSON schema checks, not text-presence tests alone. Include
unknown filter-looking keys on empty query routes, numeric failures on
retained fields, assert's above-cap inert limit and exact count, and the
unchanged chaos 422. Pin response fields as present/null separately from
request non-null semantics.

Run new wire regressions against `2a325c9` in an isolated control checkout and
the repaired tree; also record `ffded10` failures for remaining defects. If a
private seam cannot compile on the old tree, restore only the exact defective
behavior in an isolated control and identify that substitution. Blank-inbox,
valid HTTPS, cache/directness and retry proofs already pass current main:
record them honestly as preservation proofs, not new reds. Output-shape
preservation tests need not fail baseline; newly refused echo replay does.
Keep all control mutations out of the final tree.

## Quickstart and validation

After implementation, run targeted library, API, webhook and binary suites
as each owning change lands. For manual inspection against a local server:

```bash
curl -i -H 'Content-Type: application/json' --data '{"to":"a@x.io","subjet":"typo"}' http://127.0.0.1:8025/api/v1/inboxes/batch3/seed
curl -i 'http://127.0.0.1:8025/api/v1/inboxes/batch3/count?limit=1'
curl -i -X PUT -H 'Content-Type: application/json' --data '[{"url":"http://127.0.0.1:9/","secret":null}]' http://127.0.0.1:8025/api/v1/webhooks
```

Each answers 400 naming the defect after repair. These probes supplement,
not replace, no-effect and real-delivery tests. Final implementation gates:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo fmt --all
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all
cargo test --workspace --locked
cargo fmt --all
bash scripts/coverage-gate.sh
cargo fmt --all
bash scripts/coverage-gate.sh selftest
cargo fmt --all
bash scripts/publish-guard-test.sh
cargo fmt --all
cargo deny check licenses
```

No TMPDIR override. Require exact missed == 0 lines, branches and functions.
Run the existing fail-closed native-tls absence proof. Verify owns
`brokkr compile --bundle bundles/self` from the tooling-pinned binary; record
any sandbox refusal and do not install a replacement or claim compile passed.
CI retains chart/advisory/tag-only release-rate and port-80 responsibilities.
Do not claim this batch resolves later epic policy warnings.

If cargo itself is refused, stop immediately, preserve
`.brokkr/state/epic120-rest-webhook.patch`, and report the blocker. If Git or
signing is unavailable, preserve reviewable artifacts and a patch; never
bypass permissions, claim a nonexistent commit or turn off signing. The
design validator checks files/headings/checkboxes and a proposed decision;
its success cannot prove Git delivery or implementation gates.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Trailing roots are ignored or validation diverges from delivery. | One complete builder, later-invalid refusal and later-valid-root real HTTPS proof. |
| PUT churn displaces active cached clients. | Uncached build-and-discard; prove no cache effects or lock access, retain bounded LRU. Rebuild on later misses is accepted. |
| Many valid targets consume synchronous validation CPU. | Retain existing request size bound, one temporary client at a time, no shared lock. No latency/global concurrency guarantee; new admission limits are out of scope. |
| Valid PUT is mistaken for guaranteed TLS success. | Publish root-loadability-only semantics; preserve actual handshake verification and no added validity-time policy. |
| Refusal clears a secret or partially installs targets. | Parse and validate whole list before one swap; compare actual stored fields and subsequent authenticated delivery. |
| Added source strings expose credentials. | Strip URL before formatting, bound traversal, real userinfo/secret log assertions and malicious wrong-type input probes. |
| Local 400 mapping accidentally changes every route or syntax/media/size errors. | Match typed data rejection only in two handlers, explicit proposed amendment and status matrix including unchanged chaos. |
| Clients replay nullable diagnostic echoes. | Preserve response shape, document request/response distinction and retention of original input; pin replay refusal with no effects. |
| Query closure removes accepted assert parity. | Preserve and document inert typed limit, default/zero/exact count and no page cap. |
| Compat typos still broaden scope. | Explicit documented exception permitted by #134; preserve upstream client tolerance and strict latest. |
| Global counters or test sleeps hide races. | Dedicated subprocess/per-instance seams, observed events with cleanup bounds, no global resets or sleep synchronization. |
| Refusal accidentally registers a feed watcher. | Parse before feed construction; wire proof plus observed subscription entry and positive control. |
| A proposal or structural check is reported as completed delivery. | Proposed status, unchecked implementation tasks, distinct gate receipts and explicit Git/signing blocker reporting. |
