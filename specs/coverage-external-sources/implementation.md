# Implementation evidence — issue #219

Implemented locally against design HEAD `505a91e` on 2026-10-25. The operator's
packet and engine digests were checked before work. Fresh committed-candidate
verify and protected review belong to the subsequent delivery run.

## Accounting and regression proof

This main candidate already rejects external production prerequisites in
`normalized_entry` with `resolves outside the workspace` and the offending
spelling. The historical recovery scanner's lexical C/E protocol is absent.
The real reproduction confirmed that the existing generic refusal covers all
four spellings, including aliases to an otherwise measured workspace file.
Following the design's tests-only option, the scanner requires no patch; its two
raw-attribute tests remain intact. No #175 recovery tests were present at entry.

The pinned coverage selftest now builds six dependency-free temporary crates:
Cargo registry and git aliases, a Rustup toolchain alias, a rustc-prefix alias,
an ordinary alias control, and a workspace inside an active ignored root.
Each real `build.rs` creates a symlink to workspace `src/real.rs`; the library
compiles both spellings and a binary executes both branches of both functions.
Installed Cargo/rustc/rustdoc paths are resolved before substituting owned
Cargo/Rustup homes. Everything builds offline, without host `/rustc` writes.

`cargo llvm-cov run --branch --no-report` produces the instrumented production
units. Paired `report --lcov` exports read those same executables and profiles:
the default omits each external alias while measuring its ordinary spelling;
the test-only `--no-default-ignore-filename-regex` control contains executed
line, branch and function records for both spellings. The owned nested
`/rustc/0123456789abcdef/` path demonstrably matches the installed exporter's
rustc filter; its directory name alone is not taken as proof.

Before expecting refusal, the fixture independently finds the exact alias in
the compiler's production `.rlib` rule and checks file identity and executable
control records. That unit's dep-info is copied byte for byte for harvesting.
Cargo's final artifact summaries canonicalize aliases and carry transitive
build-script inputs, so they cannot supply this spelling evidence. The binary
is the execution driver; this proof harvests the production library unit.
Setup, compilation, export or evidence failure fails the selftest outright.

Each external harvest must refuse with both its category and exact source in
one invocation. Failed inventory is never joined to the report. The ordinary
control passes the real inventory/report join with positive totals and integer
zero missed in every dimension. The ignored workspace successfully harvests
its internal sources, but its default SF-less report triggers the existing
named-artifact validation refusal, backed by the live unfiltered control.
Existing accounting, containment refusals, exact arithmetic and expansion
checks are unchanged.

## Mutation and integrity

The isolated mutant silently filters external absolute prerequisites out of
`normalized_set` before normalization. Command and status evidence is retained
in `.forge/epic192/issue219/implementation-checks/mutation-receipt.json`, with
`mutant.patch`, `mutant-selftest.log` and `restored-selftest.log` beside it.
The copied gate uses a separate helper build target so mutant artifacts cannot
substitute for the canonical helper.

On the final gate bytes, the mutant's full selftest exits 1 with exactly four
`ignored production alias refuses by name: expected refusal, got success`
failures: registry, git, rustup and rustc. Compilation and all other arms
complete. The restored canonical full selftest exits 0. No mutant source is
part of the committed candidate. The retained `mutate.py` constructs the
mutation and asserts these diagnostic/status requirements.

The seven public-shim hashes were regenerated and strictly checked together
with the constitution table update. Dispatchers did not change, so
`shims.sha256` truthfully retains identical bytes and digest
`72d34b479a900da837f34a4c3c3066f5964d067050166f7ab4067ff076978566`.
The table now explicitly binds the changed canonical `coverage-gate.sh` digest
`03b22e093cb95946fedd82971b8f27fa75e60b02bf9fedcd4ee5b5f7739517a8`.

## Local validation and handoff

All nine commands in the house gate inventory passed on the implementation,
using the checksum-verified candidate-local `.forge/bin/brokkr` and offline
Cargo. The standalone scanner suite (2 tests), delivery constitution suite
(11 tests), design selftest, delivery tamper-boundary proof, strict shim checks
and fail-closed native-tls absence probe also passed. Per-command logs and
exit statuses are retained under
`.forge/epic192/issue219/implementation-checks/` in `progress.json` and the
named logs. Workspace transport timeouts did not establish a verdict; the
completed runner receipt and command logs establish the results.

Exact coverage evidence is `target/coverage/run.GMaAmv/{lcov.info,coverage-summary.json}`:
13,878/13,878 lines, 782/782 branches, 1,508/1,508 functions; missed counts are
integer zero. The freshly compiled self bundle digest is
`0b5f1337a61f47e028a65c58613fb499fa655ad9403dc816f0318377a2572360`,
with the changed gate and unchanged scanner present in its manifest.

The operator's assistant must enter fresh verify against the committed
candidate's pinned bytes using the candidate's existing recipe routing; this
implementation run cannot verify bytes pinned before its edits. CI/operator
retain the platform and live-service proofs assigned by the house rules.
No additional local acceptance criteria or decision acceptance is introduced.
