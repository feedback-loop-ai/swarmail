# Tasks: metric parser proof

Ordered implementation/proof work under [spec.md](spec.md) and [plan.md](plan.md).
These are downstream tasks, not claims that the chief executed them. The
single dated receipt belongs in [the owning plan](../decoded-inbox-metrics/plan.md).
Preserve true/low until fresh protected disposition. Check a box only after
its entire paired proof exists. AC6 stops dependent success-path tasks.

- [ ] T1 — Resolve R1 before dependent proof: identify an available offline
  upstream parser, pin provenance/version/digest, and record its exact
  invocation, format selection, byte-safe output and controls in this plan
  and the owning receipt (AC3). **Test:** execute a valid fixture and malformed
  syntax controls through that actual artifact; assert decoded bytes and
  expected success/failure. Tool absence leaves this unchecked and the proof
  blocked; lint alone, a hypothetical command or another local reader fails.
- [ ] T2 — Bind the new experiment to its candidate and preserve both historical
  events, all eight notes, five newer results, stopped triage and seq-625 ruling
  (AC1). **Test:** recheck all ten receipt SHA-256 values against inventories,
  verify unchanged historical text/typed facts and record HEAD/tree and source
  limitations; do not claim journal authentication from copied files.
- [ ] T3 — Extend binary HTTP capture in `src/api/refusal_tests.rs`, retaining
  metadata and complete correctly de-framed body (AC2). **Test:** targeted
  real-router harness observes status, actual Content-Type/parameters and
  byte length/framing; CR/NUL-containing bodies round-trip unchanged with
  identical capture/parser-input digests. Incomplete/unsupported framing
  fails explicitly; no trim, Unicode splitting or shell-variable transport.
- [ ] T4 — Extend the existing sequential hold owner with every AC2 scalar,
  combined value, original CR/NUL pair, ordinary Unicode and three escape
  controls; audit exact fixture attribution and cap headroom (AC2/AC4).
  **Test:** `cargo test --locked --lib api::refusal_tests` proves before/held/
  released states, exact existing response bytes, one decoded value-1 request
  row, no raw alias or extra fixture row, no refused await registration and
  cleanup; normal parallel lib tests pass with bounded joins and no resets.
- [ ] T5 — Execute the actual upstream parser over every complete raw capture
  using its observed format/configuration, preserving the current renderer
  (AC3/AC5). **Test:** all individual and combined cases parse completely with
  byte-exact labels and exact fixture sample sets; malformed syntax fails;
  deliberately truncated/changed expected labels and extra fixture samples
  fail comparator assertions, followed by restored-green. Record commands,
  versions, digests, exits and the exact controlled assertion-red.
- [ ] T6 — Record the experiment's disposition (AC1/AC6). **Test:** on success,
  the owning receipt maps every case to captured and parsed bytes, states
  observed version/format limits and retains true/low. On counterevidence,
  inspect `.forge/epic192/issue202/operator-proposal.md` for exact reproducible
  failing evidence, affected clauses, precise proposed amendment, alternatives
  and identity/coalescence/refusal effects, marked unnumbered PROPOSED / NOT
  ACCEPTED; verify no contradictory source/policy edit and stop. Missing
  tooling instead leaves T1/T5 open without inventing a failure or amendment.
- [ ] T7 — Preserve established contracts and proof scope (AC4/AC5/AC7).
  **Test:** run locked lib-api, integration api, store_contract_docs and
  delivery_recipes suites; retain invalid/mixed UTF-8 refusal, two parked
  awaits/same-scrape six rows/joins/cleanup/cumulative retention, hold drop
  proof and old-helper sensitivity evidence. Review unchanged decoder,
  renderer, admission, guard order, 0065/0043 marker and frozen guarantees;
  disclose the finite manual pointer-audit limit.
- [ ] T8 — Freeze the smallest conformant test/evidence candidate and verify
  every authoritative house gate (AC7). **Test:** final HEAD/tree-bound
  successful targeted and full locked suite receipts, fmt/clippy, digest-bound
  compile, exact zero-missed coverage plus selftest, publish/seal proofs,
  licenses, public-shim checks and fail-closed native-tls absence as specified
  by the charter. Preserve failures and CI-only distinctions; historical
  receipts and this design's checks cannot complete this task.
- [ ] T9 — Obtain all five fresh protected accepted-0058 results on the final
  candidate and resolve residuals only through that process (AC8). **Test:**
  actual separate Astra correctness, official DeepSeek Flash security, Sonnet
  5.5 tests, Astra guarantees and independent Fable 5.1 results bind to the
  verified HEAD/tree, with no Pro/fallback and policy-admissible security
  disposition. Missing/unknown/true security facts retain the hard stop;
  no author or design result clears them. No push, merge, tag, publication,
  issue closure or decision acceptance.
