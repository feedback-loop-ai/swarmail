# Run 6b — the trustworthy-gates batch verified, reviewed, and shipped

Feature: complete the run-6b-completion-trustworthy-gates-batch — the
"Make the gates, tests and delivery trustworthy" batch (issue #58
children #50–#53) is already implemented and committed (design `463f9d2`
… `584b23d` "tick the spec's task list: all twenty delivered", with the
operator's `b9c2184` + `a9efa40` on top); what it never received is its
verdict. The verify seat was killed twice at the bundle's 3600 s
deadline — the batch's hermetic workspace suite alone takes ~49 minutes
and the new gate set exceeds one hour — so no seat has proved the
delivered tree and no review has voted on it. The deadline was a hang
guard, not a verdict. The operator has since raised the verify budget to
10800 s (`bundles/self/bundle.json`), applied the two ruled operator
edits (`bundles/self/tooling.json` — the pinned brokkr record by commit
SHA; `bundles/self/scripts/verify-seat.sh` — the gate selftest, the
publish-guard proof and the fail-closed native-tls probe), accepted
decision 0026 (`a9efa40`), and re-run the house gates on the delivered
tree. Run 6b earns the verdict: verify runs the full hermetic gate set
to exit 0, the review panel judges the batch against the four children's
acceptance criteria, the parent spec, and accepted decision 0026, and
ship signs only on a clean, drift-free tree.

Why this is a trust matter and not a formality: the batch exists because
*a gate that sometimes lies is worse than no gate* — and a completion
claimed without its proof is the same species of lie, told by the run
itself. So run 6b has no design of its own. The design of record is
`specs/trustworthy-gates-tests-delivery/spec.md` (R1–R15) as realized in
the nine delivered commits; this spec frames the verification, the
review, and the only repairs the run may carry. Framing:
`.forge/run6b-feature.txt` (there is no `.forge/tasks/` in this tree; the
run brief is the framing) plus the delivered constitution in
`specs/trustworthy-gates-tests-delivery/`. Both panel seats read the
delivered tree this session (`.forge/design/positions/simplicity.md`,
`.forge/design/positions/robustness.md` — run-local evidence, uncommitted)
and converged on the same verdict: the delivered tree is the smallest
honest shape the rulings admit, and the lie-free pipeline holds under
every failure mode the framing names. Between them sits one named defect
and the question of the run's exact scope, ruled below; where the
positions conflict, the ruling names the loser's cost.

## Positions and rulings

**R1 — Scope: the delivered tree is the design of record; the run's
production delta is zero bytes.** *(Both seats converge; simplicity's
audit is adopted as the scope ruling.)* The batch is delivered, gated and
tested; a completion run that rewrites working machinery to satisfy an
aesthetic is a new risk with no gate demand behind it. Adopted verbatim,
simplicity's refusal list: no new gauge, registry, endpoint or bench; no
retention policy for `target/coverage/run.*`; no parser rewrite of the
gate's awk checks; no `#[cfg(test)]` seam in this batch's surface; no
re-pin, version bump or dependency ride (`Cargo.lock` and `deny.toml`
untouched); no doc ride-alongs beyond the two repairs ruled below; no
edit to `agents/`, `adapters/`, or `bundles/` by any seat (the operator
channel `.forge/run6b-operator-edits.md` exists; nothing foreseen needs
it). Cost of the refusal — the losing side is the temptation to fix
robustness's three recorded losses now: the global webhook cap that lets
one hostile sink starve every target with an unattributed dropped row,
attacker-shaped aggregate memory owned by the deployment boundary
(decision 0026's documented relation), and unbounded idle peers on the
trusted-harness plaintext posture. Each is a *next-feature candidate,
recorded here as a named residual, not run-6b work*; taking them in a
completion run is churn without a gate behind it and a reopened batch.

**R2 — The panel-named defect is repaired: the `ChaosEvent::Data` doc
comment.** *(Robustness's finding, adopted; simplicity's "no doc
ride-alongs" loses for exactly one sentence — this is not a ride-along
but a defect the panel named, which the framing's own exception covers.)*
Verified in tree by the chief: `src/chaos.rs` promises *"a delay-only
rule stores the mail and answers `250` after the wait"*, while
`src/smtp.rs`'s DATA arm — issue #17's one policy at every stage — waits,
then fails with the explicit `error` line or `451 4.3.0 chaos`, and never
stores the mail; `ChaosRule`'s own `error`-field doc in the same file
already states the true policy, so the file contradicts itself; and
`tests/smtp_edges.rs::chaos_on_connect_mailfrom_and_data_with_and_without_error`
asserts the wire truth (a delay-only DATA rule answers `451 4.3.0 chaos`
and the inbox stays empty). A test author trusting the variant doc reds
against correct code. The repair is doc-only — no behavior change, no
frozen guarantee touched, no ruling unsealed — so it carries **no
decision document**. Cost of refusing it: the trust run ships a citation
that lies, the exact species R15 deleted ("decision 0063").

**R3 — The citation graph is completed: the decisions README indexes
0026.** *(Chief-ruled; verified in tree: `docs/decisions/README.md` ends
at 0025 and contains no 0026 row, while the operator accepted 0026 in
`a9efa40` — and the index has carried rows for proposed decisions all
along.)* The realm's intake discipline reads the decisions index before
framing anything; an accepted decision absent from it frames the next run
against a stale decision set, and the deployment boundary is exactly the
kind of settled posture a future intake would otherwise re-argue. One
row, in the established format, citing the file and its accepted status.
Cost of refusing it: the index understates the accepted set by one
decision — the cheapest lie of omission in the tree. Cost of the row:
one line of churn. This is the chief's own finding, not either seat's;
it is ruled here so it lands as a named repair rather than drift.

**R4 — The 10800 s verify budget is the minimal honest completion, and
the deadline must not fire again.** *(Both seats converge; simplicity §5
adopted.)* The budget now equals the work — the ~49 min hermetic suite,
the instrumented exact-gate run that rebuilds in its own
`CARGO_LLVM_COV_TARGET_DIR`, the selftest, the publish-guard proof, deny
and the bundle compile. Refused: splitting the seat, parallelizing the
suite, or dropping a gate from the set — new machinery or dishonesty,
each. The seat runs to exit 0 or fails with its decisive output; a
deadline kill is a failed attempt (max_attempts 2 stands), reported as
such — never economized into a green.

**R5 — The exactness proof stands at the delivered tree and is
re-detected at the run's final HEAD.** The parent spec's AC50.1/AC50.2
proof exists as evidence on disk under `target/coverage/` (gitignored,
run-local): the concurrent pair `run.1DTHyN` + `run.Tg4n8z` — summaries
both exact 5826/320/729, finished 0.33 s apart, which two sequential
gate runs cannot do — plus the further exact runs `run.toMECD` and
`run.YDofH1`. That is AC50.1's three-runs-with-identical-counts and
AC50.2's distinct-evidence demonstration at the delivered HEAD. The R2/R3
repairs are comment- and doc-only, so they must move no measured line:
the verify seat's run at the final HEAD is the detector, and any drift
from 5826/320/729 stops the run (the delta was not what it claimed).
Re-staging a fresh concurrent pair at the final HEAD is refused as
redundant wall-clock; if the review panel names a *production* defect
and it lands, the exactness proof is re-staged at the new HEAD (a
concurrent pair plus one further run).

**R6 — Red is red, and every check must distinguish checked-and-clean
from could-not-check.** *(Robustness's red lines, adopted verbatim.)* No
gate retries a red verdict — the rerun habit launders the cold-line miss
and every future red with it. A red gate in run 6b is reported red and
repaired the way `cc555d6` was: named, minimal, its own commit, its test
landing with it, never re-slept; then the gates that read the touched
paths re-run (the full seat again if `src/` changed). The fail-closed
posture of the new checks — a missing or malformed record refuses with
the path named, a probe that cannot probe refuses — stays exactly as
delivered. security-hold is a hard stop (constitutional, protected
phase).

**R7 — Review closes the gap the design panel named.** *(Robustness's
ask, adopted; simplicity's argument is the reason no new test is
demanded, not a substitute for the read.)* Robustness did not read
`src/api.rs`'s gauge plumbing and named it "the only place a new lie
could hide"; simplicity argues every line of it is under the exact gate
and executed by the very polls that demand it. Both are right, and the
ruling is: the correctness member reads the run's only `src/` delta —
the three labeled gauge registries and their drop-guards, the park
wiring in `src/mcp.rs`/`src/smtp.rs`, and the pre-bind SIGINT watcher in
`src/main.rs` — against AC51.3 and the frozen guarantees before voting.
Cost: ~180 lines read. Cost of refusing it: the panel's pass rests on an
unread delta.

Review mechanics of record: the four members of
`bundles/self/bundle.json` (correctness, security, guarantees, tests)
judge the delivered tree against the four children's acceptance criteria
(issues #50, #51, #52, #53 of
https://github.com/feedback-loop-ai/swarmail/issues/58), the
parent spec (`specs/trustworthy-gates-tests-delivery/` — R1–R15 and its
ACs), and accepted decision 0026. The issues' line references are from
baseline `ac8c326`; main has moved — **locate by shape, not by line**
(the five `src/` hunks of simplicity §2, the gate's checks, the chart's
three postures, the CI posture). No member closes an issue or opens a
PR; the operator lands via PR and closes the children with CI citations.

## Acceptance Criteria

**Verify — the parked work, completed**

- AC-6b.1: `bash bundles/self/scripts/verify-seat.sh` exits 0 at the
  run's final HEAD within the 10800 s budget, with every declared gate
  green and none skipped: `cargo fmt --all -- --check`; clippy
  `--workspace --all-targets --locked -- -D warnings`;
  `cargo test --workspace --locked`; `brokkr compile --bundle
  bundles/self`; the exact coverage gate (literal 100/100/100);
  `coverage-gate.sh selftest`; `scripts/publish-guard-test.sh`;
  `cargo deny check licenses`; the fail-closed native-tls probe. The
  seat is not killed at its deadline (the parked run's failure mode);
  the seat's notes line names all nine gates; a red gate is reported
  red, never retried into green.
- AC-6b.2: The exactness proof of the parent spec's AC50.1/AC50.2 is
  available to review — the captured concurrent pair and further exact
  runs named in R5, or a re-staged concurrent pair at the final HEAD if
  that evidence was reaped — and the verify seat's run at the final HEAD
  reads the identical counts 5826/320/729; any drift stops the run.

**The repairs — the run's only delta**

- AC-6b.3: `src/chaos.rs`'s `ChaosEvent::Data` doc states the delivered
  DATA policy — a fired rule waits, then fails: an explicit `error`
  line, else `451 4.3.0 chaos`; the mail is never stored, delay-only or
  not — agreeing with `src/smtp.rs`'s DATA arm and `ChaosRule`'s
  `error`-field doc; the suite stays green, in particular
  `tests/smtp_edges.rs::chaos_on_connect_mailfrom_and_data_with_and_without_error`;
  no behavior changes anywhere in the run.
- AC-6b.4: `docs/decisions/README.md` contains a 0026 row in the
  established format — link to
  `0026-deployment-boundary-peers-tls-sizing.md`, the decision's title,
  status accepted — and the index stays ordered; no other doc prose
  changes in the run.
- AC-6b.5: Nothing else changes: measured against the delivered tree
  (`a9efa40`), the run's commits are the spec trio plus the two named
  repairs; `src/`'s only delta is the repaired `ChaosEvent::Data` doc
  comment; no commit touches `tests/`, `scripts/`, `charts/`,
  `.github/`, `Cargo.toml`, `Cargo.lock` or `deny.toml` unless a red
  gate or a review-named defect forces it, and each such commit is
  named, minimal, and lands with the test that proves it. No decision
  document is drafted: nothing semantic changes, and decision 0026 is
  already accepted.

**Review and ship**

- AC-6b.6: The review panel returns typed verdicts (clean | residual |
  security-hold per member; the bundle's aggregate) judging the batch
  against the four children's acceptance criteria, the parent spec, and
  accepted decision 0026, locating by shape; the correctness member's
  read includes the gauge plumbing per R7; any defect the panel names
  lands per AC-6b.5's discipline; security-hold stops the run.
- AC-6b.7: Ship signs only with `git status --porcelain` empty and no
  drift: the eight frozen required context names in
  `.github/workflows/ci.yml` byte-identical (`format, clippy, publish
  guard`, `test (ubuntu-latest)`, `test (macos-latest)`, `coverage`,
  `deny`, `RustSec dependency audit`, `helm chart`); `agents/`,
  `adapters/`, `bundles/` untouched by seats; no issue closed and no PR
  opened by seats; issue #32's delivery (`3e0a996`, `51bac94`,
  `ac8c326`) untouched and not reopened.
- Standing gates: the six house gates clean at the run's final HEAD —
  fmt, clippy, `cargo test`, `cargo deny check licenses`,
  `brokkr compile --bundle bundles/self`, and the exact coverage gate
  (subsumed by AC-6b.1's seat run; the floor stays exact 100/100/100,
  no `coverage(off)`, no exclusion).
