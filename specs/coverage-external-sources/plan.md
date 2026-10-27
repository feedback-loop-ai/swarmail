# Plan — coverage external sources

## Technical context

This is a Bash gate with a standalone Rust scanner, outside the measured
application workspace. The real candidate at design entry is `7606e8f` over
`58adf0b`. The scanner selects production lib/bin rules from real dep-info,
normalizes prerequisites, probes function-bearing sources and emits sorted
workspace-relative paths. Normalization uses lexical components plus metadata;
it already refuses external absolute prerequisites. The report join compares
that inventory with LCOV SF records. No recovery-candidate C/E protocol is present.

Use the installed toolchain and cached locked dependencies. No new package or
license is required. The only template in `.specify/templates/` is an unrelated
multi-repository spec template; a plan template is absent. This plan supplies the
required technical context, constitution check, project structure, research,
data model, contracts and quickstart directly.

## Constitution check

Accepted 0035/0050 require this behavior already: retain compiler-owned inventory,
fail closed, retain exact counting and expansion checks, and verify pinned bytes.
The public application contract and 0001–0005 are untouched. Keep the canonical
containment requirement independent of lexical report identity; do not label
this older scanner's lexical prefix check a canonical proof. Existing canonical
checks, wherever present at implementation entry, must survive. This work does
not resolve unrelated #175 findings or 0072's comparator limit.

Preserve current bundle routing and protected reviews/security hold. Author
commits are unsigned under 0056; the operator alone accepts decisions and seals
branches. This design requires no new decision. Run-local evidence remains
ignored and never substitutes for the implementation's committed artifacts.

## Project structure

| Path | Planned responsibility |
|---|---|
| `specs/coverage-external-sources/{spec,plan,tasks}.md` | This chief's only committed files; existing framing is unchanged. |
| `bundles/self/scripts/coverage-gate.sh` | Extend existing real-build selftest helpers and assertions; change accounting only if a demonstrated gap requires it. |
| `bundles/self/scripts/coverage-fn-scan/src/main.rs` | Preserve refusal/provenance and existing tests; a narrow repair and focused tests only if required by actual fixtures. |
| `bundles/self/scripts/shims.sha256` | Regenerate/check actual public-shim hashes alongside proof changes; retain identical content when dispatcher bytes are identical. |
| `tests/delivery_constitution.rs` | Update actual changed identities; bind changed canonical gate/scanner resources explicitly if absent from the fixed digest table, preserving manifest and dispatch proofs. |
| Ignored `target/coverage/` and `.forge/epic192/issue219/` evidence | Fresh fixture artifacts and retained command/status/assertion receipts, never source or decision authority. |

No new helper crate, permanent fixture package or public shim behavior change.
The manifest currently hashes seven public dispatchers, not canonical gate
implementations; never put a canonical digest in a dispatcher's row or create
cosmetic shim changes to force checksum churn. Review manifest regeneration and
digest-table updates together, documenting unchanged manifest bytes when true.

## Research and implementation approach

1. Recheck packet hashes, Git base and scanner tests at implementation entry.
   Retain both current raw-attribute tests, and any newly present #175 tests.
   Establish real-case behavior before choosing a scanner patch. Existing
   rejection is useful evidence, not proof that the historical issue was fictitious.
2. Extend the existing `selftest`, `fx_cmd` and contextual-refusal helpers. Each
   case owns a unique scratch root and uses real compiler-produced dep-info from
   selected production units. Independently require a successful instrumented
   build and live exports before asserting external accounting. Failed harvesting
   must never supply partial inventory to a join or reuse an older successful
   report. Dep-info used as primary evidence must belong to the instrumented
   production source graph; a different source graph's rebuild is insufficient.
3. Use four local fixture arms for Cargo registry, Cargo git, Rustup toolchains
   and rustc prefixes. Temporary environment roots belong to each fixture;
   determine installed compiler/Cargo/exporter executables before substituting
   homes so a rustup lookup cannot turn into an installation attempt. Do not
   change the process-wide HOME or installed Cargo/Rustup trees.
4. Preserve evidence of the compiled source spelling and its ordinary file
   identity separately. An alias case must distinguish an otherwise measured
   workspace source from the affected compiled spelling. Fixture sources avoid
   tests/examples/benches components and reserved harness basenames so a
   different ignore rule cannot accidentally prove the assertion.
5. For the rustc class, use an owned temporary path containing a rustc/hash
   component outside the fixture workspace. Confirm with the actual exporter
   that it matches the rustc class; the directory's name alone proves nothing.
   Compare default and filename-defaults-disabled control exports from the same
   instrumented build/profile data for every class. Keep this paired export in
   test tooling only. If the installed tool cannot express a matching local
   path, retain the exact unmet proof rather than skipping it or claiming a
   fabricated report demonstrates it.
6. Require named external-source refusal through the scanner/gate accounting
   boundary. On this base, `resolves outside the workspace` plus the specific
   prerequisite in one invocation is the intended outcome. It is meaningful
   only after build, ownership and export checks succeed. Keep the generic
   refusal; no duplicate shell recognizer or ignore regex belongs in production.
   Add a narrow scanner/join guard only if these cases expose an actual gap.
7. Add an ordinary all-covered green control and a workspace-under-ignored-root
   control. The latter uses existing report validation/membership because its
   source is lexically internal. Apply AC3's named missing-source refusal for
   live reports or fixture-specific empty-report refusal when every source is
   filtered, with a live unfiltered control proving the cause. This additional
   control cannot replace any of AC1's external-source cases. Preserve existing
   non-executable asset/declaration controls and unit selection.
8. Demonstrate AC4 with an isolated, non-shipping mutation of the accounting
   boundary and the same fixture evidence. Require the intended assertion to
   discriminate loss of source accounting and the restored candidate to pass.
   Do not disable arithmetic or unrelated gates. Retain concise receipts outside
   trap-cleaned fixture roots; do not embed a bypass switch in production.
9. Reconcile pins, run the whole house inventory plus focused proofs, and commit
   implementation. Fresh verify gate-entry is then initiated by the operator's
   assistant. Never claim the current run's pre-edit bundle verifies new bytes.

## Data model and contracts

No persistent application data changes. The existing scanner CLI and plain path
inventory remain the preferred interface. Selected production dep-info is the
source of ownership; compiler spelling is evidence of report identity; a
canonical referent, where checked, is evidence of containment, not substitute
coverage. Nonzero harvest status invalidates all emitted inventory.

A fixture receipt records case/class, installed tool identity, effective fixture
roots, build/harvest/export statuses, production prerequisite, default/control
report observations, expected category/source and observed outcome. Mutation
receipts additionally record the intended failed assertion and restored result.
Logs must distinguish unavailable tooling from an accounting refusal. A report
with no SF records never establishes a passing coverage claim.

The exact floor remains positive integer totals with zero missed lines,
branches and functions. No rounded percentage, global production ignore-disable,
function-identity equivalence engine or new external-source admission contract.

## Quickstart and validation

Verify the local engine before use; its SHA-256 must equal
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Use offline cached tools; install nothing. The authoritative gate inventory is
[house rules](../../docs/house-rules.md#gates--part-of-every-change). Execute all
of it, using `.forge/bin/brokkr` for the compile command:

```bash
(cd .forge/epic192/issue219 && sha256sum -c SHA256SUMS)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

Also run the standalone scanner suite with
`cargo test --manifest-path bundles/self/scripts/coverage-fn-scan/Cargo.toml --target-dir target/coverage/scanner-tests --locked --offline`,
`cargo test --workspace --locked --test delivery_constitution`,
`sha256sum --check --strict bundles/self/scripts/shims.sha256`, and the existing
pinned `design-check.sh selftest` and `delivery-proof-test.sh`. Native-tls absence
requires `cargo tree -i native-tls` to fail specifically with
`did not match any packages`; any other result refuses. Preserve fresh exact
coverage summary/LCOV and contextual real-fixture/mutation receipts.

After this chief commits, run `bash bundles/self/scripts/design-check.sh check .`
on HEAD. That local check is preflight; the sequence executes its own pinned
validator. Do not add a later unrelated commit. Gates run during design establish
only this candidate's observed baseline, never future implementation success.
Unavailable prerequisites or failed gates are recorded as outstanding.

## Design-seat validation — 2026-10-25

The chief ran all nine house commands on the unchanged implementation using the
checksum-verified local engine and offline Cargo. All passed. Exact coverage:
13,878/13,878 lines, 782/782 branches, 1,508/1,508 functions; evidence at
`target/coverage/run.eafHfc/`. Both standalone scanner tests, design-check
selftest and strict shim checks passed. The native-tls probe exited 101 with
`did not match any packages`, satisfying the absence proof. Run-local command,
status and log records are under `.forge/epic192/issue219/chief-checks/`.
The workspace response timed out during the long batch; the runner's completed
status file and command logs establish these outcomes. These are baseline
checks, not execution of the new AC1–AC4 fixtures or future pinned verification.

## Risks

- **Historical baseline mismatch:** a broad port could undo current protections.
  Inspect the actual scanner and retain its tests; patch only demonstrated gaps.
- **Wrong refusal mistaken for proof:** require successful setup/build/export,
  production dep-info and category plus exact source from one invocation.
- **Exporter drift or inaccessible rustc spelling:** pair real exports and fail
  the proof explicitly when the local tool cannot demonstrate the intended class.
  CI/operator own additional platform evidence; no privileged setup is required.
- **Canonical collapse:** keep spelling evidence until accounting concludes;
  do not use a covered referent to excuse another omitted compilation spelling.
- **Additional selftest cost and cleanup hazards:** use small dependency-free
  crates, reuse helper machinery, keep roots unique, and remove only owned data
  after owned children terminate. Preserve current reservation/evidence semantics.
- **False refusals for unusual layouts:** deliberately accepted by AC1 and 0035;
  future external-source support needs a separately reviewed proof.
- **Stale pins or stale verification:** regenerate truthful hashes, run the digest
  tests and actual dispatch proof, then verify a fresh bundle of committed bytes.
- **Scope growth:** frozen mail semantics, dependencies, recipe routing and
  unrelated comparator/reservation findings stay outside this implementation.
