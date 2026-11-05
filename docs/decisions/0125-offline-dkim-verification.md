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
     0.9, which carries an unfixed timing advisory (RUSTSEC-2023-0071, to be
     confirmed by the implementation seat), and its crates are not in the
     offline registry. `aws-lc-rs` is the one backend that resolves offline
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
   passes unchanged, so the "one widening" widens the dependency tree, not
   the licence allowlist. #142 forbids an unused allowance, so none is
   added. The tree gains about 58 `Cargo.lock` packages:
   - aws-lc-rs and aws-lc-sys (`ISC AND (Apache-2.0 OR ISC) AND Apache-2.0
     AND MIT AND BSD-3-Clause AND …`, every conjunct allowlisted);
   - hickory-resolver, hickory-proto and hickory-net;
   - rustls-platform-verifier and rustls-native-certs;
   - a second mail-parser, 1.0.0 beside our 0.11.9;
   - their helpers.

   `native-tls` stays absent, and `evals/Cargo.lock` is unchanged. The
   implementation seat replaces this paragraph's summary with the exact
   list and each licence, read from its real lockfile.
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
- a canonical-length oracle against mail-auth's own `l=`;
- the published RFC 8463 Appendix A vector;
- a proof that no lookup misses the adapter;
- real SMTP and REST proofs in `tests/deliverability.rs`;
- every house gate, `--locked`, with exact coverage and `cargo deny`.

This record stays proposed. Only the operator accepts it.
