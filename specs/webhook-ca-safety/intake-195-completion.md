> Historical #195 artifact restored on 2026-10-25. The current port and its
> proof obligations are governed by [webhook-ca-port-lows](../webhook-ca-port-lows/spec.md).
> Earlier stops and gate receipts below are dated history. 0067/0068 are
> accepted; 0069 was accepted for #220 on 2026-10-25 and is not implemented here.
> The low shared-blocking-pool availability residual is tracked by #220.

# Issue #195 completion — intake

Run: `epic-192-issue-195-refuse-privat-8e00b7a6`.
Intake baseline: `068b17a` (0058 routing plus earlier #195 work).
Phase outcome: resolved framing; implementation, verification and protected review remain outstanding.

## Goal

Complete issue #195 against accepted decisions 0067 and 0068 by closing gaps in certificate-only public CA projection, off-worker preparation, publication/accounting and their real-protocol regression proofs.

## Authority and boundaries

Read `docs/house-rules.md`, the decision index, accepted 0001–0003, 0041, 0048, 0058, 0067 and 0068, and proposed 0069. Both `.forge/epic192/issue195/README.md` and `operator-rulings.md` pass `sha256sum -c SHA256SUMS` when run in their directory. The candidate-local `.forge/bin/brokkr` matches `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` and `bundles/self/tooling.json`.

The 2026-10-24 operator ruling and accepted status headers govern over historical proposal/stop wording in older specs and decision bodies. 0067 selects certificate-only echoes, preserving mixed-PEM admission; the issue title does not authorize private-key rejection. 0068 records the existing GET preparation-failure 500. No further decision acceptance is authorized.

0069 remains proposed and belongs to #220. The low-severity shared-blocking-pool availability residual is tracked by #220 and does not block #195 on that basis. Preserve evidence of contention and work surviving cancellation; do not claim global resource bounds or clear unrelated findings. No dedicated executor, admission cap, new 503, shutdown drain or alternate lifecycle policy is in scope.

This work sits nearest frozen 0001: webhook work must never become an awaited condition of SMTP acceptance; DATA 250 still means immediately queryable stored mail. Preserve 0002 subscribe-before-scan, 0003 oldest-only pruning, ingest extraction and Store panic symmetry. No production SMTP/store changes are requested.

## Spot-checked implementation and history

`017e709` introduced safe echoes/off-worker builds; `b0c7c28` documented GET 500 and added pool-contention tests; `6260479` preserves the Luna/xhigh recipe assertion; `068b17a` records operator acceptance. The worktree was clean at intake.

Relevant owners are `src/webhook.rs` (`WebhookTarget::redacted`, `public_ca_pem`, `redacted_targets`, `prepare_targets`, `build_ca_client`, `build_delivery_client`, `client_for`), `src/api.rs` (`get_webhooks`, `set_webhooks`), `src/webhook/ca_construction_tests.rs`, `tests/webhook.rs`, `tests/api.rs`, `src/static/openapi.json` and `src/static/llms.txt`. Existing `specs/webhook-ca-safety/`, `webhook-ca-recovery/` and `webhook-pool-review/` contain prior design/evidence; reuse applicable proofs without inheriting obsolete authority stops.

Spot-checks confirm existing key-safe wire/empty-projection tests, builder-site pause seams, duplicate-build/LRU tests, GET 500 served-contract checks and real HTTP/SMTP pool experiments. Presence is not a proof of complete clause coverage. Downstream design must map every operative 0067 clause and 0068 contract to an assertion and command, distinguishing existing coverage from gaps to fix.

## Checkable acceptance criteria

1. **0067 §1: canonical public material.** Real unauthenticated PUT and GET using per-run loadable certificates and real private-key bytes expose exactly the canonical re-encoded certificates, preserving DER, order and duplicates. Include PKCS#8/RSA/EC/encrypted key labels, unrelated blocks and surrounding prose before/between/after certificates. Decoded response strings contain neither key markers nor distinctive private-body/unrelated-block/prose sentinels. Assert four response keys, null absent options/withheld secret and removed URL userinfo. Installed raw configuration is byte-for-byte unchanged. A raw-copy or label-only implementation must fail these assertions.
2. **0067 §1: fail-closed projection.** Direct embedding installation followed by real GET, plus synchronous `redacted`, returns present `""` for empty/no-certificate text, malformed PEM, later invalid certificate/DER and private DER relabeled CERTIFICATE; absent input remains null. No raw or partial-prefix fallback. Prove whole reconstructed-bundle loadability and canonical formatting with noncanonical input, repeated certificates and ordinary certificate-only bundles. PUT invalid input continues to return named value-free 400 without replacing any target; empty projection is not permission to admit invalid PUT data.
3. **0067 §2: admission/preparation.** Preserve mixed-PEM admission, all-root trust and raw cache/budget identity. Real later-root verified HTTPS still delivers; Basic auth, secret delivery, proxy/redirect policy, shared bytes and retry/budget behavior remain green. Hold actual GET and PUT preparation at builder entry and prove async runtime/configuration/cache progress before release. API work is offline and cache-neutral: zero changes to builds/evictions/entries/recency, including next-LRU eviction. All targets validate and safe echo prepares before exactly one async-owner swap; failed/cancelled preparation leaves the installed list unchanged, and success echoes its installed list. Synchronous embedder entry points remain available.
4. **0067 §3: delivery publication and accounting.** Deterministically race same-key duplicate misses and full-cache distinct misses through actual off-worker construction. Recheck/reuse one winner; map/deque stay a bijection with at most 32 entries and correct touches/LRU/rebuilds. Every successful actual delivery construction counts once, including duplicate losers and work completed after waiter cancellation; hits, failed constructions and API temporary builds count zero. Evictions count actual removals only. Prove the required deltas in a real HTTP/SMTP delivery/metrics scenario, using owner-local test seams where necessary; helper-only counter assertions do not discharge the requested protocol proof. Losers/evicted clients drop outside the guard and no cache guard spans build/await.
5. **0067 §4: failures/cancellation.** Inject delivery join/build failures and exercise real delivery retry recovery or four-attempt exhaustion with exactly one terminal outcome and fixed value-free attempt errors. Builder failure does not poison/publish partial cache data. GET/PUT worker failures return fixed value-free native 500s before PUT replacement. Cancel or stop while actual build/preparation is held, release it, then assert no late installation/cache publication/HTTP send/terminal-total change; successful construction counters may advance. Preserve existing stop/serve-join/drain/dispatcher-join ordering and Store mutation panic behavior without a builder drain or replacement loop.
6. **0067 §5: bounded placement proof.** With one async worker and explicitly recorded spare blocking capacity, hold distinct-root construction A while ticker, warm hit, cold root B, HTTP/cache observation, fresh SMTP greeting/NOOP/DATA 250 and immediate inbox query complete before release; then A completes verified TLS delivery. Pair with real fan-out to at least 33 distinct loadable root PEMs, cold misses, TLS delivery, concurrent SMTP and immediate lookup after each 250. Assert explicit deadlines/capacities and that watchdog release was not the success path. Preserve real pool-exhaustion and cancelled-GET experiments as finite coupling/recovery evidence assigned to #220, not an arbitrary-load latency guarantee.
7. **0068: served contract.** Real `/openapi.json` GET webhook responses retain 200/403/500, awaited off-worker preparation text and exact `webhook response preparation failed` wording, without PUT replacement language. Live injected GET failure returns that native error; PUT retains its distinct fixed failure-before-swap behavior and request/response shapes. Removing GET 500 must make the served-contract test fail.
8. **Discriminating regression evidence.** Record privacy-safe mutation-red/restored-green proofs for raw echo, construction on an async worker, construction under the cache mutex and GET-500 removal. Logs/receipts must not print real key-bearing responses or private bytes, even on assertion failure. Use generated fixtures, channels/barriers/deadlines and real servers; follow 0020 placement, no sleeps as synchronization, probabilistic chaos or unprotected global hooks. Restore all mutations before committing/verifying.
9. **Final evidence and review.** A clause-to-test/command matrix covers every operative 0067 clause and 0068, including duplicate-build/eviction accounting and empty projection on the wire. Every final production line has test coverage; exact line/branch/function misses are all zero. Every house/pinned verification gate below passes on the committed candidate with the recorded executable. Current protected review assesses the key-disclosure fix itself using existing accepted-0058 full-risk routing: separate Astra correctness, official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees and independent Fable 5.1 results. Historical findings remain historical; no author-created security clearance, Pro, unauthorized fallback or routing edits. Record #220 ownership without requiring its executor implementation.

## Required verification commands

These are downstream completion gates, not results claimed by intake. The hermetic verify seat runs offline and installs nothing. Commit the final implementation before the actual verifier; bind receipts/review to that candidate and the exact local engine. No benchmark is needed to assert a universal latency bound: §5's deterministic finite-capacity protocol proofs are the selected contract.

Targeted existing suites (extend as needed; map added test names in the proof matrix):

```bash
cargo test --locked --test webhook
cargo test --locked --test api openapi_webhook_get_documents_awaited_preparation_failure
cargo test --locked --lib webhook::ca_construction_tests::
cargo test --locked --lib webhook::tests::
cargo test --locked --test binary
cargo test --locked --test p2
```

All house gates, with the specified candidate-local executable:

```bash
export CARGO_NET_OFFLINE=true
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

The list above uses the charter's public entry points. Under actual hermetic verify, check their bytes against pinned `shims.sha256` and execute the mounted pinned bundle copies instead, as `bundles/self/scripts/verify-seat.sh` requires; do not execute candidate shims in that seat. Also run its required pinned `design-check.sh selftest` and `delivery-proof-test.sh`. Exact coverage reports land in `target/coverage/`; require zero missed lines, branches and functions, no denominator exclusions or floor reduction.

Fail-closed dependency proof (the full locked tests/clippy remain mandatory):

```bash
out=$(cargo tree --locked -i native-tls 2>&1)
code=$?
if [ "$code" -eq 0 ]; then
  echo 'REFUSAL: native-tls is present' >&2
  exit 1
elif printf '%s' "$out" | rg -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2
  exit 1
fi
```

CI owns chart history/bump/selftest, real Docker smoke, network RustSec, tag-only release-rate-5k and the privileged/OS-specific port-80 bridge proof named by the charter. Operator owns sealing/signing, live settings, publication and decision acceptance. They are not new local acceptance requirements; no network, namespace, macOS or GitHub proof is demanded of this seat.

## Intake disposition

Framing is resolved: accepted 0067/0068 authorize the remaining #195 completion work without weakening a frozen guarantee. No implementation or test mutation was performed at intake; the checks here authenticate the supplied packet and executable and spot-check named owners/gates. Full implementation gates and fresh protected review remain required downstream. Commit this framing unsigned; no push, merge, tag, publication, issue closure or decision acceptance.
