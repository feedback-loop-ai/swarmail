# Launch the three eval CLIs

Date: 2026-10-26. Issue #151 part 1 continuation, epic #164.
Design only; implementation and operator smoke remain outstanding.

Decision: 0083 — [CLI wrapper and surface contracts](../../docs/decisions/0083-cli-wrapper-launch.md), proposed.

## Feature and rationale

Replace the unconditional refusals in the three existing agent entrypoints and
in baseline_execute with launches of the explicitly selected Claude Code,
Codex and dsh executables. Preserve the proven fixture, free controls, sequential
matrix, scoring and cleanup. The supplied versioned contracts now permit
assembly to be proven offline; they do not establish live provider behavior.

The operator's 2026-10-26 directions settle authentication and unequal tools:
use the selected existing logins, apply every measured restriction, refuse
unexcludable managed configuration, and disclose each family's remaining tool
surface in every row and the report. Tool asymmetry itself is not a refusal.
This supersedes the earlier missing-contract/equal-surface assumptions in
[the earlier design](../baseline-wrappers/spec.md) and
[wrappers.md](../../evals/wrappers.md), not their control or privacy guarantees.
The [continuation intake](../../evals/issue151-continuation-intake.md) and
[original intake](../../evals/issue151-intake.md) retain their dated receipts.

The issue151 README, operator-rulings and every CLI-contract capture passed the
packet SHA256SUMS. README SHA-256 is
be74941231e61014f3f4caa0803713901a876d28a91f514b2e29febde76d5df9;
operator-rulings SHA-256 is
fee1252df8b6a3bb8b3693155fc5f94d123f2e5488652b744112e3d8160718e1.
Run-local evidence locations are plain provenance text, never tracked Markdown
links: .forge/epic192/issue151/cli-contracts and
.brokkr/state/design/positions/{simplicity,robustness}.md. The packet's prior-run
section is empty; the supplied continuation and local intake establish the
54c8240 IMPL-BLOCKED stop. No missing historical outcomes are invented.

## Scope and authority

Implementation stays in evals/ and its documentation. These three design files
and the required proposed decision are the design-phase artifacts explicitly
requested by the sequence. Preserve accepted [0055](../../docs/decisions/0055-agent-eval-harness.md)
and [0071](../../docs/decisions/0071-eval-trap-scoring.md), including their current
integration, and shipped provenance/privacy/lifecycle/sequential behavior
recorded by proposed 0063/0073/0075/0076. A proposal is not acceptance.

No root runtime, manifest/lock, gate, license policy, bundle/recipe, scoring,
mail extraction or protocol changes. Synchronous accept/store (0001),
subscribe-before-check (0002), oldest-only pruning (0003), real-wire tests
(0004) and ingest extraction (0005) remain intact. No new provider SDK,
retry/fallback, effort override, installer, broker, transcript harvester,
namespace, supervisor or dynamic adapter framework.

## Synthesis: adopted, rejected and reconciled positions

| Choice | Ruling and explicit cost |
|---|---|
| Implementation shape | Adopt simplicity's three fixed builders in wrappers.rs and robustness's typed preparation/private admission. Reconcile these within existing owners, without a new production module or executable. Cost: each new CLI recipe requires reviewed code and permanent tests. |
| Historical rows | Choose simplicity's required surface field and strict refusal over robustness's optional old-v2-to-unknown reader. This matches current required provenance/concurrency practice. Cost to the losing compatibility option: older rows need their historical binary; no migration or mixed report. Result/catalogue/snapshot remain version 2. |
| Claude allowances | Choose robustness's explicit MCP tool names over simplicity's whole-server matcher. Whole-server matching is marked unmeasured in the packet. Cost: maintain a reviewed list and a real tools/list parity proof when the server catalogue changes. |
| Helper credentials | Choose robustness's explicit env-clearing exec boundary over reliance on per-server env maps or dsh scrubbing. Cost: one small internal subcommand and real effective-environment proof, within the existing executable. No second lifecycle owner. |
| Auth layout | Adopt simplicity's closed JSON selection and dsh key records; reject the optional dotenv input considered by robustness. Adopt both positions' reduced Claude/Codex projection without real refresh tokens. Cost: operator prepares/selects the documented private records and renews expired logins; no automatic refresh or directory import. |
| Lifecycle | Choose simplicity's Unix exec after setup, satisfying robustness's same-group/retained-owner requirement. Reject a wrapper wait loop or second timer. Cost: all private-state lifetime and descendant completion remain the runner's responsibility. |
| Residual tools | Adopt both positions and the ruling: apply measured controls, retain Codex code-mode access, disclose native tools/connectors and dsh unmeasured removal. Reject speculative disable matrices, dsh plugins and equal-surface refusal. Cost: results compare different configured tool surfaces; neither fresh HOME nor a profile proves confinement. |
| Proof boundary | Adopt robustness's complete canary/descendant/strict-row proofs and simplicity's existing test owners. Stand-ins prove assembly, never provider support. Cost: raw CLI debugging output is unavailable and live usability remains the operator's first smoke. |

## Required behavior

The existing public entrypoint form remains
`agent-{claude,codex,dsh} --model MODEL --config ABSOLUTE_PRIVATE_FILE`.
No arbitrary argv/env/tool/provider override is introduced. Exact model set:

| Family / evidenced CLI | Approved models | Goal / credential channel |
|---|---|---|
| Claude Code 2.1.310 | claude-opus-5-5; claude-sonnet-5-5 | Unchanged stdin; minimal OAuth file under private HOME |
| Codex 0.164.0 | gpt-6.1-sol; gpt-6-luna | Unchanged stdin with final `-`; reduced ChatGPT auth.json under private HOME |
| dsh 0.1.5-rc.1 | dashscope/glm-5.3; spark-glm/GLM-5.3-Flash-EXL3 | One literal final argument; only the selected route key in child environment |

All eight existing EVAL inputs and exact prompt-file equality stay mandatory.
dsh refuses NUL, a leading `-`, and goals exceeding 65535 UTF-8 bytes before
launch; its single argument includes a terminating NUL below the evidenced
Linux argument limit. Claude/Codex retain the existing 1 MiB prompt bound.
No goal rewriting or appended Codex discovery notice is authorized.

A version-1 closed private manifest has exactly `version` and `families`;
`families` has exactly `claude`, `codex`, `dsh`, each with `executable` and
`credential_file`, both absolute paths. Only the selected family's credential
file is opened. No secrets occur in the manifest. Each private input is bounded
to 65536 bytes while reading, not merely by metadata. Descriptor validation
refuses symlinks, nonregular files, wrong effective owner, group/other access,
malformed/duplicate/unknown fields, unsupported versions and unsafe traversal.
Diagnostics are fixed categories, never rejected values or paths.

Selected credential records:

- Claude: exactly `claudeAiOauth` containing nonempty `accessToken`, integer
  `expiresAt`, and `scopes:["user:inference"]`. Project these three fields to
  `$HOME/.claude/.credentials.json`. This is a selected minimal record from the
  existing OAuth login, not a new API-key account or a directory copy.
- Codex: the captured ChatGPT auth.json fields `auth_mode:"chatgpt"`,
  `OPENAI_API_KEY:null`, `tokens` (nonempty id_token, access_token, refresh_token,
  account_id), and string last_refresh. Copy only id/access/account tokens;
  replace refresh_token with an inert fixed placeholder and last_refresh with
  copy time, following the measured reduced-login recipe. Never update the
  source or forward its real refresh token.
- dsh: exactly nonempty `DASHSCOPE_API_KEY` and `SPARK_API_KEY` strings in one
  JSON object. Only the selected value enters its named child environment
  variable. Do not source .env or carry DEEPSEEK_API_KEY/other routes.

New directories are 0700 and files 0600 before payload, exclusively below the
runner-owned private trial HOME. dsh's source key may be used directly in child
env; any credential-bearing staging file must obey the same HOME/mode rule.
Secret-bearing structures have no Debug/public serialization. Selected source
files stay untouched; the runner disposes generated state after owned cleanup.

The versioned tool-surface descriptor is required at provenance.agent.tool_surface.
It describes declared configuration, not an observed provider inventory or a
successful launch. Canonical finite profiles include builtin/not-applicable,
external/unknown, invalid-request and the three versioned CLI profiles. Each
wrapper profile includes explicit restrictions and conservative residuals,
not merely an opaque profile name. It is present even on setup failures and
never-started suffixes; it never asserts those restrictions were executed.

Claude records empty builtins, strict explicit MCP, named tool permissions,
restricted settings and disabled skills, plus nearby-version/live-policy
uncertainty. Codex records read-only, disabled web search, ephemeral/ignored
user config/rules, plus native reading/edit-tool availability, code-mode and
ChatGPT account connectors whose removal is unproved. dsh records fresh home,
overlay-only servers and disabled telemetry, plus native tools, including
shell/file/web/subagent capabilities whose removal is unmeasured and plugin
composition uncertainty. Every profile retains the common unconfined/same-uid
limit. Absent or malformed metadata refuses current result readers. Public
selection is a finite profile ID bound through assigned contexts; labels or
CLI output never establish a profile. Generic external commands stay unknown.
Reports preserve distinct profiles and state per-family asymmetry without
changing score arithmetic. Private argv remains private and unhashed.

## Acceptance Criteria

1. **AC1 — Real launch assembly.** For all six models, real wrapper execution
   into a local stand-in proves exact argv, stdin, cwd, complete env and parsed
   JSON/TOML/YAML against the plan's recipes. Invalid models/eight-input values,
   prompt mismatch and dsh bounds cause zero launches. Tests include literal
   spaces, quotes, newlines, Unicode and shell metacharacters; no shell executes
   the goal. Both blanket missing-contract refusal sites are replaced.
2. **AC2 — Configuration exclusion.** Offline fixtures prove each recipe's
   restrictions and prelaunch ancestry refusal. Claude refuses presence or
   indeterminate inspection of /etc/claude-code; Codex does the same for
   /etc/codex, including dangling symlinks. Internal test roots exercise these
   checks without changing host /etc or providing a production bypass. dsh
   native tool removal stays unclaimed where evidence is only inference.
3. **AC3 — Private authentication.** Bounded descriptor-based admission tests
   cover unsafe mode/owner/parent, symlink, directory/FIFO, malformed/duplicate/
   unknown fields and limit boundaries. Stand-ins verify selected-file-only
   access, source immutability, exact private projections, no real refresh token,
   0600 files/0700 directories before payload, and no credential outside HOME
   staging. Each helper starts with a credential-free effective environment.
4. **AC4 — Output privacy.** Synthetic credential canaries emitted by stand-ins
   on success, failed exec/setup, nonzero exit, deadline and either signal
   never appear in argv, safe diagnostics, JSONL, public hashes/metadata,
   retained artifacts or reports. CLI stdout/stderr are null from launch,
   never captured for redaction. Private state survives until cleanup and is
   then disposed; crash-proof erasure is not claimed.
5. **AC5 — Tool-surface evidence.** Producers, strict readers, assigned-context
   validation, refusal/unstarted rows, retention/replay and reports preserve
   the required canonical descriptor. Missing, unknown, duplicate, mismatched
   model/profile or altered restriction/residual values refuse; arbitrary
   external commands and builtins receive honest unknown/not-applicable values.
   Baseline accounting checks the expected six declarations and descriptors.
   Report tests prove asymmetry is visible and arithmetic is unchanged.
6. **AC6 — Owned descendants.** Real stand-ins create acknowledged children and
   grandchildren, including TERM resistance and early leader exit. Normal,
   failed launch/nonzero, deadline, SIGTERM and SIGINT paths prove direct-child
   reaping and observed descendant cessation before scoring/publication and
   admission release. A cleanup-bypass negative control must fail that proof.
   Use readiness/release acknowledgements, no sleep as synchronization.
7. **AC7 — Controls and preserved wire proofs.** Corrupt/missing/extra/duplicate/
   wrong-outcome controls, bad command status or invalid retained evidence cause
   zero paid stand-in launches; valid fresh controls reach stand-ins. Keep real
   SMTP/MCP/HTTP oracle/null, fixture helper identity/decoy/bounds/deadline,
   traps, privacy, provenance, full accounting and sequential-admission proofs.
   Paid failure preserves rows and a valid diagnostic report without retry.
8. **AC8 — Baseline and dry-run.** Actual dry-run shows current 16 tasks, six
   models, 32 controls, 480 requested paid slots, five repetitions, concurrency
   1 and 600000 ms, creating no output directory and reading no credentials or
   launching trials. Keep controls -> strict retention validation -> matrix ->
   report ordering and current exit precedence. Slot counts are not session counts.
9. **AC9 — Documentation and local verification.** Update eval wrapper/setup/
   run/report docs with exact schemas, recipe evidence and limitations; preserve
   dated intake receipts and omit new tracked links into run-local evidence.
   Run every local house gate and standalone eval checks in the plan, recording
   failures honestly. All required gates must pass before whole-feature success;
   known carried failures are not waived. No paid/networked agent is executed.

## Proof ownership and remaining limits

The operator owns the first authenticated smoke for each of the six models,
exact-version behavior, live tool inventories, Claude/Codex HTTP negotiation,
and especially dsh's live HTTP MCP row. Nearby-version measurements are Claude
2.1.287, Codex 0.160.0 and dsh MCP client rc.2; installed captures use Claude
2.1.310, Codex 0.164.0 and dsh MCP client rc.3. Stand-ins cannot close this gap.
The operator also owns paid baseline, publication/reprioritization, live settings,
sealing and decision acceptance. CI owns its charter checks and #201 macOS eval
proof; #221 owns namespace isolation. These are not local acceptance criteria.

Every row stays unconfined. Private HOME prevents inherited state, not malicious
same-uid reads, session escape or forged evidence. Forced runner death can leave
children/private state; such a run is invalid until the operator confirms cleanup.
The carried full-suite lifecycle assertion and #243 absent issue199 authority
link remain explicit verification work/ownership, not fabricated green receipts.

## Dated integration scope — 2026-10-26

The [integration specification](../wrapper-integration/spec.md) now authorizes
merging local main's #243 repair and indexing this proposal as 0083, proposed.
It supersedes this continuation's eval-only integration boundary and its
separately owned index/link work. The launch requirements above are preserved;
no wrapper behavior changes. The [prior receipt](../../evals/issue151-implementation.md)
retains the continuation's failed gates. Fresh combined-candidate gates, including
the full standalone suite and exact coverage, are required before completion.
