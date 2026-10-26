# 0089 — Source-citing bulk-sender checklist through the report contract

- **Status:** proposed
- **Date:** 2026-10-26
- **Scope:** Epic #251 issue #257, additive rules through proposed 0085.
- **Authority:** supplied #251 operator ruling and issue #257 task brief.
- **Preserves:** accepted 0001–0005, report/cache/wait ownership and all gates.
- **Number:** 0089 explicitly assigned by this task; absent reserved numbers
  such as 0088 are not reused.

## Proposal at intake

Extend the existing lazy, versioned typed deliverability findings with the
bulk-sender checklist: RFC 8058 one-click List-Unsubscribe/Post structure,
Message-ID, Date, a genuine text/plain alternative, and evidence-dependent
DMARC presence, From-domain alignment and aligned DKIM/SPF authentication.
Every rule has a published source in its fix hint and paired real-protocol
fixtures. The [intake criteria](../../specs/bulk-sender-checklist/intake.md)
state the checkable behavior and complete local gate commands.

Keep the existing report shape and shared collector, canonicalization, bounds,
cache and adapters. Change the rule-set version when rule semantics change.
Missing authentication observations produce explicit not-evaluated findings;
missing evidence and sender-supplied authentication headers never establish
passing authentication or alignment. #255 owns genuine authentication results.
Trusted immutable test observations may exercise the future input boundary on
real servers without adding a public observation-injection surface. Any such
inputs must be included in cache identity before evaluated results are used.

Design must pin stable rule IDs, severities, source hints, duplicate/invalid
header handling, MIME scope/bounds, not-evaluated encoding within the current
finding fields and its existing threshold behavior. It must distinguish a
structural one-click check from RFC 8058's authenticated signing requirements,
and local plain-text checklist advice from an unconditional provider mandate.
The intake does not claim live provider-policy verification or delivery success.

No authentication implementation, DNS query, external scorer, new dependency,
SMTP-path work or extraction relocation is introduced by this proposal.
Compatible implementation may proceed under accepted 0045 §8; a conflict with
an accepted ruling requires operator disposition and weakening a frozen
guarantee remains refused. This record does not accept itself or proposed 0085.
