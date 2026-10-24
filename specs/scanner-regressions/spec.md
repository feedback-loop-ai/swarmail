# Scanner regressions — issue #175

Date: 2026-10-24. Phase: design draft; implementation and verification remain due.
Scope: epic #192, findings 7.1–7.4, on the supplied rollup-a candidate
observed at `c3d124d`.

## Feature and rationale

Prove the coverage scanner's cannot-tokenize refusal and correct its two
bounded compatibility defects: Cargo library names normalize package dashes
to underscores, and make dep-info encodes a literal dollar as `$$`. Prove
that the coverage selftest harvests dep-info produced by a real instrumented
build. These checks protect the evidence behind the existing coverage verdict.
The scanner already has exclusion-kind tests; preserve them.

Authority is the intake framing and both panel positions under
`.brokkr/state/`, plus `.forge/epic192/issue175/{README,operator-rulings}.md`.
Both packet files were verified against their directory's SHA256SUMS:
README `93e147a3dde1525d49924aadb14f934dbb677ba1290b4dc6063697d362a47442`;
rulings `ce354e19307d268f9e9ab1e8f36c1897fd3d56c8049cbf65ab1f8fed1dac61fa`.
Run-local files are evidence, never committed design artifacts.

## Synthesis and rulings

Adopt both positions' command-level refusal test, literal table expectations,
library-only normalization, single-pass dollar decoding and explicit execution
of the standalone helper suite from the existing selftest. Testing proc-macro2
alone would leave the scanner's error propagation unproved. Root workspace
tests do not execute this helper's independent workspace.

Resolve the provenance choice in favor of simplicity's deletion: remove the
plain build/test predecessors in the existing binary red/green fixture loop.
Its target must be absent before llvm-cov, privately owned by that selftest,
and harvested/captured immediately afterward. Adopt robustness's freshness,
selected-depfile and source assertions. Decline keeping another plain target
and a second build pipeline: the losing option costs duplicate compilation
and another artifact lifecycle without strengthening this proof. The chosen
option gives up an independent plain-build smoke in those two fixtures;
llvm-cov still builds and runs their real executable integration tests.
Library-only fixtures retain their explicitly plain rlib harvests.

Adopt robustness's exclusive scratch creation and owned cleanup for malformed
expansion files, using only std and no shared cwd/environment changes. Accept
the small local test utility cost; decline a reusable fixture framework.
Decline the optional empty-input test as unnecessary to the requested
nonempty cannot-tokenize branch. Neither position justifies a parser rewrite,
custom library-name registry, provenance manifest or new scanner command.

This implements the existing inventory/refusal contract in accepted
[0035](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md) and
[0050](../../docs/decisions/0050-delivery-gates.md); it changes no ruling or
public contract, so no new decision is declared. Preserve accepted
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
frozen 0001–0003 and ingest extraction 0005. Preserve the supplied #174
reservation/tooling behavior; [0064](../../docs/decisions/0064-coverage-reservation-tooling.md)
remains proposed and is cited only as context, not accepted here.

## Acceptance Criteria

Exactly the issue's two criteria apply. The proof details below operationalize
them rather than adding separate feature criteria.

1. **The coverage-fn-scan helper has unit tests for cannot-tokenize refusal,
   dash-named crate targets and make `$$` unescaping.** Call actual `cmd_attrs`
   with a nonempty unbalanced expansion file and require an error containing
   `cannot tokenize expansion` and its path; a balanced valid file succeeds.
   Cover hashed/unhashed `libdash_name` rlibs for package `dash-name`, existing
   ordinary/underscore names, and nearby wrong-package/prefix-collision,
   rmeta and executable/harness negatives. Preserve literal dash-named binary
   selection with an `is_bin_target` control. Fix the confirmed library
   matcher defect locally. Decode `a$$b` to `a$b` and `a$$$$b` to `a$$b` once,
   retaining lone dollars, ordinary text and existing escaped space/hash
   handling, including mixed inputs; fix the confirmed `unescape` defect.
   Expected values are literal and independent of the implementation.
2. **The coverage-gate selftest drives the instrumented dep-info path at
   least once.** Reuse the real binary red/green fixtures with absent private
   targets, llvm-cov as the only producer before harvest, and immediate
   scanner `files --capture`. Require a nonempty selected depfile capture
   naming the executable rule and production source, and the expected
   production inventory (`src/main.rs` plus the red `src/main_tests.rs` or
   green `src/support.rs`). Retain green LCOV membership and existing red
   refusals. Missing evidence fails; no fallback to a plain build is allowed.

## Boundary and proof ownership

Implementation touches only `bundles/self/scripts/coverage-fn-scan/src/main.rs`
and the selftest portion of `bundles/self/scripts/coverage-gate.sh`.
The external-source classifier and E-row emission are out of scope. The
pre-existing HIGH denominator gap involving CARGO_HOME, RUSTUP_HOME and /rustc
is tracked by #219 under the operator's ruling, not accepted or repaired here;
any worsening introduced here remains this change's responsibility. Earlier
recovery candidates and their enlarged criteria are not reused.

No application, dependency, manifest, lock, shim, coverage floor, recipe,
policy or decision changes. Existing house gates remain delivery obligations,
not extra #175 criteria. CI owns platform/chart/Docker/network/tag/privileged
port proofs assigned by the charter; the operator owns live settings, signing
and publication. No namespace or unavailable-platform proof is required here.
A design draft or structural check does not claim feature completion.
