# Implementation plan — coverage inputs

Implements [spec.md](spec.md); ordered proofs are in [tasks.md](tasks.md).

## Technical context

One measured Rust application crate; a standalone syn/proc-macro2 scanner under
the pinned bundle; Bash with `set -euo pipefail`; cargo-llvm-cov and an installed
coverage nightly. Verification is offline and installs nothing. No dependency
or application test-placement change is needed. The repository has no
`.specify/templates/plan-template.md`; this plan supplies the required template
sections directly, with research, data model, contracts and quickstart inline.

## Constitution check

Accepted 0020/0035/0050 already require the behavior this repairs; no new ruling
or operator acceptance is inferred. Frozen guarantees and extraction-at-ingest
remain unchanged. Keep canonical executable proofs under `bundles/self/scripts/`
and fixed dispatch shims untouched. Preserve the full lane selected by triage,
protected review and the hard security hold. All author commits are unsigned;
no push, merge or history rewrite. Only the three design artifacts belong to
the chief commit; positions and result receipts remain ignored run evidence.

Candidate-local Brokkr is
`/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-173-luna1/.forge/bin/brokkr`,
SHA-256 `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Chief verified this digest. Recheck before subsequent compile/engine/provider
work; absence or mismatch stops dispatch. Retain the operator recipe commits
and their Luna xhigh implementation, GLM 5.3 test review and Flash-only routes.
The engine owns routing and the shared journal; do not manufacture its receipts.

## Project structure and files touched

| File | Work |
|---|---|
| `bundles/self/scripts/coverage-gate.sh` | Raw-off source predicate; checked target reservation; explicit clean/log boundary; both checked inventories; existing selftest additions; corresponding comments |
| `bundles/self/scripts/coverage-fn-scan/src/main.rs` | Narrow semantic identifier comparison in exclusion matching, preserving token traversal and derive exception |
| `specs/coverage-inputs/{spec,plan,tasks}.md` | Design and later honest proof disposition; retain dated clarifications if returned |

Existing `tests/delivery_constitution.rs` and `tests/store_contract_docs.rs`
are regression consumers to run, not expected edit targets. Do not change
public shims, manifests/locks, CI, verifier policy, accepted decisions or
application sources to accommodate this repair.

## Research and implementation approach

1. Preserve baseline receipts before editing. Confirm the relevant functions,
   compiler/nightly identity and the installed scanner dependencies. Reproduce
   raw-off codegen/expansion and the two independent check results. Probe raw
   exclusion attribute names with real rustc; unsupported syntax needs a
   recorded compiler refusal, not an assumed scanner exemption. Triage evidence
   is a starting observation, not implementation proof.
2. In `exclusion_kind`, compare semantic identifiers at the local attribute and
   first-argument positions, accepting raw `off` and raw exclusion names where
   applicable. Never rewrite entire token dumps or function identities. Keep
   `is_automatically_derived` and its carve-out no broader than today. Extend
   the existing grep to recognize raw-off while retaining its existing spacing,
   punctuation and error behavior; keep it a conservative tripwire.
3. In `resolve_build_dir`, check parent preparation, then attempt a single
   option-safe non-`-p` mkdir of the supplied leaf. Refuse failure, export the
   exact successful path, retain the leaf through harvest and after exit. Do
   not use an existence-check/mkdir-p pair or clean a preexisting target.
   Clarify `BUILD_DIR_OWNED` as automatic cleanup authority, not mkdir provenance.
   Keep default fresh allocation and cleanup, with checked allocation failures.
4. Move the actual clean invocation into a narrowly scoped function only if
   needed for selftest execution. Use an explicit checked command with both
   streams redirected to `run_dir/clean.log`; on failure preserve diagnostics
   and name the log. The main path must call this same function before llvm-cov
   measurement. No generic runner abstraction or runtime injection switch.
5. Capture harness `find | sort` under the existing pipefail into a checked
   assignment before the loop. Skip iteration for a successful empty capture.
   Check hidden-directory find in the same local style before testing hits.
   Keep stderr visible and distinct inventory-refusal messages. Do not consume
   partial output, retry, or change declaration/hidden-directory policy.
6. Extend the existing selftest using test-local command replacements and child
   shells so refusals cannot exit the test driver. Exercise actual production
   helpers in both ordinary and conditional contexts. Use two processes plus
   a pipe/barrier handshake for target contention. Use a blocked parent path
   or injected mkdir failure for deterministic creation errors even as root.
   Prove retained explicit/default-only cleanup by exercising the production
   cleanup path; do not duplicate trap logic just for a passing assertion.
7. Run focused proofs and baseline/mutant controls first, then the complete
   charter inventory on the final implementation. Keep selftest, real candidate
   coverage and hermetic verification distinct. Record command, exit status,
   candidate/pin identity and evidence path for each receipt. Missing tools or
   external evidence remain outstanding and cannot become success by inference.

## Data model and contracts

No application schema, endpoint or persistent format changes. The gate has a
small invocation state machine: select/reserve target → free-space/toolchain
preflight → create durable evidence → checked clean → measure → validate report
→ harvest → compare/scan → exact verdict. A reservation is represented by the
fresh directory itself, with no lease metadata. Failure never advances to a
later stage. Explicit selection retains its reservation even on failure; the
default trap removes only its generated build tree. Evidence is independent.

Source inventories are complete successful strings before they become rows;
newline-path assumptions are unchanged. `clean.log` adds evidence, not a new
public machine-readable schema. All new errors use `coverage refusal:` plus
the failed stage; no exact numerical subprocess exit code is promised beyond
nonzero. For the scanner, canonical exclusion diagnostics and function context
remain the public troubleshooting surface.

## Quickstart and validation

From this candidate root, first verify the exact binary above using `sha256sum`.
During implementation, run `bash scripts/coverage-gate.sh selftest`, then
`bash scripts/coverage-gate.sh` with the variable unset for the default target.
For an explicit target, set `CARGO_LLVM_COV_TARGET_DIR` to a never-created leaf
under a writable filesystem with the required space; do not precreate the leaf
and choose a different leaf for the next invocation. Existing parents are fine.
Keep failed and successful run logs in ignored run evidence.

Execute every command in the charter's sole gate inventory, with `--locked`
where specified and this candidate's isolated outputs. Use the absolute pinned
Brokkr for `compile --bundle bundles/self`. Also run the pinned design-check
selftest and delivery tamper proof required by verify, plus fail-closed
native-tls absence. Actual hermetic verify and protected review are engine
obligations. Do not treat an ad-hoc local invocation as their journal verdict.

The design artifact preflight is `bash bundles/self/scripts/design-check.sh check .`
after committing; the sequence's final pinned check remains the source of the
design verdict. Keep this chief design commit as HEAD through that check.
CI-owned Helm contract/history/selftest, real-Docker smoke, network RustSec audit,
applicable tag-only release-rate proof and real port-80 exchange remain explicitly
pending until corresponding source-bound evidence is available. This design
phase does not claim implementation gates passed.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Existing scripts precreate/cache explicit targets | Named refusal with new-leaf guidance; preserve exact location and all preexisting data. Reuse compatibility is deliberately lost to enforce 0020. |
| Alias, collision or abandoned build contaminates evidence | Atomic leaf creation; hold/retain the directory; prove symlink and two-process refusals. Ancestor replacement and uncooperative external writers remain outside the guarantee. |
| Cold builds and retained evidence exhaust disk | Keep free-space preflight and default cleanup; explicit caller owns cleanup. No automatic deletion of arbitrary supplied trees. |
| Raw matching rejects clean tokens or widens a carve-out | Semantic comparison only in exclusion recognition; compiler-produced red/green fixtures and existing derive tests. Grep remains conservatively textual. |
| Conditional shell calls suppress errexit | Explicit status checks; fixtures exercise both caller contexts and post-failure markers. Check find and sorting status before any consumption. |
| A copied test passes while main remains broken | Test actual production functions and top-level sequencing; preserve baseline failure receipts or narrow restored-mechanism controls. |
| Proof code escapes the pin | Edit only canonical owners; retain shim integrity and transitive verify/tamper tests. |
| Newline paths or canonical proof limits remain | Name unchanged filename assumptions, derive-provenance and uninstantiated-generic limits; do not claim this bounded repair solves them. |
| A green draft is reported as delivery | Separate structural draft validation, implementation tests, exact coverage, hermetic verify/review and CI receipts; leave unexecuted tasks unchecked. |
