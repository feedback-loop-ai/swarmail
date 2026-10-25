# Implementation plan — eval proof repair

## Technical context

Rust; `evals/` is a standalone workspace with its own lockfile. Existing private unit tests can reach the constructor and private projection without production API changes. File publication already accepts a test-local `FnMut` callback. Reuse existing rand, serde_json and tempfile dependencies; add none. Root coverage remains exact 100% lines, branches and functions, and does not substitute for the standalone eval suite.

## Constitution check

Preserve accepted 0001–0005, regression-test placement under 0020, compatible implementation rules under 0045, pinned gates under 0050, unsigned author commits under 0056, and current recipe routing. Accepted 0055/0071 govern standalone proofs and v2 business snapshots; proposed 0063 is unchanged. No semantic decision, new exclusion, allowlist widening or route change is needed. Design commits exactly spec.md, plan.md and tasks.md in this directory. Position files remain uncommitted. Implementation follows in its own phase.

## Research and decisions

Source inspection confirms `Private::with_inputs` calls `StdRng::from_os_rng`; `entropy_proofs::planned` subsequently replaces it with seed 718491. The existing projection sorts unordered collections. `retain_with` validates snapshot structure before invoking the first callback; version 1 can therefore bypass every injection. `state::SNAPSHOT_SCHEMA` is the existing authority. The Linux test module lacks the synchronous Write trait for its UnixStream calls, matching intake's E0599 receipt.

No plan template exists in this candidate's `.specify/templates/` (only spec-template.md). This plan supplies the required technical context, constitution check, project structure, research, data model, contracts and quickstart inline, keeping the chief's exact three-file scope.

## Project structure and files

| File | Planned change |
|---|---|
| `evals/src/fixture.rs`, `entropy_proofs` | Shared unplanned fixture/account setup; preserve deterministic `planned`; add four-pair per-task constructor test using existing projection. |
| `evals/src/artifacts.rs`, named publication test | Current business schema constant, local callback witness and exact injected-error assertion; retain every original assertion and stage. |
| `evals/src/runner.rs`, Linux `test_probes` | Only `use std::io::Write as _;` to restore test compilation. |
| `specs/eval-proof-repair/{spec,plan,tasks}.md` | Design and later implementation/proof receipts. |

## Implementation approach

1. Add the narrow runner import and establish that standalone eval tests compile. If the recorded diagnosis is wrong, name the actual blocker; compilation failure never counts as the constructor negative control.
2. Extract constructor plus account population into a small test-local unplanned helper with fixed public inputs. Existing `planned` calls it, installs the original deterministic RNG and calls `plan`. The new test calls it and `plan` without any RNG override. Do not add an entropy mode, optional RNG parameter or another production constructor.
3. For each catalogue task, build four fresh pairs using one fixed explicit 32-byte public seed (for example `"00".repeat(32)`) and `2026-10-24T12:00:00Z`. Collect canonical private projections into left and right vectors in pair order. Assert actual material in every fixture and expected collections for the special secret classes; assert vector inequality within each task. All state is instance-local; the existing monotonic clock is not compared or slept on.
4. Keep all old controlled-entropy assertions verbatim. Prove the new test green, then temporarily replace only the constructor initializer with `StdRng::from_seed(crate::provenance::seed_bytes(&inputs.seed).unwrap())`. Capture the intended equality failure and restore the exact initializer immediately, including on an interrupted proof. Verify the production diff is empty and rerun the focused test green before any implementation commit. This is a local regression negative control, not a committed mutation framework.
5. Set the named Snapshot's schema to `crate::state::SNAPSHOT_SCHEMA`. Inside each stage iteration, initialize a local `injected` boolean; set it only where the existing callback returns `Err("injected write failure".into())`. After `retain_with`, preserve `is_err()` and absent metadata and add witness/error equality assertions. Leave non-target calls to `publish` intact. Do not change any unrelated version literal.
6. Run focused and standalone suites, commit the implementation/evidence unsigned, then run all local pinned gates against committed work. If further fixes are needed, commit them and rerun affected proofs and required final gates. Record truthful results; a drafted design or structural design check is not feature completion.

## Data model and contracts

No serialized or runtime model changes. Test data consists of four ordered private projections on each side per task, and one boolean per publication-stage attempt. Expected populated material includes token/OTP keys for OTP tasks, password plus reset token for reset, ticket/challenge for support, and UID plus invite token for invites. A nonempty account list with all passwords null is insufficient. Secret values remain test-local; report task identifiers and verdicts, not captured secret material, in delivery receipts.

Use current snapshot v2 via its constant. Preserve separate version-1 artifact metadata/replay envelopes and the completion-marker-last contract. No API, CLI, scoring, entropy generation, storage or delivery behavior changes.

## Quickstart and validation

Work offline and install nothing. Verify the engine is sha256 `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`; use `.forge/bin/brokkr` and the candidate's current bundles. The charter is the authoritative gate inventory; pinned scripts implement its public shims. A local invocation of the unchanged `bundles/self/scripts/verify-seat.sh` with a run-local prompt/result and explicit engine path runs the complete root inventory plus pinned obligations. Check the generated result is `pass`, not just the script's exit status. Policy-sweep warnings refuse.

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked production_construction_keeps_secrets_independent_of_public_seed
cargo test --manifest-path evals/Cargo.toml --locked entropy_proofs
cargo test --manifest-path evals/Cargo.toml --locked every_publication_stage_refuses_without_a_completion_marker
cargo test --manifest-path evals/Cargo.toml --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Also run the pinned verify-seat fail-closed native-tls proof: `cargo tree -i native-tls` succeeding refuses; a failure passes only with `did not match any packages`, not an arbitrary Cargo error. Check zero missed lines, branches and functions and fixed shim digests. For the design commit only, `bash bundles/self/scripts/design-check.sh check .` must select exactly this one committed, clean spec directory from HEAD. Later implementation commits are assessed through their own delivery route, not falsely presented as design commits.

## Risks

- **Probabilistic collision:** four independent pairs per task reduce the lone-OTP collision risk to about 1.5e-24 under uniform independent draws. No retries or individual low-entropy inequalities. This is a regression detector, not an entropy certification or exhaustive per-field theorem.
- **Vacuous entropy proof:** assert actual secret population and expected special collections, use sorted private projections, compare within each task, preserve deterministic controls, and require the targeted red/green constructor experiment.
- **Vacuous publication refusal:** assert both the selected callback witness and the injected error in addition to all existing checks. Early schema/setup failures must fail the test.
- **Mutation escape:** restore the initializer exactly and inspect production hunks before commit; only the test changes may remain.
- **Prerequisite scope creep:** only the Linux test-local Write import is admitted. Preserve lifecycle tests and production runner code; report any newly discovered blocker precisely.
- **Misstated verification:** baseline intake evidence is context, not evidence for implementation. Standalone evals must pass independently; exact coverage and selftest cannot be waived. CI/operator-owned evidence stays with its named owner and is not claimed locally.

## Implementation proof — 2026-10-25

Both packet files passed packet-relative `sha256sum -c SHA256SUMS`; the
candidate-local engine matches the recorded `ab935558…c6a885f` digest.
The single test-only Linux Write import restored standalone compilation.
Source comparison confirms the production prefixes and both existing entropy
tests are byte-identical; the constructor mutation below was fully restored.

The new test plans four fresh pairs for each of the 16 catalogue tasks using
identical public inputs and the untouched constructor, checks actual secret
collections, then compares the two canonical private-projection vectors within
each task. The publication test now uses `SNAPSHOT_SCHEMA`, witnesses all four
injected stages and requires the exact injected error, preserving its original
assertions and real non-target publication.

Standalone commands passed with `CARGO_NET_OFFLINE=true`:

- `cargo test --manifest-path evals/Cargo.toml --workspace --locked --no-run`
- `cargo fmt --manifest-path evals/Cargo.toml --all -- --check`
- `cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings`
- `cargo test --manifest-path evals/Cargo.toml --locked --lib production_construction_keeps_secrets_independent_of_public_seed`
- `cargo test --manifest-path evals/Cargo.toml --locked entropy_proofs` (all three controls)
- `cargo test --manifest-path evals/Cargo.toml --locked every_publication_stage_refuses_without_a_completion_marker`
- `cargo test --manifest-path evals/Cargo.toml --workspace --locked` (185 passed, zero failed or ignored)

For the causal negative control, temporarily replace only the constructor's
`StdRng::from_os_rng()` initializer with
`StdRng::from_seed(crate::provenance::seed_bytes(&inputs.seed).unwrap())`.
The same focused `--lib` command fails with exit 101 at the intended assertion:
`signup: private projections repeat across independent constructors` (zero
passed, one failed). Restoring the exact original file bytes and rerunning the
command passes. No secret values appear in this receipt, and no mutation is
committed. This detects public-seed-derived constructor entropy; it does not
certify OS entropy quality or independent variation of every individual field.

The post-commit pinned root gate run remains outstanding at this checkpoint.
Its run-local verdict belongs in `.forge/epic192/issue234/implement-gates.json`;
that result must be `pass` before T7 or this implementation is complete.
