# Coverage accounting for ignored production sources

Created: 2026-10-25. Status: draft. Epic #192, issue #219.
Design run: `epic-192-issue-219-count-product-bef2373e`.

## Feature and rationale

The exact coverage gate must account for production source reached through
Cargo registry/git, Rustup toolchain and rustc source-prefix paths. A familiar
cache or toolchain directory name does not make application code a dependency.
A successful 100/100/100 verdict must never depend on an exporter silently
omitting such a source. This design chooses **named refusal** for external
production prerequisites; supporting measurement of external code is unnecessary
for this repair.

The operator's checksum-verified packet assigns this pre-existing HIGH gap to
#219, separate from #175. The packet describes a recovery scanner with C/E rows;
the current candidate instead has `normalized_entry`, `normalized_set` and a
plain workspace-relative inventory. Its existing lexical outside-workspace
refusal may already enforce the required outcome. Prove that on this candidate;
change enforcement only if the proof identifies a gap. Tests-only hardening is
a valid implementation outcome, provided every acceptance criterion is met.

## Authority and scope

The intake is [framing.md](framing.md), committed at `7606e8f` and identified by
the same run ID. The requested `.brokkr/state/tasks/` directory is absent. Both
`.forge/epic192/issue219/README.md` and `operator-rulings.md` passed their
`SHA256SUMS` check from the packet directory. Neither prior-run receipts nor
external evidence are inferred from the packet's empty prior-runs section.

Accepted [0035](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md)
§§1–2 already requires compiler-owned inventory and measured-or-refused
production files. Accepted [0050](../../docs/decisions/0050-delivery-gates.md)
§§1–3 owns the pinned proof chain, expansion checks and committed design identity.
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
governs exact counts and test placement. This is compatible enforcement and
regression proof, not a change to a ruling or public contract; no new decision
is declared. Proposed 0064/0072 are context, not acceptance or new authority.

Only the pinned coverage gate, its scanner where necessary, its existing
selftest and integrity bindings are implementation scope. Preserve the
single-application-crate boundary, source/data distinctions and frozen
0001–0003 plus ingest extraction 0005. No application changes, dependencies,
license widening, recipe changes, external-source support framework, unrelated
reservation/comparator repair or #175 recovery import are authorized.

## Panel rulings

1. **Adopt simplicity's one inventory and generic external refusal.** Reject
   a production catalogue of Cargo/Rustup/rustc ignore expressions and a second
   report/merge pipeline. The compiler's selected production dep-info remains
   authoritative. Cost: legitimate external-source layouts can be refused even
   when their referents are in the workspace; supporting those layouts is deferred.
2. **Adopt robustness's provenance requirement.** A normally measured spelling
   does not prove that another compiled spelling of the same file was measured.
   Preserve compiler-observed provenance until refusal/accounting is decided.
   Reject canonical collapse as evidence of coverage. This does not require a
   new inventory protocol on the present base. Cost: conservative refusals remain
   possible, and tests must retain more than aggregate coverage percentages.
3. **Rule for robustness on proof strength, for simplicity on mechanism.** The
   existing outside-workspace refusal is sufficient only after the real build,
   production ownership and actual default-export omission are established.
   A generic unrelated failure is insufficient. Also exercise a production
   workspace under an ignored root through existing report validation/membership;
   its source is lexically internal, so the external check alone is insufficient.
   Reject a generalized source discovery redesign. Cost: another bounded local
   control and additional selftest time; no claim of universal alias equivalence.
4. **Reconcile containment honestly.** Retain independent containment refusals
   and never relax them to admit an ignored spelling. Inspection confirms the
   current `normalized_entry` is lexical, not filesystem canonicalization;
   neither position establishes universal canonical containment on this base.
   Do not claim to repair broader #175 containment findings here. Preserve any
   stronger checks/tests present when implementation begins.
5. **Reject unsupported claims in either position.** The scanner currently has
   two raw-attribute tests; robustness's statement that none were found is
   superseded by direct inspection. Real ignore-class, mutant and implementation
   results remain due. No proposed decision is accepted by this design.

## Acceptance Criteria

- **AC1 — External sources refuse by name.** For production-owned Rust source
  reached through each of `$CARGO_HOME/registry/`, `$CARGO_HOME/git/`,
  `$RUSTUP_HOME/toolchains/` and the rustc source-prefix ignore class, the gate
  cannot succeed with that compiled spelling absent from the denominator.
  This implementation retains named external refusal, including aliases whose
  referent is an ordinary workspace source. Assert the category and exact
  offending source from the same failed invocation. A covered ordinary spelling
  cannot substitute for the omitted spelling.
- **AC2 — Real evidence for all three ignore classes.** The existing coverage
  selftest builds dependency-free temporary crates for all four spellings above.
  Each case records successful compilation, real production dep-info and paired
  exports of the same instrumented artifacts/profiles: default filtering and a
  test-only control with filename defaults disabled. The control must contain
  executable records for the affected source and the default must omit them.
  Setup, compiler, harvest infrastructure or empty-report failure fails this
  external-source proof; it is never the expected refusal. The rustc case uses
  owned scratch storage, demonstrates the installed exporter's matching behavior,
  and needs no host `/rustc` write, namespace or network. An unrepresentable case
  stays unproved rather than skipped.
- **AC3 — Positive and internal-path controls.** An ordinary measured green
  fixture passes the real inventory/report join and exact count checks. A
  workspace located under an active ignored root cannot silently lose its
  function-bearing production source: actual measurement passes only with exact
  counts, or report validation/membership refuses. For a live report require
  the missing source's name; if defaults suppress the entire report, require the
  existing missing/empty/SF-less category and that fixture's report path, after
  successful compilation and a live unfiltered control establish the cause.
  Preserve declarations-only, asset and dependency/build-script classification.
- **AC4 — Discriminating regression proof.** An isolated controlled mutation
  restoring the source-accounting omission makes the relevant new selftest fail
  specifically because the expected accounting/refusal is missing. Construction
  and execution must succeed; syntax/build/setup failures do not count as a
  killed mutant. The restored candidate passes the same assertions. Retain
  commands, statuses and decisive diagnostics; no mutant is committed.
- **AC5 — Existing protections survive.** Preserve all existing containment and
  outside-workspace refusals; positive denominators and integer zero-missed
  lines, branches and functions; production measured/no-coverage/release
  comparisons; and production/test expansion exclusion scans. Do not broaden
  comparator allowances. All old selftest arms and both existing scanner tests
  pass. Preserve #175 tests if actually present at implementation entry.
- **AC6 — Integrity and delivery evidence agree.** Reconcile the public-shim
  manifest and constitution digest table with every gate change, preserving
  truthful hashes for unchanged shims. The canonical changed proof resources
  are bound by the fresh compiled bundle. All house gates, standalone scanner
  tests, delivery-constitution checks and fail-closed native-tls absence proof
  pass on the implementation candidate. Record failures as outstanding. Commit
  implementation unsigned, then hand off to the operator's assistant for fresh
  verify gate-entry against those committed pinned bytes; this design and an
  earlier compiled run do not certify later edits.

## Proof ownership

This chief commits only spec/plan/tasks. The sequence's deterministic check owns
the design verdict. Implementation owns AC1–AC5 and local AC6 evidence; fresh
verify owns the committed-candidate verdict and protected review remains required.
CI/operator retain chart/history/selftest, Docker smoke, RustSec network audit,
tag-only rate proof, privileged port-80 proof, macOS-specific execution and live
settings evidence. None becomes a new local criterion. No push, merge, tag,
publication, issue closure or decision acceptance is part of this work.
