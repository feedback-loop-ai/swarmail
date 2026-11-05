# Offline DKIM verification (#254)

Decision: 0125 — [docs/decisions/0125-offline-dkim-verification.md](../../docs/decisions/0125-offline-dkim-verification.md) (proposed; only the operator accepts it)

Epic #251, issue #254, priority P1. Run `issue-254-verify-dkim-signatures-bac57cf3`,
design phase, 2026-11-05, base `main` `fad9e7e`, recipe `claude-careful`.
Framing: the issue text and the operator brief (no `.brokkr/state/tasks/`
framing exists in this worktree).

Authorities:
- the operator's #251 ruling of 2026-10-26 (build on an existing permissive
  crate, widen once);
- accepted [0085](../../docs/decisions/0085-deliverability-report.md) and its
  [contract spec](../deliverability-report-contract/spec.md);
- accepted 0020 (where tests live) and 0045 §8;
- the frozen guarantees 0001–0003 and 0005.

Built on these proposed records without amending them:
- [0120](../../docs/decisions/0120-deliverability-dns-policy.md) (the DNS policy
  and `Lookups`);
- [0121](../../docs/decisions/0121-deliverability-link-checks.md) §5 and
  [0122](../../docs/decisions/0122-deterministic-spam-rules.md) §1.4 (the
  rule-set version).

## Feature and rationale

A deliverability report should say whether a message's DKIM signatures would
verify at a receiver. Today it cannot. Swarmail is an offline test mail
catcher, so the keys come from the test-declared DNS map that #253 built, not
from real DNS. With this feature, each `DKIM-Signature` field of a stored
message is verified against keys from 0120's policy:
- the algorithms are rsa-sha256 and ed25519-sha256;
- canonicalization is relaxed or simple, for headers and body;
- both the header hash and the body hash are checked;
- `l=` and expiry are handled.

Every way a signature can fail to verify is a named finding. Tests can then
assert "my signer works" with `no_findings_at_or_above`, and assert each
specific breakage by its rule id.

The cryptography and canonicalization come from `mail-auth` (operator ruling).
Swarmail owns everything that decides a verdict and that the crate either
does not check or checks against the wall clock: which key source answers,
what time a signature is judged at, the bounds, and the mapping from the
crate's result to a stable, versioned rule id.

## Scope

In:
- verifying DKIM1 signatures (RFC 6376, RFC 8301, RFC 8463) on stored
  messages, lazily, inside the existing report evaluator;
- the mail-auth dependency;
- the 17 `dkim.*.v1` finding rules;
- `RULE_SET_ID` `swarmail-html/6`;
- the corrected `--dns-resolver real` refusal text;
- the guide section.

Out:
- SPF, DMARC and alignment (#255), and ARC (#256);
- `AuthEvidence` and the `bulk.*` rules, which do not change;
- live DNS, a `Real` policy variant, and any network lookup;
- signing in production;
- DKIM2;
- a per-signature result member in the report.

## Contract

### Dependency

`mail-auth = { version = "=0.14.0", default-features = false, features = ["aws-lc-rs"] }`
is added to `Cargo.toml` with a one-sentence reason and locked with
`--locked`. No other direct dependency is added. That includes no
`aws-lc-rs` or `hickory` line and no dev-dependency: test signing uses
mail-auth's public `DkimSigner`, `SigningKey`, `RsaKey` and `Ed25519Key`.
- `deny.toml` is expected to stay unchanged.
- `native-tls` stays absent.
- `evals/Cargo.lock` is unchanged.
- 0125 §9 lists the stop conditions.

### Key source: the cache adapter (crate-private)

mail-auth is called as
`verify_dkim(Parameters::new(&msg).with_cache(&adapter))`. `adapter`
implements mail-auth's `DnsCache`, and only its `txt()` member is present.
Its `ResolverCache<Box<str>, TxtRecord>::get(name)` **always returns
`Some`**. It works like this:
- It strips one trailing dot and tries `OwnerName::parse`.
  - If the name is outside 0120's grammar, it is not looked up and not
    recorded. The adapter remembers it as invalid and returns
    `TxtRecord::Error(DnsError::RecordNotFound)`.
  - Otherwise it calls `Lookups::resolve_txt`, which records the name. Then:
    - `Records(values)` is parsed into the record type the name's shape
      selects: an ATPS record when a label is `_atps`, an RFC 6651 report
      record when the name begins `_report._domainkey.`, and a `DomainKey`
      otherwise. A parse failure is returned the way mail-auth's own
      resolver path returns it.
    - `NoRecords` becomes `TxtRecord::Error(DnsError::RecordNotFound)`.
    - `TempFail` becomes `TxtRecord::Error(DnsError::Resolver("declared tempfail"))`,
      which mail-auth reports as `TempError`.
- `insert` is a no-op.
- The `MessageAuthenticator` is built once (`LazyLock`) from a hickory
  configuration with no name servers. It never reads `resolv.conf`.
- The async `verify_dkim` future is driven with
  `futures_util::FutureExt::now_or_never()`. The adapter never misses, so it
  completes on the first poll. The `None` case is an `expect` with a fixed
  message, not a branch.

### Verification time

`receipt = max(email.received_ms, 0) / 1000` (whole seconds, floor) is passed
into `evaluate` beside `raw`. It is not a member of `Inputs`, because the
cache's Weak identity of the immutable `Email` already covers it.
- Swarmail reads no clock.
- mail-auth reads one, and 0125 §8 states that residual.

### Evaluation

`dkim::collect(raw, &fields, body_offset, receipt, &mut lookups, &mut out)`
runs after `spam::collect`.
1. If the message has no root `DKIM-Signature` field, return at once. No
   lookup and no finding.
2. **Bounds.** More than `MAX_DKIM_SIGNATURES = 16` fields, or any field
   whose `h=` lists more than `MAX_DKIM_SIGNED_HEADERS = 128` names (counted
   by the pre-scan), gives exactly one `dkim.limits_exceeded.v1` finding.
   Its location is the first field past a bound: occurrence 16 for the
   count, or the offending field for `h=`. mail-auth is not called, and
   nothing is looked up.
3. Otherwise the message is parsed with
   `AuthenticatedMessage::parse_with_opts(raw, None, false)`. Non-strict mode
   keeps `l=` signatures so Swarmail can judge them. Then `verify_dkim` runs
   once.
4. Each field k (zero-based, in header order) gets exactly one verdict. The
   first matching row below wins:

| # | condition | verdict |
|---|---|---|
| a | the field's tag list repeats a tag name (Swarmail pre-scan, RFC 6376 §3.2) | `dkim.malformed.v1` |
| b | `Signature::parse` fails with an unsupported `v=`, `a=` or `c=` | `dkim.unsupported.v1` |
| c | `Signature::parse` fails in any other way: missing tag, bad base64 or bad quoted-printable | `dkim.malformed.v1` |
| d | `a=rsa-sha1` (RFC 8301), or a literal `l=0` tag (the crate cannot tell it from an absent `l=`) | `dkim.unsupported.v1` |
| e | `h=` does not name `from` (case-insensitive; RFC 6376 §5.4) | `dkim.from_unsigned.v1` |
| f | `x` ≠ 0 and `x` ≤ `t` | `dkim.malformed.v1` |
| g | `x` ≠ 0 and `x` ≤ `receipt` | `dkim.signature_expired.v1` |
| h | crate `Neutral(SignatureExpired)` (rows f and g did not match, so the wall clock passed `x` after receipt) | `dkim.expired_since_receipt.v1` |
| i | crate `Neutral(BodyHashMismatch)` | `dkim.body_hash_mismatch.v1` |
| j | crate `TempError` (declared `tempfail`), for the key name or the ATPS name | `dkim.not_evaluated.v1` |
| k | crate `PermError` with `is_atps()` | `dkim.atps_unauthorized.v1` |
| l | crate `PermError(RecordNotFound)` for a name the adapter found invalid | `dkim.selector_invalid.v1` |
| m | crate `PermError(RecordNotFound)` | `dkim.key_missing.v1` |
| n | crate `PermError(PublicKeyRevoked)` (`p=` empty) | `dkim.key_revoked.v1` |
| o | any other crate `PermError`: unparseable record, unsupported `k=`, or a key type that does not match `a=` | `dkim.key_invalid.v1` |
| p | crate `Fail(AuidMismatch)` | `dkim.auid_mismatch.v1` |
| q | crate `Fail` from a key the crypto library refuses, or a key/algorithm mismatch | `dkim.key_invalid.v1` |
| r | crate `Fail(FailedVerification)`: the header hash and `b=` do not verify | `dkim.signature_mismatch.v1` |
| s | crate `Pass`, `l` ≠ 0 and `l` > the canonicalized body length | `dkim.body_length_exceeds.v1` |
| t | crate `Pass`, `l` ≠ 0 and `l` < the canonicalized body length | `dkim.body_length_partial.v1` |
| u | crate `Pass` otherwise | no finding |

A field that fails rows a–g is still parsed by mail-auth. Its crate output is
ignored, and Swarmail's verdict stands. The crate lists its outputs in a fixed
order: first one `neutral` output for each field rejected with a `DkimError`,
then one output for each parsed signature, in header order. Fields rejected
with any other error get no output. Swarmail parses each field itself with
the same public `Signature::parse`, so it knows which output belongs to
which field. Each mapped output's signature must equal Swarmail's own parse
of that field. A mismatch is an `expect` with a fixed message, never a guess;
it fails that analysis through the existing offload panic symmetry. Every
crate result is mapped to one row, and no `match` arm is unreachable: a crate
variant this crate version never produces for a parsed signature is grouped
with a reachable arm.

The **canonicalized body length** is computed by Swarmail (`canonical_len`)
over `raw[body_offset..]` for the signature's body canonicalization
(RFC 6376 §3.4.3 simple, §3.4.4 relaxed). A test binds it to the `l=` that
mail-auth's signer writes with `body_length(true)`.

### Rule table

Every rule is `dkim.<mode>.v1`, at
`{"kind":"header","name":"dkim-signature","occurrence":k}`. Each fix hint is a
fixed string that cites the RFC section.

| rule id | severity | failure mode | passing fixture | failing fixture |
|---|---|---|---|---|
| `dkim.malformed.v1` | error | duplicate tag, missing required tag, bad base64, or `x` ≤ `t` | well-formed signature | `bh=` missing; `d=` twice; `x=t` |
| `dkim.unsupported.v1` | error | `v`≠1, unknown `a=`/`c=`, `rsa-sha1`, literal `l=0` | ed25519-sha256 | correctly signed `a=rsa-sha1`; `l=0` |
| `dkim.from_unsigned.v1` | error | `h=` omits `from` | `h=from:subject` | `h=subject` |
| `dkim.limits_exceeded.v1` | error | more than 16 fields, or more than 128 `h=` names | 16 valid fields; 128 names | 17 fields; 129 names |
| `dkim.signature_expired.v1` | error | `x` ≤ receipt | `x` = receipt + 1 | `x` = receipt |
| `dkim.expired_since_receipt.v1` | error | `x` after receipt but passed by the wall clock (0125 §8) | `x` far after the wall clock | receipt 1,500, `t` 1,000, `x` 2,000 |
| `dkim.selector_invalid.v1` | error | `<s>._domainkey.<d>` outside 0120's grammar; never looked up | `s=sel` | `s=bad*sel` |
| `dkim.key_missing.v1` | error | key name declared nowhere (`no_records`) | key declared | key not declared |
| `dkim.key_invalid.v1` | error | record unparseable, unsupported `k=`, or key type not matching `a=` | valid `p=` | `p=` of garbage; ed25519 key for `a=rsa-sha256` |
| `dkim.key_revoked.v1` | error | `p=` empty | valid `p=` | `v=DKIM1; p=` |
| `dkim.not_evaluated.v1` | error | key name (or ATPS name) declared `tempfail` | key declared | key name in `tempfail` |
| `dkim.body_hash_mismatch.v1` | error | body hash ≠ `bh=` | intact body | one body byte altered |
| `dkim.signature_mismatch.v1` | error | header hash does not verify against `b=` | intact headers | signed `Subject` altered |
| `dkim.auid_mismatch.v1` | error | `i=` domain not `d=` or a subdomain of it (or `t=s` key, subdomain) | `i=@d` | `i=@other.example` |
| `dkim.atps_unauthorized.v1` | error | `atps=` matches a From domain and the ATPS record is absent | ATPS record declared | ATPS record not declared |
| `dkim.body_length_exceeds.v1` | error | `l=` longer than the canonicalized body | `l` = canonical length | crafted `l` = length + 10 |
| `dkim.body_length_partial.v1` | warning | verified, but `l=` covers only a prefix (RFC 6376 §8.2) | `l` = canonical length | `l=` short, then text appended |

The table has 17 finding rules. A passing signature and an unsigned message
emit no `dkim.*` finding, at any severity. No `dkim.pass`, `dkim.missing` or
`dkim.valid` id exists.

### Report and version

- The report shape `{rule_set_id, findings, dns}` is unchanged.
- `RULE_SET_ID` is `swarmail-html/6` everywhere it is spelled: the
  constant, `html-rules.json`, OpenAPI and MCP examples, the guide,
  `llms.txt`, and the literal-report tests and fixtures.
- An unsigned message's report differs from today's only in `rule_set_id`.
- A signed message's `dns.lookups` lists each name mail-auth asked for, once,
  sorted, with its outcome (0120 §5).
- mail-auth asks for a key only for a parsed, unexpired signature whose body
  hash matches. Rows b, c, f, g, h and i therefore record no lookup for
  their field. Rows a, d and e can record the key lookup the crate made,
  because the crate still evaluates those signatures, and the report says
  so. Their verdict is still Swarmail's.
- A report with a `temp_fail` lookup still never qualifies (0120 §4).

### Configuration

`--dns-resolver real` stays refused before any listener binds. The fixed
message becomes
`real DNS is not supported; declare records with --dns-policy or PUT /api/v1/inboxes/{inbox}/dns`.
`llms.txt` and the guide say the same.

## Frozen guarantees and failure modes

| concern | answer |
|---|---|
| 0001 accept == stored | nothing in DKIM runs before or inside `Store::insert`; verification is lazy, after the `250`, from stored bytes |
| 0002 subscribe-before-check | the wait loop is untouched; a `temp_fail` lookup is never clean, so a wait reaches its existing deadline outcome |
| 0003 oldest-only pruning | verification holds the selected `Arc<Email>` like every collector; no slot, no retention exception |
| 0005 ingest extraction | the evaluator reads the committed bytes; nothing is persisted |
| hanging webhook / network | no socket on this path; declared lookups are map reads; the resolver has no name servers |
| hostile DATA | the existing 8 MiB, 64 KiB header and part bounds stand; DKIM adds the 16-signature and 128-name bounds; malformed signatures are findings, not `Failed` |
| clock | only the crate's internal read (0125 §8), which can move one verdict toward "not verified", never toward clean |
| 1,000 sessions | four report permits and the 128-slot cache are unchanged; at most 16 verifications and 16 lookups per report |

## Panel adjudication — 2026-11-05

The chief checked every disputed fact against the `mail-auth-0.14.0` source
in the offline registry before ruling.

- **Crypto backend: simplicity's `aws-lc-rs`, adopted.** Robustness asked for
  `features = ["ring"]`. mail-auth 0.14.0 has no such feature (its features
  are `arc`, `aws-lc-rs`, `dns-doh`, `dns-hickory`, `generate`,
  `rust-crypto`, `test`). Robustness also asked that no hickory crate be
  present. That is impossible, because a DNS backend is a `compile_error!`
  requirement. *Cost to robustness's side:* the resolver ships, unreachable,
  and is recorded rather than hidden. Robustness's 1,024-bit preflight
  resolves in favour: the aws-lc path verifies RSA keys of 1,024 to 8,192
  bits.
- **Key source: simplicity, adopted.** `Lookups` is called inside the
  evaluator through a cache adapter. Robustness's pre-evaluate snapshot was
  0120 §10's answer for a *live* resolver. The declared map cannot hang.
  *Cost:* a future live-DNS story builds its own pre-evaluate phase.
- **Time: simplicity's receipt time, adopted; robustness's request instant
  plus cache bypass, rejected.** Receipt time makes a cached verdict correct
  forever. A request instant would make every `x=`-bearing report
  uncacheable and clock-dependent by design. *Cost:* the named residual
  `dkim.expired_since_receipt.v1` (0125 §8). Robustness's honesty demand is
  adopted: the exception to byte-identical reports is stated.
- **Parsing: mail-auth parses (simplicity).** A narrow Swarmail pre-scan is
  adopted from robustness, covering what the crate's public struct hides:
  duplicate tags, literal `l=0` and the `h=` count. The rest of robustness's
  full envelope parser is rejected. *Cost:* a crate upgrade could change a
  verdict. Robustness's mitigation is adopted: an exact pin, and a fixture
  for each rule that snapshots the rule id.
- **`l=`: robustness, adopted.** Simplicity said a too-long `l=` surfaces as
  a body-hash mismatch. The source shows otherwise: mail-auth hashes
  `min(l, len)`, so a crafted `l=` longer than the body would pass. Swarmail
  computes the canonicalized length and reports `body_length_exceeds`
  (error) and `body_length_partial` (warning). For `partial`, simplicity's
  `warning` wins over robustness's `info`, because appended unsigned content
  is RFC 6376 §8.2's attack.
- **Rules adopted from robustness:** `from_unsigned` (mail-auth does not
  check `From` in `h=`), `key_revoked` and `selector_invalid` are kept
  distinct from `key_missing`/`key_invalid`. `x` ≤ `t` is malformed, not
  expired.
- **Rules adopted from simplicity:** `rsa-sha1` is unsupported, and
  `auid_mismatch` and `signature_mismatch` are named.
- **The chief's own additions:**
  - `atps_unauthorized`: mail-auth changes a verified signature's result
    on an RFC 6541 lookup, so that outcome needs a stable name;
  - `expired_since_receipt`, kept apart from `not_evaluated` so that a
    `tempfail` and an aged signature do not share a hint;
  - `limits_exceeded`.
- **Severities: simplicity, adopted.** Every unverified outcome is `error`.
  *Cost to robustness's side:* malformed or expired signatures disqualify
  reports at the `error` threshold too. Robustness's `info` "not evaluated"
  would be falsely clean at `warning` in the residual case, which records no
  `temp_fail` lookup.
- **Bounds: merged.** Simplicity's 16 signatures, and robustness's `h=` cap
  (raised to 128 for real oversigning, because mail-auth's header selection
  is O(`h=` × headers)). Robustness's "a finding, never `Failed`" is
  adopted. Simplicity's "do not evaluate past the bound" is adopted, because
  mail-auth hashes every signature at parse time, so a partial evaluation
  would not bound the work. *Cost:* no per-signature verdicts over a bound.
- **No pass finding, no missing finding, no report member, no `AuthEvidence`
  change, real DNS refused:** both positions agree. Robustness's
  crate-private `DkimVerdict` interface for #255 is rejected as speculative.
  #255 adds what it consumes.

## Acceptance Criteria

Each criterion names the proof that can fail it.

- **AC1: Unsigned reports unchanged.** A message with no `DKIM-Signature`
  gives exactly the previous report with `rule_set_id` `swarmail-html/6` and
  empty `dns.lookups`. *Proof:* the existing exact-JSON tests in
  `tests/deliverability.rs`, `tests/bulk_sender.rs` and
  `tests/spam_rules.rs`, updated only in the id.
- **AC2: Both algorithms, all four canonicalizations, pass.** rsa-sha256
  (2,048-bit and 1,024-bit keys) and ed25519-sha256 signatures, each under
  `simple/simple`, `relaxed/relaxed`, `relaxed/simple` and `simple/relaxed`,
  with the key declared, give no `dkim.*` finding and one `records` lookup
  for `<s>._domainkey.<d>`. *Proof:* inline `dkim` matrix test.
- **AC3: Published vector.** The RFC 8463 Appendix A signed message, with its
  A.2 keys declared, verifies both its signatures. *Proof:* inline test. If
  the published vector is shown not to verify (an erratum), the ledger
  records the evidence. The test then pins the observed verdict, and AC2
  carries the pass proof.
- **AC4: Canonicalization is real.** Folding a signed header and doubling
  inner body whitespace fails under `simple` (signature mismatch and body
  hash mismatch respectively) and passes under `relaxed`. *Proof:* inline
  test per mode.
- **AC5: Every rule has a passing and a failing fixture.** For each of the 17
  rules in §Rule table, the failing fixture yields exactly that rule id,
  severity and occurrence. The passing fixture yields no finding of that id.
  Both algorithms are exercised for `body_hash_mismatch` and
  `signature_mismatch`. *Proof:* inline `dkim` tests, one pair per rule.
- **AC6: Precedence and independence.** In one message, a passing, a
  malformed (before it) and a failing signature give exactly two findings,
  at the correct occurrences. A field rejected with a non-`DkimError`
  (missing `bh=`), placed between two parsed fields, does not shift the later
  occurrences. Each row a–u of the evaluation table is hit by at least one
  fixture. *Proof:* inline test, plus exact branch coverage.
- **AC7: Receipt time, not wall time.** The same signed bytes with
  `received_ms` before `x` give no finding. With `received_ms` at `x`, they
  give `signature_expired`. Fixtures sit at the poles: the receipt is
  generated relative to the signature's own `t`/`x`, with no sleep and no
  wall-clock assertion. The residual fixture (`t`=1,000, `x`=2,000, receipt
  1,500 s) gives `expired_since_receipt`. *Proof:* inline tests.
- **AC8: `l=`.** `canonical_len` equals the `l=` mail-auth's signer writes
  with `body_length(true)`, for simple and relaxed over a table of bodies:
  empty, trailing blank lines, trailing whitespace, a bare LF, no final
  CRLF. A signature with `l` equal to the length is clean. Text appended
  after a verified `l=` gives `body_length_partial` (warning). A crafted
  `l` beyond the length gives `body_length_exceeds`. A literal `l=0` gives
  `unsupported`. *Proof:* inline tests.
- **AC9: The only key source, never the network.** An undeclared key name
  yields `key_missing`, not a resolver error, which proves the adapter
  answered and the resolver was never asked. A `tempfail` name yields
  `not_evaluated`, records a `temp_fail` lookup, and `Report::qualifies` is
  false at every threshold. An invalid owner name yields `selector_invalid`
  and records no lookup. `r=y` on a failing signature records the
  `_report._domainkey.<d>` lookup. *Proof:* inline tests.
- **AC10: Bounds.** 16 valid signatures are all verified. 17 give exactly one
  `limits_exceeded` at occurrence 16, no other `dkim.*` finding and no
  lookup. A signature with 129 `h=` names gives the same at its occurrence.
  128 names are evaluated. *Proof:* inline tests.
- **AC11: Over the wire.** Over real SMTP, a signed message is sent to an
  inbox whose key is declared with `PUT /api/v1/inboxes/{inbox}/dns`. The
  REST report has no `dkim.*` finding and records the lookup. A second
  message with an altered body gives `dkim.body_hash_mismatch.v1`. An await
  with `no_findings_at_or_above=warning` returns the first message and not
  the second. *Proof:* `tests/deliverability.rs`.
- **AC12: Determinism.** Two evaluations of the same signed `Email`, and two
  separate processes serving the same bytes and policy, give byte-identical
  report JSON (outside the 0125 §8 window). *Proof:* inline test plus the
  existing two-process comparison in `tests/deliverability.rs`, extended
  with a signed message.
- **AC13: Dependency.**
  - `Cargo.toml` carries the exact pin with `default-features = false` and
    only `aws-lc-rs`.
  - `cargo deny check licenses` passes.
  - `deny.toml` is unchanged, unless 0125 §9's widening applies.
  - The `native-tls` absence proof passes, and `evals/Cargo.lock` is
    unchanged.
  - 0125 §9 lists the exact added packages and their licences.

  The existing real-HTTPS webhook and STARTTLS tests pass with both rustls
  providers compiled. *Proof:* the gates, `git diff --stat`, and the
  existing TLS suites.
- **AC14: Real DNS still refused.** `--dns-resolver real` and
  `SWARMAIL_DNS_RESOLVER=real` exit non-zero with the new fixed message
  before any listener binds. *Proof:* the existing refusal tests, updated.
- **AC15: Documentation.**
  - `docs/guide/deliverability.md` documents each rule id, its severity,
    the receipt-time rule, the residual, the bounds and the version
    history entry for `swarmail-html/6`.
  - The 0125 index row is present in `docs/decisions/README.md`.
  - The evaluator's header comment states the crate clock honestly.

  *Proof:* a test binding the guide's rule list to the rule table (as the
  spam phrase list is bound), plus review.
- **AC16: Gates.** Every house gate passes `--locked`: fmt, clippy, tests,
  the evals crate against the candidate binary, the coverage gate at 100%
  and its selftest, publish-guard, seal-branch and `cargo deny`. The
  brokkr bundle compile runs in verify if the sandbox has no brokkr.
