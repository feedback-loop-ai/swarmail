# Ordered tasks — admit a collected run that carries a competitor's own refusals

Implements [spec.md](spec.md) per [plan.md](plan.md). Each task names the test
that proves it. Order matters: digests first, red test before the fix, the
real collection after every gate is green, the record last.

- [x] **T0 — Digests before anything launches (AC6).** `sha256sum
  .brokkr/state/bin/mailpit .brokkr/state/bin/mailhog` equal the spec's values
  and `docs/evals/sources.lock.json`; MailHog recorded as locally observed only.
  Proof: the two lines pasted into the README collection record and the
  seat's notes.
- [x] **T1 — Red test: rates that do not round-trip (AC1).** Add
  `admission_survives_rates_that_do_not_round_trip` to `tests/mail_compare.rs`:
  run `tiny()` for real, sew a sealed copy (forge pattern of
  `admission_recomputes_the_plan_the_receipts_and_the_report`) whose Verified
  row has `start_ns = 0`, `ack_ns = 1_000_000_002`, regenerate `report.json`
  with `report::report` + `serde_json::to_vec_pretty`, and assert
  `report::admit_run` admits. Also assert the crate property on
  `json!({"r": 4.0 * 1e9 / 1_000_000_002.0})`. Proof: the test fails on
  6fdf026 with "disagrees with the rows" (paste the failure), then passes
  after T2.
- [x] **T2 — Enable `float_roundtrip` (AC2).** `Cargo.toml`:
  `serde_json = { version = "1", features = ["float_roundtrip"] }`. Proof: T1
  green; `git diff --exit-code Cargo.lock` empty; `cargo deny check licenses`
  unchanged; `cargo tree -e features -i serde_json --locked` lists
  `float_roundtrip`; the lying case in the existing admission test still
  refuses.
- [x] **T3 — Name the refusal (AC7).** In `report::admit_run`, refuse
  `report_version != REPORT_VERSION` by name before the cell comparison; add
  `first_difference(&Value, &Value) -> Option<(String, Value, Value)>` and
  put the path and both values into the "disagrees with the rows" message.
  Proof: extend the existing admission test: `report_version = 999` refuses
  with a message naming 999 and 2 and "re-collect"; the lying case's message
  contains `throughput[0].reconciliation.accepted`, `999` and `4`.
- [x] **T4 — Refusal row wording (AC4).** Add `runner::connect_failure` and
  route the worker's connect arm through it: `code > 0` → reason
  `connect: refused`; otherwise `connect: <wire reason>`. Proof:
  `connect_refusal_is_a_coded_failed_before_terminator_row` asserts
  `(421, Malformed)` → `failed_before_terminator`, `Some(421)`,
  `"connect: refused"`, and `(0, Timeout)` → `None`, `"connect: deadline"` (the wire reason).
- [x] **T5 — Wire proof of a `421` greeting (AC4).** A tokio `TcpListener`
  that writes `421 4.3.2 Too many connections, please try again later\r\n`
  and closes; `wire::Smtp::open` returns `Err((421, _))`. Proof:
  `a_421_greeting_is_a_coded_connect_refusal` in `tests/mail_compare.rs`
  (real TCP, no mock of the protocol).
- [x] **T6 — Refused and ambiguous rows reconcile and admit (AC3).** Sew from
  a real `tiny()` run: one send row → `failed_before_terminator`/`Some(421)`/
  `"connect: refused"`/`ack_ns = None`, one send row → `ambiguous`; their
  visible rows → outcome `unexpected`; regenerate the report; assert
  `admit_run` admits, the cell has `refused == 1`, `exact == false`, and
  `assert_cells_match_rows` holds (`requested = admitted + refused +
  ambiguous + unstarted`). Proof: `a_run_with_competitor_refusals_admits`.
- [x] **T7 — Gates green before launch (AC8).** `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  `cargo test --workspace --locked`; `cargo test --locked --test mail_compare`
  plus its ignored real-binary tests with `MAIL_COMPARE_MAILPIT`/`MAILHOG`;
  `brokkr compile --bundle .brokkr/recipes/self` with the digest-checked
  engine; `bash scripts/coverage-gate.sh` and `selftest` (exact 100/100/100);
  `bash scripts/publish-guard-test.sh`; `bash scripts/seal-branch-test.sh`;
  `cargo deny check licenses`. Proof: each command's exit pasted in the
  seat notes; no gate skipped.
- [x] **T8 — The real frozen collection and its admission (AC5).** One
  command, `timeoutMs 3600000`: `scripts/compare-mail.sh collect --output
  target/mail-compare/collect-<utc> --mailpit .brokkr/state/bin/mailpit
  --mailhog .brokkr/state/bin/mailhog`; then the driver's `admit <dir>
  docs/evals/protocol.json` explicitly. Proof: collect exits 0 or 1 (never 2),
  `admit` exits 0, `index.json` exists; the per-cell Mailpit
  `refused`/`exact` values and the swarmail cells' `exact == true` read from
  `report.json`. If it fails, report the actual output and stop at this
  task; never resume or re-label a partial run.
- [x] **T9 — Record it honestly (AC4, AC5, AC8).** `docs/evals/README.md`:
  § Method throughput paragraph (a competitor's own SMTP refusal is a
  measured `failed_before_terminator` row with its code, counted in
  `refused`, makes the cell inexact, is never retried or tuned away; no
  documented Mailpit flag or `MP_*` variable governs it in the pinned
  release, binary observation only, SMTP source not captured, so option (b)
  was not taken); a dated "Collection (2026-10-30)" record with run id,
  commit, verdict, collect and admit exits, index digest, Mailpit refused
  counts per repetition, digests checked (MailHog locally observed only);
  ledger rows for this run; the outstanding list reproduced as open. Tick
  T4.b in `specs/source-mail-comparison/tasks.md` with its proof line only.
  Proof: `git diff --stat` shows only the planned files; no run artifacts
  committed; `grep -n "equivalen\|repaired the pinned\|complete" docs/evals/README.md`
  reviewed line by line for claim creep.
- [x] **T10 — Commit unsigned, never push (AC8).** Imperative subject, body
  says why (the float parser, the refusal wording, the real run's outcome).
  Proof: `git log -1 --format='%G? %s'` shows `N`; `git status` clean;
  nothing pushed.
