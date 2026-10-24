# Implementation plan — release proof residuals

## Technical context

Continuation dated 2026-10-24 for run
`epic-192-issue-186-residual-foll-fa975081`. One Rust crate with an existing
Bash proof and Python loopback fixture; no new dependency or runtime code.
Only `.specify/templates/spec-template.md` is present; no plan template exists
in this checkout. This plan supplies its required sections directly.

Current inspected base: `f00d8b532d7e255086913a53b21aa47c27fbe7c6`, tree
`1c4d0f3edc82a037033386e8a7f09802c77d7351`. History runs through `238372d`
(engine pin), `79c3df9` (design), `667b825` (implementation), `f00d8b5`
(receipt). PR212/`df8630f` is historical original-design ancestry. The receipt's
`2c6f3e1`/`2dce0b4` are historical claims, not silently mapped local identities.
Capture the later design and implementation heads/trees at execution time.

## Constitution check

Preserve frozen 0001–0003, ingest extraction 0005 and accepted
0019/0020/0045/0050/0056/0057. This internal proof-helper repair restores the
existing assertion vocabulary and evidence obligations; it changes no public
contract or ruling and needs no new decision. 0051 remains proposed. Preserve
current compatible release behavior. No Rust, policy, pin, shim or dependency
change is authorized. Exact coverage remains 100% of lines/branches/functions.

Triage selected `engine`; four independent configured reviewers are the smallest
permitted review under 0057: DashScope Qwen Max correctness, Claude Opus
security, official DeepSeek Flash guarantees, DashScope GLM 5.3 tests. No Pro,
fallback, recipe edits or chief-side reviewer dispatch. Protected review and
security hold remain. Commit unsigned; do not push, merge, tag, publish or
access GHCR. Root owns integration/CI; external 18.5 stays pending.

## Project structure and files

| Path | Continuation work |
|---|---|
| `bundles/self/scripts/publish-guard-test.sh` | Add one closed mode guard and one token-log collision refusal regression; preserve existing real fixture and all recognized-mode checks. |
| `specs/release-proof-residuals/proof-receipt.md` | Append dated current-pin/base addendum, keeping the old receipt byte-for-byte as a prefix. Implementation owns this edit. |
| `specs/release-proof-residuals/plan.md` | Correct base/current scope now; later attach actual execution applicability without replacing historical evidence. |
| `specs/release-proof-residuals/tasks.md` | Preserve obligation identities; reconcile completion only with authentic run-journal evidence. |
| `specs/release-proof-residuals/spec.md` | Chief's continuation synthesis and AC9–AC14, retaining AC1–AC8 and external checklist. |
| Ignored run-private `.forge/` | Logs, guard-only disposable mutation, command receipts; never fabricate or replace a journal. |

The chief commits exactly spec.md, plan.md and tasks.md. Later implementation
is bounded to the canonical proof and receipt/plan/tasks. Existing chart,
package helper, Cargo anchors, production guard and public dispatchers remain
owned by their present proofs. Reopen them only through explicit scope review
if new evidence demonstrates another defect.

## Research and rulings

All nine existing `assert_requests` callers use `none`, `token` or `manifest`.
Unknown values currently construct the token expectation and can pass. An empty
log is therefore a vacuous regression fixture. Both positions recommend reusing
an existing token-stage HTTP case and its completed log. The chosen guard is a
small `case` before the current expectation construction; default calls `fail`
with description and invalid mode. No argument framework or new process.

Chief verified the candidate-local binary and patch hashes against tooling.json;
this is identity evidence, not an engine rebuild or gate result. No journal is
visible in this boxed workspace. Supplied context proves this run's triage and
position results, not verify/review/ship or historical task completion. Obtain
only authentic authorized read-only records when available; otherwise retain
unchecked dispositions. No importer, new evidence parser or fabricated journal.

## Data model and contracts

No application schema/API changes. The private helper accepts the closed set
`none|token|manifest`; its existing exact request-log strings remain authoritative.
Invalid modes must fail before log comparison and identify the caller and value.
Use a subshell because `fail` exits; explicitly assert nonzero and the named
refusal without depending on `set -e` inside a conditional.

The append-only receipt uses prose/tables, not a new engine schema. Each new proof
records command, exit, log locator, base/candidate/tree, changed-file applicability,
script hash, executable path/hash and input pin. Attach authentic run event or
journal receipt references for task completion separately from local command logs.
Describe unavailable records explicitly. Preserve the historical receipt as
bytes, including its old absolute sibling command; mark that command historical
in the addendum instead of rerunning in the sibling checkout.

Current engine identity (paths relative to `bundles/self/`, order significant):

| Input | Recorded identity |
|---|---|
| Source base | `731189f9097308c01ae4f2b4c9caea5f41224844` |
| `tooling-patches/0001-preserve-review-residual-facts.patch` | `5d4da2217135c5a2ca14354ae9907623cf67ff0076b078277cb77a09ad3020b5` |
| `tooling-patches/0002-preserve-review-scope-and-fixes.patch` | `361272eff344a5d5e05b832281407a99c3f1a77fc1fbc5216e19fb4ce7487173` |
| `tooling-patches/0003-bind-verified-candidate.patch` | `4098228e7649d1bda8961bb491aa29d982b462c420470e265f8b9acf0a148d96` |
| Candidate-local `.forge/bin/brokkr` SHA-256 | `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` |

Version 0.12.0 alone is insufficient. Pin absence/mismatch blocks compile proof;
never substitute PATH/sibling binaries or edit the pin to make the check pass.
Record the old e525/597f/784 identities as historical and the actual suite-level
passed/failed/ignored totals for the requested 785-test base and final candidate.
The new shell assertion need not change the Rust count. Explain discrepancies;
missing logs do not establish either count.

## Ordered implementation approach

1. Capture current head/tree/status and source/script/shim hashes; compare the
   receipt at f00d8b5 to a saved immutable prefix. Verify exact local engine and
   ordered patches before invoking it. Inventory available authentic journal
   records; record missing evidence without inferring completion. If no applicable
   base test log exists, run the locked workspace suite before the helper edit.
2. Add the helper's mode guard. Immediately after the existing token-500 case's
   successful `token` assertion, invoke an invalid mode against the SAME real
   recorded sequence. Capture status/stderr privately in a subshell, require the
   invalid-mode diagnostic and explicit parent failure if it unexpectedly succeeds.
   Preserve the nine current assertions and fixture lifetime/credential controls.
3. Run the full canonical loopback proof. Copy guard/test adjacent to one another
   in an ignored private directory with the existing relative fixture/workflow
   layout. Remove only the new guard in the copy; keep the new regression. Require
   failure specifically at invalid-mode admission, not syntax, log mismatch,
   prerequisite or workflow errors. Restore exact bytes, compare hashes and run
   the full positive proof again. Retain commands, exits, diagnostics and logs.
4. Execute every gate from the sole [house inventory](../../docs/house-rules.md#gates--part-of-every-change)
   on final implementation inputs, including its locked commands and coverage
   selftest. Check public shim hashes and native-tls absence fail-closed with a
   successful complete dependency-tree command before inspecting absence. Keep
   local candidate execution separate from engine-dispatched immutable verification;
   verify runs pinned canonical proofs offline and installs nothing. A changed
   implementation invalidates affected earlier passes. Record failures honestly.
5. Append a dated receipt addendum binding observed reruns to the identities above
   and actual candidate/base/tree. Do not claim independently rebuilding the engine.
   Compare old receipt bytes as a prefix. Add per-task evidence/disposition notes;
   only authentic journal-backed proof of the whole obligation earns a checkmark.
   T7 stays open without immutable verify; T8 without review/ship/root CI. A partial
   task may be split while preserving its unresolved original obligation.
6. Hand off to the existing full engine route. Root retains actual-event chart
   history/render/bump, Docker smoke, network RustSec, tag-only same-run rate and
   prescribed real port-80 CI proofs. Preserve all six external receipt categories
   and `18.5: EXPLICIT EXTERNAL PENDING`. Neither local proof nor a documented
   pending receipt closes external 18.5, #186 or #192.

## Quickstart

From this candidate root, first verify tooling.json's complete identity and use
this checkout's absolute `.forge/bin/brokkr` path. Entry commands:

```bash
cargo test --workspace --locked
bash bundles/self/scripts/publish-guard-test.sh
sha256sum -c bundles/self/scripts/shims.sha256
```

Capture their actual exits and logs in run-private evidence. The full house
inventory remains required; the three commands above are not its replacement.
Use offline Cargo/tool execution and no installation or registry access. After
the chief commit, `bash bundles/self/scripts/design-check.sh check .` is a local
structural preflight; the sequence's pinned final check alone owns advancement.
No design preflight or identity observation certifies later implementation gates.

## Risks

| Risk | Mitigation and retained cost |
|---|---|
| Unknown mode passes through token fallback | Explicit closed `case`; the real token-log collision test and guard-only negative control prove discrimination. |
| Bash conditional errexit masks a failed assertion | Subshell isolation with explicit status/diagnostic checks and explicit parent failure on unexpected success. |
| Negative control fails for an unrelated reason | Preserve adjacent layout and original workflow inputs; require the owning invalid-mode-admission diagnostic, then exact restore and full positive. |
| Historical receipt is overwritten or used for current proof | Byte-prefix assertion against f00d8b5; dated addendum identifying old run/pin/path/count and actual current command/candidate applicability. |
| Test counts or engine provenance are guessed | Full source/patch/binary hashes and per-suite logs; count discrepancies stay explained, missing evidence stays open. Checking hashes does not prove source rebuild. |
| Compound tasks are falsely checked | Cite authentic event/receipt and scope per obligation; split partial local/owner duties or leave original unchecked. Missing journal access costs visible incompleteness. |
| Candidate green is called immutable verify green | Separate command receipts from actual pinned dispatch; preserve shim checks, pin and canonical chain. Later implementation reruns affected proofs. |
| Scope expands into publication or runtime work | Four-file implementation boundary and retained engine review; no GHCR access. Frozen storage/wait/prune/extraction contracts remain intact. |
| Existing external release limitations persist | Retain outage refusal, concurrent probe/push race and image-success/chart-failure costs; no retry/rollback/immutability contract. All external receipt work remains pending. |

## Dated clarification — 2026-10-24

This continuation replaces the plan's stale present-tense PR212 base and broad
implementation steps with the f00d8b5 residual scope. Original chart/package and
publication proof requirements remain AC1–AC8 in spec.md and T1–T9 in tasks.md;
this clarification neither erases them nor marks them complete. Historical
receipt bytes are preserved by the later append-only addendum, not edited here.
