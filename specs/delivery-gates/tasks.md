# Ordered tasks — delivery gates

Tasks remain unchecked until their paired proofs are reconciled. The code
was committed in `d1dc551`; unchecked tasks do not request reimplementation.
A planned proof or prior green-tree report is not a final-tree test receipt.
Acceptance references point to [spec.md](spec.md); mechanisms and touched
files are in [plan.md](plan.md). The original implementation order was #140
before pinning its gate for #123, then #124, #125 and #143. Restore every
deliberate mutation.

## Resume order — 2026-10-19 third resume (controlling)

The operator ruling in chat on 2026-10-19 authorizes the documentation
amendment to `ad02e3c`'s existing policy shape. This order supersedes older
instructions to keep 0050 unchanged; retain their dated history. Two signed
commits serve separate phases: chief confirmation, then implementation's
amendment. Unchecked original tasks are proof obligations, not rebuild requests.

- [ ] D01 — Commit the three existing design artifacts as chief's confirmation.
  **Proof:** signed HEAD touches exactly `specs/delivery-gates/{spec,plan,tasks}.md`;
  `git verify-commit HEAD` and
  `bash bundles/self/scripts/design-check.sh check .` pass on a clean tree.
  Proposed 0050 remains committed and unchanged in this design step. Spec D5.
- [ ] D02 — Amend proposed 0050 §4 in place and reconcile the three artifacts.
  **Proof:** §4 carries the dated operator ruling, dead-rule cause, ordered
  ABOVE-MEDIUM → OK → UNSAFE shape, explicit predicates, unconditional hard
  fallback, no coercion/default/inference and named pin. Read spec's residual
  requirement/AC8, plan §4 and T08 together: no current instruction contradicts
  the amendment. Preserve proposed status and dated history. Spec D1/D2.
- [ ] D03 — Prove the amendment changes documentation alone and sign it.
  **Proof:** combined diff from `ad02e3c` names exactly the four documents in
  spec D3; policy and pin test are byte-identical. Both new commits verify as
  GPG-signed, no history is rewritten and the branch is clean. Run
  `cargo fmt --all` before checks and
  `cargo test --test delivery_constitution --locked` unchanged. Spec D3/D5.
- [ ] D04 — Reconcile receipts, complete remaining execution and hand off.
  **Proof:** actual evaluator truth table and actual design dispatch/retry
  close AC8/AC7; compiler sweep alone cannot close them. Full final-tree house
  inventory, warning-free pinned compile and remaining AC1–AC13 proofs have
  candidate/pin/command/verdict receipts. Record blocked or unrun work; do not
  claim completion or shipping from design, and never push. Spec D4.

## Resume order — 2026-10-19 second resume

The first resume `resume-point-read-first-epic-120-2c563256` produced
confirmation `d37533c` and comparator repair `93992af`. Current run
`resume-point-2-read-first-batch--206e588f` confirms those artifacts after
the operator's digest record `83107fb`, then checks the existing implementation.
Only a demonstrated gap requires a fix. Preserve AC1–AC13 and proposed 0050.
Prior receipts remain prior receipts until reconciled with the candidate;
unchecked proof tasks do not request rebuilding their existing implementation.

- [ ] R01 — Append the signed confirmation of the existing three artifacts.
  **Proof:** HEAD touches exactly `specs/delivery-gates/{spec,plan,tasks}.md`,
  signature verifies, tracked tree is clean, and
  `bash bundles/self/scripts/design-check.sh check .` succeeds with proposed
  0050 unchanged. This proves local artifact validation, not engine dispatch.
- [ ] R02 — Verify the launched identity against the existing operator record.
  **Proof:** `83107fb` supplies the digest and honest distinct-build provenance;
  actual launch matches it and reaches gates, missing/mismatched identities
  refuse by name, and seat/CI engines each compile without policy warnings.
  Do not infer that this digest was built from `brokkr.sha`, replace the
  operator record, or use an arbitrary PATH checksum as authorization. AC5.
- [ ] R03 — Reconcile and rerun the delivered comparator proof.
  **Proof:** existing `v4-coverage-bool` and `v4b-debug-bool` refuse, the
  debug-assert green twin and real tree pass; reconcile `93992af`'s recorded
  old-comparator mutant failures with final selftest receipts and T02.
  Keep exact no-coverage comparison and the narrow release-only allowance.
- [ ] R04 — Establish the missing actual-path evidence for T05/T07/T08.
  **Proof:** actual pinned verify on the uncovered/`exit 0` candidate returns
  fail with both integrity and coverage diagnostics, covered clean control
  passes, real design dispatch validates before advancement and demonstrates
  bounded retry, and the pinned evaluator truth table plus unsuppressed
  compile satisfy AC8. Separate component calls do not discharge AC4, even
  when combined with a clean seat dispatch. Reconcile existing receipts first;
  any missing actual-seat fixture remains a batch obligation, not a deferred
  follow-up. A minimal harness repair requires its own signed fix commit.
- [ ] R05 — Reconcile T01–T13, run final gates and hand off to review.
  **Proof:** every claimed completion has candidate/pin/command/verdict
  receipts, required #143 mutations fail then pass after restoration, full
  house inventory is green, history remains signed and the branch clean.
  Unrun or blocked obligations remain explicit; no shipping claim here. AC13.

## Original implementation and proof obligations

- [ ] T01 — Establish the authorized branch, baseline and execution interfaces.
  **Proof:** main descends from `2a325c9`, includes `18374cc` or later batches;
  real pinned minimal exec sequence demonstrates last-step result and failure
  counting; actual pin identity and evaluator access are recorded. Confirm
  signatures and clean baseline; no static-only engine proof. AC5/AC7/AC8.
- [ ] T02 — Close coverage cfg/release and package-inventory bypasses (#140).
  **Proof:** real-build selftest arms v1/v2/v3/v3b/v5b refuse for their named
  cause, v4/v4b boolean controls refuse after `93992af`, clean controls pass,
  root identity is independent of package sorting,
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
- [ ] T08 — Confirm ABOVE-MEDIUM → OK → UNSAFE (#125), per the operator
  ruling of 2026-10-19; `ad02e3c` already implements the shape.
  **Proof:** the unchanged named structural pin in AC8 passes. The pinned
  actual evaluator truth table covers each absent/null/wrong type,
  unknown/wrong-case severity, all supported levels and both security
  booleans, both fields absent, security-hold and unchanged clean-review paths.
  Above-medium severity stops first even with invalid security; only explicit
  false security plus valid severity at most medium ships; all other residual
  rows reach the final unconditional UNSAFE hard stop, without coercion,
  defaults or missing-fact inference. Compile exits zero without the residual
  policy warning. Retain baseline unsafe-row red evidence; neither the
  structural pin nor compiler sweep replaces evaluator execution. AC8.
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
