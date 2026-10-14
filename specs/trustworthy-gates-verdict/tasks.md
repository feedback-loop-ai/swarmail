# Tasks — run 6c: the verdict, earned at the final tree

Ordered. Each task is paired with the test or gate that proves it (real protocol against
real servers, never mocks — 0004). The repair lands first so the tree the verify seat
proves is the tree that ships; the verify seat is the arbiter; review is the protected
phase; ship signs only on a clean, drift-free tree. Any edit beyond task 1 requires a red
gate or a review-named defect and lands as its own minimal named commit with its test
(spec R1/R5).

- [ ] **1. Land the panel-named shutdown-comment repair (spec R2)** — in `src/smtp.rs`,
  rewrite the `JoinSet` drain parenthetical in `serve` to the stage-precise policy: a
  session that has not crossed the DATA terminator has no commit to land (the closed
  socket is the only goodbye it gets); a session parked at the insert's await lands its
  commit — `spawn_blocking` is not cancelled by the abort — stored but unacknowledged,
  the client's retry a duplicate, never a loss (0001's honest asymmetry), and the parked
  in-flight guard releases before the orphaned commit resolves. Own commit, named,
  citing spec R2; no other `src/` change; no decision document.
  *Test:* `cargo test --workspace --locked` green — in particular
  `tests/lifecycle.rs::an_in_flight_commit_neither_holds_stop_nor_half_lands`, which
  already asserts on the wire that an in-flight commit is not cancelled by the stop and
  retry is lossless; `cargo fmt --all -- --check` and `cargo clippy --workspace
  --all-targets --locked -- -D warnings` clean; the exact gate's counts unchanged (task
  2 is the detector). Proves AC-6c.3 and AC-6c.4.
- [ ] **2. Run the full hermetic verify seat (the parked work)** — `bash
  bundles/self/scripts/verify-seat.sh` to exit 0 within the 10800 s budget at the run's
  final HEAD: fmt; clippy `--workspace --all-targets --locked -- -D warnings`; full
  `cargo test --workspace --locked`; `brokkr compile --bundle bundles/self`; the exact
  coverage gate; the gate's selftest; `scripts/publish-guard-test.sh`; `cargo deny
  check licenses`; the fail-closed native-tls probe. No gate skipped, no verdict
  retried.
  *Test:* the seat's own result record (pass, with the notes line naming all nine
  gates); the exact-gate summary at the final HEAD reading **5827/320/729** — matching
  the operator's `run.BUoDkq` proves the comment repair moved nothing measurable (spec
  R3's detector; any drift stops the run); the captured output showing
  `tests/binary.rs` finish in minutes with every serve-spawning test completing proves
  AC-6c.2. Proves AC-6c.1 and AC-6c.2.
- [ ] **3. Review the delivered batch (protected phase)** — the four members judge the
  tree against the four children's acceptance criteria (issues #50–#53 of #58:
  C50.1–C50.5, C51.1–C51.6, C52.1–C52.5, C53.1–C53.5), the parent spec
  (`specs/trustworthy-gates-tests-delivery/`, R1–R15 + ACs), and accepted decision
  0026 — locating by shape, since the issues cite baseline `ac8c326`. The correctness
  member reads the deltas named in spec R6 before voting: the labeled gauge registries
  and drop-guards in `src/api.rs`, the park wiring in `src/mcp.rs`/`src/smtp.rs`, the
  pre-bind SIGINT watcher in `src/main.rs`, and the ANSI-conditional subscriber. Each
  member types clean | residual | security-hold; a named defect lands per the spec's
  discipline and re-enters verification (the full seat again if `src/` changed).
  *Test:* the panel's typed per-member results and the bundle's aggregate; the counts
  evidence cited (the seat's summary beside `run.BUoDkq`); security-hold is a hard
  stop. Proves AC-6c.5.
- [ ] **4. Ship clean and drift-free** — `git status --porcelain` empty; the run's
  commits are exactly the spec trio plus the repair (plus any panel-forced repair) on
  top of `7c2c446`; the eight frozen required context names in
  `.github/workflows/ci.yml` byte-identical; `agents/`, `adapters/`, `bundles/`
  untouched by seats; no issue closed and no PR opened by seats; issue #32's delivery
  (`3e0a996`, `51bac94`, `ac8c326`) untouched and not reopened.
  *Test:* the ship seat's checks — empty porcelain; grep of `.github/workflows/ci.yml`
  for the eight names; `git log --oneline 7c2c446..HEAD` showing only the run's named
  commits; `git diff --stat 7c2c446..HEAD -- agents adapters bundles` empty. Proves
  AC-6c.6 and AC-6c.4, and the standing-gates clause (the six house gates green at the
  final HEAD, floor unchanged).
