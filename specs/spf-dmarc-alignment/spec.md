# SPF and DMARC alignment (issue #255)

Decision: 0126 — [SPF for the observed envelope, DMARC by the RFC 9989 tree walk](../../docs/decisions/0126-spf-dmarc-alignment.md) (proposed; three operator holds, five named items)

- **Epic:** #251, phase 1, P1. Same crate as the DKIM story.
- **Run:** `issue-255-evaluate-spf-and-dmarc-d2898123`, `claude-careful`, from
  `main` `7478073` (which carries #254 as #336); continued as
  `issue-255-evaluate-spf-and-dmarc-2586f754` after `main` `c95386e` (v0.4.0)
  was merged in at `6653277`, and returned from review once (the design
  return below).
- **Authorities read:** house rules and `deny.toml`; accepted
  [0085](../../docs/decisions/0085-deliverability-report.md) with its dated
  pointer; accepted [0089](../../docs/decisions/0089-bulk-sender-checklist.md);
  accepted [0120](../../docs/decisions/0120-deliverability-dns-policy.md),
  [0121](../../docs/decisions/0121-deliverability-link-checks.md),
  [0122](../../docs/decisions/0122-deterministic-spam-rules.md) and
  [0125](../../docs/decisions/0125-offline-dkim-verification.md) (all four
  accepted by the operator on 2026-11-06, after this design was first
  committed); the pinned `mail-auth-0.14.0` source.

## Feature and rationale

A deliverability report can say today whether each DKIM signature verifies
(#254), but its `bulk.*` checklist entries for DMARC presence, From alignment
and an authenticated sender are `not_evaluated` on every message, because no
code produces SPF or DMARC evidence. Every report is therefore disqualified at
every threshold, and the await clause #254 promised cannot hold.

This feature evaluates, inside the lazy report evaluator and only through the
declared DNS policy:

- **SPF** for the SMTP envelope: the MAIL FROM identity, or the HELO identity
  when MAIL FROM is null, against the connecting address as Swarmail saw it
  (or as the policy declares it);
- **DMARC** policy discovery by the RFC 9989 tree walk (organizational
  domain, `sp=`, `np=`, `t=`), relaxed and strict alignment of SPF and of
  #254's verified DKIM identities, and the applied disposition;
- the six `AuthEvidence` facts the `bulk.*` checklist reads, so the checklist
  gets real evidence;
- results named the way Authentication-Results (RFC 8601) names them.

It needs two envelope facts Swarmail does not store yet, the connecting
address and the HELO name. They are captured on the SMTP path, in the row, at
ingest (0005), never from headers.

## Scope

In scope:

- capture and persistence of `smtp_client_ip` and `smtp_helo`;
- one optional declared `spf_client_ip` in 0120's policy;
- SPF evaluation, record selection, lookup limits, unsupported terms;
- DMARC From validity, discovery, record selection, disposition, alignment;
- the facts, the reordering that lets `bulk.*` read them, `swarmail-html/7`;
- #254's deferred await clause (0125 AC11);
- docs, `llms.txt`, OpenAPI and MCP schema for the policy member;
  integration notes on 0120 and 0125.

Out of scope:

- A, AAAA, MX and PTR records in the declared policy (0126 §4);
- a public-suffix list (0126 §7);
- exposing the envelope facts on REST, MCP, OpenAPI, webhooks or feeds;
- a report `auth` member or any top-level report shape change;
- ARC (#256), real DNS, a `Real` policy variant, any network lookup;
- the HELO check in addition to MAIL FROM (one identity per message);
- DMARC reporting (`rua`/`ruf`) and SPF reporting (`ra`/`rp`/`rr`).

## Operator holds

Three holds against three accepted records, five named items. 0126's header
owns the list and each item's remedy if refused; this section repeats it by
number. The delivery PR is not merged on green: under 0045 §8 it waits for
the operator's acceptance of every item.

1. **An amendment of accepted 0120** (operator, 2026-11-06):
   - **1a, §2, the declaration shape:** the optional `spf_client_ip` member,
     present on output only when declared;
   - **1b, §3, "the only path that answers, and it records every name it
     answers":** DMARC's `np=` existence answer (`Lookups::exists`) and SPF's
     fixed A/AAAA/MX/PTR negatives are answered and not recorded;
   - **1c, §4, "NXDOMAIN and NODATA are not distinguished":** they are
     distinguished for `np=` by the declared-subtree rule.
2. **SPF's monotonic `LookupLimit` timer** (0126 §8): an amendment of accepted
   0085 §8 (as amended by accepted 0125 §8), and the second stated exception
   to accepted 0120 §5's byte-identical reports. One mechanism, ruled once.
3. **A reading of accepted 0089 §5** ("Include all effective facts/policy
   identity in Inputs equality"), met by the facts' determinants (0126 §6).

Not holds: no crate, feature, lockfile or allowlist change; no `.github/`,
`scripts/`, `.brokkr/` or toolchain change; the report's top-level shape is
unchanged (0085 §1 not amended); accepted 0125 is consumed, not amended;
accepted 0121 §5 and 0122 §1.4 are followed (`swarmail-html/7`); the
null-sender baseline and DMARC record selection touch no accepted record.

## Contract

### Dependency

`mail-auth = { version = "=0.14.0", default-features = false, features =
["aws-lc-rs"] }`, unchanged. `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock`
and `deny.toml` are byte-identical to `main` `c95386e`, the merge-base (whose
dependency set is `7478073`'s). The crate's `spf` and `dmarc`
modules are not feature-gated. If implementation finds that SPF or DMARC needs
a feature or crate #254 did not add, it stops and reports.

### Envelope observation

| field | type | set by | value |
|---|---|---|---|
| `smtp_client_ip` | `Option<IpAddr>` | SMTP session, at accept | the TCP peer, `to_canonical()` |
| `smtp_helo` | `Option<String>` | SMTP session, last `HELO`/`EHLO` | the trimmed argument; `None` if empty or no greeting; cleared by `STARTTLS` |

- Copied onto the `Email` after `build_email`, before `insert_async`, inside
  the existing offload. `build_email`'s signature is unchanged.
- `None` for every non-SMTP constructor and for every row stored before the
  columns existed. Never backfilled, never read from a header (`Received:`,
  `Return-Path:`, `X-Forwarded-For:`, an inbound `Authentication-Results:` or
  any other).
- Persisted as nullable `ADDED_COLUMNS` `smtp_client_ip TEXT` and
  `smtp_helo TEXT`. An unparseable stored address refuses the restore.
- `#[serde(skip)]`: no REST, MCP, OpenAPI, webhook or feed member;
  `FIELD_NAMES` stays 21.

### The declared client address

0120's declaration shape becomes
`{"txt":[…],"tempfail":[…],"spf_client_ip":"<ip>"}` with `spf_client_ip`
optional on input and present on output only when declared.

- The value is an IPv4 or IPv6 literal; it is stored canonical
  (`to_canonical`, then the standard text form). Anything else refuses the
  declaration (`400` on the API; a startup refusal for the server file, before
  any listener binds, 0120 §9). It counts toward the byte bound, not the
  entry bound.
- It replaces the observed address for SPF only, for every peer. It never
  creates an observation: a row without `smtp_client_ip` is not evaluated.
- It is part of the effective policy, hence of the cache key and of the
  report's `dns.policy`.

### SPF evaluation

Identity (one `check_host`):

| `envelope_from` | `smtp_client_ip` | `smtp_helo` | evaluation |
|---|---|---|---|
| `None` | any | any | not evaluated |
| any | `None` | any | not evaluated |
| `Some("")` | `Some` | `None` | not evaluated |
| `Some("")` | `Some` | `Some(h)` | `SpfParameters::helo(ip, h, HOST)` |
| `Some(a)` | `Some` | `h` or `""` | `SpfParameters::mail_from(ip, h, HOST, a)` |

`ip` is `spf_client_ip` when declared, else `smtp_client_ip`. `HOST` is a
fixed constant (`swarmail.invalid`). "Not evaluated" emits
`spf.not_evaluated.v1` (info) and leaves both SPF facts Unknown.

The SPF adapter (per evaluation, over `Lookups`):

- **TXT:** a name outside 0120's grammar is "no record" and is not recorded.
  `tempfail` is a resolver error (temperror). Otherwise the declared values
  are filtered to those that begin, case-insensitively, `v=spf1` followed by
  a space or the end: none is "no record"; more than one is a parse error
  (permerror); exactly one is parsed as `SpfRecord`, its `ra` set to `None`,
  and answered. A parse failure is a parse error.
- **A, AAAA, MX, PTR:** every query is answered with a negative and marks
  the evaluation *unsupported*.
- The call is driven with `now_or_never()` and must complete on the first
  poll.

Verdict: marked → `spf.unsupported_mechanism.v1`; else the crate's result.

### DMARC evaluation

1. **From validity.** Count root `from` fields in Swarmail's header list.
   0 or ≥2 → `dmarc.from_invalid.v1`. Exactly one → parse with
   `AuthenticatedMessage` (reusing #254's parse when the message is signed);
   a parse failure, a body offset that differs from Swarmail's header size,
   or #254's boundary guard firing → `dmarc.not_evaluated.v1`. Otherwise call
   `verify_dmarc`; an empty `DmarcOutput::domain()` (no domain, or several
   distinct domains) → `dmarc.from_invalid.v1`.
2. **Discovery** (inside `verify_dmarc`): the crate's tree walk over the
   DMARC adapter. A walk temperror → `dmarc.temperror.v1`; a walk parse error
   → `dmarc.permerror.v1`; no applicable record (none found, or one without
   `p=` and without `rua=`) → `dmarc.none.v1`.
3. **Result**, when a record applies: `pass` emits nothing; `fail` emits
   `dmarc.fail.<disposition>.v1` from `DmarcOutput::policy()`
   (`none`|`quarantine`|`reject`); an Unknown result emits
   `dmarc.not_evaluated.v1`. The result is 0089 §5's formula over the facts:
   `valid_from ∧ ((dkim_pass ∧ dkim_aligned) ∨ (spf_pass ∧ spf_aligned))`.

The DMARC adapter (per evaluation, over `Lookups`):

- **TXT:** as SPF's, with the version filter `v` *WSP `=` *WSP `DMARC1`
  followed by whitespace, `;` or the end (RFC 7489 §6.4 ABNF: `v`
  case-insensitive, `DMARC1` case-sensitive, space or tab around `=`); values
  that fail it are discarded. None left, **or more than one left**, is "no
  record" at that name (`not_found`), so the walk continues (RFC 7489 §6.6.3
  step 5; 0126 §7). Exactly one is parsed as `DmarcRecord`; a parse failure
  is a parse error.
- **A** (the `np=` existence query): NODATA when the queried name, or any
  declared name below it (TXT owner or `tempfail`), is declared; NXDOMAIN
  otherwise. Not recorded.
- **AAAA, MX, PTR:** never asked by `verify_dmarc`; answered negative.

The inputs of the one `verify_dmarc` call:

- `dkim_output`: #254's crate outputs, retained only where #254's verdict for
  the paired field is a pass, `dkim.body_length_partial.v1` (the `l=` warning)
  or `dkim.not_evaluated.v1` (a declared temporary failure); every other
  verdict is dropped.
  Unsigned mail passes an empty slice.
- `mail_from_domain`: the evaluated SPF identity's domain
  (`SpfOutput::domain()`), or `""` when SPF was not evaluated. For the null
  sender that is the HELO identity's domain: the pinned crate's RFC 9989
  contract for `DmarcParameters::mail_from_domain` ("or the HELO domain for a
  null reverse-path"), as RFC 7489 §4.1 provides through RFC 7208 §2.4 (0126
  §5). Unchanged by the design return; the remedy if the operator reads RFC
  9989 otherwise is one arm.
- `spf_output`: `SpfOutput::new(domain).with_result(SpfResult::Pass)` when
  SPF was evaluated (so the crate reports identity alignment), else
  `SpfOutput::new("").with_result(SpfResult::None)`.

### Authentication facts

Computed in the evaluator before `bulk.*` runs. `Inputs.auth` survives only
as a `#[cfg(test)]` override; `policy_id` and `observation_id` are deleted.

| fact | True | False | Unknown |
|---|---|---|---|
| `valid_from` | one root From, crate parse agrees, one domain | 0 or ≥2 From fields, or no single domain | crate parse fails or disagrees; #254 boundary guard fired |
| `policy_present` | an applicable record was found | walk found none, record does not apply, or permerror | walk temperror; `valid_from` not True |
| `dkim_pass` | some field's #254 verdict is pass or `dkim.body_length_partial.v1` | no signature, or every field failed definitively | none passed and one is `dkim.not_evaluated.v1`; #254 stopped at its boundary or bounds |
| `dkim_aligned` | `DmarcOutput::dkim_result()` is Pass | it is Fail or None | it is TempError; no applicable record; `valid_from` not True |
| `spf_pass` | `check_host` is Pass | Fail, SoftFail, Neutral, None, PermError | TempError; unsupported; not evaluated |
| `spf_aligned` | `DmarcOutput::spf_result()` is Pass | it is Fail (not aligned) | it is TempError; SPF not evaluated; no applicable record; `valid_from` not True |

`spf_aligned` is identity alignment and does not depend on the SPF result, so
an aligned MAIL FROM rejected by its record is aligned but not authenticated,
exactly the distinction between `bulk.from_alignment` and
`bulk.authenticated_sender`.

### Rule table

A pass emits nothing. Every hint is static and never echoes sender data
(0089 §7). Each rule has a passing and a failing fixture (AC6–AC12).

| rule id | severity | location | fires when |
|---|---|---|---|
| `spf.fail.v1` | error | `return-path`, null | `check_host` = fail |
| `spf.softfail.v1` | error | `return-path`, null | = softfail |
| `spf.neutral.v1` | error | `return-path`, null | = neutral (incl. no match) |
| `spf.none.v1` | error | `return-path`, null | = none (no record; invalid identity domain) |
| `spf.temperror.v1` | error | `return-path`, null | = temperror (declared `tempfail`) |
| `spf.permerror.v1` | error | `return-path`, null | = permerror (syntax, multiple records, missing include/redirect target, the 11th term, the §8 timer) |
| `spf.unsupported_mechanism.v1` | error | `return-path`, null | evaluation reached `a`, `mx`, `ptr`, `exists` or `%{p}` |
| `spf.not_evaluated.v1` | info | `return-path`, null | no observed peer, unknown envelope, or null MAIL FROM without HELO |
| `dmarc.from_invalid.v1` | error | `from`, null/1/0 | not exactly one From field, or no single domain |
| `dmarc.none.v1` | error | `from`, 0 | no applicable record (including a walk whose only `v=DMARC1` name held more than one value, all discarded) |
| `dmarc.temperror.v1` | error | `from`, 0 | a walked `_dmarc` name is declared `tempfail` |
| `dmarc.permerror.v1` | error | `from`, 0 | a walked name holds exactly one `v=DMARC1` value and it does not parse |
| `dmarc.fail.none.v1` | error | `from`, 0 | result fail, applied policy none |
| `dmarc.fail.quarantine.v1` | error | `from`, 0 | result fail, applied policy quarantine |
| `dmarc.fail.reject.v1` | error | `from`, 0 | result fail, applied policy reject |
| `dmarc.not_evaluated.v1` | error | `from`, 0 | From cannot be judged, or a record applies and the result is Unknown |

The `return-path` location with `occurrence: null` names the envelope sender
(RFC 5321 §4.4) and says no stored header is the source; it is null whatever
headers the message carries. No `Location` variant is added.

### Authentication-Results correspondence (normative)

| A-R | report |
|---|---|
| `spf=pass` | no `spf.*` finding |
| `spf=<fail\|softfail\|neutral\|none\|temperror\|permerror>` | `spf.<token>.v1` |
| (no `spf=` clause: SPF not run) | `spf.not_evaluated.v1` or `spf.unsupported_mechanism.v1` |
| `dkim=pass` | no `dkim.*` error for that field (#254, unchanged) |
| `dkim=<other>` | #254's ids, unchanged |
| `dmarc=pass` | no `dmarc.*` finding |
| `dmarc=fail` | `dmarc.fail.<disposition>.v1` |
| `dmarc=none` | `dmarc.none.v1` (a valid From with no applicable record) |
| `dmarc=temperror` | `dmarc.temperror.v1` |
| `dmarc=permerror` | `dmarc.permerror.v1` or `dmarc.from_invalid.v1` |
| (no `dmarc=` clause) | `dmarc.not_evaluated.v1` |

The `bulk.*` checklist reads the facts: `bulk.dmarc_presence` from
`policy_present`, `bulk.from_alignment` and `bulk.authenticated_sender` from
0089 §5's formulas. A failing DMARC message therefore carries both its
`dmarc.*` id and its `bulk.*` entry.

### Organizational domain and public suffix

The crate's RFC 9989 tree walk decides it from the declared `_dmarc` records
alone: at most eight TXT names per walk; `psd=n` marks the organizational
domain, `psd=y` marks a public suffix domain (the organizational domain is one
label below it); otherwise the shortest name with a record. No public-suffix
list exists in the tree. Pinned by `mail-auth =0.14.0`.

### Work bound and lookup limits

- SPF: ten DNS-querying terms admitted, the eleventh permerror (the crate's
  counter); at most twelve TXT names per evaluation; void lookups vacuous
  (0126 §5); no MX exchange expansion (unsupported).
- DMARC: at most eight names for the Author Domain's walk, plus at most eight
  per distinct identity domain the crate walks for relaxed alignment
  (memoized), so at most `8 × (2 + 16)` names; each recorded once.
- The 256-finding and 65,536-byte report bounds stand; past them,
  `AnalysisError::Failed`.

### Ambient reads

| read | where | reachable | radius |
|---|---|---|---|
| `Instant::now()`/`elapsed()` | `spf/verify.rs` `LookupLimit` | yes, every SPF evaluation | ≥20 s inside one synchronous poll turns the verdict into `spf.permerror.v1` (fail-closed). **Operator hold 2.** |
| `SystemTime::now()` + thread-local | `sampling.rs` `is_within_pct` via `with_report` | no: the adapter clears `ra=` | none |
| `SystemTime::now()` | `spf/macros.rs` `current_time` (`%{t}`) | no: `exp=` text is never parsed as a `Macro` | none |
| any | `dmarc/verify.rs` | none present | none |
| resolver | all caches | no: every cache answers; no name servers | none |

### Report and version

`RULE_SET_ID` = `swarmail-html/7`. Shape `{rule_set_id, findings, dns}`
unchanged; `dns.policy` gains `spf_client_ip` only when declared;
`dns.lookups` gains the SPF and DMARC TXT names and stays TXT-only.

### Live surfaces

- REST: `PUT /api/v1/inboxes/{inbox}/dns` accepts `spf_client_ip`; reports
  carry the new findings. OpenAPI's policy schema gains the optional member.
- MCP: the report output and any policy schema mirror the same member.
- `serve --dns-policy FILE` accepts the member, checked before listeners bind.
- Docs: `docs/guide/deliverability.md`, `src/static/llms.txt`, the rule list
  wherever rule ids are listed, and `docs/decisions/README.md`.

## Frozen guarantees and failure modes

| concern | answer | what is lost |
|---|---|---|
| 0001 accept == stored | the two facts are fields of the `Email` the one synchronous insert stores, set before `insert_async`; the `250` still follows the commit | nothing |
| 0002 await | unchanged code; an await reads reports as before | nothing |
| 0003 pruning | unchanged code; the facts leave with their row | nothing |
| 0005 ingest | capture is a field copy at ingest; evaluation stays lazy | nothing |
| crash between accept and commit | no row, no `250`; the client retries | the session, as always |
| restored pre-#255 data file | NULL columns read `None`; SPF not evaluated; DMARC through DKIM only | SPF for old mail, forever (never backfilled) |
| peer behind a proxy or bridge | the TCP peer is evaluated honestly; a test declares `spf_client_ip`; no forwarding header is trusted | SPF of the real origin, unless declared |
| hostile DATA | SPF reads no header and no body; DMARC reads only the bounded root `From`; ambiguity is a named finding | nothing guessed |
| hostile policy | 0120's bounds and parser refuse before use; `spf_client_ip` must be an IP literal | startup, with a named refusal |
| `tempfail` | `*.temperror.v1`, facts Unknown, report never qualifies | the verdict under a temporary failure |
| SPF record using `a`/`mx`/`ptr`/`exists` | `spf.unsupported_mechanism.v1`, `spf_pass` Unknown | that SPF verdict, until address records are declared by a later story |
| descheduled process (≥20 s) | the crate's timer yields `spf.permerror.v1` (hold 2) | determinism inside that window |
| many parallel reports | per-evaluation adapters, no global mutable state, 0085's four permits | nothing new |
| report too large | `AnalysisError::Failed` | the report, never a truncated one |

## Panel adjudication — 2026-11-06

Both positions passed and conflict on eight points. Each is ruled; neither is
averaged.

1. **Envelope capture — both agree; simplicity's mechanics adopted.** Same-row
   capture before the insert (robustness D1) by setting two fields after
   `build_email` (simplicity A), avoiding churn in every `build_email`
   caller. Robustness's "no side map, no second write" is the binding
   constraint and is met. Robustness's `None`-versus-`Some("")` distinction
   for the envelope is adopted into the identity table.
2. **Client address — robustness's need, simplicity's fallback form.** An
   override exists (the brief asks how a test declares it), as one optional
   policy member applying to every peer. Robustness's loopback gate is
   rejected: the policy author already controls every record, and a
   container bridge is not loopback. *Cost to simplicity:* one grammar member,
   schema and fixture churn. *Cost to robustness:* none in safety.
3. **Record types — simplicity wins.** TXT-only, with
   `spf.unsupported_mechanism.v1`. Robustness's objection ("we did not look"
   becomes "the sender failed") is answered by making `spf_pass` Unknown and
   the id Swarmail's own. *Cost to robustness:* a production record that
   reaches `a`/`mx` cannot be evaluated as is.
4. **Organizational domain — simplicity wins.** The crate implements RFC
   9989's tree walk and keeps its helpers private, so a public-suffix list
   would be a second, disagreeing owner. Robustness's `co.uk` and `github.io`
   cases are answered by the walk itself: they misalign only if the declared
   world publishes records there, and a declared `psd=y` makes them suffixes.
   *Cost:* no PSL realism; nothing that a declared record cannot express.
5. **Facts — simplicity wins on placement, robustness on completeness.**
   Facts are computed in the evaluator (computing them before the cache would
   break 0085's bounds); all six are filled, `dkim_pass` included (robustness).
   `policy_id`/`observation_id` are deleted (simplicity), not digested.
6. **Reporting — neither wholly.** No `auth` member (simplicity: it would
   amend accepted 0085 §1). But DMARC gets its own ids with the applied
   disposition (against simplicity), so `sp=`, `np=` and `t=` are observable
   as the brief requires. *Cost:* double reporting of DMARC failures with
   `bulk.*`.
7. **Ambient reads — neither position's inventory was complete.** Robustness
   claimed both clock reads could be removed; the `LookupLimit` timer cannot,
   so it is a hold. Simplicity left the sampling read reachable as harmless;
   the adapter now clears `ra=`, making it unreachable. `%{t}` is unreachable
   (simplicity, confirmed in source). Robustness's "refuse every macro" is
   rejected.
8. **DKIM evidence — robustness adopted.** DMARC consumes #254's verdicts,
   not the crate's raw results: a signature #254 refuses (for example
   `rsa-sha1` or an unsigned `From`) never aligns.

Record selection (one `v=spf1`, one `v=DMARC1`) is added by the chief, on
0125's principle that Swarmail owns every check the crate settles by a silent
default.

## Design return adjudication — 2026-11-06

Review 1 returned the design (`spec_defect: true`, medium) for one defect that
four of five members reported: 0126's hold statement predates the operator's
acceptance of 0120, 0121, 0122 and 0125 and names one hold where the record
amends accepted text in three places. Security found nothing. Both positions
passed. They agree on the defect and on keeping the mechanism, and disagree on
how many holds there are, on four substantive edges and on the test seam. Each
point is ruled below.

1. **Hold count: neither five (robustness) nor three flat (simplicity).**
   There are three holds, one per accepted record, with 0120's split into the
   three sentences it amends (1a §2, 1b §3, 1c §4). That gives five items the
   operator can rule on one at a time. Robustness's separate "0120 §5" hold is
   folded into hold 2: the §5 byte-identity exception *is* the SPF timer, and
   ruling on one mechanism twice invites contradictory rulings. *Cost to
   robustness:* none in protection, because §5 is named in hold 2's text.
   *Cost to simplicity:* a longer header.
2. **Recording the `np=` answer: simplicity wins.** It is named as hold 1b
   and not recorded. Recording it would widen `DnsLookup.type` past the
   accepted `txt` constant, which amends accepted 0120 §5's lookup shape (a
   fourth amendment) and breaks AC17. The answer is a pure function of
   `dns.policy`, which the report embeds, so the report stays reproducible
   from its own bytes. *Cost to robustness:* a reader of a `dmarc.fail.*`
   report must derive the `np=` existence answer from the embedded policy
   instead of reading it off `dns.lookups`. The header sentence and 0120's
   note say so, so no comment claims more than is true.
3. **Two `v=DMARC1` values: both positions, under RFC 7489's text.** They
   are discarded as "no record at this name" (`dmarc.none.v1` when nothing
   else applies). RFC 7489 §6.6.3 says multiple records mean the policy is
   not applied, and §11.2 reserves `permerror` for a syntactically bad
   record. The realm has no copy of RFC 9989's text. The crate's walk treats
   any name without a record by moving on, and the selector uses that path,
   so the change is one match arm. AC10's old "two values →
   `dmarc.permerror.v1`" example is **wrong, not weakened**: it gave the
   RFC's "not applied" case the error token for a broken record. It moves to
   AC21 with the correct expectation and a second fixture (the walk reaching
   an organizational record). *Cost:* when a parent record applies, the
   duplicate yields no finding of its own. It stays visible in
   `dns.lookups`.
4. **`v=DMARC1` grammar: robustness wins over "pin what is there".** It
   follows RFC 7489 §6.4's ABNF: `v` is case-insensitive, whitespace is
   allowed around `=`, and `DMARC1` is case-sensitive. Today `v = DMARC1`, a
   valid record, reads as no policy and `v=dmarc1`, an invalid one, reads as
   a policy. Pinning that with a fixture would prove a wrong behaviour. The
   crate's parser accepts both forms, so Swarmail's selector is what decides,
   and it should decide correctly. *Cost:* one function and four fixtures.
   SPF's `v=spf1` (no whitespace, case-insensitive, RFC 7208 §4.5) is right
   as it is.
5. **HELO under the null sender: simplicity's "state and cite" wins over
   robustness's RFC 9989 drop.** The pinned crate implements RFC 9989 and
   documents `mail_from_domain` as "the HELO domain for a null
   reverse-path". With no RFC 9989 text in the realm, the crate is the
   authority here, as 0125 made it for DKIM. Robustness's safety argument
   does not hold: the HELO identity aligns only if its own SPF record
   authorizes the connecting address *and* its domain aligns with From, so
   no one gains alignment without authenticating. The baseline is stated in
   0126 §5 and pinned by AC23, with a one-arm remedy if the operator reads
   RFC 9989 otherwise. It is not a hold, because no accepted record is
   touched. *Cost to robustness:* if RFC 9989 does drop HELO, null-sender
   reports can show `spf_aligned` True where the RFC has no identifier.
6. **The `cfg(test)` `Inputs.auth` default: rejected (robustness).** The
   default of unknown facts exists only in the library's test build. Every
   `tests/` suite links the production build, which has no such input, and
   AC13, AC14 and AC21–AC23 run the real evaluation, inline with the override
   unset or over the wire. Correctness found it acceptable and the tests
   lens found no vacuity. A test asserting the default would prove nothing
   about production. *Cost:* a future inline test that forgets to opt out
   proves only the checklist. The wire suites still cover production.
7. **Stale served text, ambient-read wording, AC18's base: adopted from both
   positions and the review (AC18, AC24).** These are sentences #255 itself
   made false. The "proposed 0120/0121/0125" status wording in served text
   #255 did not author stays for #337, as the continuation directs.
8. **No redesign (simplicity).** Capture, provenance, TXT-only with the
   named unsupported term, no public-suffix list, no `auth` member, and
   facts in the evaluator all stand. Both positions defend them and all five
   reviewers found them sound.

## Acceptance Criteria

Each criterion names its proof. "Fixture pair" means one fixture that fires
the rule and one that does not, both run.

- **AC1: Capture.** A message sent over real SMTP from loopback stores
  `smtp_client_ip = 127.0.0.1` (an IPv4-mapped peer is canonicalized) and the
  last `EHLO`/`HELO` argument. No greeting, or an empty argument, stores
  `None`. A `STARTTLS` without a later greeting stores `None`. REST-seeded and
  MailSlurper-shim mail store `None` for both. A `Received:`,
  `X-Forwarded-For:` or `Authentication-Results:` header never changes either.
  *Proof:* inline `smtp.rs` tests; `tests/deliverability.rs` over the wire.
- **AC2: Persistence.** Both columns persist and restore. A data file written
  before the columns existed opens, keeps every row, and restores both as
  `None` with no backfill. A stored `smtp_client_ip` that is not an address
  refuses the restore. *Proof:* inline `persist.rs` tests;
  `tests/persistence.rs`.
- **AC3: Frozen guarantees.** `Store::insert`, the await path and pruning
  are unchanged in code; the `250` follows the commit; the burst, p2 and rate
  suites pass. *Proof:* the diff; `tests/burst.rs`, `tests/p2.rs`,
  `tests/rate.rs`.
- **AC4: No new mail surface.** REST, MCP, webhook and feed payloads of an
  `Email` are byte-identical to `7478073`'s for the same mail; `FIELD_NAMES`
  is 21; the OpenAPI and MCP mail schemas are unchanged. *Proof:* existing
  pinned payload tests pass unchanged (the payloads are also `c95386e`'s).
- **AC5: Declared client address.** `spf_client_ip` is accepted from the
  server file and the inbox `PUT`, canonicalized, echoed in `dns.policy`
  only when declared, and refused (400 / startup refusal) when not an IP
  literal. Declared, it decides SPF for a loopback peer (a record listing
  only the declared address passes; one listing only `127.0.0.1` fails). It
  never makes a row without an observed peer evaluable. *Proof:* inline
  `dns.rs` and auth tests; `tests/deliverability.rs`.
- **AC6: SPF results.** Fixture pairs for each of `spf.fail.v1`,
  `spf.softfail.v1`, `spf.neutral.v1`, `spf.none.v1`, `spf.temperror.v1`,
  `spf.permerror.v1` and `spf.not_evaluated.v1`; a passing record emits no
  `spf.*` finding. The MAIL FROM identity is used for a non-null sender and
  the HELO identity for the null reverse path. *Proof:* inline auth tests.
- **AC7: SPF record selection.** Two `v=spf1` values at a name give
  `spf.permerror.v1`; one `v=spf1` value beside unrelated TXT values is
  evaluated; a `v=spf10` value is not an SPF record. *Proof:* inline.
- **AC8: Unsupported terms.** Reaching `a`, `mx`, `ptr`, `exists` or a `%{p}`
  macro gives `spf.unsupported_mechanism.v1` with `spf_pass` Unknown (the
  bulk entries are `not_evaluated`, not failures); `ip4:127.0.0.1 a -all`
  passes for `127.0.0.1`. *Proof:* inline fixture pairs.
- **AC9: Lookup limits.** A chain of ten `include:` terms passes and eleven
  gives `spf.permerror.v1`; a `redirect=` as the eleventh term gives
  `spf.permerror.v1`; an `include:` of an undeclared name gives
  `spf.permerror.v1` at the first void answer; `a a a -all` gives
  `spf.unsupported_mechanism.v1`. An `exp=` whose text contains `%{t}` is
  looked up and recorded but never expanded, and a fail stays
  `spf.fail.v1`. No SPF evaluation records more than twelve names. A From
  with ten labels records at most eight `_dmarc` names for its walk.
  *Proof:* inline.
- **AC10: DMARC discovery and disposition.** Fixture pairs for
  `dmarc.none.v1`, `dmarc.temperror.v1`, `dmarc.permerror.v1` (one
  unparseable `v=DMARC1` value; multiplicity is AC21), `dmarc.fail.none.v1`,
  `dmarc.fail.quarantine.v1` and `dmarc.fail.reject.v1`. A subdomain From
  with only an organizational record finds it (`policy_present` True) and
  applies `sp=`; an Author Domain record applies `p=`; a non-existent
  subdomain (nothing declared under it) applies `np=`; `t=y` downgrades
  reject to quarantine; a `psd=y` record stops the walk and sets the
  organizational domain one label below it; `pct=` changes nothing.
  *Proof:* inline.
- **AC11: Alignment.** Relaxed and strict alignment for SPF (`aspf`) and DKIM
  (`adkim`) with a subdomain identity, each a fixture pair. An unverified
  signature whose `d=` equals the From domain does not align. A signature the
  crate verifies but #254 refuses (an unsigned `From`, `rsa-sha1`) does not
  align. A MAIL FROM aligned with From but rejected by its record is aligned
  and not authenticated (`bulk.from_alignment` clean,
  `bulk.authenticated_sender.v1` present). *Proof:* inline.
- **AC12: From validity.** Fixture pairs for `dmarc.from_invalid.v1` (no From,
  two From fields, two distinct domains in one field) and
  `dmarc.not_evaluated.v1` (a From that the crate parses differently from
  Swarmail's header boundary). `Email.from` is never read: a message without
  a From header, whose envelope sender's domain has a passing DMARC setup, is
  still `dmarc.from_invalid.v1`. *Proof:* inline.
- **AC13: Facts feed the checklist.** With the test override unset, a real
  evaluation sets all six facts per the facts table; the `bulk.*` entries
  follow 0089 §5 (truth tables still pass through the override). A pre-#255
  row with a passing DKIM setup is clean of `bulk.authenticated_sender*`
  through DKIM alone. Two requests with equal keys give equal facts.
  *Proof:* inline `deliverability.rs` tests; `tests/bulk_sender.rs`.
- **AC14: The await clause (0125 AC11, inherited).** Over real SMTP, with a
  per-inbox policy declaring a DKIM key, `v=spf1 ip4:127.0.0.1 ip6:::1 -all`
  and a `_dmarc` record, a signed message that satisfies the whole checklist
  is returned by an await with `no_findings_at_or_above=warning`, its REST
  and MCP reports carry no `spf.*`, `dkim.*`, `dmarc.*` or `bulk.*` finding at
  warning or above, and a copy with an altered body is not returned and keeps
  `dkim.body_hash_mismatch.v1`. *Proof:* `tests/deliverability.rs`.
- **AC15: No resolver, one clock.** Every SPF and DMARC cache answers every
  query (a test asserts `Some` from each), and each crate call completes on
  the first poll. A record with `ra=` and `rp=0` evaluates with no sampling
  (the adapter's record has no `ra`). The only reachable clock read is the
  `LookupLimit` timer, recorded as hold 2. *Proof:* inline.
- **AC16: Determinism.** Two evaluations of the same `Email` and policy give
  byte-identical report JSON, and so do two in-process servers with separate
  stores, outside the §8 window. *Proof:* inline; `tests/deliverability.rs`.
- **AC17: Version and shape.** Every report carries `swarmail-html/7`; the
  top-level members are exactly `rule_set_id`, `findings`, `dns`;
  `dns.lookups[].type` is `txt` only. *Proof:* existing shape tests, updated
  for the id.
- **AC18: Dependency.** `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock` and
  `deny.toml` are byte-identical to `main` `c95386e`, the merge-base (the
  v0.4.0 version bump makes `7478073` the wrong base since the merge);
  `cargo deny check licenses` passes. *Proof:* `git diff --exit-code c95386e
  -- Cargo.toml Cargo.lock evals/Cargo.lock deny.toml`, and
  `git diff c95386e 7478073 -- Cargo.lock` shows only the `swarmail` package's
  own version line (no package added or removed).
- **AC19: Gates.** Every house gate passes `--locked`: fmt, clippy, tests,
  the binary build, the evals crate's fmt/clippy/tests against the
  candidate's binary, the coverage gate at exactly 100% and its selftest, the
  publish-guard and seal proofs, `cargo deny`. The brokkr bundle compile runs
  where a pinned brokkr binary exists; otherwise verify runs it and the
  implementer says so. *Proof:* the gate log.
- **AC20: Records and docs.** 0126 is listed in `docs/decisions/README.md` as
  proposed; 0120 and 0125 carry dated integration notes (AC22 states what
  0120's must say); the guide,
  `llms.txt` and OpenAPI describe `spf_client_ip`, the new ids, the A-R table
  and the unsupported mechanisms. *Proof:* the existing doc-consistency tests
  and review.

Added on the design return (2026-11-06; plan.md says which finding each
answers):

- **AC21: DMARC record selection.** Fixture pairs, each run:
  `v = DMARC1; p=reject` (whitespace around `=`) and `v=DMARC1\t;p=reject` are
  records and apply `p=reject`, against `v=dmarc1; p=reject` and
  `v=DMARC10; p=reject`, which are not (`dmarc.none.v1` when nothing else
  applies). Two `v=DMARC1` values at the Author Domain's `_dmarc` name, with
  nothing above, give `dmarc.none.v1` and `policy_present` False, never
  `dmarc.permerror.v1`; the same two values below an organizational record
  `v=DMARC1; p=reject; sp=quarantine` let the walk find it and apply `sp=`
  (`dmarc.fail.quarantine.v1` on failure); both names and all their values
  appear in `dns.lookups`. One unparseable value stays `dmarc.permerror.v1`.
  Two `v=spf1` values stay `spf.permerror.v1` (AC7, unchanged). The
  `dmarc.none.v1` hint names "exactly one" record; the `dmarc.permerror.v1`
  hint no longer does; the guide's rule table matches the code
  (`guide_lists_every_auth_rule`). *Proof:* inline `auth.rs` fixture pairs.
- **AC22: Holds on record.** 0126's header names holds 1 (1a/1b/1c), 2 and 3
  as amendments or readings of accepted 0120, 0085 and 0089 and lists 0120,
  0121, 0122 and 0125 as accepted; it no longer says "one named hold", "no
  accepted ruling is contradicted" or "0120's proposed declaration grammar".
  The `docs/decisions/README.md` row for 0126 names the same three holds and
  says "integration notes on accepted 0120 and 0125". 0120's dated
  integration note labels the grammar member §2 (not §3) and adds the §3
  (unrecorded answers) and §4 (NXDOMAIN/NODATA for `np=`) items as hold 1,
  with the §5 timer as hold 2. 0085 and 0089 each carry a dated pointer to
  the pending item (0085 §8: hold 2; 0089 §5: hold 3) that changes no status.
  No decision's status changes. *Proof:* `grep` checks in the task list;
  review.
- **AC23: The null-sender baseline is pinned.** A null-sender message whose
  HELO name passes SPF and aligns (relaxed) with the From domain has
  `spf_aligned` True and passes DMARC through SPF; with a HELO name that does
  not align, `spf_aligned` is False. 0126 §5 states the baseline (the pinned
  crate's RFC 9989 contract) and the one-arm remedy. *Proof:* an inline
  `auth.rs` fixture pair, plus the existing wire test in
  `tests/deliverability.rs` (`mx.x.io` aligning with `x.io`).
- **AC24: Served text #255 made false is corrected.** `src/static/openapi.json`'s
  `DeliverabilityReport` description no longer says "no production message
  currently qualifies at any predicate threshold" and says the authentication
  entries read facts computed from the stored mail, the envelope observation
  and the policy; its `DnsRecord` description no longer says "SPF and DMARC
  will record theirs when they land" and says DKIM, SPF and DMARC record the
  TXT names they resolve. The "one stated exception" sentences in
  `src/deliverability.rs`, `src/deliverability/dns.rs`,
  `docs/guide/deliverability.md` and `src/static/llms.txt` name both reads
  (0125 §8's `x=` comparison and 0126 §8's SPF timer). Status wording that
  still says "proposed 0120/0121/0125" in served text #255 did not author is
  left for #337. *Proof:* any test that pins the OpenAPI bytes or the guide
  is updated in the same commit and passes; `grep` checks in the task list.
