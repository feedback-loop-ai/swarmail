> Historical #195 artifact restored on 2026-10-25. The current port and its
> proof obligations are governed by [webhook-ca-port-lows](../webhook-ca-port-lows/spec.md).
> Earlier stops and gate receipts below are dated history. 0067/0068 are
> accepted; 0069 was accepted for #220 on 2026-10-25 and is not implemented here.
> The low shared-blocking-pool availability residual is tracked by #220.

# Plan — webhook CA verification recovery

Implements [spec.md](spec.md). This is a verification recovery of existing
0207248 behavior from starting candidate 1b1f0df in run
`continue-epic-192-issue-195-usin-2b482394`; no new production implementation
is planned.

## Technical context

Single Rust crate; Tokio, axum, reqwest/rustls, rustls-pemfile and base64 are
already present. No dependency or license change. The checkout has no
`.specify/templates/plan-template.md`; required research, data model, contracts
and quickstart are inline, following the existing repository plan structure.

## Constitution check

Preserve accepted 0001–0003/0005, 0041 and 0048. Existing 0062 stays proposed;
no new semantic amendment is introduced by recovery. Stop contradictory work
for an exact operator ruling. Do not edit gates, recipe policies, journal
history, accepted records, dependency allowances or coverage exclusions.
Use real servers, generated fixtures, isolated inboxes and deterministic
channels/barriers. Keep 0020's measured inline default and only its justified
cold-path sibling harness. Commit unsigned; never push, merge or close issues.
The engine alone dispatches protected phases and rules their results.

## Research and resolved questions

The latest verifier refused the `/tmp/brokkr-main-latest-54d5/target/release/brokkr`
identity in `.forge/results/d148a45b-47fa-430c-87ea-e5badf7fe1fb.json` before
proving the candidate. The earlier
`.forge/results/4d4da8e7-fe92-4565-86c8-4fde50227082.json` refused the
host-installed executable. Both are provenance failures, not functional
regressions. The candidate-local binary's observed digest matches tooling;
actual launch and dispatch still need to prove that identity. The bundle passes
`{brokkr}` to `verify-seat.sh`, which checks that supplied executable before
running gates. Changing PATH for a manual command cannot repair an already
incorrect launch. Subsequent execution must originate through
`/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-195-fast/.forge/bin/brokkr`
and preserve that identity into boxed dispatch. This chief does not relaunch the
run, impersonate verify or alter either historical receipt.

`public_ca_pem` reconstructs certificate DER and validates the whole projection
through the uncached builder. `get_webhooks` snapshots then offloads;
`set_webhooks` awaits preparation before its own synchronous swap. `client_for`
checks under a short lock, awaits `build_delivery_client` without the guard,
then rechecks. Both placement boundaries matter. Existing owner-local tests
cover them and their cancellation/publication consequences; start by rerunning
those tests rather than creating new machinery.

0048 preserves unrelated-section input and diagnostic non-round-trip echoes.
The selected alternative needs no new admission ruling. 0041's earlier-batch
rejection of CPU offload does not specify builder thread identity; proposed
0062 already records this compatible follow-up interpretation. Reusing that
proposal does not elevate it to accepted authority.

The mandatory design commit changes HEAD. Keep a clean baseline comparison
showing only this directory updated from 1b1f0df; retain full commit and
source-blob IDs in run-local receipts. Later repair commits must invalidate prior candidate-bound
verification/review. Never advance a phase by copying or editing a receipt.

## Project structure and files

| Path | Recovery ownership |
|---|---|
| `specs/webhook-ca-recovery/spec.md`, `plan.md`, `tasks.md` | The only three files committed by this chief. |
| `src/webhook.rs`, `src/api.rs` | Existing projection/preparation/cache owners to inspect and prove; change only if fresh evidence demonstrates a concrete compatible defect. |
| `src/webhook/ca_construction_tests.rs`, `tests/webhook.rs` | Existing deterministic and real HTTP/TLS/SMTP proofs. Extend only for an evidenced proof gap. |
| `src/smtp.rs`, `src/store.rs` | Preserve production semantics; inspect existing test-only observation seams and ordering. |
| `README.md`, `src/static/openapi.json`, `src/static/llms.txt` | Existing public contract documentation to check for consistency, not rewrite by default. |
| `specs/webhook-ca-safety/`, `docs/decisions/0067-webhook-ca-safety.md` | Historical implementation design and proposed clarification retained unchanged. |
| `bundles/self/tooling.json`, pinned scripts and recipe | Read-only execution authority; use recorded candidate-local engine by absolute path. |
| `.brokkr/state/design/positions/`, `.brokkr/state/tasks/`, `.forge/results/` | Ignored panel/intake and proof evidence. Never stage or fabricate journal evidence. |

## Data model and contracts

No new persisted entity, public field or cache representation. Original
`WebhookTarget.ca_pem` stays an optional raw string; the response is a separate
optional canonical certificate projection, with Some(empty) distinct from None.
GET is a snapshot; PUT echoes its own installed list despite competing writers.
An owned blocking job can construct/prepare and count completed delivery builds,
but cannot install configuration, publish cache entries, send requests or alter
terminal delivery totals. Those effects remain with the async owner.

One cache mutex owns the clients map and LRU deque; successful duplicate builds
are permitted. Preserve all accepted request/status rules, whole-list atomicity,
cache-neutral offline API work, TLS policy and existing shutdown accounting.
Evidence records carry command, exit status, full candidate ID, relevant source
blobs, executable digest, workload/capacities and decisive output. Historical
mutation controls remain labelled historical unless rerun on the current source.

## Implementation and proof sequence

1. Commit only the recovery artifacts, verify the HEAD path set and unchanged
   implementation blobs, and run the structural design preflight. The sequence's
   pinned final check still owns advancement.
2. At subsequent implementation/verification, preflight the candidate-local
   engine and retain both original failures. Prove actual engine launch and
   verifier dispatch use its absolute path; inspect the supplied executable and
   digest in fresh dispatch evidence. A missing lawful executable or repeated
   wrong dispatch is a setup blocker; do not silently use host PATH, replace the
   pinned binary, change accepted digests or bypass the provenance refusal.
3. Run real GET/PUT privacy, direct-install fail-closed, complete-root refusal
   and later-root HTTPS tests. Check raw configuration, response shape,
   credentials and API cache neutrality separately from label absence.
4. Run the owner-local construction module, including the constrained held
   builder and saturation controls, plus the 33-root/eight-SMTP fan-out.
   Record fixed capacities/deadlines and require milestones before release.
   Serialize global hooks/cache tests, release/join owned work during cleanup,
   and retain independent watchdog escape paths as failures, never success.
5. Compare existing three mutation-red/restored-green receipts to current source
   blobs. Reuse only as explicitly historical mechanism evidence. If a repair
   changes a relevant proof, rerun the corresponding isolated mutation and
   restored-green control with bounded cleanup, leaving no mutant behind.
6. Fix only a concrete compatible defect found by fresh proof, in its existing
   owner. Update dependent design and prove it. A semantic contradiction instead
   requires a proposed amendment and operator acceptance before dependent work.
7. Run every house gate and the actual pinned hermetic verifier against the
   final candidate. Full protected correctness/security/guarantees/tests review
   follows through engine policy, retaining hard security and unknown-fact stops.
   No local script or this document substitutes for dispatched phase results.

## Quickstart and validation

Record `git rev-parse HEAD`, `git status --porcelain --untracked-files=all`,
`git diff --name-only 02072482292d4ee7cc1a0cbbd518e21f6224b60b HEAD` and the
candidate-local engine hash. Compare the hash to the tooling record before
invoking its absolute path. Local design preflight:
`bash bundles/self/scripts/design-check.sh check .`; the engine subsequently
runs the pinned copy against the committed artifacts.

Focused proof commands (all results must be retained, not inferred):

```bash
cargo test --locked --test webhook private_keys_in_ca_pem_never_appear_in_get_or_put_responses
cargo test --locked --test webhook direct_webhook_install_projects_unloadable_ca_as_empty_on_get
cargo test --locked --test webhook webhook_ca_put_rejects_every_invalid_certificate
cargo test --locked --test webhook webhook_second_ca_root_delivers_over_verified_https
cargo test --locked --lib webhook::ca_construction_tests
cargo test --locked --test webhook deliveries_through_more_pems_than_the_cache_cap_all_succeed
```

Then execute the entire [authoritative gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
and verifier's additional pinned selftests and fail-closed native-tls probe.
Use pinned proof resources and check shims; offline verification installs
nothing. Keep CI-owned chart/history/selftest, Docker, RustSec, tag-rate and
port-80 evidence distinct. A green local command is not the engine's verifier
or protected review result. No new production path means no new test by default;
existing discriminating tests must still run fresh.

## Risks

| Risk | Mitigation and accepted remainder |
|---|---|
| Old receipts appear to repair VERIFY-FAIL | Preserve failure, record current candidate/executable and require actual new verification and protected review. |
| Design commit defeats exact-candidate wording | Prove only three docs differ from 0207248; record descendant HEAD and bind all subsequent evidence to it. |
| Relabelled keys or embedder bypass leak | Whole-bundle loadability and direct-install tests; no raw or partial fallback. Covert data in valid certificates is outside scope. |
| Duplicate work and pool saturation | Recheck publication; test spare/exhausted capacities separately. No global work or arbitrary-load latency promise. |
| Detached construction outlives cancellation | Job has no publication/install/send authority; lifecycle tests allow build count only, not late terminal changes. |
| Canonical diagnostics hide malformed roots | Empty supplied output on any projection failure; retain original configuration and document non-round-trip behavior. |
| Global test races or stalled mutants | Scoped hooks, serialized cache ownership, independent watchdog cleanup and real deadlines; no sleep synchronization. |
| Gate or provider unavailable | Report unresolved execution; no fallback review model, gate weakening, fabricated pass or manual transition. |
| Accepted-rule drift | No narrower PEM admission or new resource policy; proposed 0062 stays proposed, and exact conflicts wait for the operator. |

## Historical evidence — predecessor chief, 2026-10-24

The predecessor chief recorded that its precommit local run passed every command
in the house inventory: formatting,
locked all-target Clippy, full locked workspace tests, candidate-local recorded
engine compile, exact coverage, coverage selftest, publish-guard proof,
seal-branch proof and licenses. Shim integrity, design-check selftest and
fail-closed native-tls absence also passed. Full tests include the named privacy,
placement, capacity and distinct-root fan-out regressions. Receipts and exact
command/exit records are retained under the ignored
`.brokkr/state/design/recovery-chief-gates/` directory.

That predecessor workspace call timed out after 300 seconds, but its gate process
continued; the subsequently read completed summary records exit 0 for all
checks (coverage completed in 226.6 seconds). This is a local precommit gate
receipt on unchanged implementation bytes, not an engine-dispatched hermetic
verification result. After the design commit, the sequence's pinned check and
fresh candidate-bound verification/protected review remain mandatory. CI-only
proof is not claimed. Those receipts do not cover this new chief commit or
replace the latest failed verifier. This continuation records its own local gate receipts separately under
`.brokkr/state/design/continuation-chief-gates/`; only completed command exits
may be reported as local passes. Fresh engine-dispatched verification and
protected review remain outstanding even when all local gates pass. No test,
source, accepted decision or policy change is planned.
