# Plan — port coverage external-source proofs

## Technical context

The application is one Rust crate. Coverage proof tooling is a Bash gate with
Python fixture assertions and a standalone Rust scanner. Use installed tools,
locked cached dependencies and offline Cargo. Add no dependency or package.
The candidate starts this design at `af32176` over main `aec4b46`; #175's
`55d0f02` changes are already present.

No plan template exists in this workspace. The only `.specify/templates/` file
is a spec template for an unrelated multi-repository workspace. This plan
supplies the requested technical context, constitution check, project structure,
research, data model, contracts and quickstart directly.

## Constitution check

Accepted 0035 and 0050 already require compiler-owned accounting and measured-or-
refused outcomes. This tests-only port changes no semantic contract and needs
no proposed decision. Preserve every accepted ruling, frozen 0001–0003 and
0005's ingest extraction. Preserve exact coverage and all existing exclusions,
expansion-equivalence and containment checks. A lexical workspace-prefix check
is not a new canonical containment proof. The known comparator limit and
adjacent reservation work remain outside scope, not accepted exemptions.

Use the current bundle routing and digest-checked `.forge/bin/brokkr`; retain
protected review and security hold. Commit unsigned under 0056, never push.
The design commit contains exactly spec.md, plan.md and tasks.md in this existing
port directory. Run-local positions, result and gate logs stay ignored.

## Project structure and touched files

| Path | Action |
|---|---|
| `specs/coverage-external-sources-port/{spec,plan,tasks}.md` | Chief's three committed artifacts; existing intake and baseline receipt stay intact. |
| `bundles/self/scripts/coverage-gate.sh` | Implementation ports the shipped six-case `ignored_source_fixtures` block additively. |
| `tests/delivery_constitution.rs` | Implementation adds/updates the canonical gate digest binding from the final merged bytes. |
| `bundles/self/scripts/coverage-fn-scan/` | Inspect and test; preserve current source, manifest, lock and all five tests. |
| `bundles/self/scripts/shims.sha256` and public shims | Check existing identities; no changes when dispatcher bytes are unchanged. |
| `.forge/evidence/issue219-port-design/` | Fresh ignored design-stage command logs and receipts. |
| `.forge/evidence/issue219-port-implementation/` and `target/coverage/` | Fresh implementation receipts and owned temporary artifacts; never overwrite the immutable packet. |

Implementation may record fresh results in this port directory. Do not apply the
old intake/design commits wholesale or copy their historical implementation
receipt into the port as if freshly executed.

## Research and approach

1. Reverify the supplied packet and engine; record entry commit/tree identity.
   Compare the shipped implementation patch with current main, especially the
   conflict after `resolve_coverage_toolchain; ensure_fn_scan`. Current `harvest`
   selects production rules, `normalized_set` propagates `normalized_entry`
   errors, and external lexical prerequisites already refuse. Keep that owner.
2. Retain main's helper suite immediately after tool resolution, with its owned
   fresh target and `fx_cmd` handling. Insert the shipped fixture function and
   call after that suite and before the existing clean-failure probes. Preserve
   every later main selftest arm, including instrumented binary provenance.
3. Keep one parameterized fixture helper with registry, git, rustup, rustc,
   green and internal arms. Resolve installed Cargo/rustc/rustdoc before setting
   per-fixture Cargo/Rustup homes. Use dependency-free real crates, private build
   directories and offline locked commands. Keep fixture paths free of unrelated
   tests/examples/benches components and harness basenames. Do not change HOME,
   installed toolchains or host `/rustc`.
4. Preserve the compiler-created alias, execution of both branches through both
   compiled spellings, and default/control exports from the same instrumented
   production run. Independent assertions must establish selected library-unit
   dep-info membership, file identity and executed DA/BRDA/FNDA records. Capture
   the selected unit's dep-info without rewriting it; do not replace it with
   Cargo's canonicalized artifact summaries or a second uninstrumented build.
   The fixture binary drives the library; main's separate binary-only fixtures
   continue proving binary-unit selection.
5. Use existing contextual-refusal helpers for all four external arms. Never
   consume failed-harvest output as valid inventory. Retain the ordinary positive
   join/exact-count control and internal ignored-root missing-source/empty-report
   control. Keep default-filter disabling confined to the unfiltered test export.
6. Repeat the shipped isolated mutant/restored proof, adapting only its owned
   paths and current source context. Use separate helper build artifacts and
   retain causal diagnostics for each of the four intended failures. Restore
   and check scanner identities before a complete passing selftest. No mutant,
   switch, altered scanner or historical receipt enters the implementation commit.
7. Hash the final merged gate, update its constitution binding, check unchanged
   shim identities and compile a fresh bundle. Commit implementation unsigned;
   run all local gates below on those committed bytes and retain fresh receipts.
   Any later code/proof change invalidates affected results and requires rerun.
   Downstream verification must bind this committed candidate to fresh pinned
   resources; the design run's initial bundle cannot certify changed proof bytes.

## Data model and contracts

No application schema or API changes. Existing scanner CLI and sorted plain
workspace-relative inventory remain intact. A selected compiler production rule
establishes source ownership; the original spelling establishes report identity;
file identity alone does not establish coverage of a second compiled instance.
Nonzero harvesting invalidates the entire output.

Each fixture's evidence includes case, tool identities, owned roots, build and
export status, selected dep-info, source spellings, default/control observations
and contextual refusal or green-count outcome. A mutant receipt includes tested
gate/scanner hashes, isolated mutation identity, commands, exits, four causal
assertions and the restored full-selftest result. Store concise receipts outside
trap-cleaned fixture roots. No receipt schema or new framework is required.

## Quickstart and validation

Run from the candidate root. First verify the packet in its own directory and
compare the local engine SHA-256 to the spec and `tooling.json`. Check fixed
shims before executing the canonical gate copies below. These canonical commands
are the implementations of the public commands in the sole house-gate inventory;
verify executes pinned copies rather than trusting dispatch shims. Set
`CARGO_NET_OFFLINE=true` for this local run; install nothing.

```bash
(cd .forge/epic192/issue219 && sha256sum --check --strict SHA256SUMS)
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
cargo test --manifest-path bundles/self/scripts/coverage-fn-scan/Cargo.toml --locked --offline
cargo test --workspace --locked --test delivery_constitution
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Also run the fail-closed native-tls absence proof: capture `cargo tree -i native-tls`
output and status; accept only a nonzero exit with the expected `did not match any
packages` diagnostic. A successful lookup or any other error fails the proof.
The workspace test command does not include the separate scanner workspace.
Record exact positive totals and zero misses, not rounded percentages.

After the chief commit, `bash bundles/self/scripts/design-check.sh check .`
must pass on HEAD. It checks the design artifacts; it does not discharge the
implementation tasks or own the sequence's final result. Do not make an
unrelated commit after the design commit. Final implementation verification and
review follow the candidate's existing recipe; no route edit is needed.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Conflict resolution deletes #175's new protection | Keep the fresh-target helper invocation and all current scanner/binary/clean-failure proofs; review an additive diff and run the standalone suite plus full selftest. |
| Canonical file coverage masks an omitted alias | Require original production-unit spelling and separate executed LCOV records; retain paired exports and both controls. Extra builds and exports cost time and disk. |
| A setup error looks like a successful refusal | Require completed build/export/evidence first, then exact category and source; inspect causal mutant assertions. |
| Exporter ignore behavior changes | Fail the real omission assertion visibly and investigate the installed tool; never invent LCOV or weaken the case to regain green. |
| Stale helper/profile or mutant artifacts contaminate proof | Use fresh owned targets, separate mutant outputs, identity checks and a restored full selftest. Preserve existing cleanup/reservation semantics without claiming external-kill or hostile-filesystem guarantees. |
| Digest certifies the historical gate instead of merged bytes | Recompute after the final edit, run the digest-binding test and full constitution/delivery proof, compile and verify fresh pinned resources. |
| Port turns into coverage-policy redesign | Keep scanner and runtime bytes unchanged; preserve canonical containment requirements without claiming a new symlink proof, and leave comparator/reservation/adjacent issues with their owners. |
| Baseline results are mistaken for completion | Label phase and commit in fresh receipts; keep implementation checkboxes open until their actual proof. Assign unavailable platform/network/release evidence to CI/operator as in the spec. |
