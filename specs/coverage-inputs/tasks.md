# Ordered implementation tasks — coverage inputs

Each task includes its proving test. Check a box only when its evidence exists;
these are implementation obligations, not claims made by the design phase.
AC references point to [spec.md](spec.md). Preserve dated clarifications on return.

- [ ] **T1 — Establish baseline and tooling identity (AC1, AC6–AC8).** Recheck the
  candidate-local absolute Brokkr digest and retained recipe ancestry; record
  compiler identity and baseline probes. **Test:** exact SHA-256 equality;
  `git merge-base --is-ancestor` for the recipe commits; real rustc raw-off
  expansion plus codegen; independent baseline grep/scanner results; reproduced
  swallowed harness-find status and qualified clean/hidden-find diagnostics.
- [ ] **T2 — Repair raw exclusion recognition (AC1, AC6).** Update only the local
  token comparisons and independent source predicate in the canonical owners.
  **Test:** existing selftest plus real compiler-produced raw-off/raw-name
  fixtures; independently named grep and scanner refusals, clean token/string
  twins, existing whitespace/comment/multiline/macro/cfg_attr/inner cases and
  compiler-derived impl controls. Restoring baseline matching must fail the
  added regression assertions; record compiler rejection of unsupported forms.
- [ ] **T3 — Reserve explicit targets freshly (AC2–AC3, AC6).** Replace reuse
  with checked parents and atomic leaf creation; preserve exact selection,
  retention, default allocation/preflight and cleanup authority.
  **Test:** actual resolver and cleanup paths for new/spaced paths, empty/dirty
  dirs, files, symlinks/dangling links, blocked parents/leaf errors and preserved
  sentinels; two processes with deterministic handshake yield one winner; retry
  of retained/abandoned target refuses; empty/unset default ignores TMPDIR and
  only default build trees are trap-cleaned. Baseline reuse fails these assertions.
- [ ] **T4 — Make clean an explicit logged prerequisite (AC4, AC6).** Check the
  actual llvm-cov clean command before measurement and preserve its evidence.
  **Test:** test-local cargo failure/success through production control flow in
  ordinary and conditional contexts, checking nonzero/name/log/diagnostic and
  absent post-clean measurement marker on failure; success reaches the marker;
  log-open failure refuses. Baseline fails the named-boundary proof, without
  claiming baseline top-level clean ever continued after failure.
- [ ] **T5 — Check complete source inventories (AC5–AC6).** Replace process
  substitution with checked find/sort capture and check hidden-directory find
  before consumption. **Test:** immediate and partial-output find failures for
  both sites, sorting failure, normal/conditional invocation, visible underlying
  stderr and inventory-specific refusal; empty/valid successes plus real hidden
  files and undeclared/unguarded harness refusals. Baseline must fail the new
  status/diagnostic assertions; no partial rows may be interpreted first.
- [ ] **T6 — Prove the integrated pinned gate (AC6–AC7).** Run full canonical
  selftest and real exact candidate coverage after all changes; reconcile
  comments and proof records. **Test:** every selftest arm passes; candidate
  report has positive counts and exact covered==count in all three dimensions;
  existing file-membership, three-expansion, test-unit, package and derive
  controls stay enforced; delivery-constitution and store-contract-docs suites
  pass. Record final source identities and negative-control receipts.
- [ ] **T7 — Execute local delivery gates and hand off (AC7–AC8).** Run every
  gate in the charter's sole inventory and all verifier-required supplementary
  proofs using isolated outputs and the digest-matched absolute Brokkr.
  **Test:** clean fmt, locked clippy/test, pinned compile, coverage/selftest,
  publish/seal proofs, licenses, native-tls absence, design-check selftest and
  pinned delivery tamper-boundary proof; actual hermetic verify and protected
  independent review receipts. Preserve recipe history, unsigned commits and
  unchanged shims/policy/dependencies; never push or manufacture receipts.
- [ ] **T8 — Reconcile external evidence before claiming issue completion
  (AC8).** Bind applicable CI receipts to the final source and list each missing
  proof explicitly. **Test:** charter-owned Helm contract/history/bump selftest,
  real-Docker persistence/cleanup/context smoke, RustSec audit, real port-80
  exchange and tag-only release-rate proof when required. Absence or documented
  inapplicability is distinct from a pass; no publication is authorized. Keep
  #173 completion unclaimed while required proof remains outstanding.
