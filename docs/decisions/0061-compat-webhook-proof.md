# 0061 — Compat refusal recognition and extraction repair boundaries

- **Status:** accepted (operator, 2026-10-26)
- **Date:** 2026-10-23
- **Current ruling:** the dated 2026-10-26 addendum below supersedes the draft authority boundaries; the original proposal and D3 notes remain historical.
- **Stories:** #170 and #190 under epic #192; lane D.
- **Specification:** [compat/webhook proof](../../specs/epic192-compat-webhook-proof/spec.md).
- **Proposes amendment to:** accepted [0049](0049-extraction-compat.md) §6, only the refusal-recognition order described in §1 below.
- **Records compatible clarifications to:** 0049 §1, nested attribute URLs and trailing curly punctuation (§2 below).
- **Preserves:** accepted 0001–0005, 0023, 0031, 0040, 0041, 0044, 0048 and 0049 except the explicitly proposed §6 amendment after operator acceptance.
- **Number:** 0061 is the existing proposed number reserved to this lane, retained by D3. Other lanes' reservations are not consumed.

## Context

The original synthesis recorded a dispute over R4.2 authority. The D3 positions both agree on existing owners, actual work/failure proof and the need for an exact operator amendment; the original disagreement is not attributed to this panel. Accepted 0049 §6 explicitly checks one parenthesis, one exclusion marker and one leading double quote in that order. The issue now asks to strip every double quote first, including quotes inside field identifiers. That changes normative recognition and can change 200/zero matches to a named 400. Broad intent in 0044 §7 does not silently amend the later explicit rule.

The chief therefore selects an explicit proposal. This is an additional authority-dependent obligation; all independently compatible #170/#190 work proceeds. Acceptance of this proposal belongs to the operator, not the design council, tests or recipe. Accepted 0045 §8 permits compatible work under a proposal and forbids contradictory implementation before acceptance.

## §1 Proposed narrow amendment: refusal-only quote normalization

For generic containing queries on both Mailpit and MailHog:
1. Trim the query as today and examine each whitespace-delimited token.
2. For refusal recognition only, remove every double quote from that token.
3. Remove at most one leading parenthesis, then at most one leading exclusion marker (- or !), then apply the existing ASCII identifier-colon recognition.
4. Keep the :// URL exception. Name the unsupported field in the existing JSON 400.
5. Preserve the original accepted query for literal execution, existing folding/escaping, per-dialect supported explicit kinds and error precedence. No general grammar, exclusion execution or accepted-query rewrite.

This replaces 0049 §6's ordered single-double-quote recognition only after explicit operator acceptance. Required forms are `"-to:x"`, `"!from:alice"`, `"-subject:a b"`, `to":x` and `"to":x`; both shims must name to/from/subject respectively. Ordinary accepted literals, URLs and supported explicit-kind values retain their contracts.

**Cost:** formerly accepted quoted colon words can newly refuse, and normalization adds bounded-by-input linear work/allocation. A smaller patch retaining ordered wrappers leaves vacuous exclusion assertions. A full parser or semantic normalization would introduce a larger unsupported contract. The selected amendment is the narrow refusal rule the issue requests.

**Authority boundary:** until acceptance, keep current ordered recognition and report R4.2 open. The proposal is not a flag, dormant alternate implementation or permission to land contradictory behavior. Test preparation does not establish acceptance.

## §2 Compatible extraction clarifications

For R4.1, source quotes delimit an attribute region before its one decode. Quote-tolerant URL data applies when the URL begins that decoded value after optional whitespace. Quotes surrounding a URL embedded deeper inside CSS, JavaScript-like text, refresh content or prose are its local delimiters; later URLs must still be visited. Direct URL attributes keep encoded quotes in their URL data. This interprets the existing source-aware URL contract without changing ordinary comments, selected source bodies, HTTP(S) spelling, exact-string deduplication or text-before-HTML order. The inherited dated clarification under accepted 0049 §1 remains with its evidence; never rewrite its body. D3 does not own a fresh edit there.

For R4.5, U+201D/U+2019 join trailing closing punctuation. Remove whole scalars, count removed bytes, preserve interior punctuation and matched brackets/parentheses. This clarifies the existing suffix-punctuation rule, not browser URL validation or broad Unicode normalization.

These compatible repairs may proceed while this record is proposed. They preserve decode-once, linear work and exact counters, fresh-ingest extraction, raw/body truth, 64-code policy and verbatim historical arrays. Historical wrong arrays remain wrong; no migration, re-extraction or write-back is authorized.

## §3 Two independent pending operator questions

R4.3 asks whether independent MIME parts' links change from text-first to source order, using the same transient BodyOrder as codes. Until an exact amendment is authorized, preserve accepted text-first links. The mixed[alternative[HTML verify/code121212], text footer/unsubscribe/code2026] fixture and both orders remain specified in AC11.

R4.4 asks whether quoted attributes in Outlook conditional comments gain source-aware handling, producing only the complete O'Brien verification URL, or an operator decision explicitly retains the known truncated prefix. Until that ruling, preserve ordinary-comment behavior, including the truncated prefix plus whole anchor URL. AC12 retains the full fixture and alternatives.

This record does not choose or accept either amendment. Acceptance of §1 alone cannot be read as acceptance of §3 alternatives. Any operator ruling must identify the exact clause/behavior and be recorded with its own provenance. Do not create another lane's ADR, prebuild alternate APIs or suppress the open criteria.

## §4 Unchanged execution and proof contracts

Moving lowering/compilation into existing awaited offload preserves 0023 ordering and panic symmetry; it adds no size or admission policy. Mailpit's named complex-needle JSON 400 and MailHog's bare staging errors remain distinct. Response lookup, projection, render, copy and serialization remain off-worker, without holding store guards.

Real DNS/TLS errors retain informative bounded causes and no URL userinfo, including username. Basic auth remains internally valid, proxies/redirects disabled and certificate verification enabled. There is no new scrubber framework, dependency, webhook outbox or #178 helper work.

Shim deletion remains per-id ordered commit with a persistent prefix on failure, not atomic rollback. Tests use a real selective SQLite fault and restart/retry. The store edit is only a regression comment; no mutation logic or seam expansion is authorized.

The spec's AC1–AC14 and paired tasks require real protocols, actual construction-site placement, independent entity equality and discriminating mutation-red/restored-green receipts. Coverage stays exact 100%; no excluded production path or artificial test denominator. Full locked house gates, pinned proofs and independent reviews remain. Operator rulings and CI-only evidence are separate from local proof.

## Consequences and delivery

The R4.2 rule remains pending; R4.3/R4.4 remain separately open. This attempt can deliver compatible repairs with truthful residuals under the existing engine's review/security rules; it cannot declare whole #190 or epic completion. No security hold is waived.

The chief commits exactly this proposed record and the reserved spec/plan/tasks. Implementation may append owned dated corrections with actual evidence. Parent commit 874b4a4 already supplies the proposed 0061 index row. Do not edit the index again or infer acceptance from it; historical missing-row failures remain historical and final-tree gates must be rerun. Commit unsigned under accepted 0056, never push. Only deterministic design-check owns the designed verdict.

## D3 continuation clarification — 2026-10-23

This update retains proposed status and the exact unaccepted §1 amendment; it accepts no clause. Both current positions support preserving inherited compatible implementation and requiring discriminating evidence. AC10/R4.2, AC11/R4.3 and AC12/R4.4 remain separately open. Index repair and copied source/evidence identities supply no operator ruling, gate success or policy bypass. Current edit scope is the D3 spec/plan ownership table; earlier implementation's broader file list is historical. Required new unowned edits/mutations go to parent while independent compatible work proceeds.


## Operator acceptance and decisions — 2026-10-26

The operator's supplied “Decision rulings of 2026-10-26”, independently
SHA-256 verified against the workspace evidence packet, accept §1 and resolve
both §3 questions. The operator explicitly directs recording this record as
accepted (operator, 2026-10-26). This is the provenance of acceptance, not a
seat's inference from tests or a proposal. No other decision is accepted here.

1. **§1 / R4.2 accepted; amends 0049 §6.** Both Mailpit and MailHog generic
   containing searches trim as before and examine whitespace tokens. For
   refusal recognition only, remove every double quote, then at most one
   leading parenthesis, then at most one leading exclusion marker (- or !),
   then apply the existing ASCII identifier-colon rule with its :// exception.
   The five required forms in §1 name to/from/subject in JSON HTTP 400.
   Accepted queries execute unchanged. Ordinary literals, URLs, supported
   explicit kinds, case and dialect error precedence keep their contracts.
2. **R4.3 decided: keep text-first links across independent MIME parts.**
   The mixed HTML-verification/text-footer fixture stores unsubscribe then
   verify links, while its codes remain 121212 then 2026 in source order.
   No link BodyOrder wiring or new source-order link contract is authorized.
3. **R4.4 decided: source-aware quoted attributes in Outlook conditional
   comments.** Recognize their source attribute boundaries before one decode.
   The specified O'Brien fixture stores only the complete verification URL;
   the truncated prefix disappears. Ordinary comments retain their decoded
   delimiters and all comment text remains suppressed for code extraction.
   This extends 0049 §1's source-aware handling to the conditional tags,
   without DOM/browser recovery or a general comment-semantic rewrite.

These rulings close the authority questions previously left open; earlier
pending/proposed statements above retain their historical date and scope.
Extraction remains once at fresh ingest (0005); existing stored arrays,
including old truncated prefixes, restore verbatim, with no migration,
read-time fallback or write-back. Frozen 0001–0003 and all other accepted
contracts remain unchanged. The dated 0049 addendum records this amendment.
The implementation and evidence are recorded in the spec's 2026-10-26
addendum and proof ledger; protected verify/review/ship remain engine-owned.
