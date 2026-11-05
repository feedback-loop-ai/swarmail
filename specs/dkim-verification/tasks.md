# Tasks: offline DKIM verification (#254)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision:
[0125](../../docs/decisions/0125-offline-dkim-verification.md) (proposed; never
accept it).

Work in order. Each task names the test that proves it. A task is done only
when three things hold:
- that test runs;
- it has been seen to fail without the change, where that is possible;
- it passes with the change.

Commit unsigned, in the repository's style. Never push.

## Phase 0: preflight (the stop conditions)

- [ ] **T1. Dependency preflight.**
  - Add `mail-auth = { version = "=0.14.0", default-features = false, features = ["aws-lc-rs"] }`
    with its one-sentence reason, and resolve `Cargo.lock` offline.
  - Then run `cargo tree --locked -i native-tls` (it must be empty),
    `cargo deny check licenses`, and `git diff --exit-code deny.toml evals/Cargo.lock`.
  - Write the exact added-package list and licences into 0125 §9.
  - **Stop and report**, without widening further, in any of these cases:
    - a licence is non-permissive;
    - `deny.toml` would need a change the operator's one widening does not
      justify;
    - the crate does not build offline.

  *Test:* the commands above, with their outputs quoted in the commit body.
- [ ] **T2. Offline injection preflight.** Build the `MessageAuthenticator`
  from a hickory configuration with an empty name-server list inside a
  `LazyLock`. Then verify an ed25519 fixture through a cache adapter that
  answers from a hard-coded map.
  - **Stop and report** if the crate calls its resolver despite the cache.
  - Record the result in 0125 §9 if hickory refuses an empty list. The
    plan's Risks row says how to proceed.

  *Test:* inline `adapter_answers_without_resolver`. An undeclared name
  yields `PermError(RecordNotFound)`, not `TempError`.

## Phase 1: the verifier (`src/deliverability/dkim.rs`, inline tests)

- [ ] **T3. Test signing kit.**
  - Embed the RFC 8463 A.1 keys and one fixed 2,048-bit RSA test key.
  - Add `sign(body, opts)` over `DkimSigner`.
  - Add `sign_with(tags)` for explicit `t`/`x`/`l`/duplicate tags, as
    plan Research 6 describes.

  *Test:* `kit_signs_verifiably`: each kit output verifies through T2's
  adapter.
- [ ] **T4. Pre-scan and bounds.**
  - `PreScan` detects a duplicate tag name, a literal `l=0`, and the `h=`
    name count.
  - Add `MAX_DKIM_SIGNATURES` = 16 and `MAX_DKIM_SIGNED_HEADERS` = 128.
  - Past a bound: one `dkim.limits_exceeded.v1` at the first offending
    occurrence, no crate call, and no lookup.

  *Test:* `prescan_table`, covering duplicates, `l=0`, `l=00`, folded tags
  and empty tags. Also `bounds_sixteen_and_seventeen` and
  `bounds_h_128_and_129`, which assert no `dns.lookups` entries past a
  bound (AC10).
- [ ] **T5. The cache adapter over `Lookups`.**
  - `get` always returns `Some`.
  - An invalid owner name is remembered and never recorded.
  - The record type is chosen by name shape (`_atps`, `_report._domainkey.`,
    otherwise `DomainKey`).
  - `NoRecords` becomes `RecordNotFound`, and `TempFail` becomes
    `Resolver`.
  - `insert` is a no-op.
  - Remove the `expect(dead_code)` on `Txt`/`resolve_txt` and the
    `expect(unused_variables)` in `collect`.

  *Test:* `adapter_outcomes`, one case per outcome and name shape, asserting
  the recorded `dns.lookups` (AC9).
- [ ] **T6. `canonical_len`** for simple and relaxed (RFC 6376 §3.4.3/§3.4.4).
  *Test:* `canonical_len_matches_signer_l`, over the table of AC8 bodies, for
  both modes, against the `l=` that `DkimSigner::body_length(true)`
  writes.
- [ ] **T7. Verdict mapping.**
  - Implement rows a–u of the spec's evaluation table, one verdict per
    field.
  - Map outputs to fields by the crate's documented order, using
    Swarmail's own `Signature::parse`, with the signature-equality `expect`.
  - No unreachable `match` arm.
  - Add `enum Rule` with fixed ids, severities and RFC-citing hints.

  *Tests:*
  - `precedence_and_occurrences` (AC6): passing, malformed and failing
    signatures in one message, plus a missing-`bh=` field between parsed
    fields;
  - `row_coverage`: one fixture per row a–u;
  - exact branch coverage.
- [ ] **T8. One passing and one failing fixture for every rule** (AC5).
  - One test pair for each of the 17 rules in the spec's rule table.
  - `body_hash_mismatch` and `signature_mismatch` with both algorithms.
  - `atps_unauthorized` with and without the ATPS record declared.
  - `key_invalid` for both a garbage `p=` and a key type that does not
    match `a=`.

  *Test:* `rule_<id>_pass` / `rule_<id>_fail` for each rule. Each failing
  case asserts the exact id, severity and occurrence. Each passing case
  asserts that id is absent.
- [ ] **T9. Algorithms and canonicalization** (AC2, AC4).
  - rsa-sha256 (2,048 and 1,024 bits) and ed25519-sha256, each under the
    four `c=` pairs.
  - Whitespace mutations that fail under simple and pass under relaxed.

  *Test:* `algorithm_canonicalization_matrix` and
  `whitespace_mutation_by_mode`.
- [ ] **T10. Published vector** (AC3). Use the RFC 8463 Appendix A signed
  message, with the A.2 records declared. If it does not verify, record the
  evidence in 0125 §9 and pin the observed verdict.
  *Test:* `rfc8463_appendix_a`.
- [ ] **T11. Receipt time and the residual** (AC7).
  - Pass `receipt = max(received_ms, 0) / 1000` into `evaluate` and
    `collect`.
  - Expired means `x ≤ receipt`, and `x ≤ t` is malformed.
  - The residual maps to `dkim.expired_since_receipt.v1`.

  *Tests:*
  - `expiry_at_receipt_poles`: receipt `t` → clean; receipt `x` →
    `signature_expired`;
  - `expired_since_receipt_residual`: `t`=1,000, `x`=2,000, receipt 1,500;
  - `negative_received_ms_clamps`;
  - no sleep and no wall-clock assertion.
- [ ] **T12. `l=` rules** (AC8).
  *Test:* `l_equal_clean`, `l_partial_warning`, `l_exceeds_error`,
  `l_zero_unsupported`.

## Phase 2: integration

- [ ] **T13. Wire the collector.**
  - Declare `mod dkim;` and call `dkim::collect` after `spam::collect`.
  - Set `RULE_SET_ID` to `swarmail-html/6` and update `html-rules.json`.
  - Update every literal report, OpenAPI and MCP example, and fixture
    (`grep -rn 'swarmail-html/5'` must be empty outside dated history
    notes).
  - Make the evaluator's header comment honest about the crate clock.

  *Test:* the existing exact-JSON suites (`tests/deliverability.rs`,
  `tests/bulk_sender.rs`, `tests/spam_rules.rs`,
  `tests/mcp_revisions/structured.rs`) pass with only the id changed (AC1).
  Add inline `unsigned_report_has_no_lookups`.
- [ ] **T14. Determinism** (AC12).
  *Test:* inline `signed_report_byte_identical_twice`, plus the
  two-process comparison in `tests/deliverability.rs` extended with a
  signed message and a declared key.
- [ ] **T15. Wire proof** (AC11). Use real SMTP, `PUT /api/v1/inboxes/{inbox}/dns`,
  a REST report, and a REST await with `no_findings_at_or_above=warning`.
  *Test:* `tests/deliverability.rs::dkim_signed_mail_verifies_over_the_wire`,
  covering a pass, a body-altered failure, and the await returning only the
  pass.
- [ ] **T16. `tempfail` never clean** (AC9).
  *Test:* inline `dkim_tempfail_not_evaluated_never_qualifies`, asserting
  `qualifies` is false at info, warning and error.
- [ ] **T17. Refusal text** (AC14). In `src/config.rs`, `llms.txt` and the
  guide, the message becomes
  `real DNS is not supported; declare records with --dns-policy or PUT /api/v1/inboxes/{inbox}/dns`.
  *Test:* the existing `--dns-resolver real` / `SWARMAIL_DNS_RESOLVER=real`
  refusal tests, updated, still exit before bind.

## Phase 3: documentation and records

- [ ] **T18. Guide** (AC15). `docs/guide/deliverability.md` gains a "DKIM
  verification" section covering:
  - declaring keys;
  - the 17 rules, with severity and hint;
  - receipt time and the residual;
  - the bounds;
  - known limits: the `_report` selector, and the unreachable resolver;
  - the `swarmail-html/6` history entry.

  *Test:* inline `guide_lists_every_dkim_rule`, which binds the guide's rule
  list to `Rule`.
- [ ] **T19. Decision record.**
  - Complete 0125 §9 with the real lockfile list and licences, and the
    preflight evidence.
  - Add the 0125 row to `docs/decisions/README.md` as
    `proposed — awaiting the operator`.
  - Keep the status `proposed`.

  *Test:* review, plus `design-check`-style status grep:
  `grep -c '^- \*\*Status:\*\* proposed' docs/decisions/0125-*.md` = 1.

## Phase 4: gates

- [ ] **T20. Every house gate, `--locked`** (AC13, AC16).
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo test --workspace --locked`
  - `cargo build --locked --bin swarmail`
  - the evals crate's fmt, clippy and tests against that binary
  - `bash scripts/coverage-gate.sh` at 100% exact, and its `selftest`
  - `bash scripts/publish-guard-test.sh`
  - `bash scripts/seal-branch-test.sh`
  - `cargo deny check licenses`
  - the `native-tls` absence proof
  - the existing real-HTTPS webhook and STARTTLS suites, green with both
    rustls providers compiled
  - the brokkr bundle compile, if brokkr is present; otherwise state that
    verify runs it

  If Docker is available, also build the image and report the result. The
  docker-smoke, macOS and RustSec jobs are CI-owned, and the seat says so.
  *Test:* the gate outputs, quoted in the implementation summary.
