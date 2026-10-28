# Bulk-sender checklist

Created: 2026-10-26. Status: draft design. Epic #251, issue #257.
Decision: 0089 — [proposed rules](../../docs/decisions/0089-bulk-sender-checklist.md).

## Purpose and authority

Add seven source-citing checks to #252's existing lazy deliverability report.
A program inspecting captured mail needs actionable defects and explicit missing
authentication evidence. This is a scoped Gmail/Yahoo checklist, not provider
certification or a delivery guarantee.

The operator's #251 ruling requires lazy cached analysis outside SMTP, future
authentication through an existing permissive crate, and no external scorer.
This issue needs no new dependency or verification implementation.
[Tracked intake](intake.md) provides the framing; both supplied packet files
passed their SHA256SUMS. The packet has no prior outcomes. The legacy tasks
directory is absent; tracked intake and the supplied task suffice. Published
URLs below are supplied citations, not live policy verification.
Preserve the [0085 contract](../../docs/decisions/0085-deliverability-report.md)
and accepted 0001–0005.

## User scenarios

1. After SMTP 250, a test reads a report, fixes a cited header/MIME defect, and
   sees that finding disappear on the corrected mail without any stored mutation.
2. Sender-written authentication headers still yield explicit unknowns until
   trusted observations exist. A client distinguishes unknown from failed by ID.
3. REST, current MCP text results, UI and predicates share the same report.
   Repeated reads with identical immutable inputs reuse it.

## Contract

### Findings and thresholds

Set rule_set_id to swarmail-bulk/1. Retain demo.marker.v1, its fixed hint,
first-occurrence byte behavior and demo_enabled flag. New checks run regardless
of that flag. Keep Report/Finding JSON unchanged: no pass rows or status field.
A proved pass emits nothing; a failure or unknown emits the matching ID below.
At most one finding per logical checklist rule uses the existing canonicalizer.

| Check | Failure ID | Severity, including unknown | Unknown ID |
|---|---|---|---|
| One-click structure | bulk.one_click.v1 | error | none |
| Message-ID | bulk.message_id.v1 | error | none |
| Date | bulk.date.v1 | error | none |
| Genuine plain body | bulk.plain_text.v1 | warning | none |
| DMARC presence | bulk.dmarc_presence.v1 | error | bulk.dmarc_presence.not_evaluated.v1 |
| From alignment | bulk.from_alignment.v1 | error | bulk.from_alignment.not_evaluated.v1 |
| Passing aligned sender | bulk.authenticated_sender.v1 | error | bulk.authenticated_sender.not_evaluated.v1 |

Keep inclusive info < warning < error. Production currently emits all three
authentication unknowns, excluding every message at every
no_findings_at_or_above threshold. Exact zero qualifying-message assertions
can still succeed; waits still require at least one. Preserve completed-scan
timeout counts, zero-timeout ready-only behavior and existing sanitized errors
for failed/incomplete analysis. No authentication-specific predicate exception.

Root header names are lowercase. Occurrence is null for absence, 0 for a present
invalid singleton, and 1 for the second occurrence witnessing duplicates.
One-click locates list-unsubscribe if it fails, otherwise list-unsubscribe-post.
Identity/date locate their own fields; authentication locates from under the
same null/0/1 rule. Plain text uses MimePart path []. Never echo sender content
in hints.

### Header checks

Scan original bytes, ASCII-insensitive field names and occurrence counts.
Unfold legal continuation lines to a space and trim outer space/tab. Never
lossy-decode bytes into valid tokens; require full value consumption. Stop at
the root header/body separator; malformed unrelated fields terminate prior
continuation association. Controls, invalid encoding, duplicates and trailing
garbage cannot buy a pass.

One-click requires exactly one List-Unsubscribe and one List-Unsubscribe-Post.
Parse a complete comma-separated list of angle-bracketed absolute URIs with
optional folding whitespace. Every entry must be well formed; at least one is
HTTPS with valid nonempty host/authority and optional valid port. Mailto may
coexist but alone cannot suffice. Reject broken brackets, invalid percent escapes,
URI whitespace/controls, userinfo in HTTPS, and trailing garbage. HTTPS fragments
are unsupported by this profile. Never resolve or fetch a target.
Post equals the case-sensitive literal List-Unsubscribe=One-Click after
unfolding/outer trimming; extra parameters and wrong case fail.
This proves neither DKIM coverage of those fields nor endpoint operation.
General aligned authentication is not signed-header coverage.

Message-ID and Date each require one complete original field with legal
surrounding RFC 5322 CFWS (balanced comments and legal quoted pairs).
Use current generation syntax; obsolete-only forms are unsupported by this
checklist, not universally invalid Internet mail. Do not validate a normalized
stored Message-ID or a parser's first usable token.

Message-ID follows current RFC 5322 §3.6.4: one angle-delimited id-left@id-right,
dot-atom-text left, dot-atom-text or no-fold-literal right. Empty/doubled atoms,
multiple IDs and malformed literals fail. No DNS, uniqueness or deduplication
database.

Date follows current §3.3/§3.6.1: optional weekday (must agree), day/month/year,
hour:minute with optional seconds, and numeric zone. Year >=1900, valid calendar
day/leap year, hour 00–23, minute 00–59, second 00–60 (60 is syntactically allowed,
without live leap-second verification); zone ±HHMM with HH 00–23 and MM 00–59.
Accept -0000. Named zones, obsolete years, impossible dates and suffix junk fail.
No ambient-clock freshness check or received-time change.

### Plain-text presence

Pass for a genuine sender-supplied outer-message text/plain body candidate,
including implicit default text/plain, empty bodies, nested inline multipart
children and transfer-encoded leaves. No literal multipart/alternative parent
or content equivalence is required. Exclude attachments, descendants of attached
containers and embedded message/rfc822 bodies. A multipart/digest child's default
is message/rfc822. Explicit attachment disposition or filename attachment metadata
marks attachment ancestry. HTML-only mail warns. Never synthesize text from HTML,
render HTML, rewrite stored bodies or rerun link/code extraction. This is
checklist advice, not an unconditional provider mandate.

### Trusted authentication

Extend internal immutable Inputs with three-state facts (True, False, Unknown):
applicable valid DMARC policy presence; valid unambiguous RFC5322.From; and for
each DKIM/SPF, authentication pass and policy-compatible From alignment.
All default to Unknown. Sender headers never populate facts.
#255 owns verification, normalization, strict/relaxed alignment, organizational
domains, provenance and binding observations to selected mail/policy.
#253/#255 own expiry, refresh and wakes; this issue adds no DNS or authentication.

AND: False wins, otherwise Unknown wins, otherwise True.
OR: True wins, otherwise Unknown wins, otherwise False.

- DMARC presence is its independent trusted fact; valid p=none counts as present.
- From alignment = valid From AND (DKIM alignment OR SPF alignment).
- Authenticated sender = valid From AND ((DKIM pass AND DKIM alignment) OR
  (SPF pass AND SPF alignment)).

False yields failure, Unknown yields not evaluated, True yields no finding.
Presence alone is not authentication; alignment may pass despite auth failure.
A passing aligned mechanism wins over unknown on the other; known failure plus
an unresolved possible pass stays unknown. Invalid/ambiguous trusted From
prevents alignment/authentication success. This epic's DKIM-or-SPF check does
not certify every separate provider authentication requirement.

Include all effective facts and immutable policy/observation identity in Inputs
equality before caching evaluated results. Selected Weak mail identity remains
mandatory; public Message-ID is not an evidence key. Production supplies only
Unknown. A per-Service cfg(test) seam may supply an immutable snapshot frozen at
request entry for real SMTP/REST fixtures. No public parameter, mutable ID-only
registry or prebuilt Finding injection may substitute for actual rule evaluation.

### Source hints

Each new fix hint is its exact sentence below followed by one space and
"Sources: <RFC URL> <Gmail URL> <Yahoo URL>" with single spaces and no punctuation
after the URLs. Fixtures pin complete strings.

Gmail URL: https://support.google.com/mail/answer/81126
Yahoo URL: https://senders.yahooinc.com/best-practices/

| Rule | Exact failure sentence | RFC URL |
|---|---|---|
| one_click | Provide one HTTPS List-Unsubscribe list and one exact List-Unsubscribe-Post: List-Unsubscribe=One-Click value. This structural check does not verify DKIM header coverage or endpoint operation. | https://www.rfc-editor.org/rfc/rfc8058#section-3 |
| message_id | Provide exactly one Message-ID using the supported current RFC 5322 syntax; obsolete-only forms are unsupported by this checklist. | https://www.rfc-editor.org/rfc/rfc5322#section-3.6.4 |
| date | Provide exactly one calendar-valid Date using current RFC 5322 syntax and a numeric zone; obsolete-only forms are unsupported by this checklist. | https://www.rfc-editor.org/rfc/rfc5322#section-3.6.1 |
| plain_text | Include a sender-supplied text/plain body. This is checklist advice, not proof of equivalent content or an unconditional provider mandate. | https://www.rfc-editor.org/rfc/rfc2046#section-5.1.4 |
| dmarc_presence | Publish a valid applicable DMARC policy; p=none satisfies this presence check. | https://www.rfc-editor.org/rfc/rfc7489#section-6.1 |
| from_alignment | Align a DKIM d= or SPF MAIL FROM identity with an unambiguous RFC5322.From domain under the applicable DMARC policy. | https://www.rfc-editor.org/rfc/rfc7489#section-3.1 |
| authenticated_sender | Supply a passing DKIM or SPF result aligned with RFC5322.From. This scoped check does not certify every provider authentication requirement. | https://www.rfc-editor.org/rfc/rfc7489#section-3.1 |

Unknowns use the same sources and these exact sentences:
- DMARC: "Not evaluated: trusted applicable DMARC policy evidence is unavailable."
- Alignment: "Not evaluated: trusted From identity or DKIM/SPF alignment evidence is incomplete."
- Authentication: "Not evaluated: trusted From identity or passing-and-aligned DKIM/SPF evidence is incomplete."

### Analysis bounds and errors

Preserve four worker-held jobs, 128 weak cache slots, 256 findings and 65,536
compact JSON report bytes. Inclusive new limits: 8,388,608 raw bytes; 65,536
bytes per header block; 262,144 aggregate header bytes; 256 outer MIME entities
(root counts); multipart depth 32 (root zero). Header counts include line endings
and blank separator; aggregate includes root. Embedded messages are opaque leaves.
Attachment containers still undergo structural/budget checks.

Enforce limits before unbounded allocation, recursion or decoding. A bounded
metadata walk consumes the complete outer tree and closing boundaries; do not
stop at the first plain part. Preamble/epilogue count toward raw bytes. Missing
header/body separator, ambiguous content-type/disposition, invalid boundaries,
unsupported encoded multipart or incomplete classification yields existing
sanitized AnalysisError::Failed. Individual invalid checklist header values
yield findings instead. Partial trees cannot yield a clean report or guessed
plain-text absence. Leaf media type requires no body transfer decoding.
Ordinary failures remain uncached; accepted mail stays available. These are
analysis limits, not SMTP admission, total RSS or latency guarantees.

## Panel adjudication — 2026-10-26

Adopt both positions' single report/evaluator/cache/wait owners, unchanged
adapters, version bump and retained demo. Reject frameworks, profiles, DNS work
and runtime services; accept cold analysis and recomputation costs.

Rule for robustness's distinct unknown IDs and error severity against simplicity's
same-ID warnings. Pay three extra stable IDs and no production qualifying mail
even at error threshold. Preserve predicate semantics instead of adding exceptions.
Choose robustness's error structural headers and warning plain-text policy over
simplicity's warning/info; these are checklist severities, not provider claims.

Rule for robustness's per-mechanism facts against simplicity's three precombined
booleans. Pay a larger snapshot and truth-table matrix to preserve the distinction
between alignment and authenticated alignment, without implementing #255.

Adopt simplicity's narrow modern syntax and presence-only body scope; require
robustness's full-consumption, calendar/weekday and attachment ancestry proofs.
Reject general legacy syntax, content equivalence and global uniqueness. Some
obsolete legal mail is flagged; a plain body may be empty or useless.

Rule for robustness's explicit structural budgets against a raw ceiling plus
unchecked library parse. Adopt a small metadata-only walk in the existing module,
reusing bounded primitives only with complete-consumption proof; reject a second
general MIME renderer/parser. Pay limit fixtures and analysis failure on large or
indeterminate accepted mail. Choose duplicate occurrence 1 over simplicity's
aggregate 0; pay another location case for a concrete source witness.

Preserve crash/commit, chaos, shutdown, prune and webhook behavior. No outbox,
stronger durability, separate executor or cache liveness recheck. Selected reads
may finish after deletion; fresh lookups cannot. Shared pool/OS contention remains.

## Acceptance Criteria

1. AC1: All seven logical rules have passing and evaluated-failing real SMTP/report
   fixtures with exact IDs, severities, locations and complete source hints.
   Auth pairs evaluate immutable facts, not inserted Findings; pass means neither
   failure nor unknown ID for that logical rule.
2. AC2: One-click wire fixtures cover HTTPS, HTTPS+mailto, mailto-only, missing/
   wrong Post, folding/case-varied names, duplicates, malformed URI/list and
   trailing junk. No target is contacted.
3. AC3: Identity/date wire pairs and measured inline tables cover absence,
   duplicates, controls/binary, CFWS/folding, multiple IDs, literals, impossible
   dates, optional seconds/60, weekday mismatch, zones and obsolete syntax.
4. AC4: Plain-text wire pairs cover default/empty/nested/encoded bodies versus
   HTML-only, attachments/ancestry, embedded messages and digest defaults.
   Raw/body/links/codes are identical before and after cold/warm reports.
5. AC5: Truth tables prove independent policy (including p=none), alignment,
   authentication, false/unknown mixtures and a winning aligned pass. Production
   wire reports stay unknown even with spoofed Authentication-Results/DKIM-Signature.
6. AC6: At-limit and one-over cases for every analysis budget prove complete
   analysis or sanitized failure. Malformed/truncated MIME never yields partial
   success. Report/UI errors preserve #252; SMTP-accepted raw remains retrievable.
7. AC7: REST/current MCP HTTP/stdio match, UI escapes hints, cold/warm results agree.
   Predicates prove all-threshold unknown exclusion, exact zero, minimum-one
   waits and existing deadline/zero-timeout/failure behavior; no #153 prerequisite.
8. AC8: Owner tests prove full observation/config/version identity and mail binding,
   zero analysis on SMTP/seed/restore, worker bounds through cancellation and SMTP
   success while analysis is held. Persistence, oldest-prune, burst, chaos and
   subscribe-before-scan regressions remain green.
9. AC9: Living docs explain scope, sources, unsupported syntax, bounds and
   thresholds. 0089/0085 stay proposed; accepted decisions, historical specs,
   routing and dependency/license policy remain intact.
10. AC10: Committed implementation passes all local house gates, exact zero-missed
    coverage/selftest, pinned compilation and publish/seal proofs with recorded
    commands/exits/hashes. CI owns macOS, network RustSec, chart/Helm, Docker,
    privileged port-80 and tag proofs; operator owns settings, acceptance,
    sealing and publication. These are not local proof claims.

Design is not implementation completion. Chief commits exactly spec, plan,
tasks and proposed 0089; deterministic artifact validation is separate.
