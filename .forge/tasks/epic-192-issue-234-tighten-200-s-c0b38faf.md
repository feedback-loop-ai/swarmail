# Intake — tighten issue #200's eval test proofs (#234)

Run: epic-192-issue-234-tighten-200-s-c0b38faf. Phase: intake only.
Disposition: resolved framing; implementation and verification remain later work.

## Goal

Strengthen the eval secrecy regression proof through production construction and update the publication-failure test to snapshot schema 2, preserving all existing assertions and production behavior.

## Authority and evidence

Read `.forge/epic192/issue234/README.md` and `operator-rulings.md` first; both pass `(cd .forge/epic192/issue234 && sha256sum -c SHA256SUMS)`. The first checksum command ran from the repository root and was invalid; the corrected packet-relative command verifies both files. The README contains the two MEDIUM residual findings from #200's REVIEW-RESIDUAL-OK port and no prior #234 run outcomes. Nothing needed for this framing is missing.

Read the house rules, decision index, accepted 0055's standalone-evals and dated v2 integration contracts, proposed 0063 and accepted 0071. Local history identifies c13a507 (#200/#237) as the introduction of these tests, following aec4b46 (#198/#231). The current `Private::with_inputs` initializes `private_rng` with `StdRng::from_os_rng()`. `entropy_proofs::planned` subsequently replaces it with a fixed RNG; both existing entropy tests therefore bypass constructor entropy. `artifacts::tests::every_publication_stage_refuses_without_a_completion_marker` constructs a schema-1 snapshot and injects failures at result.json, state.json, diagnostics.log and metadata.json. Accepted 0071 requires business snapshot version 2.

The local engine hashes to ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f, matching bundles/self/tooling.json. Keep current candidate bundles/routing and review/security holds unchanged. The proposed status of 0063 is not acceptance; this test-only work preserves accepted contracts and needs no new semantic decision. Only local files and git history were consulted; no remote services, journal or sibling worktree.

## Scope and frozen guarantees

Implementation is limited to test code in `evals/src/fixture.rs` and `evals/src/artifacts.rs` (and the required design/delivery records). No production behavior, manifests/locks, dependencies/allowlist, gate/coverage policy, engine pin, recipe or decision status changes. Existing assertions may not be removed or weakened. Do not introduce mocks, sleep synchronization or production entropy-control seams.

No frozen guarantee 0001–0003 is directly changed: synchronous accept/store, subscribe-before-check and oldest-only pruning remain intact. These tests concern eval fixture entropy and retained-artifact publication, not SMTP acceptance, await ordering or pruning. No performance claim or benchmark is needed.

## Checkable acceptance criteria

1. A regression test constructs two fresh fixtures via production `Private::with_inputs` with the same explicit public seed and fixture time, without replacing/reseeding `private_rng` after construction, then plans comparable fixtures and asserts their private secrets differ. Exercise the existing catalogue/private-projection coverage so the assertion is backed by populated secret-bearing fixtures rather than only fresh IDs or unrelated state. A constructor regression that derives private entropy deterministically from that public seed must make this proof fail. The implementation/design records identify the focused test and demonstrate this negative control locally, restoring it before final gates.
2. Preserve the existing controlled-entropy assertions: changing the public seed with fixed private entropy leaves the private projection equal; snapshots exclude tokens, OTPs, passwords, ticket challenges and invite UIDs as currently checked; changing secret consumption does not change the public message suffix/header projection or invite schedule. Existing deterministic seams may remain for these separate controls, but cannot be used to satisfy criterion 1.
3. `every_publication_stage_refuses_without_a_completion_marker` uses the current business snapshot version (2, preferably the existing schema constant if accessible). Retain all four failure stages and both failure/absent-metadata assertions for every stage. Do not alter schema readers, accepted version semantics or the failure injection mechanism to make the test pass.
4. The final implementation diff changes only the scoped tests and required delivery records, preserves production construction/planning/publication behavior, and passes the standalone evals checks plus every local house/pinned gate below on committed work. Root green alone is insufficient because evals is its own workspace. No acceptance/merge/publication action is authorized.

## Verify commands and ownership

These commands define the completion proof; intake framing is not implementation success. Work offline with no installation. Admit the pinned engine, check public shim digests, and execute the pinned bundle scripts against the candidate, as the existing verify seat requires (0050). Public scripts/ entries in the house inventory are dispatch shims, not alternate gate implementations.

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
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
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
```

Run the pinned verify-seat's fail-closed native-tls absence proof too: successful `cargo tree -i native-tls` refuses; a failed command passes only when its diagnostic says `did not match any packages`; arbitrary Cargo errors refuse. Coverage must have zero missed lines, branches and functions, with its selftest passing. Bundle policy-sweep warnings refuse. Do not lower the coverage floor or expand exclusions.

Focused commands supplement the full suites:

```bash
cargo test --manifest-path evals/Cargo.toml --locked entropy_proofs
cargo test --manifest-path evals/Cargo.toml --locked every_publication_stage_refuses_without_a_completion_marker
```

Add the new constructor-entropy test to the entropy_proofs module or record its exact filter if design chooses another test location. Run it normally and under the temporary public-seeded constructor mutation; the latter must fail for secret equality, not setup/schema/tool failure. Restore the mutation and rerun green before committing implementation. No mutation or source implementation is performed by intake.

CI owns macOS eval proof through #201, privileged port-80 exchanges, chart/bump/history proofs, Docker smoke, network RustSec and tag-only release-rate-5k. The operator owns live GitHub settings and branch sealing. Namespace isolation belongs to #221. These are ownership records, not local prerequisites or evidence claims. Do not access these services or add criteria this seat cannot prove.

Commit the framing unsigned; later seats commit their work before verification/return. No push, merge, tag, publication, issue closure or decision acceptance. The engine alone selects the next phase.

## Intake baseline execution evidence

Executed the existing pinned `bundles/self/scripts/verify-seat.sh` with the candidate-local `.forge/bin/brokkr` against framing commit 5635510. Its local result `.forge/epic192/issue234/intake-gates.json` is `pass`: root fmt/clippy/tests, bundle compilation, exact 100/100/100 coverage and selftest, design-check selftest, delivery tamper-boundary proof, publish guard, keyless seal proof, license check and fail-closed native-tls absence all passed. The workspace transport timed out at 300 seconds while execution continued; the subsequently read completed result, not the timeout, is the gate evidence. No runtime/source/test edits occurred.

Standalone eval fmt passed. Eval clippy, the full suite and both focused filters each exited 101 before executing tests. The unchanged eval library test unit fails E0599 at `evals/src/runner.rs:3168`, `:3182` and `:3726`: its UnixStream `writeln!`/`write_all` calls need `std::io::Write` in scope in the test module (compiler suggests the import at line 2900). Logs are `.forge/epic192/issue234/intake-evals-{fmt,clippy,test}.log`, `intake-entropy.log`, `intake-publication.log`; exit statuses are in `intake-evals-status.txt`.

This is an existing test-compilation prerequisite, separate from the two requested findings; intake did not repair it or expand implementation scope. Design must account for this exact prerequisite to the mandatory standalone checks; final eval green is not waived, and no feature/regression red/green claim can be made from these compilation failures. Nothing external is missing. The framing remains resolved; implementation, constructor negative control, standalone green and protected review remain outstanding. This receipt-only addition does not change any code or gate and is not a claim that the feature has been verified.
