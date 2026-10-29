# Intake — Epic #192 / issue #220

Run: epic-192-issue-220-give-server-m-b73df672
Phase: intake; disposition: resolved; date: 2026-10-25.
Inspected base: 386acf6, following #195 at ca76e24. No implementation in this phase.

## Goal

Implement accepted 0069 so all server-managed webhook CA preparation uses its own bounded, nonblocking admission and dedicated execution capacity, while real SMTP parsing, durable DATA commit and immediate queryability progress with every CA worker held.

## Authority and scope

The evidence README and operator-rulings were read first and both passed their directory-local SHA256SUMS. The README has no prior-run entries. The operator's 2026-10-25 acceptance, committed at 386acf6 and indexed in docs/decisions/README.md, supersedes the older proposal/stopping language in 0069 and the evidence README. 0067/0068 are also accepted. Do not re-accept, downgrade or re-litigate these rulings. The old security residual remains for the authorized security reviewer to resolve against the implementation; intake resolution supplies no security clearance.

0069 narrowly amends 0041 §2 and 0048 §§1–3. Before dependent code, design must settle its numeric W/B defaults (J = W), byte formula, oversized-job posture, API/delivery fairness and worker ownership/panic/shutdown choices against accepted 0067/0068. Explicitly resolve the 0041 §5 relationship: preserve dispatcher stop → serve join → drained → dispatcher join, and 0067's no-builder-drain behavior. A choice that changes stop completion or introduces a drain needs an explicit accepted amendment before contradictory code; 0069 does not silently authorize that amendment. Record compatible lifecycle selection and its costs in the design artifacts. Do not infer a worker lifecycle merely from the decision's accepted header.

Reuse the complete offline CA builder, public projection and single delivery cache/publication owner. No generic service framework, second cache, public tuning surface, authentication rollout, SMTP pressure refusal, Store/SMTP executor migration or broader memory/latency guarantee is in scope. No dependency unless design supplies a defensible reason and permissive-license proof; never silently widen the allowlist. Preserve accepted admission/privacy contracts and the #195 LOW repairs, including keyed API panic seams and sanitized private-DER assertions. Use bundles/ routing unchanged and retain protected review/security hold; 0069 calls for the existing five separate correctness/security/tests/guarantees/independent offices. Intake does not launch another phase or its panel.

## Local spot-check

- src/webhook.rs: redacted_targets and prepare_targets use shared spawn_blocking; public_ca_pem/build_ca_client own complete safe projection and validation. client_for clones the delivery key before build_delivery_client, which also uses spawn_blocking. Admission must precede the extra job-owned clones/snapshots.
- src/api.rs: get_webhooks currently clones current configuration before redacted_targets; set_webhooks prepares before its single set. Both retain fixed 500 paths; overload needs a distinct typed failure and served 503 contract.
- src/webhook/ca_construction_tests.rs owns actual builder holds, GET capacity/cancellation, duplicate/cache races and real SMTP/HTTP proofs. Its existing exhaustion tests establish the old coupling and must be updated into causal isolation proofs rather than removed without replacement.
- src/store.rs retains awaited whole-mutation offload and panic re-raise; src/smtp.rs awaits insert_async before DATA acceptance. Existing tests/webhook.rs, tests/p2.rs, tests/burst.rs, tests/persistence.rs, tests/lifecycle.rs and tests/rate.rs are protocol/regression owners.
- Read the house charter, decision index, 0001–0003, 0023/0030, 0041/0048 and 0067–0069; inspected local feature history. No GitHub, CI, journal or sibling-worktree evidence is required for framing.
- .forge/bin/brokkr matches ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f and bundles/self/tooling.json.

## Frozen guarantees

Directly adjacent to 0001: DATA 250 follows parsing and the complete successful persistent mutation, and means immediately queryable mail. Preserve 0023/0030 awaited offload and Store panic symmetry. 0002 subscribe-before-check and 0003 oldest-only pruning remain mandatory regression invariants; this task authorizes no change to them or 0004/0005.

## Checkable acceptance criteria

1. Design records finite numeric W >= 1 and B > 0 with J = W, rationale, overflow-safe byte formula, oversized-snapshot behavior, fairness/refusal cost and exact lifecycle selection. Tests exercise boundary and overflow cases for that formula. No dependent executor code precedes resolution of 0069's §5/lifecycle relationship with 0041 §5 and accepted 0067/0068; any incompatible lifecycle amendment waits for operator acceptance.
2. All server-managed GET projection, PUT entire-list validation/projection and delivery CA misses execute outside SMTP/Store's Tokio blocking pool. Admission atomically reserves an idle worker and all job/byte capacity before extra snapshots, keys or copies. Saturation, byte overflow/over-budget and closed intake refuse immediately. At every observed transition jobs <= J, running <= W and charged bytes <= B; no pending queue, waiter list, shared-pool permit waiter or uncharged handoff exists. Completed results retain reservations until handoff/drop. Include mixed API/delivery and configuration-churn cases, not only repeated GET.
3. Accounting tests cover every job-owned string/snapshot/copy, delivery keys, validation and projection inputs, retained outputs, temporary DER/roots/clients and ownership transfer/drop. Before-entry cancellation drops input exactly once; entry races transfer the reservation; running cancellation never releases it early. Explicitly document that B is a charged job budget, not allocator-wide memory: installed configuration, HTTP extraction, transport and synchronous embedder allocation remain outside it. Preserve the body limit and disclose permanent resource refusal of oversized valid snapshots if selected; do not truncate or narrow valid PEM admission.
4. Real HTTP GET and PUT under exhausted/closed CA admission return exactly native-envelope 503 with fixed `webhook CA preparation unavailable`, with no supplied values or worker/queue details. Existing guards/extractors still win their 403/400/413/415 cases. Admitted execution failures retain GET `webhook response preparation failed` and PUT `webhook configuration preparation failed` as fixed 500s. Failed/refused/cancelled PUT leaves installed bytes unchanged; success validates all targets and prepares the safe full echo before exactly one swap. Served OpenAPI documents each route's 503 separately and retains 500; wire tests fail if either overload status is omitted or misclassified.
5. Failed delivery CA admission consumes an attempt with fixed value-free `webhook CA preparation unavailable`; existing retry count/backoff and per-sink/shared-payload behavior persist and each admitted delivery ends exactly once as delivered/exhausted/aborted. Tests prove pressure recovery and exhaustion, final totals and no SMTP refusal/defer caused by webhook work. Document reduced delivery opportunities and the chosen API/delivery fairness policy without promising starvation freedom.
6. Only a live awaiting owner installs PUT configuration, publishes a cache winner or sends delivery. Controlled cancellation on both sides of worker entry and after completion proves no late publication/send, no early capacity reuse and eventual reclamation for finite work. Duplicate builds recheck under the same cache mutex; 32-entry LRU invariants, actual-build counts (including successful losers/abandoned work), actual evictions and outside-guard loser drops survive. GET/PUT stay offline and delivery-cache-neutral, including at full cache.
7. Lifecycle tests close intake, dispose admitted handoffs, reclaim retained results and finite abandoned work, and demonstrate the selected thread-join/ownership behavior for server and embedding use. Worker panic yields value-free failure and leaves capacity usable. No unrecorded drain, hard-cancellation promise or assertion that aborting delivery tasks drains workers. Keep sweep-through-Lagged to Empty/Closed, stop ordering, Store mutation panic re-raise and exactly-one terminal accounting.
8. Deterministically hold every dedicated CA worker through actual HTTP GET preparation, with a constrained shared Tokio blocking pool. Before any explicit CA release, real SMTP must accept a session, parse DATA, persistently commit, answer final 250 and expose exactly that message immediately via real HTTP query. Check persistent recovery as appropriate to prove the durable leg. Excess CA requests refuse while holds remain. Cancellation of a held GET must not make its worker/charge reusable early and must not prevent the same SMTP progression. Assert holds still active and watchdog unused at each success; use explicit synchronization, never sleeps as synchronization or watchdog release as success. Retain old causal coupling evidence; candidate tests must fail if CA work is restored to the shared pool, rather than succeeding only after release.
9. Complete-root validation, later-root verified TLS, >32-root real-wire fan-out, mixed/unrelated PEM admission, safe canonical public certificates with no raw/private/partial fallback, secret/userinfo redaction, direct/no-proxy/no-redirect policy and existing API schemas/nulls remain proven. Synchronous embedder validate/redacted remain available outside server-only resource claims. Preserve frozen guarantees through existing protocol suites. Apply 0020: measured inline tests by default; crate-private cold-path interleavings belong in declared cfg(test) sibling harnesses; public-only protocol races belong in tests/. New production lines remain fully measured; no coverage exclusion or denominator reduction.
10. The unsigned committed implementation candidate passes every house gate and pinned proof below, deterministic design validation and the existing hermetic verify/protected review route. Only the engine and authorized reviewers provide their phase/review facts. No push, merge, tag, publication, issue closure or decision acceptance occurs in this run.

## Verify commands and ownership

Run offline using cached tools, install nothing, retain commands/exits and bind receipts to candidate HEAD. Focused tests supplement the full suite. Update the construction harness's expectations for bounded isolation. Design must name the new admission/isolation tests explicitly in tasks; the whole owning harness runs here so no new test can silently fall outside a guessed name filter. No cargo bench is required: the claim is causal isolation and finite accounting, not throughput or a latency threshold.

```bash
export CARGO_NET_OFFLINE=true
(cd .forge/epic192/issue220 && sha256sum --check --strict SHA256SUMS)
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo test --workspace --locked --lib webhook::ca_construction_tests
cargo test --workspace --locked --test webhook
cargo test --workspace --locked --test p2
cargo test --workspace --locked --test burst
cargo test --workspace --locked --test persistence
cargo test --workspace --locked --test lifecycle
cargo test --workspace --locked --test rate
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash -c 'out=$(cargo tree --locked -i native-tls 2>&1); code=$?; if [ "$code" -ne 0 ] && printf "%s" "$out" | rg -q "did not match any packages"; then exit 0; else printf "%s\n" "$out"; exit 1; fi'
```

The charter remains the single inventory. The paths above map its public dispatch commands to pinned proof owners: verify integrity-checks the public shims and executes pinned copies against the candidate (0050), never candidate dispatch shims. Bundle compile warnings that sweep protected facts refuse even with exit zero. Coverage requires exact zero missed lines, branches and functions and passing selftest. Structural design validation remains the engine's own later verdict; its selftest alone does not validate a future design.

CI owns macOS, Helm/chart-bump/history/selftests, Docker persistence/context smoke, network RustSec and privileged real port-80 proof. Tag-only rate/publication proofs remain CI/release work. Operator owns signing/sealing and live settings. None is an invented local intake acceptance blocker; no network or namespace proof is required here. Dedicated execution establishes blocking-pool isolation, not OS scheduling/memory isolation, fair admission, default-runtime latency, durable webhook delivery or bounded whole-server memory.

## Intake receipt

Framing only: no runtime implementation or feature proof is claimed. Evidence and engine hashes verified; source/history spot-checked within the intake exploration budget. The intake gate-run receipt is retained separately under .forge/epic192/issue220/intake-gates/ and does not waive final implementation verification. No workspace input is missing for framing. Lifecycle and sizing choices are required design outputs, not grounds to re-litigate accepted 0069.
