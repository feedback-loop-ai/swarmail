# Eval privacy — Epic #192 / issue #196

Date: 2026-10-24. Design draft; implementation and delivery proofs outstanding.

Decision: 0062 — [Closed eval environments and safe diagnostics](../../docs/decisions/0062-eval-privacy.md), proposed; only the operator accepts.

## Feature and rationale

Keep caller credentials and unrelated environment values out of eval children,
and arbitrary subprocess text out of eval diagnostics. These are independent
boundaries: a child can print a secret from a file even with an empty environment.
Current runner commands inherit arbitrary variables; raw output tails enter
artifacts and errors; Cargo/matrix stderr and path-bearing errors bypass collection.

Construct closed environments and drain then discard arbitrary child output.
Keep harness-authored failure facts, real exit statuses and secret-free snapshots.
Preserve literal external commands, eight agent inputs, scoring, cleanup,
schema 1 and `confinement:"unconfined"`.

Read intake `.brokkr/state/tasks/epic-192-issue-196-keep-caller-c-8465b441.md`
and both `.brokkr/state/design/positions/{simplicity,robustness}.md` files.
Those files remain uncommitted run-local evidence.

## Authority and scope

This tightens the public README environment/diagnostic contract, hence proposed
0062. It supplements accepted [0055](../../docs/decisions/0055-agent-eval-harness.md)
§§6–9, which requires sanitized settings, eight inputs, draining, automatic
offline/locked preparation and honest lifecycle/results, not ambient credentials
or raw transcripts. No accepted ruling is contradicted. A contradiction discovered
in implementation waits for specific operator acceptance under house rules/0045 §8.

Root source, manifests/locks, dependencies, allowlists, gates, recipes and policies
remain unchanged. Synchronous accept/store, subscribe-before-check, oldest-only
pruning, AUTH inbox routing and ingest extraction remain intact. No sandbox,
secret broker, forwarding flag, debug-log escape hatch or model measurement.

## Explicit panel rulings

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
  migration and unconfined limits. Add proposed 0062's accurate index row before
  implementation gates. No forbidden scope/dependency/gate or acceptance change.
- **AC10 — Delivery.** Separate eval fmt, locked clippy/full tests including
  proof.rs, all pinned house verification gates and four independent reviews with
  security hold precede implementation completion. CI-only obligations stay with
  CI. Structural design validation is none of these receipts.
