# Plan — webhook CA port and LOW repairs

Implements [spec.md](spec.md), following [intake.md](intake.md) and proposed
[0077](../../docs/decisions/0077-webhook-ca-port.md). Design only.

## Technical context

One Rust 2024 crate; existing Tokio, axum, reqwest/rustls, rustls-pemfile,
base64, serde and rcgen suffice. Keep manifests, lockfile, allowlist, recipes,
pins and gates unchanged. No new service, production module, integration suite
or persistent model. `.specify/templates/plan-template.md` is absent; required
technical context, constitution, structure, research, data model, contracts and
quickstart are inline here.

## Constitution check

Preserve 0001–0003/0005 and Store mutation ordering/panic symmetry. SMTP port
changes are cfg(test)-only observers. Use 0067/0068 as accepted authority;
preserve 0069's supplied history and assign implementation to #220 under the
later ruling. 0077 is proposed and compatible, not a new acceptance prerequisite
or permission to contradict a ruling. Frozen guarantees cannot be weakened.

Use real protocols, fresh inboxes and generated TLS fixtures. Branch-stable
tests stay inline and measured; only concrete cold watchdog/failure paths
justify cfg(test)-declared sibling harnesses under 0020. Every seam is test-only
and exercised. No sleep-as-sync, mid-band chaos, exclusions or whole-suite lock.
Commit unsigned, never push. The design commit contains exactly spec/plan/tasks
in this directory plus proposed 0077; intake and run-local evidence stay as is.
Implementation restores historical artifacts later. The final design validator
selects this commit's one slug, never an artifact's mtime.

## Research and resolved choices

Current GET parses ViewQuery before config access; keep that order and main's
OpenAPI 400 when merging offload. Current PUT validates synchronously then reads
configuration again for its echo; port an owned `(raw_targets, prepared_echo)`
result to remove that response race. Current client_for builds under the mutex
and redacted copies raw PEM. The local packet supplies their fixes.

Certificate::from_der alone does not establish root-store loadability. Rebuild
all parsed DER, then validate with the uncached delivery builder; any failure
suppresses the whole supplied projection. Preserve raw admission/identity.
Store offload rethrows panics; use the port's webhook-local fixed-error join
boundaries instead. The 32-entry LRU bounds retained clients only. Actual build
accounting belongs inside successful delivery construction; publication and
sending belong to the waiter. API work affects neither cache metrics nor LRU.

## Patch disposition ledger

Numbers refer to authenticated `patches-0058/`. Port final semantics rather
than stale snapshots or obsolete decision names. Record source/proof disposition
for every row. Historical checkmarks and passes never become current passes.

| Patch | Disposition and current owner |
| --- | --- |
| 01 | Preserve original safety spec history; final contract comes through 0067 and this design, not occupied 0062. |
| 02 | Port projection, API/cold-build offload, cache publication, harness, wire tests and docs into current owners; reconcile main API/query/SMTP/schema changes. |
| 03 | Retain recovery spec as history; provenance obligations become current AC9/10, not proof of this tree. |
| 04 | Retain recovery corrections; bind new evidence to this candidate and exact engine. |
| 05 | Restore final GET contract as 0068; retain pool-review history with current-scope pointer. |
| 06 | Port GET-500 schema/test and pool coupling/cancellation proofs, preserving main's 400 and other schemas. |
| 07 | Retain bounded-CA proposal history only; no executor or obsolete #195 stop requirement. |
| 08 | Use final 0067/0068/0069 names/links; preserve unrelated current 0062/0064 and all index rows. |
| 09 | Retain 0069 refinement/history; later ruling assigns accepted implementation to #220. |
| 10 | Restore supplied accepted 0067/0068 bytes and historical 0069 record; additive index merge cites later ruling. Do not re-accept or edit accepted bodies. |
| 11 | Retain completion intake as dated provenance linked to this current intake/design. |
| 12 | Retain final safety contract/proofs; add dated supersession pointers for stale stop/gate instructions in recovered specs/plans/tasks. Current obligations live here. |
| 13 | Port comprehensive projection, actual-handler cancellation, cache/publication and scheduling proofs plus docs. |
| 14 | Port cfg(test) harness declarations and explicit guard scopes; preserve test placement. |
| 15 | Port duplicate-waiter winner TLS-connection reuse proof and its explanation. |
| 16 | Port boolean private-byte comparisons; extend the still-unsafe private-DER refusal path under AC6. |
| 17 | Port complete raw prior-state assertions for API failures, then isolate injection under AC8. |

Recovered spec files may carry a dated leading notice that this design governs
the current port; preserve earlier dated clarifications below it. Restore old
safety/recovery/pool families only as supplied history, not competing active
requirements. Index 0077 as proposed during implementation.

## Project structure and files

| Path | Planned implementation |
| --- | --- |
| `src/webhook.rs` | Whole-bundle projection, preparation helpers, delivery blocking boundary, async acquisition, short guards/outside-lock disposal, keyed panic seam, inline proofs. |
| `src/api.rs` | Preserve query/body admission; GET snapshot/PUT owned preparation, fixed errors, single swap, test-only actual-handler cancellation hook/declaration. |
| `src/webhook/ca_construction_tests.rs` | Restore cold-path scheduling/cancellation/protocol-accounting harness; add keyed-seam interleavings here only where cold watchdogs require it. |
| `src/api/webhook_handler_tests.rs` | Restore actual request-owner cancellation proof scoped to Webhooks instance. |
| `src/smtp.rs` | Restore only cfg(test) DATA-submission observer; preserve main production parsing/commit behavior. |
| `tests/webhook.rs` | Port HTTP/SMTP/TLS privacy, invalid-root, >32-root and admission proofs; fix private-DER diagnostics/sentinel. |
| `tests/common/mod.rs` | Remove non-JSON body/parser dump; retain explicit failure, safe method/path/status, no-content semantics and matching comment. |
| `tests/api.rs` | Restore served GET-500 proof preserving main assertions; focused documentation/non-JSON regressions where compatible with existing helpers. |
| `README.md`, `src/static/llms.txt`, `src/static/openapi.json` | Port CA/error contract, qualify SMTP/#220 limits, preserve current query/schema/metrics/chaos text. |
| `docs/decisions/0067-webhook-ca-safety.md`, `docs/decisions/0068-webhook-get-failure.md`, `docs/decisions/0069-bounded-ca-execution.md`, `docs/decisions/README.md` | Restore final supplied records; additive index/current-ruling citation; index 0077 as proposed. |
| `specs/webhook-ca-safety/`, `specs/webhook-ca-recovery/`, `specs/webhook-pool-review/` | Restore supplied history, final links and dated current-design pointers without obsolete stop requirements. |
| `specs/webhook-ca-port-lows/{spec,plan,tasks}.md` | Own current requirements, corrections and evidence; keep intake unchanged. |

No production Store/extraction/POP3, gate or routing change. Existing burst/p2/
rate, strict-query, metrics and chaos tests remain proofs without needing edits.

## Data model and ownership

Retain WebhookTarget's four fields. Installed raw targets remain separate from
diagnostics; None and Some(empty) differ. GET owns a cloned snapshot. PUT's job
owns the list and returns it with its safe echo; it has no installation handle.
After await, the handler swaps once and constructs the response with no await.
Concurrent GET may return its earlier snapshot.

Retain map/deque cache and counters. A delivery job owns PEM and returns a
client/error; it counts successful construction but cannot publish, install,
send or change terminal totals. The awaiting owner rechecks under the mutex,
uses a winner or inserts/selects eviction, then drops losing/evicted clients
outside guard scope. Preserve cancellation, retry and terminal ownership.

Use cfg(test) Mutex<Option<String>> keyed by fixture PEM, following
CA_BUILD_PANIC. Check the owned target slice: mismatch leaves state intact;
match takes it, releases the lock, then panics with fixed text. Existing
cache-test serialization may protect installation; unrelated requests need no
such lock. Owner cleanup compares its key before clearing. Serialize only
exact global-counter proofs, never the whole suite to conceal interference.

## Contracts and implementation sequence

1. Inventory final semantics/history and restore records at final names.
   Reconcile current owners; compile focused suites.
2. Port projection and API ownership as one privacy boundary: sequential job,
   pure preparation, one swap, fixed failures and retained query precedence.
3. Port unlocked delivery construction/recheck publication; restore actual-owner
   counter/winner/cancellation/terminal proofs.
4. Fix LOW diagnostics at unsafe call sites/common helper. Use non-header
   sentinel, boolean comparisons and fixed messages; never format raw targets
   or responses. Exercise every named failure class with in-memory capture
   before any output is written. Emit only fixed verdicts/statuses/hashes, even
   on control failure. Preserve loud malformed-JSON failure and 204/304 absence.
5. Key injection; add unrelated-B-before-matching-A GET/PUT sequences and
   one-shot/cleanup checks using channels/barriers and watchdog deadlines.
6. Align README/module/llms wording and add old-claim regression; distinguish
   network/backoff duration from queue/build time.
7. Commit, bind fresh controls/full gates to the candidate, then hand complete
   scope to unchanged protected review.

## Proof map and quickstart

Retain existing patch test names where practical; new names below are proposed.

| Criteria | Discriminating proof |
| --- | --- |
| AC1 | Strict-query, decoded-inbox metrics and object-only chaos/seed tests; 17-row disposition and diff audit. |
| AC2 | `private_keys_in_ca_pem_never_appear_in_get_or_put_responses`, `direct_webhook_install_projects_unloadable_ca_as_empty_on_get`, inline validated-public-CA test and later-root TLS tests. |
| AC3 | `webhook_api_preparation_is_off_worker_and_swaps_once`, `cancelled_put_handler_cannot_install_after_preparation`, `api_preparation_preserves_lru_order_and_next_eviction`, live join failures and `openapi_webhook_get_documents_awaited_preparation_failure`. |
| AC4/5 | Held builders, `protocol_ca_cache_publication_and_accounting`, winner reuse, retry/exhaustion/shutdown/cancel, >32-root fan-out, workspace burst/p2/rate/Store tests. |
| AC6 | Repaired invalid-certificate wire test; `webhook_private_failure_diagnostics_are_value_free` and `http_json_rejects_non_json_without_body_diagnostics`. |
| AC7 | `webhook_docs_state_shared_pool_limits` plus old-claim negative/restored-positive control. |
| AC8 | `api_preparation_panic_is_keyed_and_one_shot`: GET/PUT mismatch, match, recovery and guarded cleanup. |
| AC9/10 | Sanitized controls, candidate/source/engine-bound receipts and all charter gates. |

After porting, run focused suites and the complete house inventory:

```bash
cargo test --locked --test webhook
cargo test --locked --test api
cargo test --locked --lib ca_construction_tests
cargo test --locked --lib webhook_handler_tests
cargo test --locked --lib a_redacted_target_keeps_only_validated_public_ca_certificates
sha256sum .forge/bin/brokkr
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
```

Use pinned copies after shim verification. Also run design-check selftest and
delivery-proof-test as required by intake. Use the pinned verify script's
fail-closed `cargo tree --locked -i native-tls` probe: only Cargo's explicit
absent-package error proves absence; success or unrelated error refuses.
Verification is offline and installs nothing. Coverage means zero missed lines,
branches and functions, not rounded 100%. Match spec AC10's full engine digest.

This design phase runs `bash bundles/self/scripts/design-check.sh check .` on
its four committed artifacts; the final sequence check still owns its verdict.
Gate attempts on documentation do not prove future implementation. Record
actual limitations/failures, never waive gates or reuse predecessor receipts.
External CI/operator proofs remain outside local acceptance.

## Risks

| Risk | Mitigation and remaining limit |
| --- | --- |
| Old patches erase main | Merge named owners/additive schemas, keep GET 400 beside 500, run strict-query/metrics/populated-state regressions. |
| Diagnostics disclose generated DER on failure | Safe helper at every fixture-bearing step, fixed messages/boolean comparisons, in-memory capture and no retained body output. Fail loudly without disclosing fixtures. |
| Prefix/label selection mistaken for validation | Complete reconstruction passes delivery builder or projects empty; direct-install and later-invalid regressions. |
| Worker outlives owner and mutates state | No install/publication/send capability in closures; actual-owner cancellation and complete-state tests. CPU completion/build accounting remain possible. |
| Panic seam hits unrelated tests | Unique PEM, mismatch retention, take-before-panic outside lock, owner-key cleanup and explicit GET/PUT interleavings. |
| Schedule-dependent cache counters | Serialize existing exact-counter fixtures only; state shared-global limits of fan-out assertions. |
| Shared pool delays SMTP | Honest docs and spare-capacity proof; low residual tracked by #220, no new global bound or executor here. |
| Historical text implies obsolete authority | Dated pointers/final numbers; cite later ruling, retain old receipts as history and bind new gates to this candidate. |
| Harness changes evade coverage | 0020 cold-path placement, cfg(test) declarations, no exclusions/features, exact coverage and selftest. |
| Winner connection test fragile | Retain generated TLS fixture and observable connection ownership; investigate deterministically, never weaken to build counts alone. |
| Tools/socket restrictions prevent gates | Record exact command/status/limitation, install nothing, claim no pass. Downstream implementation still requires all gates. |
