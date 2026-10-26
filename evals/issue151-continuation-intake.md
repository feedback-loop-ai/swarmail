# Issue #151 part 1 continued — launching wrappers (intake)

Run: `epic-164-issue-151-part-1-contin-7f07ebde`.
Phase: intake; framing only. This record adds no wrapper implementation.

## Goal

Replace the three refusing eval entrypoints with evidence-based launches of the
operator-approved CLIs, proven offline for configuration, private credentials,
recorded tool surfaces and owned cleanup, so the operator can perform the first
live smoke and then the existing control-gated baseline.

## Authority and orientation

Read the house charter, decision index, accepted 0055/0071 and the current
provenance/lifecycle/privacy/sequential contracts described by proposed
0063/0073/0075/0076. Those proposals remain proposed. Frozen guarantees
0001 (synchronous accept/store), 0002 (subscribe before check) and 0003
(oldest-only pruning) are not changed; existing real-server proofs remain
required. This is compatible wrapper work under the supplied operator ruling,
not a request for new semantics or a decision acceptance.

The supplied issue151 packet passed every entry of its SHA256SUMS from the
packet directory, including README.md
`be74941231e61014f3f4caa0803713901a876d28a91f514b2e29febde76d5df9`
and operator-rulings.md
`fee1252df8b6a3bb8b3693155fc5f94d123f2e5488652b744112e3d8160718e1`.
The current ruling supersedes the earlier intake's authentication/tool-surface
assumptions. Evidence is local at `.forge/epic192/issue151/cli-contracts/`:
README.md and the versioned claude.md, codex.md and dsh.md distinguish help,
config probes, nearby-version measurements and inference. Do not promote an
inference or offline stand-in result into a provider-behaviour claim.

History spot-check: 54c8240 supplied the plan, control gate, fixture-http helper
and refusing entrypoints; its outcome was IMPL-BLOCKED, with missing contracts
listed in [wrappers.md](wrappers.md). Those contracts and operator rulings are
now supplied. The packet README has an empty prior-runs section; the task's
provided continuation account and existing local prior result establish the
stop, without reading any journal or remote source.

Code spot-check: `wrappers::run` validates model/config locator/eight inputs
then calls `require_cli_contracts`; `baseline_execute` independently calls that
refusal after strict controls and retained-control validation. Both refusal
sites must be addressed without bypassing controls. `RuntimeDirs::apply`
clears the environment. `evals/tests/wrappers.rs` already proves admission,
secret-free refusal and real loopback identity-bound fixture HTTP; baseline,
proof, runner, boundary, provenance, scoring and report suites remain owners.

## Scope

Change only `evals/` and its documentation. Keep the current candidate bundle
routing, protected review, gates, dependencies, root runtime and frozen
semantics. No fallback models, effort overrides, broker, namespace isolation,
new supervisor or alternative scoring. The existing [intake](issue151-intake.md)
and [run plan](run-plan.md) retain the already delivered baseline requirements;
the criteria below govern this continuation and supersede the former blanket
refusal for unequal tool surfaces. Every row remains unconfined.

## Acceptance criteria

1. All three entrypoints launch a selected executable after valid admission,
   rather than returning `wrapper_cli_contract_missing`. Offline stand-ins
   assert the complete literal argv, stdin bytes, cwd, child environment and
   generated configuration for all six exact models: Claude
   `claude-opus-5-5`, `claude-sonnet-5-5`; Codex `gpt-6.1-sol`, `gpt-6-luna`;
   dsh `dashscope/glm-5.3`, `spark-glm/GLM-5.3-Flash-EXL3`. No substitution,
   fallback or effort setting. Preserve invalid-model/eight-input/prompt
   refusal before launch. Claude/Codex goals use stdin; dsh uses one literal
   argv element and the overlay model row, never a fictitious --model flag.
   Unsupported dsh goal sizes/option-shaped inputs refuse before launch
   unless the supplied evidence establishes a literal transport for them.
2. Stand-ins parse and compare the exact two-server configuration against the
   versioned contracts: Swarmail HTTP MCP at EVAL_MCP_URL and the stdio
   `fixture-http` helper, with the intended app URL and X-Eval-Identity inputs.
   Assert JSON/TOML/YAML quoting with spaces, quotes, newlines, Unicode and
   shell metacharacters, and exact route/provider rows for dsh. Keep existing
   real loopback helper tests: exactly one eval_http tool, identity on every
   request, no arbitrary destinations/identity overrides, bounded responses
   and deadline handling. A stand-in proves assembly, not live MCP negotiation.
3. Apply every measured restriction and record the residuals instead of
   refusing merely because families differ. Assert Claude --tools empty,
   --strict-mcp-config and the two-server --allowedTools restriction; Codex
   read-only sandbox, disabled web search, ephemeral state, ignored user config
   and rules; dsh fresh DSH_HOME, servers only in its overlay, disabled tool
   rows only where proven. Tests distinguish applied switches from unmeasured
   effects. Assert cleared ambient environment and fresh HOME/XDG state,
   private working-directory/ancestry checks and the evidence's ambient-source
   exclusion/refusal inventory. Simulated system roots/config fixtures prove
   refusal for unexcludable managed configuration under /etc/claude-code and
   /etc/codex before executable launch; absence permits launch. Do not require
   modifying the host's /etc or add speculative restrictions as new authority.
4. Credentials come only from one explicitly selected owner-only private file
   per family: Claude OAuth token, Codex ChatGPT auth.json and dsh route keys.
   Design defines and tests a bounded closed input layout based on that evidence.
   No copying entire login directories, ambient credential fallback or API-key
   substitution for the selected Claude/Codex logins. Tests refuse unsafe
   ownership/permissions, symlinks, nonregular, malformed and oversized inputs
   before launch, without echoing their values. Credential-bearing generated
   files are inside the private trial HOME, 0600 before payload, with private
   0700 containing directories. Only the selected credential reaches its
   evidenced CLI auth channel; no credential reaches the fixture helper.
5. Synthetic credential canaries emitted noisily by stand-ins never appear
   in argv, rows, public metadata, retained evidence/logs or reports. Assert
   raw CLI stdout/stderr are discarded on success, child failure, setup
   refusal, deadline and interruption. Secret/private paths are not public
   configuration identifiers. Assert private credential/config state disposal
   through existing owned cleanup. Deliberately hostile same-uid file access
   is outside the unchanged unconfined contract.
6. Every emitted/retained result row records a credential-free tool-surface
   description with restrictions and remaining access, including Codex native
   reading/edit tooling and ChatGPT connectors where removal is unproved,
   and dsh native tools whose removal is unmeasured. Test row production,
   strict reading, refusal/unstarted accounting, retention/replay and report
   propagation; the baseline report explicitly states per-family asymmetry.
   Keep catalogue/snapshot/business scorer versions and scoring/report
   arithmetic; design chooses the narrow typed metadata integration. Neither
   absent measurements nor switches alone justify a claim of two-tools-only.
7. Real stand-in processes exercise wrapper -> CLI -> descendant cleanup for
   normal completion, failed launch/exit, deadline, SIGTERM and SIGINT,
   including a TERM-resistant descendant. Readiness/acknowledgement and actual
   cessation/reaping assertions must fail if cleanup is bypassed. Reuse the
   current runner's process group owner and cleanup budgets; admission stays
   held through cleanup. No sleep-based synchronization or competing timer.
8. Preserve every existing proof's assertion strength, adapting only assertions
   whose old purpose was the now-replaced missing-contract refusal. The
   baseline remains controls -> strict validation/retention -> six-model paid
   matrix -> report, concurrency 1, 600000 ms, five trials per cell. Offline
   stand-ins prove continuation past valid controls and zero CLI launches for
   invalid controls. Run the actual dry-run and assert 32 control and 480 paid
   slots for the present 16-task catalogue, without trial/provider execution.
   Preserve all v2 trap, provenance, privacy, retention, complete accounting,
   sequential admission and real SMTP/MCP/HTTP oracle/null proofs.
9. Update wrapper/setup/run/report documentation to describe the supported
   versions, selected-file schema, all restrictions and residuals, and exact
   operator command. Mark authenticated smoke with each of the six models,
   live CLI interoperability and the live dsh HTTP MCP row as the operator's
   first smoke proof, explicitly outstanding until performed. Delivery executes
   no paid or networked agent and invents no tokens/transcripts/costs. Baseline
   publication and epic reprioritization remain operator-owned issue #151 work.

## Verify gate commands

Run offline (`CARGO_NET_OFFLINE=true`), install nothing, and record each exit
and log even if another gate fails. Verify the engine digest exactly equals
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Check public shim digests, then execute pinned proof owners (0050), not the
public dispatch shims. The inventory is the house charter's; these commands
instantiate it for this candidate, with its additional pinned self-proofs:

```bash
sha256sum .forge/bin/brokkr
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
```

Coverage must be exact zero missed lines, branches and functions, and its
selftest must pass. Also execute verify-seat.sh's fail-closed absence proof:

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then exit 0
else exit 1; fi
```

The standalone crate needs its own checks; root green cannot substitute:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked
cargo test --manifest-path evals/Cargo.toml --locked --test wrappers
cargo test --manifest-path evals/Cargo.toml --locked --test baseline
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo run --manifest-path evals/Cargo.toml --locked -- baseline --config /private/wrappers.json --output /tmp/issue151-dry-run-only --dry-run
```

All new offline wrapper proofs belong to the wrappers test target or another
explicitly documented Cargo target run by the full suite. No benchmark is
needed: this changes launch correctness, not a performance contract.

Known carried failure: the prior full eval suite stopped at
`boundary::sequential_contracts_and_decision_links_are_honest` on the absent
`.forge/epic192/issue199/operator-rulings.md` linked by the root spec and
accepted 0055. #243 owns tracked authority repair outside evals. Do not invent
that evidence, weaken the test, change out-of-scope files or report a green
full suite while it fails. Run it and retain the exact current failure and
ownership; this is not missing wrapper evidence or grounds to refuse framing.

CI owns root macOS/chart/Docker/RustSec/port-80/network receipts; #201 owns
macOS eval gates and #221 namespace isolation. Operator owns live settings,
first authenticated smoke, paid baseline/report publication/reprioritization,
sealing and decisions. They are not additional local acceptance criteria.
No remote/journal/sibling-worktree reads, push, merge, tag, publication,
issue closure or acceptance is authorized. Commit framing unsigned before
returning. Intake receipts record checks of this framing candidate only;
future seats must rerun gates for their implementation.

## Intake result

Resolved: the supplied contracts and explicit rulings remove the former
framing prerequisites. This is an actionable recorded task, not a claim that
wrappers or provider interoperability have been implemented or proven.

## Observed intake checks — 2026-10-26

The commands above ran offline against framing commit `4aaeee0`; this follow-up
commit adds only this receipt. All house gates passed: root fmt, locked clippy
and tests, digest-checked local-engine compile (zero unruled valuations), exact
100/100/100 coverage and its selftest, publish guard, keyless seal and license
check. Shim integrity, fail-closed native-tls absence, design selftest and
pinned delivery tamper proof also passed. Independent eval fmt/clippy and the
wrapper, baseline and real-server proof targets passed. The actual dry-run
passed; its parsed plan has 16 tasks, six models, 32 control rows, 480 paid
slots, concurrency 1 and 600000 ms, and created no output directory.

The full standalone eval suite exited 101 at
`tests::pending_matrix_decisions_observe_queued_signals_and_retain_spawned_children`
(`evals/src/main.rs:3096`): its assertion that every row is a zero-time
harness-error refusal with no agent exits failed. The isolated exact-test
recheck passed; that does not erase the full-suite failure or establish its
cause. No implementation/test assertion was changed. Later seats must retain
this receipt and resolve/reprove the failing lifecycle assertion before a green
whole-feature verdict.

A separate boundary target exited 101 at `evals/tests/boundary.rs:85`, confirming
`specs/agent-eval-harness/spec.md` links to the absent issue199 operator-rulings
packet. This is the carried #243 limitation described above. The full suite did
not reach that target in its failed invocation. Current results are not a green
full-eval receipt. Detailed commands and logs are run-local under
`.forge/intake151-continued/`; no paid/networked agent was executed.
