# Issue #151 part 1 — integration and proof receipt

Date: 2026-10-26. Run: epic-164-issue-151-part-1-integr-09634aea.
Phase: implement. Local integration and required proofs passed.

## Integrated candidate

Tested commit: `00e82befce72d55ab5ad2ec6a616548aacc0ddbb`.
Unsigned merge `290bbea` retains the candidate and locally available main
`89581763b8b4f4ddbc1a800c7cdb37dae0f2e3ff`. Both that main tip and wrapper
baseline `6a67f7e` are ancestors. The README keeps #243's tracked ruling link
and documentation boundary together with #151's wrapper guidance. Incoming
#234/#243 executable changes match main's patch exactly.

Unsigned repair `00e82be` renames and indexes proposed
[0083](../docs/decisions/0083-cli-wrapper-launch.md), corrects all wrapper-proposal
citations and dates the newly authorized integration scope. Its runtime ruling
is byte-identical to the previous proposal. The [earlier receipt](issue151-implementation.md)
retains its blocked verdict and failed exits as history.

Wrapper source, main entrypoints, baseline code/declarations, fixture helper,
wrapper tests/launch helpers, common test helpers and baseline tests match
`6a67f7e` byte-for-byte. Authored integration changes are Markdown only.
Root runtime/tests, manifests/locks, dependencies, gates and bundle/routing
inputs are unchanged from the design input. No wrapper behavior, scoring,
version, coverage floor, exclusion or license allowance was changed.

## Verified authority and local checks

The complete issue151 packet passed its own SHA256SUMS, including README
`be74941231e61014f3f4caa0803713901a876d28a91f514b2e29febde76d5df9`
and operator rulings
`bafa819f9f9296574aa3c1f5d5f30876d16584ae03fd05c7a71e2cc72ef9579e`.
The candidate-local engine matches SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`;
public shims passed their pinned digests. Pinned bundle proof owners ran offline
with `CARGO_NET_OFFLINE=true`; nothing was installed. The normative inventory
remains [house rules](../docs/house-rules.md#gates--part-of-every-change).

All commands below exited zero on the tested commit. Full output and records
of exact command, commit, start/end time and exit are retained locally under
.forge/implementation151-integration (plain provenance path, no delivery-state
link). completed-checks.json aggregates the separate status records.

| Command | Exit | Local log |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | `root-fmt.log` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `root-clippy.log` |
| `cargo test --workspace --locked` | 0 | `root-tests.log` |
| `cargo test --locked --test store_contract_docs` | 0 | `decision-inventory.log` |
| `.forge/bin/brokkr compile --bundle bundles/self` | 0 | `bundle-compile.log` |
| `bash bundles/self/scripts/coverage-gate.sh` | 0 | `coverage.log` |
| `bash bundles/self/scripts/coverage-gate.sh selftest` | 0 | `coverage-selftest.log` |
| `bash bundles/self/scripts/publish-guard-test.sh` | 0 | `publish-guard.log` |
| `bash bundles/self/scripts/seal-branch-test.sh` | 0 | `seal-proof.log` |
| `cargo deny check licenses` | 0 | `licenses.log` |
| `bash bundles/self/scripts/design-check.sh selftest` | 0 | `design-selftest.log` |
| `bash bundles/self/scripts/delivery-proof-test.sh` | 0 | `delivery-proof.log` |
| `out=$(cargo tree -i native-tls 2>&1); code=$?; printf '%s\n' "$out"; if [ "$code" -eq 0 ]; then exit 1; elif printf '%s' "$out" \| grep -q 'did not match any packages'; then exit 0; else exit 1; fi` | 0 | `native-tls-absence.log` |
| `cargo fmt --manifest-path evals/Cargo.toml --all -- --check` | 0 | `eval-fmt.log` |
| `cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings` | 0 | `eval-clippy.log` |
| `cargo test --manifest-path evals/Cargo.toml --locked` | 0 | `eval-tests.log` |
| `cargo test --manifest-path evals/Cargo.toml --locked --test boundary` | 0 | `eval-boundary.log` |
| `cargo test --manifest-path evals/Cargo.toml --locked --test wrappers` | 0 | `eval-wrappers.log` |
| `cargo test --manifest-path evals/Cargo.toml --locked --test baseline` | 0 | `eval-baseline.log` |
| `cargo test --manifest-path evals/Cargo.toml --locked --test proof` | 0 | `eval-proof.log` |
| `test ! -e /tmp/issue151-integration-dry-run-only && cargo run --manifest-path evals/Cargo.toml --locked -- baseline --config /private/wrappers.json --output /tmp/issue151-integration-dry-run-only --dry-run && test ! -e /tmp/issue151-integration-dry-run-only` | 0 | `eval-dry-run.log` |

Bundle compilation reports 76 valuations over 14 groups, zero unruled and no
warning. Exact coverage covers **14,677/14,677 lines, 822/822 branches and
1,597/1,597 functions**, zero misses in every dimension. The full lcov and
summary evidence is under target/coverage/run.rA0xYt. Coverage selftest,
publish/seal proofs, design selftest and delivery tamper proof all passed.

The unchanged decision-inventory target passes 25 tests. Full standalone eval
fmt, locked all-target clippy and tests pass; the direct targets pass 9 boundary,
15 wrapper, 3 baseline and 6 real-server proof tests. Existing negative controls
and assertions remain intact; no ignored authority packet was fabricated.

Actual dry-run JSON was parsed and checked: 16 tasks, the exact six approved
models, oracle/null controls, 32 control rows, 480 paid slots, five paid
repetitions, concurrency 1 and 600000 ms. Its steps retain controls, validation,
wrapper launch, paid matrix/accounting and report ordering. The output path
was absent before and after the command. Existing baseline tests retain the
no-credential-read/no-launch proofs. No provider or paid trial ran.

The long batches exceeded the workspace transport's 300-second response window;
recorded command exits and completion markers establish the passes above. The
eval batch then reached its execution limit during the extra wrapper target,
after the full suite and boundary target passed. That incomplete wrapper log
is preserved as eval-wrappers-interrupted-batch.log and was not counted as a
pass. Its successful separate rerun and remaining proofs have independent
status/completion records. No failed or incomplete attempt was converted to green.

This receipt and the integration task checklist follow the tested commit and
change documentation only. The final committed receipt is rechecked with the
decision-inventory and full eval boundary targets, diff/ancestry/preservation
checks and tracked cleanliness before delivery.

## Remaining ownership

CI owns the charter's platform, Helm/chart, Docker, network RustSec and real
port-80 evidence; #201 owns macOS eval gating and #221 owns namespaces. The
operator owns authenticated six-model smoke and live MCP/tool inventories,
especially dsh HTTP, the paid baseline, publication/reprioritization, live
settings, sealing and decision acceptance. Those are external owners, not
additional local completion criteria. All rows remain unconfined. Decision
0083 remains proposed. No remote service, journal or sibling worktree was read;
no push, remote merge, tag, publication, closure, sealing or acceptance occurred.
