# Tasks: offline DKIM verification (#254)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision:
[0125](../../docs/decisions/0125-offline-dkim-verification.md) (proposed; never
accept it).

**Revision 3, 2026-11-05.** The review returned the run to design a third
time. Phases T, R and S are delivered (S in `8241489`, verify green). **Phase U
is the open work, in order.** Each U task answers the findings named in
[plan § Revision 3](plan.md#revision-3-what-changes-and-which-finding-it-answers).

**Revision 2, 2026-11-05.** The review returned the run to design a second
time. The T tasks were delivered in `adeaa12` and the R tasks in `38fd111`
(verify passed both). Both are checked off below, with their text corrected to
the mechanism actually built. **Phase S is the open work, in order.** Each S
task answers the review findings named in
[plan § Revision 2](plan.md#revision-2-what-changes-and-which-finding-it-answers).

**Standing rule (spec header):** if implementing a task shows a mechanism in
spec.md to be wrong, correct spec.md (and this file and plan.md) in the same
commit, beside any dated record in 0125. A correction that lives only in 0125
is the defect this return was about.

A task is done only when all of these hold:
- its test runs;
- the test has been seen to fail without the change, where that is possible;
- the test passes with the change.

Commit unsigned, in the repository's style. Never push.

## Delivered in `adeaa12` (corrected record)

- [x] **T1. Dependency preflight.** The exact pin and its reason, the lockfile,
  `native-tls` absent, `cargo deny check licenses` passes, and `deny.toml` and
  `evals/Cargo.lock` are unchanged. *Test:* the gate outputs (0125 §9).
- [x] **T2. Offline injection.** A `LazyLock` authenticator with no name
  servers, and an adapter that never misses. *Test:* `adapter_outcomes`. An
  undeclared name gives `RecordNotFound`, not `TempError`.
- [x] **T3. Signing kit.** RFC 8463 keys, a 2,048-bit RSA key, and signing with
  explicit tags. *Test:* `forged_signatures_verify_for_both_algorithms` and the
  kit tests.
- [x] **T4. Pre-scan and bounds.**
  - *(Corrected.)* Bounds are taken from the crate's strict parse, not from a
    Swarmail field scan.
  - `Scan` = duplicate, `has_v`, `has_l`, `h_names`.

  *Test:* the bounds tests at 16/17 and 128/129.
- [x] **T5. The cache adapter.**
  - *(Corrected.)* `name_of` reads the key's `Hash`, and there is no `invalid`
    set.
  - The record type is chosen by name shape.
  - The `expect(dead_code)` and `expect(unused_variables)` attributes are gone.

  *Test:* `adapter_outcomes`.
- [x] **T6. Canonicalizer.**
  - *(Corrected.)* The oracle is the canonicalizer against the crate's own
    `bh=`, over 55,987 bodies in both modes. The first design's "against the
    signer's `l=`" could not hold.

  *Test:* the oracle test.
- [x] **T7. Verdict mapping.**
  - *(Corrected.)* Fields come from the crate's `dkim_signatures()` and
    filtered `errors()`, sorted by position.
  - Pairing was `first_parsed + index`. **R4 replaces it.**
  - Rows q and r collapse per 0125 item 3.

  *Test:* the precedence and row tests.
- [x] **T8. One passing and one failing fixture for each of the 17 rules.**
  *Test:* the `rule_*` pairs.
- [x] **T9. Algorithms and canonicalization.** *Test:* the c= matrix and
  whitespace-mutation tests.
- [x] **T10. RFC 8463 Appendix A.** It verifies as published, including the
  1,024-bit RSA path. *Test:* `rfc8463_appendix_a`.
- [x] **T11. Receipt time and the residual.** *Test:* the pole and residual
  fixtures, and the clamp tests.
- [x] **T12. `l=` rules (raw-byte semantics).** *Test:* the `l_*` tests.
- [x] **T13. Wire the collector; `swarmail-html/6` everywhere.** *Test:* the
  exact-JSON suites.
- [x] **T14. Determinism.**
  - Inline: two evaluations of the same `Email`.
  - Two in-process servers in `tests/deliverability.rs`.

  *Test:* both.
- [x] **T15. Wire proof.**
  - *(Corrected.)* The REST report asserts the DKIM ids.
  - The await clause is moved to #255 (AC11).

  *Test:* `tests/deliverability.rs`.
- [x] **T16. `tempfail` never clean.** *Test:* the inline qualifies-at-every-threshold test.
- [x] **T17. Refusal text.** *Test:* the refusal tests.
- [x] **T18. Guide section (17 rules).** **R7 extends it to 18.** *Test:* the
  guide-binding test.
- [x] **T19. 0125 §9 and the README row.** *Test:* review.
- [x] **T20. Gates.** *Test:* verify's pass at `adeaa12`.

## Phase R: review return 1 (delivered in `38fd111`; corrected record)

- [x] **R1. Record the clock hold honestly (G1).**
  - Change only the evidence and the code comment. The chief has already
    redrafted 0125 §8 and its header in the design commit, so do not edit that
    text.
  - Rewrite the header comment in `src/deliverability.rs` (lines ~3–7) to the
    radius in spec § Verification time. Remove "can only move verified to not
    verified".
  - Add the AC7 radius fixture: the residual signature (`t` 1,000, `x` 2,000,
    receipt 1,500 s) with an **undeclared** key gives `expired_since_receipt`,
    not `key_missing`, and `dns.lookups` holds no key name.

  *Test:* `dkim::tests::expired_since_receipt_hides_key_and_lookup`.

  Accepting 0125 §8 is not part of R1, or of any task: only the operator
  accepts it.
- [x] **R2. One body boundary (S; G12).**
  - Pass `fields` and `size` from `collect` into `dkim::collect`.
  - Add `Rule::HeaderBoundaryAmbiguous` (`dkim.header_boundary_ambiguous.v1`,
    error, plus its hint).
  - Implement spec steps 3 and 4 before the bounds, in the order plan detail 1
    gives.

  *Tests:*
  - `boundary_agrees_for_crlf_lf_and_empty_body`: no boundary finding, normal
    verdicts;
  - *(Corrected: the whitespace-line fixtures did not trigger; mail-auth folds
    an inside-block whitespace line.)* `boundary_stray_line_after_signature`
    and `boundary_stray_line_before_signature`: exactly one finding at
    occurrence 0, no other `dkim.*`, empty `dns.lookups`;
  - `boundary_whitespace_line_inside_the_block_is_no_difference`: agreeing
    offsets, normal verdicts;
  - `boundary_strict_parse_none_while_signed`;
  - `unsigned_whitespace_line_has_no_dkim_finding` (AC1, AC17).
- [x] **R3. Pre-scan what the crate defaults (G3; I info on `l=abc`).**
  - `Scan` gains `has_a`, `has_t`, `has_x` and `numbers_ok`.
  - Rows a, d, f and g change as in plan detail 2.
  - The `malformed` and `unsupported` hints are corrected.

  *Tests:*
  - `missing_a_is_malformed`: a valid rsa-sha256 signature with no `a=`, which
    verifies without the check, is malformed; the same signature with `a=` is
    clean;
  - `non_numeric_tags_are_malformed`: `t=abc`, `x=abc`, `l=abc`, and
    `x=99999999999999999999`;
  - `folded_numeric_tag_is_clean`;
  - `x_zero_without_t_is_expired` and `x_zero_with_t_is_malformed` (AC18).
- [x] **R4. Pair outputs by identity (C4; I residual).**
  - `Field.parsed` becomes `Result<&Signature, &Error>`.
  - Pick the output by `ptr::eq` on `DkimOutput::signature()`, with the fixed
    `expect`.
  - Delete `first_parsed` and the `errors()` count.

  *Tests:*
  - `arc_and_rejected_fields_do_not_shift_pairing`: a passing signature beside
    malformed `ARC-Seal` and `ARC-Message-Signature` fields and a missing-`bh=`
    DKIM-Signature keeps its own verdict and occurrence;
  - the existing precedence tests stay green;
  - `grep -n first_parsed src/deliverability/dkim.rs` is empty (AC19).
- [x] **R5. Judge the name the crate asked (G11).**
  - Add `owner_of(name)`. *(Corrected: it is `OwnerName::parse` alone, which
    already strips one trailing dot; a second strip would accept `..`.)*
  - The adapter's `get` uses it, and so does row l, on `signature.domain_key()`.
  - The `format!` reconstruction is deleted.

  *Test:* `trailing_dot_domain_is_selector_invalid`: `d=example.com.` gives
  `selector_invalid` and no lookup. The `s=bad*sel` case stays (AC9).
- [x] **R6. Wire proof on both surfaces (G6, T1).**
  - Make sure the over-the-wire test reads the MCP report as well as the REST
    one. Add the MCP read if it is missing.
  - Assert no `dkim.*` finding plus the recorded key lookup for the signed
    message, and `dkim.body_hash_mismatch.v1` for the altered one.
  - Do not add an await assertion. It belongs to #255 (AC11).

  *Test:* `tests/deliverability.rs::dkim_signed_mail_verifies_over_the_wire`
  (or the existing name), over real SMTP, REST and MCP.
- [x] **R7. Live-surface and guide sweep (G8, G13, C2; G4 text).**
  - Edit every surface in spec § Live surfaces:
    - `openapi.json`: `DnsRecord`, the route and `DeliverabilityReport`. The
      route's "no network or ambient time input" names the 0125 §8 exception;
    - `llms.txt` ~564;
    - the guide: ~98, ~186, the overview `l=` sentence, the malformed list
      (`v=`, `a=`, the numeric tags), `key_invalid`/`signature_mismatch`
      wording, `atps_unauthorized` covering junk records, and a new heading for
      the boundary rule;
    - `src/mcp.rs` `MODERN_REPORT_DESCRIPTION`, which adds the DKIM family.
  - The guide's known limits gain the `l=` interop gap, with the #255 note, and
    the radius.
  - Update any snapshot that pins the old text.

  *Tests:*
  - the guide-binding test, now 18 headings;
  - `grep -rn -e "No rule resolves a name" -e "until #254" src/static docs/guide docs/decisions/README.md`
    finds nothing outside dated history;
  - the MCP and OpenAPI snapshot tests (AC15).
- [x] **R8. Resolver proofs (G2).**
  - Preflight: call `AUTHENTICATOR.verify_dkim(Parameters::new(&msg))` with
    **no** cache on a signed fixture, under `now_or_never()`.
  - If it completes on the first poll with a `TempError` or `PermError`
    output, keep it as a test.
  - If it does not complete on the first poll, write no timed test. Record in
    0125 §9's evidence that the empty-name-server proof is by construction, and
    that `adapter_outcomes` is the tested proof.

  *Test:* `resolver_without_nameservers_answers_nothing` (conditional), plus
  the existing `adapter_outcomes`.
- [x] **R9. Rule table at 18 (AC5, AC6).**
  - Add a passing and a failing fixture for `header_boundary_ambiguous` (from
    R2).
  - Add the new failing fixtures the spec's rule table lists for `malformed`,
    `signature_expired`, `selector_invalid` and `atps_unauthorized` (junk ATPS
    value).
  - Extend the row-coverage test to the message-level steps 3, 4 and 5.

  *Test:* `rule_*` pairs for all 18 rules, plus exact branch coverage.
- [x] **R10. Records (G9, G14, C5).**
  - Add a dated 2026-11-05 integration note to proposed 0120 covering §4/§5,
    §7 and §10 (spec AC21). Do not touch accepted 0085.
  - Correct the 0120 and 0125 rows in `docs/decisions/README.md`.
  - In `deny.toml`, change the CDLA-Permissive-2.0 **comment only**, and the
    matching sentence in `docs/house-rules.md` § Dependencies. Both name
    `webpki-roots` and `webpki-root-certs`.

  *Tests:*
  - `git diff fad9e7e -- docs/decisions/0085-deliverability-report.md` is
    empty;
  - `git diff fad9e7e -- deny.toml` touches comment lines only;
  - `cargo deny check licenses` passes (AC13, AC20, AC21).
- [x] **R11. Docker evidence (G10).**
  - If a usable Docker daemon exists, run one bounded image build, as in plan
    § Risks. Cap it at nproc/4 CPUs and 10 minutes, with a named, timed
    command, and report the load applied.
  - If it fails for a missing tool, add exactly that `apk` package to the
    Dockerfile.
  - If no daemon exists, write in the implementation summary that CI's
    `docker smoke` is the CI-owned proof.

  *Test:* the bounded build's output, or the named CI check.
- [x] **R12. Every house gate, `--locked` (AC16).**
  - fmt, clippy and tests;
  - `cargo build --locked --bin swarmail`;
  - the evals fmt, clippy and tests against that binary;
  - `bash scripts/coverage-gate.sh` at 100% exact, and its `selftest`;
  - `bash scripts/publish-guard-test.sh`;
  - `bash scripts/seal-branch-test.sh`;
  - `cargo deny check licenses`;
  - the `native-tls` absence proof;
  - the existing real-HTTPS webhook and STARTTLS suites;
  - the brokkr bundle compile if brokkr is present, otherwise state that verify
    runs it.

  The PR body lists the three operator holds of spec § Operator holds.
  *Test:* the gate outputs, quoted in the implementation summary.

## Phase S: review return 2 (delivered in `8241489`)

Order matters: S1's premise test runs before S2's guard change is trusted.

- [x] **S1. Prove the premise first (C3, I3; AC22).**
  - Add a table test: for each spelling of a field that Swarmail's
    `headers()` names `dkim-signature` (`DKIM-Signature`, `dkim-signature`,
    `DKIM-SIGNATURE`, `dKiM-sIgNaTuRe`; a kit-signed value, a value missing
    `bh=`, an empty value, a value folded across lines), the crate's
    `fields()` of the strict parse holds exactly the fields that the root
    list holds, in the same order. Assert the counts per case.
  - If any case fails, **stop and report** to the run. Do not weaken the
    guard, and do not change the shared `headers()` scanner.

  *Test:* `dkim::tests::crate_lists_every_root_dkim_field`.
- [x] **S2. Count conjunct in the boundary guard (C3, I3, I4; G3a/G3e text;
  AC17, AC22).**
  - In `collect`, compute
    `root_count = root.iter().filter(|(n, _)| is_dkim_signature(n)).count()`
    and require `first.len() == root_count` beside
    `body_offset() == size` (plan detail 1). A mismatch is the existing
    `Rule::HeaderBoundaryAmbiguous.finding(0)`: no verification, no lookup.
  - Widen the `header_boundary_ambiguous` hint to name both conditions the
    code tests (plan detail 5).
  - Rewrite the module comment ("One body.") and the `collect` comment
    ("a whitespace-only line ends the crate's headers and continues
    Swarmail's field") to the real triggers and the field-count condition.
  - The existing boundary, limits and precedence tests stay green unchanged.

  *Tests:*
  - `boundary_dkim_field_name_with_space_is_ambiguous`: a kit-signed valid
    message plus a `DKIM-Signature : v=1; …` field, before it and (in a second
    case) after it, gives exactly
    `one("dkim.header_boundary_ambiguous.v1", 0, ERROR)` and empty
    `dns.lookups`. Seen to fail (a finding at occurrence 1, or a pass) before
    the conjunct;
  - `rule_header_boundary_ambiguous` gains that failing fixture and keeps the
    inside-block whitespace passing fixture;
  - `boundary_whitespace_line_inside_the_block_is_no_difference` stays.
- [x] **S3. Occurrence agreement (AC22).**
  - A message with two kit-signed fields and a parser-rejected
    (missing-`bh=`) field between them, with the second signature's body
    altered so it fails: assert each finding's occurrence equals its index
    among `headers(raw).0` entries named `dkim-signature` (compute the
    expected index from the root list in the test, not by hand).

  *Test:* `dkim::tests::finding_occurrence_is_the_root_occurrence`.
- [x] **S4. Same-type wrong key, both algorithms (C4, I6; AC5).**
  - In `rule_signature_mismatch`, delete the block commented "A signature
    made with another key is the same verdict", whose `other_key` and
    `wrong_key` bind the correct keys and assert passes. Keep the
    `IncompatibleAlgorithms` assertion (`key_invalid`) under an honest comment.
  - Add: the kit's ed25519 signature with RFC 8463's published `brisbane`
    ed25519 `p=` declared at `sel._domainkey.example.com` gives exactly
    `one("dkim.signature_mismatch.v1", 0, ERROR)` and one `records` lookup;
    the kit's RSA-2048 signature with RFC 8463's published 1,024-bit `test`
    RSA `p=` declared there gives the same. Share the two published key
    strings with `rfc8463_appendix_a_verifies_both_signatures` as test
    constants. No new key material.

  *Test:* `dkim::tests::rule_signature_mismatch`.
- [x] **S5. Pin the measured lookups (G3d, G6; AC9).**
  - `x_zero_with_t_is_malformed` also asserts that `dns.lookups` is exactly
    the one `sel._domainkey.example.com` `records` lookup.
  - Confirm, and add where missing, exact empty-lookup assertions for an
    unsigned message, each boundary refusal, a bounds refusal and the row-h
    residual (`expired_since_receipt_hides_key_and_lookup`).
  - `failure_reports_ask_for_the_report_name` stays as the `r=y` pin.

  *Tests:* `x_zero_with_t_is_malformed`, plus the listed existing tests.
- [x] **S6. No calendar expiry in a passing fixture (AC23).**
  - In `forged_signatures_verify_for_both_algorithms`, replace
    `"t=1000; x=9000000000"` and receipt 1500 with an `x` derived from
    `parsed(&base()).t` (as `folded_numeric_tag_is_clean` does) and a receipt
    before it. Review every other `forged(` call that expects a pass.

  *Test:* `forged_signatures_verify_for_both_algorithms`; `grep -n "x=9000000000" src/deliverability/dkim.rs`
  empty.
- [x] **S7. Live surfaces and comments (G4, G5, G6, C5, I5; AC15, AC22).**
  - `docs/guide/deliverability.md` ~180–183 and `src/static/llms.txt`
    ~645–648: replace "no DNS/authentication verification, external spam
    scorer, external service or new dependency ships here" with the claim in
    spec § Live surfaces (#253 shipped no verification; DKIM arrives with
    proposed 0125 on mail-auth, offline; SPF/DMARC/ARC are #255/#256; no
    external scorer or service).
  - `src/mcp.rs` `MODERN_REPORT_DESCRIPTION`: one finding per field that does
    not verify, a `body_length_partial` warning on a field that verifies only
    a prefix, or one message-level finding.
  - Guide and llms.txt: occurrence k is the root `dkim-signature` occurrence;
    the boundary rule lists the field-name trigger. Touch `openapi.json` only
    if a sentence contradicts the spec.
  - `src/deliverability/dns.rs` module comment: bytes, receipt time and
    configuration, except proposed 0125 §8.
  - `Cargo.toml` rustls comment (no `rustls/ring everywhere`); `deny.toml`
    CDLA-Permissive-2.0 **comment only** (`webpki-root-certs` only through
    hickory's unreachable platform verifier).
  - Update any snapshot that pins the old text; never keep a false one.

  *Tests:*
  - `grep -rn -e "No rule resolves a name" -e "until #254" -e "no DNS/authentication verification" -e "one finding per signature that does not verify" src/static src/mcp.rs docs/guide docs/decisions/README.md`
    finds nothing outside dated history;
  - `grep -n "and the configuration (decision 0085)" src/deliverability/dns.rs`
    and `grep -n "rustls/ring everywhere" Cargo.toml` are empty;
  - `grep -c "root DKIM-Signature" docs/guide/deliverability.md src/static/llms.txt`
    finds at least one line in each (the occurrence sentence: k is the
    zero-based occurrence among the message's root DKIM-Signature fields);
  - `guide_lists_every_dkim_rule` (18 headings) and the MCP/OpenAPI snapshot
    tests;
  - `git diff fad9e7e -- deny.toml` touches comment lines only.
- [x] **S8. Records and gates (AC16, AC20; standing rule).**
  - Append a dated "Implementation of return revision 2" evidence entry to
    0125: the premise result, the conjunct, the test names. Edit no ruling
    text in 0125, and if anything in spec.md proves wrong, correct spec.md in
    the same commit (standing rule).
  - `git diff fad9e7e -- docs/decisions/0085-deliverability-report.md` empty;
    `Cargo.lock`, `evals/Cargo.lock` and the `deny.toml` allowlist unchanged
    since `38fd111`.
  - Run every house gate `--locked`, as R12 lists: fmt, clippy, tests, the
    binary build, the evals fmt/clippy/tests against that binary, the
    coverage gate at exact 100% and its selftest, publish-guard,
    seal-branch, `cargo deny check licenses`, the `native-tls` absence proof,
    and the brokkr bundle compile if brokkr is present (otherwise say verify
    runs it). No load proof is needed; if one is run, cap it at nproc/4
    workers for 10 minutes with a named, timed command.
  - The PR body lists the three operator holds (spec § Operator holds), and
    that #255's intake inherits AC11's await clause.

  *Test:* the gate outputs, quoted in the implementation summary.

## Phase U: review return 3 (in order; the open work)

U1 runs first, because U2's text depends on the variants it measures.

- [ ] **U1. Make the report sample unreachable (S spec defect; AC24).**
  - In `src/deliverability/dkim.rs` `record()`, delete the
    `name.starts_with("_report._domainkey.")` arm, and remove
    `DkimReportRecord` from the `mail_auth` import (plan § Revision 3,
    detail 1). Rewrite `record()`'s doc comment: two record types, chosen by
    name; a report record is never built, because the crate's sampling read is
    reached only through one; an `_atps`-shaped name is still read as ATPS. The
    comment no longer says a selector "literally named `_report`" fails closed.
  - `adapter_answers_without_resolver`: replace the `TxtRecord::DkimReport(_)`
    assertion, and the `broken.example` one, with the exact variants that
    `DomainKey::parse_records` gives for `v=DKIM1; ra=postmaster` and for
    `nonsense`. Measure both first (detail 3). Do not name `DkimReport`.
  - `failure_reports_ask_for_the_report_name`: add a leg declaring
    `_report._domainkey.example.com` as `v=DKIM1; ra=postmaster; rp=0`. Assert
    exactly `one("dkim.body_hash_mismatch.v1", 0, ERROR)` and exactly the one
    `("_report._domainkey.example.com", Outcome::Records)` lookup, equal to the
    `rp=100` leg. Then assert that two evaluations of that message give
    byte-identical report JSON. Rewrite the comment "Declared, it is read as a
    report record" to say the answer is never typed as a report record.
  - New `report_selector_is_a_key`: a kit signature with
    `Sign { selector: "_report", ..Sign::default() }` and its key declared at
    `_report._domainkey.example.com` gives no `dkim.*` finding and exactly one
    `records` lookup for that name.

  *Tests:* `adapter_answers_without_resolver`,
  `failure_reports_ask_for_the_report_name`, `report_selector_is_a_key`;
  `grep -rn "DkimReport" src/` empty. `report_selector_is_a_key` should be
  seen to fail (`dkim.key_invalid.v1`) before the arm is deleted.
- [ ] **U2. Locate the boundary finding at `null` when the root holds no
  DKIM-Signature field (I1 spec defect; AC17, AC22, AC25).**
  - Add `Rule::located(self, occurrence: Option<u64>) -> Finding`, holding
    today's `finding` body. `finding(usize)` calls
    `self.located(Some(occurrence as u64))`. In `collect`'s guard `else` arm,
    push `Rule::HeaderBoundaryAmbiguous.located((root_count > 0).then_some(0))`
    (detail 2). No other call site changes.
  - Update `collect`'s comment to name the `null` case.
  - New `boundary_lone_spaced_field_is_ambiguous_at_null`: a kit-signed
    message whose only DKIM field is renamed `DKIM-Signature :` (a space before
    the colon). Assert that its `dkim.*` findings are exactly one
    `dkim.header_boundary_ambiguous.v1`, `ERROR`, at
    `Location::Header { name: "dkim-signature", occurrence: None }`, and that
    `dns.lookups` is empty. Add the same case to
    `rule_header_boundary_ambiguous`'s failing fixtures. Seen to fail
    (`Some(0)`) before the change.
  - Every existing boundary fixture keeps occurrence 0 unchanged.

  *Tests:* `boundary_lone_spaced_field_is_ambiguous_at_null`,
  `rule_header_boundary_ambiguous`,
  `boundary_dkim_field_name_with_space_is_ambiguous` (unchanged), and exact
  branch coverage of both `then_some` arms.
- [ ] **U3. Surface the `null` location (I1; AC15, AC25).**
  - `docs/guide/deliverability.md` § `dkim.header_boundary_ambiguous.v1`
    (~l.507) and its overview (~l.440 and ~l.480): the finding is on
    occurrence 0, or `null` when the message has no root DKIM-Signature field.
  - `src/static/llms.txt` (~l.670): the same clause. The boundary sentence also
    names the field-name trigger (a space before the colon).
  - `src/static/openapi.json`, the deliverability route description: after
    "or one message-level finding (…)", say the boundary finding is at
    occurrence 0, or `null` when the message has no root DKIM-Signature field.
  - Update any snapshot or test that pins the old OpenAPI or llms.txt text
    (for example a description hash). Never keep a false one.

  *Tests:* `grep -c "no root DKIM-Signature field" docs/guide/deliverability.md src/static/llms.txt src/static/openapi.json`
  is at least 1 in each file; the OpenAPI/llms snapshot tests pass.
- [ ] **U4. Evidence that fails on drift (C2, C3, C4, T; AC22, AC26).**
  - `crate_lists_every_root_dkim_field`: hoist `unfold`. In the combined
    message, make sure the empty-value case sits between two parseable
    cases, and replace the two count assertions with list equality by
    unfolded value (detail 5).
  - New `strict_and_full_parses_list_the_same_fields`: a message with an `l=`
    kit signature, a non-`l=` kit signature and a missing-`bh=` field. Assert
    equal lengths and equal raw offsets, position by position, between
    `fields()` of `parse_with_opts(raw, None, true)` and of
    `parse_with_opts(raw, None, false)` (detail 6). No `debug_assert!` in
    production code.
  - `tests/deliverability.rs`, the `nokey` leg: replace
    `dkim_findings(&report_none)[0]["rule_id"]` with a full-vector equality on
    the one `dkim.key_missing.v1` finding (rule id, severity, location
    occurrence 0, fix hint). The `tempfail` leg: the same for
    `dkim.not_evaluated.v1`, and replace
    `report_temp["dns"]["lookups"][0]["outcome"]` with full equality on the
    lookups array (`[{"name":DKIM_KEY_NAME,"type":"txt","outcome":"temp_fail","values":[]}]`,
    or whatever exact shape the live server returns; measure and pin it).
  - The same file's `over_mcp["dns"]["lookups"][0]["name"]` read (~l.1727)
    becomes full equality on the lookups array, like the legs above.
  - The altered-body leg's `assert_eq!(report_bad["dns"]["lookups"], json!([]))`
    gains a comment: this pins the crate's measured ask order on the pinned
    version, an upgrade tripwire, not a lookup contract (spec § Report and
    version).

  *Tests:* `crate_lists_every_root_dkim_field`,
  `strict_and_full_parses_list_the_same_fields`, and
  `cargo test --locked --test deliverability`; `grep -n '\[0\]\["rule_id"\]\|lookups"\]\[0\]' tests/deliverability.rs`
  is empty.
- [ ] **U5. The clock account and comments (S; AC15, AC24).**
  - `docs/guide/deliverability.md`, the clock-residual paragraph: after "One
    read of the host clock remains", add one sentence. The crate's
    RFC 6651 report sampling also reads the clock, but Swarmail never hands
    it a report record, so that read is never reached. Add the await warning:
    a test that awaits a clean report on a signature whose `x=` is near the
    host clock can start to fail without new mail, so keep fixture `x=` far
    from the wall clock or derive it from the receipt.
  - The guide's Known limits: delete the `_report` selector clause and keep
    the `_atps` one ("a name with an `_atps` label and no `_domainkey` one is
    read as an ATPS record and fails closed as `dkim.key_invalid.v1`").
  - `src/deliverability.rs` header comment: "the one reachable clock read"
    wording, if the comment says the crate reads the clock once without that
    qualifier. Keep it short.
  - `Cargo.toml` mail-auth comment: "`aws-lc-rs` compiles hickory in; it is
    unreachable because the adapter answers every name and the authenticator
    has no name servers. Exact pin."

  *Tests:* `grep -n "literally named" docs/guide/deliverability.md src/deliverability/dkim.rs`
  empty; `guide_lists_every_dkim_rule` passes; review.
- [ ] **U6. The 0121 integration note (G3; AC21).**
  - Append to `docs/decisions/0121-deliverability-link-checks.md` a section
    `## Integration note — 2026-11-05 (proposed 0125, issue #254)`. It
    says that §7's "free of network, clock and resolver access" holds for the
    link probe and the static link rules. Under proposed 0125 §8 the report
    evaluator now holds one reachable crate-internal clock read (mail-auth's
    `x=` comparison, operator hold 2), the crate's report-sampling read is
    unreachable, and a hickory resolver is compiled in and unreachable
    (hold 3). It also says the note changes no status, the record stays
    proposed, and §7's text is not edited.
  - Do not edit 0121 §7 or any other ruling text, and do not edit accepted
    0085.

  *Tests:* `grep -n "Integration note — 2026-11-05 (proposed 0125" docs/decisions/0121-deliverability-link-checks.md`
  finds one line; `git diff fad9e7e -- docs/decisions/0085-deliverability-report.md`
  is empty.
- [ ] **U7. Records, PR body and gates (AC16, AC20; standing rule).**
  - Append a dated "Implementation of return revision 3" evidence entry to
    0125: the measured `_report` variants, the `rp=0` leg, the
    `s=_report` verdict before and after, the `null` fixture, and the test
    names. Edit no ruling text. If any mechanism in spec.md proves wrong,
    correct spec.md, plan.md and this file in the same commit (standing rule).
  - `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are unchanged since
    `8241489`. `RULE_SET_ID` stays `swarmail-html/6`.
  - The PR body lists the three operator holds as **one merge question**
    (spec § Operator holds): hold 2 now covers exactly one reachable clock
    read. It also says that #255's intake inherits AC11's await clause.
  - Run every house gate `--locked`: fmt, clippy, tests, the binary build,
    the evals fmt/clippy/tests against that binary, the coverage gate at exact
    100% and its selftest, publish-guard, seal-branch,
    `cargo deny check licenses`, the `native-tls` absence proof, and the
    brokkr bundle compile if brokkr is present (otherwise say that verify
    runs it). No load proof is needed. If one is run, cap it at nproc/4
    workers for 10 minutes with a named, timed command, and report the load.

  *Test:* the gate outputs, quoted in the implementation summary, and
  `grep -rn "DkimReport" src/` empty.
