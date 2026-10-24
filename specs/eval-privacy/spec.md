# Eval privacy — Epic #192 / issue #196

Date: 2026-10-24. Recorded-gap repair design for run
`continue-epic-192-issue-196-from-119060a2`; implementation, candidate verification
and protected review remain downstream.

Existing proposal: 0062 — [Closed eval environments and safe diagnostics](../../docs/decisions/0062-eval-privacy.md), proposed; only the operator accepts.

## Feature and rationale

Keep caller credentials and unrelated environment values out of eval children,
and arbitrary subprocess text out of eval diagnostics. These are independent
boundaries: a child can print a secret from a file even with an empty environment.
The original fault was inherited arbitrary variables, raw output tails in
artifacts/errors, and Cargo/matrix stderr and path-bearing errors bypassing
collection. Commits cf3aa2f and c232602 implement that boundary. Recovered review of
f6dbbb53 identified execution and proof gaps; this continuation repairs those
gaps while preserving the reviewed environment and publication policy.

Construct closed environments and drain then discard arbitrary child output.
Keep harness-authored failure facts, real exit statuses and secret-free snapshots.
Preserve literal external commands, eight agent inputs, scoring, cleanup,
schema 1 and `confinement:"unconfined"`.

This continuation reads intake
`.brokkr/state/tasks/continue-epic-192-issue-196-from-119060a2.md`, both earlier
intake records, both `.brokkr/state/design/positions/{simplicity,robustness}.md`,
and all five reports plus feature/issue/manifest under
`.forge/epic192/issue196/recovered-review-evidence/`. All seven manifest byte
lengths and SHA-256 values were verified before drafting. These inputs remain
uncommitted evidence. Historical security clearance and gate passes do not
verify the changed candidate.

## Earlier continuation clarification — 2026-10-24

Historical record for `epic-192-issue-196-continuation--f920b5a8`; the repair
clarification below supplies the current scope and review route. Preserve this
dated history without treating its old starting tree as this run's candidate.

Preserve c232602e92b008c6c7ee2f92eaaab9c022adb773 (starting tree
9ccd6767ccb56ee9b1cbeea58420cc2c8246e974). The previous run concluded after a
missing pinned executable (exit 127). Candidate-local `.forge/bin/brokkr` now
hashes to ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f,
as recorded in `bundles/self/tooling.json`. Identity is not execution proof.
Prior reported green gates and mutation controls remain historical until their
source binding and admissibility are established; required gates are not skipped.

Reuse this feature slug and its exact contracts. No new semantic ruling is
introduced: 0062 remains committed and proposed, and the index already names it.
Do not recreate implementation merely because its original tasks were unchecked.
A concrete failing proof must demonstrate a defect before code changes. A missing
receipt or missing platform is outstanding evidence, not automatically a defect.
The chief commits only this spec, plan and tasks; the existing 0062 is unchanged.

### Continuation panel rulings

| Question | Adopt, reject, reconcile and cost |
|---|---|
| Implementation scope | Adopt simplicity's existing owners and both positions' preservation of c232602. Reject speculative hardening, a new framework, duplicate specs, dependency changes and gate/recipe repairs to obtain green results. Cost: residual limits remain explicit rather than being redesigned. |
| What counts as proof | Adopt robustness's candidate/source, engine/resource and platform binding. Reject compile-only closure or inheriting reported greens as new executions; simplicity also requires the complete pinned route. Cost: rerun time and a possible incomplete outcome despite unchanged code. |
| Audit breadth and mutation controls | Rule for simplicity against a fresh broad audit or mutation framework by default; adopt robustness's coverage of all claimed publication/lifecycle paths. First run existing focused and full suites. Reuse attributable red/green controls only when their exact source/test binding remains relevant; reproduce narrowly missing necessary controls downstream. Cost to simplicity: missing evidence can require additional execution; cost to robustness: no speculative redesign or audit beyond a concrete gap. |
| Diagnostic usefulness | Adopt both positions' omission and finite categories. Reject a raw debug escape hatch, secret dictionary and ambient-auth fallback. Cost: less error detail and explicit wrapper/configuration setup. |
| Robustness scenarios | Adopt crash, webhook, cap, chaos, DATA, TLS and waiter boundaries as regression constraints, not new eval promises. Reject an outbox, write-behind, query extraction or concurrency rewrite. Cost: serialization, best-effort notification loss and existing durability/resource limits remain. |
| Platform and review | Adopt robustness's explicit macOS gap and simplicity's unchanged protected route. Reject Linux-as-macOS proof or fallback after an adverse review. Cost: local green cannot establish complete supported-platform proof, and provider failure may stop delivery. |

No hostile-fixture HTTP response allocation bound is claimed: Admin::call still
collects response bytes under a request timeout. AC5 concerns the specified
stream/protocol storage, not all HTTP bodies or total process memory. Stored mail
is authoritative; webhook loss is allowed under its existing best-effort contract.
Commit-before-reply can leave durable mail after a lost reply and permit retry
duplicates; memory-only restart is not durability. Waiters need not recover mail
legitimately evicted by the cap. Existing TLS verification, raw DATA parsing,
ingest extraction and both subscribe-before-scan sites remain unchanged.

## Recorded-gap repair clarification — 2026-10-24

Continue SHIP-COMPLETE run `epic-192-issue-196-continuation--b77ca73c`, whose
review transition was REVIEW-RESIDUAL-OK with a medium non-security residual.
This design starts at 0f0613f8ee9cf050c7f59d7e42b79d6dbd992ace; historical
reviewed candidate f6dbbb53/tree dd109014 is distinct. Triage classified this
repair as engine: cross-platform execution and privacy proofs require the full
accepted [0058](../../docs/decisions/0058-codex-review-routing.md) high-risk route.

Repair only the recorded Cargo producer/proof, OS-string worker transport,
portable canary/cfg structure and lazy HOME defaults. Include closely coupled
matrix rejection variants and evidence repair. This is restoration of the
existing lookup, preparation and publication contracts, not a new public
contract or ruling. The unpublished Rust request type changes internally;
JSON inputs/results, CLI syntax and accepted guarantees do not. No new decision
is declared, and no decision or index is edited or accepted. Proposed 0062
remains proposed. Preserve the earlier dated design entries as provenance;
this section and the revised acceptance criteria/plan/tasks govern this run.

### Synthesis of the current positions

Both positions agree on the actual repairs; there is no substantive disagreement
to manufacture. The following rulings resolve implementation and proof tradeoffs
explicitly, including the cost of rejecting each cheaper or broader alternative.

| Question | Ruling and losing alternative's cost |
|---|---|
| Implementation shape | Adopt simplicity's existing main/runner/test owners and robustness's end-to-end OS-string requirement. Reject an argv codec, worker IPC, new supervisor, environment framework or crate. Cost: local Rust caller adaptations; broader abstractions receive no general extensibility work. A resolver-only fix loses because worker Clap would still reject valid native paths. |
| Cargo proof | Adopt robustness's successful byte-count/completion evidence and simplicity's valid oversized JSON followed by real Cargo. Reject the cheaper newline-only repair, start marker alone, and a NUL flood whose malformed JSON masks missing overflow checks. Cost: a larger fixture and real build execution; generic spawn/build failure cannot count as drain proof. |
| Portable canary | Adopt both positions' portable ordinary Fail/GaveUp case plus separate Linux server-death case. Reject weakening all assertions to ordinary failure or cfg-skipping all non-Linux privacy tests. Cost: two controls and explicit platform evidence; neither false macOS death nor lost Linux error-path coverage is acceptable. |
| HOME defaults | Adopt both positions' local lazy defaults. Reject leaving this LOW/INFO defect merely because the three medium code/test defects dominate. Cost: a small preparation change and explicit/empty/missing-selector tests; the closed map remains exact. |
| Evidence breadth | Rule for simplicity against a fresh universal mutation/audit framework; adopt robustness's requirement that every retained receipt binds to its actual source/test and intended assertion. Reproduce missing controls and reassess raw-publication proof after changing it. Cost to simplicity: additional execution; cost to broader hardening: unrelated diagnostics/lifecycle/HTTP work remains debt. |
| Scope and review | Adopt robustness's frozen-guarantee constraints and both positions' five separate candidate-bound reviews. Reject root/server rewrites, reduced panels, raw output escape hatches and ambient environment fallback. Cost: existing unconfined/HTTP/lifecycle limits and reduced diagnostic detail remain visible; review or unavailable macOS evidence may prevent full completion. |

### Every recovered finding has a disposition

Identifiers below refer to the numbered findings in the manifest's correctness,
tests and independent reports and guarantees G1. Security reported clean on the
historical candidate, not on this repair.

| Finding | Disposition for this run and reason |
|---|---|
| Correctness 1; tests 1; guarantees G1 | Repair the nonexecuting Cargo script and require actual counted >1 MiB output, post-write completion, successful real Cargo, full drain and no echo (AC15). |
| Correctness 2 | Carry OS strings through the actual matrix worker CLI/request/runner path and prove execution from non-UTF-8 PATH plus literal bytes (AC16). |
| Correctness 3 | Repair portable canary outcome and cfg ownership; retain Linux server-death proof. Actual macOS clippy/tests and T4 preparation stay open until run (AC17, AC13). |
| Correctness 4; independent 5 | Repair lazy HOME lookup with explicit absolute selector success and missing/empty refusal controls (AC18). |
| Correctness 5; independent 6 | Establish exact historical mutation source/test/receipt binding or reproduce narrowly missing controls. Existing pre-c232602 blob/exit claims are leads, not current proof; reassess changed canary (AC8, AC19). |
| Independent 1 | Retain LOW matrix process_group/kill_on_drop interruption debt. Detached workers may continue to deadlines; killing a worker can orphan its independently grouped children. Deadline is not orphan-cleanup proof. Process ownership redesign is separate; expose it to reviewers, and return through the bounded route if inseparable. |
| Independent 2 | Retain LOW unreachable fixture_index/safe_trial_category specificity debt. The finite fallback protects privacy; category redesign is unrelated. |
| Independent 3 | Retain LOW generic main::fail, dead formatting and silent matrix launch-category debt. Preserve safe publication and row cardinality; do not expand logging here. |
| Independent 4 | Repair the touched canary's absent >64 KiB both-stream artifact proof (AC17). Retain the separate LOW safe-diagnostics replacement-versus-cap wording/untested branch discrepancy: practically unreachable finite strings do not justify artifact-policy redesign. This is open debt against the existing cap requirement, not a waiver or claim of conformance. |
| Tests 2 | Retain LOW untested help/version, operation_failed, oracle_failed and startup/service-readiness diagnostic branches. Add repair-required coverage only; broad category coverage is separate. |
| Tests 3 | Retain LOW hardcoded /usr/bin/python3 readiness-test availability debt. Record interpreter availability during the existing test; absence is failed/missing proof, never skipped success. No new interpreter dependency. |
| Tests 4 | Add wrong-agent-label, exit mismatch and CR-containing row cases in the existing worker validation test (AC20); these constrain the unchanged protocol at the touched boundary. |
| Tests 5 | Retain INFO canned HTTP-500 injector as real HTTP/reqwest hostile-response proof; it is not business acceptance. Keep real oracle/null/wrong-action/isolation proofs. |
| Security and all reports' limitations | Historical clean is not clearance. Preserve documented Admin::call response-body collection under timeout and unconfined filesystem/network limits. Separate actual local evidence, outstanding macOS T4 and charter CI-owned proofs. No zero-byte log or old gate pass supplies missing execution. |

Retained debt does not erase AC4/AC6 obligations or establish complete conformance.
Downstream reviewers must see each residual and rule under existing policy. No
implementation-complete or supported-platform-complete claim is authorized while
required repairs, gates, reviews or platform evidence remain outstanding.

## Authority and scope

The original feature tightened the public README environment/diagnostic contract,
recorded in proposed 0062. This continuation repairs its implementation/proofs.
It supplements accepted [0055](../../docs/decisions/0055-agent-eval-harness.md)
§§6–9, which requires sanitized settings, eight inputs, draining, automatic
offline/locked preparation and honest lifecycle/results, not ambient credentials
or raw transcripts. No accepted ruling is contradicted. A contradiction discovered
in implementation waits for specific operator acceptance under house rules/0045 §8.

Root source, manifests/locks, dependencies, allowlists, gates, recipes and policies
remain unchanged. Synchronous accept/store, subscribe-before-check, oldest-only
pruning, AUTH inbox routing and ingest extraction remain intact. No sandbox,
secret broker, forwarding flag, debug-log escape hatch or model measurement.

## Original design panel rulings — 2026-10-24

| Question | Decision and accepted cost |
|---|---|
| Omission versus filtering | Adopt both positions' closed environments and zero retained arbitrary output. Reject provider denylists, regex redaction and environment-value dictionaries. Cost: raw debugging transcripts disappear. |
| Trusted Cargo exception | Adopt robustness's cleared preparation environment; reject simplicity's option to leave trusted build inheritance. It leaves an exposure before trials. Cost: explicit toolchain discovery and narrower ambient build compatibility. |
| Runtime defaults | Reconcile simplicity's fixed PATH/private HOME with robustness's explicit locale/config/cache/temp using the exact map below. Reject unspecified runtime additions. Custom runtimes need deliberate wrappers. |
| Readiness buffering | Adopt robustness's bounded typed path. Merely closing the old receiver after startup, as simplicity permits, leaves pre-ready flooding unbounded. Cost: a small parser/mailbox change and overflow proofs. |
| Diagnostic implementation | Adopt simplicity's local changes in existing modules and robustness's complete publication audit, including panic/Clap. Reject a framework, new dependency or arbitrary-text fallback. |
| Authentication | Adopt explicit caller-managed wrapper/config access. Reject automatic keys/HOME inheritance and new forwarding options. Ambient-auth users must migrate. |
| Explicit inputs | Preserve task/agent attribution and literal arguments. Reject substring redaction of results/state: common environment values would corrupt counts/labels. State the privacy limits explicitly. |
| Runtime guarantees | Preserve real protocols, lifecycle and scoring. Robustness's crash/webhook/cap/chaos/DATA/TLS/waiter scenarios constrain regressions; they do not authorize production rewrites or new eval guarantees. |

## Exact child environment contract

Every harness-created command starts with `env_clear()`. Never forward prefixes
or mutate process-global environment. Use OS strings for environment/path discovery,
including non-UTF-8 values; no lossy values in diagnostics. Unix Linux/macOS remains
the platform scope. Unsupported layouts fail safely, without inheritance fallback.

The common runtime map is exactly:

| Name | Constructed value |
|---|---|
| PATH | `/usr/bin:/bin` |
| LANG, LC_ALL | `C` for each |
| HOME | fresh private child home |
| TMPDIR | fresh private child temp directory |
| XDG_CONFIG_HOME, XDG_CACHE_HOME, XDG_DATA_HOME, XDG_STATE_HOME | fresh private child directories |

Create scratch directly under `/tmp`, never caller TMPDIR/TMP/TEMP, with a fresh
0700 owner directory. Each child/identity has separate directories. Prompts remain
in each identity's working directory under this owner. Owners live through child
completion/draining/cleanup; matrix worker runtime owners live until worker exit.
An unavailable fixed root yields a safe infrastructure error, not caller-HOME fallback.

Add only the following role entries:

| Role | Additional entries |
|---|---|
| Swarmail | `RUST_LOG=swarmail=info,tower_http=off`; fixed CLI configuration remains. |
| Fixture | `EVAL_FIXTURE_ADMIN_SECRET`, freshly generated for this trial. |
| All agents, including oracle/null | Exactly EVAL_TASK_ID, EVAL_APP_URL, EVAL_MCP_URL, EVAL_IDENTITY, EVAL_INBOX, EVAL_GOAL, EVAL_PROMPT_FILE, EVAL_DEADLINE_MS, generated per trial/identity. |
| Matrix trial worker | SWARMAIL_BIN, the absolute binary resolved once by matrix. Resolved external program paths travel in literal argv. |

No fixture secret enters another role. No other caller EVAL_*, SWARMAIL_*, provider,
proxy, loader, shell/language-startup, SSH-agent or locale value is forwarded.
Test controls are not map exceptions. The existing top-level runner fault seam is
not forwarded; cross-boundary tests use test-owned launch arrangements.

### Executable lookup and authentication

At the original caller boundary, accept absolute executable paths, resolve relative
paths containing a slash against original cwd, and resolve bare programs through
caller PATH (fixed system PATH if unset). Interpret relative/empty PATH components
against original cwd. Require executable files; launch absolute paths. Apply to
agent argv[0], SWARMAIL_BIN and selected build tools before changing cwd/environment.
Matrix resolves each agent program at its caller boundary, before worker launch.
Missing programs produce value-free categories, never lookup paths.

Subsequent arguments stay literal, never split or shell-expanded. System shebangs
and ordinary system subprocesses work with fixed PATH. Custom interpreters/runtime
search paths require deliberate configuration in a caller-selected wrapper.
Built-ins use the same environment/spawn path as external agents.

Authentication no longer arrives through inherited API keys or caller HOME.
A caller-selected wrapper can read an explicitly selected existing private config
file or obtain credentials by another deliberate mechanism. The harness neither
reads nor copies that credential. Credential values must not enter argv, tracked
files or reports. Prove compatibility using synthetic file-backed configuration.
Deliberate credential retrieval and a child's independent private logs remain outside
this unconfined harness's guarantee; no new auth adapter or broker is introduced.

### Automatic build preparation

Preserve automatic `cargo build --offline --locked --bin swarmail`, once per
matrix/proof setup, and Cargo JSON executable discovery with custom targets.
Preparation also clears its environment. Its exact map is the common runtime map
with only these replacements/additions:

- PATH: deduplicated directories of resolved Cargo, rustc and rustdoc, then
  `/usr/bin:/bin`. Select Cargo from the existing CARGO selector or caller PATH.
- RUSTC and RUSTDOC: absolute programs resolved from original caller PATH.
  Caller RUSTC/RUSTDOC overrides and compiler-wrapper variables are not forwarded.
- CARGO_HOME and RUSTUP_HOME: explicit directory selectors, or original HOME
  joined with `.cargo` / `.rustup` when absent. Resolve relative paths against
  original cwd. Missing required discovery information fails safely; original
  HOME itself is not forwarded.
- RUSTUP_TOOLCHAIN: optional nonempty selector, preparation only.
  CARGO_TARGET_DIR: optional locator made absolute against original cwd;
  otherwise retain repository Cargo target configuration.

These named locators/selectors are intentional preparation inputs, not a promise
of zero caller-derived build values. None flows into runtime children or metadata.
No registry tokens, general CARGO_/RUST_ families, RUSTFLAGS/encoded flags,
credential-provider overrides, proxies, loaders or compiler wrappers are forwarded.
Support ordinary installed system/rustup toolchains and offline caches; unsupported
custom layouts may supply existing prebuilt SWARMAIL_BIN. Local Cargo config,
build scripts and chosen executables remain trusted filesystem inputs on an
unconfined host, not environment-isolated supply-chain attestation.

## Publication and buffering contract

Publish only finite harness-authored role/stage/category strings and numeric
exit code/signal, HTTP status, deadline and optional saturating byte counts.
No child stdout/stderr, raw response body, library/OS error Display/Debug, panic
payload, environment value, argv or executable/artifact/toolchain path enters
a diagnostic. Apply this at validation, build/resolution, spawn/readiness,
HTTP/MCP/admin, snapshot/scoring errors, server death, joins/panics, artifact
write/rename, matrix launch/parse, and the CLI common failure path.

Use fallible Clap parsing and safe categories on failure; generated help/version
remain. Install a value-free panic hook once at executable entry before parsing
or runtime creation; library trials never install competing global hooks.
Library diagnostic/error returns obey the policy, but an embedding application's
own hook/output is its responsibility.

Concurrently drain both pipes with fixed 8192-byte buffers, retaining zero raw
diagnostic bytes. Keep diagnostics.log as safe facts with a 64 KiB total cap and
one fixed truncation notice; empty on success is allowed. Metadata keeps
`swarmail_bin` as null and its other existing fields. Result schema 1 is unchanged.

Internal protocol exceptions are never diagnostic text:

- Readiness uses complete lines of at most 8192 bytes, validates required
  loopback addresses, and publishes at most the first valid typed tuple into
  one slot. Producers never wait on consumers; no raw-line queue. Drop whole
  oversized/malformed lines; after readiness continue draining without retention.
- Cargo allows at most 1 MiB per complete JSON record and retains only the
  required executable path internally. Oversized records are discarded through
  newline and mark preparation failed; do not stop draining or print fragments.
- Matrix permits one schema-valid, expected task/agent result row, at most 1 MiB;
  extra/non-row output or overflow fails safely. This is an explicit protocol
  resource limit: excessively large labels can yield infrastructure failure.
  Discard worker stderr concurrently; never echo rejected bytes.

No unbounded channel or `output()` accumulation remains at these boundaries.
Missing required protocol data yields infrastructure error. Buffer bounds do not
claim bounds on child memory or a hostile child's independent resource use.

Only safe content reaches final artifacts and temporary files. Preserve the
validated secret-free fixture snapshot; do not rewrite mail, snapshot or scoring
predicates. Per-file rename is not whole-bundle atomicity. SIGKILL/abort/unavailable
stdout retain 0055's no-result limits; safe partial artifacts may remain.

Privacy means no information flow from caller-only environment or discarded child
output into harness publication. Explicit task/agent labels remain attribution,
including refused rows; argv reaches the selected command; explicit artifact paths
select destinations. Those inputs must not contain secrets. Coincidental matches
such as environment values `1` or `oracle` do not justify corrupting results.
No concealment promise covers explicit labels, filenames on the local filesystem,
deliberate child file writes, arbitrary hostile peers or filesystem/network access.

## Acceptance Criteria

- **AC1 — Exact environments.** Observe the complete actual command environment
  for Swarmail, fixture, agents and matrix workers. Parent-only credential,
  arbitrary-name, stale EVAL/admin, HOME/TMPDIR, proxy/loader and non-UTF-8 canaries
  are absent except declared selectors in their intended role. Require execution
  markers, correct generated values and fresh fixture-only admin secret.
- **AC2 — Compatibility.** Eight inputs are correct and prompt bytes equal
  EVAL_GOAL. Trial/matrix tests cover built-ins and external absolute/relative/
  bare-PATH programs, shebangs/subprocesses, missing programs, literal
  metacharacters and a synthetic private-file authentication wrapper.
- **AC3 — Preparation.** Real offline/locked preparation succeeds with its declared
  map, including custom targets; matrix resolves/builds once. Build credentials,
  flags and wrappers are absent; no preparation selector reaches runtime roles.
  Failed launch/exit/malformed/oversized Cargo output produces safe errors.
- **AC4 — Publication.** Children independently emit file/argument-supplied canaries
  on both streams, split across writes, with invalid bytes, no newline and more
  than the old 64 KiB tail. No such bytes appear in trial/matrix stdout/stderr or
  final/temporary artifacts, in ordinary or handled failure cases. Unsafe paths,
  CLI inputs, response errors and panic payloads yield useful safe categories.
  Environment clearing alone cannot satisfy this proof.
- **AC5 — Bounds.** Tests and storage/queue inspection prove finite retained
  memory. Flooding before/after readiness and during agents cannot block draining,
  valid readiness, deadlines or cleanup. An oversized line followed by a valid
  announcement works without accepting truncated fragments.
- **AC6 — Lifecycle.** Startup/spawn failure, server death, panic, interruption,
  deadline and artifact failure preserve established result/exit behavior and
  terminate/kill/reap. Matrix retains one row per valid product element on
  build/launch/row failure and its aggregate exit precedence. No invented success.
- **AC7 — Existing behavior.** Every oracle task passes; every null task fails in
  its idle class; wrong-action classes, actual exits/scoring precedence and two
  overlapping isolation trials remain proven through real SMTP/HTTP/MCP. No mocks,
  global env mutation, mid-band chaos or sleep-as-synchronization.
- **AC8 — Discrimination.** Focused tests fail with the relevant old inheritance,
  raw-publication or unsafe metadata/error behavior restored, then pass with the
  repair restored. Record separate synthetic red/green controls.
- **AC9 — Documentation/scope.** README and earlier owning eval spec/plan replace
  obsolete inheritance/tail/path-metadata wording, state exact maps, authentication
  migration and unconfined limits. Verify the existing proposed 0062 index row
  and status agreement before gates; do not duplicate it. No forbidden scope,
  dependency, gate or acceptance change.
- **AC10 — Delivery.** Separate eval fmt, locked clippy/full tests including
  proof.rs, all pinned house verification gates and the five separate AC14 reviews
  with security hold precede implementation completion. CI-only obligations stay with
  CI. Structural design validation is none of these receipts.

- **AC11 — Pinned continuation.** Every new receipt names the tested commit/tree,
  command, actual exit, OS and relevant engine/resource identity. Execute the
  exact candidate-local engine compile without the policy-sweep warning refusal,
  actual prescribed design/verify dispatch and policy/delivery tamper proofs.
  A matching digest, compile alone or this draft is insufficient. Historical
  records are preserved, not overwritten or fabricated. Any source change
  requires affected proofs and required gates on the final candidate.
- **AC12 — Fix-specific evidence.** Execute
  `admin_error_categories_discard_injected_response_text`, checking
  admin_response_status/admin_response_rejected and canary absence, and
  `automatic_build_uses_a_cleared_named_preparation_map`, requiring the actual
  executable under the selected custom target. The HTTP case proves a non-200
  JSON response, not every transport/read/parse path; AC4/AC6 require broader
  suite evidence and meaningful execution markers, not absence alone.
- **AC13 — Honest platform boundary.** T4 macOS system/rustup/toolchain preparation
  requires actual execution on macOS. On Linux it stays explicitly outstanding
  for a macOS runner/CI; do not assert current CI already provides it. Report
  charter CI-only obligations separately from local passes. Do not mark T4 or
  overall supported-platform verification complete without that receipt.
- **AC14 — Protected reviews and bounded returns.** Execute five separate
  candidate-bound results under accepted 0058: Astra correctness, official
  DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees and Fable 5.1
  independent review. This supersedes earlier GLM/Haiku/Flash-guarantees routing
  for this continuation. Never DeepSeek Pro, author substitution or invented
  fallback. Preserve protected review, worst verdict, hard security hold and
  bounded returns; no recipe/decision edit, push, PR, merge or issue close.
- **AC15 — Executed Cargo overflow.** A real executable wrapper writes a valid
  JSON record longer than 1,048,576 bytes with an independently supplied
  synthetic canary, followed by successful real offline/locked Cargo output.
  Assert launch, actual successful byte count, post-write completion and real
  Cargo success; drains finish through EOF under a bounded test deadline and
  yield the intended safe build refusal without publishing raw bytes anywhere.
  Missing launch, short write, missing completion or unrelated Cargo failure
  fails the test. Preparation succeeds with the below-limit equivalent; removing overflow
  rejection makes the oversized control fail its intended assertion. Stopping
  drainage must fail completion or deadline, never satisfy expected refusal.
- **AC16 — Native worker arguments.** Launch the actual matrix CLI with an
  unchanged agents JSON naming a UTF-8 bare program in a non-UTF-8 Unix PATH
  directory. Require execution/completion and literal-argument byte markers,
  exactly one matching schema-1 row, actual child status and correct aggregate
  exit. Reinstating the String worker boundary fails the regression. Direct
  trial preserves OS-string arguments too. Retain caller cwd/PATH lookup,
  built-ins, missing-command refusal, all matrix cardinality and scoring rules;
  no lossy path conversion or runtime PATH forwarding.
- **AC17 — Portable nonpublication.** On supported Unix, an ordinary noisy
  signup agent exits 0 without completing the task: assert Fail/GaveUp, real
  status 0 and trial exit 1, with artifacts present. Successfully emit >64 KiB
  per stream, including independent canaries in the retained-tail region,
  split writes, invalid bytes and no required newline; count both streams and
  mark completion after full writes. Scan CLI output and every remaining final
  or temporary artifact for raw/canary bytes, with null swarmail_bin metadata
  and private modes. Retain a distinct Linux-only actual server-death
  HarnessError test with verified kill of the runner's own server. Entire
  Linux-specific probe/oracle continuation lives in cfg blocks; no unreachable
  non-Linux code or warning suppression. Actual macOS clippy/tests, not Linux
  inspection or cross-compilation, establish the macOS portion.
- **AC18 — Lazy preparation defaults.** With executable discovery otherwise
  supplied, real preparation succeeds with explicit nonempty absolute
  CARGO_HOME/RUSTUP_HOME and HOME removed. HOME is consulted only for a missing
  selector's default. Missing HOME for either required default and explicit
  empty selectors refuse safely. Exact preparation/runtime maps and existing
  relative-locator semantics remain unchanged; no locator enters metadata.
- **AC19 — Evidence remains falsifiable.** Bind separate inheritance,
  raw-publication and unsafe metadata/error red/restored-green controls to
  source/test regions, actual commands, nonzero counts, exits and OS. Historical
  receipts may count only after explicit binding; reproduce missing controls
  and changed raw-publication proof. Keep their mutations out of final code and
  run all final-candidate gates after restoration. Record AC15 overflow and
  AC16 old-worker controls independently. No aggregate green or empty log
  replaces an intended-assertion failure.
- **AC20 — Worker validation variants.** Existing matching single-row acceptance
  still passes; wrong label, row/worker exit mismatch and a CR-containing row
  each refuse independently alongside existing wrong-task/extra/malformed cases.
  Rejected bytes are never echoed; result schema and protocol limits do not change.
