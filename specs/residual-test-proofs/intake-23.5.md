# Intake: repair the issue #191 snapshot proof

Status: resolved framing; implementation and verification remain outstanding.
Run: `epic-192-issue-191-repair-the-fr-71efc5f2`.
Date: 2026-10-25.
Candidate: `68b4ef8602d754541512dea3793aedaa18fe0261`.

## Goal

Replace finding 23.5's source-literal snapshot assertion and its defective acceptance wording with a deterministic behavioural test that passes the existing single-snapshot scrape and fails a compiling split-lock scrape, preserving production behaviour and findings 23.1–23.4.

## Evidence and authority

The current request and `.forge/epic192/issue191/review-6cf3ced1.md` identify a spec defect, not a production snapshot bug. Spot checks confirm `src/api.rs::cumulative_rows` holds one registry lock through summing and rendering; `api::tests::cumulative_scrape_uses_one_registry_snapshot` instead discriminates via `include_str!` and a copied function literal. `spec.md` ruling 3, AC6, AC7, and matching plan/tasks mandate or protect that inadequate technique. The five replayed commits end at the candidate above; 23.1–23.4 are already reviewed clean and are preservation constraints.

Read the house charter and decision index. Accepted 0047 §2 requires total and rows from one locked snapshot; 0043 supplies the surrounding metrics contract. Accepted 0020 rules 1 and 4 permit a deterministic inline measured test with an owning-module `#[cfg(test)]` seam that a test actually installs. Accepted 0050 requires executed pinned proofs; 0056 requires unsigned author commits; 0058 preserves the full careful review route. Proposed 0072 grants no exception to accepted proof requirements. No new runtime semantics or decision acceptance is requested, so no new decision document is required.

The packet's named `operator-rulings.md` is absent. The present request explicitly permits the seam and supersedes the old spec's seam prohibition; no missing ruling blocks this framing. No GitHub, CI, journal or sibling worktree was consulted.

## Acceptance criteria for the repair

1. Amend `specs/residual-test-proofs/spec.md` ruling 3 and AC6 to require an execution-based discriminator for a second registry lock/snapshot. Remove the allowance for behavioural output to pass while a source comparison alone rejects the mutant. Reconcile AC7 and `plan.md`/`tasks.md` so no active source-pin requirement or seam prohibition contradicts the new criterion. Historical statements remain clearly historical. Preserve the criteria for 23.1–23.4.
2. Remove the source-literal assertion, source extraction anchors and copied implementation from `src/api.rs::tests::cumulative_scrape_uses_one_registry_snapshot`. Retain the local-registry fixture's exact baseline rendering: total six; LF overflow two; `a` two, `b` one, `c` one. Add deterministic runtime observations that distinguish the real single locked snapshot from sum/unlock/relock/render. An idle total-equals-rows assertion alone, source/token inspection, or a separate model of the scrape is insufficient.
3. The test executes the actual `cumulative_rows` path and observes its locking/snapshot behaviour or a controlled interleaving that exposes the split. Any required seam belongs to `src/api.rs`, is entirely `#[cfg(test)]`, and is installed and executed by at least one test. Keep the branch-stable test inline and measured under 0020. No feature flag, coverage exclusion, harness relocation, sleep-as-synchronization, probabilistic retry, or uncontrolled shared global state is introduced. All seam arms and inline test code satisfy the exact coverage gate.
4. Record an executed green/red/green proof with the same exact test command below. The negative control changes the scrape to a compiling sum/unlock/relock/render implementation while keeping the new test and seam observations effective and unchanged. Exactly one selected test must fail because the runtime snapshot/locking assertion detects the split. A compiler error, zero tests, source mismatch, unrelated panic or timeout does not count. Save the precise mutation diff, command, candidate identity, platform, exit status and relevant assertion diagnostic in local evidence. Arrange cleanup before mutation, restore exact pre-mutation bytes even on failure, verify restoration, and rerun exactly one passing test. Never stage or commit the mutant.
5. Relative to candidate `68b4ef8`, shipping production logic remains byte-identical; additions outside the inline test module are limited to test-only seam support compiled out of non-test builds. Findings 23.1–23.4 stay unchanged: no edits to Dockerfile, `tests/common/mod.rs`, `tests/binary.rs` or `tests/webhook.rs`. No dependency, license allowlist, gate, policy, routing or decision changes. Compare the final diff against that candidate, and retain the existing real-protocol regression suites.
6. Every local house gate and the native-tls absence proof below exits successfully on the restored candidate. Coverage reports zero missed lines, branches and functions and exits zero; a printed summary followed by timeout is insufficient. Record current evidence rather than inheriting historical passes. CI-only checks remain CI-owned, with no fabricated platform success. Commit the final scoped work unsigned; no push, merge, tag, publication, issue closure or decision acceptance.

## Frozen guarantees

No change sits on the implementation boundary of 0001 (synchronous accept/store), 0002 (subscribe before checking) or 0003 (oldest-only pruning). All three remain unchanged, as does 0005 ingest extraction. The nearby invariant is accepted 0047 §2's cumulative-registry scrape snapshot; this task strengthens its proof without changing it.

## Verify commands and required outcomes

Run this command on the real code, the temporary compiling mutant, and the exact restored code; require one selected test on each invocation and green/red/green outcomes as criterion 4 defines:

```bash
cargo test --locked --lib api::tests::cumulative_scrape_uses_one_registry_snapshot -- --exact
```

Preserve and execute the existing affected real-server proofs and consumers:

```bash
cargo test --locked --test binary sigterm_with_a_delivery_in_flight_prints_the_final_counts -- --exact
cargo test --locked --test webhook refused_connection_never_blocks_the_smtp_path -- --exact
cargo test --locked --test api --test lifecycle --test binary --test webhook --test mcp --test smtp_edges
```

Run the complete sole inventory in `docs/house-rules.md` on the restored candidate. The inventory commands are:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
sha256sum .forge/bin/brokkr
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

Before invoking the engine, require its SHA-256 to equal `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` and the checked-in tooling manifest's binary identity. A mismatch refuses execution. The hermetic verify seat checks public dispatch shims against `bundles/self/scripts/shims.sha256` and executes the corresponding pinned `bundles/self/scripts/` copies under 0050; it installs nothing and runs offline. The public spellings above identify the house inventory, not permission to bypass pinned proof ownership.

Also execute this fail-closed dependency proof; only cargo's explicit no-matching-package refusal passes:

```bash
python3 - <<'CHECK_NATIVE_TLS'
import subprocess
r = subprocess.run(
    ['cargo', 'tree', '--workspace', '--locked', '--offline', '--invert', 'native-tls'],
    text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
)
print(r.stdout, end='')
expected = "error: package ID specification 'native-tls' did not match any packages"
if r.returncode == 0 or expected not in r.stdout.splitlines():
    raise SystemExit('native-tls absence was not proven')
CHECK_NATIVE_TLS
```

No benchmark is required: this is a test/spec repair with unchanged shipping behaviour. Linux execution is local. macOS refusal runs, chart contract/history/selftest, real-Docker smoke, network-backed RustSec, tag-only release-rate proof and privileged real port-80 exchange remain CI-owned exactly as the house charter specifies.

## Intake handoff

This record frames the repair only. Design must amend the existing owning spec/plan/tasks; implementation must replace the test and execute the mutation proof; verify must execute the final gates. Intake has not changed implementation or those design artifacts and claims no test, mutation or gate pass.
