# Implementation plan — delivery gates

Implements [spec.md](spec.md) and proposed
[0050](../../docs/decisions/0050-delivery-gates.md).

## Technical context

Single Rust application crate; real SMTP/POP3/HTTP integration suites;
Bash gates on a mawk host; cargo-llvm-cov and the installed coverage nightly;
standalone syn/proc-macro2 scanner; brokkr 0.12.0 with CI's source SHA and
the seat's separately authorized binary digest/provenance recorded in
`bundles/self/tooling.json`. These identify different builds, explicitly;
both must compile the bundle without residual-policy warnings.
Network-denied verify installs nothing.
The plan template is absent from `.specify/templates/`; this plan supplies
the required technical context, constitution check, structure, research,
data model, contracts and quickstart directly.

## Constitution check

0007 and 0009 require pinned exec proof and a deterministic design check.
0050 refines refusals and the 0035/0045 proposals without accepting them or
rewriting historical decisions. Preserve all spec non-goals and guarantees.
Inline measured tests remain the default; an owner harness needs a concrete
cold diagnostic path under 0020, not merely private seam access. All seams
are `#[cfg(test)]`, installed by tests, instance-local where possible. Real
protocols, deterministic ordering, no sleep-as-sync and no weakened coverage.

The original chief commit contained spec.md, plan.md, tasks.md and 0050.
The resume chief commits only the three existing spec artifacts; 0050 stays
proposed. For the third resume, the operator explicitly authorizes implementation
to amend §4 in place. The original code and documentation-index work below
is delivered background, not authorization to expand this documentation change.

## Resume approach — 2026-10-19 third resume (controlling)

Start from `ad02e3c` on the existing branch. The operator's 2026-10-19 chat
ruling governs the amendment; spec D1–D5 reconcile the panel and intake.
The next section records the second resume's approach, not current scope.

1. Chief confirms the existing design in only
   `specs/delivery-gates/{spec,plan,tasks}.md`, aligns its current residual
   requirements, signs the commit, and runs the existing canonical design
   check against that clean HEAD. No new semantic decision is drafted.
2. Implementation amends `docs/decisions/0050-delivery-gates.md` §4 in place,
   including the dated operator marker and engine refusal cause. Retain
   proposed status and name the unchanged structural pin. Reconcile the
   spec residual paragraph/AC8, this plan §4 and T08 with that amendment;
   already-aligned text needs no gratuitous rewrite. Preserve dated entries.
   Append one signed implementation commit after chief. The combined scope
   from `ad02e3c` must be exactly the four documents, not two commits that
   conceal executable changes. No decision-index or accepted-history edits.
3. Prove diff scope and byte-identical policy/test, run the unmodified
   delivery-constitution suite, and reconcile prior execution receipts.
   The prior run explicitly left actual design dispatch/retry and the full
   evaluator truth table open; complete those existing obligations without
   inventing another policy evaluator or counting the compile sweep as proof.
4. Run the complete final-tree gate inventory below, with pinned compile
   owned by verify if this sandbox refuses brokkr. Preserve exact coverage,
   signing and clean-tree requirements; hand off to verify/review/ship without
   pushing or merging. A failed/unrun proof remains outstanding.

The data model, dependencies, schemas and runtime contracts do not change.
The research question is settled by the recorded engine refusal and existing
policy/pin; no engine upgrade or alternative rule architecture is requested.

## Resume approach — 2026-10-19 second resume

The first resume, `resume-point-read-first-epic-120-2c563256`, confirmed the
design in `d37533c` and delivered comparator repair `93992af`. This run,
`resume-point-2-read-first-batch--206e588f`, starts on
`epic120-delivery-constitution` at the operator's tooling commit `83107fb`.
Preserve all history. The original implementation approach below is the
reference for checking the delivered change, not an instruction to rebuild it.

1. Commit the dated confirmation in `specs/delivery-gates/{spec,plan,tasks}.md`
   and run `bash bundles/self/scripts/design-check.sh check .` against that
   clean HEAD. Keep strict HEAD selection and the pinned prerequisite chain.
2. Use the operator's existing `83107fb` identity record without editing it.
   The missing-digest precondition is resolved; actual verify must compare
   the launched bytes with that digest before gates. Preserve its explicit
   provenance: it does not attest a build of `brokkr.sha`. Retain missing and
   mismatched identity refusals, and require warning-free compilation from
   the recorded seat binary and CI's source-pin build. A future authorized
   rebuild updates digest and provenance together in a signed commit.
3. Reconcile original tasks against actual receipts rather than ticking them
   from the implementation message. Execute the actual verify red/green
   controls, design dispatch/retry proof and pinned evaluator truth table.
   The existing delivery harness proves components separately; any necessary
   actual-seat harness repair belongs in a new signed implementation commit,
   with fixed pinned recursion prevention and no candidate skip switch.
   The second-resume ruling rejects deferring AC4's red seat fixture beyond
   this batch: separate component failures and a clean seat run do not
   establish the combined failing path. First locate and reconcile receipts;
   missing evidence is an execution obligation, not automatic code churn.
4. Retain `93992af`'s comparator and reconcile its recorded mutant proof.
   Run the existing `v4-coverage-bool` and `v4b-debug-bool` selftest arms and
   debug-assert green twin as part of final selftest. No-coverage outputs
   must match exactly; release permits only the existing narrow line shape.
   The equivalent hand-written debug-cfg block remains a documented limit,
   not an excuse to widen normalization. No source-token tripwire is added.
5. Run the full final-tree inventory and the issue-specific mutations, preserve
   command/candidate/pin/verdict receipts in ignored run evidence, then hand
   off to verify and review. No push, PR, merge, issue closure or history rewrite.

The design confirmation touches only the three paths above. Any later fixes
use the existing file owners in the table below; the operator's engine digest
belongs to `bundles/self/tooling.json`. No new artifact hierarchy or dependency
is required. Original tasks remain unchecked until their paired proof is
established; unchecked does not mean the corresponding code must be rewritten.

## Research and implementation checkpoints

1. Confirm the pin's sequence exec grammar, result aggregation, failure
   counting and actual directory remount using a minimal real dispatch.
   Inspect the pinned executable's supported identity and policy interfaces.
   Do not guess a version-output format. If no evaluation CLI exists, run
   a small fixture linked to `brokkr_core` at the recorded SHA using already
   available tooling outside the production dependency graph. No shell/jq
   policy clone counts as proof. A sandbox refusal is an explicit unrun
   obligation for verify, not grounds for a silent engine upgrade.
2. Trace every local executable read by coverage, publish proof, design and
   prerequisites, including the guard invoked by its own proof. Inventory
   transitive helpers and lockfiles before relocation. Verify what dispatch
   mounts, including the tooling record; do not infer visibility from bundle
   manifest inclusion alone. Keep required resources beneath the dispatched
   script directory or the engine's existing supported pinned-resource path.
3. Check current compiler expansion differences on the real crate before
   freezing the differential. Compare raw canonical outputs by default.
   Do not strip function bodies, cfg-gated items or debug-only logic to make
   the comparison green. Any demonstrably nonsemantic normalization needs
   a narrow documented rule and red/green fixtures; unresolved legitimate
   differences block the claim rather than broaden a carve-out.
4. Confirm runtime test seams at their owners. The existing awaits metric
   proves a parked await, not a healthz head's dispatch; those are distinct
   obligations. Existing CLI has no idle flags, and existing binary tests
   already parse bound addresses. Use these facts instead of adding knobs.

## Project structure and touched files

| Paths | Implementation responsibility |
|---|---|
| `scripts/coverage-gate.sh`, `tools/coverage-fn-scan/{Cargo.toml,Cargo.lock,src/main.rs}` | First repair package inventory, expansions, scanner diagnostics and compiler fixtures; then relocate full implementations. |
| `bundles/self/scripts/coverage-gate.sh`, `bundles/self/scripts/coverage-fn-scan/` | Canonical gate/selftest and scanner with pinned source/manifests/lock, built into candidate scratch output. No committed build artifacts. |
| `scripts/publish-guard.sh`, `scripts/publish-guard-test.sh`, pinned counterparts | Canonical guard plus real-local-server proof; candidate release inputs stay candidate inputs. Preserve republish refusal and both publication forms. |
| `scripts/design-check.sh`, `.specify/scripts/bash/check-prerequisites.sh`, missing `common.sh`, pinned counterparts | Public shims where required; one canonical validator, prerequisites and minimal common support. No missing-source success. |
| `bundles/self/scripts/verify-seat.sh`, wrapper templates/integrity helper | Direct pinned calls, candidate-root plumbing, tooling identity, integrity failure accumulation, atomic result writing. |
| `bundles/self/{bundle.json,policy.json,tooling.json}` | Final exec check, fail-closed policy destinations, pin identity/resource plumbing. No source-pin upgrade assumed. |
| `bundles/self/scripts/delivery-proof-test.sh`, `scripts/delivery-proof-test.sh`, `.github/workflows/ci.yml` | One focused pinned delivery regression harness, optional thin public dispatcher, executed by verify and CI; actual dispatch/evaluator fixtures with bounded subprocesses. Avoid recursive verify-selftest invocation. |
| `tests/lifecycle.rs`, `tests/common/mod.rs`, `src/api.rs` and a declared owner harness if needed | Synchronized handler 503, separate stalled-body fence proof and named read deadlines; minimal installed test-only ordering seam. |
| `tests/{binary,config_cli,store_contract_docs,listener_contract_docs}.rs`, `src/config.rs` | Port-0 and occupied-port cases, actual configuration wiring proofs, code-linked docs/status checks and narrowly corrected help. |
| `src/pop3.rs`, `src/pop3/alloc_tests.rs`, test allocator support if needed; `src/webhook.rs` | RETR/sizing allocation instrumentation and actual post_json request observation; thread-local scoped counters and cold-path harness declaration. |
| `src/extract.rs`, remaining scratch-fixture owners in `src/` and `tests/` | Decode-window negative control, retained exact-count pins, tempfile guards and unwind probes. |
| `docs/house-rules.md`, `docs/decisions/README.md`, directly affected charters | Accurate proof ownership, structural-vs-by-effect wording, proposed 0050 index and evidence references. No broad #142/#144 sweep or accepted-body rewrite. |

## Implementation approach

### 1. Repair coverage before pinning it (#140)

Use full locked offline Cargo metadata for package/resolve facts. Canonicalize
the root manifest and require a unique match; pass explicit package and
target selectors throughout. Refuse additional application workspace members
and unharvested local normal/build dependency closure, including target-specific
production paths. Dev-only/tooling packages are not silently treated as
production; prove the standalone scanner's separate workspace. Missing or
unclassifiable metadata refuses. Do not filter paths by directory names.

Capture dep-info before expansion, retaining existing fn-bearing/SF checks.
For each root lib/bin compare measured expansion with no-coverage expansion
at the measured profile, and with no-coverage expansion under the release
profile. Use the same installed compiler, explicit flags and separate dump
names. Scan measured production output and actual `cfg(test)` library
expansion with the current token walker. Verify the test expansion contains
the offending fixture; `--profile test` alone must not be assumed to enable
`cfg(test)`. Keep exact count arithmetic and existing carve-outs intact.

Add real-build v1/v2/v3/v3b/v5b/v6 arms to selftest with clean controls and
named expected refusal reasons. Isolate the inner-exclusion fixture; use
token lookahead past visibility/modifiers for a useful function diagnostic,
not a general item parser. Remove the inner walker in a disposable copy to
prove that specific arm turns red. Update the charter's factual sentence.

### 2. Make execution transitively pinned (#123)

Move the repaired proof chain under the bundle. Resolve proof resources
relative to the dispatched script; accept/derive candidate root separately.
Refactor old `cd` assumptions so release manifests and source remain candidate
inputs. Public scripts contain only fixed dispatch logic and argument
forwarding. Delete duplicate full implementations, including the old scanner
crate; keep any necessary compatibility entry point tiny and integrity-checked.
Build the scanner offline/locked from pinned manifests into candidate
`target/coverage/` scratch space; do not trust an arbitrary cached executable.

Pin wrapper expectations and the tooling record in the resources available
to the seat. Verify checks the actual launched brokkr identity against that
record before using its compiler. A source SHA must be established through
the recorded engine's trusted identity mechanism or an independently bound
binary digest; unknown identity refuses rather than comparing only version.
Do not add an ungrounded binary checksum or let a candidate value supply it.

Record wrapper integrity failure without executing the wrapper. For the
combined tampered-wrapper/uncovered-candidate proof, continue the canonical
coverage invocation under the existing seat deadline, retain both diagnostics
and return failure regardless of subsequent results. This narrow continuation
is unconditional for that failure class, not a test bypass switch. Other
missing critical pinned resources fail closed immediately. Write JSON using
existing JSON-capable tooling to a sibling temporary file then rename; errors
cannot reuse a stale successful result.

Actual verify-path fixtures need complete prerequisites and isolated candidate
workspaces, so earlier gates cannot accidentally preempt the intended proof.
If a delivery harness is itself a verify gate, invoke a low-level seat for
fixtures without recursively invoking the same harness; the separation is
fixed in pinned code, never a candidate-controlled skip flag. Assert coverage
diagnostics and result shape, not only nonzero exit. Exercise a clean control,
wrapper/helper substitutions, old pinned resources versus candidate edits,
bundle-digest changes and mismatched brokkr identity.

### 3. Wire design validation (#124)

Add the final exec check using the grammar proved in checkpoint 1. Remove
the drafted-to-implement route and add an explicit drafted hard stop. Retain
the retry and second-failure hard-stop rules with proved counter semantics.
If the engine requires a separate validation phase, carry failure accounting
across it explicitly and prove exactly one chief retry; do not leave both
routes active.

Use NUL-delimited Git plumbing on HEAD's committed change paths to derive
one kebab-case slug. Validate committed blobs and matching worktree files;
include declared decisions in trackedness and cleanliness checks. No `.git`
directory assumption; `git rev-parse` works with linked-worktree gitfiles.
Explicit `Decision: NNNN` declarations require exactly one proposed record
and a relative spec link. Historical authority links do not declare a new
decision. Restrict declared records to `docs/decisions/` and refuse ambiguous
matches, missing files or invalid status. Pass the selected feature path to
the pinned prerequisite checker rather than rediscovering it from branch
name or mtime. Return only the engine's exact result/inputs/notes schema.

Use temporary Git fixture repositories for malformed and clean designs.
Run actual exec dispatch with prompt/result files, plus real engine transition
proofs. Test dirty declared decisions as well as dirty spec files; test two
slugs, zero slugs, unrelated HEAD commits and linked worktrees.

### 4. Require residual safety facts (#125)

The behavior is already implemented by `ad02e3c`; this resume aligns
proposed 0050 §4 and the design text under the operator ruling of 2026-10-19.
Describe specific stops first where the engine admits them, concretely:
`REVIEW-RESIDUAL-ABOVE-MEDIUM` → `REVIEW-RESIDUAL-OK` →
`REVIEW-RESIDUAL-UNSAFE`. The allow requires explicit boolean
`has_security_residual: false` and `max_residual_severity_at_most: "medium"`.
The final rule has no `when` and stops hard for remaining security residuals
and absent/null/malformed facts. A separate security stop ahead of the allow
leaves an unknown-only fallback dead under the engine's present-valuation
sweep; record that forcing cause in the dated §4 amendment.

Keep policy and `residual_admission_requires_explicit_safe_facts_and_unknowns_stop`
unchanged. Do not default, coerce, normalize case or infer security from
severity. Enumerate supported levels from the pinned evaluator and assert
AC8's first-match identities, including invalid security paired with
above-medium severity. Exercise clean-review routes and security-hold.
Capture compiler stdout and stderr without warning filters. Structural pins
and a zero-unruled sweep do not replace the actual evaluator truth table;
missing evaluator access leaves that proof open.

### 5. Restore tests that distinguish broken behavior (#143)

Keep the public stalled-body deadline/liveness test. For the mandatory
healthz stopping reply, establish handler entry and stop observation with
an installed test-only owner seam if public observations cannot prove that
order. Assert status and body unconditionally over real HTTP. Put cold
diagnostics in the owner harness; do not expose a production test API or
claim a parked await proves a different handler. Use the server's own fence
clock with a small named allowance solely to observe expiry.

Audit all helper reads, including byte/read-until/EOF paths. Use existing
deadline helpers and add only missing forms. Run silent local peers in
bounded subprocesses to capture named panics; an outer watchdog distinguishes
the test's expected timeout from a removed deadline.

Tie documentation numbers to public constants/defaults or owner tests;
avoid exporting new runtime APIs solely for string comparisons. Parse
supported status spellings from decision headers and compare index/file
values in both directions using fixture copies for acceptance mutations.
Exercise independent idle overrides through running servers or owner config
wiring, because copying and assigning struct fields proves nothing and no
idle CLI flags exist. Correct and pin only relevant Host/Origin help text.
For binds, use child-owned port 0 and ready-line addresses for successful
connections; hold occupied listeners throughout separate env-refusal cases
for each protocol, so ignoring an env value still fails.

Use a scoped thread-local, nonallocating test counter around real RETR
construction and sizing, excluding setup and other runtime threads. Pin
allocation counts/bytes tightly enough to catch a second message-sized buffer
and even one allocation per sized mail; include wire-content correctness.
Install only one test allocator if one is needed, forwarding transparently
when disarmed; use a Drop guard to disarm after panics. Observe webhook body
sharing after `post_json` constructs the actual request, before send, so its
caller cannot hide a copy. Keep runtime work on the observed thread or move
the counter scope with the actual operation.

Convert scratch ownership to existing tempfile guards; prove unwind cleanup
with child tests and before/after inventories in the real temp root. Never
delete unrelated artifacts. For decode, try a valid numeric reference with
leading zeros and semicolon beyond the 40-character window, including a
boundary-valid control. If output is unchanged by the exact mutant, instrument
the real search traversal rather than a disconnected accounting counter.
Preserve batch 4's ASCII numeric checks and full entity-table fidelity.

## Data model and contracts

No database or mail model change. Proof identity is the tuple of pinned
bundle digest, established brokkr identity, candidate commit and selected
spec path. Logs additionally name fixture/mutant and command exit/result.
Logs under `.brokkr/state/` are evidence, never inputs authorizing passage.

Public gate commands and argument forwarding remain compatible. Seat results
contain exactly `result`, `inputs`, `notes`; chief uses `drafted`, design
check uses `designed|fail`, verify uses `pass|fail`. Atomic output does not
turn missing output into success. Residual fields remain typed engine inputs;
the policy changes admission, not their schema. New decisions remain proposed
and only explicit declarations are structural design obligations.

## Quickstart and validation

The initial branch was cut from current main; on resume stay on the existing
`epic120-delivery-constitution` and append to `ad02e3c`. Do not recut, reset,
amend or squash. Commit exactly the chief's three confirmation files,
GPG-signed, subject:
`Confirm the residual documentation amendment (#123, #124, #125, #140, #143)`.
Implementation commits map each issue to decisive test names and receipts.
Never push, open a PR, merge or close issues from this run.

For implementation completion, run `cargo fmt --all` before checks, then
`cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, `bash scripts/coverage-gate.sh`,
`bash scripts/coverage-gate.sh selftest`, `bash scripts/publish-guard-test.sh`
and `cargo deny check licenses`. Run the new delivery proof harness, the
fail-closed native-tls absence check, and pinned
`brokkr compile --bundle bundles/self` in verify. No TMPDIR override, installs
or warning suppression. Retain required CI-only jobs from house rules.
Run the named negative controls in disposable candidates, restore them and
finish with all gates on the final tree. Logs of blocked/unrun commands must
remain distinct from passed proofs; a baseline compile warning is not the
post-repair warning-free result.

## Risks

- **Documentation reintroduces the dead rule.** Align the four amendment
  surfaces with the existing three-rule pin and record the engine refusal in
  0050 §4. The final stop deliberately shares one ID for two causes; consult
  typed journal inputs and preserve first-match precedence when diagnosing it.
- **Confirmation is mistaken for completion.** Keep D1–D5 and all paired
  proofs open until evidenced. Two signed commits are required by the phase
  split; verify their combined four-document scope and distinguish structural
  tests, compile sweep, evaluator execution and actual dispatch receipts.

- **Recorded identity differs from source-pin provenance.** The operator's
  digest is present, but it identifies a different build. Verify launched
  bytes, preserve mismatch refusal and honest provenance, and require both
  engines' warning-free compile; do not infer general engine equivalence.
- **The comparator allowance can be widened accidentally.** Keep `93992af`'s
  strict no-coverage comparison, release-only shape and both boolean negative
  controls. The hand-written debug-cfg shape remains a known limit; source
  review must not mistake expanded-source comparison for behavior equivalence.
- **Pinned resources unavailable after dispatch.** Confirm actual remounts
  before relocation, then exercise nested helper and tooling reads in the
  real seat. Keep candidate paths separate and missing resources fatal.
- **Compiler comparison false refusals or cost.** Keep the measured profile,
  use explicit units/configurations, preserve dumps and clean twins. No
  semantic stripping. Expect extra expansion/release work; optimize only
  after correctness and retain the existing seat deadline.
- **Metadata graph accidentally excludes production.** Traverse Cargo
  resolution facts, handle target-specific edges and external local paths,
  refuse unsupported cases and prove standalone tooling separately.
- **Self-certifying fixtures.** Invoke real entry points and evaluator, check
  named failures and restored green twins, and prevent recursion through a
  fixed runner structure. Never count early unrelated refusal as the target
  proof or relabel static inspection as executed evidence.
- **Strict HEAD identity complicates resumes.** Require a fresh chief commit
  containing the single design; do not guess from branch history or mtime.
- **Shutdown tests assert an impossible window.** Honor accepted 0027's
  handler condition with an owner seam and keep independent liveness proof.
  Never alter wire shutdown semantics merely to force the desired result.
- **Allocation/timing tests see unrelated work.** Scoped thread-local
  counters, fixture setup outside measurements, real operation observation,
  named bounded reads and deterministic seams prevent false positives.
- **Test-only code evades exact coverage.** Inline is default; cold-path
  harnesses are declared beside owners and must execute all production
  branches. No coverage exclusions or floor changes.
- **Sandbox or signing prevents delivery.** Preserve draft artifacts and
  report the exact failed operation; never fabricate a commit or use unsigned
  fallback. Brokkr refusal is deferred to its authorized verify seat; cargo
  refusal follows the commission's patch-and-stop rule. No completion claim
  while any required execution or commit remains outstanding.

## Recovery reconciliation — 2026-10-24 (NEXT-171)

The recovery uses the existing recorded-core harness and committed design;
raw outputs and identities are indexed in
[`delivery-proof-residuals/receipts.md`](../delivery-proof-residuals/receipts.md).
The exact 105-row evaluator and its negative control, the 12 real aggregation
unit regressions, warning-free pinned self compile, and targeted constitution
test satisfy the original T08 proof. Its dated task entry is checked with
these bounded receipts.

R04 stays open: this seat produced no actual pinned verify dispatch combining
shim integrity and uncovered-production diagnostics, no covered live control,
and no actual design validator retry dispatch. The recovery's inherited
preflight says the canonical `.forge/forge.db` is absent and the fleet snapshot
is dated 2026-10-23; it assigns overlapping `verify-seat.sh` hunks to #172 and
#206. Those reservations were not refreshed here, so neither canonical gate
script was edited. The self-provider lifecycle remains open for the precise
seat limitation and prior DSH transport report recorded in preflight.

D04 stays open for all unsatisfied parts of AC1–AC13. Existing older text
requesting signed author commits is historical: accepted 0056 governs this
recovery, so its implementation commit is unsigned. This addendum does not
accept 0072, authorize a provider fallback, or claim ship readiness.
