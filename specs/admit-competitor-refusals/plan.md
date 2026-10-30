# Plan — admit a collected run that carries a competitor's own refusals

Date: 2026-10-30. Implements [spec.md](spec.md). Parent family:
[source-mail-comparison](../source-mail-comparison/plan.md).

## Technical context

Rust, single root crate; the comparison driver is the `mail_compare` bench
(`benches/mail_compare.rs` + `benches/mail_compare/*.rs`), proved by
`tests/mail_compare.rs` against real servers and real TCP. serde_json 1.0.151
is already locked; the only change to the dependency graph is enabling its
dependency-free `float_roundtrip` feature. `.specify/templates/plan-template.md`
is absent; this plan supplies the dialect's sections itself.

## Constitution check

0001–0003 and 0005 are untouched: nothing in `src/store.rs`, `src/smtp.rs`,
the `await` path, pruning or extraction changes. 0020 placement: all tests are
public-API or bench-module tests in `tests/mail_compare.rs` (benches are not in
the production coverage denominator; no `*_tests.rs` harness, no seam, no
sleep-as-sync, no wall-clock assertion: the fixture window is a fixed integer).
0045 §8: no accepted ruling is contradicted; 0091–0093 stay proposed and
unedited. Dependencies: no crate, no license, `Cargo.lock` byte-identical.
0056: commits unsigned, never pushed.

## Research (settled by the chief)

- serde_json default float parsing is not round-trip safe; the
  `float_roundtrip` feature makes it exact (confirmed on the locked crate: 12 117
  of 100 000 report-formula rates misparse without it, 0 with it; fixtures
  `4 / 1 000 000 002 ns` and `1 / 1 000 000 002 ns` misparse).
- `admit_run` compares `to_value(report(...))` (never parsed) with
  `from_slice(report.json)` (parsed once), restricted to `throughput` and
  `waits`; the lifecycle it passes is empty by design and those keys do not
  depend on it.
- Mailpit's pinned binary has no documented connection-limit flag or `MP_*`
  variable; it carries a fixed `421 4.3.2 Too many connections` string and a
  `connection limit of %d reached` log line. Binary observation only; SMTP
  source not captured.
- `runner.rs` connect arm: `Smtp::open` → `Err((code, WireError))`; the row is
  `failed_before_terminator`, `code = (code > 0).then_some(code)`, reason
  `connect: {e.reason()}` (today `malformed` for a non-220 banner or non-250
  EHLO). Readiness needs a `220` banner, so a run-level fake that always says
  `421` cannot become ready; the mapping is therefore proved at the wire and
  row levels, and end to end by the real collection.
- `scripts/compare-mail.sh collect` runs the driver, then `admit` when the
  driver exit is ≤ 1, and returns the driver's status (1 for workload
  failures); the README documents exit 1 as a sealed run whose workload failed
  somewhere.

## Data model

Unchanged on disk. `Row` keeps `outcome`, `code`, `reason`; `ThroughputCell`
keeps `requested/unstarted/sessions/retention_cap/admitted/refused/exact` and
the two rates; `REPORT_VERSION` stays 2 because the bytes written do not
change. Only a reason string's wording and two refusal messages change.

## Contracts

- `wire::Smtp::open(addr, step) -> Result<Smtp, (u16, WireError)>` unchanged.
- New small pure helper in `runner.rs`, e.g.
  `pub fn connect_failure(r: &mut Row, code: u16, e: &WireError)`, used by the
  worker closure: sets `outcome = "failed_before_terminator"`,
  `code = (code > 0).then_some(code)`, `reason = "connect: refused"` when
  `code > 0`, else `connect: {e.reason()}`.
- `report::admit_run(dir, protocol_bytes)`: after the receipt check, refuse
  `on_disk["report_version"] != REPORT_VERSION` by name; on cell mismatch,
  error = `the report disagrees with the rows it was derived from at <path>:
  report <a>, rows <b>` from a small recursive `first_difference` helper.
- `scripts/compare-mail.sh`: unchanged.

## Project structure (files touched)

- `Cargo.toml` — serde_json feature line (one line).
- `benches/mail_compare/runner.rs` — `connect_failure` helper; worker uses it.
- `benches/mail_compare/report.rs` — version gate; `first_difference`; message.
- `tests/mail_compare.rs` — four tests (AC1, AC3, AC4 wire + row, AC7).
- `docs/evals/README.md` — § Method throughput paragraph (option a, why not b);
  dated "Collection (2026-10-30)" record with run id, verdict, exits, Mailpit
  refused counts per cell, index digest, digests checked; ledger rows updated;
  outstanding list reproduced as open.
- `specs/source-mail-comparison/tasks.md` — only the T4.b checkbox and its
  proof line may be updated once the admitted run exists; no other edit there.

## Quickstart (implementer)

```bash
sha256sum .brokkr/state/bin/mailpit .brokkr/state/bin/mailhog   # AC6, before anything launches
cargo test --locked --test mail_compare admission                 # AC1 red on 6fdf026, then green
git diff --exit-code Cargo.lock                                   # AC2: lock untouched
bash scripts/compare-mail.sh collect --output target/mail-compare/collect-$(date -u +%Y%m%dT%H%M%SZ) \
  --mailpit .brokkr/state/bin/mailpit --mailhog .brokkr/state/bin/mailhog   # timeoutMs 3600000; ~16 min
# the script runs `admit` itself; re-run it explicitly and record its exit:
"$(cargo bench --locked --offline --no-run --bench mail_compare --message-format=json 2>/dev/null \
  | grep -o '"executable":"[^"]*mail_compare[^"]*"' | tail -n1 | cut -d'"' -f4)" admit <dir> docs/evals/protocol.json; echo admit=$?
```

Then the full gate inventory (house rules § Gates) plus
`MAIL_COMPARE_MAILPIT=… MAIL_COMPARE_MAILHOG=… cargo test --locked --test mail_compare -- --ignored`.
Output and scratch stay under `target/` on `/home`, never on a RAM tmpfs.

## Risks

- **The feature changes the server's float parser too.** Mitigation: `src/`
  consumes no parsed f64 beyond `webhook.rs` `visit_f64`, which discards it;
  all gates run against the whole workspace; the lock diff is asserted empty.
  Accepted cost: slower float parsing in a path that parses none.
- **The diagnosis could be incomplete on the real run.** Mitigation: AC7's
  offender-naming message. If the real `admit` still refuses after AC2, the
  named path shows whether it is a float (diagnosis wrong: stop and report)
  or an integer (a second cause: report it, do not loosen). No run artifact
  is in the worktree, so the chief could not re-run the operator's refusal.
- **Sewn-run tests must mirror what the runner would write.** Mitigation: sew
  from a real `tiny()` run via the existing forge pattern; alter only
  `outcome/code/reason/ack_ns` of send rows and the paired visible rows'
  outcome; regenerate the report with `report::report` and the driver's
  serializer; keep the row count equal to the plan so `read_rows` admits.
- **A run-level `421` fake cannot become ready.** Mitigation: prove the
  mapping at the wire level (`Smtp::open` → `Err((421, _))` on real TCP) and
  the row level (`connect_failure`), and end to end on the real collection
  where Mailpit refuses. Not claimed: a scripted run-level 421.
- **`collect` exits 1 and a reader mistakes it for a broken feature.**
  Mitigation: AC5 states the expected exit; the README record and ledger say
  "sealed, admitted, workload failures recorded: Mailpit refused N of 1000 in
  reps …"; exit 2 alone is failure.
- **The one-hour command bound.** Collection took ~14–16 min; mitigation:
  one command for collect with `timeoutMs 3600000`; do not background it (a
  background process does not survive its command in this box); never resume
  a partial run; delete an interrupted run unread and say so.
- **Reviewer reads the version gate as the fix.** Mitigation: the spec says
  explicitly that skew was not the cause; the gate is a receipt, the feature
  is the fix, and AC1 is red without the feature even with the gate in place.
- **Claim creep.** Mitigation: the README ledger reproduces the outstanding
  list verbatim as open; no "equivalence", "repaired engine" or "complete"
  wording; the inherited HIGH residual row keeps its examined/not-examined
  shape and gains only a pointer to the committed 0081 note and engine pin.
