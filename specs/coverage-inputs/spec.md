# Coverage inputs belong to one successful run

Issue #173, epic #192. Design date: 2026-10-24. Scope: the five coverage-gate
findings framed by triage for run `epic-192-next-173-luna-xhigh-del-6bd17b44`.
This is a design draft, not an implementation or issue-completion receipt.

## Feature and rationale

Refuse raw-identifier coverage exclusions, reserve a fresh instrumented target
for explicit as well as default directory selection, and require successful
cleaning and complete source inventories before using their results. A perfect
percentage cannot prove the candidate when hidden functions, another build's
artifacts, or an incomplete inventory can change what was measured.

The implementation restores accepted [0020 §3](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0035](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md) and
[0050 §§1–2](../../docs/decisions/0050-delivery-gates.md). Those authorities
already require unique instrumented targets, exclusion refusal in any spelling,
complete by-effect evidence and a pinned proof chain. No semantic amendment or
new decision is declared: the explicit path still selects the exact build
location and remains caller-retained; its previously unchecked reuse violates
the existing unique-run requirement. The protocol contracts and frozen
accept==stored, subscribe-before-check and oldest-only guarantees remain intact.

Triage selected the `engine` lane; this design does not redo or bypass triage.
Its recorded experiment shows nightly-2026-10-22 accepts and code-generates
`#[coverage(r#off)]`. Compiler rejection is therefore not a current solution.
Triage also demonstrated swallowed harness-find failure, but top-level clean
and hidden-directory find already stop under errexit. For the latter two,
the repair is an explicit named failure boundary, not a claim of a reproduced
stale merge or successful hidden-directory check after failure.

## Panel settlement

Both run-local positions were read: `.brokkr/state/design/positions/simplicity.md`
and `.brokkr/state/design/positions/robustness.md`. They agree on the central
architecture; there is no genuine conflict about whether to allow reuse.

- **Adopt both positions' fresh-target refusal.** A checked atomic leaf
  creation wins over cleaned reuse, Cargo locks or an advisory-lock protocol.
  Keep the leaf throughout measurement and harvest. Cost: callers must choose
  a new leaf for every invocation, including retries after a failed or killed
  run; empty precreated directories and mount roots are not suitable targets.
  Retained explicit targets need caller cleanup and cold builds cost time/disk.
- **Adopt simplicity's local repair and robustness's matching boundary.**
  Normalize raw identifiers only where exclusion names/arguments are compared
  in the existing token walk. Prove compiler-permitted raw attribute names as
  well as the raw argument; an argument-only fix could leave the same defect
  one token earlier. Keep the independent conservative grep. Reject a lexer,
  AST replacement, global token rewriting, derive-provenance work and recount.
  Cost: extra focused raw-name fixtures; known canonical-proof limits remain.
- **Adopt robustness's executable failure proofs.** Success/failure command
  injection must execute production control flow, including conditional caller
  contexts where errexit is suppressed. Preserve clean diagnostics and prove
  no measurement follows failure. This costs more selftest cases than a short
  textual patch, but source inspection cannot establish refusal behavior.
- **Choose simplicity's checked in-memory inventories.** Robustness permits
  either checked capture or a temporary file; capture avoids a new file lifecycle.
  Finish and check `find | sort` before iteration and check hidden-directory
  find before interpreting output. Do not retry a partial inventory. Retain the
  existing newline-delimited path convention rather than add a filename-protocol
  redesign; arbitrary newline-containing source paths are outside this repair.
- **Reject broader mechanisms from either option space.** No lock registry,
  stale-lock recovery, recursive deletion of supplied trees, package harvesting
  expansion, new configuration switch, bypass flag, dependency or license change.
  Fresh creation arbitrates cooperating runs; it does not attest a filesystem
  against an external writer replacing ancestors or writing into a live target.

## Required behavior

A nonempty `CARGO_LLVM_COV_TARGET_DIR` must name a new leaf directory. Checked
parent creation may retain current convenience, but only one non-`-p` leaf
`mkdir` establishes the reservation. Existing leaves (directories, files,
symlinks including dangling links) and all creation failures refuse by name
before cleaning or measurement. Paths are quoted and filesystem operands are
option-safe. Do not reinterpret the value as a parent for a random child.
The directory remains caller-retained on success and failure. Default empty
or unset selection retains fresh disk-backed `mktemp` allocation, free-space
preflight and EXIT cleanup; durable run evidence remains separate.

The actual `llvm-cov clean --workspace` command must be explicitly checked,
with stdout/stderr preserved in the invocation's evidence directory. Failure
names the clean stage and log path and stops before measurement, merge, harvest
or summary publication. The two source inventories must similarly name their
own failed stage and retain useful stderr. Partial output is never evidence
of a successful walk; an empty successful inventory remains a valid empty set.

## Acceptance Criteria

- **AC1 — Independent exclusion refusals.** A real compiler-produced expansion
  of a codegen-accepted `#[coverage(r#off)]` fixture refuses through the token
  scanner, and its source independently refuses through grep. Each rejection
  has a named coverage diagnostic and a clean twin. Exercise raw attribute-name
  forms accepted by the installed compiler, ordinary/spaced/comment-separated,
  multiline, inner, macro and cfg_attr exclusions through their applicable
  existing checks. Token-walk controls with unrelated identifiers/strings and
  real compiler-derived impls pass; do not widen the derive carve-out. Grep's
  conservative text matching is not promised to parse Rust strings/comments.
- **AC2 — Fresh explicit target.** A missing leaf succeeds at the exact supplied
  path, including a path with spaces and separately checked parent creation.
  Existing empty/dirty directories, regular files, symlinks and dangling
  symlinks refuse without modifying their data or link referents. Parent/leaf
  creation errors produce a named coverage refusal before clean/build.
- **AC3 — Reservation lifetime and cleanup.** Two cooperating processes using
  one explicit path yield exactly one successful reservation; the loser refuses
  while the winner holds it. A later retry also refuses the retained/abandoned
  leaf. Use a deterministic handshake, never sleeps. Explicit targets and run
  evidence survive normal/failing exits; default build targets alone are
  trap-cleaned. Unset and empty selection remain fresh under `target/coverage/`,
  independent of TMPDIR, with the existing disk-space preflight intact.
- **AC4 — Checked clean.** Injected failure in the actual production clean path
  returns nonzero, names clean and its preserved diagnostic log, and leaves no
  post-clean measurement/merge marker. Successful clean reaches the next stage.
  Prove both normal entry and conditional function invocation; do not depend on
  `set -e` to enforce the boundary. Log-creation failure also cannot proceed.
- **AC5 — Complete inventories.** For each find site, immediate failure and
  failure after valid-looking partial output refuse nonzero with a diagnostic
  identifying that inventory, before interpreting the rows. Harness sorting
  failure also refuses. Empty/valid inventories pass; real hidden-directory and
  undeclared/unguarded-harness violations still refuse. Exercise conditional
  contexts as well as normal entry and preserve the producer's useful stderr.
- **AC6 — Discriminating regression tests.** Extend the existing pinned selftest
  to execute production functions/control flow, not copied code or text-only
  assertions. The baseline scanner/grep, reused-target path and swallowed-find
  path must fail the corresponding new regression assertions. For clean/hidden
  find, assert the missing named boundary rather than invent baseline continued
  measurement. Keep command injection test-local and pair red cases with green.
- **AC7 — Existing proof preserved.** Literal nonzero integer 100/100/100, the
  production file set, measured/no-coverage/release expansion equivalence,
  library-test scan, local-package refusals, shim-integrity checks and all
  existing selftests still hold. Accepted decisions, runtime code, fixed shims,
  verifier/CI policy, recipes and dependencies are unchanged. Retain candidate
  history including `450bef65c581b3a4791cf94dbc145912814bd6ed` and `9b15fda`.
- **AC8 — Delivery evidence.** Implementation/verify execute the complete
  [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
  including exact candidate coverage, coverage selftest, native-tls absence and
  pinned verifier proofs, followed by protected review. Verify the candidate's
  absolute `.forge/bin/brokkr` digest before compile/engine/provider work.
  CI-owned evidence stays pending until actual receipts exist; a design check,
  compiler pass or selftest alone never closes #173. No push, merge, tag or publish.
