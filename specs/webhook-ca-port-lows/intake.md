# Intake — port issue #195 and close its three LOW findings

Run: `epic-192-issue-195-port-the-webh-41495018`.
Phase: intake; disposition: resolved (framing only).
Base inspected: `aec4b46` (current main supplied to this run).

## Goal

Port all 17 supplied #195 commits onto current main without regressing its newer behavior, then fix the three LOW non-security findings while preserving accepted webhook CA contracts and recording the existing low shared-blocking-pool availability residual as tracked by #220.

## Evidence and authority

The workspace packet `.forge/epic192/issue195/` is the evidence source.
`(cd .forge/epic192/issue195 && sha256sum -c SHA256SUMS)` passed for README,
operator-rulings, port-and-fix, last-review and every patch. An initial check
from the repository root used the wrong relative directory; it is superseded
by that successful directory-local check. No packet file was changed.
`.forge/bin/brokkr` independently hashes to
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`,
also the digest in `bundles/self/tooling.json`.

Read the house charter, decision index, frozen 0001–0003, 0041, 0048 and
0070; inspected the supplied 0067/0068 decision text and its renumbering and
operator acceptance patches. The 2026-10-25 operator ruling supersedes older
review narratives: 0067/0068 are accepted; 0069 is accepted for #220, whose
implementation is outside this task. Restore the supplied decision records
and index entries without rewriting accepted bodies or accepting anything.
Preserve the patch record of 0069 and identify its later acceptance through
the supplied ruling; historical proposal prose is historical evidence.

The last review's only security residual is low availability coupling on the
shared blocking pool, tracked by #220. Preserve that attribution and truthful
security facts; intake does not clear it or choose a phase transition. The
operator owns the instructed merge disposition despite REVIEW-RESIDUAL-UNSAFE.
No executor, admission cap, overload policy or 0069 implementation belongs here.

## Port surface and bounded inspection

The packet supplies the evidence; intake did not repeat its audit. Inspected
`WebhookTarget::redacted`, `build_ca_client`, `client_for`, API GET/PUT owners,
the response parser in `tests/common/mod.rs`, and the pinned verify commands.
This base has raw `ca_pem` echoes, synchronous API preparation and cold client
construction under the cache mutex, so reviewing only three new fixes would
miss the production disclosure fix being introduced by this port.

Read recent local history for these owners: #197 (`4b0281e`), group A
(`14dee3c`), #202 (`58adf0b`) and #181 (`3ed2fbb`). Do not apply old hunks
blindly or replace current files with the old branch. Reconcile the ordered
17 patches into current owners, including `src/api.rs`, `src/webhook.rs`,
`src/smtp.rs`, `src/static/openapi.json`, `src/static/llms.txt`, README,
`tests/webhook.rs`, `tests/api.rs`, new owner-local harnesses and the supplied
spec/decision artifacts. Preserve unrelated base changes and current bundles,
routing, gate scripts, tooling pin, lockfile and dependency policy.

## Acceptance criteria

1. **Complete port with preserved base contracts.** Every supplied patch's
   behavior/proof/document obligation has a disposition in the design/task
   ledger (ported, reconciled with named current owner, or historical evidence
   retained). No production #195 behavior is silently dropped. Existing #197
   strict method-local query refusals still occur before effects, #202 decoded
   inbox metric labels retain exact escaping/round trips, and #181/0070 seed
   and chaos positional-array refusals still preserve populated state and
   their existing native 400 versus extractor 422 envelopes. Existing API,
   webhook and workspace regressions pass on the reconciled candidate.
2. **Certificate-only public projection, including bypassed admission.** Real
   HTTP GET/PUT with generated certificate-plus-private-key bundles (PKCS#8,
   RSA, EC and encrypted labels), unrelated blocks/prose, noncanonical wrapping
   and duplicate roots returns exactly canonical, loadable certificate DER in
   original order including duplicates. All four response fields retain their
   shape; absent CA and withheld secrets remain null and URL userinfo is absent.
   Neither private body bytes nor unrelated content appear in either response.
   Directly installed malformed/unloadable CA bundles, including relabeled
   private DER and valid-prefix/later-invalid material, project the empty
   string with no raw or partial fallback. Compare complete installed raw
   configuration by boolean/fixed diagnostics: it is unchanged. Mixed PEM
   admission and all-root TLS trust continue under 0048; do not substitute
   private-key refusal for accepted 0067's response projection.
3. **Preparation ownership and failure contract.** GET snapshots and PUT's
   complete-list validation/echo preparation execute off async workers and
   outside config/cache guards. Only the awaiting PUT owner installs, once,
   after success, with no await between swap and response. Real handler
   cancellation cannot install late. Invalid input returns value-free named
   ca_pem 400 before swap; injected join failure gives the fixed native 500
   envelope and preserves the complete prior raw target list. API preparation
   changes no delivery-cache entries, LRU order, builds or evictions. Served
   OpenAPI GET lists 200/403/500 and the exact response-preparation failure
   contract under accepted 0068; its removal makes the contract test fail.
4. **Delivery construction, publication and accounting.** Missing CA clients
   build in spawn_blocking without the cache mutex. The waiter rechecks under
   the mutex, reuses a concurrent winner and publishes only while still owned;
   losers and evicted clients drop after unlocking. Preserve raw-text cache
   identity, 32-entry LRU and exact successful actual-build/actual-eviction
   counts, including losing builds and cancelled waiters. Keep discriminating
   real SMTP/TLS/metrics proofs for duplicates, hits, eviction and rebuild,
   including reuse of the winner's live TLS client. Join errors are value-free
   attempt failures; retry success, exhaustion, cancellation and shutdown keep
   exactly-one terminal accounting, with no late publication/send by cancelled
   waiters. Preserve proxy/redirect refusal, TLS verification, budgets, shared
   payload bytes and shutdown ordering.
5. **Honest SMTP contract.** Held delivery/API preparation with finite spare
   blocking capacity permits real SMTP DATA 250 followed immediately by the
   accepted message's queryability, and the >CA_CLIENT_CAP distinct-root
   fan-out proof remains. README's opening webhook claim uses “never awaited
   by SMTP acceptance”, matching webhook module documentation and llms.txt.
   All three describe the shared-pool dependency and #220 where applicable;
   no unconditional “never blocking [the] SMTP [path]” claim remains on these
   surfaces. A documentation regression assertion fails if the old README
   claim is restored. No zero-latency or global work bound is claimed.
6. **LOW T1/tests-1: private diagnostics are fixed and discriminating.** The
   private-DER negative control in `tests/webhook.rs` uses the fixed-diagnostic
   privacy HTTP helper, an actual private body slice (not the constant DER
   header), and value-free assertions for status, prefix, private-byte absence
   and full installed-state comparisons. It never formats the response into
   failure output. `tests/common/mod.rs` non-JSON failure diagnostics cannot
   dump response bodies or supplied private material; affected documentation
   matches the diagnostic behavior. Demonstrate intended failure paths with
   generated disposable fixtures and sanitized red/restored-green controls;
   private bytes must not enter logs, retained receipts or command output.
   Record only fixed diagnostics, booleans, statuses and source/log digests.
7. **LOW tests-2: preparation panic isolation.** Replace the unkeyed
   API_PREPARATION_PANIC process-global flag with owner/input-scoped test-only
   injection following the existing CA_BUILD_PANIC pattern. A deterministic
   concurrency regression proves unrelated API preparation cannot consume or
   trigger the armed injection, while the intended preparation does trigger
   it and a later request succeeds after cleanup. No sleep-as-sync or global
   test serialization substitutes for a keyed seam. Seams remain cfg(test)
   only, installed by tests, at their owning module; follow 0020 cold-path
   placement and keep every production line measured.
8. **Fresh candidate-bound proof.** Record sanitized defect-specific red and
   restored-green evidence for the ported disclosure fix, the affected
   preparation/cache placement and GET-500 contract, plus the three LOW fixes.
   Recovered receipts remain historical; do not claim predecessor gates prove
   this port. Bind fresh receipts to HEAD/tree, source hashes, command, exit
   status and the exact local engine digest. Full gates below pass after all
   changes are committed; exact coverage has zero missed lines, branches and
   functions, with its selftest passing. No exclusions or lowered floor.
9. **Protected full-scope review.** Use candidate bundles/routing unchanged.
   The protected security-relevant panel must assess the entire ported
   key-disclosure fix and surrounding API/cache ownership, not only the new
   LOW diff. Retain the current risky-work five-office route (correctness,
   official Flash security, tests, guarantees, independent) as provided by
   the recipe; intake dispatches no reviewers. Review records the known low
   shared-pool residual as tracked by #220 and reports any genuinely new
   finding truthfully. Fresh verify and panel results are required; none is
   inferred from the packet or supplied operator merge direction.

## Frozen guarantees and exclusions

Nearest is 0001: webhook work stays downstream of synchronous stored-mail
acceptance; SMTP 250 must still mean immediately queryable mail, with no await
of webhook delivery. 0002 subscribe-before-check and 0003 oldest-only pruning
are unchanged and remain protected by the workspace suites. Preserve ingest
extraction (0005), mutation ordering and Store panic symmetry (0023). SMTP
port changes are test-only observers, not an acceptance scheduling redesign.

Do not read GitHub, CI, journals or sibling worktrees. No new dependency,
allowlist widening, engine re-pin, bundle routing edit, decision acceptance,
push, merge, tag, publication or issue closure. INFO findings are optional;
none is grounds to expand runtime semantics. No missing evidence prerequisite
was found for framing; later missing local inputs must be named precisely.

## Verify gate to run on the committed implementation

Focused commands (retain existing test names when porting; add a named keyed
seam/diagnostic/document regression in the design task list):

```bash
cargo test --locked --test webhook
cargo test --locked --test api
cargo test --locked --lib ca_construction_tests
cargo test --locked --lib webhook_handler_tests
cargo test --locked --lib a_redacted_target_keeps_only_validated_public_ca_certificates
cargo test --locked --test api openapi_webhook_get_documents_awaited_preparation_failure
```

These run real HTTP/SMTP/TLS servers and owner-local deterministic seams;
workspace tests include the burst, p2 and rate suites. No benchmark is needed
for the selected documented shared-pool alternative; spare-capacity behavior
is tested rather than claimed as a latency bound.

Every house gate is mandatory downstream. Verify uses the pinned bundle
copies after checking public-shim integrity (0050), not candidate dispatch
shims. The commands below express that same inventory with the local engine:

```bash
sha256sum .forge/bin/brokkr # must equal ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f
sha256sum --check --strict bundles/self/scripts/shims.sha256
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
```

Require the fail-closed TLS absence proof as well (the pinned verify script
owns the canonical probe; pass only on Cargo's explicit absent-package error):

```bash
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
    exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
    :
else
    exit 1
fi
```

Verification is offline and installs nothing. CI owns macOS and other required
platform runs, Helm/chart history and bump proofs, Docker smoke, network-backed
RustSec and the privileged port-80 bridge exchange. Live GitHub protection,
namespace provisioning, tag-only rate/publication and merge/sealing evidence
remain CI/operator owned; they are not local acceptance claims. The operator
owns merging #195 and #220's later executor implementation.

## Intake completion

Only this framing is delivered now. No source code, decision record, test,
bundle or gate was changed; no implementation gates or reviews are claimed
as executed by intake. Design must reconcile the supplied artifacts before
implementation; downstream verification must execute every listed gate on
its final committed candidate.
