# Ordered tasks — delivery gates

All tasks are initially unchecked. A planned proof is not a test receipt.
Acceptance references point to [spec.md](spec.md); mechanisms and touched
files are in [plan.md](plan.md). Implement #140 before pinning its gate for
#123, then #124, #125 and #143. Restore every deliberate mutation.

- [ ] T01 — Establish the authorized branch, baseline and execution interfaces.
  **Proof:** main descends from `2a325c9`, includes `18374cc` or later batches;
  real pinned minimal exec sequence demonstrates last-step result and failure
  counting; actual pin identity and evaluator access are recorded. Confirm
  signatures and clean baseline; no static-only engine proof. AC5/AC7/AC8.
- [ ] T02 — Close coverage cfg/release and package-inventory bypasses (#140).
  **Proof:** real-build selftest arms v1/v2/v3/v3b/v5b refuse for their named
  cause, clean controls pass, root identity is independent of package sorting,
  external path closure refuses and standalone scanner workspace passes;
  original behavior passes the bypass fixtures, proving the regression.
  Preserve dep-info-before-expansion and exact real-tree counts. AC1/AC2.
- [ ] T03 — Scan test expansions and repair exclusion selftests (#140).
  **Proof:** v6 refuses, single-offender inner fixture fails when the inner
  walker is deleted and passes with it restored, inner allow stays green,
  pub-fn diagnostics name the offender, all old A–G/selftests pass. Correct
  charter wording and verify accepted 0020 is unchanged. AC3.
- [ ] T04 — Relocate the repaired gates, scanner, publish guard/proof and
  prerequisite support into one pinned implementation chain (#123/#124).
  **Proof:** actual dispatched seat opens nested resources from the pin,
  scanner builds offline/locked outside the bundle, missing/substituted helper
  and wrapper fixtures fail by name, ordinary public commands still pass;
  no duplicate full implementation or committed target artifacts. AC4/AC5.
- [ ] T05 — Prove verify's tamper boundary, pin identity and result writing (#123).
  **Proof:** actual verify with uncovered candidate plus `exit 0` public gate
  records both integrity and canonical coverage failure; covered clean control
  passes, `exit 1`/missing/modified wrappers refuse, wrong or unknown brokkr
  identity refuses, changed canonical bytes change bundle digest, old pinned
  dispatch ignores candidate proof replacements. JSON/atomic-write fault
  fixtures never expose a stale or partial successful result. AC4/AC5.
- [ ] T06 — Implement deterministic HEAD artifact validation and pinned
  prerequisites (#124). **Proof:** dispatched Git fixtures cover missing risks
  beside a newer-mtime old valid spec, missing/empty files, missing acceptance
  or tasks, zero/multiple slugs, unrelated HEAD, untracked/dirty artifacts and
  decisions, missing/invalid declared proposed records, helper absence/failure,
  ordinary authority links, normal checkout and linked worktree; only the valid
  committed design succeeds. AC6.
- [ ] T07 — Make the last deterministic check own advancement (#124).
  **Proof:** real engine dispatch runs check after chief, `drafted` alone stops,
  valid artifacts advance only after `designed`, first failure retries once,
  second stops. Exercise fallback phase accounting only if the pinned grammar
  requires it. Verify/CI run the delivery harness without recursion. AC7.
- [ ] T08 — Require explicit residual facts and stop unknowns (#125).
  **Proof:** pinned actual evaluator truth table covers each absent/null/wrong
  type, unknown/wrong-case severity, all supported levels and both security
  booleans, security-hold and unchanged clean-review paths. Only explicit safe
  residual rows ship; compile exits zero without the residual-policy warning.
  Retain baseline unsafe-row results as red evidence. AC8.
- [ ] T09 — Restore deterministic lifecycle assertions and bound helper reads
  (#143). **Proof:** handler-observed real HTTP request always asserts 503
  stopping; drop-instead-of-reply mutant fails; independent stalled-body fence
  test catches a removed drain bound; every silent-peer helper case fails
  within its named deadline, with an outer watchdog catching removed deadlines.
  Remove the 30 s fence substitution and preserve 0027 closure semantics. AC9.
- [ ] T10 — Strengthen code/document, configuration, help and env-bind pins
  (#143). **Proof:** changed DEFAULT_MAX_SESSIONS and other documented values
  fail, file/index status mismatch fails both directions, simultaneous fixture
  acceptance/index update passes; crossed/ignored idle wiring fails actual
  Config-use tests, incorrect Host/Origin help fails, ignoring each env bind
  fails port-0/held-listener controls with no release/rebind gap. AC10.
- [ ] T11 — Add actual-path allocation and payload-sharing proofs (#143).
  **Proof:** scoped declared POP3 allocation harness fails separately for a
  second full RETR buffer and per-mail payload_len allocation; real RETR bytes
  remain correct. Copy inserted in post_json fails the request-sharing proof.
  Counters are isolated under parallel tests and disarm on unwind. AC11.
- [ ] T12 — Guard remaining scratch fixtures and prove decode-window behavior
  (#143). **Proof:** deliberate assertion-failure child runs leave no new owned
  swarmail-* artifacts or database sidecars in the actual temp root; decode
  search-past-window mutant fails the bounded output/traversal test, restored
  decoder and all earlier exact extraction pins pass. AC12.
- [ ] T13 — Reconcile delivery records and execute final proofs for all issues.
  **Proof:** signed implementation commit messages map #123/#124/#125/#140/#143
  to named passing tests and red controls, with candidate/pin IDs and command
  receipts; 0050 indexed as proposed, #105/#119 assertions corrected by actual
  evidence, no accepted decision body rewritten. Run the full ordered house
  inventory, both script selftests, publish proof, delivery harness, fail-closed
  native-tls absence and warning-free pinned compile on the restored final
  tree. Confirm exact 100/100/100, no dependency allowlist widening, clean
  authorized branch and signed history; no push/PR/merge/issue closure. AC13.
