# Ordered tasks — coverage selftest

All boxes describe downstream work, not work claimed by the design commit.
Each task owns its proof; preserve dated clarification entries if returned.

- [ ] **T1 — Establish the bounded baseline (AC8).** Verify ancestry from
  ff9bf8aba26fabaeea89876d5257b58198a12601 and inventory every refusal arm with
  its cause/context. Confirm only selftest code is an implementation target.
  **Proof:** git diff/ancestry inspection, scanner and non-selftest gate
  byte comparison, and an arm-to-reason audit preserving all #173 regressions.
- [ ] **T2 — Require diagnostic-specific refusal assertions (AC1–AC2).**
  Convert and remove the status-only helper, strengthen weak existing needles,
  preserve context/partial-output checks, remove the aggregate inner-attribute
  claim while retaining the isolated compiler proof. Add child-local controls.
  **Proof:** syntax check and selftest; intended diagnostic + failure passes,
  unrelated exit 127 and intended text + success increment child failures;
  parent failures remain intact. Static audit finds no status-only helper/call.
- [ ] **T3 — Cover malformed free-space output (AC3, AC7).** Add the scoped
  successful-df/nonnumeric-column fixture without touching production parsing.
  **Proof:** selftest requires read-space category and fixture path; numeric
  success, short-space and stat-failure controls retain their own outcomes,
  with subsequent real commands unaffected by the override.
- [ ] **T4 — Prove derive scope and the green dump premise (AC4–AC5, AC7).**
  Generate a real derive-before-offender red/green pair and assert canonical
  ordering/context. Inspect the existing green dump's generated Eq impl.
  **Proof:** uniquely named post-derive offender refuses, clean twin passes;
  green dump contains the associated minted coverage(off)/assert_fields_are_eq
  before scanner success is accepted. Missing shape is an explicit test
  failure, not a skip or inferred pass.
- [ ] **T5 — Exercise the binary paths independently (AC6–AC7).** Generate
  binary-only red/green crates, offline locks, executable tests, actual reports,
  dep-info inventories and explicit bin expansions with checked fixture builds.
  **Proof:** harvest names the bin root and bin-only fn-bearing include;
  stage 1 names the red missing file and cause, stage 2 names the excluded bin
  function, and the exercised measured green binary passes both checks.
- [ ] **T6 — Verify complete selftest coverage and scope preservation
  (AC1–AC8).** Run the full strengthened selftest and audit final negative
  assertions, real-fixture premises, isolation and cleanup.
  **Proof:** selftest exits zero with all new/retained arms, no arbitrary
  infrastructure failure counts as a refusal, and baseline comparison confirms
  unchanged scanner, production gate/verdict, shims, dependencies and routes.
  Return any discovered production defect through scope handling.
- [ ] **T7 — Prove the implemented candidate through every local gate
  (AC8–AC9).** Run the entire house-rule inventory, separately including exact
  application coverage and coverage selftest; also run design-check selftest,
  delivery tamper proof, strict shim digests and native-tls absence.
  **Proof:** command/status/candidate/pin receipts show fmt, locked clippy/tests,
  digest-matched bundle compile, exact positive-count 100/100/100, all required
  selftests, publish-guard, keyless seal, licenses and explicit package-absence
  diagnostic. Failed/blocked checks remain outstanding; baseline runs do not
  certify the implemented candidate.
- [ ] **T8 — Hand off the proved candidate on the selected engine lane
  (AC9).** Preserve the full protected review and security hold, pinned Qwen,
  Opus, official DeepSeek Flash and GLM routes, with no fallback or Pro.
  **Proof:** actual engine verify/review receipts refer to the implemented
  candidate; CI-only obligations are recorded as CI-owned until evidenced.
  Keep honest task disposition, unsigned author commits and no push, merge,
  tag or publish.
