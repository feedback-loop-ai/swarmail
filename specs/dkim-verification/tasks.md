# Tasks: offline DKIM verification (#254)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision:
[0125](../../docs/decisions/0125-offline-dkim-verification.md) (proposed; never
accept it).

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

## Phase S: review return 2 (in order; the open work)

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
