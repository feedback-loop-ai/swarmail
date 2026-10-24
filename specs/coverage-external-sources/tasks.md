# Tasks — coverage external sources

Ordered implementation tasks. Each checkbox remains open until its paired proof
is recorded. The chief commits the design; these are not claimed implementation
results. AC references are to [spec.md](spec.md).

- [ ] **T1 — Confirm authoritative inputs and current baseline.** Verify the
  packet and engine digests, record candidate identity, inspect current scanner
  and pin bindings, and enumerate existing/#175 tests. **Proof:** both packet
  checksum checks and exact engine-digest comparison pass; standalone scanner
  tests pass with both current raw-attribute tests retained (AC5–AC6).
- [ ] **T2 — Add the four real ignore-class fixture arms.** Extend the existing
  selftest with isolated Cargo registry/git, Rustup and local rustc-prefix cases,
  compiler-owned production evidence and paired actual exports. **Proof:** every
  real build succeeds; control exports contain executable affected-source
  records, defaults omit them, and selected dep-info proves production ownership;
  setup/empty-report failures cannot satisfy the external-source assertions (AC2).
- [ ] **T3 — Prove named external refusal and repair only an observed gap.**
  Retain the generic scanner refusal and provenance; keep any necessary change
  local to existing scanner/join logic. **Proof:** each T2 case refuses with its
  category and source in one invocation, including the alias-to-measured-source
  case; scanner suite passes, and no partial/failed harvest supplies a successful
  accounting result (AC1, AC5).
- [ ] **T4 — Prove the controls.** Add the ordinary measured green fixture and
  workspace-under-ignored-root case, retaining declaration/data/unit-selection
  controls. **Proof:** green passes the real join and integer exact floor;
  internal ignored sources are counted or trigger AC3's contextual report
  refusal, backed by the live control export; all prior red/green selftest cases
  still pass (AC3, AC5).
- [ ] **T5 — Demonstrate mutation discrimination.** Exercise an isolated
  source-accounting mutation and restore the candidate; retain command, status
  and decisive assertion receipts outside cleaned fixture roots. **Proof:** the
  mutation runs successfully far enough to violate the intended accounting
  assertion, that assertion fails, and restored code passes the identical proof;
  no mutation remains in the candidate (AC4).
- [ ] **T6 — Reconcile the proof identities.** Regenerate/check the seven-shim
  manifest and update the constitution table for actual changed canonical proof
  resources, preserving identical shim hashes when appropriate. **Proof:** strict
  manifest verification, `pinned_bundle_shims_and_proof_dispatch_are_digest_bound`,
  the full delivery-constitution suite and pinned delivery dispatch proof pass;
  fresh bundle compilation includes every changed resource (AC6).
- [ ] **T7 — Run every local delivery gate and preserve evidence.** Execute the
  house inventory with the checked candidate-local engine, standalone scanner,
  full coverage selftest, design-check selftest and fail-closed native-tls probe.
  **Proof:** all required commands pass, native-tls fails only with the expected
  absent-package diagnostic, and fresh exact summary/LCOV plus AC1–AC5 receipts
  are retained. Any unavailable/failed proof remains explicitly outstanding;
  CI/operator-only evidence is assigned to those owners (AC5–AC6).
- [ ] **T8 — Commit and hand off pinned verification.** Commit implementation and
  coordinated proof changes unsigned without pushing; record commit/bundle
  identities and request the operator assistant's existing-route verify entry.
  **Proof:** implementation files are committed and clean, mutations are absent,
  and a fresh verify run binds the committed candidate to its compiled proof
  resources before any implementation-complete claim (AC6).
