# Eval provenance port onto current main

Date: 2026-10-25. Status: draft for implementation. Epic #192 / issue #200.
Base: aec4b46; intake commit: 1823f55.

Decision: 0063 — [proposed eval provenance](../../docs/decisions/0063-eval-provenance.md).

## Feature and rationale

Port the six shipped commits in the verified issue-200 packet onto current
main. Every handled eval row must say which measured executable, build revision,
declared agent/model and public fixture inputs produced it. Matrix retention
must preserve bound per-row evidence that check can score offline. A passing
business verdict alone cannot establish reproducibility, attribution or privacy.

The chief read both run-local panel positions and the actual intake at
`.forge/tasks/epic-192-issue-200-port-the-ship-4ab8f513.md`; the requested
`.brokkr/state/tasks/` is absent. All 53 packet entries, including README and
operator rulings, and the candidate-local engine digest were verified. The
packet's six patches are historical behavior evidence, not current test receipts.
Nothing outside this workspace is needed for this design.

Accepted [0071](../../docs/decisions/0071-eval-trap-scoring.md) governs current
v2 trap scoring and supersedes the shipped v1 compatibility/scorer assumptions.
Accepted [0055](../../docs/decisions/0055-agent-eval-harness.md), as amended by
0071 and the operator's scoped rulings, remains authority. Existing cancellation
and privacy owners described by proposed [0073](../../docs/decisions/0073-matrix-cancellation.md)
and [0075](../../docs/decisions/0075-eval-privacy.md) retain their behavior and
status. Restore 0063 at the operator-allocated number rather than allocating
0077 for the same proposal; accepted 0071 explicitly preserves this reservation.
This restores a shipped proposal, accepts nothing and changes no frozen guarantee.

Scope is the shipped eval implementation, tests and owning documentation.
No root runtime/manifests/lock, dependency allowlist, coverage floor/denominator,
engine pin or recipe changes; no namespaces, model measurements, retries,
concurrent-trial redesign, publication or broader attestation system.

## Panel synthesis and explicit costs

The positions largely agree; the following rulings settle implementation choices
rather than inventing a disagreement between panel members.

| Question | Adopted ruling | Rejected alternative and accepted cost |
|---|---|---|
| Existing owners | Adopt simplicity's one executable, sequential matrix, cleanup owner and publication primitive, with robustness's full boundary proofs. | Reject wholesale old-file replacement or a second supervisor. Cost: targeted integration and baseline test mapping. |
| Business compatibility | Both positions win: accepted v2 scorer/catalogue/snapshot; no v1 adapter or mixed report path. Require strict new provenance-bearing v2 rows. | Reject historical decoder import and optional-field bags. Cost: historical rows need their historical reader; no invented provenance. |
| Typed evidence | Robustness's required keys, finite availability and complete binding checks govern. | Reject best-effort metadata and a smaller nullable-field bag. Cost: typed modules, repeated hashes and malformed-input tests. |
| Completion race | Adopt robustness's immutable-conflict rule and simplicity's refusal of a new handshake. | Reject child-evidence rewriting or child-ready/parent-ack staging. Cost: an honest cancellation can leave incomplete artifacts and exit 2. |
| Timeout ownership | Current cleanup owner governs; envelope expiry requests stop. | Reject a second parent kill timer or kill-and-return. Cost: no unconditional wall-clock completion promise under scheduling/output stalls. |
| Private claims | Adopt canonical provisioned identity or null, all roles retained, with writer-only guard. | Reject broad redaction and reader membership refusal. Cost: unknown spelling is discarded in new events; historical v2 evidence may remain sensitive. |
| Command/source facts | Adopt measured bytes, build-time revision and explicit public declarations. | Reject runtime-HEAD attribution, model inference and automatic private argv hashing. Cost: opaque/private inputs have explicit unavailable facts. |
| Repeatability | Adopt semantic public choices and independent CSPRNG secrets. | Reject one seeded global RNG. Cost: fixed seed plus recorded fixture time reproduces public difficulty, not secrets, wire bytes or agent outcomes. |
| Scope | Simplicity wins against frameworks; robustness wins on causal negative controls within existing scope. | Reject new storage/webhook machinery, durability, resume, namespaces or external attestation. Cost: unconfined execution and detectable incomplete evidence remain explicit limitations. |

## Dated clarifications

2026-10-24, recovery: the original schema-1-only design was superseded by the
operator's scoped schema-2 direction. It did not accept the whole 0063 proposal.

2026-10-24, privacy return: canonical claimant recording and writer-only guards
repair unknown-header disclosure without changing otherwise valid historical
claim scoring. Keep those repairs across the current v2 integration.

2026-10-25, earlier port: current #194/#196 owners superseded historical
orchestration, environment transport and publication mechanisms. Patches 04–06
complete the boundary and source-helper cancellation repairs.

2026-10-25, this port: accepted 0071 supersedes old schema-1 snapshots/catalogue,
business-score-v1 and mixed-report acceptance. The historical 0055 addendum may
be retained only as clearly superseded history beside a dated integration note;
it must never reactivate those paths. Decision-index repair belongs to T1 under
the chief's exact four-file scope, with no waiver of the inventory gate.

## Requirements

### R1 — Inputs and deterministic choices

Add --seed and --fixture-time to trial and matrix. Seed is exactly 64 lowercase
hex digits (32 bytes); time is UTC RFC3339, whole seconds with Z suffix, from
1970-01-01 through 9998-12-01 inclusive (checked calendar arithmetic).
Invalid supplied inputs refuse before launch with exit 2. Defaults generate
one seed from fresh entropy and sample UTC once per invocation, truncated to
seconds. Matrix shares its recorded time across cells. Pass effective values
to fixture children without resampling. Print resolved public seed/time on stderr
even without retention; every newly emitted result is schema 2 and carries
the same resolved values in its required provenance object.

Standalone trial uses its effective seed directly. Matrix derives each cell
seed from master seed, zero-based task occurrence/ID, agent index/label and
repetition. Keep task-major, agent-major, repetition-major order, duplicate
task occurrences, unique agent labels, one row per valid element, no retries
and aggregate exit 2 > 1 > 0. Matched fixtures across agents require separate
trials with the same seed; matrix coordinates intentionally differ.

Algorithm fixture-sha256-v1:
- F(x) is eight-byte big-endian byte length followed by x. Text is UTF-8,
  without normalization; integers are eight big-endian bytes before framing.
- Cell seed is SHA-256 of concatenated F values: domain
  swarmail-eval-cell-v1, master seed bytes, task index, task ID, agent index,
  agent label, repetition index, in that order.
- Choice digest is SHA-256 of F values: domain swarmail-eval-choice-v1,
  effective seed bytes, task ID, logical identity slot, flow name, generation
  index, field name. Slots/generations are zero-based; field keys are fixed
  literals enumerated in implementation docs and tests.
- Choice in [0,n) takes the first eight digest bytes as big-endian u64 modulo
  fixed nonzero n. This specifies repeatability, not exact uniformity.
  Non-capability Message-ID/MIME suffixes use the first 16 digest bytes in
  lowercase hex with distinct keys.
- Pin exact framing/cell/choice vectors, duplicate tasks and Unicode labels.
  Algorithm or key changes require a new version; no DefaultHasher/StdRng.

Stable projection v1: selected task/scenario, ordered logical identity slots
and roles, per-slot/flow message roles/generation order, digest target position,
invite start/end, RFC5322 Date, ICS DTSTAMP, MIME structure, non-capability
Message-ID/boundary suffixes, and (for identical scripted actions) OTP
generation/supersession and business transitions by slot. Independent slots
compare in slot/flow/generation order, not arrival order. Scenario stays the
selected task's scenario.

Mask only capabilities, OTP digits, passwords, support challenge/ticket,
calendar UID, actual identity/inbox strings, execution IDs, URLs/hosts/ports,
observed elapsed/event/delivery times and cross-slot arrival order.
Actual identities/inboxes remain fresh with stable logical slots. Choices
cannot depend on secret RNG consumption or concurrent interleaving.

Admin bearer, confirmation/magic/reset/invite tokens, OTP values, passwords,
support challenges/tickets and calendar UID keep independent fresh private
entropy. No deterministic-secret switch, published private seed or secret
hash. Schedule alone grants no authority: token/owner/UID checks remain.
Inject fixture UTC into calendar/mail dates; readiness, deadlines, OTP expiry
and cleanup remain monotonic; TLS time stays real. No byte-identical mail or
deterministic external-agent/scheduler/outcome promise.

### R2 — Strict results and truthful, secret-safe provenance

**Version boundary.** Current main already uses result, snapshot and catalogue
business schema 2 under accepted 0071. Keep all three at 2. Every new result
retains the current execution fields and adds one required closed provenance
object. Require every key, including nullable keys; reject duplicate, unknown,
missing, unsupported and hybrid fields from original bytes before map conversion.
Do not introduce a v1 decoder, fallback, legacy scorer or business schema 3.

| Consumer | Contract |
|---|---|
| report | Accept strict provenance-bearing v2 rows only; refuse v1, mixed-version and pre-provenance v2 input before aggregation. Preserve attempts, infrastructure counts, success denominators, Wilson intervals and quantiles. |
| new trial/matrix | Emit provenance-bearing v2 for every handled row. Workers must return one row with assigned context and matching exit. |
| retained check | Require the new strict envelope and provenance-bearing v2 result, current v2 catalogue/snapshot and business-score-v2. Refuse old unversioned metadata and old scorer bindings. |
| bare-state check | Preserve current v2 snapshot semantics and v1 refusal; no provenance guarantee. |
| replay | Keep validated original result unchanged beside business_replay. No provenance synthesis, relabeling or pooling of history. |

Historical result readers/execution use the matching historical source/binary,
including pre-provenance v2 rows. This deliberately closes the new result
contract without inventing absent facts. Artifact, replay and projection schema
1 identifiers below are independent envelopes, not business-v1 compatibility.

**Required v2 provenance fields.** These names and ownership are normative:

| Field | Required content |
|---|---|
| fixture | seed (64 lowercase hex), fixture_time (R1 UTC), seed_algorithm=fixture-sha256-v1, projection_version=1 |
| matrix | null for standalone, otherwise matrix_id, master_seed, task_index, agent_index, repetition_index; exact R1 coordinates |
| task | catalogue_schema=2, catalogue_sha256, definition_sha256 availability, scorer_id=business-score-v2; hash the default task definition, not an override |
| harness | package, version, executable_sha256 availability, build_commit availability, source_dirty availability |
| target | executable_sha256 availability, build_commit availability, source_dirty availability; settings include effective per-inbox cap 100000, sanitized server settings and observed chaos policy/outcome |
| agent | kind builtin/external/invalid (invalid only on request-invalid error), executable_sha256 availability, command availability, model availability, config_id availability |
| limits | effective agent_deadline_ms (availability for invalid request only), startup/I/O/shutdown bounds and matrix_child_envelope_ms (null standalone; cancellation-request threshold for matrix, not a cleanup-completion deadline) |

The row's trial/task/agent/confinement fields remain authoritative; do not
replicate the entire provenance object in metadata or the index. Unobserved target settings/chaos outcomes use explicit unavailable values;
never report planned events as observed. Catalogue
hashes use catalogue-json-v1: compact UTF-8 serialization of existing typed
structs in declared field order, without whitespace/trailing newline; pin
bytes/digests. Changed scoring semantics must change scorer_id.

Availability is a closed tagged union: {status:present, value:T,
source:S}, {status:unavailable, reason:R}, or, only for builtin model,
{status:not_applicable, reason:builtin}. Values are typed, not arbitrary JSON.
Sources are closed field-specific codes: measured_bytes for executable SHA-256;
build_git for harness revision/dirty; owned_build_context for target source;
identical_owned_artifact for its known byte-identical alias; declared_public_argv
for command; operator_declared for external model/config; builtin for builtin
command/config; resolved_input for deadline/settings, observed_fixture for
actual fixture events, and catalogue_definition for task identity. Reject impossible source/field combinations. Revisions are full Git
object IDs; SHA-256 values are exactly 64 lowercase hex. source_dirty is a
boolean availability, distinct from the revision, never a clean-commit claim
when true or unavailable.

Unavailable reasons are bounded enum codes: not_resolved, not_readable,
not_built, no_git, build_context_changed, external_source_unknown,
not_declared, private_argv, request_invalid. No arbitrary error text or
placeholder digests. Required fields remain present on errors even when their
facts are unavailable. Parsed invalid standalone requests can have null
task/agent IDs, unavailable deadline/task definition and no snapshot only
with harness_error/request_invalid. Invalid seed/time/CLI syntax refuses
before a trial is admitted. Matrix cells always have validated assigned inputs.

**Build and executable identity.** Measure actual harness/target/agent
executable bytes with SHA-256. Capture harness revision/dirty facts in an
eval-only build.rs at build time. Rebuild triggers must cover source changes,
HEAD/ref/index changes, worktree Git indirection and packed refs. Missing Git
or an archive build records no_git, never runtime HEAD. Running from another
checkout cannot change those embedded facts. This is build context, not
proof of reproducible compilation.

Resolve/build Swarmail once per matrix. Runner-owned offline/locked Cargo
builds bind the selected Cargo artifact digest to source revision/dirty
observations before/after the build, including a local relevant-source content
fingerprint (a dirty boolean alone cannot detect dirty-to-dirty edits). Do not
retain that private comparison fingerprint as public identity. A changing
source context refuses source
association and fails affected cells with build_context_changed. Do not call
a root build revision the harness revision. An opaque SWARMAIL_BIN is measured
but its source is external_source_unknown, unless its bytes equal an already
measured artifact from a known owned build in this invocation. No external
receipt format or extra build just to attribute an override. Unknown source
alone is accurately recorded and allowed; unreadability, replacement, or
contradiction of an asserted association is a per-cell harness_error.

Capture once, then rehash immediately before relevant launches. PATH resolution
must measure the executable actually selected; shell/interpreter executable
identity does not attest loaded scripts/modules. Do not run a target merely
to scrape a version string. Cooperative checks do not eliminate hostile
check-to-exec races; hashes prove content consistency, not authenticity.

**Agent identity.** Add optional --agent-model and --agent-config-id to trial;
agent-file entries gain optional model/config_id and public_argv (default
false). Trial --public-argv is an explicit declaration that its entire literal
program/argv after -- is public and credential-free. Old label/argv files run
unchanged with external model/config unavailable and command private_argv.
Builtin model is not_applicable, builtin command/config identifies oracle/null.
Model is operator_declared, never inferred from label/program/config ID or
claimed as measured remote-service identity. Supplying model/config_id or public_argv=true for a builtin refuses before
admission rather than replacing known builtin facts; omitted values and
public_argv=false preserve old builtin entries.

For opted-in external commands, command.value is {algorithm:public-argv-sha256-v1,
sha256:H}. Hash F(domain swarmail-eval-public-argv-v1), F(argc encoded as
big-endian u64), then F(each exact UTF-8 argv item, including program), using
R1's length framing. Compute this projection from caller-supplied argv before
native executable resolution. Execution keeps OsString and the resolved native
path independently. Non-UTF-8 private argv continues to execute literally;
public-argv opt-in with non-UTF-8 input refuses value-free before admission,
because this declared hash format encodes UTF-8, never lossy conversion. Pin boundary/empty/Unicode vectors. No normalization,
PATH expansion or secret filtering; never persist literal argv. Its source is
declared_public_argv: this is the actual argv's digest following an explicit
public assertion, not a digest of hidden environment, loaded scripts or
credentials. With no assertion, record private_argv and only the independently
measured executable identity. A config ID is never a substitute argv digest.
No automatic argv/environment hashing, heuristic redaction, prompts or raw
mail retention. Declared public labels/model/config/argv are caller-controlled;
no claim to sanitize credentials deliberately marked public.

**Envelope and diagnostics.** Strict metadata schema 1 contains schema_version,
trial_id, result (fixed path/length/SHA-256), state (fixed path/length/SHA-256
or explicit absent reason) and diagnostics (fixed path/length/SHA-256).
Result owns all provenance; metadata binds exact bytes. Plan owns intended
cell identities/settings; index binds plan and ordered metadata bytes.
Cross-file shared inputs must agree. No self-hash or duplicate identity cache.
A snapshot must match the result's task/trial and validated task semantics.
Legacy unversioned metadata is not this envelope.

Public diagnostics contain bounded harness-controlled event/reason codes,
not child logs, panic payloads or arbitrary errors/paths. Raw child output is drained and discarded under #196; retain no new scratch
transcripts. Private runtime directories follow existing cleanup. Newly published snapshots/results stay secret-free
under R5; historical reader success makes no secrecy certification. Canary tests
cover normal and error paths; direct public declarations have the explicit
caller responsibility above, not a universal secret-sanitizing claim.

### R3 — Retention, matrix ownership and honest failures

trial --artifacts-dir ROOT retains ROOT/<fresh-trial-id>/.
matrix --artifacts-dir ROOT creates ROOT/<fresh-matrix-id>/ with
matrix-plan.json, trials/<fresh-trial-id>/ directories and matrix.json.
IDs use at least 128 fresh random bits independent of seed and exclusive
creation; collision returns harness_error without overwriting. Only generated IDs form
path components, never labels/task strings.

Before a retained matrix launches, check arithmetic, resolve every cell's
ID/seed/time, exclusively claim its fresh directory and atomically write the
plan. Concurrent invocations may share ROOT. Never reuse/overwrite an existing
run/trial destination. Invalid inputs before product admission exit 2 without
rows. Valid admitted products still emit one harness_error row per affected
cell on handled root/storage/launch errors, where stdout works.

Parent owns plan/index and assigns trial IDs/destinations; child claims its
directory exclusively and owns finalization. Route normal completion, handled
setup failures, interruption and cleanup errors through one finalizer after
cleanup. Write/close result, optional state and safe diagnostics, then publish
metadata.json from a same-directory private temporary file last, using the
current no-clobber primitive described below. Final metadata is
the trial completion marker; no separate trial transaction manifest.

Parent accepts exactly one v2 child row, validating task/agent/trial ID,
assigned seed/time/matrix coordinates and provenance, schema, command exit-code consistency and retained-result equality when requested.
Drain stdout within 64 KiB. Overflow, multiple/malformed/mismatched rows fail
the cell. The shipped matrix_child_envelope_ms remains effective agent deadline plus
300 seconds, but is a cancellation-request threshold, never a total cleanup
bound. Expiry requests TERM once through the existing run_child owner, marks
the cell an infrastructure failure and awaits its direct wait, drainage and
owned cleanup. The child retains the shared five-second TERM grace followed
by five-second post-KILL observation; the parent must not install the old
patch's competing five-second kill timer. A handled cell timeout may continue
later cells after that fence; a matrix signal latches cancellation and refuses
the suffix. Repeated signals/timeouts cannot reset or shorten cleanup. Keep
historical recorded envelope values readable; they are historical execution
facts, not guarantees about this implementation. No new hostile-process
confinement or post-SIGKILL cleanup guarantee.

Only after reaping, and only if no child directory exists, may parent finalize
fallback under the assigned identity. Partial/conflicting evidence remains
untouched/incomplete; emit artifact harness_error, never overwrite or bless a
published record. Forced child death cannot reconstruct lost state.

Requested retention failure overrides business pass/fail with harness_error,
exit 2 and a bounded stderr reason. An unwritable/full disk cannot guarantee
retained errors. Leave no completion marker for failed finalization; continue
other cells. A retained setup harness_error with absent state can be complete
evidence; completeness is not business success.

Publish matrix.json last, binding exact plan bytes and ordered metadata
paths/lengths/hashes for every planned cell. Missing/conflicting evidence
prevents publication. Per-file no-clobber publication promises detectable incomplete
evidence, not fsync/power-loss durability. Hashes establish consistency, not
authenticity. No repair/resume/rerun. SIGKILL, abort, parent death or unavailable
stdout retain 0055's limits on terminal rows.

### R4 — Offline validation and scoring

Extend check with exclusive modes:
- Existing --task ID --state FILE, unchanged; no provenance guarantee.
- --artifacts-dir DIR, a completed standalone trial directory.
- --matrix DIR, a completed matrix run directory.

Validate all evidence before any replay row: supported versions, lengths/hashes,
expected files, catalogue/task/scorer compatibility, result invariants, snapshot
task/trial identity, exact matrix product/order/unique coordinates and IDs,
seed derivation and cross-file input equality. Reject missing markers, partial
trees, duplicate/unknown JSON fields, mixed runs, swapped state, unsupported
versions and hash mismatches. Hash bound bytes before parsing.

Paths are fixed filenames or exactly trials/<generated-id>/metadata.json.
Reject absolute/parent paths, symlink components/files and non-regular files.
Check opened files under the selected root, not just lexical prefixes.
Never execute evidence. Hostile concurrent local rewriting remains outside
unconfined provenance.

Artifact-envelope-v1 matrices allow at most 4096 cells, checked before allocation.
Encoded bounds: plan/index 8 MiB each; metadata/result 64 KiB each; state
16 MiB; diagnostics 64 KiB; retention-mode agent input 1 MiB; task/label/model/config
strings 1024 bytes; JSON nesting at most 64 levels. New result rows are bounded
to 64 KiB including their newline even without retention. Writers/readers enforce the same bounds. Refuse before
allocating claimed lengths, stream hashes and process snapshots one at a time.
Parsing retains bounded depth and does not allocate arbitrary dimensions.
Legacy non-retained matrix product limits and report statistics remain unchanged;
result emission/reading follows the strict R2 contract.

After envelope validation use tasks::validate_snapshot and tasks::score.
An otherwise consistent snapshot ineligible for scoring (e.g. final Failed/
Parked delivery) produces explicit unscorable business replay, preserving its
historical harness_error. Missing snapshots are also unscorable. Replay does
not contact services, launch/build binaries, reparse extraction or use saved
commands. Original executables need not exist; scorer/catalogue compatibility
must match. For the request-invalid standalone record defined in R2, validate
the current catalogue/scorer and absent-state error shape; unavailable task
identity produces unscorable replay rather than an invented task definition.

New replay output schema 1 carries trial ID/coordinates, original_result
(unchanged row), and business_replay with status scored/unscorable, verdict
when scored or bounded reason otherwise. Matrix emits in plan order after all
validation. Invalid evidence exits 2 with no replay rows. Valid evidence exits
2 for any unscorable cell, else 1 for any replay business failure, else 0.
These are replay exits: original budget/infra outcomes never get rewritten
even when business replay passes. Report still consumes original result JSONL.

### R5 — Canonical publication and compatible historical reads

Snapshot schema remains 2; Event.claimed remains Option<String>. At the shared
Private::record boundary, look up the supplied claimant in fixture-owned
accounts and copy the account's canonical identity only on a match. Retain
all provisioned roles, including decoys and other agents. Missing, empty,
whitespace-only, unrecognized and lossy-decoded unknown values become null
after existing header parsing. Never retain the original unknown bytes,
a digest, prefix or secret-derived sentinel. Keep existing trimming, routes,
HTTP status/messages, action, flow, accepted flag, category and sequence order.

Missing/unknown still return HTTP 400 and record ToolMisuse. Recognized decoys
still return 403 with FOREIGN_TOKEN/WrongMessage and their identity; known
agents presenting another owner's token keep existing foreign-token behavior.
Accepted fact backing, first relevant refusal, accepted 0071 trap-before-goal precedence,
isolation and reset-replay exemption remain unchanged (business-score-v2).
Do not drop refusal events or erase recognized identities.

One small publication invariant checks every non-null event claim against the
snapshot's provisioned identity list. Apply it before admin snapshot bytes are
serialized and before any retained result/state bytes are published. Fixture
recording supplies canonical data from the actual registry; the additional
check refuses a noncanonical Snapshot supplied to a writer. It does not rewrite
it. Admin invariant failure returns a fixed value-free HTTP 500 without state;
retention invariant failure follows the existing infrastructure-error/exit-2
finalizer path, preserves cleanup and leaves no completion marker. Ordinary
unknown-header trials remain fail/tool_misuse, not infrastructure failures.
The invariant must work for unfrozen admin snapshots without requiring scorer
eligibility; preserve admin authentication and existing freeze semantics.

| Reader/interface | Required treatment |
|---|---|
| Historical v1, mixed-version or pre-provenance v2 results | Refuse in the new reader; use the matching historical binary. No invented provenance. |
| Bare schema-2 snapshot | Omitted claimed, explicit null and arbitrary unknown strings remain accepted when otherwise valid; same verdict/exit, no snapshot/claim in output. |
| Strict retained snapshot | All current keys remain required, including nullable claimed. Null, known identities and historical arbitrary unknown strings are accepted when otherwise structurally/binding-valid; score unchanged. |
| Invalid retained evidence | Refuse before any replay rows, including a bad later matrix cell; diagnostic contains no input-derived values. |

Do not call publication validation from any historical reader, shared scorer
or structural validator. No timestamp cutoff, fallback, normalization,
recomputed binding, automatic cleanup, archive conversion or saved-command
execution. Original bound bytes and execution outcomes remain untouched;
replay emits only its existing result/verdict fields. Old unversioned metadata
and nonconforming results embedded in new envelopes still refuse per R2. Reader success
proves consistency/scoreability only, never historical privacy or permission
to publish an input tree. Operator handling of old sensitive files is outside
this change. A membership rule cannot certify arbitrary caller-authored
identity lists; public declarations retain R2's stated responsibility.

Map read/parse/structural/scoring errors at bare-state and retained check
boundaries to bounded harness-controlled diagnostics without raw serde error,
field/key, claim, identity, path or other input text. Keep current output
shapes and exits: invalid bare state still emits its existing harness_error
row and exits 2; invalid retained evidence emits no replay row and exits 2.
Known catalogue task IDs are safe output; unknown supplied task IDs use a
fixed error. Apply the same nonreflection to library retained errors reachable
by the CLI. No low-entropy secret hashes in logs, assertions or receipts.

### R6 — Integration with current cancellation and privacy owners

Keep main.rs::matrix/run_child/write_row/matching_trial_row/capture_stdout and
runner.rs::Groups as the orchestration owners. Do not import historical
matrix.rs orchestration or a second string-based program resolver. Assigned
public context travels in explicit internal worker CLI arguments before `--`;
validate its strict schema, size, assigned IDs, coordinates and identity before
launch. Pass no EVAL_MATRIX_CONTEXT environment field. Keep the existing
SWARMAIL_BIN runtime field for the private resolved binary locator; send only
its public measured/source facts in context, not TargetBuild's private paths
or source-comparison fingerprint. Preserve original native agent argv after
`--`, including option-looking later arguments, without reinterpreting it.
An internal context is trusted parent plumbing on an unconfined host, not an
external source attestation API; reject inconsistent context, never grant
caller-authored values measured authority. Recheck selected bytes in the worker.

Every provenance/helper subprocess must start after signal subscription, use
cleared explicitly constructed environment and join existing ownership. Resolve
caller programs before changing cwd/environment; retain lazy HOME/default
selectors, offline locked Cargo JSON artifact discovery, fixed runtime maps,
generated agent PWD and no_proxy clients. Embed harness revision during build;
runtime Git must not replace it. Build-time Git also uses an explicit cleared
map, bounded capture and no raw failure output. Source paths/fingerprints and
caller HOME/PWD/environment never become rows, reports or diagnostics.

Trial finalization occurs once after cleanup and covers early setup refusal,
interruption and errors, with truthful availability for facts never observed.
Retention errors override pass/fail before row emission. Reuse #196's exclusive
0700 leaf and descriptor-private 0600 temporary before payload, complete writes,
no-clobber final publication and ownership-only cleanup for every new artifact.
Keep unowned parent modes/bytes unchanged; never mutate global umask. A shared
artifacts module may move these functions with their tests, not replace their
semantics with create_dir/overwriting rename.

Matrix finalization order is: assign plan/context; admit one worker; await
worker cleanup/wait/drain; validate its row and exit; arbitrate pending signals
at the existing commitment fence; commit exactly one authoritative row through
write_row; validate retained equality against that exact committed row. Only
then may the parent include that cell's immutable metadata in the final index.
A signal while queued output awaits acknowledgement retains #194's existing
commit semantics and exit 2; already committed rows never change. If pending
cancellation or corrupt child output changes the authoritative row after child
retention, preserve the conflicting private files, issue a bounded artifact
failure, exit 2 and publish no matrix.json. Do not overwrite the child's result
or complete an index for a different row. Do not add a deferred child-marker
handshake or a second transaction manifest. Post-reap fallback is allowed only
when no child-owned leaf exists; suffix refusal evidence can then use assigned
fresh IDs and explicit absent state. Index failure after rows commit changes
exit to 2, never retroactively rewrites stdout rows or invents a summary row.
Completeness is agreement of all bound cells, not proof of business success.

Keep pre-spawn subscriptions, latched admission, once-only forwarding,
independent Cargo ownership, body abort/join, retained groups/reapers/drains,
actual cessation/direct waits and private-directory lifetime. Preserve the
64 KiB total child capture with overflow drainage, full ordered cardinality,
committed prefix, post-cleanup active result, zero-time unstarted suffix,
cancellation-winning commitment, late-signal exit 2, honest observed statuses,
2 > 1 > 0 exit precedence and report math. No retries or new outcomes. Baseline
#194/#196 regressions retain assertion strength; authorized v2/API adaptations
must not remove their negative controls.

### R7 — Preserve current v2 business semantics and final shipped repairs

Bind actual current catalogue/definition bytes, including trap-disclosing goals,
to catalogue-json-v1 hashes with business-score-v2. Regenerate independent v2
vectors; changing an old fixture label alone is not proof. Keep one tasks::score.
Infrastructure/invalid state precedes live deadline, earliest exact matching
trap precedes goal, then current isolation/rejection/idle fallbacks apply.
Preserve all seven 0071 traps, including sibling known-Decoy/null-flow refusal,
expired-but-not-superseded OTP recovery, reset replay and non-target recovery.
Writer membership never becomes a new reader/scorer policy. Historical unknown
claims remain readable only within otherwise compatible v2 state/envelopes;
v1 state still refuses. Replay certifies no historical privacy.

Include patches 04–06, not only implementation patch 03. Source-observation
failure must join owned helper cleanup; SIGTERM or SIGINT observed during that
cleanup remains latched before later Cargo admission. Preserve patch 05's
acknowledgement-based cleanup controls and patch 06's hold/release/bypass proofs.
The matrix remains sequential with fresh HOME per trial per the #199 operator
ruling. Namespace/concurrent-trial work is outside this port.

## Acceptance Criteria

- AC1: Every newly emitted schema-2 trial/matrix row, including setup failure, cancellation, unstarted suffix and retention failure, carries the shipped typed provenance: Swarmail source commit and SHA-256, build-time harness commit, explicit model and argv identity/digest, and per-trial seed, or the shipped explicit unavailability reason. No runtime HEAD attribution for an unrelated binary. Identity tests cover changed runtime checkout, worktree/packed refs, dirty/archive builds and revision rebuilds. A replaced/misassociated SWARMAIL_BIN is refused before affected execution, or its mismatch is explicit in every affected row; it cannot appear verified.
- AC2: Preserve the shipped command identity/privacy contract: private/legacy argv is not automatically hashed or published; opted-in public argv uses the shipped framed digest. Model labels are declarations. Native program paths and literal arguments still execute through CLI/worker boundaries. Independent canaries prove no private argv/path or caller environment reaches rows, reports, diagnostics or artifacts.
- AC3: Fixed seed and fixture time reproduce the same public projection in fresh processes, including digest position and invite time; selected changed-seed vectors change public layout. Retain shipped seed/time parsing, semantic keys, duplicate-task/Unicode vectors and pre-admission matrix assignment. Invalid options refuse before launch. Tokens, OTPs, passwords, challenge/ticket secrets, admin secrets and invite UID use independent CSPRNG entropy, demonstrated with controlled independent entropy. Trial IDs/inboxes remain fresh. Real SMTP/MCP/HTTP tests retain MIME Date/DTSTAMP, attachment/page equality, OTP supersession and invite acceptance/expiry proof.
- AC4: A real two-task × oracle/null × two-repetition matrix retains eight ordered rows with unique IDs, assigned seeds, per-row state/result/metadata and bound plan/index. Missing/ineligible state is explicit. Shared-root runs do not overwrite. Early failure, crash, wrong ID/exit, malformed/multiple/oversized output and output failure produce honest infrastructure accounting. Retain the 64 KiB child capture bound, exactly-one matching row validation and continued drainage.
- AC5: Requested retention failure changes infrastructure outcome/exit and prevents a valid completion marker. Metadata is last; the final index binds all planned cells. Parent fallback occurs only after reaping and without child-owned output. Preserve exclusive 0700 leaf, descriptor-private 0600 temporaries before payload, complete writes, no-clobber publication and ownership-only cleanup. Collision, partial write, interruption, metadata/index failure and both umask extremes leave unowned bytes/modes unchanged and still exercise cleanup.
- AC6: Trial/matrix replay works offline without original services/binaries and launches no saved command. Validate versions, byte hashes/lengths, semantic associations, dimensions, paths and bounds before any retained replay row. Mutated/swapped state/results, symlinks, duplicate/missing coordinates, wrong seeds, incompatible scorer/catalogue and over-limit inputs refuse, including an invalid later cell. Test at-limit and over-limit boundaries. Replay leaves evidence untouched, uses current tasks::score, separates original infrastructure/budget outcomes from business replay and marks absent/ineligible snapshots unscorable.
- AC7: Preserve accepted 0071 across all integrations: catalogue/snapshot/result business versions remain 2; v1 snapshot and v1/mixed report rejection tests stay red on acceptance regressions. New rows require provenance; new report/retained readers refuse pre-provenance v2 rows as well as v1/mixed input. No reader fabricates provenance or pools pre-trap history. Seven named traps still beat recovered goals in either order, infrastructure precedes deadline, deadline precedes traps, and earliest exact matching trap wins. Retain claim/flow/status/class matching, known-Decoy/null-flow sibling attribution, expired-but-not-superseded OTP recovery, reset replay, isolation and non-target recovery. All completed external naive controls still exit zero with healthy real mail and recovered goals yet fail with their expected trap classes. Catalogue goals continue to disclose the traps. Replay those retained completed controls through the actual CLI with matching verdict/class.
- AC8: At shared fixture recording, new evidence retains canonical provisioned claimant or null; unknown/empty/missing claimants retain HTTP/action/category/order semantics. Known decoys/foreign agents keep wrong-message evidence and 0071 attribution. Live OTP and nonnumeric canary unknown identities produce HTTP 400/tool_misuse and null claims, with no canary in admin snapshots, stdout/stderr, reports, replay or retained trees. Cover link, OTP, reset/login, invite, support, release and marketing callers. Writer-only membership guards run before admin/artifact publication; they do not become new scorer membership rules. Preserve compatible historical claim handling within accepted v2 evidence, strict required-key checks and value-free malformed-input diagnostics.
- AC9: Every #196 regression retains assertion strength: env_clear maps, private HOME/runtime directories, generated agent PWD, caller-resolved native executable, explicit preparation selectors, lazy HOME/custom-target offline builds, no_proxy clients, bounded typed diagnostics and no raw panic/serde/child bytes. Worker context uses CLI arguments without widening the worker environment map. No production test-control or ambient inheritance escape hatch.
- AC10: Every #194 and shipped #200 cleanup regression retains assertion strength for both SIGTERM/SIGINT: pre-spawn subscription, latched admission, once-only forwarding, owned Cargo/source helpers, abort/join, retained group/reaper/drain identities, concurrent five-second TERM grace and five-second post-KILL observation, direct waits and honest observation failure. Cancellation observed during source-task failure/cleanup prevents later Cargo admission. Preserve causal Pending hold/release and bypass controls, repeated signals, startup/completion races, resistant descendants and output pressure. Full product order/cardinality, committed prefix, post-cleanup active row, fresh zero-time refused suffix, late-signal exit 2, statuses/lifetimes, no retries and 2 > 1 > 0 exits remain. Envelope expiry cannot bypass cleanup or reset its budget. At the acknowledged child-retention/row-commit seam cancellation wins; divergent immutable child results prevent matrix.json and exit 2 without mutation. Index failure changes exit without rewriting committed rows.
- AC11: Port the shipped independent provenance/boundary controls and map baseline #194/#196/#198 tests to retained counterparts. Do not delete assertions, weaken time/size bounds, substitute mocks or use sleeps as synchronization. All-task oracle passes, null idle failures, wrong-action and isolation controls remain real SMTP/HTTP/MCP. Documentation describes current v2 scoring, provenance availability/privacy, deterministic public choices, retained replay and unconfined limitations without stale v1 compatibility promises. Decision inventory/status tests pass after T1 adds exactly one proposed 0063 index row; any historical addendum is explicitly subordinate to 0071, with no new acceptance. Final committed implementation passes every gate in plan.md under candidate routing; historical receipts do not substitute for fresh evidence.

- AC12: Shipped strict boundary controls cover required nullable keys, duplicate/unknown/hybrid fields, exact public-argv framing and non-UTF-8 refusal on public opt-in while private native argv remains literal. Producer/reader at-limit and over-limit tests cover every R4 bound; all malformed-input diagnostics are value-free. Independent v2 catalogue/definition hashes and business-score-v2 agree across producer, retention, replay and test fixtures.
- AC13: Both signals at the acknowledged child-retention/row-commit seam preserve immutable child bytes, cancellation-winning rows and full suffix accounting; divergent evidence prevents matrix.json and exits 2. Index failure never rewrites committed rows. Envelope expiry and source-observation failure cannot bypass cleanup or allow later Cargo admission after latched cancellation. Worker context uses bounded CLI arguments with the exact closed worker environment map unchanged.

## Proof ownership and design handoff

AC1–AC13 are local implementation obligations, not claims of completed work.
The plan pairs each obligation with tests and records all gate commands. CI
owns macOS eval evidence through #201, privileged port-80 exchanges, chart/
history/selftests, Docker, network RustSec and tag-only rate proofs. The operator
owns live settings and sealing. None becomes a new local acceptance criterion.

This chief commits exactly spec.md, plan.md, tasks.md and proposed 0063. The
index/addendum integration is the first unchecked implementation task; the
full gate inventory cannot be called clean until it passes. The final pinned
sequence check alone rules the design attempt. This seat returns drafted,
never implementation completion or decision acceptance. Commit unsigned;
never push, merge, tag, publish or close the issue.
