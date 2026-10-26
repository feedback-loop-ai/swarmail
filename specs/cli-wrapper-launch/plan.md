# CLI wrapper launch implementation plan

## Technical context

The unpublished standalone Rust crate evals already owns wrapper admission,
fixture-http, baseline planning/controls, strict provenance/results, artifacts
and Unix process-group cleanup. Use its existing serde/serde_json, libc, tokio,
chrono and private-file primitives. No new package, executable, dependency or
lock change is required. Root exact coverage excludes evals: independent locked
eval checks are mandatory. No plan template exists in this checkout; this file
supplies technical context, constitution, structure, research, data model,
contracts, quickstart and risks inline within the three-artifact limit.

## Constitution check

Preserve frozen 0001–0003, real protocols (0004), ingest extraction (0005),
accepted 0055/0071, current v2 versions, scorer/report arithmetic, sequential
admission and complete row accounting. New proposal 0083 records compatible
input/metadata contracts without accepting itself or amending frozen semantics.
Keep current bundles and all proof owners unchanged. No paid/networked agent,
remote/journal/sibling read, installation, push, merge or publication. Only the
operator accepts decisions. Commit unsigned. Design-check success is structural,
not implementation success. Required gates and carried failures are never waived.

## Research resolved and evidence limits

Read both panel positions, original run-local intake and current continuation
intake, decision index/relevant authorities and all three supplied CLI contracts.
The issue packet passed all checksum entries; the spec records authority digests.
Use the captures as data: no invoking provider CLIs to discover settings.

The useful facts are Claude's empty builtins/strict MCP, Codex's per-key merging
and read-only/web-search restrictions, and dsh's insert-row grammar, route rows
and fresh DSH_HOME. Unknown effects remain unknown: whole-server Claude matcher,
Codex feature disables/connector removal, dsh native row disable composition,
exact-version OAuth/provider behavior and HTTP interoperability. No tool-row
disable qualifies as measured in the supplied dsh pack; therefore this revision
adds none of the speculative native-tool disables and records that residual.
Use DSH_TELEMETRY_DISABLED=1, whose effect is package-evidenced.

## Project structure and ownership

| Path | Change |
|---|---|
| evals/src/wrappers.rs | Keep admission; add typed private selection, no-follow bounded reader, ambient preflight, fixed builders, credential projection and exec. |
| evals/src/main.rs | Remove baseline's post-control missing-contract refusal, keep control/retention checks; dispatch the internal fixture-http-isolated exec boundary; propagate profile selections through trial/matrix contexts. |
| evals/src/fixture_http.rs | Preserve single eval_http behavior; share validated eight-input handling with isolated entry. |
| evals/src/provenance.rs | Own finite tool-surface type, canonical profiles and validation. |
| evals/src/runner.rs | Carry declared surface through request/context/provenance and every refusal; retain RuntimeDirs/OwnedGroup ownership. Only narrow accessors or creation helpers if needed. |
| evals/src/results.rs, artifacts.rs | Enforce shared strict descriptor and retained equality/replay; never infer missing surface data. |
| evals/src/report.rs | Optional finite declaration selector; required row validation; expose distinct configured surfaces and asymmetry in report. |
| evals/src/baseline.rs, evals/agents-baseline.json | Bind exactly six public profiles/models, keep private argv false and strict controls/accounting. |
| evals/tests/wrappers.rs | Main assembly, file/ambient refusal, canary, isolated-helper and wrapper-descendant proofs; test-only executable support if needed. |
| evals/tests/baseline.rs, provenance.rs, report.rs, runner.rs and existing binary unit tests | Context/retention/accounting/control/lifecycle integration regressions; adapt only superseded missing-contract assertions. |
| evals/tests/proof.rs and other current suites | Preserve real SMTP/MCP/HTTP, traps, admission and every existing assertion. |
| evals/wrappers.md, README.md, run-plan.md | Replace obsolete refusing-wrapper guidance with exact schemas, recipes, evidence labels, run command and smoke ownership. Preserve dated intake receipts. |

Do not create research/model/contracts files outside the requested artifact set;
the sections below own that material. No production platform or general config
parser is introduced. Test support stays test-only and uses synthetic credentials.

## Preparation and private input contracts

Keep `--model` and `--config` literal. Validate family/model, eight inputs and
prompt bytes, manifest structure, clean runtime/system sources, selected file,
and family goal constraints before executing any CLI. A launch-plan value holds
literal OsString argv, stdin policy, cwd, public profile and private payloads;
private payloads cannot derive Debug or public Serialize. Preserve native paths
through process APIs; the JSON manifest path strings are UTF-8 by definition.

Read manifest/credential files with O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC, inspect
opened descriptors, require regular file, effective-uid owner and no group/other
permission bits. Bound both metadata length and the read to 65536 plus one byte.
Closed typed decoding rejects duplicate/unknown fields at every nested level;
no conversion through an unvalidated JSON map that loses duplicate keys.
Use a directory-descriptor walk for path components, rejecting symlinks and
non-directories. Ancestors must be root/current-uid owned without group/world
write, except the established root-owned sticky /tmp rendezvous followed by
owned private directories; a permissive source ancestor is not made safe by
chmodding the selected file. Use descriptor operations for creation too; no
check-then-reopen payload write. No hostile same-uid/privileged race claim.

New files use exclusive creation and mode 0600 before writing, containing
directories 0700 from creation; reuse existing primitives where they meet this
contract. Do not change global umask, chmod caller paths or copy directories.
Generated state lives below HOME, including Claude mcp.json, Codex auth.json,
dsh seat.yml and private sessions. Keep existing runner-created TMPDIR/XDG
ownership; wrapper files never acquire a separate TempDir cleanup owner.

Use the spec's exact manifest and three auth shapes. Validate nonempty token/key
strings and reject NUL in anything passed to env; expiresAt is a nonnegative
integer representable by the captured format. Do not locally infer token
validity/expiry or contact an auth service. Source Codex last_refresh is parsed
as a timestamp; generated copy time uses UTC RFC3339. Use the fixed non-secret
refresh placeholder `unavailable` and never copy the genuine refresh value.
Expiry becomes an operator smoke/auth failure, never fallback to another login.

Absolute executables are operator selections, not PATH searches. Require an
executable regular target and fixed safe runtime: PATH=/usr/bin:/bin,
LANG=LC_ALL=C, private HOME/TMPDIR/XDG and PWD equal to actual private cwd.
Direct installations or operator-provided launchers must work in that map;
no automatic VOLTA_HOME/Node/PATH inheritance or install/version negotiation.
The selected executable remains trusted; profile version is a declared recipe,
not a measured binary-version assertion. Copy the eight validated EVAL inputs
only where required by the CLI/helper configuration. No proxy, loader, startup,
auth fallback, admin secret or test-control inheritance.

Preflight checks actual cwd ancestors through filesystem root conservatively
(the private non-repository cwd has no nearer git boundary). Reject ambient
.claude, .codex, .agents, .dsh, .mcp.json, AGENTS.md, AGENTS.local.md, CLAUDE.md,
CLAUDE.local.md and .env. Check fresh HOME for unexpected family settings/skills
before generating the owned family subtree. Known own generated config/auth
is permitted only as part of that preparation. System checks use symlink-aware
metadata: any /etc/claude-code for Claude or /etc/codex for Codex refuses,
including empty directories, dangling links and inspection errors other than
proven absence. Tests inject a root only into internal validation; no public
flag/env override can hide real /etc. dsh has no evidenced system source.

## Exact family recipes

All argv entries below are separate literal values, never a shell command.
All CLI stdout/stderr go directly to Stdio::null before exec. Claude/Codex stdin
is the already validated prompt file descriptor positioned at byte zero;
dsh stdin is null. Exec replaces the wrapper inside the runner-owned group;
no setsid/setpgid, wait loop, timeout thread or detached work. Failed exec returns
a finite safe category; the existing owner still disposes runtime state.

### Claude Code 2.1.310

Use `-p --output-format stream-json --verbose --model MODEL
--no-session-persistence --permission-mode dontAsk --permission-prompts none
--tools "" --strict-mcp-config --mcp-config PRIVATE_MCP_JSON
--allowedTools EXPLICIT_COMMA_LIST --restricted --disable-slash-commands`.
Set DISABLE_AUTOUPDATER=1. Do not use --bare (OAuth excluded) or --safe-mode
(required MCP uncertain). No positional prompt follows variadic arguments.

JSON contains exactly mcpServers.swarmail (type http, EVAL_MCP_URL, identity
header) and mcpServers.fixture (type stdio, absolute current eval executable,
args [fixture-http-isolated], validated helper env). The allowed list is
mcp__fixture__eval_http plus mcp__swarmail__ followed by each current server
name: swarmail_search_emails, swarmail_get_email, swarmail_get_latest_email,
swarmail_delete_email, swarmail_clear_inbox, swarmail_clear_all,
swarmail_wait_for_email, swarmail_extract_links, swarmail_extract_codes,
swarmail_seed_email, swarmail_set_chaos, swarmail_clear_chaos. A real tools/list
parity test guards this explicit inventory; this is no narrower permission
claim about individual mailbox operations. OAuth lives only in the minimal
$HOME/.claude/.credentials.json. Server-delivered policy remains unmeasured.

### Codex 0.164.0

Use `exec --json --model MODEL -C PRIVATE_CWD --skip-git-repo-check
--ephemeral --ignore-user-config --ignore-rules --sandbox read-only`, followed
by per-key `-c` values and final `-`. CODEX_HOME=$HOME/.codex contains only the
reduced auth.json before launch. No provider-key env, -o output file or effort.

Overrides: web_search="disabled";
projects."PRIVATE_CWD".trust_level="untrusted" (defense in depth, override effect
unmeasured); mcp_servers.swarmail.url and http_headers (identity),
startup_timeout_sec=20, tool_timeout_sec=60;
mcp_servers.fixture.command (absolute eval executable), args
["fixture-http-isolated"], env (eight inputs plus fixed runtime map), cwd;
and default_tools_approval_mode="approve" on both server entries. Quote keys
and TOML values independently. Do not use whole-table mcp_servers replacement:
it merges. Do not disable code-mode_host/exec, apps or a speculative matrix of
features. Read-only restricts writes; it neither removes native edit tools nor
prevents same-uid reads. Account connectors remain recorded residuals.

### dsh 0.1.5-rc.1

Use `--profile headless --patch PRIVATE_SEAT_YML GOAL`, exactly one final goal
argument. DSH_HOME=$HOME/.dsh, DSH_TELEMETRY_DISABLED=1; export only the selected
DASHSCOPE_API_KEY or SPARK_API_KEY. Fresh home contains no home/profile patch,
credentials.yaml, .env, skills or plugin bundle additions.

The one YAML patch list restates llm-pi-ai.config.providers for the selected
route (the other route is absent), then agent-default-model.config with both
provider and model, then session-persistence-jsonl.config with root below HOME,
compression:none and packChunks:false. Insert both @deepseek-ai/dsh-mcp-client
rows through `- insert:`: mcp-swarmail uses serverName:swarmail,
transport:streamable-http, EVAL_MCP_URL, identity header, failOnStartupError:true,
toolCallTimeoutMs:60000; mcp-fixture uses serverName:fixture, transport:stdio,
absolute eval executable, args [fixture-http-isolated], validated env, private
cwd and failOnStartupError:true. No unknown-id-only server patch.

Route constants from the supplied contract:

| Provider | API and endpoint | Model/options |
|---|---|---|
| dashscope | api:openai-completions; apiKeyEnv:DASHSCOPE_API_KEY; baseURL:https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1; displayName:Model Studio (Token Plan) | glm-5.3; compat:{thinkingFormat:deepseek,supportsDeveloperRole:false} |
| spark-glm | api:openai-completions; apiKeyEnv:SPARK_API_KEY; baseURL:http://spark:8888/v1; displayName:DGX Spark vLLM (GLM) | GLM-5.3-Flash-EXL3; contextWindow:200000; maxTokens:32768 |

Keep the captured spark-glm compaction-basic modelPolicies entry with
thresholdRatio:0.8 and retainRatio:0.16, restating its captured key structure;
no arbitrary task expressions or !!js. No effort or guessed native-tool
removal. LAN spark's HTTP endpoint is the operator-supplied route, not a new
TLS bypass. No provider is contacted during delivery.

## Helper boundary and serialization proof

Add `fixture-http-isolated` as an internal subcommand of the same executable.
Read only the validated eight EVAL values and safe runtime map, then exec the
same absolute executable with `fixture-http`, env_clear and that explicit map.
Preserve protocol stdin/stdout and group; suppress raw helper error leakage
through existing finite diagnostics. This boundary is mandatory even for dsh,
whose scrub behavior is evidenced. Prove it with deliberately contaminated
synthetic parent env and an effective-child-map observation seam. It does not
prevent malicious same-uid reads of private files.

Use serde_json for JSON. Fixed TOML/YAML encoders support only the needed scalar,
array and object forms with format-correct key/string escaping. Validate against
independent expected bytes and test-only decoding of that finite subset; do not
validate an encoder with itself. Golden fixtures derive from the packet schemas,
covering quotes, newlines, controls, Unicode, spaces and shell metacharacters.
Live installed-CLI parsing/negotiation remains smoke, not a stand-in claim.

## Data model, baseline and reporting

Add a required ToolSurface in AgentIdentity with a closed tagged representation.
Wrapper variant has revision:1, basis:declared_configuration, profile enum
claude_2_1_310/codex_0_164_0/dsh_0_1_5_rc_1, and canonical finite restriction and
residual arrays matching the spec. Other variants are builtin_not_applicable,
external_unknown and invalid_request. Validate exact canonical sets, without
duplicates, unbounded free text or private paths; unsupported fields/revisions
refuse. Version labels describe captured recipes, never runtime attestation.

AgentSpec gains optional wrapper_profile with the finite three-profile enum;
omitted generic external declarations resolve to external_unknown. Builtins
cannot override it. Trial/library/assigned worker requests carry the same
selection; public trial selection, if needed, uses the same enum rather than
free-form descriptors. Validate family/model agreement and prevent a declaration
from being upgraded based on label spelling. A profile is operator-declared,
not proof of executable authenticity. Baseline constructs the trusted wrapper
argv itself, matches the six expected model/profile pairs and keeps
public_argv:false. Invalid requests use invalid_request; admitted but unstarted
requests retain their declared profile with basis:declared_configuration.

Use one constructor/validator in provenance for every normal/refused/setup/
retention/cancellation row. Extend existing worker-context equality, matrix
plan/index validation and retained replay checks. Missing fields, old rows,
unknown fields, model/profile disagreements and noncanonical arrays refuse.
Do not default old rows to restricted or unknown. Keep all current versions,
seed/time/target identities, privacy hashes and 64 KiB row/capture bounds.

Reports expose distinct profile descriptors per existing aggregation group and
an explicit asymmetry statement. Mixed profiles remain visible, not silently
summarized as one restricted toolset. Existing attempts, infrastructure counts,
Wilson success rates, failure classes, median/p95 and absence of token estimates
remain unchanged. Keep unknown external commands supported, honestly labeled.

Remove require_cli_contracts and its two call sites only after builders exist.
The baseline still opens no credentials: it supplies the private locator after
fresh controls and retained validation. Valid controls proceed through the
existing matrix owner; invalid controls cause zero CLI launches. Unit fault
seams can reduce execution for orchestration proofs but must not weaken the
actual fixed full plan. Keep six models, all v2 tasks, five trials, concurrency
1, 600000 ms and report generation after valid failed paid rows. No retry/filter.

## Quickstart and operator handoff

Implementation documents the manifest shape with inert placeholders:

```json
{"version":1,"families":{"claude":{"executable":"/opt/claude","credential_file":"/private/claude-oauth.json"},"codex":{"executable":"/opt/codex","credential_file":"/private/auth.json"},"dsh":{"executable":"/opt/dsh","credential_file":"/private/dsh-keys.json"}}}
```

The operator selects private files in the spec's schemas and executables that
work in the fixed environment. All inputs must pass ownership/type/mode checks.
No delivery seat reads actual login values. Build/check offline:

```bash
CARGO_NET_OFFLINE=true cargo build --manifest-path evals/Cargo.toml --locked
CARGO_NET_OFFLINE=true cargo run --manifest-path evals/Cargo.toml --locked -- baseline --config /private/wrappers.json --output /tmp/issue151-dry-run-only --dry-run
```

Dry-run reads no private file and launches no trial; assert its output cardinality
and absence of output directory. The future operator command is the same
baseline invocation without --dry-run and with a fresh private output path.
Document smoke first, output/retention paths and exit meanings. Six-model live
smoke and live dsh HTTP MCP remain outstanding operator evidence. Baseline
publication/reprioritization is not achieved by installing wrappers. Raw provider
transcripts/tokens/costs are not available from this implementation and must not
be fabricated; the larger baseline publication work retains its separate owner.

## Verification strategy

Use existing Cargo targets; real protocol proofs remain real servers. Stand-ins
are real local executables for assembly, failures, env, noisy synthetic canaries
and child/grandchild lifecycle only. Reuse the runner's readiness/Pending/release
seams and observed-cessation proof; no sleeps or test-only public bypasses.
Test normal leader exit with live descendants, failed exec, nonzero, deadline,
both signals and TERM-resistant children. Prove admission/state lifetime and
require a cleanup-bypass negative control to fail. Preserve assertion strength.

Run offline, install nothing, retain every command/exit even after a failure.
The charter remains the one inventory; execute pinned owners after shim checks.
The pinned candidate-local engine digest must equal
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.

```bash
export CARGO_NET_OFFLINE=true
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
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked
cargo test --manifest-path evals/Cargo.toml --locked --test wrappers
cargo test --manifest-path evals/Cargo.toml --locked --test baseline
cargo test --manifest-path evals/Cargo.toml --locked --test proof
```

Also execute verify-seat.sh's fail-closed native-tls absence proof: cargo tree
-i native-tls must fail specifically with "did not match any packages"; success
or a different failure is not absence. Exact coverage means zero missed lines,
branches and functions, plus a passing selftest. CI owns the charter's external/
platform jobs; independent eval tests are not covered by root 100%.

The chief checks the final design HEAD with
`bash bundles/self/scripts/design-check.sh check .` after committing exactly
spec/plan/tasks and proposed 0083. This local check does not replace the pinned
sequence check or its engine verdict. Leave the design commit last, paths clean,
positions uncommitted and result in the designated ignored engine file.

## Risks

| Risk | Mitigation and accepted limit |
|---|---|
| Exact-version behavior differs from nearby live evidence | Pin documented recipes and mark source basis; offline stand-ins prove assembly only. Operator first smoke covers all models/HTTP, especially dsh. No automatic fallback. |
| Tool lists differ across families or account policy adds tools | Canonical conservative per-row surfaces and explicit report asymmetry; managed config refusal; no claim of runtime inventory, equal tools or confinement. |
| Credential refresh rotates operator login | Project no real refresh tokens, never write source, test canaries and source byte equality. Expiry requires operator renewal. |
| Environment or config reintroduces secrets | Descriptor reads, closed schemas, private files before payload, null output and isolated helper exec. Same-uid/privileged attacks remain outside scope. |
| Native runtime launcher needs host state | Explicit absolute launcher compatible with fixed PATH; document prerequisites. No VOLTA_HOME inheritance, installer or silent runtime discovery. |
| Serialization accepts literal-looking but wrong configuration | Independent golden/finite-decoder probes against packet schemas for each model; real tool-list parity; live composition remains operator-owned. |
| Private files vanish before descendants stop | Exec preserves runner group identity; runtime/admission held through existing cleanup; acknowledged canary/cessation and bypass tests. SIGKILL/session escape retain existing limits. |
| Required surface field breaks prior v2 readers/rows | Proposed 0083 and explicit current strict-reader contract. Use historical binaries for history; no business version/scorer changes or silent migration. |
| Existing full-suite lifecycle assertion intermittently fails | Reproduce pending_matrix_decisions_observe_queued_signals_and_retain_spawned_children with the full suite, retain failure evidence, repair only demonstrated eval-local defect and reprove. An isolated pass cannot erase the recorded failure. |
| Missing issue199 authority link breaks boundary suite | #243 owns tracked authority repair; absent path is .forge/epic192/issue199/operator-rulings.md. Do not fabricate a packet, weaken the test or edit unrelated root docs. Record current failure and withhold whole-feature green until owner resolves it. |
| Proposed decision absent from index after exact four-file chief commit | Index integration is documentation/operator-owned outside this implementation scope. No index/acceptance claim; any resulting inventory failure remains outstanding. |
| Old live docs contradict supplied ruling | Update current eval docs in implementation with dated supersession, preserving intake receipts. New tracked links target committed docs only. |

## Dated integration scope — 2026-10-26

The [integration plan](../wrapper-integration/plan.md) supersedes the earlier
index-owner and absent-#243-link restrictions above. This delivery performs the
explicitly authorized local-main merge, proposal renumbering to 0083 and proposed
index row, then reruns every required proof. Historical risk/failure descriptions
are retained; they do not restrict the newly authorized documentation repair.
Wrapper code, tests, runtime contracts and current recipe routing are preserved.
