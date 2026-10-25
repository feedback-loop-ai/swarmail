# Port eval privacy onto matrix cancellation

Date: 2026-10-24. Status: continuation design draft; fresh verification and review remain open.
Feature: Epic #192, issue #196; preserve the committed port at unreviewed handoff
529d916 on #194, framed by a87fb96.

Decision: 0075

Proposed [0075 — eval privacy](../../docs/decisions/0075-eval-privacy.md)
is renumbered from 0062. Only the operator accepts decisions.
Authority: accepted [0055](../../docs/decisions/0055-agent-eval-harness.md)
§§6–10, [0058](../../docs/decisions/0058-codex-review-routing.md), and the
[house rules](../../docs/house-rules.md). Preserve the implemented contract
described by proposed [0073](../../docs/decisions/0073-matrix-cancellation.md)
and the complete [matrix cancellation specification](../matrix-cancellation/spec.md).

## Feature and rationale

Keep caller-only environment and arbitrary child output out of eval trials and
their published diagnostics while preserving #194 cancellation, cleanup ordering,
status honesty and result cardinality. Handoff 529d916 already contains the port;
this continuation corrects platform-proof ownership, verifies every remaining
criterion, and repairs only demonstrated defects at their existing owners.
Replaying the packet or redesigning the runner is not authorized.

Evidence is the checksum-verified local issue packet (all eleven SHA256SUMS
entries), both current run-local simplicity/robustness positions, and the
[current intake framing](framing.md). The expected .brokkr/state/tasks/ directory
is absent. The packet README's older candidate description and prior green
summaries are historical evidence, not fresh verification of 529d916. The
candidate-local engine digest matches the pinned digest in plan.md. No journal,
sibling tree, GitHub or CI access is needed or authorized.

## Clarification — 2026-10-24 platform ownership

The operator's dated row **macOS evidence for `evals/`** in
[operator-rulings.md](../../docs/rulings/epic-192.md) rules:
`evals/` is outside CI on every OS until #201; #201's CI gating owns macOS-only
eval proof. AC13 and AC23 below supersede the former requirement to obtain
macOS receipts in #196. This records ownership, not execution or portability.
Every Linux preparation/toolchain/custom-target, artifact, mutation and gate
obligation remains. The previous run's macOS blocker remains historical; it is
not rewritten as a pass. Proposed 0075 is updated in place for this same feature;
no new decision number or acceptance of 0075 or 0073 follows from this ruling.

## Explicit synthesis

The current positions agree on behavior. Their tension is between minimizing
change and demanding enough evidence; the ruling is a small repair scope with
complete proof, not a smaller proof obligation.

| Question | Ruling | Accepted cost / rejected alternative |
| --- | --- | --- |
| Continuation scope | Adopt simplicity's preservation of 529d916 and robustness's candidate-bound verification. Repair only an identified failed criterion/test. | Reject a second port, speculative hardening and treating unchecked historical tasks as missing code. Existing code may need no change, but Linux proof still costs execution. |
| Platform ownership | Adopt both positions' dated #201 correction across spec, plan, tasks and 0075. | Reject local macOS receipts as a #196 blocker and any inference of CI success. Platform execution remains unproved here and owned by #201; Linux work stays mandatory. |
| Evidence reuse | Adopt simplicity's reuse of the existing test map and robustness's lineage/assertion audit of the eleven supplied mutation receipts. Reproduce missing, stale or insufficient evidence. | Reject both blind historical-green reuse and an unnecessary new proof framework. Receipt audit and fresh final gates cost time; valid discriminating evidence need not be recreated merely for a new date. |
| Runner ownership | Adopt both positions: retain #194 admission, waits, groups, drains and cleanup; retain private runtime owners through those same fences. | Reject another supervisor, descendant registry or shorter parent timer. Lifetime proofs must observe actual cleanup Pending, not eventual death. |
| Closed commands | Retain exact runtime and separate Cargo preparation maps, native argv and generated agent PWD. | Reject trusted-build inheritance and credential forwarding. Ambient auth/config discovery is lost; explicit wrappers/private config or a prebuilt binary remain the setup paths. |
| Diagnostics and bounds | Retain omission, finite facts, honest numeric statuses and distinct typed readiness/Cargo/matrix storage. | Reject raw-debug options, redaction dictionaries and a generic parser framework. Transcripts are lost; readiness stays 8192 bytes, Cargo 1 MiB per record, matrix and safe diagnostics 64 KiB. |
| Artifacts | Retain exclusive private leaf/temp creation, descriptor normalization before payload and complete-file no-clobber publication. | Reject global umask changes, replacing rename and durable multi-file transactions. Per-file atomic visibility does not promise crash recovery. |
| Runtime preservation | Adopt robustness's SMTP/store/await/prune/chaos/webhook/DATA/TLS examples as preservation constraints. | Reject expanding them into new root-server features, namespaces or performance promises. Existing suites retain their assertions and accepted limits. |
| Review | Adopt both positions' five separate accepted-0058 careful results in the review phase. | Reject author verdicts and historical reviews as substitutes. Five fresh invocations remain required; this design supplies none. |

When economy of edits conflicts with complete privacy/cancellation proof, rule
for proof; the losing shortcut costs confidence in the handoff and is refused.
When extra robustness mechanisms conflict with the existing single ownership
model without a demonstrated defect, rule for simplicity; the accepted cost is
the documented unconfined and per-file boundary, not a new isolation guarantee.
Any actual contradiction with an accepted ruling waits for the operator rather
than weakening either contract. No frozen guarantee may be weakened.

## Closed command contract

Every harness-created command begins with env_clear; no process-global
environment mutation or family/prefix forwarding. Common runtime map:

| Name | Value |
| --- | --- |
| PATH | /usr/bin:/bin |
| LANG, LC_ALL | C |
| HOME | Fresh private child home |
| TMPDIR | Fresh private child temporary directory |
| XDG_CONFIG_HOME, XDG_CACHE_HOME, XDG_DATA_HOME, XDG_STATE_HOME | Separate fresh private child directories |

Use exclusively owned 0700 roots directly under /tmp, ignoring caller TMPDIR,
TMP and TEMP. Each role/identity has distinct directories. Prompts and agent
workdirs are under private ownership. Runtime owners survive all relevant
child reaping, drainage and group cleanup, including body abort. Unavailable
/tmp fails safely rather than falling back to caller HOME.

| Role | Only additions to the common map |
| --- | --- |
| Swarmail | RUST_LOG=swarmail=info,tower_http=off |
| Fixture | Fresh EVAL_FIXTURE_ADMIN_SECRET |
| All agents, oracle/null included | EVAL_TASK_ID, EVAL_APP_URL, EVAL_MCP_URL, EVAL_IDENTITY, EVAL_INBOX, EVAL_GOAL, EVAL_PROMPT_FILE, EVAL_DEADLINE_MS; generated PWD equal to actual agent workdir |
| Matrix worker | Resolved absolute SWARMAIL_BIN |

Fixture credentials never enter another role. The eight EVAL inputs retain
their existing semantics; EVAL_GOAL bytes equal the prompt file. No stale
EVAL/SWARMAIL values, provider credentials, proxy/loader/startup settings,
SSH agent sockets or test controls cross these maps.

Capture only discovery inputs at the original caller boundary using native OS
strings. Resolve absolute programs, relative programs containing slash, and bare
programs through original PATH before changing cwd or clearing environments.
Unset PATH uses /usr/bin:/bin; relative and empty PATH components use original
cwd. Require executable files. Subsequent argv stays literal. Preserve native
paths/arguments through TrialRequest, actual CLI parsing and matrix worker
handoff; agents JSON remains textual, but its resolved executable need not be
UTF-8. Ordinary system shebangs/subprocesses work with fixed PATH.

Authentication/custom runtime setup uses an explicit caller-selected wrapper
and private configuration, not inherited keys/HOME or a new forwarding option.
Credentials must not appear in labels, argv or tracked evidence. Chosen
executables and deliberate configuration access remain trusted local inputs.

### Cargo preparation

Preserve once-per-batch automatic offline/locked root binary build and Cargo
JSON executable discovery, including custom target directories. Preparation
clears its environment and uses the common private map with only:

- PATH replaced by deduplicated directories of resolved Cargo/rustc/rustdoc,
  then /usr/bin:/bin. CARGO selects the Cargo executable before launch.
- RUSTC and RUSTDOC set to absolute programs resolved from original PATH;
  caller RUSTC/RUSTDOC overrides and compiler wrappers are not forwarded.
- CARGO_HOME and RUSTUP_HOME from explicit nonempty locators, made absolute
  against original cwd, or original HOME/.cargo and HOME/.rustup defaults.
  Consult HOME lazily: both explicit homes work without HOME. Missing required
  defaults or empty selectors fail with safe categories.
- Optional nonempty RUSTUP_TOOLCHAIN selector. Optional nonempty CARGO_TARGET_DIR
  locator made absolute against original cwd; when absent, retain repository
  Cargo target configuration.

These are intentional preparation inputs, not runtime inheritance. No general
CARGO_/RUST_ families, tokens, flags, proxies, loaders or wrappers. Selectors
never enter metadata/runtime children. Ordinary system/rustup installations and
offline caches need Linux proof here; #201 owns actual macOS execution under
AC13. Unsupported layouts may use
an explicit prebuilt SWARMAIL_BIN. Cargo config/build scripts are trusted file
inputs; this is not supply-chain confinement.

## Output and publication contract

Drain both child streams concurrently and retain zero arbitrary diagnostic
bytes. No raw queue, tail, error Display/Debug, response text, executable path,
argv, panic payload or caller-only environment value reaches CLI publication,
library diagnostic returns, metadata or final/temporary artifact payloads.
Publish finite harness-authored role/stage/category facts and honest numeric
codes/signals/statuses/deadlines/counts. Preserve distinct cleanup observation,
wait and expiry failures; safe rendering cannot turn them into success.

Use fallible CLI parsing and a value-free executable panic hook installed
before parsing/runtime construction. Generated help/version remain available.
Library trials must not install competing global panic hooks; embedding callers
own their hooks/output. Cover validation, lookup/build, spawn/readiness, HTTP/MCP/
admin, snapshots/scoring, joins, cleanup, artifacts and matrix parsing/errors.
Admin transport/read/JSON/status failures suppress URLs, bodies and error text.
Every eval reqwest builder explicitly uses no_proxy, including HTTP in the parent.

| Internal protocol | Retained data and bound |
| --- | --- |
| Readiness | First validated typed loopback announcement; complete lines at most 8192 bytes; nonblocking bounded storage, no raw-line queue |
| Cargo | Complete records at most 1,048,576 bytes; only required typed executable data and overflow/error state retained |
| Matrix worker | #194's total 65,536-byte capture; exactly one expected schema-valid task/agent row with valid exit pairing, no CR/extra/malformed output |
| diagnostics.log | At most 65,536 bytes of safe facts including a fixed truncation notice |

Discard a whole oversized line through its newline, never parse its suffix.
Continue draining after readiness, overflow and executable discovery. Cargo
overflow makes preparation fail safely after owned cleanup. Signal responsiveness,
independent waits, progressing-stdout bound and post-exit EOF bound remain.
Worker stderr may be null at spawn; any piped stream has an owned bounded drain,
with no new unbounded join after worker exit. Malformed protocol bytes are never
echoed. A valid row is an internal protocol exception, not arbitrary diagnostic
publication. Metadata keeps swarmail_bin as null.

Create a fresh exclusive trial leaf, initially no broader than 0700 and exactly
0700 before payload. Existing parents' permissions stay unchanged. Create
exclusive temporary files initially no broader than 0600, set exact 0600 via
descriptor before writing, finish payload, then publish without clobbering.
Final visibility is atomic per file. Existing leaves (directory, file, live or
dangling symlink) refuse; collisions with files/directories/symlinks/hard links
preserve unowned bytes and modes. Clean only owned names. All four ordinary files
(state.json, result.json, diagnostics.log, metadata.json) and private residual
temporaries remain private. Preserve artifact order and artifact-failure row/exit
semantics, with safe artifact_write_failed diagnostics. No post-write-only chmod,
global umask changes or overwriting rename.

## Cancellation and accounting preservation

Subscribe to SIGTERM/SIGINT before any owned spawn, including fallback Cargo.
The resolved-binary worker path starts no subprocess before handler installation.
First signal latches admission closed. A racing admitted child remains owned;
forward SIGTERM once and await the worker's cleanup. Repeated signals neither
renew nor shorten cleanup. Cargo remains independently owned and reaped.

Close body admission by aborting and joining it before final ownership collection.
Retain direct-child waiters, identities, runtime owners and drains. TERM remaining
groups concurrently for the existing five-second grace, KILL survivors, then
observe cessation/reaping for the existing five-second bound. Normal completion
uses the same final fence. No matrix-side shorter timer can kill its cleanup
owner. Leader exit, pipe EOF and successful kill requests are insufficient proof.
Keep Linux identity/live/zombie rules and conservative non-Linux group absence;
unknown observation, expiry or wait errors stay harness failures.

Keep schema 1, full task → agent → repetition cardinality, byte-identical
committed prefix, one active error row after cleanup, and fresh zero-time refused
suffix rows with unknown statuses and no launches. Cancellation wins simultaneous
commitment; a late signal changes exit to 2 without rewriting the last committed
row. Preserve real status/lifetime observation, output-failure admission stop,
snapshot before server shutdown, scoring after cleanup, infrastructure/deadline/
business precedence and report denominators. Every result remains unconfined.

## Acceptance Criteria

The original privacy AC1–AC23 retain their numbers below; current routing,
generated PWD and the stricter matrix limit explicitly supersede stale wording.

- **AC1 — Exact roles:** whole actual-command environments match the maps for
  server, fixture, agents and workers, with execution markers and synthetic
  ordinary/non-UTF-8 caller canaries absent. Private directories are distinct;
  generated agent PWD equals real cwd; fixture secret stays fixture-only.
- **AC2 — Compatibility:** all eight inputs/prompt bytes, builtin/external
  absolute/relative/bare lookup, unset/relative/empty PATH, executable checking,
  literal argv, shebang/subprocess and private-file wrapper auth pass.
- **AC3 — Preparation:** actual offline/locked builds use the exact named map,
  discover a selected custom target, build once per matrix and never leak selectors
  to runtime/metadata. Launch/exit/malformed/oversized errors are safe.
- **AC4 — Nonpublication:** independently supplied child canaries on both streams
  (split writes, invalid bytes, no newline and over 64 KiB) are absent from CLI
  output and all remaining final/temp artifact payloads. Unsafe input/path,
  panic and HTTP/admin failures produce finite safe categories.
- **AC5 — Bounds:** pre/post-readiness floods and oversized-then-valid lines keep
  draining, preserve typed readiness, deadlines and cleanup. Storage inspection
  and counted writes prove bounds; truncated oversized suffixes never announce.
- **AC6 — Lifecycle:** startup/spawn failure, server death, panic, signals,
  deadlines and artifact errors preserve honest rows, exits and owned cleanup.
  Matrix build/launch/row failures retain full product and exit precedence.
- **AC7 — Existing behavior:** real oracle all-pass, null declared idle failures,
  wrong actions, actual exits/scoring and two overlapping isolation trials pass
  over real SMTP/HTTP/MCP. No mock acceptance, sleeps as synchronization,
  process-global test env/umask mutation or probabilistic chaos.
- **AC8 — Discrimination:** separate inheritance, raw-publication and unsafe
  metadata/error mutations each fail the intended exposure assertion and pass
  after restoration, with actual execution rather than missing-output success.
- **AC9 — Documentation:** exact maps, authentication migration, output omission,
  native paths and limitations agree across README, owning eval spec/plan and
  proposed 0075/index. Old 0062 live links and inheritance promises disappear;
  historical evidence stays. No accepted ruling or dependency/gate edit.
- **AC10 — Delivery:** independent eval fmt/locked all-target clippy/full tests
  and every house gate pass on the final candidate; root exact coverage has
  zero missed lines/branches/functions and passes its selftest. CI-owned
  obligations remain distinct; structural design validation is not verification.
- **AC11 — Pinned proof:** receipts identify candidate/tree, command, actual exit,
  OS and engine/resource identity. Candidate-local engine compile, pinned dispatch,
  design selftest and delivery/tamper proofs execute without warning refusal.
  Historical passes or a digest alone cannot replace execution.
- **AC12 — HTTP/build regression:** admin_error_categories_discard_injected_response_text
  proves non-200 JSON body omission with safe status/rejection facts; transport,
  read and parse failures also have exercised safe branches. The cleared-build
  test proves the actual executable under its chosen custom target. Every eval
  reqwest builder has no_proxy; dead HTTP/HTTPS/ALL proxy settings still permit
  real oracle success and never receive admin Authorization.
- **AC13 — Platforms:** retain every Linux preparation/toolchain/custom-target,
  standalone eval fmt/locked clippy/full tests and selected artifact-primitive
  proof required here. Under the 2026-10-24 operator ruling, row **macOS evidence
  for `evals/`**, in [operator-rulings.md](../../docs/rulings/epic-192.md),
  `evals/` is outside CI on every OS until #201. #201's CI gating owns macOS eval
  preparation/toolchain, clippy/tests and selected artifact-primitive execution;
  those macOS receipts are not a completion prerequisite for #196. Record that
  ownership without claiming macOS execution or existing CI proof.
- **AC14 — Review:** five fresh accepted-0058 careful results: Astra correctness,
  official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees, independent
  Fable 5.1. No Pro, author substitute, combined result or unauthorized fallback.
  Protected review, worst verdict and security hold remain binding.
- **AC15 — Cargo overflow:** actual wrapper writes a valid JSON record over 1 MiB
  with independent canary, counted successful bytes and completion acknowledgment,
  followed by successful real Cargo output. Drain to EOF within deadline, refuse
  safely, and pass the below-limit control. Disabling overflow rejection fails
  its assertion; stopping drainage fails completion rather than falsely passing.
- **AC16 — Native worker argv:** actual matrix CLI uses unchanged JSON naming a
  UTF-8 bare executable in non-UTF-8 PATH. Execution/completion and literal byte
  markers, one matching row, true status/exit prove native handoff. Reinstating
  a String-only boundary fails. Direct-trial later arguments stay native too.
- **AC17 — Portable output proof:** noisy signup agent exits 0 without completing;
  require Fail/GaveUp, status 0, CLI exit 1, snapshot and all artifacts. Count
  successful writes over 64 KiB per stream and scan retained-tail-region canaries
  everywhere. Keep a distinct Linux server-death HarnessError proof. Platform
  cfg covers whole Linux-only continuations without unreachable-code suppression.
- **AC18 — Lazy defaults:** explicit nonempty Cargo/Rustup homes work without
  HOME; missing HOME for either needed default and explicit empty selectors
  refuse safely. Relative locator semantics and selector exclusion hold.
- **AC19 — Falsifiable evidence:** source/test-region and command/OS/exit-bound
  red/restored-green receipts cover AC8, AC15 and AC16 separately. No aggregate
  green or empty log substitutes for intended-assertion failure; restore all
  mutations before final gates.
- **AC20 — Worker variants:** matching row succeeds; wrong task/label, extra,
  malformed, CR-containing, oversized or row/worker-exit mismatch each refuses
  independently without echoing bytes, under #194's 64 KiB bound.
- **AC21 — Modes:** ordinary failed trial with real snapshot/artifacts executes
  under permissive 0000/0002 and owner-bit-removing 0777 umasks in isolated
  subprocesses with accessible pre-existing parent. Assert exact 0700 leaf,
  0600 files, unchanged parent, null metadata and no canaries. Missing artifacts
  or launch failure cannot pass; old writer fails the permissive-mask assertion.
- **AC22 — Before payload/failure:** owning tests observe restrictive creation
  and descriptor normalization before writes, force write/publication failures
  deterministically and inspect every surviving owned file/temp. No after-write
  chmod-only proof, obsolete operation test or production test flag.
- **AC23 — Exclusive publication:** refuse existing leaf variants and temp/final
  file/directory/symlink/hard-link collisions without touching sentinels.
  Final name appears only with complete private payload; cleanup removes only
  owned names. Real CLI failure retains safe category/row/exit semantics.
  Linux primitive receipts remain mandatory here; macOS primitive execution
  belongs to #201 under AC13, separately from Linux evidence.
- **AC24 — Cancellation:** all #194 fence, both-signal, repeated-signal,
  queued admission/commit/final-row, bootstrap/Cargo, leader-first/resistant-child,
  pressure/EOF, observation/expiry and checked-test-teardown proofs remain,
  including acknowledged actual-Pending hold/release and bypass negative controls.
  Ordinary dev/release binaries ignore every test control and former fault value.
- **AC25 — Integrated ownership/accounting:** cancel noisy workers/builds while
  private runtime directories remain owned through cleanup; safe cleanup failures
  still produce honest error rows, complete refused suffix and no later launches.
  Preserve all cancellation/accounting/precedence rules above.
- **AC26 — Test preservation:** every named baseline and packet test in plan.md
  has its retained/adapted counterpart and discriminating assertions. Changes
  to seams/structure cannot justify deletions, diluted assertions or runtime
  test-control allowlist entries.

## Limits

This is unconfined environment/output isolation, not prevention of deliberate
child file writes, host filesystem/network access, hostile same-user activity
or escaped process groups. Explicit labels, argv, file selections and trusted
Cargo configuration are intentional inputs and must not contain secrets.
Coincidental environment values such as 1 or oracle do not justify redacting
schema/labels/state. Artifacts have per-file atomic visibility, not multi-file
transactions, crash durability, fsync recovery or guaranteed orphan removal.
Forced death/host loss/unavailable stdout retain existing no-result limits.
No root production, dependency, license, coverage boundary, routing or release
change is authorized.
