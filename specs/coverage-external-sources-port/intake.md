# Intake — port epic #192 issue #219 onto current main

Run: `epic-192-issue-219-port-the-ship-35c4cdeb`; phase: intake; 2026-10-25.

## Goal

Port the three shipped #219 commits onto `aec4b46`, preserving main's newer coverage proofs, so production sources reached through Cargo-home, Rustup-home and `/rustc/<hash>` ignore paths are measured or refused rather than silently omitted.

## Evidence and authority

Verified the entire local `.forge/epic192/issue219/SHA256SUMS` packet, including README.md, operator-rulings.md, port.md and the three patches. README.md has the issue's three acceptance criteria and an empty prior-runs section; port.md records the earlier SHIP-COMPLETE delivery and identifies its three patches as the shipped evidence. Historical receipts are evidence of that delivery, not verification of this port.

Read the house rules, decision index and accepted 0035/0050 coverage rulings. The operator assigns this pre-existing denominator gap to #219, leaving #175 scoped to findings 7.1–7.4. No new decision or decision acceptance is needed for this compatible enforcement. Adjacent reservation work and the known expansion-comparator limit remain outside scope; proposed decisions are not authority to change accepted semantics.

The worktree began clean at `aec4b46`. Local history confirms `55d0f02` (#175/#229) changed the conflicting `bundles/self/scripts/coverage-gate.sh` and the scanner. Spot-checks confirm current `harvest`, `normalized_entry`, `normalized_set`, `declares_function`, the report join and selftest. Current main already refuses an external lexical source spelling through workspace containment; the shipped implementation adds real-crate regression proofs rather than replacing that scanner. The historical E/C-row APIs in the issue body are not current APIs.

Main's selftest now runs the standalone scanner unit suite in a fresh target directory and proves instrumented dep-info provenance. Preserve those additions, the scanner's fail-closed tests, Cargo crate-name handling and make-dollar decoding. The shipped patch changes the gate selftest and constitution digest, with accompanying specifications; its conflict is not permission to replace main's newer proof code.

Verified candidate-local `.forge/bin/brokkr` SHA-256 against both the requested value and `bundles/self/tooling.json`:
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.

## Acceptance criteria

1. For compiled production sources reached through `$CARGO_HOME/registry/…`, `$CARGO_HOME/git/…`, `$RUSTUP_HOME/toolchains/…` and `/rustc/<hash>/…`, the actual coverage denominator includes the production instance or the gate exits nonzero naming its source. A symlink alias to an already counted workspace file cannot silently discard the independently compiled spelling.
2. The full coverage selftest builds real, dependency-free temporary crates for registry, git, Rustup toolchains and the rustc ignore class. Each proves the intended source spelling in actual production dep-info and the installed exporter's omission using paired default/unfiltered LCOV exports. Build/setup failures fail the proof. Retain the shipped ordinary measured green control and workspace-inside-an-ignored-root control. Use owned temporary homes and the shipped owned path containing `/rustc/<hash>/`; demonstrate actual exporter filtering without host `/rustc` writes, namespaces, installs or network.
3. On the ported candidate, an isolated mutant restoring silent omission of external absolute production prerequisites makes all four new named-refusal assertions fail because the harvest incorrectly succeeds. The restored full selftest passes. Retain commands, exits, decisive assertion output and gate/scanner identities; compilation or setup failure cannot count as killing the mutant. No mutant is committed.
4. All pre-existing red and green protections remain effective: canonical containment, exact integer zero missed lines/branches/functions, expansion exclusions, and measured/no-coverage/release expansion equivalence. Preserve main's #175 standalone scanner tests, explicit selftest invocation and instrumented dep-info proofs. Existing scanner tests and the complete selftest pass; do not broaden the expansion comparator's allowance or change reservation semantics.
5. Update the constitution's canonical gate digest for the final ported bytes and pass `pinned_bundle_shims_and_proof_dispatch_are_digest_bound` plus the full delivery-constitution suite. Public dispatch shims remain fixed and pass their pinned checksum manifest. The final diff is limited to the shipped coverage proof, its digest and accurate port/design evidence; application code, dependencies, allowlist, engine pin and recipe routing stay outside scope.
6. Run every local house gate and pinned verifier addition below on the committed implementation candidate, offline without installs. Preserve fresh logs, exits, commit/tree identity and the exact coverage summary. A freshly compiled/pinned verification run must assess the modified proof bytes; the earlier shipped receipts or an intake baseline run cannot verify this port. All required local gates must pass before feature completion is claimed; unavailable or failing proofs remain outstanding.

## Frozen guarantees and limits

No direct proximity to 0001 (synchronous accept == stored), 0002 (subscribe before check) or 0003 (oldest-only pruning): this is proof-tooling work. Preserve them and 0005's ingest extraction. Accepted 0035 and 0050 govern compiled-file accounting and pinned proof ownership; no accepted semantics are weakened.

Scope is the port, not a re-audit or redesign of #219. No benchmark or new targeted wire test is warranted; the full workspace suite retains the real-server protocol tests. CI owns macOS execution, Helm/chart history/selftests, Docker smoke, network RustSec, privileged port-80 exchange and tag-only release-rate proof. Live settings and any publication/sealing remain operator-owned. These are not new locally provable acceptance criteria. Do not access GitHub, CI, journals or sibling worktrees. No push, merge, tag, publication, issue closure or decision acceptance is authorized.

## Verification commands

Use the existing candidate bundle routing and candidate-local engine. Compare the engine digest before running it; version equality alone is insufficient. Validate fixed shims, then execute canonical pinned script copies as the verifier does (the corresponding public house commands dispatch to these copies).

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
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

Additional pinned verifier and focused proofs:

```bash
cargo test --manifest-path bundles/self/scripts/coverage-fn-scan/Cargo.toml --locked --offline
cargo test --workspace --locked --test delivery_constitution
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf '%s\n' "$out" >&2; exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2; exit 1
fi
```

Run the isolated mutant/restored full-selftest pair described in criterion 3, adapting the packet's `implementation-checks/mutate.py` to fresh owned output paths and the port's current bytes. Do not rerun the historical script in place or overwrite packet evidence. Root `cargo test --workspace` does not cover the separate scanner workspace.

## Intake outcome

Resolved: sufficient verified local evidence exists to frame this compatible port. This phase records and commits framing only; no shipped patch is applied and no implementation or feature-completion claim is made. The explicit task-wide gate request also requires an intake baseline gate attempt, recorded separately below; downstream verification must rerun against the implementation candidate. The engine determines phase progression.

## Intake baseline verification

All 16 commands in [baseline-gates.json](baseline-gates.json) exited zero on committed intake baseline `633fc306758bff3912cbbe40702ccd04e0127350` (tree `b7a675fb6baf1aa578ae5cecadaf7c454db20f87`), with offline Cargo and the verified candidate-local engine. This includes every house gate, shim integrity, standalone scanner and constitution tests, design/delivery proofs and fail-closed native-tls absence. Bundle compile reported 76 valuations over 14 groups, zero unruled. Coverage was exact: 14,009/14,009 lines, 788/788 branches and 1,527/1,527 functions; the complete selftest passed. Full logs remain under `.forge/evidence/issue219-port-intake/`; LCOV and summary are under `target/coverage/run.fwDV0K/`. The receipt records their identities and command exits.

The workspace tool's client wait timed out before the runner returned; the persisted completion receipt and logs establish that every command finished successfully. This final documentation/receipt commit does not change tested implementation or gate bytes. No new #219 fixture or mutant was run during intake: those require the actual port and remain downstream obligations.
