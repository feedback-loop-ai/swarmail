# Ordered implementation tasks — published Brokkr pin

Unchecked tasks are future implementation work, not claims by the design seat.
Each task names the proof that must distinguish its correct and broken states.

- [ ] T01 (AC1): Extend `tests/delivery_constitution.rs` with the exact published
  identity/ordered-patch assertion, then add `binary_url` and truthful
  base-plus-patches publication provenance to `tooling.json`. Proof:
  `cargo test --locked --test delivery_constitution` includes record mutations
  or independent expected values that detect URL/digest/base/patch-order drift.
- [ ] T02 (AC2): Add deterministic extraction/execution of the actual workflow
  record reader; implement strict whole-string URL/digest validation and env
  transport. Proof: missing, wrong-type, malformed and newline-bearing field
  fixtures refuse by name before GitHub outputs; valid record exports the
  expected values; structural checks reject run-body expression interpolation.
- [ ] T03 (AC2–AC3): Replace the source release-build/compile authority with
  private bounded HTTPS download, checked hash before execution and one explicit
  admitted path; retain source policy/aggregation/candidate-binding tests.
  Proof: execute the actual workflow body with failed transfer, wrong bytes and
  matching bytes; refusal has no execution marker, matched control invokes all
  four recipes with that artifact, and neither PATH nor source output is used.
- [ ] T04 (AC4): Capture fresh combined diagnostics and preserve compiler status
  for each recipe; reject the exact policy-sweep substring on exit zero.
  Proof: actual-body fixtures for stdout warning, stderr warning, nonzero
  compiler exit and warning-free success; warning diagnostics name the recipe.
- [ ] T05 (AC5): Correct canonical verifier identity wording and extend the
  existing missing/mismatched fixture plus matched successful control. Proof:
  actual JSON `fail` names `binary_sha256`, no gate marker exists on refusal,
  matched control passes; disabling comparison in a scratch mutation makes
  the mismatch assertion fail. Run the restored fixture green.
- [ ] T06 (AC6): Regenerate/check the existing seven-entry shim manifest and
  update canonical tooling/verifier digest expectations together. Proof:
  `sha256sum --check --strict bundles/self/scripts/shims.sha256` and
  `pinned_bundle_shims_and_proof_dispatch_are_digest_bound` pass; a changed
  canonical byte or public shim is detected. Unchanged wrappers leave the
  manifest byte-identical; do not add entries to force a diff.
- [ ] T07 (AC6–AC7): Review preservation of #171/#172 proof ownership, preflight,
  non-login execution and current recipes; commit the coherent implementation
  unsigned before final verification. Proof: clean committed candidate diff,
  `cargo test --locked --test delivery_constitution`,
  `cargo test --locked --test delivery_recipes`, design-check selftest and
  canonical `delivery-proof-test.sh` pass without bypasses.
- [ ] T08 (AC7): Hash the candidate-local engine, run every house gate from the
  charter inventory with offline Cargo, including exact coverage and its
  selftest, plus four warning-free recipe compiles and fail-closed native-tls
  absence. Proof: per-command logs/statuses on the committed candidate, zero
  missed lines/branches/functions and successful selftest; report failures
  rather than treating baseline design evidence as implementation verification.
- [ ] T09 (CI-owned): Obtain the PR CI receipt for the real published artifact
  download, matching digest and four successful recipe compiles, retaining
  source tests and all ordinary CI obligations. Proof: `brokkr bundle (pinned)`
  on the PR uses the committed URL/digest and succeeds. This seat neither
  accesses GitHub nor fabricates the receipt; mark pending until CI provides it.
