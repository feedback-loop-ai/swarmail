# Implementation plan — REST and webhook validation

Implements [spec.md](spec.md) and proposed
[0048](../../docs/decisions/0048-rest-webhook-validation.md).
Resume confirmation, 2026-10-18: `e1e142a` committed the design and `2079455`
implemented it. Preserve both. This plan records the delivered approach and
remaining verify/review handoff; it is not a fresh execution-gate receipt.
Only the three existing spec files change in this confirmation. Decision 0048
stays proposed and unchanged; no new semantic decision is needed.

## Technical context

Single Rust crate; existing axum/serde request parsing, reqwest with rustls,
rustls-pemfile, Tokio and tracing are sufficient. No dependencies, features,
license allowances, persisted fields or configuration knobs change. The
checkout contains a spec template but no plan template; this plan supplies
the required technical context, constitution, structure, research, data model,
contracts and quickstart sections directly, following the existing spec plans.

`set_webhooks` validates all targets before one `Webhooks::set`.
Validation and delivery cache misses share `build_ca_client`, which reads all
certificates and completes the TLS build; validation never uses the cache.
Count/assert have closed flat query structs and views parse an empty closed
query. `json_body` maps data errors to 400 only for seed and webhook. The
local `WebhookTargets`/`TargetSlot`/`Text` readers and `ShapeVisitor` refuse
wrong-shaped values by kind without echoing credentials. Retain this local
machinery: plain serde diagnostics would lose that protection.

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

Both current positions and all issue evidence were read. The original base
is `ffded10`, descending from `2a325c9`. Decision 0048 already exists. The
original intake is `batch3-intake-framing.md` under `.brokkr/state/epic120/`;
the tasks directory contains only older coverage framing. Resume authority is
`resume-intake-framing.md` in the same directory. The spec's
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

| File | Implementation ownership |
|---|---|
| `src/webhook.rs` | Complete uncached CA builder; reuse from validation and cache miss; close target deserialization with local value-free shape readers; bounded error-chain formatting; explicit tracing message; inline deterministic proofs. |
| `src/api.rs` | Closed SeedRequest, CountQuery, AssertQuery and empty view query; local JSON data-rejection mapping for two handlers; docs and parser/schema unit pins. |
| `src/model.rs` | Reuse `present_string` and `validate_inbox`; no new production rule expected. |
| `tests/webhook.rs` | Real invalid-CA/body refusals and full-state preservation; generated later-root HTTPS; existing redaction/direct-delivery/retry/cache proofs. |
| `tests/api.rs` | Seed no-effect, closed query, JSON status matrix, successful filter/assert/view behavior and served contract proofs. |
| `tests/binary.rs` | Stop-message position and cause-chain/redaction; isolate global cache observations in a child process if needed. |
| `tests/common/mod.rs` | Only necessary real-server/process fixture support and bounded cleanup. |
| `src/static/openapi.json`, `src/static/llms.txt`, `README.md` | Exact accepted keys/statuses, CA loadability, input/output null distinction, compat tolerance and non-replayable echoes. |
| `docs/decisions/README.md` | Index 0048 as proposed during implementation. Older records need at most clearly dated proposal pointers; no accepted text rewrite. |

The original design commit contains the three spec files and 0048. This
resume commit contains exactly `specs/rest-webhook-validation/{spec,plan,tasks}.md`.
The table records implementation ownership, not newly authorized edits.
Change production code only if a concrete acceptance gap is proven.

## Implementation approach

Steps 2–7 record the approach delivered by `2079455`; do not redo them.

1. Confirm branch `epic120-rest-webhook`, signed ancestors `e1e142a` and
   `2079455`, and the unchanged lifecycle test. Do not recreate the branch,
   rebase, amend or squash. Sign this documentation confirmation with the
   configured key; a signing failure remains a blocker.
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
4. Close WebhookTarget and SeedRequest. Apply `default` plus local
   `optional_text` to the three optional webhook strings, with `text` for URL
   and value-free shape readers for target objects and the array; preserve
   Serialize behavior. Change only the seed and webhook handlers to receive/examine
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
8. Reconcile the existing issue-to-test receipts in `2079455` with the tests
   named in tasks.md; unchecked boxes from the original design are not evidence
   of missing implementation. Implement changes nothing absent a real gap.
   Verify runs fresh full gates and the unchanged lifecycle suite on a quiet
   host, recording load and every failure. Review assesses acceptance and
   residual proof scope; ship is a local handoff. Do not push, open a PR,
   merge or close issues.

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

Actual test names in [tasks.md](tasks.md) map to issue and AC identifiers;
`2079455` records original red/green evidence. The proof design below remains
the review standard for those existing tests.
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

Resume inspection found no application tracing or middleware copying the
seed/webhook 4xx envelopes into logs. This is a source-inspection finding,
not a new dynamic log-capture test or a claim about external logging.
Connection-refusal redaction is pinned by `a_transport_error_carries_no_url`
and the real-binary SIGTERM test. DNS/TLS source-chain non-disclosure has no
separate execution pin; review must not call this coverage universal.

## Quickstart and validation

Verify runs the existing library, API, webhook and binary proofs as part of
the fresh workspace gates. For manual inspection against a local server:

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
cargo test --locked --test lifecycle
cargo fmt --all
bash scripts/coverage-gate.sh
cargo fmt --all
bash scripts/coverage-gate.sh selftest
cargo fmt --all
bash scripts/publish-guard-test.sh
cargo fmt --all
cargo deny check licenses
```

Record host load (for example `uptime`) before and after test runs; avoid
competing heavy builds. Leave `tests/lifecycle.rs` and its timings unchanged.
Retain failed receipts and investigate recurrence instead of retrying until
a green result hides it. No TMPDIR override. Require exact missed == 0 lines, branches and functions.
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
| Many valid targets consume synchronous validation CPU. | Retain the 2 MiB body bound, one temporary client at a time, no shared lock. No latency/global concurrency guarantee; `spawn_blocking` and new admission limits are separate follow-ups. |
| Valid PUT is mistaken for guaranteed TLS success. | Publish root-loadability-only semantics; preserve actual handshake verification and no added validity-time policy. |
| Refusal clears a secret or partially installs targets. | Parse and validate whole list before one swap; compare actual stored fields and subsequent authenticated delivery. |
| Added source strings expose credentials. | Strip URL before formatting; retain real connection-refusal userinfo/secret assertions and wrong-type probes. Eight causes bounds traversal, not bytes. DNS/TLS non-disclosure is not separately execution-pinned; review source paths and dependency changes. |
| Local 400 mapping accidentally changes every route or syntax/media/size errors. | Match typed data rejection only in two handlers, explicit proposed amendment and status matrix including unchanged chaos. |
| Clients replay nullable diagnostic echoes. | Preserve response shape, document request/response distinction and retention of original input; pin replay refusal with no effects. |
| Query closure removes accepted assert parity. | Preserve and document inert typed limit, default/zero/exact count and no page cap. |
| Compat typos still broaden scope. | Explicit documented exception permitted by #134; preserve upstream client tolerance and strict latest. |
| Global counters or test sleeps hide races. | Dedicated subprocess/per-instance seams, observed events with cleanup bounds, no global resets or sleep synchronization. |
| Refusal accidentally registers a feed watcher. | Parse before feed construction; wire proof plus observed subscription entry and positive control. |
| Load causes the unchanged lifecycle proof to fail again. | Quiet host, recorded load, fresh suite receipts; retain failures and investigate without timing edits. |
| A proposal or structural check is reported as completed delivery. | Keep 0048 proposed; distinguish existing implementation from pending verify/review/ship, and report any Git/signing blocker. |
