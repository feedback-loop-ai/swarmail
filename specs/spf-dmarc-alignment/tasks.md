# Tasks: SPF and DMARC alignment (issue #255)

Ordered. Each task names the test that proves it; a task is done only when
its proof runs green. Spec: [spec.md](spec.md); plan: [plan.md](plan.md).

## Phase 0 — confirm the ground

- [ ] T0. Re-read `mail-auth-0.14.0` `spf/verify.rs`, `spf/output.rs`,
  `spf/macros.rs`, `sampling.rs`, `dmarc/verify.rs` and `dns/lookup.rs`
  against 0126 §8's inventory; confirm no feature or crate is needed. Any
  new reachable clock, resolver, random or environment read stops the run
  and is reported.
  *Proof:* the inventory in the 0126 implementation record matches;
  `git diff --exit-code c95386e -- Cargo.toml Cargo.lock evals/Cargo.lock deny.toml` (AC18;
  the base is the merge-base since the v0.4.0 merge, see T34).

## Phase 1 — the envelope observation (AC1–AC4)

- [ ] T1. Add `smtp_client_ip: Option<IpAddr>` and `smtp_helo: Option<String>`
  to `Email`, `#[serde(skip)]`, documented as 0126 §2; give every `Email`
  literal in `src/` and `tests/` `None`.
  *Proof:* existing payload pin tests unchanged; an inline `model.rs` test
  that serializing an `Email` with both set yields the same bytes as with
  both `None` (AC4).
- [ ] T2. Pass the accepted peer into `session`; record `to_canonical()` of
  it; keep the last non-empty `HELO`/`EHLO` argument; clear it on
  `STARTTLS`; set both fields after `build_email` and before `insert_async`.
  *Proof:* inline `smtp.rs` tests over a loopback listener: EHLO name
  stored, HELO replaced by a later EHLO, no greeting → `None`, empty
  argument → `None`, STARTTLS without re-greeting → `None`, a forged
  `Received:`/`X-Forwarded-For:` header changes nothing; an IPv4-mapped
  address canonicalizes (AC1).
- [ ] T3. Confirm REST seed and the MailSlurper shim leave both `None`.
  *Proof:* inline tests in `api.rs`/`compat.rs` (AC1).
- [ ] T4. Add `ADDED_COLUMNS` `smtp_client_ip TEXT`, `smtp_helo TEXT`; write
  and restore them; refuse a stored address that does not parse.
  *Proof:* inline `persist.rs` tests: round trip; an old-schema data file
  restores `None` and keeps every row; a corrupt address refuses; plus
  `tests/persistence.rs` restart over real SMTP (AC2).
- [ ] T5. Leave `Store::insert`, await and pruning untouched.
  *Proof:* the diff shows no change there; `tests/burst.rs`, `tests/p2.rs`,
  `tests/rate.rs` green (AC3).

## Phase 2 — the declared client address (AC5)

- [ ] T6. Add `spf_client_ip` to `dns::Policy`, the one parser, canonical
  form and `PolicyView` (present only when declared); refuse a non-IP value.
  *Proof:* inline `dns.rs` tests: v4, v6, v4-mapped canonicalization,
  refusal text, byte bound, absent member keeps the existing view bytes.
- [ ] T7. Thread the member through `PUT /api/v1/inboxes/{inbox}/dns`, the
  server file check before bind, OpenAPI and MCP schemas.
  *Proof:* `tests/deliverability.rs`: PUT accepts and echoes it, 400 on a bad
  value; `tests/binary.rs` (or the existing server-file test) refuses a bad
  file before bind; the OpenAPI/MCP schema tests updated.
- [ ] T8. Add the pure existence rule on `Policy` (a name exists when it or a
  descendant is declared, TXT or `tempfail`).
  *Proof:* inline `dns.rs` table test.

## Phase 3 — evaluation (AC6–AC12, AC15)

- [ ] T9. Refactor `dkim::collect` to return the parsed message, the crate
  outputs and #254's verdict per output (and whether #254 stopped), without
  changing its findings.
  *Proof:* every existing `dkim.rs` test green, unchanged apart from the
  version; a new inline test that the retained outputs are exactly the pass,
  `dkim.body_length_partial.v1` and `dkim.not_evaluated.v1` fields.
- [ ] T10. Create `src/deliverability/auth.rs` with the SPF adapter: TXT via
  `owner_of`, `v=spf1` selection (none / one / many), `ra` cleared, tempfail
  as resolver error; A/AAAA/MX/PTR negative with the unsupported mark.
  *Proof:* inline tests that call each cache's `get` directly and assert
  `Some` (AC15); record-selection fixture pairs (AC7); a record with
  `ra=x rp=0` has no `ra` after the adapter (AC15).
- [ ] T11. SPF evaluation: the identity table, `HOST` constant,
  `now_or_never().expect(..)`, the verdict mapping and the eight `spf.*`
  rules with static hints at `return-path`/null.
  *Proof:* inline fixture pairs for every `spf.*` id, MAIL FROM vs HELO
  identity, `spf_client_ip` override, a row without a peer stays
  `spf.not_evaluated.v1` even with the override (AC5, AC6).
- [ ] T12. Unsupported terms and limits.
  *Proof:* inline: `a`, `mx`, `ptr`, `exists`, `%{p}` each give
  `spf.unsupported_mechanism.v1` with `spf_pass` Unknown;
  `ip4:127.0.0.1 a -all` passes; ten includes pass, eleven permerror;
  redirect as the eleventh permerror; undeclared include permerror;
  `a a a -all` unsupported; `exp=` with `%{t}` recorded, never expanded;
  at most twelve recorded names (AC8, AC9).
- [ ] T13. DMARC adapter: `v=DMARC1` selection, `DmarcRecord` parsing,
  tempfail, the A cache's existence rule, negative AAAA/MX/PTR.
  *Proof:* inline `get` tests for every cache (AC15); two-record and
  unparseable fixtures (AC10).
- [ ] T14. From validity: Swarmail's root From count, the crate parse and
  boundary agreement, empty crate domain.
  *Proof:* inline fixture pairs for `dmarc.from_invalid.v1` (none, two
  fields, two domains) and `dmarc.not_evaluated.v1` (boundary disagreement);
  a From-less message whose envelope domain would pass stays
  `dmarc.from_invalid.v1` (AC12).
- [ ] T15. One `verify_dmarc` call with the filtered DKIM outputs and the
  identity-alignment SPF slot; map discovery and result to the `dmarc.*`
  ids with the applied disposition.
  *Proof:* inline fixture pairs for `dmarc.none.v1`, `dmarc.temperror.v1`,
  `dmarc.permerror.v1`, `dmarc.fail.none.v1`, `dmarc.fail.quarantine.v1`,
  `dmarc.fail.reject.v1`, `dmarc.not_evaluated.v1`; `p=` vs `sp=` vs `np=`;
  `t=y`; `psd=y`; `pct=50` changes nothing; a ten-label From records at most
  eight walk names (AC9, AC10).
- [ ] T16. Alignment cases.
  *Proof:* inline: relaxed/strict SPF and DKIM with a subdomain identity;
  unverified `d=` equal to From does not align; a crate-verified signature
  #254 refuses (unsigned `From`, `rsa-sha1`) does not align; aligned MAIL
  FROM with a rejecting record is aligned, not authenticated (AC11).

## Phase 4 — facts and the checklist (AC13, AC16, AC17)

- [ ] T17. Compute the six facts per the facts table; delete `policy_id` and
  `observation_id`; make `Inputs.auth` a `#[cfg(test)] Option` override;
  reorder `collect` so `bulk` reads the facts.
  *Proof:* inline: each fact's True/False/Unknown rows; the 0089 truth
  tables through the override; the cache-identity test varies policy and
  mail instead of the deleted strings; equal keys give equal facts (AC13).
- [ ] T18. Set `RULE_SET_ID` to `swarmail-html/7`; update every pinned
  `/6` fixture.
  *Proof:* shape tests assert exactly `rule_set_id`, `findings`, `dns` and
  `type` `txt` only (AC17).
- [ ] T19. Determinism.
  *Proof:* inline byte-identical double evaluation; the two-server test in
  `tests/deliverability.rs` extended with SPF and DMARC (AC16).

## Phase 5 — over the wire (AC1, AC5, AC13, AC14)

- [ ] T20. `tests/deliverability.rs`: per-inbox policies over real SMTP,
  REST and MCP: SPF pass and fail; DMARC pass through SPF only and through
  DKIM only; `spf_client_ip` deciding SPF; a pre-#255-shaped row (seeded)
  clean through DKIM alone.
  *Proof:* the suite itself.
- [ ] T21. The inherited await clause (0125 AC11): a signed, SPF-passing,
  DMARC-passing message satisfying the whole checklist is returned by an
  await with `no_findings_at_or_above=warning`; the altered copy is not and
  keeps `dkim.body_hash_mismatch.v1`, on REST and MCP.
  *Proof:* `tests/deliverability.rs` (AC14).
- [ ] T22. `tests/bulk_sender.rs`: replace "always not evaluated" production
  expectations with the real evidence, keeping the spoofed-header proofs
  (headers never supply evidence).
  *Proof:* the suite itself (AC13).

## Phase 6 — records, docs and gates (AC19, AC20)

- [ ] T23. Docs: `docs/guide/deliverability.md`, `src/static/llms.txt`,
  OpenAPI descriptions: `spf_client_ip`, the rule table, the A-R table, the
  unsupported mechanisms, the timer residual.
  *Proof:* existing doc-consistency tests (`tests/agent_setup_docs.rs` and
  peers) green.
- [ ] T24. Records: add 0126 to `docs/decisions/README.md` as proposed; dated
  integration notes on 0120 (the member, non-TXT answers) and 0125 (DKIM
  verdicts feed DMARC); an implementation record in 0126. Do not change any
  decision's status and do not add an 0085 pointer (that follows acceptance).
  *Proof:* review; `tests/` link checks if present.
- [ ] T25. Run every house gate `--locked` and record the log: fmt, clippy,
  tests, `cargo build --locked --bin swarmail`, the evals fmt/clippy/tests
  against that binary, `scripts/coverage-gate.sh` (exactly 100%) and its
  `selftest`, `scripts/publish-guard-test.sh`, `scripts/seal-branch-test.sh`,
  `cargo deny check licenses`, and the brokkr bundle compile where a pinned
  binary exists (otherwise say it runs in verify).
  *Proof:* the gate log (AC18, AC19).
- [ ] T26. Commit unsigned in the repository's style; never push, merge or
  accept a decision. Report the operator holds (0126 header: 1a/1b/1c, 2, 3)
  in the result.
  *Proof:* `git log --format='%G? %s'` shows `N`; the PR is held under
  0045 §8.

## Phase 7 — design return, review 1 (AC10, AC18, AC20–AC24)

T0–T26 were delivered at `6653277` and verify passed them there. This phase
works on top of that tree. 0126's header and the spec were corrected in the
design commit. Each task below answers a review finding (plan.md, "Design
return"). Do not edit 0126's header again except to fix a factual error, and
change no decision's status.

- [ ] T27. Records: rewrite the 0126 row in `docs/decisions/README.md` to
  "held for the operator on three holds: (1) an amendment of accepted 0120
  §2/§3/§4 (`spf_client_ip`; unrecorded non-TXT answers; NXDOMAIN/NODATA for
  `np=`), (2) SPF's monotonic timer, an amendment of accepted 0085 §8 and a
  second exception to accepted 0120 §5, (3) a reading of accepted 0089 §5",
  and "integration notes on accepted 0120 and 0125". In 0120's dated
  integration note (2026-11-06, proposed 0126), relabel "§3, the declaration
  grammar" as **§2**. Split "§4, non-TXT answers" into a §3 item (the `np=`
  answer and SPF's fixed negatives are answered and not recorded) and a §4
  item (NXDOMAIN/NODATA distinguished for `np=`), and point each item at
  0126's hold 1 (1a/1b/1c) and the §5 paragraph at hold 2.
  *Proof:* `grep '^| \[0126\]' docs/decisions/README.md | grep -c 'one item\|proposed 0120'`
  is 0; `grep -n '§2, the declaration'
  docs/decisions/0120-deliverability-dns-policy.md` matches;
  `git diff <the design-return commit> -- docs/decisions | grep -i '^[-+].*\*\*Status'`
  is empty (AC22).
- [ ] T28. Dated pointers, no status change: add "Dated pointer — 2026-11-06
  (proposed 0126 §8, hold 2)" to `0085-deliverability-report.md`, in the form
  of its existing 0125 pointer, and "Dated pointer — 2026-11-06 (proposed 0126
  §6, hold 3)" to `0089-bulk-sender-checklist.md`. Each is at most five lines,
  states that the item waits for the operator under 0045 §8, and links 0126.
  *Proof:* `grep -n '0126' docs/decisions/0085-*.md docs/decisions/0089-*.md`
  shows both; each record keeps exactly one status line (AC22).
- [ ] T29. `src/static/openapi.json`: rewrite the `DeliverabilityReport`
  description's "authentication unknown … no production message currently
  qualifies at any predicate threshold" to say the authentication entries
  read facts computed from the stored mail, its envelope observation and the
  policy, and that a message can qualify. Rewrite `DnsRecord`'s "SPF and DMARC
  will record theirs when they land" to say DKIM, SPF and DMARC record the
  TXT names they resolve. Touch no status wording (#337).
  *Proof:* `grep -c 'no production message currently qualifies\|will record
  theirs when they land' src/static/openapi.json` is 0; every OpenAPI pin or
  doc-consistency test (`tests/agent_setup_docs.rs` and peers, the MCP and
  OpenAPI schema tests) passes, updated in the same commit if it pins these
  bytes (AC24).
- [ ] T30. The ambient-read wording: in `src/deliverability.rs`'s module
  comment ("one stated exception"), `src/deliverability/dns.rs` ("the one
  clock read of proposed decision 0125 section 8"),
  `docs/guide/deliverability.md` (the "one stated exception" sentences near
  the byte-identity text and the cache-eviction text) and
  `src/static/llms.txt` (the same sentence), name both reads: 0125 §8's
  `x=` comparison and 0126 §8's SPF `LookupLimit` timer. Doc comments and
  prose only.
  *Proof:* `grep -rn 'one stated exception\|the one clock read' src docs/guide`
  returns nothing; the guide and llms.txt consistency tests pass; the coverage
  gate is unchanged (AC24).
- [ ] T31. DMARC version grammar in `Kind::is_ours` (`src/deliverability/auth.rs`):
  for DMARC, accept `v`/`V`, then optional space or tab, `=`, optional space
  or tab, then case-exact `DMARC1`, followed by space, tab, `;` or the end.
  SPF is unchanged (`v=spf1`, case-insensitive, followed by a space or the
  end). Update the doc comment to cite RFC 7489 §6.4.
  *Proof:* inline fixture pairs: `v = DMARC1; p=reject`, `v=DMARC1 ;p=reject`
  and `v=DMARC1\t;p=reject` apply `p=reject`; `v=dmarc1; p=reject`,
  `v=DMARC10; p=reject` and `vv=DMARC1; p=reject` are not records
  (`dmarc.none.v1` when alone); the existing `v=spf10` and `v=spf1` cases stay
  green (AC21).
- [ ] T32. DMARC multiplicity in `Kind::select`: for `Kind::Dmarc`, two or more
  values that pass `is_ours` return `not_found()` (no record at that name, so
  the crate's walk continues). SPF keeps `TxtRecord::Error(Error::Parse)`.
  Move "exactly one v=DMARC1 record per _dmarc name" from the
  `dmarc.permerror.v1` hint to the `dmarc.none.v1` hint, keeping both hints
  free of characters that need escaping, and update the guide's rule table to
  match. Move the fixture at the old two-record permerror case to the none
  pair. List every test or fixture in `src/` and `tests/` that asserts
  `dmarc.permerror.v1` and change only the ones that use two values.
  *Proof:* inline: two values alone give `dmarc.none.v1` and
  `policy_present` False; two values under `_dmarc.mail.example.com` with
  `_dmarc.example.com` = `v=DMARC1; p=reject; sp=quarantine` give
  `policy_present` True and `dmarc.fail.quarantine.v1` for a failing message.
  Both names and their values appear in `dns.lookups`. `v=DMARC1; p=bogus`
  stays `dmarc.permerror.v1`; two `v=spf1` values stay `spf.permerror.v1`;
  `guide_lists_every_auth_rule` passes (AC10, AC21).
- [ ] T33. Pin the null-sender baseline (no behaviour change): add an inline
  fixture pair in `auth.rs`. A null MAIL FROM with HELO `mx.example.com`, a
  passing SPF record there and From `a@example.com` under
  `v=DMARC1; p=reject` has `spf_aligned` True and no `dmarc.*` finding; with
  HELO `mx.example.net` (passing SPF, not aligned) it has `spf_aligned` False
  and `dmarc.fail.reject.v1`. If either fixture already exists, name it in
  the 0126 implementation record instead of duplicating it.
  *Proof:* the pair, plus the existing `tests/deliverability.rs` null-sender
  wire test (AC23).
- [ ] T34. AC18's base: the proof is `git diff --exit-code c95386e --
  Cargo.toml Cargo.lock evals/Cargo.lock deny.toml`, and `git diff c95386e
  7478073 -- Cargo.lock evals/Cargo.lock deny.toml` changes only the
  `swarmail` package's own `version` line. Add a dated line to 0126's
  implementation record that the earlier `7478073` proof is superseded for
  this reason; do not rewrite the earlier record.
  *Proof:* both commands, with their output in the gate log (AC18).
- [ ] T35. Implementation note for the return: append a dated
  "Implementation record — design return" to 0126. It says which tasks
  landed, the fixture names for AC21 and AC23, and that the RFC 9989 text was
  not consulted (no copy in the realm, no web access), so the readings rest
  on RFC 7489 and the pinned crate as 0126 §5 and §7 state.
  *Proof:* review; the status line stays `proposed` (AC20, AC22).
- [ ] T36. Rerun every house gate `--locked` on the new tree, as T25 lists,
  and commit unsigned in the repository's style (never push, merge or accept
  a decision). Report holds 1a/1b/1c, 2 and 3 in the result.
  *Proof:* the gate log; `git log --format='%G? %s'` shows `N` (AC19).
