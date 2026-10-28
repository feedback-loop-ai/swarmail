# Intake framing — epic #192 issue #219

Run: `epic-192-issue-219-count-product-bef2373e`; intake only, 2026-10-25.

## Goal

Make the pinned coverage gate count production sources reached through Cargo-home, Rustup-home and `/rustc` ignore paths in the exact coverage denominator, or refuse them by name, so none silently escapes measurement.

## Evidence and authority

Read `.forge/epic192/issue219/README.md` and `operator-rulings.md`; both passed `(cd .forge/epic192/issue219 && sha256sum -c SHA256SUMS)`. The packet contains the issue, reproduction, three acceptance criteria and an empty prior-runs section. Its operator ruling assigns this pre-existing HIGH denominator gap to #219 and keeps #175 limited to findings 7.1–7.4. No external evidence is needed for framing.

Read `docs/house-rules.md`, the decision index, accepted 0035 and 0050, and coverage/test-placement authority in 0020. Proposed 0064 and 0072 describe adjacent reservation and proof work; neither is accepted by this task. Spot-checked the scanner's production dep-info harvest, path normalization and function probe, the gate's report join and expansion comparisons, selftest fixture construction, pinned verify dispatch and the constitution digest test.

Baseline: clean worktree at `58adf0b`; local scanner/gate history includes `14dee3c` and `1d45e57`. This main version uses `harvest`, `normalized_entry`, `normalized_set` and `declares_function`; it does not contain `is_report_excluded_spelling` or the recovery candidate's lexical E/C-row inventory described in the packet. Treat the packet's function names and line numbers as historical evidence, not assumed current APIs. The two current scanner unit tests cover raw exclusion/harness-marker spellings; retain them. The described #175 recovery scanner tests are not present here at intake; preserve them if they reach main before implementation. No reproduction was rerun during intake.

Verified `.forge/bin/brokkr` SHA-256: `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, matching `bundles/self/tooling.json`.

## Frozen guarantees and decision boundary

No direct proximity to frozen 0001 (accept == stored), 0002 (subscribe before check) or 0003 (oldest-only pruning): this changes proof tooling, not mail semantics. Preserve all three and ingest extraction 0005. Accepted 0035's compiled-file inventory and measured-or-refused contract, and 0050's pinned proof ownership and expansion equivalence, govern this work. Closing silent omission is compatible enforcement of those rulings; intake identifies no semantic contradiction requiring a new decision. No decision acceptance, denominator exemption or new policy route is authorized.

## Scope

Primary files: `bundles/self/scripts/coverage-fn-scan/src/main.rs`, its standalone test suite, and `bundles/self/scripts/coverage-gate.sh` selftests/report checks. Coordinate every gate-script change with `bundles/self/scripts/shims.sha256` and the fixed digest table in `tests/delivery_constitution.rs`. Public shims remain fixed dispatchers; do not bypass pinned verification. No application changes, dependency/allowlist widening, unrelated reservation/comparator repair or recipe-routing edits are part of this task.

## Acceptance criteria

1. **Measured or named refusal for every affected spelling.** A compiled production source reached through `$CARGO_HOME/registry/…`, `$CARGO_HOME/git/…`, `$RUSTUP_HOME/toolchains/…` or `/rustc/<hash>/…`, including a symlink alias to an in-workspace source, cannot yield a successful gate while omitted from its coverage denominator. Either the actual report includes that production instance and applies the exact floor, or the gate exits nonzero naming the offending source. Canonicalizing an alias to an already counted source must not erase an independently omitted compiled spelling. Tests assert inclusion/counts or the specific named refusal, never arbitrary command failure.
2. **Real temporary-crate selftest for all three ignore classes.** `coverage-gate.sh selftest` builds real temporary crates exercising the Cargo-home class (registry and git spellings), Rustup toolchain class and `/rustc/<hash>` class without tests/examples/benches components. Exercise the real production inventory/report path and actual llvm-cov omission behavior. Each fixture reaches the intended check; build/tooling/setup failure is a failed selftest, not evidence of refusal. Include an ordinary measured green control so a blanket refusal cannot satisfy the proof. Use locally available capabilities; do not require namespaces, network or host-wide `/rustc` mutation. Design must explain the local `/rustc` spelling fixture and how it proves the actual ignore class.
3. **The regression proof discriminates.** A controlled mutant restoring the silent-drop bypass makes the relevant new selftest fail, while the restored implementation passes the same proof. Retain command, status and decisive assertion evidence for the mutant and restored control. The mutant is absent from the committed candidate.
4. **Existing protections remain effective.** Existing canonical-containment/outside-workspace refusal checks, exact integer zero-missed checks for lines/branches/functions, and measured/no-coverage/release expansion-equivalence and exclusion checks keep their current refusal behavior. Existing red and green selftest arms and scanner tests pass; retain #175's scanner tests if present when implementation starts. Do not weaken or broaden the expansion comparator's existing allowance as part of this repair.
5. **Pins and candidate identity agree.** Gate-script changes are accompanied by the required shim-manifest/digest-table coordination; `pinned_bundle_shims_and_proof_dispatch_are_digest_bound` and all delivery-constitution tests pass on the committed tree. Every changed pinned scanner/gate resource is included in the freshly compiled bundle used for verification. A run compiled before those changes supplies no verification claim for the changed bytes.
6. **All required proofs are green on the final candidate.** The complete house inventory and pinned verify additions below pass, including the exact coverage gate, its full selftest, standalone scanner tests and fail-closed native-tls absence proof. Record failures or unavailable proof as outstanding. Preserve protected review/security hold and the candidate's existing recipe routing. No feature-complete claim may rely on intake, historical receipts or an uncommitted tree.

## Verification commands and ownership

Intake records these commands; it does not implement or execute build/test gates. Implementation runs all house gates against its candidate and commits all work unsigned before returning. Because this work changes the pinned gate layer, the operator's assistant then starts a fresh verify gate-entry run from the committed candidate using the existing recipe routing and `.forge/bin/brokkr`. The hermetic verify seat installs nothing, checks fixed shim identities and executes newly pinned copies of the canonical scripts against that candidate, as required by 0050. Do not claim the earlier run verified modified proof bytes.

House inventory, with the candidate-local engine:

```bash
sha256sum .forge/bin/brokkr
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

Focused obligations (scanner is a separate workspace and is not tested by root workspace tests):

```bash
cargo test --manifest-path bundles/self/scripts/coverage-fn-scan/Cargo.toml --locked --offline
cargo test --workspace --locked --test delivery_constitution
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
sha256sum --check --strict bundles/self/scripts/shims.sha256
```

Verify must compare the engine digest above, not merely print it. Use the pinned verify seat's fail-closed native-tls probe: `cargo tree -i native-tls` must fail specifically with `did not match any packages`; success or a different failure refuses. Retain fresh LCOV/summary evidence with integer missed counts exactly zero in all three dimensions, plus the real-fixture and mutant/restored receipts. No benchmark claim is made, so no new `cargo bench` obligation is introduced.

CI owns the house inventory's chart contract/history/selftest, real-Docker smoke, network RustSec audit, tag-only release-rate proof and privileged port-80 proof. macOS-specific execution and live service/settings evidence belong to CI/operator, not new locally provable acceptance criteria. This seat does not access GitHub, CI, journals or sibling worktrees. No push, merge, tag, publication, issue closure or decision acceptance is authorized.

## Intake outcome

Resolved: sufficient local evidence exists for compatible coverage-gate strengthening, with checkable acceptance criteria and explicit downstream commands. No implementation or gate-pass claim is made by this framing.
