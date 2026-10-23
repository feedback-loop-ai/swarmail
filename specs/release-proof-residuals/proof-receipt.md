# #186 local implementation receipt

2026-10-23; run `epic-192-186-close-the-locally-p-6fc4f549`.
Design commit `2c6f3e1` passed `bash bundles/self/scripts/design-check.sh check .`
before implementation. Triage selected `engine`; the committed design retains
that route. Implementation commit: `2dce0b4` (unsigned, no push).

Criteria **18.1–18.4 are locally proved**, subject to the engine's independent
verify/review and root's integration. **18.5: EXPLICIT EXTERNAL PENDING**;
#186 and #192 remain open. This receipt does not certify CI or a release.
0051 remains proposed in its authoritative header/index. PR212's publication
order and compatible release behavior are preserved; no decision is accepted
by this change.

The only implementation changes are specific presence-failure needles and
request-stage/identity assertions in the canonical publish proof, explicit
Cargo include anchors, and the adjacent reproducible package fixture. The
three chart cases, held refusal socket, manifest-close case, HTTP-000 observer
and license/NOTICE anchors were already in PR212; they are freshly proved,
not represented as new repairs. Production guard, public shims, CI, versions,
dependencies, license policy and Rust code remain byte-identical to the design
base.

## Identities and execution boundaries

| Candidate input | SHA-256 |
|---|---|
| `Cargo.toml` | `0f207d4f19be34dd7f3f2b5353b8f0a5da593ff9012e600e1b4d3fb7cea6a72f` |
| `scripts/chart-verify.sh` | `5ff500f5031ed05bb1810013fb189a868cfc4da5b10467ee32f2a41ed8cba62e` |
| `bundles/self/scripts/publish-guard.sh` | `183a9c8bb15c6d41ea4a0a887404cb1aebca1cf7001f430e2e329355ff27a764` |
| `bundles/self/scripts/publish-guard-test.sh` | `88769c24b30b7261417c5a600a387f4164e27667be145622ce692fed39f534fd` |
| `specs/release-proof-residuals/package-proof.py` | `449ba5caff45567121122a2ce5d4a1aacc6c9d5ee938953d5edbc99d02466272` |
| `bundles/self/scripts/shims.sha256` | `72d34b479a900da837f34a4c3c3066f5964d067050166f7ab4067ff076978566` |

Installed tool identities: rustc `1.98.0 (88d9e12ae 2026-08-18)`, Cargo
`1.98.0 (797e8a9bc 2026-08-05)`, cargo-llvm-cov `0.9.1`, cargo-deny `0.20.2`,
Python `3.14.4`, curl `8.18.0` (OpenSSL `3.5.5`). Coverage selected installed `nightly-2026-10-22-x86_64-unknown-linux-gnu`
(as recorded by the LCOV run log); no tool was installed. Helm was absent, so optional Helm parser/render
coverage is not claimed.

The executed `.forge/bin/brokkr` is version `0.12.0`, SHA-256
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`, matching
`bundles/self/tooling.json`'s recorded build of base
`597f4868e3fb971b62e6a368a2d8aa23271f4531`. All three ordered patch files also
matched their recorded SHA-256 values. This seat checked the recorded binary
and patch identities; it did not rebuild or independently inspect the external
engine source checkout.

All Cargo execution was offline (`CARGO_NET_OFFLINE=true`, explicit `--offline`
for packaging/tree proofs); Rustup was offline and nothing was installed. Real
registry-shaped HTTP was loopback-only with synthetic credentials. The
implementation seat's network capability was not a hermetic verify dispatch.

Mutable candidate proof and inherited proof are separately identified:
`bash bundles/self/scripts/publish-guard-test.sh` passed, and an exact
`git show 2c6f3e1:bundles/self/scripts/{publish-guard,publish-guard-test}.sh`
copy under `.forge/proof186/pinned/scripts/` passed against the candidate.
Public shim checks passed without changing any hash. These local executions
are not relabeled as the engine's later immutable, network-denied verify seat.

## Discriminating selftests

`.forge/proof186/mutations.py` ran the real chart selftest from a private copy
preserving `scripts/` and the actual candidate workflow. Each chart control
changed only its owning validation/refusal. The publish controls copied the
real canonical guard/proof adjacent to each other and ran the actual proof
against the candidate workflow. Wrong-reason controls still refused with
exit 1; inappropriate-admission controls exited 0. Each owning selftest then
failed with exit 1 and the diagnostic below. Exact original bytes were restored
and the full real selftest passed after **every** control.

| Private control | Owning failure diagnostic | Red / restored exit |
|---|---|---|
| chart-appVersion | `coherence: a malformed appVersion: missing diagnostic appVersion is not a release semver string` | 1 / 0 |
| chart-push-baseline | `history: no push before: refusal must name` | 1 / 0 |
| chart-malformed-baseline | `history: malformed baseline: refusal must name` | 1 / 0 |
| token-reason | `token exchange 500:` | 1 / 0 |
| token-admission | `token exchange 500:` | 1 / 0 |
| manifest-reason | `manifest transport:` | 1 / 0 |
| manifest-admission | `manifest transport:` | 1 / 0 |
| http500-reason | `probe 500:` | 1 / 0 |
| http500-admission | `probe 500:` | 1 / 0 |
| http000-reason | `000 classifier only after real authenticated 404:` | 1 / 0 |
| http000-admission | `000 classifier only after real authenticated 404:` | 1 / 0 |
| presence500 | `require-present + 500:` | 1 / 0 |
| presence-token500 | `require-present + token 500:` | 1 / 0 |
| presence-refused | `require-present + connection refused:` | 1 / 0 |
| holder-close | `refusal socket ownership lost` | 1 / 0 |
| token-scope | `token exchange 500: unexpected registry request sequence` | 1 / 0 |
| manifest-tag | `probe 500: unexpected registry request sequence` | 1 / 0 |

The three presence controls were mode-specific; the refused-port control also
selected the actual holder's registry URL. Earlier absence failures cannot
certify these assertions. The two additional request controls reached the
expected refusal arm but supplied a wrong token scope or manifest tag, proving
the new request-sequence assertions discriminate independently of the needle.
There were 17 selftest red/restored pairs (15 initial plus two identity controls).
The initial batch also ran both unmodified positive selftests.

The socket control closes the same holder **after** the ready port file has
been written, and fails with `refusal socket ownership lost`. In the restored
fixture, one child binds both ephemeral sockets, retains the non-listening
holder through all attempts and is killed and joined by its exact PID at
cleanup. No released-port allocation or fixed-sleep synchronization is used.

The raw HTTP-000 experiment on installed curl returned curl exit **1**,
`Unsupported response code in HTTP response`; the guard refused with
`manifest query failed`. Separately, the narrow observer performed a real
authenticated 404 and replaced only successful status output with `000`,
triggering `HTTP 000`. This is classification-only evidence. Independent real
manifest-close evidence, stdin/header handling, real errors/exit propagation,
credential-free argv and all existing workflow mutations passed. No GHCR,
TLS-readiness, digest-attestation or registry-immutability proof is inferred.

Per-case commands, mutation replacements, script digests, exits and logs remain
under `.forge/proof186/`: `mutations.json`, `request-controls.json`,
`candidate-scripts.json` and the paired `*-red.log`/`*-restored.log` files.
These are run-private evidence, not a fabricated journal or a new gate owner.

## Actual Cargo package proof

Reproduce from a clean candidate with:

```bash
python3 specs/release-proof-residuals/package-proof.py
```

The final candidate ran actual `cargo package --locked --offline --list`
without `--allow-dirty` after committing Cargo.toml. An earlier dirty-candidate
attempt correctly refused; that attempt is not counted as a package pass.
Only disposable modified fixtures use `--allow-dirty`. The synthetic fixture
copies declared root families, creates `.cargo-home/synthetic/` decoys for
every include family and has **no nested Cargo.toml boundary** or real cache
contents/credentials.

The candidate has 68 members. The fixture differs only by the automatic
`.cargo_vcs_info.json` Git provenance member. The original five slash patterns,
individually unanchored patterns and final explicit `/` anchors have identical
membership. Those five edits clarify intent; they are not claimed as five
newly fixed leaks. Root source/test/bench/asset/data files and Cargo's automatic
manifest/lock/readme members remain present.

Individually removing the leading slash from `LICENSE-MIT`, `LICENSE-APACHE`
and `NOTICE` makes Cargo exit 0 **and** list exactly the corresponding
`.cargo-home/synthetic/<basename>` extra member. The membership assertion fails
and names that leaked member; restoration passes after each. No decoy enters
the final listing or archive.

An actual offline `cargo package --no-verify` archive passed byte comparisons
for both licenses, NOTICE, the WHATWG entity snapshot and Cargo.lock; original
manifest bytes and normalized name/version/SPDX/repository/readme/description/
keywords/categories/edition are preserved. `--no-verify` separates archive
inspection from build verification and is not a claim of compiling packaged
tests. Root locked builds/tests are separately proved below. Detailed command
exits, lists and archive are under `.forge/proof186/package/`.

## House gates and additional proofs

Every command in the sole house gate inventory passed on these implementation
inputs; `.forge/proof186/gates.json` records commands, exits, durations and logs.
The RPC waiting for the long batch timed out at 300 seconds; the workspace
process continued and completed every gate successfully. The completed status
records and reports, not the timeout, establish these verdicts.

| Command | Exit |
|---|---|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-186/.forge/bin/brokkr compile --bundle bundles/self` | 0 |
| `bash scripts/coverage-gate.sh` | 0 |
| `bash scripts/coverage-gate.sh selftest` | 0 |
| `bash scripts/publish-guard-test.sh` | 0 |
| `bash scripts/seal-branch-test.sh` | 0 |
| `cargo deny check licenses` | 0 |
| `sha256sum -c bundles/self/scripts/shims.sha256` | 0 |
| complete locked offline dependency tree with error-sensitive absence assertion | 0 |
| `cargo tree -i native-tls --locked --offline` with named-absence-only admission | 0 (wrapper; Cargo reported no matching package) |
| `bash bundles/self/scripts/design-check.sh selftest` | 0 |
| `bash bundles/self/scripts/delivery-proof-test.sh` | 0 |
| inherited exact-byte publish proof against candidate workflow | 0 |

Locked tests: **784 passed, 0 failed, 3 ignored**. Exact coverage:
**12,953/12,953 lines, 762/762 branches, 1,437/1,437 functions**, with the full
LCOV and summary under `target/coverage/run.lwxwam/`. Coverage selftest and the
additional design/tamper proofs passed. Bundle compile reported 76 valuations,
14 groups, 0 unruled and no policy-sweep warning. License deny and both native-tls
absence checks passed. No gate failure was skipped or reclassified as scope.

Later owners remain distinct: the engine performs independent immutable verify,
Qwen Max correctness, Claude Opus security, official DeepSeek Flash guarantees
and DashScope GLM-5.3 tests review (no Pro/fallback), then protected ship. Root
owns integration/CI. Actual-event chart baseline/render/bump checks, Docker
persistence/volume/context smoke, network RustSec audit, tag same-run rate proof
and the prescribed real port-80 exchange remain their CI obligations.

## 18.5 — EXPLICIT EXTERNAL PENDING

Only a separately authorized future guarded release can provide all of:

1. Approved release commit and signed triggering ref with deliberately aligned
   Cargo/Chart/appVersion; workflow revision/run URL and quality/test/coverage/
   deny/audit/compile, chart, smoke and same-run release-rate-5k results/URLs.
2. Exact `ghcr.io/feedback-loop-ai/swarmail:<version>` and `:v<version>`, plus
   `ghcr.io/feedback-loop-ai/charts/swarmail:<version>` identities.
3. Timestamped credential-redacted successful token exchanges and pre-push
   404s for both image forms and the bare chart, tied to guard revision,
   requested manifest identity and publication ordering.
4. Successful image publication before bare-appVersion presence 200, followed
   by chart absence preflight and chart push; immutable artifact/digest IDs.
5. Post-publication exact-tag 200/readback, image/chart digests and packaged
   metadata/notice identities. These are receipt observations, not new
   automated post-push enforcement or an immutability guarantee.
6. Honest partial failures, reruns and manual recovery; a partial release
   cannot be called a complete receipt.

No release tag, registry credential, GHCR publication/deletion, push, merge or
issue closure occurred. Registry-outage refusal, the concurrent check/push
window and possible image-success/chart-failure remain the accepted costs.
Local fixtures do not discharge any part of the future receipt.
