# Issue #151 part 1 — integrate the finished wrappers (intake)

Run: `epic-164-issue-151-part-1-integr-09634aea`.
Phase: intake. This record frames the work; it implements no integration.

## Goal

Integrate the completed wrapper candidate through 6a67f7e with locally available
origin/main, repair its decision numbering/index and documentation links, and
prove every local gate without changing wrapper behavior.

## Authority and orientation

Read the house charter and decision index, accepted 0055 and 0050, and proposed
0076 and the current wrapper-launch proposal. The checksum-verified operator
ruling allocates 0079/0080 to #152, 0081 to routing, 0082 to #153 and **0083**
to this wrapper proposal. Keep 0083 proposed; accept nothing further.

The supplied issue151 SHA256SUMS passed from its packet directory, including
README.md `be74941231e61014f3f4caa0803713901a876d28a91f514b2e29febde76d5df9`
and operator-rulings.md
`bafa819f9f9296574aa3c1f5d5f30876d16584ae03fd05c7a71e2cc72ef9579e`.
The initial checksum invocation used the repository root instead of the packet
directory; its path failures were corrected by checking from the packet directory.
The packet README's prior-runs section is empty; the supplied feature and
[implementation receipt](issue151-implementation.md) record the prior outcome.
No GitHub, CI, journal or sibling worktree was consulted.

History spot-check: HEAD is 6a67f7e; origin/main is locally available at
8958176 (#243), following a037dc9 (#234) and f49d122 (#191).
The prior receipt records 15 passing wrapper tests and the successful dry-run,
with root tests/coverage blocked by the unindexed decision and the full eval
suite blocked by the authority link. These are historical receipts, not proof
of this integration. No required workspace input is missing.

Code spot-check: wrappers::run still owns validation, private selection,
preflight and fixed launch builders; wrappers::validate_config reads only the
selection before paid admission. Existing wrappers/wrappers_launch tests own
launch, secrets, tool surfaces and cleanup. Main's boundary suite adds
tracked_markdown_never_links_into_delivery_state and its negative controls.
Its README repairs the authority link to docs/rulings/epic-192.md and adds the
tracked-documentation boundary guidance. The root decision-inventory tests
require every decision file to have exactly one resolving index row.

Frozen guarantees nearby: none is changed. Preserve 0001 synchronous
accept/store, 0002 subscribe-before-check and 0003 oldest-only pruning, along
with 0005 ingest extraction. This is compatible integration/documentation work
under existing authority, with no new runtime semantics needing acceptance.

## Acceptance criteria

1. The final branch contains both 6a67f7e and the recorded local origin/main
   tip as ancestors (git merge-base --is-ancestor succeeds for both). Perform
   the expressly requested local merge; resolve evals/README.md by retaining
   #243's tracked ruling link and documentation-boundary section together with
   #151's wrapper documentation. No fetch or remote merge is needed.
2. The wrapper proposal exists as docs/decisions/0083-cli-wrapper-launch.md,
   has title number 0083 and Status proposed, and its number-allocation text
   follows the supplied ledger. Its old wrapper-launch filename is absent.
   The decision index contains exactly one resolving proposed 0083 row.
   Rename every reference to this wrapper proposal, including references in
   specs/cli-wrapper-launch, evals/wrappers.md and
   evals/issue151-implementation.md. No stale wrapper-proposal filename reference
   remains in tracked files; do not repurpose #152's reserved number or
   rewrite unrelated historical evidence/acceptances.
3. No tracked Markdown link targets delivery state. Main's full boundary
   target passes, including its existing link checks and #243's scanner and
   negative controls; the decision inventory target passes without weakening
   either suite. Documentation links to the renamed decision resolve.
4. Wrapper behavior is unchanged: evals/src/wrappers.rs and existing wrapper
   tests/helpers/entrypoint scripts match 6a67f7e, verified by a scoped git
   diff after enumerating the wrapper paths from that commit. Other executable
   changes are exactly the incoming main changes; compare the merge result
   against both parents to rule out additional runtime/test modifications.
   All 15 existing wrapper tests pass, retaining assertions, provider-free
   stand-ins, private credentials, canonical surfaces and owned cleanup.
5. The full standalone eval suite and its fmt/locked all-target clippy pass.
   Baseline and real-server proof targets pass. The actual dry-run succeeds
   with 16 tasks, six models, 32 control rows, 480 paid slots, five repetitions,
   concurrency 1 and 600000 ms; it reads no credentials, launches no trial and
   creates no output directory. No paid/model-provider trial is executed.
6. Every local house gate and pinned supplemental proof below exits zero
   against the committed integrated candidate; coverage reports exactly zero
   missed lines, branches and functions and its selftest passes. No coverage
   exclusion, denominator/floor reduction, dependency/allowlist widening,
   recipe-routing change or assertion weakening is introduced. Record command
   exits/logs; a failed check is outstanding work, never a green receipt.
7. Integration and receipts are committed unsigned and the tracked worktree is
   clean before returning from delivery. No push, remote merge, tag,
   publication, issue closure, sealing or decision acceptance occurs.

The task's specific local origin/main merge is the integration action; the
prohibition on merge/push/publication remains in force for remote delivery.
The previous proposal's index-owner restriction describes the earlier run;
this request explicitly authorizes that documentation repair now. Preserve
the prior failure history and add dated integration context where necessary.

## Verify commands

Run offline with CARGO_NET_OFFLINE=true; install nothing. Before running the
engine, require its sha256 to equal the supplied pin exactly:

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --locked --test store_contract_docs
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

These are the house commands instantiated with the candidate-local engine
and pinned script owners (0050); public shims are digest-checked, not executed.
Use the candidate's bundles/routing as supplied. Compile must report no policy
sweep warning or unruled valuation. Also run the fail-closed absence proof:

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then exit 0
else exit 1; fi
```

The independent workspace requires its own proof:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked
cargo test --manifest-path evals/Cargo.toml --locked --test boundary
cargo test --manifest-path evals/Cargo.toml --locked --test wrappers
cargo test --manifest-path evals/Cargo.toml --locked --test baseline
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo run --manifest-path evals/Cargo.toml --locked -- baseline --config /private/wrappers.json --output /tmp/issue151-integration-dry-run-only --dry-run
```

Verify the dry-run output product and output-directory absence explicitly;
existing baseline tests prove its no-launch/no-credential-read contract.
No benchmark is required: no performance contract changes.

CI owns macOS, Helm/chart baseline/history, Docker, network RustSec and real
port-80 receipts named by the charter; #201 owns macOS eval integration.
#221 owns namespaces. The operator owns live settings, authenticated six-model
smoke/live MCP inventory (including dsh HTTP), paid baseline, publication and
epic reprioritization, sealing and decision acceptance. These are recorded
owners, not extra local acceptance criteria. No remote proof is claimed.

## Intake result

Resolved: the request is actionable and contradicts no frozen guarantee or
accepted decision. Integration and its green-gate verdict remain later-phase
work. Intake itself changes only this framing record; any checks below prove
this unintegrated framing candidate and cannot certify the requested merge.

## Observed intake checks — 2026-10-26

Every house gate ran offline on framing commit 9d52f53 using the verified
candidate-local engine and pinned script owners. No integration or renumbering
was performed. The gate batch's workspace response timed out after 300 seconds;
the subsequently read completion marker, full status file and saved logs prove
that all commands finished. A transport timeout alone was not counted as proof.
Logs and per-command exits are retained locally under .forge/intake151-integration
(plain path, not a tracked link to delivery state).

| Check | Exit |
|---|---|
| Engine pin / shim integrity | 0 / 0 |
| Root fmt / locked all-target clippy | 0 / 0 |
| Root locked workspace tests | 101 |
| Candidate-local brokkr compile | 0 |
| Exact coverage / coverage selftest | 1 / 0 |
| Publish guard / keyless seal proofs | 0 / 0 |
| License check / fail-closed native-tls absence | 0 / 0 |
| Design selftest / delivery tamper proof | 0 / 0 |

The root failure is the existing wrapper proposal's missing index row in
store_contract_docs: every_decision_file_has_exactly_one_resolving_index_row
and the_inventory_refuses_missing_dangling_duplicate_and_malformed_entries.
Coverage refuses because its instrumented tests encounter the same failure;
there is no exact-coverage verdict. These are outstanding implementation
repairs, not waived gates. The standalone eval proofs and dry-run above remain
required after integration; this intake does not re-derive their prior receipts.
This follow-up changes only framing wording and adds the observed receipt.
