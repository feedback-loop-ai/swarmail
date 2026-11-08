# 0126 — SPF for the observed envelope, DMARC by the RFC 9989 tree walk, on mail-auth

- **Status:** accepted (operator, 2026-11-07 — "accept all three"; see the dated acceptance note at the end)
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
  here; §8 below proposes one more amendment of its ambient-time sentence,
  hold 2); accepted [0089](0089-bulk-sender-checklist.md) (the three-state
  facts and the `bulk.*` checklist, read as §6 states, hold 3); accepted
  [0120](0120-deliverability-dns-policy.md) (operator, 2026-11-06) **with an
  integration note**: §3 and §4 below amend its §2, §3 and §4, hold 1, and §8
  adds a second exception to its §5, hold 2; accepted
  [0125](0125-offline-dkim-verification.md) (operator, 2026-11-06) **with an
  integration note** (its per-signature verdicts feed DMARC, §7; nothing in it
  is amended); accepted [0121](0121-deliverability-link-checks.md) §5 and
  accepted [0122](0122-deterministic-spam-rules.md) §1.4 (the next
  never-shipped `rule_set_id`; both operator, 2026-11-06); accepted 0044 §8
  (the envelope sender).
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md) and 0005. The two new envelope facts
  are copied from the SMTP session into the `Email` before the one
  synchronous insert, so they ride the same row and the same commit, before
  the `250`. Nothing is evaluated on the SMTP path, at ingest, in
  `Store::insert`, at seed or restore, or in fan-out. Evaluation stays inside
  the lazy report evaluator.
- **Held for the operator: three holds against three accepted records, five
  named items.** The delivery PR is not merged on green; under 0045 §8 it waits
  for the operator's acceptance of every item below. The operator may rule on
  each item separately; a refusal of any item returns the run to design with
  the remedy that item names.
  1. **An amendment of accepted 0120** (operator, 2026-11-06), in three
     sections:
     - **1a, 0120 §2, the declaration shape.** "Both members … optional on
       input and always present on output" no longer describes the whole
       shape: §3 below adds a third member, `spf_client_ip`, optional on input
       and present on output (and in `dns.policy`) only when declared. It
       changes the API body, the server file, the OpenAPI and MCP policy
       schemas and, when declared, the report's `dns.policy` bytes.
       *If refused:* tests bind the SPF record to the loopback peer they
       actually use (`ip4:127.0.0.1`, `ip6:::1`), and the "a record listing
       only the declared address" fixtures of AC5 go with the member.
     - **1b, 0120 §3, "the only path that answers, and it records every name
       it answers".** Two answers bypass the recorder: DMARC's `np=` existence
       query (an A query) is answered by `Lookups::exists` from the embedded
       policy and recorded nowhere, and SPF's A, AAAA, MX and PTR queries get a
       fixed negative (§4). `dns.lookups` keeps listing TXT answers only. Each
       unrecorded answer is a pure function of the policy the report embeds,
       so the report stays reproducible from its own bytes. *If refused:* the
       answers are recorded, which amends 0120 §5's lookup shape (`type`
       beyond `txt`) instead; that is a new design return.
     - **1c, 0120 §4, "NXDOMAIN and NODATA are not distinguished".** §4 below
       distinguishes them for `np=` only, by a fixed rule over the declared
       names: a name exists when it or a name below it is declared (a TXT
       owner or `tempfail`). *If refused:* every Author Domain is treated as
       existing, so `np=` never applies, and AC10's `np=` fixture pair goes.
  2. **SPF's monotonic lookup timer (§8): an amendment of accepted 0085 §8**,
     as already amended for DKIM's `x=` by 0125 §8 (accepted 2026-11-06),
     **and the second stated exception to accepted 0120 §5's byte-identical
     reports**, after 0125 §8's. One mechanism, two accepted sentences; the
     operator rules on it once. §8 names the remedies if refused.
  3. **A reading of accepted 0089 §5** (operator, 2026-11-02): "Include all
     effective facts/policy identity in Inputs equality". §6 below meets it by
     the facts' determinants (mail identity carrying the envelope
     observation, policy snapshot, rule set), not by the facts as request
     inputs. It is a reading of accepted text, so the operator rules on it.
     *If refused:* the facts are computed before the cache lookup, outside
     0085's cache and its four job permits, which amends accepted 0085's
     bounds instead; that is a new design return.
- **Not holds, stated for the operator:**
  - `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` stay
    byte-identical to `main` `c95386e` (the merge-base) and to `7478073`'s
    package set. No crate and no feature is added: mail-auth 0.14.0 compiles
    `spf` and `dmarc` with the features #254 enabled.
  - No file under `.github/`, `scripts/`, `.brokkr/` or the toolchain changes.
  - The report's top-level shape is unchanged, so accepted 0085 §1 is not
    amended. The one public input change is 0120 §2's optional member, hold
    1a.
  - Accepted 0125 is not amended: DMARC consumes its verdicts as they are.
  - Accepted 0121 §5 and 0122 §1.4 are followed, not amended:
    `swarmail-html/7` is the next never-shipped id (`/6` shipped in v0.4.0).
  - The null-sender baseline (§5) and DMARC record selection (§7) follow the
    pinned crate and RFC text; neither touches an accepted record.

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
   path (RFC 7208 §2.4). SPF runs only when the row
   has an observed peer and a known envelope sender, and, for the HELO
   identity, a HELO name. Otherwise the message gets one
   `spf.not_evaluated.v1` (info) finding.
   - **The DMARC baseline for the null sender is the pinned crate's RFC 9989
     contract** (design return, 2026-11-06). `mail-auth 0.14.0` implements
     RFC 9989, and its `DmarcParameters::mail_from_domain` is "the
     RFC5321.MailFrom domain (or the HELO domain for a null reverse-path) that
     SPF was evaluated for" (`dmarc/verify.rs`, the field and
     `DmarcParameters::new`). Swarmail passes exactly that, so for the null
     sender the HELO identity's domain is DMARC's SPF identifier, as RFC 7489
     §4.1 also provides through RFC 7208 §2.4's null-path processing. The realm holds no copy of RFC 9989's
     text, so the crate the operator pinned in 0125 is the authority, as it is
     for DKIM. This is not a security exposure: the HELO identity aligns only
     when its own SPF record authorizes the connecting address and its domain
     aligns with the From domain, so no unauthenticated party gains alignment.
     *If the operator reads RFC 9989 as dropping HELO,* the remedy is one
     adapter arm: the null sender passes the empty SPF slot, so `spf_aligned`
     is False and a bounce passes DMARC through DKIM only. `spf_pass` keeps
     the HELO `check_host` result either way.
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
   - **Record selection is Swarmail's** (revised on the design return,
     2026-11-06). A value is a DMARC record when it opens with the version
     tag in its ABNF form, `v` *WSP `=` *WSP `DMARC1` (RFC 7489 §6.4): the
     tag name `v` is case-insensitive, whitespace (space or tab) around `=` is
     allowed, `DMARC1` is case-sensitive (`%x` octets), and it is followed by
     whitespace, `;` or the end. So `v = DMARC1; p=reject` is a record and
     `v=dmarc1; p=reject` and `v=DMARC10` are not. The crate's own parser is
     whitespace- and case-tolerant, so this selector is what decides.
     - **Two or more `v=DMARC1` values at one name are all discarded**, as
       though the name held no record, and the tree walk goes on, as the
       crate's RFC 9989 §4.10 walk treats any name without a record (RFC 7489
       §6.6.3 step 5: multiple records mean the policy is not applied, A-R
       `none`). Permerror is kept for one record that does
       not parse (RFC 7489 §11.2: "a syntactically incorrect DMARC record").
       When nothing else on the walk applies the finding is `dmarc.none.v1`,
       whose hint says "exactly one v=DMARC1 record". The discarded values
       stay visible in `dns.lookups`, which records the name and its values.
       This replaces the earlier `dmarc.permerror.v1` for multiplicity; both
       are errors with `policy_present` False, so no `bulk.*` verdict moves.
     - SPF keeps `permerror` for more than one `v=spf1` value: RFC 7208 §4.5
       says so explicitly.
     - A declared `tempfail` on a walked name is `dmarc.temperror.v1`. No
       applicable record is `dmarc.none.v1`.
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
8. **Ambient reads on the SPF and DMARC paths: one reachable, hold 2.** Read in
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
   above, and still covers every other read. The same timer is the second
   stated exception to accepted 0120 §5's byte-identical reports, after
   0125 §8's `x=` read. Under 0045 §8 the delivery PR waits for the
   operator's acceptance of this section (hold 2). *If the operator
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

## Implementation record — 2026-11-06 (the implement seat)

The implementation follows §1–§10. This record states what was confirmed in the
pinned source, and the few places where building it settled something the design
left to the builder. None changes a ruling above; the status stays proposed and
the one hold (§8) stands.

**Task 0, the source re-read.** `mail-auth-0.14.0` `spf/verify.rs`,
`spf/output.rs`, `spf/macros.rs`, `sampling.rs`, `dmarc/verify.rs` and
`dns/lookup.rs` were read again. §8's inventory is exact and complete:

- `spf/verify.rs` `LookupLimit::new` calls `Instant::now()` once `check_host`
  passes its domain check, and `can_lookup` reads `elapsed()`: reachable on every
  evaluation, the one hold.
- `sampling.rs` `is_within_pct` (`SystemTime::now()`) is reached only through
  `SpfOutput::with_report` with an `ra=` in the record in effect; the adapter
  clears `ra=` from every record, and a test asserts both the cleared record and
  the crate's own parse of the same text still carrying it.
- `spf/macros.rs` `current_time` is reached only by `%{t}`, which parses only in
  `exp=` text, fetched as a `Macro` record the SPF adapter never builds.
- `dmarc/verify.rs` reads no clock, random value or environment variable.
- `dns/lookup.rs` consults the cache before any resolver call, and every cache
  answers every query, so no resolver is reached. `cache.insert` is the only
  other use of an `Instant`, and the adapter's `insert` ignores it.
- `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are
  byte-identical to `7478073` (`git diff --exit-code`); `spf` and `dmarc` are not
  feature-gated. No new reachable ambient read exists.

**One crate leniency found and closed.** `SpfRecord::parse` treats a bare word
that is no mechanism (`inlcude:x.example`) as an unknown modifier-like term,
skips it, and swallows the term after it, so `v=spf1 inlcude:x.example -all`
reads as neutral where RFC 7208 section 4.6 requires a permanent error. Under
0125's principle that Swarmail owns every check the crate settles by a silent
default, the SPF adapter checks that every term names a mechanism or is a
`name=value` modifier before the crate parses the record (`known_terms`); the
crate still parses every argument. A typo is `spf.permerror.v1`.

**Where building settled a detail the design left open.**

- **DKIM stopped at its bound.** When #254 stops at the 16-field or 128-name
  bound it pushes its one finding and verifies nothing, but the parse is sound,
  so DMARC still runs with no DKIM outputs; `dkim_pass` and `dkim_aligned` are
  Unknown then, not False. At the boundary guard the crate's parse is not the one
  Swarmail's collectors read, so DMARC is `dmarc.not_evaluated.v1` and
  `valid_from` and `policy_present` are Unknown.
- **The DKIM hand-off.** `dkim::collect` became `prepare` (parse and guards, an
  enum of unsigned, stopped, limited, ready) and `verify` (the verdicts, unchanged,
  returning the retained crate outputs and `dkim_pass`). The cache adapter is
  owned by the caller, because the crate's outputs borrow the message and the
  cache for one lifetime and DMARC reads them afterwards. One shared borrow of
  the `Lookups` serves the three evaluations, which run in sequence.
- **Test builds.** `Inputs.auth` is a `#[cfg(test)] Option<AuthEvidence>`; test
  builds default it to `Some(unknown facts)` so every test that predates the
  evaluation keeps its meaning, and a test of the evaluation sets `None`
  (`Service::auth` carries `None` for the same purpose at the service level).
  Production has no such input.
- **Hints.** The sixteen fix hints are constants with no character the page or a
  JSON string must escape (no apostrophe, angle bracket, ampersand, quote or
  backslash), and the guide binds each rule's id, severity and hint to the code.
- **`known_terms` placement, and `HOST`.** `swarmail.invalid` (RFC 2606) is the
  fixed `%{r}` host name.

**Where each acceptance criterion is proved.** AC1: `smtp.rs` inline tests
(`the_peer_and_the_greeting_ride_the_row_before_the_250`,
`starttls_forgets_the_greeting_but_not_the_peer`, a mapped-peer unit test) and
`tests/deliverability.rs`. AC2: `persist.rs` inline tests and
`tests/persistence.rs` (round trip across restarts, an old-schema file, a corrupt
address refusing to serve). AC3: no line of `Store::insert`, await or pruning
changed; the burst, p2 and rate suites run in the gates. AC4:
`the_observed_envelope_never_serializes` and every pinned payload test, unchanged.
AC5: `dns.rs` and `auth.rs` inline tests, `tests/deliverability.rs`,
`tests/binary.rs` and the `lib.rs` startup test. AC6–AC12, AC15, AC16: the
`auth.rs` inline tests. AC13: the six-facts table test,
`tests/bulk_sender.rs` and a pre-columns data file in
`tests/deliverability.rs`. AC14: `tests/deliverability.rs`
(`a_message_that_passes_everything_is_returned_by_an_await`). AC17: the shape
tests, updated for `swarmail-html/7`. AC18: the diff above. AC19: the gate log in
the delivery result. AC20: this record, the README row, the notes on 0120 and
0125, the guide, `llms.txt` and the OpenAPI text.

**Fixtures edited by hand.** The seed report row of
`tests/fixtures/mcp-structured-output/march-current-main.jsonl` is replaced by the
report this tree serves for the same seeded mail, with its provenance and hash
recorded; unlike the earlier id-only edits it gains findings and lookups.

## Design return — 2026-11-06 (review 1, `spec_defect`)

The first review (run `issue-255-evaluate-spf-and-dmarc-2586f754`, head
`6653277`, tree `578acbd`) found the frozen guarantees intact, the code true
to §1–§10 and every gate green, and returned the design for one medium defect
that four of five members reported: this record's hold statement. It was
written while 0120, 0121, 0122 and 0125 were proposed; the operator accepted
them on 2026-11-06, after the design commit, so what had been integration
notes on proposals became amendments of accepted records. The header now names
three holds and five items, and lists those records as accepted. The
implementation record above says "the one hold (§8)": that sentence is dated
history, and the header supersedes it.

The return also settles three RFC-fidelity points the review raised, each in
its section: DMARC record selection (§7: multiplicity discarded instead of
permerror, the `v=DMARC1` ABNF), and the null-sender baseline (§5: the pinned
crate's contract, unchanged behaviour). The other review items (two stale
OpenAPI descriptions, the "one exception" wording that predates §8, the
AC18 proof base `c95386e`) are tasks in the specification. Nothing else
changes: no rule id is added or removed, `swarmail-html/7` stays the id, and
this record stays proposed.

## Implementation record — design return, 2026-11-06 (the implement seat)

Tasks T27–T36 of the specification landed on top of `35a0db2`. The status stays
`proposed`, no decision's status changed, and no rule id was added or removed;
`swarmail-html/7` stands.

- **T27–T28, records.** The README row names the three holds and the accepted
  0120 and 0125. 0120's dated integration note is relabelled §2 (declaration
  grammar, hold 1a), §3 (unrecorded non-TXT answers, hold 1b) and §4
  (NXDOMAIN/NODATA for `np=`, hold 1c), with the timer paragraph pointing at hold
  2. Dated pointers were added to 0085 (hold 2) and 0089 (hold 3); each says
  the item waits for the operator under 0045 §8.
- **T29, OpenAPI.** `DeliverabilityReport` no longer says that no production
  message qualifies, and `DnsRecord` no longer says SPF and DMARC "will record
  theirs"; status wording is untouched (#337 owns that sweep). Descriptions are
  not part of the structural OpenAPI pins.
- **T30, ambient-read wording.** The module comments of `deliverability.rs` and
  `dns.rs`, the guide and `llms.txt` name both reads: DKIM's `x=` (0125 §8,
  accepted) and SPF's lookup timer (§8 above, hold 2).
- **T31–T32, DMARC selection** (`Kind::is_ours`, `Kind::select`). The version
  tag follows RFC 7489 §6.4. Two or more `v=DMARC1` values at a name are no
  record there. The "exactly one record" hint moved from `dmarc.permerror.v1`
  to `dmarc.none.v1`, which changes served hint bytes: the guide, the bulk-sender
  test and the MCP structured-output test and fixture follow (the fixture's hash
  and provenance were recomputed by hand). A declared value is printable ASCII,
  so the tab forms of the tag are proved on the selector itself, not through a
  policy. Fixtures: `dmarc_version_tag_grammar` (AC21),
  `two_dmarc_records_at_a_name_are_no_record` (AC10, AC21).
  `v=DMARC1 p=reject` (no `;`) is a record by the tag rule and a permerror by the
  crate's parse; that is left as the crate decides.
- **T33, null-sender baseline.** `null_sender_aligns_through_the_helo_identity`
  (AC23): HELO `mx.example.com` aligns, HELO `mx.example.net` fails with
  `dmarc.fail.reject.v1`. The older HELO-identity case in the facts-table test
  and the null-sender wire test in `tests/deliverability.rs` remain.
- **T34, AC18's base.** `git diff --exit-code c95386e -- Cargo.toml Cargo.lock
  evals/Cargo.lock deny.toml` is empty. `git diff c95386e 7478073 -- Cargo.lock
  evals/Cargo.lock deny.toml` changes only the `swarmail` package's own `version`
  line (0.4.0 to 0.3.0). The earlier proof against `7478073` is superseded for
  that reason; it is not rewritten.
- **RFC 9989.** Its text was not consulted (no copy in the realm, no web
  access). The readings of §5 and §7 rest on RFC 7489 and on the pinned crate.
- **Gates**, all `--locked`, on this tree: fmt, clippy, `cargo test --workspace`,
  the evals crate's fmt, clippy and tests against this tree's `swarmail`, the
  coverage gate (exact 100/100/100) and its selftest, the publish-guard and seal
  proofs, and `cargo deny check licenses`. The sandbox has no `brokkr`
  binary, so the bundle-compile gate runs in verify.

## Dated acceptance note — 2026-11-07

The operator ruled on 2026-11-07, in chat to the dispatcher, "accept all
three", for the three holds the header names. Each was put to them with its
radius:

1. **The amendment of accepted 0120** is accepted in all three parts:
   - 1a, the optional `spf_client_ip` member;
   - 1b, the unrecorded `np=` existence answer and SPF's fixed non-TXT
     negatives, each reproducible from the embedded policy;
   - 1c, the existence rule over the declared names for `np=`.

   0120 carries a dated pointer to this note.
2. **SPF's 20-second monotonic lookup timer** is accepted as a further
   amendment of accepted 0085 §8 (after 0125 §8's `x=` read), and as the second
   stated exception to accepted 0120 §5's byte-identical reports. It is bounded
   as §8 states and fails closed. 0085 carries a dated pointer.
3. **The reading of accepted 0089 §5** is accepted: §6 meets it through the
   facts' determinants (mail identity with the envelope observation, policy
   snapshot, rule set). 0089 carries a dated pointer.

The ruling lifts the 0045 §8 holds on #255. The rest of the record, compatible
work that held nothing, is accepted with it.
