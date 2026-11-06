# 0126 — SPF for the observed envelope, DMARC by the RFC 9989 tree walk, on mail-auth

- **Status:** proposed
- **Date:** 2026-11-06
- **Scope:** epic #251, issue #255 (operator direction: the remaining
  deliverability stories in the order #253, #254, #255; launched on
  2026-11-06 once #254 merged as #336). Run
  `issue-255-evaluate-spf-and-dmarc-d2898123`, from `main` `7478073`, on the
  `claude-careful` recipe. The dispatcher assigned the number 0126. No other
  number is used.
- **Specification:** [SPF and DMARC alignment](../../specs/spf-dmarc-alignment/spec.md).
- **Authority:** the operator's #251 ruling of 2026-10-26 (authentication on an
  existing permissive crate, one widening, taken by 0125 for #254, #255 and
  #256), and the operator's 2026-11-06 acceptance of 0125's three holds.
- **Builds on:** accepted [0085](0085-deliverability-report.md) (lazy, cached,
  bounded reports; the report shape `{rule_set_id, findings, dns}` is unchanged
  here; §8 below proposes one more amendment of its ambient-time sentence);
  accepted [0089](0089-bulk-sender-checklist.md) (the three-state facts and
  the `bulk.*` checklist, read as §6 states); proposed
  [0120](0120-deliverability-dns-policy.md) **with an integration note**
  (one optional policy member, and how non-TXT queries are answered, §3 and
  §4); proposed [0125](0125-offline-dkim-verification.md) **with an
  integration note** (its per-signature verdicts feed DMARC, §7); proposed
  [0121](0121-deliverability-link-checks.md) §5 and proposed
  [0122](0122-deterministic-spam-rules.md) §1.4 (the next never-shipped
  `rule_set_id`); accepted 0044 §8 (the envelope sender).
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md) and 0005. The two new envelope facts
  are copied from the SMTP session into the `Email` before the one
  synchronous insert, so they ride the same row and the same commit, before
  the `250`. Nothing is evaluated on the SMTP path, at ingest, in
  `Store::insert`, at seed or restore, or in fan-out. Evaluation stays inside
  the lazy report evaluator.
- **Held for the operator: one named hold.** The delivery PR is not merged on
  green:
  1. §8, SPF's monotonic lookup timer inside mail-auth's `check_host`, is a
     proposed amendment of accepted 0085 §8 (as already amended for DKIM's
     `x=` by 0125 §8, accepted 2026-11-06). Under 0045 §8 the delivery waits
     for the operator's acceptance of §8.
- **Not holds, stated for the operator:**
  - `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` stay
    byte-identical. No crate and no feature is added: mail-auth 0.14.0
    compiles `spf` and `dmarc` with the features #254 enabled.
  - No file under `.github/`, `scripts/`, `.brokkr/` or the toolchain changes.
  - No accepted ruling is contradicted. §6 states how 0089 §5's "Inputs
    equality" is met by the facts' determinants. If the operator reads §5 as
    requiring the facts themselves as request inputs, that reading conflicts
    with accepted 0085's cache and permit bounds, and is the operator's to
    rule.
  - The report's top-level shape is unchanged, so accepted 0085 §1 is not
    amended. The one public contract change is an optional member of 0120's
    proposed declaration grammar.

## Decision proposed

1. **The crate, unchanged.** mail-auth `=0.14.0`, `default-features = false`,
   `features = ["aws-lc-rs"]`, exactly as 0125 §1 pinned it. Its `spf` and
   `dmarc` modules are not feature-gated (its features are `arc`, `aws-lc-rs`,
   `default`, `dns-doh`, `dns-hickory`, `generate`, `rust-crypto`, `test`).
   Swarmail calls `MessageAuthenticator::check_host` and
   `MessageAuthenticator::verify_dmarc` on the one `AUTHENTICATOR` that 0125
   §2 built with no name servers.
2. **The envelope observation is captured at ingest, in the row.** `Email`
   gains `smtp_client_ip: Option<IpAddr>` and `smtp_helo: Option<String>`.
   - The SMTP session records the TCP peer address once at accept,
     canonicalized (`IpAddr::to_canonical`, so `::ffff:127.0.0.1` is
     `127.0.0.1`), and the argument of the last `HELO`/`EHLO`, trimmed; an
     empty argument or no greeting is `None`. `STARTTLS` clears the HELO name
     (RFC 3207 §4.2). Implicit TLS keeps the accept-time peer.
   - Both are set on the `Email` after `build_email` and before
     `insert_async`, inside the same offload. `Store::insert`, await and
     pruning change no line. Pruning removes the facts with their row.
   - Every non-SMTP constructor (REST seed, the MailSlurper shim, fixtures)
     leaves both `None`. No header ever supplies either.
   - Persistence: two `ADDED_COLUMNS`, `smtp_client_ip TEXT` and
     `smtp_helo TEXT`, nullable, no default, never backfilled (the 0044 §8
     and 0094 §2 pattern). A restored older row reads `None`. A stored
     `smtp_client_ip` that does not parse as an address refuses the restore
     as corruption.
   - **Surfaces.** Neither fact appears in REST, MCP, OpenAPI, webhook or feed
     payloads in this story (`#[serde(skip)]`, the `link_metadata`
     precedent). `FIELD_NAMES` stays at 21 and every pinned payload keeps its
     bytes. A later story may expose them additively; a published member can
     never be withdrawn.
3. **The address SPF sees: the socket's, unless the policy declares one.**
   0120's declaration gains one optional member, `spf_client_ip`, an IP
   literal (canonical form on output, present in `dns.policy` only when
   declared, so existing reports' policy bytes keep their shape). When the
   effective policy declares it, SPF evaluates against that address instead
   of the observed one. It never manufactures an observation: mail with no
   observed peer is still not evaluated. It applies to every peer, loopback
   or not: whoever declares the policy already declares every SPF record, so
   the override grants no power the policy did not hold, and it is echoed in
   the report's `dns.policy`, so it is never silent. Without it, a loopback
   test declares `ip4:127.0.0.1` (and `ip6:::1`) in its record.
4. **Record types: 0120 stays a TXT map.** SPF's `a`, `mx`, `ptr` and
   `exists` mechanisms, and the `%{p}` macro, need A/AAAA/MX/PTR answers the
   declared policy cannot hold. The SPF adapter answers each of those caches
   with a fixed negative, so the crate never reaches its resolver, and marks
   the evaluation. A marked evaluation's verdict is
   `spf.unsupported_mechanism.v1` whatever the crate decided, and its
   `spf_pass` fact is Unknown, never False: Swarmail did not look, so the
   sender did not fail. A term after the first match is never evaluated
   (RFC 7208 §4.6.2), so `ip4:127.0.0.1 a -all` passes for `127.0.0.1`.
   DMARC's one non-TXT query, the A query for the Author Domain that decides
   `np=`, is answered by a fixed rule over the embedded policy: the name
   exists (NODATA) when it or any name below it is declared (TXT or
   `tempfail`), otherwise it does not (NXDOMAIN; RFC 8020's reading of an
   empty subtree). `dns.lookups` keeps listing TXT answers only; non-TXT
   answers are not listed, and each is reproducible from the embedded policy.
5. **SPF.** One `check_host` per message, for one identity: MAIL FROM when
   `envelope_from` is non-empty, the HELO name when it is the null reverse
   path (RFC 7208 §2.4; RFC 7489's SPF identity). SPF runs only when the row
   has an observed peer and a known envelope sender, and, for the HELO
   identity, a HELO name. Otherwise the message gets one
   `spf.not_evaluated.v1` (info) finding.
   - Record selection is Swarmail's (RFC 7208 §4.5): of a name's declared
     values, only those beginning `v=spf1` followed by a space or the end
     count; none is "no record", and more than one is `permerror`. The
     crate's own first-parse-wins default never decides.
   - The adapter clears each parsed record's RFC 6652 `ra=` modifier before
     the crate sees it, so the crate's report sampling never runs (§8).
   - `host_domain` is a fixed constant, never the configured hostname, so a
     report does not depend on configuration.
   - Results map one-to-one onto RFC 8601 tokens: `spf.fail.v1`,
     `spf.softfail.v1`, `spf.neutral.v1`, `spf.none.v1`, `spf.temperror.v1`
     and `spf.permerror.v1`; a pass emits nothing. All are `error`, located
     at `return-path` with `occurrence` null: the envelope, never a header.
   - **Limits (RFC 7208 §4.6.4), stated and proven.** Ten DNS-querying terms
     are admitted and the eleventh is `permerror`: the crate's `LookupLimit`
     counts `include`, `redirect`, `a`, `mx`, `ptr`, `exists` and the `%{p}`
     PTR, and does not count the initial record lookup. Under §4 only
     `include` and `redirect` can draw on it without being unsupported, so a
     declared chain proves it. **Void lookups are vacuous by construction:**
     an `include` or `redirect` target with no record is already `permerror`
     at its first void answer, and every other term that can be void is
     unsupported before a void answer can decide anything. The crate has no
     void counter, and none is needed. One evaluation looks up at most twelve
     TXT names (the record, ten terms, one `exp=` target).
6. **The facts are outputs of the evaluation, not request inputs.** The six
   `AuthEvidence` facts are computed in the evaluator, from the stored bytes,
   the row's immutable envelope observation, #254's verdicts and the frozen
   policy, before the `bulk.*` rules read them. The placeholder `policy_id`
   and `observation_id` strings are deleted, and the request-path `auth`
   input becomes a `#[cfg(test)]` override that keeps 0089's truth-table
   tests. 0089 §5 asks that every effective fact and the policy identity sit
   in Inputs equality alongside the Weak mail identity. Every determinant of
   every fact does: the mail identity (which now carries the envelope
   observation, immutable for a stored `Email`), the policy snapshot and the
   rule set. Two requests with equal keys therefore get equal facts, and
   "provenance and mail binding" hold by construction: a fact can describe
   only the message the report is for. Computing the facts before the cache
   lookup instead would run DKIM's crypto and the SPF and DMARC walks on
   every request, outside 0085's cache and its four job permits.
7. **DMARC on the crate's RFC 9989 tree walk; no public-suffix list.**
   - **From validity is Swarmail's, first.** Exactly one root `From` field,
     whose crate parse agrees with Swarmail's header boundary, and whose
     addresses share one domain. Zero or several fields, or no single domain,
     is `dmarc.from_invalid.v1`; a crate parse that fails or disagrees is
     `dmarc.not_evaluated.v1`. `Email.from` is never read, because it falls
     back to the envelope when the header is absent.
   - **Discovery and the organizational domain** are the crate's: TXT queries
     for `_dmarc.<name>` from the Author Domain upward, at most eight; a
     `psd=y` or `psd=n` record stops the walk; the organizational domain is
     taken from the records found. Every query goes through 0120's map and
     lands in `dns.lookups`. **The public suffix is determined offline, by the
     declared policy alone** (a declared `psd=y` record marks a public suffix
     domain), **and is pinned by `mail-auth =0.14.0`'s walk.** No list is
     fetched, checked in or compiled in: a second owner of the organizational
     domain would disagree with the walk that decides alignment, and 0085 §8
     forbids an ambient registry.
   - Record selection is Swarmail's: only values beginning `v=DMARC1` count.
     More than one at a name, or one that does not parse, is
     `dmarc.permerror.v1`. A declared `tempfail` on a walked name is
     `dmarc.temperror.v1`. No applicable record is `dmarc.none.v1`.
   - **Disposition.** The crate applies `p=` to the Author Domain's own
     record, `sp=` (or `np=` for a non-existent Author Domain, §4) to a
     record found above it, and `t=y`'s one-step downgrade. A DMARC failure
     carries the applied disposition in its id: `dmarc.fail.none.v1`,
     `dmarc.fail.quarantine.v1` or `dmarc.fail.reject.v1`. `pct=` is not an
     RFC 9989 tag and the crate does not parse it; no message is sampled.
   - **Alignment** is the crate's (`adkim` and `aspf`, relaxed by default).
     One `verify_dmarc` call per message takes #254's DKIM outputs **filtered
     by #254's own verdicts** (verified and the `l=` partial warning kept,
     not-evaluated kept as temporary, every other verdict dropped), so a
     signature Swarmail refuses never aligns, and an unverified `d=` never
     aligns. Its SPF slot carries the evaluated identity's domain with a pass
     result, built with the public `SpfOutput::new(..).with_result(..)`, so
     the crate reports that identity's **alignment** whatever SPF's own
     result was; the SPF result itself stays the `check_host` result.
     Alignment facts are computed only when an applicable record was found;
     otherwise they are Unknown, because alignment is defined against the
     applicable policy.
   - Every `dmarc.*` finding is `error`, located at the root `From` field
     (`occurrence` null when absent, 1 when duplicated, 0 otherwise). The
     DMARC result is derived from the facts with 0089 §5's formula; Swarmail
     computes no second DMARC verdict.
8. **Ambient reads on the SPF and DMARC paths: one reachable, held.** Read in
   the pinned source (`mail-auth-0.14.0/src`), every one:
   - `LookupLimit` (`spf/verify.rs`) reads `Instant::now()` once `check_host`
     passes its domain check, and `elapsed()` at every DNS-querying term,
     refusing a term once 20 s have passed. **Reachable on every SPF
     evaluation.** *Radius:* only if 20 s of monotonic time pass between the
     start of one `check_host` and a later DNS-querying term, inside a single
     synchronous poll (a descheduled process: SIGSTOP, a VM pause, a swap
     storm), the verdict becomes `spf.permerror.v1` and `spf_pass` False. It
     is fail-closed (never a pass) and not one-way, and it is the second
     exception to 0120 §5's byte-identical reports after 0125 §8. No public
     `check_host` takes a time or skips the timer.
   - `is_within_pct` (`sampling.rs`: `SystemTime::now()` and a thread-local
     counter), reached through `SpfOutput::with_report` only when the record
     in effect carries `ra=` with `rp` below 100: **unreachable**, because the
     adapter clears `ra=` (§5). It could change only `report_address`, which
     Swarmail never reads.
   - `current_time` (`spf/macros.rs`, `%{t}`): **unreachable**. `%{t}`
     parses only in `exp=` explanation text, which the crate fetches as
     `txt_lookup::<Macro>`; the SPF adapter only builds SPF records, so the
     unwrap fails and no explanation is expanded. The `exp=` name is still
     looked up and recorded.
   - DMARC (`dmarc/verify.rs`): no clock, random or environment read.
   - Resolver: every SPF and DMARC cache answers every query, and the
     authenticator has no name servers (0125 §2, accepted).

   *The amendment proposed.* Accepted 0085 §8, as amended by 0125 §8, admits
   no ambient time into the collector other than DKIM's `x=` read. This record
   proposes that it does not cover SPF's monotonic lookup timer, bounded as
   above, and still covers every other read. Under 0045 §8 the delivery PR
   waits for the operator's acceptance of this section. *If the operator
   refuses,* the remedies are a scope ruling that is not built here: an
   upstream mail-auth API that takes the time or skips the timer, or not
   evaluating SPF (the SPF facts stay Unknown and DMARC passes through DKIM
   only). Re-implementing `check_host` is refused (a second owner of the
   verdict), and vendoring or `[patch]` is refused (the coverage gate refuses
   local path packages).
9. **Version.** Every report's bytes change (the new `spf.*` and `dmarc.*`
   findings, the `bulk.*` facts, and more `dns.lookups`), so `RULE_SET_ID`
   becomes `swarmail-html/7`, the next never-shipped id (`/6` ships in
   v0.4.0; 0121 §5, 0122 §1.4). The shape `{rule_set_id, findings, dns}` is
   unchanged. The 256-finding and 65,536-byte bounds stand; past them a
   report is `AnalysisError::Failed`, never truncated.
10. **#254's deferred await clause is delivered** by name (0125, AC11): over
    real SMTP, with a per-inbox declared DKIM key, SPF record and DMARC
    record, a signed message that passes the whole checklist is returned by
    an await with `no_findings_at_or_above=warning`, and an altered copy is
    not.

## Alternatives ruled and costs

The [panel adjudication](../../specs/spf-dmarc-alignment/spec.md#panel-adjudication--2026-11-06)
records both positions and each ruling. In short:

- **A public-suffix snapshot (robustness): refused.** The crate decides the
  organizational domain by the RFC 9989 tree walk, and its alignment helpers
  are private; a list would be a second owner that disagrees with it, and a
  data-file hold besides. *Cost:* a test that wants `example.co.uk` treated
  as under a public suffix declares `_dmarc.co.uk` with `psd=y`.
- **Extending 0120 to A/AAAA/MX/PTR (robustness): refused for this story.**
  It costs four grammar members, an NXDOMAIN/NODATA model that 0120 §4 does
  not have, a temperror that the crate's non-TXT caches cannot express, and a
  void counter Swarmail would own. *Cost:* a record whose evaluation reaches
  `a`, `mx`, `ptr` or `exists` is `spf.unsupported_mechanism.v1` (Unknown,
  never a failure); a later story can add address records additively.
- **An `auth` report member (robustness): refused.** It amends accepted 0085
  §1 and adds a hold. The A-R correspondence is carried by rule ids named by
  RFC 8601 tokens and a normative mapping table. *Cost:* a pass is an
  absence, and the evaluated address is not echoed unless declared.
- **No override of the client address (simplicity): refused.** The brief
  asks how a test declares the address SPF sees, and a user checking a
  production record against a production address should not have to edit the
  record. *Cost:* one grammar member and its schema change.
- **A loopback-only override (robustness): refused.** It protects nothing the
  policy author does not already control, and a containerized peer is not
  loopback.
- **No `dmarc.*` ids and no disposition (simplicity): refused.** The brief
  names `sp=` and `np=`, and their only effect is the disposition. *Cost:* a
  DMARC failure is reported twice, as `dmarc.fail.*` and as its `bulk.*`
  checklist entry.
- **Refusing every SPF macro (robustness): refused.** `%{t}` is unreachable
  and `%{p}` is unsupported by §4; every other macro expands only the
  envelope.
- **Facts at request entry (robustness): refused** (§6).

## Proof and disposition

The [tasks](../../specs/spf-dmarc-alignment/tasks.md) pair every change with
its proof, and the [specification](../../specs/spf-dmarc-alignment/spec.md#acceptance-criteria)
holds the acceptance criteria. Every house gate runs `--locked`, including
the evals crate, the exact coverage gate and its selftest, the publish-guard
and seal proofs, and `cargo deny`. This record stays proposed; only the
operator accepts it.
