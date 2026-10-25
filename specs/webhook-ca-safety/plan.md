> Historical #195 artifact restored on 2026-10-25. The current port and its
> proof obligations are governed by [webhook-ca-port-lows](../webhook-ca-port-lows/spec.md).
> Earlier stops and gate receipts below are dated history. 0067/0068 are
> accepted; 0069 was accepted for #220 on 2026-10-25 and is not implemented here.
> The low shared-blocking-pool availability residual is tracked by #220.

# Plan — complete webhook CA safety

Implements [spec.md](spec.md) under accepted
[0067](../../docs/decisions/0067-webhook-ca-safety.md) and
[0068](../../docs/decisions/0068-webhook-get-failure.md).

## Technical context

Single Rust crate; existing Tokio/axum, reqwest/rustls, rustls-pemfile, base64,
serde and test certificate support suffice. No dependencies, features, public
knobs or new production files are planned. There is no
`.specify/templates/plan-template.md` in this workspace; required sections are
supplied inline. This design commits exactly
`specs/webhook-ca-safety/{spec,plan,tasks}.md`.

## Constitution check

Accept==stored, subscribe-before-scan, oldest-only pruning and ingest extraction
stay unchanged. SMTP acceptance never waits for webhook success. Preserve
complete offline admission, mixed-PEM behavior, raw identity, Store panic
symmetry and delivery/lifecycle rules. 0067/0068 are accepted; no further
semantic choice or acceptance occurs. 0069 and the low shared-pool availability
residual belong to #220. No routing, gate, pin, dependency, production
Store/SMTP or accepted-decision edits.

Follow 0020: branch-stable synchronous tests stay inline/measured; public API
tests use `tests/`; private seams and cold failure/watchdog paths use the
already declared `src/webhook/ca_construction_tests.rs`. Every seam is
cfg(test)-only, owner-local and exercised. Use fresh inboxes with retention
headroom, real servers and isolated hooks. No sleeps as synchronization,
probabilistic chaos, mocks, coverage exclusions or lowered floor.

## Research and resolved design

Both positions and matching intake were read. Inspection confirms
`public_ca_pem` reconstructs then calls `build_ca_client`; GET snapshots;
`prepare_targets` owns validation/echo; `set_webhooks` alone swaps; delivery
builds in `spawn_blocking` then rechecks cache publication. Preserve these
mechanisms. Whole-bundle loadability protects direct-install bypasses; labels
alone do not. No new service, parser or single-flight registry is needed.

Privacy tests currently print whole responses and equality operands on failure.
Remove those paths, including HTTP/JSON helper diagnostics, before negative
controls. PUT cancellation/cache tests largely invoke helpers; strengthen
actual handler coverage. Race helper tests and broad fan-out lower-bound
counters do not prove exact protocol duplicate/eviction accounting.

Cancellation must be observed in the publication-owning task, not inferred
from client disconnection. If necessary a narrow cfg(test) seam observes or
cancels the real request-originated handler at its await boundary, without
replacing preparation or changing production control flow. Observe its drop or
join before releasing construction. A cancelled preparation helper alone does
not discharge this obligation.

## Project structure and touched files

| File | Intended work |
|---|---|
| `tests/webhook.rs` | Safe failure diagnostics; canonical duplicate/noncanonical/unrelated-block fixtures; direct-install empty output; preserve TLS/admission/fan-out proofs. |
| `src/webhook/ca_construction_tests.rs` | Real API holds/cache neutrality, protocol accounting, observed handler cancellation and counter-completion proofs. |
| `src/webhook.rs` | Correct stale authority comments; extend inline projection assertions and only necessary exercised cfg(test) observations. Fix runtime only on demonstrated accepted-contract violation. |
| `src/api.rs` | Correct stale authority comment; if necessary provide narrow exercised cfg(test) observation of actual PUT owner; preserve one swap. |
| `tests/api.rs`, `src/static/openapi.json`, `src/static/llms.txt` | Keep served GET 500 and exact descriptions; align living 0067/0068 references and finite pool/counter statements where stale. |
| `tests/common/mod.rs` | Only if a scoped safe HTTP/JSON helper is required; no broad fixture rewrite. |
| `specs/webhook-ca-safety/{spec,plan,tasks}.md` | Current design, task state and actual proof receipts; retain dated history as history. |

Reuse existing binary/p2/store/SMTP regressions. No new suite or accepted-record
index edit: 0067/0068 already appear there.

## Data model and contracts

`WebhookTarget` keeps four fields. Raw optional CA input and projected optional
CA output are separate values, not new stored types; None versus Some(empty)
remains observable. Preserve DER/order/duplicates. Jobs own snapshots/PEM and
return results, without installation, publication, send or terminal-ledger
authority. Delivery jobs alone count successful builds. Keep
`CaCache { clients, lru }`, capacity 32 and existing atomics.

GET snapshots then awaits projection. PUT preserves closed-body,
null/type/media-size/guard refusals, validates every target and prepares every
echo before one swap; success echoes that list even if a later writer wins.
Worker errors remain distinct native fixed GET/PUT 500s; invalid CA remains
named value-free 400. No overload status, admission limit, network preflight,
submission-order or diagnostic round-trip guarantee.

## Implementation approach

1. Make privacy failures safe first: fixed case/field identifiers and boolean
   comparisons, including status/decoding/raw-state paths. Generated disposable
   keys only; no response/fixture dumps in receipts.
2. Extend existing wire fixtures with repeated/noncanonically wrapped roots,
   unrelated blocks and prose. Assert exact canonical bytes and DER sequence,
   private-sentinel absence throughout decoded responses, shape/redaction and
   privately compared raw installation. Retain direct-set + GET and synchronous
   empty/null/whole-bundle cases.
3. Hold real GET and PUT at builder entry, separately holding PUT validation
   and projection. Observe old configuration and runtime/SMTP/cache progress,
   release then check exactly one swap and echo. Use only necessary exercised
   owner-local observations for swap count/cancellation. Retain helper tests
   as supplementary evidence.
4. Serialize cache/hook harness tests with existing CACHE_TESTS discipline;
   never park an async worker on a contended synchronous test lock. Seed known
   state/order, drive actual GET/PUT, assert neutral counters/entries/recency,
   then deliver a fresh key to prove the same next eviction.
5. Add `protocol_ca_cache_publication_and_accounting` in the existing harness.
   Two actual deliveries sharing one raw PEM enter held cold builds before
   publication. Release deterministically; await real TLS and quiescence;
   scrape `/metrics`: +2 builds, one key, zero evictions. Fill 32 keys, touch a
   known entry, hold two distinct misses and release in known order: +2/+2 and
   expected victims. Deliver a hit (+0/+0) and rebuild a victim (+1/+1).
   Include equivalent certificates with different raw text as distinct keys.
   Direct helpers may establish setup and inspect winner/bijection invariants;
   claimed delivery and counters must cross real protocols.
6. Pin exact fixed errors, retry/one terminal result and failed-build
   non-counting. After cancelled/stop-held builds, observe completed successful
   counter state as well as builder-finished signals before no-late-effects
   assertions. Preserve Store panic and shutdown order; no builder drain.
7. Preserve one-worker/four-blocking-slot held-A progress and >=33-root/eight-
   SMTP-session fan-out. Record actual capacity/workload and fixed deadlines.
   Independent-thread watchdog releases cleanup even if a worker blocks; using
   it means failure. Preserve finite GET/delivery pool coupling/recovery as
   #220 evidence, without unbounded latency or starvation claims.
8. Align living descriptions, collect sanitized focused negative/restored
   results, restore sources, then run targeted tests and all gates. Only a
   demonstrated production defect warrants a minimal repair plus its proof.

## Clause-to-assertion and command matrix

**W(name)** = `cargo test --locked --test webhook name`;
**H(name)** = `cargo test --locked --lib webhook::ca_construction_tests::name`;
**U(name)** = `cargo test --locked --lib webhook::validation_tests::name`;
**A(name)** = `cargo test --locked --test api name`.
Existing tests require fresh execution. Tests marked new remain implementation
work. Check selected test counts: zero executed tests is not a passing receipt.

| Authority / AC | Assertion and focused command | Work remaining |
|---|---|---|
| 0067 §1 / AC1 | W(private_keys_in_ca_pem_never_appear_in_get_or_put_responses): canonical bytes/DER/order/duplicates, private/unrelated/prose absence, four fields/nulls, redaction and unchanged raw state | Implemented: fixed diagnostics, canonical CRLF/37-column input, ordered duplicate DER, all unrelated/private/prose sentinels, raw-state comparisons. |
| 0067 §1 / AC2 | W(direct_webhook_install_projects_unloadable_ca_as_empty_on_get), U(a_redacted_target_keeps_only_validated_public_ca_certificates): whole-bundle empty output on any parse/load failure, no prefix/raw fallback, absent null | Implemented: eight direct-input cases run synchronously and over GET; absent/null is covered by the paired successful PUT/GET. |
| 0067 §2; 0068 §2 / AC2,9 | W(garbage_ca_pem_is_refused_at_put_and_nothing_is_applied), W(webhook_second_ca_root_delivers_over_verified_https), W(webhook_validation_does_not_dial), full webhook suite: complete-list refusal, mixed admission, all-root trust, offline work, auth/secret/proxy/redirect/body/budgets | Reuse existing proofs; raw identity also covered in AC4. |
| 0067 §2 / AC3 | H(webhook_api_preparation_is_off_worker_and_swaps_once): actual GET/PUT validation/projection holds, progress before release, one swap/echo; H(cancelled_put_handler_cannot_install_after_preparation): actual owner cancellation before release, no late install | Implemented: actual GET/PUT holds and swap count; an instance-local Abortable seam drops the real set_webhooks future and acknowledges cancellation before release. |
| 0067 §§2–3 / AC3,4 | H(api_preparation_preserves_lru_order_and_next_eviction): real GET/PUT neutral metrics/entries/recency and known next eviction | Implemented: real GET/PUT, unchanged LRU and counters, then SMTP/TLS delivery evicts the expected oldest root. |
| 0067 §3 / AC4 | H(protocol_ca_cache_publication_and_accounting): exact SMTP/TLS/metrics deltas/raw identity; H(concurrent_cache_misses_recheck_publication_and_preserve_lru_bijection): winner reuse/touch/bijection/cap; full inline webhook suite | Implemented: exact +2/0 duplicate, +0/0 touch/hit, +2/+2 distinct and +1/+1 rebuild phases; known victims and raw-text identities. Lock/drop ownership checked in client_for. |
| 0067 §4; 0068 §1 / AC5,7 | H(api_preparation_join_failures_are_value_free_and_leave_put_untouched): live exact GET/PUT 500 and unchanged list/cache; H(builder_panic_is_value_free_and_does_not_poison_cache), H(delivery_builder_join_failure_retries_to_one_successful_outcome): fixed attempt error/retry success/no failed-build count | Implemented: exact native API errors; injected delivery join failure retries over SMTP/TLS; invalid unchecked input exhausts once without cache work. Existing four-attempt wire proofs remain. |
| 0067 §§3–4 / AC5 | H(cancelled_delivery_build_counts_work_without_late_publication), H(shutdown_during_held_build_aborts_once_without_late_publish_or_send): successful CPU work can count, no late publish/send/terminal change, one abort | Existing tests await the allowed successful-build increment after cancellation, with real shutdown/ledger/no-send assertions; helper cancellation supplements them. |
| 0067 §5 / AC6 | H(smtp_acceptance_and_queryability_continue_during_held_distinct_ca_build), H(cold_client_construction_leaves_the_async_worker_and_cache_mutex_free): ticker/hit/B/HTTP/SMTP DATA/query before A release; W(deliveries_through_more_pems_than_the_cache_cap_all_succeed): >=33 roots/concurrent SMTP/verified TLS | Held-A proof: one async worker/four blocking slots, five-second deadlines/eight-second cleanup watchdog. Fan-out: one async worker/64 blocking slots, 33 roots/eight SMTP sessions, 30-second delivery deadline. |
| 0067 §5; 0068 §3 / AC7 | H(exhausted_blocking_pool_queues_smtp_parse_until_delivery_build_releases), H(get_projection_exhausts_blocking_pool_until_release), H(concurrent_get_projection_capacity_controls_smtp_progress), H(cancelled_get_preparation_keeps_its_blocking_slot_until_finish) | Preserve finite coupling/recovery/surviving-work evidence; residual tracked by #220. |
| 0068 §1 / AC7,8 | A(openapi_webhook_get_documents_awaited_preparation_failure) and live API failure proof: served 200/403/500/awaited description/exact GET text/distinct PUT | Retain sanitized GET-500-removal negative/restored proof. |
| Preservation / AC9 | Full webhook inline/harness/integration suites, `cargo test --locked --test binary`, `cargo test --locked --test p2`, full locked workspace tests | Preserve shutdown/Store panic, 250/queryability, watcher/pruning/extraction and compatibility. |
| Evidence / AC8–10 | Focused privacy/placement/served-contract negative controls then restored positives; all house/pinned gates; current protected five-role review | Six negative/restored-positive receipts are in .forge/results/implement-195-completion/negative-controls.json; final local gates below. Protected review remains downstream; no author clearance. |

## Quickstart and verification

Authenticate the packet and engine; run focused suites:

```bash
(cd .forge/epic192/issue195 && sha256sum -c SHA256SUMS)
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
export CARGO_NET_OFFLINE=true
cargo test --locked --test webhook
cargo test --locked --test api openapi_webhook_get_documents_awaited_preparation_failure
cargo test --locked --lib webhook::ca_construction_tests::
cargo test --locked --lib webhook::tests::
cargo test --locked --lib webhook::validation_tests::a_redacted_target_keeps_only_validated_public_ca_certificates
```

Run every command in the authoritative
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
using `.forge/bin/brokkr compile --bundle bundles/self` for compilation.
Do not substitute an installed engine or change the pin. Include exact zero
misses for lines/branches/functions, coverage selftest, publish/seal proofs,
licenses and fail-closed locked native-tls absence. Check public shims against
`bundles/self/scripts/shims.sha256`; run pinned design-check selftest and
delivery-proof-test. The hermetic verifier executes its mounted pinned copies,
installs nothing, and binds evidence to the committed candidate. Chief local
checks do not replace downstream verification or protected review.

Record commands, source/commit identity, status and sanitized receipts in ignored
run evidence. Restore negative controls before final gates/implementation
commit. Keep this design HEAD limited to its three files, unsigned; run
`bash bundles/self/scripts/design-check.sh check .`. The sequence owns the
final design verdict; this seat writes only `drafted`. CI owns chart/history/
selftest, Docker, RustSec/network, tag rate and privileged port-80 proofs;
operator owns sealing/live settings/publication/acceptance. No additional
external prerequisite is imposed here.

## Risks

| Risk | Mitigation and accepted remainder |
|---|---|
| Private regression failure logs | Fixed identifiers/boolean comparisons and safe HTTP/JSON helper errors before negative tests; no fixture dumps. |
| Labels/partial roots bypass loadability | Exact wire and synchronous fail-closed cases using shared builder; covert data inside loadable certificates stays outside policy. |
| Helpers conceal publication errors | Actual GET/PUT holds and observed owning-handler cancellation, one synchronous swap, no new lifecycle. |
| Global counters/hooks race | Existing serialized cache harness, event gates, quiescence and counter completion; process isolation if otherwise impossible. |
| Held build stalls sole worker | Independent-thread deadline/release and joined cleanup; watchdog use is failure. |
| Builder notification precedes counter increment | Observe completed jobs/counter state before exact deltas and no-late-effects claims. |
| Duplicate/abandoned CPU work | Honest accounting, short locks and owner-only publication; finite work can survive cancellation. |
| Shared pool delays SMTP | Preserve finite evidence and #220 ownership; no isolation/admission implementation or unlimited-load guarantee. |
| Canonical output surprises clients | Diagnostic-only documentation; raw installed identity intact; invalid direct input can hide otherwise good roots. |
| Stale proposal prose reopens settled authority | Accepted headers and authenticated ruling govern; update living descriptions and retain historical records. |
| Green gates mistaken for clearance | Current accepted-0058 protected review assesses disclosure fix; no old-panel substitution or author safety facts. |

## Historical chief validation evidence — 2026-10-24, earlier attempt

The following dated entry is retained from the prior plan, not a result of this
completion design or a present prerequisite. Its 0062/index and unavailable
engine circumstances are superseded by accepted 0067/0068 and the supplied
candidate-local engine. Fresh results must be recorded separately.


Artifact structure, relative links and required sections pass local checks.
Formatting, locked all-target clippy, coverage selftest, publish-guard proof,
seal-branch proof, license check, fail-closed native-tls absence, shim integrity
and design-check selftest passed. Receipts are run-local under
`.brokkr/state/design/chief-gates/`.

The full locked test rerun failed two `tests/store_contract_docs.rs` assertions:
0062 has no decision-index row. The explicit four-file design-commit scope
excludes `docs/decisions/README.md`; T9 requires that index row during
implementation. This is a real outstanding gate failure, not a waiver or a
passing baseline. Do not call all gates clean. The initial full test and
coverage attempts were limited to 90 seconds and timed out; the full test
rerun supersedes its timeout with the concrete failure above. Coverage remains
unverified and must rerun after the index repair rather than repeat the known
failing suite here. Bundle compilation is also unverified: `brokkr` is absent
from this workspace tool's PATH; no replacement engine was installed or used.
The design result is only `drafted`, with full verification still required.

## Implementation proof and handoff

The runtime mechanisms inherited by this candidate satisfy the operative
0067 contract; this change adds missing discriminating protocol proofs and
updates living authority/placement wording. Production admission, projection,
cache/publication, retry and shutdown semantics are preserved. New observations
are cfg(test)-only: an installation count on each Webhooks instance, and an
instance-local cancellation wrapper around the unchanged actual PUT handler.
The latter lives in the declared 0020 harness `src/api/webhook_handler_tests.rs`
and acknowledges only after dropping the publication-owning future.

The protocol accounting harness clears retained fixture clients outside the
mutex while preserving cumulative counters, then quiesces actual deliveries
before scraping exact metrics. Two same-key misses build twice/publish once; after the first delivery returns,
the losing waiter must reuse the winner's idle TLS connection for request two.
A separate losing client cannot satisfy that real-wire proof;
full-cache distinct misses evict exactly their two known victims; real hits
only touch recency; an evicted equivalent raw PEM rebuilds and evicts once.
GET/PUT preparation does not touch cache state or influence the next real TLS
delivery's eviction. Code inspection confirms construction/await have no cache
guard, and losing/evicted clients drop after unlocking.

Privacy checks use generated disposable key DER with PKCS#8, RSA, EC and
encrypted labels, unrelated PEM/prose and noncanonical repeated roots. These
labels test unrelated-section handling; they do not claim four independently
encoded key formats. All comparisons of key-bearing values are boolean and
HTTP/JSON error helpers print fixed diagnostics. Eight invalid direct-install
cases prove empty projection synchronously and on GET, including a valid root
followed by invalid or private DER. Supplied mixed input remains byte-exact in
installed state. Existing complete-list refusal, later-root verified HTTPS,
Basic auth/secret, proxy/redirect, shared bytes and four-attempt wire proofs
remain part of the full suite.

Private mutation receipts retain only commands, source/output hashes, exit
codes and fixed diagnostics; subprocess output is never written to receipts.
Raw echo, label-only projection, on-worker construction, construction under
the cache mutex, returning a private losing client and GET-500 removal each failed its focused test (one failed,
exit 101), then passed after byte-exact restoration (one passed, exit 0).
No PEM marker appeared in captured failure output. Receipt paths are ignored
run evidence under `.forge/results/implement-195-completion/`.

Local gate results and candidate identity are recorded there by the implementer.
The engine must still perform hermetic verification and protected review on the
committed candidate. Review scope includes the **key-disclosure fix itself**,
including inherited runtime code, rather than treating a proof-focused diff as
security clearance. Retain accepted-0058 full-risk routing: separate Astra
correctness, official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees
and independent Fable 5.1. No reviewer or engine result is authored by this seat.
The low-severity shared-blocking-pool availability residual remains tracked by
#220; 0069 stays proposed and unimplemented. Existing finite coupling/recovery
and surviving-work tests are evidence of that residual, not a latency guarantee.
CI and operator ownership remain as listed in the charter; no external proof
or decision acceptance is added to this implementation.
