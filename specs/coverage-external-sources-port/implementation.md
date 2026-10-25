# Implementation — port issue #219 onto current main

The port adds the shipped 154-line `ignored_source_fixtures` block unchanged
from verified patch 03. It follows main's fresh-target standalone scanner suite
and precedes the existing clean-failure probes. Removing that block yields the
entry gate byte for byte. Main's five scanner tests, instrumented binary-unit
dep-info proofs, exclusion scans, expansion comparator, exact arithmetic and
reservation behavior remain intact. No scanner or application changes are needed:
existing external lexical prerequisite refusal owns the measured-or-refused result.
This does not establish a new canonical symlink-containment guarantee.

The six dependency-free real-crate fixtures cover Cargo registry/git, Rustup
toolchains, the exporter's rustc/hash ignore class, an ordinary alias and an
internal workspace under an ignored root. Private homes and an owned nested
`/rustc/<hash>/` path avoid installed-cache or host `/rustc` writes. A real build
script symlinks `src/real.rs`; both compiled spellings execute both branches.
Paired default/unfiltered exports read the same instrumented binaries and
profiles. Before refusal assertions, independent evidence checks exact alias
membership in the production library's compiler dep-info, file identity and
executed DA/BRDA/FNDA records for both spellings. That unit's dep-info is copied
byte for byte. Four external cases prove default omission and contextual
refusal; the controls prove an exact green inventory join and a named default
report/source refusal for the ignored workspace.

## Mutation proof

Fresh commands, statuses, canonical identities and the isolated omission patch
are retained under `.forge/evidence/issue219-port-implementation/`, outside the
immutable packet. `mutate.py` adapts only the shipped runner's owned paths and
keeps mutant scanner artifacts separate. The mutant skips external absolute
prerequisites before normalization. Its complete selftest exits 1 with exactly
four named-refusal failures caused by successful harvesting: registry, git,
rustup and rustc. Setup, compilation and all other assertions succeed. The
canonical restored full selftest exits 0. Canonical scanner bytes match the
entry digest; no mutation or bypass enters the committed sources.

## Integrity and local gate handoff

The merged canonical gate SHA-256 is
`f96b60bef6582efbce6166a6a8da71ab73aad81e545908e2bf6238e5b36c1fbf`,
explicitly bound by the constitution test. Fixed public shims and their manifest
are unchanged and pass strict checksum validation. The operator packet and
candidate-local engine digest passed verification. All five standalone scanner
tests pass. All nine house-gate commands and the additional scanner,
constitution, design/delivery proof and fail-closed native-tls checks passed on
committed implementation `e3186a74ca0642c70ac16ad14e2b29110ebc4aa1` (tree
`888ee4074994ea3df1a0d028f09e5452dcd905f3`). Commands, exits, log hashes and tested
proof identities are committed in [implementation-gates.json](implementation-gates.json).
The subsequent receipt commit changes documentation only.

The exact coverage report at `target/coverage/run.btbroT` records
14,009/14,009 lines, 788/788 branches and 1,527/1,527 functions: positive totals
and integer zero missed in every dimension. Full selftest confirms all six new
fixtures and main's existing proofs. The fresh self-bundle digest is
`42eca49ef622af176e12ef8d966c27a27e8f4c3d44993da3ed5af96c018ec9e3`, and its manifest
binds the merged gate and unchanged scanner identities. The local gate runner
continued to completion after a 300-second tool-transport timeout; the finished
receipt and per-command logs, rather than that timeout, establish these verdicts.

This discharges the local implementation proofs for T1–T6 and hands off T7's
committed candidate. Fresh pinned recipe verification and protected review are
subsequent seats' obligations; no feature-level SHIP-COMPLETE claim is made here.

The implementation seat does not replace subsequent committed-candidate verify
or protected review. Those must use freshly pinned changed proof resources and
the candidate's existing recipe routing. CI/operator-only platform, network,
settings, sealing and release proofs retain the owners recorded in the spec.
No decision, dependency allowance, engine pin or route changes are introduced.
