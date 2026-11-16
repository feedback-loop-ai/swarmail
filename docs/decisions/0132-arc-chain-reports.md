# 0132 — ARC chain validation in the deliverability report, on mail-auth

- **Status:** proposed
- **Date:** 2026-11-16
- **Scope:** epic #251, issue #256, the epic's last story. Filed P3 ("only if
  a need is shown"); the operator ruled on 2026-11-16 to deliver it. Run
  `issue-256-report-arc-chains-part-b186e59e`, from `main` `f05f2ee`. 0132 is
  the next free number after 0131.
- **Specification:** [ARC chain reports](../../specs/arc-chains/spec.md).
- **Authority:** the operator's #251 ruling of 2026-10-26 (authentication on
  an existing permissive crate, one widening, taken by 0125 for #254, #255
  and #256), and the operator's 2026-11-16 ruling to deliver #256.
- **Builds on:** accepted [0085](0085-deliverability-report.md) (lazy,
  cached, bounded reports); accepted
  [0120](0120-deliverability-dns-policy.md) (the declared TXT map and its
  `Lookups` recorder are the only key source); accepted
  [0122](0122-deterministic-spam-rules.md) (section 1.4, the next
  never-shipped `rule_set_id`; section 2, weights, with no new exception);
  accepted [0125](0125-offline-dkim-verification.md) (the crate, the adapter,
  the receipt second, the boundary guard, and the rule that Swarmail owns
  every check the crate settles by a silent default); accepted
  [0126](0126-spf-dmarc-alignment.md) (the Authentication-Results
  correspondence).
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md) and 0005. ARC is validated only inside
  the lazy report evaluator, on the stored bytes, after the `250`. Nothing
  runs on the SMTP path, at ingest, in `Store::insert`, at seed or restore,
  or in fan-out.
- **Held for the operator: two holds.** The delivery PR is not merged on
  green; under 0045 section 8 it waits for the operator's acceptance of both.
  The operator may rule on each separately. A refusal of either returns the
  run to design with the remedy that item names.
  1. **Section 2, the `arc` report member: an amendment of accepted 0085
     section 1**, whose report shape 0120 (`dns`) and 0122 section 2
     (`weight`, `total`) have already amended. It also departs, for ARC only,
     from the manner 0126 chose for SPF, DKIM and DMARC, where no typed
     result member exists. It changes every report's bytes and the OpenAPI
     and MCP schemas. *If refused:* the member is removed and the report
     shape stays `{rule_set_id, findings, dns, total}`. `arc=fail` and "not
     evaluated" are still the one `arc.*` finding. `arc=pass` and `arc=none`
     are then both an absence, told apart only by reading the message's own
     headers, and the failing instance is read off the located field. The
     specification's AC3 and AC15 go with the member.
  2. **Section 5, the crate's clock read: a reading of accepted 0085 section
     8**, as amended by 0125 section 8 and 0126 section 8. `verify_arc` reads
     `SystemTime::now()` on every call that passes its count checks. Under
     section 5's gate the value is never compared with anything, so no
     verdict, lookup or report byte can depend on it. 0125 section 8 sorted
     crate reads into reachable and unreachable; this one is a third kind,
     executed with a dead value, and 0085's dated pointers say the sentence
     "covers every other ambient read". Whether it covers this one is the
     operator's reading. *If refused:* no public `verify_arc` takes a time or
     skips the read, so the remedies are a scope ruling that is not built
     here: an upstream mail-auth entry point that takes the verification
     time, or reporting every complete chain as `arc.not_evaluated.v1`
     without calling the verifier. Re-implementing the verifier is refused (a
     second owner of the crypto verdict), and vendoring or `[patch]` is
     refused (the coverage gate refuses local path packages), as 0125 section
     8 already ruled.
- **Not holds, stated for the operator:**
  - `Cargo.toml` changes one line: mail-auth's features gain `arc`, which
    0125 section 1 said #256 would turn on. The feature is `arc = []` in the
    crate's manifest. `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are
    byte-identical to `main` `f05f2ee`. No package and no licence is added.
  - No file under `.github/`, `scripts/`, `.brokkr/` or the toolchain
    changes.
  - Accepted 0125 section 6's work bound is preserved, not amended: section 6
    below adds the guard that keeps "at most one re-parse (at most 16 more
    hashes)" literally true once the crate also parses ARC fields. The one
    visible effect is on a message that carries a DKIM `l=` signature beside
    more than fifteen ARC `l=` fields.
  - Accepted 0126 is not amended. No `AuthEvidence` fact, no `bulk.*`,
    `spf.*`, `dmarc.*` or `dkim.*` verdict reads ARC.
  - Accepted 0122 section 2's weights apply unchanged: 10 for each rule, 0
    for `arc.not_evaluated.v1`.

## Decision proposed

1. **The crate.**
   `mail-auth = { version = "=0.14.0", default-features = false, features = ["aws-lc-rs", "arc"] }`.
   Swarmail calls `MessageAuthenticator::verify_arc` on the one
   `AUTHENTICATOR` 0125 section 2 built with no name servers, through the
   same cache adapter over `Lookups`, which answers every name.
2. **A typed chain state in the report (hold 1).** Every report gains one
   required member, after `dns` and before `total`:
   `"arc": {"state", "sets", "failing_instance", "reason"}`.
   - `state` is `none`, `pass`, `fail` or `not_evaluated`. The first three
     are RFC 8617 section 5.2's, and the token Authentication-Results would
     carry. `not_evaluated` is Swarmail's: it declined to judge, and no
     `arc=` clause corresponds.
   - `sets` is the largest number of fields any one ARC family holds.
   - `failing_instance` is the `i=` that broke the chain, or `null`.
   - `reason` is the rule id of the report's one `arc.*` finding, or `null`.

   The evaluator computes it once and the cache holds it. REST, MCP and the
   message page render that value; none derives it.

   *Why a member, when 0126 refused one.* For SPF and DMARC, `none` is a
   defect and is itself a finding, and a pass is the absence of findings.
   For ARC, `none` is the healthy state of ordinary mail and cannot be a
   finding without disqualifying every direct message, and a pass cannot be
   one either (0122's weights; a passing check emits nothing). Two of the
   three RFC states would have no representation, and a test asserting "my
   forwarder's chain validates" would pass when the forwarder sealed
   nothing.
3. **`none` means no ARC field.** The crate returns "no chain" whenever no
   `ARC-Message-Signature` parsed, without looking at seals or
   `ARC-Authentication-Results` fields. Swarmail reports `none` only when the
   root header list holds no field of any of the three ARC families. ARC
   fields without a complete set are `fail`.
4. **A closed rule table, one finding per report.** Thirteen rules, all
   `error`, each with a fixed state, hint and location, normative in the
   [specification](../../specs/arc-chains/spec.md#rule-table):
   `arc.header_boundary_ambiguous.v1`, `arc.limits_exceeded.v1`,
   `arc.malformed.v1`, `arc.unsupported.v1`, `arc.set_incomplete.v1`,
   `arc.signature_expired.v1`, `arc.instance_invalid.v1`, `arc.cv_invalid.v1`,
   `arc.body_hash_mismatch.v1`, `arc.key_missing.v1`, `arc.key_invalid.v1`,
   `arc.signature_mismatch.v1` and `arc.not_evaluated.v1`. Chain validation
   has one verdict and one breaking point, so a report carries at most one of
   them. A pass emits nothing, and so does a message with no ARC field.
   - **mail-auth owns** parsing, canonicalization, the hashes, the signature
     checks, and the structural verdict: the instance sequence and the `cv=`
     values.
   - **Swarmail owns** what the crate cannot say or settles by a silent
     default (0125 section 5's rule, applied to ARC): `none` against
     incomplete; one body and one field list on every path, mail without a
     `DKIM-Signature` included; a repeated tag, a missing `a=`, a
     non-numeric `t=` or `x=`, and an `i=` literal that is missing, outside 1
     to 50, or not the instance the crate parsed (the crate truncates `i=` to
     32 bits); `rsa-sha1`; any `l=` on an `ARC-Message-Signature`; two fields
     of one family with the same instance (the crate sorts with an unstable
     sort, so which one it reads is not defined by the bytes; distinct
     instances of 1 to 50 also enforce RFC 8617's 50-set cap); expiry at the
     receipt second; the bound of 128 `h=` names.
   - **A finding's occurrence is the root occurrence**, never the chain
     index. The crate sorts its lists by `i=`; a field is mapped to its root
     occurrence by its position in the message.
   - **The failing seal is named by the lookup count.** The crate looks up
     the newest `ARC-Message-Signature`'s key, then each seal's key from the
     newest down, once each, and stops at the first failure. The number of
     lookups in the one `verify_arc` call therefore names the field that
     failed, exactly, for every chain length, with no extra parse and no
     extra signature check. The order is the pinned crate's; fixtures pin it.
   - **Keys follow 0125:** an undeclared name is `arc.key_missing.v1`, a
     definite absence under 0120; a declared record that is no usable key is
     `arc.key_invalid.v1`; a declared `tempfail` is `arc.not_evaluated.v1`
     and never clean.
5. **No ambient clock decides anything (hold 2).** Expiry is judged at the
   receipt second, as 0125 section 4 rules for DKIM. The crate offers no
   entry point that takes a time, so Swarmail decides every chain whose
   newest `ARC-Message-Signature` carries an `x=` tag before the crate is
   called:
   - `x=` at or before the receipt second, or at or before its own `t=`:
     `arc.signature_expired.v1`, state `fail`;
   - any other `x=`: `arc.not_evaluated.v1`, state `not_evaluated`.

   The crate is called only when that field has no `x=` tag. Its expiry test
   is `x == 0 || (x > t && x > now)`, so with `x` equal to 0 the clock value
   it read is never compared. A report is a function of the stored bytes, the
   receipt second and the policy, and 0120 section 5's byte-identical reports
   gain no third exception.

   *The cost.* A chain whose newest `ARC-Message-Signature` carries an `x=`
   after the receipt second goes unverified, and reports an error. This is
   option B of 0125 section 8, which was not built for DKIM. *The
   alternative, not built:* let such a chain reach the crate and accept
   DKIM's residual (once the host clock passes `x=`, the verdict becomes
   expired and its lookups disappear). The framing for #256 rules out an
   ambient clock, so this record does not propose it; the operator may
   prefer it.
6. **0125 section 6's work bound, with ARC compiled in.** The strict parse
   still hashes the body at most four times: an `ARC-Message-Signature`
   shares the (canonicalization, hash) pairs DKIM uses, and one with a
   non-zero `l=` is rejected unhashed. The non-strict re-parse that DKIM
   makes for an `l=` signature would now also hash every
   `ARC-Message-Signature` with a distinct `l=`. So `dkim::prepare` re-parses
   only when the `DKIM-Signature` fields carrying `l=` plus the
   `ARC-Message-Signature` fields carrying `l=` number at most 16; past that
   it emits the existing `dkim.limits_exceeded.v1` and does not re-parse.
   ARC itself always reads the strict parse, so its verdict never depends on
   an unrelated DKIM field.
7. **ARC decides nothing else.** The contents of an
   `ARC-Authentication-Results` field are not read. No fact, no DMARC
   disposition and no other finding changes with the chain state. `arc=pass`
   says the chain validated; it does not say the sender is authentic. Using
   a chain to rescue a DMARC failure is a separate decision with its own
   security argument.
8. **Version.** `RULE_SET_ID` becomes `swarmail-html/9`, the next
   never-shipped id (0122 section 1.4). The 256-finding and 65,536-byte
   bounds stand; a report past them is `AnalysisError::Failed`, never
   truncated.

## Alternatives ruled and costs

The [panel adjudication](../../specs/arc-chains/spec.md#panel-adjudication--2026-11-16)
records both positions and each ruling. In short:

- **Findings only, with `arc=` read off a table (simplicity): refused** for
  the reason in section 2. *Cost of the member:* hold 1, new bytes in every
  report, two schema changes and a regenerated fixture.
- **Prefix bisection to find the failing seal (robustness): refused.** The
  lookup count is exact for all 50 sets and free. *Cost:* attribution leans
  on the pinned crate's lookup order.
- **A Swarmail structural pre-scan (robustness): refused.** It is a second
  owner of a verdict the crate already gives. *Cost:* a chain that is both
  structurally wrong and carries `x=` reports `not_evaluated`, not `fail`.
- **Any `x=` is not evaluated (simplicity): refused** in favour of the split
  at the receipt second. *Cost:* one more rule.
- **A missing key as `not_evaluated` (robustness, and the intake's wording):
  refused** toward accepted 0125. *Cost:* an undeclared forwarder key weighs
  10, as an undeclared DKIM key does.
- **Not-evaluated as `fail` (robustness): refused.** It would report a broken
  chain nobody observed. *Cost:* a fourth state.
- **Failing the report for the `l=` combination (robustness): refused.** 0125
  rules that a bound is a finding, never a failed report, for sender-shaped
  data.

## Proof and disposition

The [tasks](../../specs/arc-chains/tasks.md) pair every change with its
proof, and the
[specification](../../specs/arc-chains/spec.md#acceptance-criteria) holds the
acceptance criteria. Every house gate runs `--locked`, including the evals
crate, the exact coverage gate and its selftest, the publish-guard and seal
proofs, and `cargo deny`. This record stays proposed; only the operator
accepts it.
