# Tasks — port coverage external-source proofs

Ordered implementation tasks, not completed by the design commit. Each checkbox
requires its paired test/evidence. AC references point to [spec.md](spec.md);
commands and ownership are in [plan.md](plan.md).

- [ ] **T1 — Establish the port baseline.** Verify the complete packet and engine
  digests, record commit/tree identity, and compare the shipped selftest insertion
  with main's #175 additions. **Test:** strict packet/engine checks pass; the
  separate scanner suite passes all five current tests, and the current helper
  invocation and instrumented dep-info arms are identified for preservation
  (AC5–AC6).
- [ ] **T2 — Port the four external cases additively.** Insert the shipped helper
  after main's fresh-target scanner suite and before clean-failure probes; retain
  owned registry/git/rustup/rustc fixtures and paired exports. **Test:** all four
  real instrumented builds/exports succeed; selected production-unit dep-info,
  same-file identity, live unfiltered line/branch/function records and actual
  default alias omission satisfy AC2. Each real harvest refuses with the category
  and exact source required by AC1, without consuming partial inventory.
- [ ] **T3 — Retain the two shipped controls and main's proofs.** Include ordinary
  alias and internal ignored-root arms, preserving all existing gate/scanner
  behavior. **Test:** the green control passes the inventory join with positive
  totals and integer zero misses; the internal control has a live unfiltered
  report and the specified contextual refusal. The complete old-plus-new selftest
  and standalone scanner suite pass, including main's fresh-target, binary
  provenance and clean-failure checks (AC3, AC5).
- [ ] **T4 — Prove the regression test discriminates.** Repeat the isolated shipped
  omission mutation against current bytes with separate helper artifacts and
  fresh receipts outside the packet. **Test:** all four intended named-refusal
  assertions fail specifically because harvesting succeeds after valid setup;
  restoring original scanner bytes yields a passing full selftest. Record hashes,
  commands, exits and causal assertions; no mutation remains (AC4).
- [ ] **T5 — Bind final proof bytes.** Update the constitution's canonical gate
  digest from the merged file, check unchanged public shims and compile with the
  checksum-verified local engine. **Test:** strict shim checks,
  `pinned_bundle_shims_and_proof_dispatch_are_digest_bound`, the entire
  `delivery_constitution` suite and `delivery-proof-test.sh` pass; fresh bundle
  compilation succeeds without changing routing or engine identity (AC6).
- [ ] **T6 — Commit and run every local gate on the candidate.** Commit scoped
  implementation unsigned, then execute the plan's complete house inventory,
  standalone scanner, constitution, design/delivery proofs and fail-closed
  native-tls check offline. **Test:** every command exits as required; exact
  coverage records positive totals and zero missed lines/branches/functions;
  full selftest passes. Retain fresh commit/tree/hash-bound command receipts and
  diagnostics. Failures remain outstanding, never converted to completion;
  changed code/proof bytes require affected reruns (AC5–AC6).
- [ ] **T7 — Hand off the committed implementation to fresh verification.** Commit
  accurate implementation evidence in this port directory and leave source/proof
  artifacts clean, with no mutant, route change, dependency change or historical
  receipt presented as new. **Test:** scoped diff and clean tracked worktree
  confirm the intended changes; subsequent recipe verification binds committed
  candidate and fresh pinned proof identities and passes before feature completion
  is claimed. CI/operator-only proofs retain their named owners (AC6).
