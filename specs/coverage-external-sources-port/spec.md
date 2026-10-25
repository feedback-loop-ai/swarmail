# Specification — port coverage external-source proofs

Date: 2026-10-25. Status: draft for the design sequence's deterministic check.
Feature: epic #192, issue #219. Slug: `coverage-external-sources-port`.

## Feature and rationale

Port the shipped #219 coverage proof onto main base `aec4b46`, preserving
main's subsequent #175 work in `55d0f02`. Production source instances reached
through Cargo-home registry/git, Rustup toolchains or rustc ignore paths must
be measured or refused, never silently omitted from an apparently exact
coverage denominator. An ordinary measured spelling of a file does not account
for another independently compiled spelling that the exporter filters out.

The current scanner already rejects external production prerequisites. This
port adds the shipped six real-crate selftest cases and binds the merged gate
bytes in the constitution test. It preserves main's scanner, standalone scanner
test invocation, instrumented dep-info proofs and clean-failure assertions.
The historical issue body's C/E-row API is not the current implementation.

## Evidence and authority

The complete `.forge/epic192/issue219/SHA256SUMS` packet passed verification,
including README.md, operator-rulings.md, port.md and all three shipped patches.
The engine digest matches the packet and `bundles/self/tooling.json`:
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
README supplies the three shipped acceptance criteria; port.md records the
historical SHIP-COMPLETE outcome and the conflict. Historical receipts are not
fresh verification of this port.

The requested `.brokkr/state/tasks/` directory is absent. The committed
[intake](intake.md) supplies current framing and agrees with the verified
packet. Both run-local panel files were read: simplicity.md and robustness.md
under `.brokkr/state/design/positions/`. They remain uncommitted evidence;
this document records the resulting rulings independently of those files.

This is compatible test strengthening under accepted
[0035](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md) and
[0050](../../docs/decisions/0050-delivery-gates.md), not a semantic amendment.
No new decision is declared. Preserve synchronous accept == stored (0001),
subscribe before check (0002), oldest-only pruning (0003), and ingest extraction
(0005). No application contract, accepted ruling or frozen guarantee changes.

## Synthesis and rulings

The positions agree on the final mechanism; there is no genuine policy conflict
to manufacture. Their competing priorities are resolved as follows.

| Question | Ruling and adopted position | Rejected alternative and cost of the ruling |
|---|---|---|
| Production accounting | Adopt simplicity's single owner: retain the scanner's existing generic named external-source refusal. Robustness concurs. | Reject a new ignore-root classifier, external admission scheme, C/E protocol or global default-filter removal. External production layouts remain unsupported; this is explicitly permitted by the shipped measured-or-refused criterion. |
| Proof size | Adopt robustness's complete six-case evidence chain, including both controls, paired live exports and independent compiler provenance. Simplicity explicitly retains these too. | Reject a reduced path-existence test, fabricated inventory or one red case without a green twin. Pay for six offline instrumented builds, paired exports and temporary disk use; reduced proof would not establish omission of a compiled instance. |
| Conflicting selftest insertion | Compose both additions: keep main's fresh-target standalone scanner suite immediately after tool resolution, then insert the shipped fixture block before the clean-failure probes. | Reject taking either side wholesale. Additional selftest code remains necessary to avoid losing either #175's reviewed protections or #219's shipped proof. |
| Mutation evidence | Adopt robustness's causal proof requirement and simplicity's isolated, non-shipping execution: the four intended assertions must fail because harvesting wrongly succeeds, then the restored full selftest must pass. | Reject any nonzero exit as sufficient evidence. Compilation, setup or unrelated failures require investigation and do not discharge the proof. |
| Runtime and adjacent work | Use robustness's runtime analysis as preservation rationale, not new functionality; adopt simplicity's strict port boundary. | Reject new webhook, TLS, reservation, comparator, namespace or concurrency guarantees. Their existing limits remain; this port does not resolve adjacent issues. |

## Acceptance Criteria

These refine the shipped README's three criteria without expanding feature scope.
AC1–AC3 prove its measured-or-refused and real-fixture requirements; AC4 proves
its mutant requirement; AC5 preserves its existing guarantees; AC6 binds and
verifies the port under existing delivery rules.

1. **AC1 — External compiled instances refuse by name.** Real temporary crates
   cover `$CARGO_HOME/registry`, `$CARGO_HOME/git`,
   `$RUSTUP_HOME/toolchains` and the rustc/hash ignore class. For each, after
   successful build and export, the current scanner exits nonzero with both
   `resolves outside the workspace` and the exact external source spelling in
   the same invocation. No failed or partial inventory feeds a successful join.
   A symlink to an already measured workspace source must still refuse.
2. **AC2 — Actual omission and production ownership are established.** Each
   external fixture independently proves the alias in the selected production
   library's `.rlib` prerequisite rule and its file identity with `src/real.rs`.
   Preserve that compiler-unit dep-info byte for byte. Both source spellings
   have executed line, branch and function records in an unfiltered control;
   the default report is live, omits the alias and measures the ordinary source.
   Both exports come from the same instrumented run, binaries and profiles.
   Build-script inputs, env-dep comments and Cargo artifact summaries alone are
   insufficient. Setup, build or export failure fails the proof. Use private
   temporary homes and an owned path containing `/rustc/<hash>/`; actual exporter
   behavior must establish the ignore match without host `/rustc` writes.
3. **AC3 — Both shipped controls discriminate.** An ordinary in-workspace alias
   passes the real harvest/report join with positive line, branch and function
   totals and integer zero misses in each dimension. A workspace inside an
   active ignored root successfully harvests internal sources, then refuses
   either the missing `src/real.rs` with the existing missing-production category
   or its specific default report with the existing empty/SF-less category.
   Its successful unfiltered live control establishes why default filtering
   caused the refusal. Neither control replaces an external case.
4. **AC4 — Regression discrimination is causal.** Repeat the shipped isolated
   omission mutation against the port's final bytes. All four external named-
   refusal assertions fail because harvesting incorrectly succeeds after valid
   setup and execution. A crash or earlier unrelated failure is insufficient.
   Restore the original scanner and run the full selftest successfully using
   uncontaminated helper artifacts. Retain commands, statuses, decisive assertion
   output and gate/scanner identities outside cleaned fixture roots; commit no
   mutation or bypass switch.
5. **AC5 — Main's protections survive.** Preserve existing containment checks,
   exact zero-missed arithmetic, production/test exclusion scans and measured/
   no-coverage/release expansion comparisons. Do not widen comparator allowances
   or change reservation semantics. Retain all five current scanner tests,
   their explicit fresh-target selftest invocation, instrumented binary dep-info
   evidence and clean-failure probes. The separate scanner suite and complete
   old-plus-new selftest pass. The current lexical normalization check must not
   be described as newly proven canonical symlink containment.
6. **AC6 — Proof identities and delivery agree.** Bind the final merged canonical
   gate digest in `tests/delivery_constitution.rs`. Fixed public shims retain
   their bytes and pass the existing checksum manifest. The digest-binding test,
   full constitution suite, delivery/design proof tests, every local house gate,
   exact coverage, full selftest and fail-closed native-tls absence proof pass
   against committed implementation bytes with the verified candidate-local
   engine. Fresh compiled/pinned verification must assess the changed resources.
   Retain commands, exits, commit/tree identity and exact coverage summary.
   Earlier shipped or design-baseline logs cannot certify implementation.

## Scope and proof ownership

Expected implementation edits are the canonical coverage selftest and its
constitution digest, plus accurate evidence in this port's spec directory.
No scanner rewrite, application code, dependency, license allowance, public
shim change, engine re-pin or recipe-route change is authorized by this design.
A contrary fixture result is an unresolved requirement to report, not permission
to broaden the port. The standalone scanner workspace stays outside the measured
application crate, and the existing full workspace tests retain real-server
protocol coverage.

The chief commits this design only. Implementation owns the new fixtures and
mutation proof; downstream verify owns fresh committed-candidate verification,
and protected review remains mandatory. Design-stage gate runs verify this
baseline only. The sequence's final deterministic check owns the design verdict.

CI owns macOS, Helm/chart history/selftests, Docker smoke, network RustSec,
privileged port-80 and tag-only release-rate evidence. The operator owns live
settings, sealing and release actions. Those are not new local acceptance
criteria. Do not access GitHub, CI, journals or sibling worktrees; do not push,
merge, tag, publish, close an issue or accept a decision.
