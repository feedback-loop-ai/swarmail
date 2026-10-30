# Admit a collected run that carries a competitor's own refusals

Date: 2026-10-30. Status: draft (chief's synthesis). Epic #164 / issue #163,
operator continuation of 2026-10-29. Base: `6fdf026a58a1e55dec422ad95cc7b448fcaa57da`
(main's #274–#280 stack merged; engine `swarmail-engine-a9893b4-p4`).

No new decision is declared by this spec. The work is confined to the
comparison harness (`benches/mail_compare/`, `tests/mail_compare.rs`,
`docs/evals/README.md`, one `Cargo.toml` feature line). It touches no frozen
guarantee (0001–0003), no accepted ruling, and no public contract of the
server. Proposed [0091](../../docs/decisions/0091-fair-mail-comparison.md),
[0092](../../docs/decisions/0092-official-mail-comparison.md) and
[0093](../../docs/decisions/0093-source-mail-comparison.md) are referenced
as the comparison's contracts and stay proposed, unedited and unrenumbered.
The parent spec family
[source-mail-comparison](../source-mail-comparison/spec.md) and everything it
carries forward remain normative; this spec adds the two blockers' resolution
and does not rewrite any historical spec, ledger or receipt.

## What and why

With the operator's three fixes committed on this branch (512 MiB artifact
bound, `--max-sessions 1000`, 4096 listen backlog) the frozen collection
completes and swarmail is exact in every stress repetition. Two things still
stop admission of the collected run, and this feature resolves both:

1. **Mailpit answers `421` to a few `stress-c1000-fresh` connections** in 4
   of 6 repetitions. Today that is already a measured row
   (`failed_before_terminator`, `code: 421`) counted in the cell's `refused`,
   and the run is sealed under `Verdict::WorkloadFailures` (exit 1). The only
   defect is that the row's `reason` reads `connect: malformed`: the peer sent
   a well-formed refusal, not a malformed reply.
2. **`mail_compare admit` refuses with "the report disagrees with the rows it
   was derived from"** (`benches/mail_compare/report.rs:315`). The root cause
   is not the workload failures. `admit_run` compares a `serde_json::Value`
   built in memory with one parsed from `report.json`, and serde_json's
   default float parser (feature `float_roundtrip` off; `de.rs` double
   rounding) does not round-trip every f64. The chief reproduced this with
   the locked crate (serde_json 1.0.151, offline probe, deleted): of 100 000
   rates computed with the report's own formula `count as f64 * 1e9 /
   window_ns as f64`, 12 117 did not round-trip; the fixtures `4 / 1 000 000 002 ns`
   (`3.9999999920000002`) and `1 / 1 000 000 002 ns` (`0.9999999980000001`)
   both misparse; with `float_roundtrip` enabled, zero of the same 100 000
   failed. A full collection carries two rates per throughput cell across
   every target, profile and repetition, so it trips the check regardless of
   workload failures; the tiny protocol in the existing admission test passes
   by luck.

### Rulings between the panel positions

The two positions were read in full; the chief rules rather than averages.

- **Root cause of the admit refusal: simplicity's diagnosis is adopted and
  was independently confirmed by the chief.** Robustness's version-skew
  theory is rejected as the cause: the run and the admission used the same
  driver and `REPORT_VERSION` has been 2 throughout, so no skew existed.
  Robustness's two cheap receipts are adopted anyway because they cost
  almost nothing and sharpen the refusal: `admit` refuses a `report_version`
  other than its own by name, and a cell mismatch names the first differing
  JSON path with both values. Not adopted: a driver digest in the report,
  deserialising lifecycle facts to pass the recorded lifecycle into the
  re-derivation (throughput and waits do not depend on the lifecycle, and the
  comparison is restricted to those two keys by design), streaming
  admission, and the `missing_accepted` clamp. Those are outside the two
  blockers and would add schema or code with no failing case behind them.
- **The fix is the dependency feature, not a bench-local workaround.**
  `serde_json = { version = "1", features = ["float_roundtrip"] }` makes
  `parse(shortest(x)) == x` for every finite f64, so the existing exact
  comparison becomes what its comment promises. The fallback (re-parse the
  fresh report's text so both sides go through the same lossy parser) is
  refused: it leaves a theoretical 1-ULP blind spot where two neighbouring
  values parse equal, which is a loosening in all but name. Cost, named: the
  feature unifies across the root crate, so the server's JSON parser also
  switches to the exact (slower) float algorithm. The only f64 consumer in
  `src/` is `webhook.rs` `visit_f64`, which discards the value; hot paths
  parse no floats. The feature is `float_roundtrip = []`: no crate, no
  license, and `Cargo.lock` is byte-identical (features are not recorded in
  the lock). The evals workspace is separate and unaffected.
- **Mailpit's `421` stays measured data under the existing `refused`
  category (intake's AC4 option a).** Robustness's relabel to `rejected` is
  rejected: `rejected` means the server refused the message at the
  terminator after it was offered; a `421` at the greeting is literally a
  session that failed before the terminator, and a relabel would change how
  the frozen protocol counts, which intake ruled needs its own proposed
  decision. Robustness's "publish `admitted/sessions` beside every rate" is
  already true: each cell carries `requested`, `unstarted`, `sessions`,
  `admitted` and `refused`. Simplicity's no-retry, no-throttle, no-new-field,
  no-new-verdict cuts are adopted.
- **Intake's AC4 option (b), a documented Mailpit setting, is unavailable and
  refused.** Chief's own observation of the digest-checked binary
  (`262cfa28…8edec8`, equal to `sources.lock.json`): `--help` lists no
  connection or session limit (`--max`, `--max-age`, `--max-message-size`,
  `--smtp-max-recipients`, `--webhook-limit` only); no `MP_*` environment
  name in the binary mentions a connection limit; the binary carries the
  fixed strings `421 4.3.2 Too many connections, please try again later` and
  `[%s] connection limit of %d reached, new connections are being refused or
  delayed`. That is binary observation only: the SMTP server source is not in
  the source capture, so the cap's value and semantics are unsourced, and
  applying any undocumented change would be an invented fairness change.
  Robustness's conditions for a declared profile (D2) are recorded as the bar
  any future setting must clear; none is applied here.
- **`collect` keeps exiting 1 on a run with workload failures; `admit` exits
  0.** The script already admits after an exit-1 driver run and then returns
  1. Making `collect` exit 0 by exempting a competitor's refusal from the
  exact-count verdict would change the frozen protocol's counting; it is not
  done. The verify seat reports the actual exits.
- **Intake's AC3 identity is corrected**, not loosened: the full identity is
  `requested = admitted + refused + ambiguous + unstarted` with
  `attempted = accepted + rejected + failed_before_terminator + ambiguous`;
  `ambiguous` is not dropped to match the AC's shorter wording. The existing
  `assert_cells_match_rows` helper already checks this and is reused.
- **Red-first.** Intake's AC1 ties the red test to workload failures; the
  confirmed root cause is the float round-trip, so the red test is a sewn run
  whose rate hits a non-round-tripping value. A second test proves that a
  run with refused and ambiguous rows admits with the full identity. The real
  frozen collection (AC5) is the end-to-end proof of both.

## Scope

In: the reason text of a connect-time refusal row; one `Cargo.toml` feature;
`admit_run`'s version gate and offender-naming diff; four tests in
`tests/mail_compare.rs`; one README paragraph plus a dated collection
record and ledger rows; one real `collect` and `admit` run inside the seat.

Out, carried as enumerated open gaps exactly as intake scoped them: #148/#201
eval coordination, #151 paid baseline and final MCP contracts, operator
acceptance of 0091–0093, CI, the five-member protected review, the inherited
HIGH cross-attempt review-integrity residual (carried, not reproduced, not
widened, not claimed repaired by the re-pin beyond what the committed 0081
note records), source-review equivalence, T7–T12, best-wait, binary build
provenance. No change to `src/store.rs`, `src/smtp.rs`, `await`, pruning,
extraction or `tests/{burst,p2,rate}.rs`.

## Acceptance Criteria

- **AC1 (red first, float round-trip).** A test in `tests/mail_compare.rs`
  sews a sealed run from a real `tiny()` run whose throughput cell has 4
  accepted sends and a verified send window of exactly 1 000 000 002 ns
  (rate `3.9999999920000002`), writes the report with the driver's own
  serializer, and asserts `report::admit_run` admits it. On 6fdf026 the test
  fails with "disagrees with the rows"; with the feature it passes. The same
  test asserts the crate property it relies on: `to_vec` then `from_slice`
  of `json!({"r": rate})` equals the original for that fixture.
- **AC2 (root cause fixed, nothing loosened).** `Cargo.toml` enables
  `float_roundtrip` on serde_json. `git diff --exit-code Cargo.lock` is empty
  and every `--locked` gate passes. `deny.toml` is unchanged. The comparison
  in `admit_run` still compares `throughput` and `waits` exactly: the
  existing lying case (`reconciliation.accepted = 999`) still refuses.
- **AC3 (refusals reconcile).** A sewn run derived from a real run, with one
  send row `failed_before_terminator` / `code 421` / `reason "connect:
  refused"` and one `ambiguous` send row (their visible rows relabelled
  `unexpected`, as the runner would), admits through `admit_run`; its cell
  reports `refused == 1`, `exact == false`, and
  `requested == admitted + refused + ambiguous + unstarted` with
  `attempted == accepted + rejected + failed_before_terminator + ambiguous`
  (via `assert_cells_match_rows`). No row is dropped, retried or relabelled
  by production code.
- **AC4 (recording choice, option a).** A connect-time refusal with a reply
  code produces a `Send` row with `outcome failed_before_terminator`,
  `code Some(code)` and `reason "connect: refused"`; a connect failure
  without a code keeps `connect: <wire reason>` and `code None`. Proved by a
  real-TCP test of `wire::Smtp::open` against a listener that answers
  `421 4.3.2 Too many connections, please try again later` (result
  `Err((421, _))`) and by a direct test of the row mapping. The choice and
  why option (b) is unavailable are written into `docs/evals/README.md`
  (§ Method, throughput); no new decision number, field, outcome, verdict,
  retry or per-target throttle.
- **AC5 (the real run).** Inside the implement seat, with `timeoutMs`
  3600000, `scripts/compare-mail.sh collect` runs to completion on the
  frozen protocol with the digest-checked executables, and
  `mail_compare admit <dir> docs/evals/protocol.json` exits 0 on its output.
  `collect` is expected to exit 1 (`WorkloadFailures`) whenever Mailpit
  refuses; exit 2 is a failure of this feature. The run id, verdict, admit
  exit, every Mailpit cell's `refused` count and the index digest are
  recorded in the README's dated collection record. Run artifacts are not
  committed. If collect or admit fails, the actual output is reported and the
  ledger says so.
- **AC6 (digests before launch).** `sha256sum .brokkr/state/bin/mailpit` equals
  `262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8` and
  `.brokkr/state/bin/mailhog` equals
  `e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7`, checked
  before collect; MailHog's digest is recorded as locally observed only.
- **AC7 (admission names its refusal).** `admit_run` refuses a report whose
  `report_version` is not `REPORT_VERSION` with a message naming both
  versions and "re-collect with the admitting driver", before any cell
  comparison; a cell mismatch names the first differing JSON path and both
  values while still containing "disagrees with the rows".
- **AC8 (gates and boundaries).** Every gate in house rules § Gates is clean
  with `--locked`, coverage exact 100/100/100 with its selftest, publish-guard
  and seal-branch proofs, licenses, pinned brokkr compile, and the ignored
  real-binary `mail_compare` tests with the digest-checked executables. No
  dependency crate is added, no allowlist widened, no decision accepted,
  renumbered or rewritten, nothing pushed or published, commits unsigned.
  The outstanding list in § Scope is reproduced in the README ledger as open,
  never inferred complete.
