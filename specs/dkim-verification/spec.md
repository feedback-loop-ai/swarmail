# Offline DKIM verification (#254)

Decision: 0125 — [docs/decisions/0125-offline-dkim-verification.md](../../docs/decisions/0125-offline-dkim-verification.md) (proposed; only the operator accepts it)

Epic #251, issue #254, priority P1. Run `issue-254-verify-dkim-signatures-bac57cf3`,
base `main` `fad9e7e`, recipe `claude-careful`. Framing: the issue text and the
operator brief (no `.brokkr/state/tasks/` framing exists in this worktree).

- **First design:** 2026-11-05, commit `8705096`. Implemented in `adeaa12`.
- **Revision 1 (this text):** 2026-11-05, after the review panel returned the
  run to design (`spec_defect: true`, a HIGH accepted-ruling finding and a
  security residual; decision 0102's addendum of 2026-10-31). This file is now
  the **single normative account**. It absorbs 0125's implementation record
  items 1–5 and replaces every mechanism that record superseded. The
  [return adjudication](#return-adjudication--2026-11-05-revision-1) says what
  changed and why. The [first panel adjudication](#panel-adjudication--2026-11-05)
  is kept as dated history.

Authorities:
- the operator's #251 ruling of 2026-10-26: build on an existing permissive
  crate, and widen the allowlist once;
- accepted [0085](../../docs/decisions/0085-deliverability-report.md) and its
  [contract spec](../deliverability-report-contract/spec.md), including §8's
  "no ambient DNS/time" (see [Operator holds](#operator-holds));
- accepted 0020 (where tests live) and 0045 §8 (code under a proposed decision);
- the frozen guarantees 0001–0003 and 0005.

Proposed records this builds on:
- [0120](../../docs/decisions/0120-deliverability-dns-policy.md): the DNS
  policy and `Lookups`. This story moves 0120 §5's "production lookups are
  empty", §7's live-answer semantics and §10's resolver promise. It therefore
  carries a dated 2026-11-05 **integration note** on 0120. It does not claim
  "without amending".
- [0121](../../docs/decisions/0121-deliverability-link-checks.md) §5 and
  [0122](../../docs/decisions/0122-deterministic-spam-rules.md) §1.4: the
  rule-set version.

## Feature and rationale

A deliverability report should say whether a message's DKIM signatures would
verify at a receiver. Swarmail is an offline test mail catcher, so the keys come
from the test-declared DNS map that #253 built, not from real DNS. Each
`DKIM-Signature` field of a stored message is verified against keys from 0120's
policy:
- the algorithms are rsa-sha256 and ed25519-sha256;
- canonicalization is relaxed or simple, for headers and body;
- both the header hash and the body hash are checked;
- `l=` and expiry are handled.

Every way a signature can fail to verify is a named finding, so a test can
assert each specific breakage by its rule id.

The cryptography, canonicalization and hashing come from `mail-auth` (operator
ruling). **Swarmail decides every verdict the crate would otherwise decide by
default, silently or against the wall clock.** Revision 1 sharpens this
principle because the review found five places where the crate decided a report
byte that no Swarmail code chose:
- the body boundary;
- a missing `a=`;
- non-numeric `t=`, `x=` and `l=`;
- output-to-field pairing;
- the asked key name.

The sixth such place is the crate's wall clock. Swarmail cannot remove it
without a second verifier. It is stated with its true blast radius, and it is a
merge hold for the operator ([Operator holds](#operator-holds)).

## Scope

In:
- verifying DKIM1 signatures (RFC 6376, RFC 8301, RFC 8463) on stored
  messages, lazily, inside the existing report evaluator;
- the mail-auth dependency;
- the 18 `dkim.*.v1` finding rules;
- `RULE_SET_ID` `swarmail-html/6`;
- the corrected `--dns-resolver real` refusal text;
- the guide section and every live surface that describes the report;
- the 0120 integration note and the dependency records (CDLA comment).

Out:
- SPF, DMARC and alignment (#255), and ARC verification (#256);
- `AuthEvidence` and the `bulk.*` rules, which do not change;
- the await clause of the first AC11, which moves to #255
  ([AC11](#acceptance-criteria));
- live DNS, a `Real` policy variant, and any network lookup;
- signing in production;
- DKIM2;
- a per-signature result member in the report.

## Operator holds

The delivery PR is **not merged on green**. It is held for the operator on
three named items. Each is listed in 0125's header and in the PR body:

1. **`Cargo.lock` gains packages, and the `deny.toml` comment changes**
   (0056's 2026-11-01 addendum).
2. **The crate's clock against accepted 0085 §8** (0125 §8). mail-auth
   0.14.0 compares `x=` with `SystemTime::now()` inside `verify_dkim`.
   Accepted 0085 §8 says no ambient time is introduced into the collector.
   Under 0045 §8, code that may contradict an accepted ruling waits for the
   operator's acceptance of the proposal that amends it. 0125 §8 is drafted as
   that **proposed amendment**, with the full blast radius below. The chief
   does not decide that the read is compatible.
3. **The resolver is compiled in** (0125 §2). The brief said to turn off any
   network resolver features. mail-auth cannot build without a DNS backend,
   and `aws-lc-rs` enables `dns-hickory`. The resolver is unreachable for two
   independent reasons, but the feature is not off. The operator accepts the
   deviation or rules otherwise.

If the operator refuses hold 2, the alternatives are scope rulings for the
operator, and none is built here:
- **B:** a signature with `x=` after the receipt second is excluded from the
  crate's verification with the public `retain_dkim_signatures` and gets one
  deterministic error id. Every `x=`-bearing signature then goes unverified.
- **B′:** wait for an upstream mail-auth API that takes the verification time.

Two other options were ruled out:
- vendoring or `[patch]`ing mail-auth, because the coverage gate refuses a
  local path package (house rules § Gates, rule 6) and a git patch does not
  resolve offline;
- re-implementing the verifier, because that makes a second owner of the crypto
  verdict. mail-auth 0.14.0 also exposes no public verify that takes a time or
  a parsed key: `verify_dkim_`, `DomainKey.p` and the public-key types are
  `pub(crate)`.

## Contract

### Dependency

`mail-auth = { version = "=0.14.0", default-features = false, features = ["aws-lc-rs"] }`
is in `Cargo.toml` with a one-sentence reason, locked with `--locked`.
- No other direct dependency, and no dev-dependency. Test signing uses
  mail-auth's public `DkimSigner`, `SigningKey`, `RsaKey`, `Ed25519Key` and
  `canonicalized_dkim_headers`.
- The `deny.toml` **allowlist** is unchanged. Its CDLA-Permissive-2.0
  **comment**, and the matching house-rules § Dependencies sentence, now name
  both crates under that allowance: `webpki-roots` and `webpki-root-certs`,
  the second pulled in by hickory's platform verifier. Both are data.
- `native-tls` stays absent.
- `evals/Cargo.lock` is unchanged.
- 0125 §9 lists the 58 added packages and their licences, and the stop
  conditions.

### Key source: the cache adapter (crate-private)

mail-auth is called as
`verify_dkim(Parameters::new(&msg).with_cache(&adapter))`. `adapter` implements
mail-auth's `DnsCache`, with only its `txt()` member present. Its
`ResolverCache<Box<str>, TxtRecord>::get(name)` **always returns `Some`**:
- The name is read from the key's `Hash` output through `name_of`, as the bytes
  of the `str` followed by `0xff`. `ResolverCache::get` is generic over the
  borrowed key, so this is the only way to read it. This is a **known
  off-idiom read** of a std internal. A test holds it to the real lookups. Any
  other key shape reads as no name and answers "not found", which fails closed.
- **One function judges a name** (`owner_of(name) -> Option<OwnerName>`): strip
  exactly one trailing dot, then `OwnerName::parse`. A name outside 0120's
  grammar is not looked up and not recorded, and it answers
  `TxtRecord::Error(DnsError::RecordNotFound)`.
- Otherwise the adapter calls `Lookups::resolve_txt`, which records the name:
  - `Records(values)` is parsed into the record type the name's shape selects:
    an RFC 6651 report record when the name begins `_report._domainkey.`; an
    ATPS record when it has an `_atps.` label and no `_domainkey.` label; a
    `DomainKey` otherwise;
  - `NoRecords` becomes `RecordNotFound`;
  - `TempFail` becomes `DnsError::Resolver("declared tempfail")`, which
    mail-auth reports as `TempError`.
- `insert` is a no-op.
- The `MessageAuthenticator` is built once (`LazyLock`) from a hickory
  configuration with **no name servers**. It never reads `resolv.conf`.
- The async `verify_dkim` future is driven with `now_or_never()`. The adapter
  never misses, so the future completes on the first poll. `None` is an
  `expect` with a fixed message, not a branch.

### Verification time

`receipt = max(email.received_ms, 0) / 1000` (whole seconds, floor) is passed
into `evaluate_at` beside `raw`. It is not a member of `Inputs`, because the
cache's Weak identity of the immutable `Email` already covers it.
- Swarmail reads no clock.
- The crate reads one. Its effect is the **clock residual** (0125 §8):
  1. It applies only to a parsed signature that passed every Swarmail row a–g
     below, so it carries a numeric `x=` after both `t=` and the receipt
     second.
  2. Once the host clock reaches that `x=`, the crate returns
     `Neutral(SignatureExpired)` **before** it checks the body hash, fetches
     the key, checks the AUID or verifies `b=`.
  3. Every verdict rows h–u could have produced for that field therefore
     becomes `dkim.expired_since_receipt.v1` (error). This includes no finding
     (a pass), the `body_length_partial` warning, `body_length_exceeds`,
     `body_hash_mismatch`, `not_evaluated`, `atps_unauthorized`,
     `selector_invalid`, `key_missing`, `key_revoked`, `key_invalid`,
     `auid_mismatch` and `signature_mismatch`.
  4. The key and ATPS lookups for that field disappear from `dns.lookups`.
  5. A clean or warning-only report can begin to disqualify. A report
     recomputed after cache eviction can differ from the cached one, which is
     the one exception to 0120 §5's byte-identical reports.
  6. The move is one-way unless the host clock is set back.
  7. Rows a–g, every message-level step and every unsigned message are
     clock-free.

### Evaluation

`dkim::collect(raw, root, size, receipt, lookups, out)` runs after
`spam::collect`. `root` and `size` are the root header list and boundary that
`headers(raw)` already computed for every other collector. `size` is the byte
offset where Swarmail's body starts.

1. **Strict parse.** `strict = AuthenticatedMessage::parse_with_opts(raw, None, true)`.
2. **Fields.** The crate's own view, in header order:
   - `dkim_signatures()`;
   - plus `errors()` entries whose header name `is_dkim_signature`, comparing
     the name case-insensitively with whitespace removed.

   Both lists are sorted by the field's position in `raw`.
3. **Signed?** The message is signed when `root` holds a field named
   `DKIM-Signature` by `is_dkim_signature`, or when step 2 lists any field. An
   unsigned message returns at once, with no lookup and no finding.
4. **Boundary guard.** If `strict` is `None`, or `strict.body_offset() != size`,
   the message gets exactly one `dkim.header_boundary_ambiguous.v1`. It is
   located at occurrence 0, and the step returns: no verification, no lookup, no
   other `dkim.*` finding. Example: a whitespace-only line `" \r\n"` ends the
   header block for mail-auth, but Swarmail folds it as a continuation.
   - When the boundaries agree, DKIM's body is exactly `raw[size..]`, the body
     every other collector and the UI see.
   - The crate's header block then equals Swarmail's, which `headers()` caps at
     64 KiB.
5. **Bounds.** Two limits apply: more than `MAX_DKIM_SIGNATURES = 16` fields in
   step 2, or any field whose `h=` lists more than
   `MAX_DKIM_SIGNED_HEADERS = 128` names. Past either, the message gets exactly
   one `dkim.limits_exceeded.v1`. It is located at the first field past a bound:
   occurrence 16 for the count, or the offending field for `h=`, whichever is
   lower. Then the step returns: no verification, no lookup.
6. **Re-parse for `l=`.** If any field carries `l=`, the message is parsed again
   with `parse_with_opts(raw, None, false)`, so `l=` signatures are judged, not
   dropped. That parse is an `expect`: a message that parsed strictly parses
   without strict. Otherwise `strict` is used.
7. **Verify once.** `verify_dkim` runs once over the chosen message through the
   adapter.
8. **Pair by identity.** For each parsed field, its output is the one whose
   `DkimOutput::signature()` is the same reference, compared with `ptr::eq`, as
   the field's `&Signature` from `dkim_signatures()`. A parsed field with no
   such output is an `expect` with a fixed message. It fails that one analysis
   through the existing offload panic symmetry (`503 unavailable`), never a
   wrong verdict. No index arithmetic exists: the first design's `first_parsed`
   count is deleted. A field rejected by the parser is judged by its own error
   and needs no output.
9. **Verdict.** Each field k (zero-based, in header order) gets exactly one
   verdict. The first matching row wins. The **pre-scan** (`Scan`) reads the
   field's raw tag list, splitting on `;` and the first `=`, with FWS trimmed
   around names and values.

| # | condition | verdict |
|---|---|---|
| a | pre-scan: a repeated tag name (RFC 6376 §3.2); no `v=`; **no `a=`** (RFC 6376 §3.5: the crate would default to rsa-sha256); or a `t=`, `x=` or `l=` value that is not one or more ASCII digits fitting `u64` (the crate reads garbage or overflow as 0, which means "absent") | `dkim.malformed.v1` |
| b | the crate's parse error `UnsupportedVersion`, `UnsupportedAlgorithm` or `UnsupportedCanonicalization` | `dkim.unsupported.v1` |
| c | any other parse error: missing `d=`, `s=`, `b=`, `bh=` or `h=`, bad base64 or bad quoted-printable | `dkim.malformed.v1` |
| d | `a=rsa-sha1` (RFC 8301), or a literal `l=0` (the crate cannot tell it from an absent `l=`) | `dkim.unsupported.v1` |
| e | `h=` does not name `from` (case-insensitive; RFC 6376 §5.4) | `dkim.from_unsigned.v1` |
| f | the field carries both `x=` and `t=`, and `x` ≤ `t` | `dkim.malformed.v1` |
| g | the field carries `x=` (a literal `x=0` included), and `x` ≤ `receipt` | `dkim.signature_expired.v1` |
| h | the crate returns `Neutral(SignatureExpired)` (the clock residual) | `dkim.expired_since_receipt.v1` |
| i | the crate returns `Neutral(BodyHashMismatch)`, or any other `Neutral` or `None` (this crate version returns neither for a parsed, unexpired signature) | `dkim.body_hash_mismatch.v1` |
| j | the crate returns `TempError`: a declared `tempfail` for the key name or the ATPS name | `dkim.not_evaluated.v1` |
| k | the crate returns `PermError`, and the output `is_atps()`: the ATPS record is undeclared, declared with no records, or not a valid ATPS record | `dkim.atps_unauthorized.v1` |
| l | the crate returns `PermError(RecordNotFound)`, and `owner_of(signature.domain_key())` is `None`: the name the crate asked for, trailing dot included, is outside 0120's grammar | `dkim.selector_invalid.v1` |
| m | the crate returns `PermError(RecordNotFound)` | `dkim.key_missing.v1` |
| n | the crate returns `PermError(PublicKeyRevoked)`: `p=` is empty | `dkim.key_revoked.v1` |
| o | any other `PermError`: `p=` not base64, `p=` missing, `k=` unsupported, or a non-DKIM record | `dkim.key_invalid.v1` |
| p | the crate returns `Fail(AuidMismatch)` | `dkim.auid_mismatch.v1` |
| q | the crate returns `Fail(Crypto(IncompatibleAlgorithms))`: the key type does not match `a=` | `dkim.key_invalid.v1` |
| r | any other `Fail`: `FailedVerification`, a crypto-library error (how aws-lc's ed25519 path words every failed check), or a `p=` that is base64 but not a key | `dkim.signature_mismatch.v1` |
| s | the crate returns `Pass`, the field carries `l=`, and `l` > max(raw body length, canonical body length) | `dkim.body_length_exceeds.v1` |
| t | the crate returns `Pass`, the field carries `l=`, and the canonicalized first `l` raw bytes are shorter than the canonicalized body | `dkim.body_length_partial.v1` |
| u | the crate returns `Pass` otherwise | no finding |

Notes on the table:
- **The crate still evaluates fields judged by rows a and d–g.** Its output for
  them is ignored, and Swarmail's verdict stands.
- **Every crate result maps to one row, and no `match` arm is unreachable.** A
  variant this crate version never produces for a parsed signature is grouped
  with a reachable arm (rows i and r).
- **`l=` counts raw body bytes in mail-auth.** Its signer writes the raw body
  length, and its verifier hashes the first `l` raw bytes. RFC 6376 §3.4.5
  counts canonicalized octets. **Known interop gap:** an RFC-conformant signer
  whose raw and canonical `l`-prefixes differ fails as
  `dkim.body_hash_mismatch.v1`. This is fail-closed and documented in the
  guide's known limits and in 0125. #255 must not read `body_hash_mismatch` on
  an `l=` field as evidence of tampering.
- **The canonicalizer.** The crate's `BodyHasher` is crate-private, so the
  canonicalizer used by rows s and t is Swarmail's (`canonicalize` /
  `canonical_len`). It mirrors the crate's observable output, including bare LF
  and lone CR. A test holds it byte-equal to the crate's own `bh=` over the
  oracle table (AC8).

### Work bound

The strict parse runs first. It hashes the body at most once for each distinct
pair of body canonicalization and hash algorithm among signatures without
`l=`: at most four hashes over at most 8 MiB. It rejects every `l=` signature
without hashing it.

Past the boundary guard or either bound, `verify_dkim` is not called and nothing
is looked up. Within the bounds:
- the optional re-parse adds at most one hash for each distinct
  (canonicalization, hash, `l`) triple, at most 16;
- there are at most 16 verifications;
- each signature asks for at most three names: its key, its ATPS name, and the
  RFC 6651 `_report._domainkey.<d>` name when `r=y`;
- so at most 48 distinct names are asked for. Only names inside 0120's grammar
  are recorded.

A report carries at most 16 DKIM findings, or one message-level finding, which
is well under 0085's 256.

### Rule table

Each rule's id is `dkim.<mode>.v1`. Its location is
`{"kind":"header","name":"dkim-signature","occurrence":k}`; the two
message-level rules use occurrence 0 or the first offending field. Each fix
hint is a fixed string that cites the RFC section and names the condition the
code actually tests.

| rule id | severity | failure mode | passing fixture | failing fixture |
|---|---|---|---|---|
| `dkim.malformed.v1` | error | duplicate tag; missing `v=`, `a=`, `d=`, `s=`, `b=`, `bh=` or `h=`; bad base64; non-numeric or overflowing `t=`/`x=`/`l=`; `x` ≤ `t` | well-formed signature | `bh=` missing; `d=` twice; no `v=`; no `a=`; `x=abc`; `t=abc`; `l=abc`; `x=` 99999999999999999999; `x=t` |
| `dkim.unsupported.v1` | error | `v`≠1, unknown `a=`/`c=`, `rsa-sha1`, literal `l=0` | ed25519-sha256 | `a=rsa-sha1` (the tag rewritten on a signed header and re-signed); `l=0` |
| `dkim.from_unsigned.v1` | error | `h=` omits `from` | `h=from:subject` | `h=subject` |
| `dkim.header_boundary_ambiguous.v1` | error | mail-auth's header/body boundary differs from Swarmail's, for example a whitespace-only line in the header block | signed message whose boundaries agree | `" \r\n"` after the DKIM-Signature field; `" \r\n"` before it |
| `dkim.limits_exceeded.v1` | error | more than 16 fields, or more than 128 `h=` names | 16 valid fields; 128 names | 17 fields; 129 names |
| `dkim.signature_expired.v1` | error | `x` ≤ receipt, or a literal `x=0` | `x` = receipt + 1 | `x` = receipt; `x=0` with no `t=` |
| `dkim.expired_since_receipt.v1` | error | `x` after receipt but reached by the host clock (0125 §8) | `x` one day after signing, receipt at signing | receipt 1,500 s, `t` 1,000, `x` 2,000 |
| `dkim.selector_invalid.v1` | error | the asked name `<s>._domainkey.<d>.` is outside 0120's grammar; never looked up | `s=sel` | `s=bad*sel`; `d=example.com.` (trailing dot) |
| `dkim.key_missing.v1` | error | the key name is declared nowhere (`no_records`) | key declared | key not declared |
| `dkim.key_invalid.v1` | error | record not DKIM, `p=` missing or not base64, unsupported `k=`, or a key type not matching `a=` | valid `p=` | `p=!!!!`; an ed25519 key for `a=rsa-sha256` |
| `dkim.key_revoked.v1` | error | `p=` empty | valid `p=` | `v=DKIM1; p=` |
| `dkim.not_evaluated.v1` | error | the key name (or ATPS name) is declared `tempfail` | key declared | key name in `tempfail` |
| `dkim.body_hash_mismatch.v1` | error | body hash ≠ `bh=` | intact body | one body byte altered (both algorithms) |
| `dkim.signature_mismatch.v1` | error | the header hash does not verify against `b=` with the declared key, including a `p=` that is base64 but no key | intact headers | signed `Subject` altered (both algorithms); `p=` of base64 non-key bytes |
| `dkim.auid_mismatch.v1` | error | the `i=` domain is not `d=` or a subdomain of it (or a `t=s` key and a subdomain) | `i=@d` | `i=@other.example` |
| `dkim.atps_unauthorized.v1` | error | `atps=` matches a From domain, and the ATPS record is undeclared, empty or invalid | ATPS record declared | ATPS record undeclared; ATPS name declared with a junk value |
| `dkim.body_length_exceeds.v1` | error | `l` > max(raw, canonical) body length | `l` = raw body length | `l` = raw length + 10 |
| `dkim.body_length_partial.v1` | warning | verified, but `l=` covers only a canonical prefix (RFC 6376 §8.2) | `l` = raw body length | `l=` short, then text appended |

The table has 18 finding rules. A passing signature and an unsigned message emit
no `dkim.*` finding at any severity. No `dkim.pass`, `dkim.missing` or
`dkim.valid` id exists.

### Report and version

- The report shape `{rule_set_id, findings, dns}` is unchanged.
- `RULE_SET_ID` is `swarmail-html/6` everywhere it is spelled. Revision 1 adds
  bytes (one rule, more malformed cases) but **takes no new id**: `/6` has never
  shipped, because the PR is unmerged. Under 0121 §5 and 0122 §1.4, an
  unshipped id still owns its bytes.
- An unsigned message's report differs from `/5` only in `rule_set_id`.
- A signed message's `dns.lookups` lists each name mail-auth asked for, once,
  sorted, with its outcome (0120 §5, as amended by the integration note).
- The crate asks for a key only for a parsed signature that it judges
  unexpired and whose body hash matches.
  - Rows b, c, f, h and i record no lookup for their field.
  - Rows a, d, e and g (for example `x=0`, or a missing `a=`) can record the
    lookup the crate made. The report says so, and the verdict is still
    Swarmail's.
- A report with a `temp_fail` lookup still never qualifies (0120 §4).

### Configuration

`--dns-resolver real` stays refused before any listener binds, with the fixed
message
`real DNS is not supported; declare records with --dns-policy or PUT /api/v1/inboxes/{inbox}/dns`.

### Live surfaces

Every live text that describes the report must be true of this build:
- `src/static/openapi.json`:
  - the `DnsRecord`, route and `DeliverabilityReport` descriptions;
  - the route's "no network or ambient time input", which must name the 0125 §8
    exception;
- `src/static/llms.txt`;
- `docs/guide/deliverability.md`;
- `src/mcp.rs` `MODERN_REPORT_DESCRIPTION`;
- `docs/decisions/README.md` (the 0120 and 0125 rows).

The claims they make:
- DKIM records the names it asks for, and SPF and DMARC come later.
- The DKIM rule family is listed.
- Link rules are part of the current rule set's history, not `/4` "now".
- The malformed list names each tag in row a.
- `key_invalid` and `signature_mismatch` are worded as rows o, q and r.
- `atps_unauthorized` covers junk records.
- The `l=` overview matches rows s and t and the interop gap.

Historical version notes stay as written.

## Frozen guarantees and failure modes

| concern | answer |
|---|---|
| 0001 accept == stored | nothing in DKIM runs before or inside `Store::insert`; verification is lazy, after the `250`, from stored bytes |
| 0002 subscribe-before-check | the wait loop is untouched; a `temp_fail` lookup is never clean, so a wait reaches its existing deadline outcome |
| 0003 oldest-only pruning | verification holds the selected `Arc<Email>` like every collector; no slot, no retention exception |
| 0005 ingest extraction | the evaluator reads the committed bytes, and the boundary guard makes DKIM's body the committed body every collector sees; nothing is persisted |
| crash between accept and commit | no DKIM state exists to lose; the stored bytes are the only durable state, and the next report re-derives |
| a hanging webhook / the network | DKIM shares no task, client or permit with webhooks; declared lookups are map reads; the compiled resolver is unreachable (the adapter always answers, and there are no name servers) |
| per-inbox cap racing a burst | one `Arc<Effective>` policy snapshot per request, the policy in the cache key, the selected `Arc<Email>` held; pruning stays oldest-only; no lock across evaluation |
| chaos mid-pipeline | chaos acts at SMTP accept, at the poles (0020 §5); altered bytes give a named mismatch, never a pass; a crate panic fails one evaluation (`503`), never the process |
| hostile DATA | the 8 MiB, 64 KiB header and part bounds stand; the boundary guard, the pre-scan, the 16-field and 128-name bounds and the work bound above; malformation is a finding, not `Failed` |
| a cert expiring under a test | DKIM touches no TLS; the only time input Swarmail supplies is the immutable receipt; fixtures derive receipts from the signature's own `t`/`x`, so there is no sleep and no wall-clock assertion |
| clock | the crate's internal read only (0125 §8, [Operator holds](#operator-holds) item 2), with the blast radius stated in [Verification time](#verification-time) |
| 1,000 sessions | four report permits and the 128-slot cache unchanged; the [work bound](#work-bound) per cold report; a cache hit does no analysis |

## Return adjudication — 2026-11-05 (revision 1)

The review panel returned the design with `spec_defect: true`. The two contrarian
positions (simplicity, robustness) argued against those findings. The chief read
`mail-auth-0.14.0` and `aws-lc-sys-0.46.0` in the offline registry before ruling
on each disputed fact.

- **The clock (guarantees HIGH): both positions agree on the gate. Robustness's
  vendored patch is rejected.**
  - Facts: `verify_dkim_(.., now)` is `pub(crate)`, as are `DomainKey.p` and the
    public-key types. No public path verifies without the clock.
  - Vendoring or `[patch]` is refused by the coverage gate's local-path-package
    rule, and a git patch does not resolve offline. A re-implementation would be
    a second crypto owner.
  - Ruling: 0125 §8 becomes a proposed amendment of 0085 §8's ambient-time
    sentence for the crate-internal read. It states the full radius: every
    crate-mapped verdict, plus the lookups. Operator acceptance is a merge hold.
  - *Cost to robustness:* the residual ships behind a hold rather than being
    engineered away. If the operator refuses, the choice of `x=` scope (B or B′)
    is theirs.
- **The body boundary (security): simplicity's whole-message short-circuit is
  adopted over robustness's partial trust.**
  - The rule name is `dkim.header_boundary_ambiguous.v1`.
  - The guard runs only when Swarmail's headers or the crate list a DKIM field,
    so unsigned mail cannot gain a finding (AC1).
  - *Cost to robustness:* a boundary-ambiguous message gets no per-signature
    verdicts. Real receivers find it ambiguous too, so one "fix your header
    block" answer is right.
- **The pre-scan: simplicity's narrow set is adopted. Robustness's "fits `u64`"
  is added.**
  - The added checks are missing `a=` and non-numeric `t=`, `x=` and `l=`.
    Overflowing digits are malformed too, because the crate reads them as 0.
  - A literal `x=0` is a 1970 expiry, not "absent".
  - Robustness's own checks of `c=` grammar, base64 and the presence of `d`,
    `s`, `b`, `bh` and `h` are rejected. The crate already rejects each of those
    into an existing row.
  - *Cost:* a crate upgrade that stopped rejecting one of them would change a
    verdict. The exact pin and the per-rule fixtures make that loud.
- **Pairing: simplicity's `ptr::eq` identity with an `expect` is adopted. It
  deletes `first_parsed`.**
  - Robustness's malformed-ARC fixture is also adopted. It costs one test and
    proves the claim on mail the first suite never showed.
- **The asked name (G11): simplicity is adopted.** The verdict judges
  `signature.domain_key()` with the same `owner_of` the adapter uses, so
  `selector_invalid` and `key_missing` cannot diverge.
- **The work bound (G7): the honest bound is adopted.** Both positions offered
  it. Robustness's "at least as permissive" second scanner is rejected, because
  two scanners disagree, which is why implementation item 1 removed it.
  - *Cost:* the strict parse, at most four hashes over at most 8 MiB, runs
    before a bound refuses.
- **`l=` semantics: simplicity is adopted. There is no
  `dkim.body_length_semantics.v1`.** Robustness's minimum is adopted: the gap
  is named in rows s and t, the guide and 0125, with an instruction to #255.
  - *Cost to robustness:* an RFC-conformant `l=` signer whose raw and canonical
    prefixes differ gets `body_hash_mismatch`, a false accusation on a rare,
    discouraged tag (RFC 6376 §8.2). It is documented, not detected.
- **Docker (G10): simplicity's evidence is adopted.** `aws-lc-sys` 0.46.0 uses
  its cc builder unless the build is FIPS, `NO_ASM`, a sanitizer build or
  `CMAKE_BUILDER`, and it maps `*-alpine-linux-musl` onto shipped
  pregenerated bindings. The Alpine stage has a C compiler and `musl-dev`, so
  no speculative `apk` change is made.
  - Robustness's demand is adopted: if a daemon exists, run one bounded build.
    If CI's `docker smoke` fails, add exactly the missing package. A red
    required check is never merged.
- **AC11: both positions agree.** The await clause moves to #255 by name, and
  the wire proof asserts the DKIM ids over SMTP, REST and MCP.
- **AC12: simplicity's rewording is adopted. Robustness's new two-process test
  is rejected.**
  - *Cost:* cross-process statics are not proven. The only DKIM static is the
    stateless `LazyLock` authenticator.
- **The resolver: both positions agree.** It is an operator hold.
  - Robustness's no-cache proof is adopted conditionally (T-R8): kept only if
    the crate completes it on the first poll. A timed or racy test is never
    written.
- **No new id: both positions agree.** `/6` is unshipped.
- **Docs, the 0120 note and the CDLA comment: both positions agree.**
- **The load proof: optional.** Robustness's mandatory proof is rejected,
  because the work bound is structural. If one is run, it is capped at
  nproc/4 workers for 10 minutes (plan § Risks).

Findings that need no code change are answered here in text:
- the `name_of` hash-layout read, named in [Key source](#key-source-the-cache-adapter-crate-private);
- `Neutral(_) | None` grouped under row i, which identity pairing makes safe;
- the frozen-table counts, in [Work bound](#work-bound).

## Panel adjudication — 2026-11-05

*Dated history: the first panel's rulings. Revision 1 keeps every one. Where
the mechanism changed, the return adjudication above rules.*

- **Crypto backend: simplicity's `aws-lc-rs`.** mail-auth 0.14.0 has no `ring`
  feature, and a DNS backend is a `compile_error!` requirement.
- **Key source: simplicity.** `Lookups` is called inside the evaluator through
  a cache adapter.
- **Time: simplicity's receipt time.** Robustness's request instant plus cache
  bypass was rejected.
- **Parsing: mail-auth parses.** A narrow Swarmail pre-scan covers what the
  crate's public struct hides.
- **`l=`: robustness.** `body_length_exceeds` (error) and `body_length_partial`
  (warning).
- **Rules from robustness:** `from_unsigned`, `key_revoked`,
  `selector_invalid`, and `x` ≤ `t` as malformed.
- **Rules from simplicity:** `rsa-sha1` unsupported, `auid_mismatch`,
  `signature_mismatch`.
- **Chief's additions:** `atps_unauthorized`, `expired_since_receipt` and
  `limits_exceeded`.
- **Severities: simplicity.** Every unverified outcome is `error`.
- **Bounds: merged.** 16 signatures and 128 `h=` names. A bound is a finding,
  and nothing past it is evaluated.
- **Agreed by both:** no pass finding, no missing finding, no report member, no
  `AuthEvidence` change, real DNS refused.

## Acceptance Criteria

Each criterion names the proof that can fail it. AC1–AC16 keep their numbers.
Where revision 1 amends one, it says so. AC17–AC21 are new.

- **AC1: Unsigned reports unchanged.** A message with no `DKIM-Signature` gives
  exactly the previous report, with `rule_set_id` `swarmail-html/6` and empty
  `dns.lookups`. That includes an unsigned message whose header block contains a
  whitespace-only line. *Proof:* the existing exact-JSON tests in
  `tests/deliverability.rs`, `tests/bulk_sender.rs` and `tests/spam_rules.rs`,
  plus the inline unsigned-boundary fixture.
- **AC2: Both algorithms, all four canonicalizations, pass.** rsa-sha256
  (2,048-bit) and ed25519-sha256 signatures, each under `simple/simple`,
  `relaxed/relaxed`, `relaxed/simple` and `simple/relaxed`, with the key
  declared. Each gives no `dkim.*` finding and one `records` lookup for
  `<s>._domainkey.<d>`. *(Amended: aws-lc refuses to sign with 1,024 bits, so
  AC3's vector proves 1,024-bit RSA.)* *Proof:* the inline matrix test.
- **AC3: Published vector.** The RFC 8463 Appendix A signed message, with its
  A.2 keys declared, verifies both its signatures, including the 1,024-bit RSA
  one. *Proof:* inline test.
- **AC4: Canonicalization is real.** Folding a signed header and doubling inner
  body whitespace fails under `simple` (signature mismatch and body hash
  mismatch) and passes under `relaxed`. *Proof:* an inline test per mode.
- **AC5: Every rule has a passing and a failing fixture.** For each of the **18**
  rules, every failing fixture listed in the rule table yields exactly that rule
  id, severity and occurrence, and the passing fixture yields no finding of that
  id. Both algorithms are exercised for `body_hash_mismatch` and
  `signature_mismatch`. *Proof:* inline `dkim` tests, one pair per rule, and one
  case per listed failing fixture.
- **AC6: Precedence, independence and row coverage.** One message holds a
  passing signature, a malformed one before it and a failing one. It gives
  exactly two findings, at the correct occurrences. A field rejected by the
  parser (missing `bh=`) between two parsed fields does not shift later
  occurrences. Each row a–u, and each message-level step (3, 4, 5), is hit by at
  least one fixture. *Proof:* inline tests, plus exact branch coverage.
- **AC7: Receipt time, not wall time.** The same signed bytes give no finding
  with `received_ms` before `x`, and `signature_expired` with `received_ms` at
  `x`. Receipts are generated from the signature's own `t`/`x`, with no sleep and
  no wall-clock assertion. The residual fixture (`t` 1,000, `x` 2,000, receipt
  1,500 s; `x` in 1970, so the host clock is always past it) gives
  `expired_since_receipt`. The residual fixture also proves the radius: the same
  field with an **undeclared** key gives `expired_since_receipt`, not
  `key_missing`, and records no key lookup. *Proof:* inline tests.
- **AC8: `l=` (amended).** These hold:
  - Swarmail's canonicalizer is byte-equal to the crate's own `bh=` input. The
    oracle covers every body of up to six bytes drawn from `a`, space, tab, CR,
    LF and `0x01` (55,987 bodies), in both modes.
  - A signature with `l` = the raw body length is clean.
  - Text appended after a verified `l=` gives `body_length_partial` (warning).
  - `l` = raw length + 10 gives `body_length_exceeds`.
  - A literal `l=0` gives `unsupported`, and `l=abc` gives `malformed`.

  The RFC 6376 §3.4.5 interop gap is documented, not tested as a pass.
  *Proof:* inline tests.
- **AC9: The only key source, never the network.**
  - An undeclared key name yields `key_missing`, not a resolver error.
  - A `tempfail` name yields `not_evaluated`, records a `temp_fail` lookup, and
    `Report::qualifies` is false at every threshold.
  - An invalid owner name (`s=bad*sel`, and the trailing-dot `d=example.com.`)
    yields `selector_invalid` and records no lookup.
  - `r=y` on a failing signature records the `_report._domainkey.<d>` lookup.

  *Proof:* inline tests.
- **AC10: Bounds.** 16 valid signatures are all verified. 17 give exactly one
  `limits_exceeded` at occurrence 16, no other `dkim.*` finding and no lookup.
  129 `h=` names give the same at that field's occurrence, and 128 names are
  evaluated. *Proof:* inline tests asserting against the code constants.
- **AC11: Over the wire (amended).** A signed message is sent over real SMTP to
  an inbox whose key is declared with `PUT /api/v1/inboxes/{inbox}/dns`. Its
  REST and MCP reports have no `dkim.*` finding and record the key lookup. A
  second message with an altered body gives `dkim.body_hash_mismatch.v1` on
  both surfaces.
  - The first design's await clause cannot hold before #255: an await with
    `no_findings_at_or_above=warning` returns the signed message and not the
    altered one. The bulk checklist's `*.not_evaluated.v1` errors disqualify
    every report at every threshold until #255 supplies authentication
    evidence.
  - The clause **moves to #255 by name**. #255's spec inherits it as an
    acceptance criterion.

  *Proof:* `tests/deliverability.rs`.
- **AC12: Determinism (amended).** Two evaluations of the same signed `Email`
  give byte-identical report JSON. So do two in-process servers, with separate
  stores and caches, serving the same bytes and policy. Both hold outside the
  0125 §8 window. *Proof:* an inline test, plus the two-server test in
  `tests/deliverability.rs`.
- **AC13: Dependency.**
  - `Cargo.toml` carries the exact pin, `default-features = false`, and only
    `aws-lc-rs`.
  - `cargo deny check licenses` passes.
  - The `deny.toml` allowlist is unchanged. Its CDLA-Permissive-2.0 comment and
    house-rules § Dependencies name `webpki-roots` and `webpki-root-certs`.
  - The `native-tls` absence proof passes, and `evals/Cargo.lock` is unchanged.
  - 0125 §9 lists the added packages and their licences.
  - The existing real-HTTPS webhook and STARTTLS tests pass with both rustls
    providers compiled.

  *Proof:* the gates, `git diff`, and the existing TLS suites.
- **AC14: Real DNS still refused.** `--dns-resolver real` and
  `SWARMAIL_DNS_RESOLVER=real` exit non-zero with the fixed message before any
  listener binds. *Proof:* the existing refusal tests.
- **AC15: Documentation (amended).**
  - The guide documents each of the 18 rule ids with its severity and condition,
    as listed in [Live surfaces](#live-surfaces).
  - It documents the receipt-time rule, the clock residual with its full radius,
    the work bound, the known limits (the `l=` interop gap, the `_report`
    selector, the unreachable resolver), and the `swarmail-html/6` history
    entry.
  - Every live surface in [Live surfaces](#live-surfaces) is true.
  - The evaluator's header comment states the radius as
    [Verification time](#verification-time) does.

  *Proof:*
  - a test binding the guide's rule headings to `Rule`, 18 of them;
  - `grep -rn -e "No rule resolves a name" -e "until #254"` over `src/static`,
    `docs/guide` and `docs/decisions/README.md`, which must find nothing
    outside dated history;
  - review.
- **AC16: Gates.** Every house gate passes `--locked`:
  - fmt, clippy and tests;
  - the evals crate against the candidate binary;
  - the coverage gate at 100%, and its selftest;
  - publish-guard and seal-branch;
  - `cargo deny`.

  The brokkr bundle compile runs in verify if the sandbox has no brokkr. *Proof:*
  the gate outputs.
- **AC17: One body boundary (new; security).** A signed message whose boundaries
  agree is verified normally. Each of the following gives exactly one
  `dkim.header_boundary_ambiguous.v1` at occurrence 0, no other `dkim.*` finding,
  and no lookup:
  - a whitespace-only line `" \r\n"` after the DKIM-Signature field;
  - the same line before the field, where mail-auth sees no signature at all;
  - a message whose strict parse returns `None` while Swarmail sees a
    DKIM-Signature field.

  *Proof:* inline tests.
- **AC18: Required tags and numbers (new).** Each of these is
  `dkim.malformed.v1`, while the same signature with the tag present and numeric
  is clean:
  - a signature with no `a=` that verifies today as rsa-sha256;
  - `t=abc`, `x=abc` and `l=abc`;
  - an `x=` that overflows `u64`.

  A literal `x=0` with no `t=` is `signature_expired`. With a `t=`, it is
  `malformed`. *Proof:* inline tests.
- **AC19: Pairing by identity (new).** No output is selected by index. A passing
  signature placed beside malformed `ARC-Seal` and `ARC-Message-Signature`
  fields, and beside a parser-rejected DKIM-Signature, keeps its own verdict and
  occurrence. *Proof:* an inline test, plus `grep -n first_parsed
  src/deliverability/dkim.rs` empty.
- **AC20: Operator holds recorded (new).** 0125's header and the PR body list
  the three holds of [Operator holds](#operator-holds). 0125 §8 is worded as a
  proposed amendment of 0085 §8 with the radius stated here. 0125 §2 asks the
  resolver question. Accepted 0085 is not edited. *Proof:* review, plus
  `git diff fad9e7e -- docs/decisions/0085-deliverability-report.md` empty.
- **AC21: Records (new).**
  - Proposed 0120 carries a dated 2026-11-05 integration note covering §4/§5
    (production lookups are no longer empty), §7 (the live-answer semantics move
    with the resolver) and §10 (the resolver promise and the `Real` variant move
    to a later story with its own decision).
  - The README index rows for 0120 and 0125 match.
  - 0125's header says "with an integration note on proposed 0120".

  *Proof:* review, plus the AC15 grep.
