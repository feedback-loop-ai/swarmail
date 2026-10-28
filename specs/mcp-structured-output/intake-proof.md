# Intake gate receipt — issue #153

All thirteen local gate steps exited zero on committed framing
`54a6716260d6df2e221dbb388866dd0c1fe01bde`. This proves the intake candidate, not the
unimplemented #153 feature. The source tree, dependencies, scripts and routing
were unchanged. This receipt and the decision-index row placement are the only
follow-up documentation changes; the complete gate run was not repeated for them.

Candidate engine SHA-256: `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Offline execution installed nothing. Public shim hashes passed; pinned bundle
scripts owned the proofs. Bundle compilation emitted no policy-sweep warning.

| Step | Exit | Seconds |
| --- | --- | --- |
| shims | 0 | 0.01 |
| fmt | 0 | 0.76 |
| clippy | 0 | 17.44 |
| test | 0 | 165.53 |
| bundle | 0 | 0.03 |
| coverage | 0 | 254.09 |
| coverage-selftest | 0 | 19.84 |
| design-selftest | 0 | 4.77 |
| delivery-proof | 0 | 9.08 |
| publish-guard | 0 | 6.07 |
| seal-branch | 0 | 4.82 |
| licenses | 0 | 0.66 |
| native-tls | 0 | 0.21 |

Exact coverage and its selftest both passed: no floor change or exclusion.
The workspace call timed out at 300 seconds, but its runner continued; the
completed summary and per-step logs were subsequently read from
`.forge/issue153-intake-gates/`. No timeout is represented as a successful gate.

CI retains macOS, chart/history, Docker, privileged port-80 and network RustSec
proofs. The operator retains live clients/settings, model measurements, #151
baseline, sealing and merge. #152/#153 remain held unmerged. No implementation,
paid trial, push, publication, issue closure or decision acceptance occurred.
