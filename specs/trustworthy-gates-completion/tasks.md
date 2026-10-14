# Tasks — run 6b: verify, review and ship the trustworthy-gates batch

Ordered. Each task is paired with the test or gate that proves it (real
protocol against real servers, never mocks — 0004). The repairs land
first so the tree the verify seat proves is the tree that ships; the
verify seat is the arbiter; review is the protected phase; ship signs
only on a clean, drift-free tree. Any edit beyond tasks 1–2 requires a
red gate or a review-named defect and lands as its own minimal named
commit with its test (spec R1/R6).

- [ ] **1. Land the panel-named doc repair (spec R2)** — in
  `src/chaos.rs`, rewrite the `ChaosEvent::Data` variant doc to state
  the delivered DATA policy: *a fired rule waits, then fails — an
  explicit `error` line, else `451 4.3.0 chaos`; the mail is never
  stored, delay-only or not* — agreeing with `src/smtp.rs`'s DATA arm
  and `ChaosRule`'s `error`-field doc in the same file. Own commit,
  named; no other `src/` change.
  *Test:* `cargo test --workspace --locked` green — in particular
  `tests/smtp_edges.rs::chaos_on_connect_mailfrom_and_data_with_and_without_error`,
  which already asserts on the wire that a delay-only DATA rule answers
  `451 4.3.0 chaos` and stores nothing; `cargo fmt --all -- --check`
  and `cargo clippy --workspace --all-targets --locked -- -D warnings`
  clean; the exact gate's counts unchanged (task 3 is the detector).
  Proves AC-6b.3.
- [ ] **2. Complete the citation graph (spec R3)** — add one row to
  `docs/decisions/README.md` indexing accepted decision 0026
  (link `0026-deployment-boundary-peers-tls-sizing.md`, its title,
  status accepted), in the established format and order. Own commit,
  named; no other docs change.
  *Test:* grep proves the row exists and links the file; the index
  remains ordered with 0026 after 0025; `git diff a9efa40..HEAD --
  docs/decisions/README.md` shows exactly one added row. Proves
  AC-6b.4.
- [ ] **3. Run the full hermetic verify seat (the parked work)** —
  `bash bundles/self/scripts/verify-seat.sh` to exit 0 within the
  10800 s budget at the run's final HEAD: fmt, clippy, full `cargo
  test`, `brokkr compile --bundle bundles/self`, the exact coverage
  gate, the gate's selftest, `scripts/publish-guard-test.sh`,
  `cargo deny check licenses`, the fail-closed native-tls probe. No
  gate skipped, no verdict retried.
  *Test:* the seat's own result record (pass, with the notes line
  naming all nine gates) and the exact-gate summary at the final HEAD
  reading 5826/320/729 — identical counts prove the doc repairs moved
  nothing measurable (spec R5's detector); any drift stops the run.
  Proves AC-6b.1 and AC-6b.2's detector half.
- [ ] **4. Review the delivered batch (protected phase)** — the four
  members judge the tree against the four children's acceptance
  criteria (issues #50–#53 of #58), the parent spec
  (`specs/trustworthy-gates-tests-delivery/`, R1–R15 + ACs), and
  accepted decision 0026 — locating by shape, since the issues cite
  baseline `ac8c326`. The correctness member reads the run's only
  `src/` delta (the labeled gauge registries and drop-guards in
  `src/api.rs`, the park wiring in `src/mcp.rs`/`src/smtp.rs`, the
  pre-bind SIGINT watcher in `src/main.rs`) against AC51.3 before
  voting (spec R7). Each member types clean | residual |
  security-hold; a named defect lands per the spec's discipline and
  re-enters verification.
  *Test:* the panel's typed per-member results and the bundle's
  aggregate; the exactness evidence of spec R5 cited or re-staged
  (AC-6b.2's citation half); security-hold is a hard stop. Proves
  AC-6b.6 and AC-6b.2.
- [ ] **5. Ship clean and drift-free** — `git status --porcelain`
  empty; the run's commits are exactly the spec trio plus the two
  repairs (plus any panel-forced repair) on top of `a9efa40`; the
  eight frozen required context names in `.github/workflows/ci.yml`
  byte-identical; `agents/`, `adapters/`, `bundles/` untouched by
  seats; no issue closed and no PR opened by seats; issue #32's
  delivery (`3e0a996`, `51bac94`, `ac8c326`) untouched.
  *Test:* the ship seat's checks — empty porcelain; grep of
  `.github/workflows/ci.yml` for the eight names;
  `git log --oneline a9efa40..HEAD` showing only the run's named
  commits; `git diff --stat a9efa40..HEAD -- agents adapters bundles`
  empty. Proves AC-6b.7, AC-6b.5 and the standing-gates clause (the
  six house gates green at the final HEAD, floor unchanged).
