# Plan — scanner regressions

## Technical context

The Rust helper has its own workspace and existing syn/proc-macro2 dependencies.
Bash selftests build real temporary Cargo crates. No new dependency or tool is
needed. The application coverage denominator excludes this standalone helper;
its unit suite must be executed explicitly. The workspace contains only a
spec template, no plan template; this plan supplies the requested technical
context, constitution check, structure, research, data model, contracts and
quickstart directly.

## Constitution check

Accepted 0035/0050 already require build-derived inventories and malformed
expansion refusal. These are compatible corrections and tests, not a semantic
amendment. Keep exact integer 100% lines/branches/functions, pinned proof
ownership, #174 reservation/cleanup, and all accepted mail guarantees intact.
No dependency allowance changes. Existing recipe routing remains authoritative;
implementation changes to pinned resources must be compiled through that
route before downstream dispatch, never bypassed by editing pins or policies.
Commit unsigned, never push, and leave the chief's design commit last for the
sequence's structural check.

## Project structure and files touched

| Path | Work |
|---|---|
| `specs/scanner-regressions/{spec,plan,tasks}.md` | Only files in the chief's design commit |
| `bundles/self/scripts/coverage-fn-scan/src/main.rs` | Later bounded helper fixes and inline unit tests |
| `bundles/self/scripts/coverage-gate.sh` | Later selftest-only suite invocation and binary fixture provenance checks |
| `target/` and run-local evidence directories | Ignored build output and receipts; never staged |

No separate research/model/contract files are needed for this local repair.

## Research and implementation approach

Static inspection confirms `unescape` currently replaces only escaped spaces
and hashes, and `is_lib_target` compares against the literal package name.
`cmd_attrs` already returns a named cannot-tokenize error. The binary selftest
currently builds/tests plainly in the same target before llvm-cov, whereas
its production gate already captures before expansion.

1. Extend the existing inline test area, preserving exclusion tests. Use
   exclusive temporary-file creation with process/test uniqueness and owned
   cleanup. Feed nonempty unbalanced delimiters through `cmd_attrs`; assert
   the stable category/path, not proc-macro2's full diagnostic. A balanced
   fixture must return success. No sleep, environment mutation or tempfile
   dependency.
2. Normalize only the package argument inside `is_lib_target` with `-` to `_`.
   Retain exact-or-hash-tail boundary matching and the `.rlib` requirement.
   Do not change `is_bin_target` or introduce custom target-name discovery.
   Table-test the positive and negative controls in AC1.
3. Add one nonrecursive `$$` replacement to `unescape`, keeping its existing
   backslash handling. Test literal expected strings, including repeated
   pairs and mixed escapes. Run new dash/dollar tests before the respective
   fixes to witness their failures, then after the fixes; the existing
   cannot-tokenize implementation should already pass its new regression.
4. In the selftest, run the helper unit suite once via the pinned helper
   manifest with `--offline --locked`, using a target inside the owned
   selftest scratch root. Propagate test failure through existing selftest
   failure handling. Keep `ensure_fn_scan`'s production behavior unchanged.
5. Remove only the plain build and plain integration-test calls preceding
   llvm-cov in the binary fixture loop. Check that each private target is
   absent before use. Run the existing real llvm-cov command, then immediately
   harvest/capture against that same target before any expansion/plain pass.
   Assert captured selected depfile content and expected source inventory;
   preserve the green report checks and red stage-1/stage-2 refusals. Keep
   library-only plain-rlib checks and all unrelated selftest arms intact.
6. Review the two-file diff, run focused tests and all house gates, and commit
   implementation before hermetic verification. Do not import recovery code
   or change classifier/E-row behavior to address #219.

## Data model and contracts

No application data model, wire API, storage format or scanner CLI/output
format changes. A package name maps to a library crate spelling only at the
rlib matcher; executable spelling stays literal. Make dollar decoding is
single-pass. Malformed nonempty expansion remains an error naming its input.

Fixture evidence follows: fresh private target -> real instrumented build
and executable test -> selected dep-info capture and inventory -> existing
report/refusal checks and later expansion. Only the owning selftest removes
its scratch root. These local ownership facts establish build provenance;
they promise no hostile concurrent filesystem or distributed-lock protection.

## Quickstart and validation

Run from the repository root. The helper's independent checks are:

```bash
cargo fmt --manifest-path bundles/self/scripts/coverage-fn-scan/Cargo.toml -- --check
cargo clippy --manifest-path bundles/self/scripts/coverage-fn-scan/Cargo.toml --all-targets --offline --locked -- -D warnings
cargo test --manifest-path bundles/self/scripts/coverage-fn-scan/Cargo.toml --offline --locked
bash -n bundles/self/scripts/coverage-gate.sh
```

Run every command in the sole [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
including exact coverage and its separate selftest. Substitute
`.forge/bin/brokkr compile --bundle bundles/self` for unqualified brokkr;
first verify digest `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Retain command/status/output receipts and distinguish failures from passes.
Verify also owns strict pinned-shim validation and fail-closed native-tls
absence: a failed cargo-tree command proves absence only with the specific
`did not match any packages` diagnostic. CI/operator obligations remain with
the owners named in the spec. Offline verify installs nothing.

For this design, run `git diff --check`, commit exactly the three artifacts,
and run `bash bundles/self/scripts/design-check.sh check .` against that HEAD.
The sequence's final pinned check owns its verdict. Gates on this documentation
commit do not prove the future implementation's two acceptance criteria.

## Risks

| Risk | Mitigation and cost |
|---|---|
| Wrong refusal branch tested | Nonempty unbalanced input through actual cmd_attrs, named error and valid control |
| Overbroad target matching | Library-local normalization, suffix/boundary negatives and literal binary control |
| Repeated decoding corrupts dollar names | One replacement pass; literal paired/repeated/mixed examples |
| Plain or stale dep-info certifies coverage | Absent private target, sole llvm-cov producer, immediate checked capture; no fallback |
| New suite is silently omitted | Explicit helper tests wired into existing selftest; small additional build cost |
| Scratch collisions or lost evidence | Exclusive creation and owned cleanup; preserve existing #174 lifecycle |
| Scope regrows around #219 | Two implementation files, unchanged classifier/E rows, diff review against this fresh base |
| Gate result is overclaimed | Record actual receipts; baseline/design checks and downstream candidate verification remain distinct |

The accepted limit is focused coverage of the three named helper behaviors,
not every make grammar case or scanner refusal. Existing library-only fixture
provenance remains plain; the binary fixtures discharge the at-least-once
instrumented requirement. The pre-existing #219 gap remains tracked work.
