# 0125 — Offline DKIM verification on mail-auth, keyed by receipt time

- **Status:** proposed
- **Date:** 2026-11-05
- **Scope:** epic #251, issue #254 (operator direction: the remaining
  deliverability stories in the order #253, #254, #255). Run
  `issue-254-verify-dkim-signatures-bac57cf3`, from `main` `fad9e7e`, on the
  `claude-careful` recipe. The dispatcher assigned the number 0125. No other
  number is used.
- **Specification:** [offline DKIM verification](../../specs/dkim-verification/spec.md).
- **Authority:** the operator's #251 ruling of 2026-10-26: DKIM, SPF, DMARC
  and ARC are built on an existing permissive-licensed crate, widening the
  dependency allowlist once with that justification. This record takes that
  one widening, so #255 and #256 reuse it.
- **Builds on, without amending:** accepted
  [0085](0085-deliverability-report.md) (lazy, cached, bounded reports; the
  report shape is unchanged), proposed
  [0120](0120-deliverability-dns-policy.md) (the declared TXT map and its
  `Lookups` recorder are the only key source), proposed
  [0121](0121-deliverability-link-checks.md) §5 and proposed
  [0122](0122-deterministic-spam-rules.md) §1.4 (the next never-shipped
  `rule_set_id`).
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md) and 0005. Verification runs only inside
  the lazy report evaluator, on the stored bytes, after the `250`. Nothing
  runs on the SMTP path, at ingest, in `Store::insert`, at seed or restore,
  or in fan-out.
- **Held for the operator.** The delivery PR adds packages to `Cargo.lock`,
  so under 0056's 2026-11-01 addendum it is not merged on green. §8 asks the
  operator one question about the crate's clock.

## Decision proposed

1. **The crate and its features.**
   `mail-auth = { version = "=0.14.0", default-features = false, features = ["aws-lc-rs"] }`,
   added with `--locked`.
   - mail-auth 0.14.0 is `Apache-2.0 OR MIT`.
   - Its manifest has no `ring` feature. The `rust-crypto` set needs `rsa`
     0.9, which carries an unfixed timing advisory (RUSTSEC-2023-0071), and its
     crates are not in the offline registry. The implementation confirmed that the
     `rsa` crate is not in the locked tree (`cargo tree --locked -i rsa` finds no
     package). `aws-lc-rs` is the one backend that resolves offline
     and verifies RSA keys of 1,024 to 8,192 bits, which RFC 8301 requires a
     verifier to accept.
   - `arc`, `dns-doh` and `generate` stay off. #256 turns `arc` on when it
     needs it.
2. **The resolver is compiled in and never reached.** mail-auth refuses to
   build without a DNS backend (`compile_error!`), and `aws-lc-rs` turns on
   `dns-hickory`. The brief's "turn off any network resolver features"
   therefore cannot be met literally. This record does not claim the
   resolver is off. It guarantees it is unreachable, for two independent
   reasons:
   - every lookup goes through a per-evaluation cache adapter over 0120's
     `Lookups`, and the adapter answers every name it is asked, so mail-auth
     never calls its resolver; and
   - the one `MessageAuthenticator` is built with an empty name-server list,
     so even a miss would have nowhere to send a packet.
3. **One key source.** 0120's declared TXT map, through `Lookups`, is the
   only source of DKIM keys. Every name mail-auth asks for is recorded in
   the report's `dns.lookups`. That covers the key name
   `<s>._domainkey.<d>`, the RFC 6541 ATPS name, and the RFC 6651
   `_report._domainkey.<d>` name when `r=y` is set. Real DNS stays
   refused, and no `Real` policy variant is added. 0120 §10 said #254
   would bring the resolver; that promise moves to a later story with its
   own decision. 0120 §7's live-answer TTL, refresh and wake semantics move
   with it, because #254 ships no live answers. The fixed
   `--dns-resolver real` refusal text, which names #254, is corrected. 0120
   §5's "production `lookups` is always empty" was scoped to #253, and signed
   messages now record lookups.
4. **Verification time is the receipt time.** A signature is expired when its
   `x=` is at or before the stored message's receipt second
   (`max(received_ms, 0) / 1000`). The receipt time is immutable for each
   stored message, so the existing cache identity already covers it, and the
   verdict is a function of the bytes, the receipt time and the policy.
   `t=` in the future is not enforced. An `x=` at or before `t=` is
   malformed (RFC 6376 §3.5).
5. **A closed rule table.** One verdict for each `DKIM-Signature` field,
   located at `header{name:"dkim-signature", occurrence:k}`. Every outcome
   that leaves the signature unverified is `error`. One outcome is a
   `warning`: a verified signature whose `l=` covers only a prefix of the
   body. A pass emits nothing, and so does an unsigned message. The
   [specification](../../specs/dkim-verification/spec.md#rule-table) holds
   the normative table of 17 finding rules, each proven by a passing and a
   failing fixture. Swarmail owns these checks:
   - duplicate tags;
   - `rsa-sha1` and a literal `l=0` as unsupported;
   - `From` missing from `h=`;
   - `x=` against `t=` and against receipt;
   - `l=` against the canonicalized body length;
   - the bounds.

   mail-auth owns parsing, canonicalization, the hashes and the signature
   check.
6. **Bounds.** At most 16 `DKIM-Signature` fields per message, and at most
   128 `h=` names per signature. Past either bound, mail-auth is not
   called. The message gets one `dkim.limits_exceeded.v1` error finding and
   no other DKIM finding, and nothing is looked up. A report therefore
   carries at most 16 DKIM findings, well under 0085's 256.
7. **Version.** Signed messages' report bytes change, so `RULE_SET_ID`
   becomes `swarmail-html/6`, the next never-shipped id (0121 §5, 0122
   §1.4). The report shape `{rule_set_id, findings, dns}` is unchanged. No
   OpenAPI or MCP schema member is added, and `AuthEvidence` and the
   `bulk.*` rules are untouched (#255 owns them).
8. **The crate's clock: one named residual, and a question for the
   operator.** mail-auth 0.14.0 checks `x=` against `SystemTime::now()`, and
   its variant that takes a time is crate-private. Swarmail reads no clock.
   When `x=` is after receipt but at or before the wall clock, so a stored
   message has aged past its own `x=`, mail-auth returns "expired" without
   checking the signature. Swarmail then reports
   `dkim.expired_since_receipt.v1`. That finding is never a pass and never
   clean. The wall clock can move only that one verdict, and only from
   "verified" to "not verified". For a message in that window, a report
   recomputed after cache eviction can differ from the earlier cached one.
   That is the one stated exception to 0120 §5's byte-identical reports.
   Accepted 0085 §8 forbade ambient time in #252's collector. The operator
   is asked whether this crate-internal read is compatible with it. If it
   is not, the alternative is to vendor or re-implement mail-auth's verifier
   at exact coverage, which this record declines.
9. **Transitive additions and licences.** On the offline registry, `deny.toml`
   passes unchanged (`cargo deny check licenses`: `licenses ok`), so the "one
   widening" widens the dependency tree, not the licence allowlist. #142 forbids
   an unused allowance, so none is added. `native-tls` stays absent
   (`cargo tree --locked -i native-tls` finds no package), and
   `evals/Cargo.lock` is unchanged. `Cargo.lock` gains **58 packages**, read from
   the real lockfile with the licence each declares (the SPDX expression is the
   package's own; every conjunct and every alternative that `deny.toml` needs is
   already allowlisted, and `webpki-root-certs` is under the
   `CDLA-Permissive-2.0` allowance the tree already holds for `webpki-roots`):

   | package | version | licence |
   |---|---|---|
   | `async-trait` | 0.1.92 | MIT OR Apache-2.0 |
   | `aws-lc-rs` | 1.18.2 | ISC AND (Apache-2.0 OR ISC) |
   | `aws-lc-sys` | 0.46.0 | ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0) |
   | `bstr` | 1.13.1 | MIT OR Apache-2.0 |
   | `cmake` | 0.1.58 | MIT OR Apache-2.0 |
   | `combine` | 4.6.9 | MIT |
   | `core-foundation` | 0.9.4 | MIT OR Apache-2.0 |
   | `core-foundation` | 0.10.1 | MIT OR Apache-2.0 |
   | `core_detect` | 1.0.0 | MIT/Apache-2.0 |
   | `critical-section` | 1.2.0 | MIT OR Apache-2.0 |
   | `crossbeam-channel` | 0.5.17 | MIT OR Apache-2.0 |
   | `data-encoding` | 2.11.1 | MIT |
   | `dunce` | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 |
   | `encodify` | 1.0.2 | Apache-2.0 OR MIT |
   | `encoding_rs` | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause |
   | `fs_extra` | 1.3.0 | MIT |
   | `futures-io` | 0.3.34 | MIT OR Apache-2.0 |
   | `futures-macro` | 0.3.34 | MIT OR Apache-2.0 |
   | `hashify` | 0.3.1 | Apache-2.0 OR MIT |
   | `hickory-net` | 0.26.3 | MIT OR Apache-2.0 |
   | `hickory-proto` | 0.26.3 | MIT OR Apache-2.0 |
   | `hickory-resolver` | 0.26.3 | MIT OR Apache-2.0 |
   | `ipconfig` | 0.3.4 | MIT/Apache-2.0 |
   | `jni` | 0.22.4 | MIT OR Apache-2.0 |
   | `jni-macros` | 0.22.4 | MIT OR Apache-2.0 |
   | `jni-sys` | 0.4.1 | MIT OR Apache-2.0 |
   | `jni-sys-macros` | 0.4.1 | MIT OR Apache-2.0 |
   | `jobserver` | 0.1.35 | MIT OR Apache-2.0 |
   | `linked-hash-map` | 0.5.6 | MIT/Apache-2.0 |
   | `lru-cache` | 0.1.2 | MIT/Apache-2.0 |
   | `mail-auth` | 0.14.0 | Apache-2.0 OR MIT |
   | `mail-parser` | 1.0.0 | Apache-2.0 OR MIT |
   | `moka` | 0.12.16 | (MIT OR Apache-2.0) AND Apache-2.0 |
   | `multiversion_no_op` | 1.0.0 | Apache-2.0 OR MIT |
   | `ndk-context` | 0.1.1 | MIT OR Apache-2.0 |
   | `openssl-probe` | 0.2.1 | MIT OR Apache-2.0 |
   | `parking_lot` | 0.12.5 | MIT OR Apache-2.0 |
   | `portable-atomic` | 1.15.0 | Apache-2.0 OR MIT |
   | `prefix-trie` | 0.8.4 | MIT OR Apache-2.0 |
   | `resolv-conf` | 0.7.6 | MIT OR Apache-2.0 |
   | `rustc_version` | 0.4.1 | MIT OR Apache-2.0 |
   | `rustls-native-certs` | 0.8.5 | Apache-2.0 OR ISC OR MIT |
   | `rustls-platform-verifier` | 0.7.1 | MIT OR Apache-2.0 |
   | `rustls-platform-verifier-android` | 0.2.0 | MIT OR Apache-2.0 |
   | `schannel` | 0.1.30 | MIT |
   | `security-framework` | 3.7.0 | MIT OR Apache-2.0 |
   | `security-framework-sys` | 2.17.0 | MIT OR Apache-2.0 |
   | `semver` | 1.0.28 | MIT OR Apache-2.0 |
   | `simd_cesu8` | 1.2.0 | Apache-2.0 OR MIT |
   | `simdutf8` | 0.1.5 | MIT OR Apache-2.0 |
   | `similar` | 3.2.1 | Apache-2.0 |
   | `system-configuration` | 0.7.0 | MIT OR Apache-2.0 |
   | `system-configuration-sys` | 0.6.0 | MIT OR Apache-2.0 |
   | `tagptr` | 0.2.0 | MIT/Apache-2.0 |
   | `untrusted` | 0.7.1 | ISC |
   | `webpki-root-certs` | 1.0.11 | CDLA-Permissive-2.0 |
   | `widestring` | 1.2.1 | MIT OR Apache-2.0 |
   | `windows-registry` | 0.6.1 | MIT OR Apache-2.0 |

   The Windows, Android and Apple platform-verifier packages (`jni*`,
   `ndk-context`, `windows-registry`, `ipconfig`, `schannel`, `security-framework*`,
   `core-foundation*`, `system-configuration*`,
   `rustls-platform-verifier-android`) sit in the lockfile because Cargo records
   every target, but a Linux build does not compile them. The tree also holds a
   second `mail-parser` (1.0.0, the crate's) beside Swarmail's 0.11.9.
   `aws-lc-sys` builds C with CMake; the Linux build here succeeded with the
   toolchain present. The macOS, Alpine (`docker smoke`) and RustSec
   (`cargo deny check advisories` needs the advisory database, which this offline
   run cannot fetch) checks are CI-owned.
   - **Stop conditions.** The implementation seat stops and reports, and
     does not widen further, in any of these cases:
     - mail-auth cannot answer every lookup from the adapter without its
       resolver;
     - any added package carries a non-permissive licence;
     - `cargo deny` needs an allowlist change that the operator's one
       widening does not justify.

## Alternatives ruled and costs

The 2026-11-05 panel argued two positions. The specification's
[panel adjudication](../../specs/dkim-verification/spec.md#panel-adjudication--2026-11-05)
lists every ruling. The ones that shape this record:

- **`aws-lc-rs`** over robustness's `ring`-only set, because that feature
  does not exist in mail-auth 0.14.0. *Cost:* a C-built crypto library and a
  second rustls provider compiled into the tree. Every Swarmail TLS config
  already pins ring explicitly.
- **Receipt time** (simplicity) over a request instant plus a cache bypass
  for `x=`-bearing mail (robustness). *Cost:* the §8 residual window.
  *What it buys:* cached reports stay correct forever, with no per-request
  clock and no cache exception.
- **Lookups inside the evaluator** (simplicity) over resolving keys before
  `evaluate` into a snapshot (robustness). The declared map is a pure,
  synchronous read that cannot hang. 0120 §10's pre-evaluate phase exists
  for a live resolver, which this record does not add. *Cost:* the story
  that adds live DNS builds that phase from scratch.
- **mail-auth parses, and Swarmail pre-scans only what the crate's public
  struct cannot express** (duplicate tags, literal `l=0`, the `h=` count),
  over a full Swarmail envelope parser (robustness). *Cost:* a crate upgrade
  could change a verdict. The exact pin and a snapshot test for each rule
  make such an upgrade fail loudly.
- **Every unverified outcome is `error`** (simplicity) over warning and info
  tiers (robustness). *Cost:* a broken signature disqualifies a report at
  every threshold. Robustness's `info` for "not evaluated" would have been
  clean at `warning` in the §8 residual, which has no `temp_fail` lookup to
  catch it.
- **A bound is a finding, and nothing past it is evaluated.** This combines
  robustness's "never `Failed` for sender-shaped data" and its `h=` cap with
  simplicity's 16-signature bound. *Cost:* a message over a bound gets no
  per-signature verdicts.

## Proof and disposition

The specification's acceptance criteria and its paired tasks hold the
proofs:

- inline fixtures for every rule, both algorithms and all four `c=` pairs;
- a canonical-body oracle against mail-auth's own `bh=` (the implementation
  record, item 2, says why it is not `l=`);
- the published RFC 8463 Appendix A vector;
- a proof that no lookup misses the adapter;
- real SMTP and REST proofs in `tests/deliverability.rs`;
- every house gate, `--locked`, with exact coverage and `cargo deny`.

This record stays proposed. Only the operator accepts it.

## Implementation record (2026-11-05)

The implementation seat read `mail-auth` 0.14.0 again while building and found
that the design's account of the crate was right on every stop condition (it
answers every lookup from the adapter, no non-permissive licence, `deny.toml`
unchanged) and wrong in five details. Each is a refinement of this record's
mechanism, not of its decisions, and each is proved by a test.

1. **The crate's own view of the fields.** The specification had Swarmail scan
   the header block, then pair the crate's outputs with its own scan. The crate's
   scanner accepts names Swarmail's does not (`DKIM-Signature :`, a name folded
   across lines), so the pairing could misalign, and a count taken from
   Swarmail's scanner could be exceeded by fields the crate still hashes. The
   module therefore enumerates fields through the crate's public
   `AuthenticatedMessage::dkim_signatures()` and `errors()`, sorted by position in
   the message, so outputs and fields line up by construction and the signature
   `expect` the specification described is not needed. The bounds are taken from a
   strict parse first (`parse_with_opts(.., true)`), which hashes the body at most
   once per canonicalization of an `l=`-less signature and rejects every `l=`
   signature unhashed; only a message within the bounds is parsed again without
   strict when a field carries `l=`. The 16-signature count is the crate's count.
2. **`l=` is a raw-byte length in the crate.** Its signer writes `l=` as the raw
   body length and its verifier hashes the first `l=` raw bytes. The
   specification's oracle ("`canonical_len` equals the `l=` the signer writes")
   could therefore not hold. The rules are: `body_length_exceeds` when
   `l=` is longer than both the raw and the canonical body; `body_length_partial`
   when the canonicalized first `l=` raw bytes are shorter than the canonicalized
   body. The crate's `BodyHasher` is crate-private, so the canonicalizer is
   Swarmail's, mirroring the crate's observable output (bare LF, lone CR and its
   whitespace handling included). The oracle is a test that holds it byte-equal to
   the crate's own `bh=` over every body of up to six bytes drawn from
   `a`, space, tab, CR, LF and `0x01` (55,987 bodies, both modes). A signer that
   counts canonical bytes where the raw and canonical prefixes differ fails its
   body hash in the crate; that is the crate's behaviour and is documented.
3. **No key validation, and ed25519 failures are library errors.** The crate
   builds a key from any base64 bytes and learns that it is no key only when
   verification fails; the aws-lc ed25519 path words every failed check as a
   library error, not `FailedVerification`. The rows that told the two apart
   collapse: a key type that does not match `a=` (`IncompatibleAlgorithms`) is
   `key_invalid`, and any other failed check is `signature_mismatch`, for both
   algorithms. A `p=` that is not base64, a missing `p=`, an unsupported `k=` and
   a non-DKIM record are still `key_invalid`; a `p=` that is base64 but no key is
   `signature_mismatch` (a test pins this). The `signature_mismatch` hint names
   all three causes.
4. **A signature with no `v=` is malformed.** The crate reads a missing `v=` as
   version 0 and goes on to verify. RFC 6376 section 3.5 requires `v=1`, so the
   pre-scan adds that case to `malformed`.
5. **1,024-bit RSA is proved by the published vector.** aws-lc refuses to sign
   with a 1,024-bit key (`TooSmall`) but verifies one, so the kit signs with
   RSA-2048 and ed25519, and the RFC 8463 appendix A message (whose `test` key is
   1,024 bits) proves the 1,024-bit path. The vector verifies both of its
   signatures as published; no erratum was found. `a=rsa-sha1` cannot be "correctly
   signed" with this crate (only `RsaKey<Sha256>` signs), so that fixture rewrites
   the algorithm tag of a signed header; the verdict is Swarmail's and comes before
   the crypto.

**Two things the specification asked for that no code can deliver yet.** AC11's
await clause (an await with `no_findings_at_or_above=warning` returns the signed
message and not the altered one) cannot hold while the bulk checklist's three
`*.not_evaluated.v1` rules are errors on every report: they disqualify every
message at every threshold until #255 supplies authentication evidence. The wire
proof therefore reads the report itself (no DKIM finding and the recorded lookup
for the signed message; `dkim.body_hash_mismatch.v1` for the altered one), and the
guide says to assert on DKIM rule ids until #255. The second is the specification's
`OwnerName`-based adapter: `ResolverCache::get` is generic over the borrowed key,
so the adapter reads the name back from the key's `Hash` output (the bytes of the
`str`, then `0xff`); a test holds that to the real lookups, and any other key
shape reads as no name and answers "not found".

**The operator's question in section 8 stands.** The wall-clock read inside
`verify_dkim` is the one residual; if it is not compatible with accepted 0085
section 8, the alternative named there is the only one.

This record stays proposed. Only the operator accepts it.
