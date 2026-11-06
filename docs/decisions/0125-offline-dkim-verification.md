# 0125 — Offline DKIM verification on mail-auth, keyed by receipt time

- **Status:** proposed; its three operator holds (§2 the compiled-in
  resolver, §8 the crate's clock as an amendment of accepted 0085 §8, §9 the
  `Cargo.lock` additions) were accepted by the operator on 2026-11-06 (see the
  dated acceptance note at the end)
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
- **Builds on:** accepted [0085](0085-deliverability-report.md) (lazy,
  cached, bounded reports; the report shape is unchanged; §8 below proposes an
  amendment of its ambient-time sentence and is not compatible work until the
  operator accepts it); proposed
  [0120](0120-deliverability-dns-policy.md), **with an integration note on
  proposed 0120** dated 2026-11-05 (the declared TXT map and its `Lookups`
  recorder are the only key source; §3 below moves 0120 §5's empty-lookups
  scope, §7's live-answer semantics and §10's resolver promise); proposed
  [0121](0121-deliverability-link-checks.md) §5 and proposed
  [0122](0122-deterministic-spam-rules.md) §1.4 (the next never-shipped
  `rule_set_id`).
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md) and 0005. Verification runs only inside
  the lazy report evaluator, on the stored bytes, after the `250`. Nothing
  runs on the SMTP path, at ingest, in `Store::insert`, at seed or restore,
  or in fan-out.
- **Held for the operator: three named holds.** The delivery PR is not
  merged on green:
  1. it adds packages to `Cargo.lock` and changes a `deny.toml` comment
     (0056's 2026-11-01 addendum);
  2. §8, the crate's clock, is a proposed amendment of accepted 0085 §8, and
     under 0045 §8 it waits for the operator's acceptance;
  3. §2, the resolver is compiled in: the operator accepts this deviation from
     the brief, or rules otherwise.

  *(2026-11-05, return revision 3.)* The three holds are **one merge
  question**. The delivery merges as built only when all three are resolved.
  If the operator refuses any one, the delivery does not merge, and that
  ruling starts a new design return. They stay three items because they answer
  to three authorities.

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

   **Operator question (hold 3).** Does a compiled-in but unreachable
   resolver satisfy the brief's "turn off any network resolver features"? No
   mail-auth 0.14.0 feature set builds without one. If the answer is no, the
   remedy is a different crate or an upstream change, which is a scope ruling
   this record does not make.
3. **One key source.** 0120's declared TXT map, through `Lookups`, is the
   only source of DKIM keys. Every name inside 0120's grammar that mail-auth
   asks for is recorded in the report's `dns.lookups`: the key name
   `<s>._domainkey.<d>`, the RFC 6541 ATPS name, and the RFC 6651
   `_report._domainkey.<d>` name when `r=y` is set. A name outside the grammar
   is not looked up, not recorded, and fails closed. *(Narrowed in return
   revision 2.)* Which names a given field causes is the crate's choice, not a
   promise of this record; the specification lists the absences Swarmail
   guarantees and the measured presences its tests pin. Real DNS stays
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
   the normative table of **18** finding rules (revision 1 adds
   `dkim.header_boundary_ambiguous.v1`), each proven by a passing and a
   failing fixture. Swarmail owns every check that the crate would otherwise
   settle by a silent default:
   - one body boundary and one set of DKIM fields: a signed message whose
     mail-auth body offset differs from Swarmail's, whose strict parse fails,
     or whose DKIM-Signature field count differs from Swarmail's root count
     (return revision 2) gets one boundary finding and nothing else, so a
     finding's occurrence is always the root occurrence. The boundary finding
     itself is at occurrence 0, or at `null` when the root list holds no
     DKIM-Signature field (return revision 3);
   - duplicate tags, a missing `v=` or `a=`, and non-numeric or overflowing
     `t=`/`x=`/`l=` as malformed (the crate defaults `a=` to rsa-sha256 and
     reads bad numbers as 0);
   - `rsa-sha1` and a literal `l=0` as unsupported;
   - `From` missing from `h=`;
   - `x=` (a literal `x=0` included) against `t=` and against receipt;
   - `l=` against the raw and canonicalized body lengths;
   - pairing each crate output to its field by signature identity, never by
     index;
   - judging the exact key name the crate asked for;
   - the bounds.

   mail-auth owns parsing, canonicalization, the hashes and the signature
   check.
6. **Bounds and the real work bound.** At most 16 `DKIM-Signature` fields
   per message, and at most 128 `h=` names per signature, counted from the
   crate's strict parse. That parse runs first: it hashes the body at most
   once per distinct (canonicalization, hash) pair of an `l=`-less
   signature, so at most four hashes over at most 8 MiB, and rejects every
   `l=` signature unhashed. Past the boundary guard or either bound,
   `verify_dkim` is not called, nothing is looked up, and the message gets
   one error finding and no other DKIM finding. Within the bounds: at most one
   re-parse (at most 16 more hashes), 16 verifications, and at most three
   names per signature (key, ATPS, `_report`), so 48 in all. A report carries
   at most 16 DKIM findings, well under 0085's 256.
7. **Version.** Signed messages' report bytes change, so `RULE_SET_ID`
   becomes `swarmail-html/6`, the next never-shipped id (0121 §5, 0122
   §1.4). The report shape `{rule_set_id, findings, dns}` is unchanged. No
   OpenAPI or MCP schema member is added, and `AuthEvidence` and the
   `bulk.*` rules are untouched (#255 owns them).
8. **The crate's clock: a proposed amendment of accepted 0085 §8 (hold 2).**
   mail-auth 0.14.0 checks `x=` against `SystemTime::now()` inside
   `verify_dkim`. No public path verifies with a given time or a parsed key:
   `verify_dkim_(.., now)`, `DomainKey.p` and the public-key types are
   `pub(crate)`. Swarmail itself reads no clock.

   *The true blast radius.* It applies only to a parsed signature that
   Swarmail's own rows let through, so one carrying a numeric `x=` after the
   receipt second and, when `t=` is present, after `t=` too (an `x=` after
   receipt with no `t=` is inside the radius). Once the host clock reaches that `x=`, the
   crate returns "expired" before it checks the body hash, fetches the key,
   checks the AUID or verifies `b=`.
   - Every verdict the crate would have decided for that field becomes
     `dkim.expired_since_receipt.v1`, an `error`: a pass, the `l=` partial
     warning, body_length_exceeds, body_hash_mismatch, not_evaluated,
     atps_unauthorized, selector_invalid, key_missing, key_revoked,
     key_invalid, auid_mismatch and signature_mismatch.
   - The field's key and ATPS lookups disappear from `dns.lookups`.
   - So a clean or warning-only report can begin to disqualify, and a report
     recomputed after cache eviction can differ from the cached one. This is
     the one exception to 0120 §5's byte-identical reports.
   - The move is one-way unless the host clock is set back.
   - Swarmail's own rows, the message-level steps and every unsigned message
     are clock-free.

   *Every crate clock read, reachable or not (2026-11-05, return revision
   3).* `verify_dkim` reads `SystemTime::now()` once per call
   (`dkim/verify.rs:84`) and uses it only to compare `x=`. That is the one
   **reachable** read, with the radius above. mail-auth 0.14.0 has one more
   read on this path: `is_within_pct` (`sampling.rs:15-26`), which reads
   `SystemTime::now()` and a thread-local counter to sample RFC 6651 reports
   for a non-passing `r=y` signature whose `_report._domainkey.<d>` record has
   `rp<100`. Swarmail's cache adapter never answers with a report record.
   `txt_lookup::<DkimReportRecord>` then unwraps every cached answer to `Err`,
   and the report loop `continue`s before the sample
   (`dkim/verify.rs:222-236`). The sampling read is therefore **unreachable**.
   The `_report` name is still looked up and recorded, and no verdict changes,
   because `report_address` is unused. No resolver path runs, so no
   resolver-side time read is reached either.

   *The amendment proposed.* Accepted 0085 §8 says no ambient time is
   introduced into the collector. This record proposes that the sentence does
   not cover this one reachable crate-internal `x=` read, bounded as stated
   above. It covers no other read, because no other read is reachable. The
   record does not decide that the read is compatible. Under 0045 §8 the
   delivery PR waits for the operator's acceptance of this section.

   *If the operator refuses,* the choice is a scope ruling, and neither option
   is built here:
   - **B:** exclude every signature whose `x=` is after the receipt second
     from the crate's verification (the public `retain_dkim_signatures`), and
     give it one deterministic error id. This is deterministic, but such
     signatures go unverified.
   - **B′:** wait for an upstream mail-auth API that takes the verification
     time.

   Two options are ruled out:
   - vendoring or `[patch]`ing the crate, because the coverage gate refuses a
     local path package (house rules § Gates, rule 6) and a git patch does not
     resolve offline;
   - re-implementing the verifier, because that makes a second owner of the
     crypto verdict.
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

**The operator's question in section 8 stands.** Revision 1 below
supersedes this paragraph's wording.

Items 1–5 and the two undeliverable clauses were **folded into
specs/dkim-verification/spec.md on return, 2026-11-05**. That file is now the
single normative account; this record is kept as dated history.

## Return revision (2026-11-05, review return 1)

The review panel returned the design (`spec_defect: true`):
- a HIGH finding that §8 understated the clock's radius against accepted
  0085 §8;
- a security residual: mail-auth's and Swarmail's header/body boundaries can
  differ;
- several places where the crate's defaults decided a verdict: a missing
  `a=`, non-numeric `x=`/`t=`/`l=`, index-based output pairing, and the
  reconstructed key name.

The chief's revision, argued by both panel positions, is in the
[specification's return adjudication](../../specs/dkim-verification/spec.md#return-adjudication--2026-11-05-revision-1).
In this record:
- §5 now lists 18 rules and the checks Swarmail took back from the crate's
  defaults;
- §6 states the real work bound;
- §8 is redrafted as a proposed amendment of 0085 §8, with the full radius
  and the operator's options;
- §2 asks the resolver question;
- the header names the three holds and the integration note on proposed
  0120.

`rule_set_id` stays `swarmail-html/6`, because it has never shipped. No
dependency, feature or allowlist entry changes. Only the `deny.toml` comment
on CDLA-Permissive-2.0 changes: it now also covers `webpki-root-certs`.

This record stays proposed. Only the operator accepts it.

## Implementation of the return revision (2026-11-05)

The implementation seat brought the branch into line with the revised
specification. Evidence, and the three places where reading the crate showed
the specification's text to be loose:

1. **The boundary guard's real trigger.** `mail-auth` 0.14.0 does **not** end
   its headers at a whitespace-only line inside the block: such a line starts
   with a space, so the crate folds it into the previous field exactly as
   Swarmail does (`headers/parse.rs`, `scan_value`). The two parsers differ
   instead at (a) a CRLF line that holds no colon and does not start with a
   space or tab, which the crate takes as the end of the headers and Swarmail
   skips, and (b) a whitespace-only *first* line, after which the crate lists
   no header at all (`parse_with_opts` returns `None`). The guard compares
   `body_offset()` with Swarmail's offset, so it catches both without naming
   either. The fixtures are therefore a `stray line` CRLF line after the
   signature field, the same line before it, and a leading `" \r\n"`; a test
   records that the specification's `" \r\n"` inside the block gives agreeing
   offsets and no finding. The hint names the real triggers.
2. **`owner_of` strips no dot of its own.** `OwnerName::parse` already strips
   exactly one trailing dot, so a second strip in `owner_of` would accept the
   crate's `sel._domainkey.example.com..` for `d=example.com.`. `owner_of` is
   `OwnerName::parse` under one name that the adapter and row l both call, and
   row l hands it `signature.domain_key()` unchanged. The first design's
   reconstruction (`format!("{s}._domainkey.{d}")`, no trailing dot) was the
   divergence; a test asserts `d=example.com.` gives `selector_invalid` and no
   lookup.
3. **Overflowing numbers.** The crate saturates an overflowing `number()` to
   `u64::MAX`; it does not read it as 0. Row a refuses it as malformed either
   way, so no verdict depends on the difference.

Other evidence:
- the empty-name-server proof (spec T-R8): `AUTHENTICATOR.verify_dkim` with
  **no** cache completes on the first poll with a `TempError` or `PermError`, so
  `resolver_without_nameservers_answers_nothing` is kept (no timer, no race);
  `adapter_outcomes` remains the proof that the adapter never misses;
- the over-the-wire test reads the signed and the altered message over real
  SMTP, REST and MCP;
- the lockfile, `deny.toml` allowlist and `evals/Cargo.lock` are unchanged since
  `adeaa12`; `deny.toml` differs from `fad9e7e` in the CDLA comment only; accepted
  0085 is unchanged; `cargo tree -i native-tls` finds no package;
- Docker: this sandbox has the client but no daemon (`/var/run/docker.sock`
  absent), so no image was built. CI's `docker smoke` is the named proof, per the
  spec's risk table; no `apk` change is made speculatively.

Items 1–3 above were **folded into specs/dkim-verification/spec.md on return
2, 2026-11-05** (step 4, the rule table, AC17, § Key source and row a).

This record stays proposed. Only the operator accepts it.

## Return revision 2 (2026-11-05, review return 2)

The review panel returned the design again (`spec_defect: true`). Four of five
members found the code right and the specification stale: items 1–3 of the
implementation of the return revision, above, were never folded into the specification. The
chief's revision is in the
[specification's revision 2 adjudication](../../specs/dkim-verification/spec.md#return-adjudication--2026-11-05-revision-2).
In this record:
- §3 is narrowed to names inside 0120's grammar, and no longer reads as a
  per-field promise of which names are asked;
- §5's boundary bullet gains the DKIM field-count condition, which makes a
  finding's occurrence the root `dkim-signature` occurrence and closes the
  case where Swarmail sees a signature the crate does not list;
- **#255's intake inherits AC11's await clause as a required acceptance
  criterion** (an await with `no_findings_at_or_above=warning` returns the
  correctly signed message and not the altered one). It cannot hold here
  while the bulk `*.not_evaluated.v1` errors disqualify every report; it is a
  handoff, not a deletion.

The three operator holds stand unchanged, and this revision does not argue
them: the clock (§8) against accepted 0085 §8, the compiled resolver (§2), and
the `Cargo.lock` additions. `rule_set_id` stays `swarmail-html/6` (unshipped).
No dependency, feature or allowlist entry changes.

This record stays proposed. Only the operator accepts it.

## Implementation of return revision 2 (2026-11-05)

The implementation seat brought the branch into line with the revision 2
specification. No ruling text above changes. Evidence:

1. **The premise held.** `dkim::tests::crate_lists_every_root_dkim_field`
   covers every spelling of the field name that the shared `headers()` scanner
   names `dkim-signature` (`DKIM-Signature`, `dkim-signature`, `DKIM-SIGNATURE`,
   `dKiM-sIgNaTuRe`), and a kit-signed value, a value without `bh=`, an empty
   value and a value folded across lines. In every case the crate's strict
   parse (its `dkim_signatures()` plus the filtered `errors()`) lists exactly the
   field the root list holds, and a message that carries all of them keeps the
   same count and order. It passed on the first run, so the guard was not
   weakened and the shared scanner was not touched.
2. **The conjunct.** `collect` now requires the crate's field count to equal
   the root `dkim-signature` count beside the body-offset equality. A mismatch is
   the existing `dkim.header_boundary_ambiguous.v1` on occurrence 0, with no
   verification and no lookup. The hint names both conditions. Seen to fail
   before the change: `boundary_dkim_field_name_with_space_is_ambiguous` (a valid
   signature beside a `DKIM-Signature :` field, before it and after it) found no
   finding at all, because both fields verified. The root scanner accepts only
   names of printable bytes without a space, so such a field is a signature to
   the crate and no field to Swarmail; the offsets agree, only the counts differ.
3. **Occurrence is the root occurrence.** `finding_occurrence_is_the_root_occurrence`
   computes the expected index from the root list for a message with a valid
   field, a parser-rejected field and a field signed over another body.
4. **Wrong key, same type, both algorithms.** `rule_signature_mismatch` now
   declares RFC 8463's published ed25519 key for the kit's ed25519 signature and
   its published 1,024-bit RSA key for the kit's RSA-2048 signature. Each gives
   exactly `dkim.signature_mismatch.v1` and one recorded lookup. The two
   vacuous passes under a failure comment are gone; the cross-type case stays
   `dkim.key_invalid.v1`.
5. **Measured lookups pinned.** `x_zero_with_t_is_malformed` asserts the one
   `sel._domainkey.example.com` `records` lookup. The unsigned, boundary,
   bounds and row-h cases already assert an empty `dns.lookups`.
6. **No calendar expiry in a passing fixture.**
   `forged_signatures_verify_for_both_algorithms` derives `x` from the signer's
   `t`; no `x=9000000000` remains.
7. **Live surfaces.** The guide and `llms.txt` no longer say that no
   authentication verification ships; the MCP description follows the OpenAPI
   wording; the occurrence sentence says "root DKIM-Signature"; the `dns.rs`,
   `Cargo.toml` and `deny.toml` comments are corrected (the last in comment
   lines only); the `--dns-resolver` help text no longer promises #254 a
   resolver.

Gates, all `--locked`, all green except one that cannot run here: fmt, clippy,
`cargo test --workspace`, the `swarmail` binary build, the evals fmt, clippy and
tests (441 passed) against that binary, the coverage gate at exact 100% (lines
28,595, branches 2,062, functions 2,888) and its selftest, publish-guard,
seal-branch, `cargo deny check licenses`, and `native-tls` absent from the
tree. This sandbox has no `brokkr` binary: the bundle compile runs in verify.
`Cargo.lock`, `evals/Cargo.lock` and the `deny.toml` allowlist are unchanged
since `38fd111`; accepted 0085 is unchanged. No load proof was run.

The three operator holds stand: the clock (§8), the compiled resolver (§2), and
the `Cargo.lock` additions. #255's intake inherits AC11's await clause.

This record stays proposed. Only the operator accepts it.

## Return revision 3 (2026-11-05, review return 3)

The review panel returned the design a third time (`spec_defect: true` from
security and independent). Every member found 0001–0003 and 0005 intact and no
security hold. The chief's revision is in the
[specification's revision 3 adjudication](../../specs/dkim-verification/spec.md#return-adjudication--2026-11-05-revision-3).
In this record:
- §8 states every crate clock read on the verification path. The
  report-sampling read (`is_within_pct`) is made unreachable: the cache adapter
  no longer types any answer as an RFC 6651 report record. The amendment
  therefore covers exactly one reachable read, the `x=` comparison. One side
  effect, a correction: a selector literally named `_report` is now read as
  the key record it is, not failed closed as `key_invalid`;
- §5's boundary finding is located at occurrence 0 when the root list holds a
  DKIM-Signature field, and at `null` (0085 §1's missing field) when it holds
  none;
- the header says the three holds are one merge question;
- the implementation adds a dated integration note on proposed 0121 §7, which
  still says the evaluator is free of clock and resolver access.

No rule id, schema member, dependency, feature or allowlist entry changes.
`rule_set_id` stays `swarmail-html/6`, which has never shipped. The three
operator holds stand.

This record stays proposed. Only the operator accepts it.

## Implementation of return revision 3 (2026-11-05)

The implementation seat brought the branch into line with the revision 3
specification. No ruling text above changes. Evidence:

1. **The report sample is unreachable.** `record()` lost its
   `_report._domainkey.` arm and `DkimReportRecord` left the `mail_auth` import;
   `grep -rn "DkimReport" src/` is empty. The adapter's `answer` still records the
   name before it parses, so `dns.lookups` bytes do not move.
2. **Measured variants.** `DomainKey::parse_records` gives
   `Error(Dns(InvalidRecordType))` for both `v=DKIM1; ra=postmaster` and
   `nonsense`; `adapter_answers_without_resolver` pins both and names no report
   variant.
3. **The `rp=0` leg.** `failure_reports_ask_for_the_report_name` declares
   `_report._domainkey.example.com` with `rp=100` and with `rp=0`. Each gives
   exactly one `dkim.body_hash_mismatch.v1` at occurrence 0 and exactly one
   `records` lookup of that name, and two evaluations of each are
   byte-identical. (The two legs' reports differ from each other only by the
   declared policy they echo.)
4. **`s=_report` before and after.** `report_selector_is_a_key` signs with the
   selector `_report` and declares the key at `_report._domainkey.example.com`.
   Before the arm was deleted it gave `dkim.key_invalid.v1`; after, no `dkim.*`
   finding and one `records` lookup.
5. **The `null` location.** `Rule::located(Option<u64>)` holds the finding body;
   the boundary guard passes `(root_count > 0).then_some(0)`.
   `boundary_lone_spaced_field_is_ambiguous_at_null` (a lone `DKIM-Signature :`
   field) was seen to fail with `Some(0)` before the change; it now finds exactly
   one `dkim.header_boundary_ambiguous.v1` at `occurrence: None` and no lookup.
   The same case is a failing fixture of `rule_header_boundary_ambiguous`, and
   every older boundary fixture keeps occurrence 0.
6. **Evidence that fails on drift.** `crate_lists_every_root_dkim_field` asserts
   list equality by unfolded value, with the empty value between two parseable
   fields; `strict_and_full_parses_list_the_same_fields` asserts equal length and
   equal raw offsets between the strict and the full parse; the wire test's
   `nokey`, `tempfail` and MCP legs assert whole findings and whole lookups
   arrays, and its `lookups == []` carries a comment marking it a measured ask
   order on the pinned version.
7. **Surfaces.** The guide, `llms.txt` and the OpenAPI description state the
   `null` case; the guide's clock paragraph names the unreachable sampling read and
   the await warning, and its known limits lost the `_report` clause. A dated
   integration note was appended to proposed 0121 (§7's text is not edited).

`Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are unchanged since `8241489`,
and `rule_set_id` stays `swarmail-html/6`. The three operator holds stand as one
merge question; #255's intake inherits AC11's await clause.

This record stays proposed. Only the operator accepts it.

## Dated acceptance note — 2026-11-06

The operator ruled on 2026-11-06, in chat to the dispatcher, choosing
"Accept all, fix docs" for the three holds named in the header. The question
put to them stated each hold: `Cargo.lock` gains 58 packages, including
`aws-lc-sys` (C crypto built with `cmake`) and `hickory-resolver`, with the
licence allowlist unchanged (§9 and 0056's 2026-11-01 addendum); mail-auth's
one reachable `SystemTime::now()` read for `x=`, whose radius is §8's and is
the one exception to byte-identical reports (§8, amending accepted 0085 §8);
and the compiled-in, twice-guarded, unreachable resolver (§2). The ruling:

1. §9 is accepted: the `Cargo.lock` additions and the `deny.toml` comment
   change may merge.
2. §8 is accepted as the amendment of accepted 0085 §8 it proposes, bounded
   as stated there; 0085 carries a dated pointer to this note. This lifts the
   0045 §8 hold on #254.
3. §2 is accepted: a compiled-in resolver that is unreachable by the two
   stated means satisfies the brief's "turn off any network resolver
   features" for mail-auth 0.14.0.

The same ruling asked for the review's four low documentation findings to be
fixed before merge: the in-grammar qualifier on recorded names in `llms.txt`,
the OpenAPI `DnsRecord` description and the guide, and §8's radius naming the
no-`t=` case (also in the guide and the specification). The rest of this
record stays proposed. CI remains the first proof of the Alpine and macOS
`aws-lc-sys` builds (§9).

## Integration note — 2026-11-06 (proposed 0126, issue #255)

[0126](0126-spf-dmarc-alignment.md) builds on this record and changes no
status; the record stays as the operator left it above.

- **DMARC consumes §6's verdicts, not the crate's raw results.** The crate
  outputs that reach DMARC's alignment are exactly those whose field's
  verdict here is a pass, `dkim.body_length_partial.v1` (it verified) or
  `dkim.not_evaluated.v1` (a declared temporary failure, which leaves the
  alignment Unknown). Every other verdict drops its output, so a signature
  this record refuses (an unsigned `From`, `rsa-sha1`, an expired or
  mismatched signature) never aligns, and an unverified `d=` equal to the From
  domain never aligns.
- **The shape of `collect`.** The single entry point is split into `prepare`
  (the strict parse and the boundary and bound guards, which still produce
  their one finding and stop) and `verify` (the verdicts, unchanged), so the
  parse and the crate's outputs can be handed to DMARC while the cache adapter
  is alive. No verdict, finding, bound or lookup changed.
- **§8, the clock.** SPF adds one more reachable ambient read, the monotonic
  lookup timer of `check_host`, proposed as a further amendment of accepted
  0085 §8 in 0126 §8 and held for the operator. The `x=` read is unchanged.
- **The inherited await clause.** AC11's deferred clause is delivered by 0126
  §10: over real SMTP a signed message that passes SPF, DKIM and DMARC and
  satisfies the whole checklist is returned by an await with
  `no_findings_at_or_above=warning`, and an altered copy is not.
