# 0048 — Validate complete webhook roots and close native REST inputs

- **Status:** proposed — acceptance belongs to the operator.
- **Date:** 2026-10-18.
- **Scope:** epic #120 batch 3, issues #122, #134, #135.
- **Spec:** [REST and webhook validation](../../specs/rest-webhook-validation/spec.md).
- **Proposes amendment to:** accepted [0025](0025-shared-domain-validation.md)
  §4 and [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)'s
  preserved body-envelope posture, only for seed POST and webhook PUT.
- **Clarifies/extends:** 0029 §5's delivery-stack CA validation; proposed
  [0041](0041-webhook-boundary.md) §§2/3/6/7 and
  [0044](0044-compat-validation.md) §3.
- **Preserves:** accepted [0030](0030-loopback-plane-bounded-results.md)
  §3's inert assert limit; 0001–0005 and the existing mutation boundary.

## Context

PEM decoding plus reqwest's lazy certificate wrapper accepts invalid DER;
only constructing the delivery client exercises the root-store rule. Taking
one certificate also hides errors and roots later in a bundle. Shared query
structs accept keys count/assert do not use, while seed and webhook objects
silently discard unknown keys. Webhook option strings still treat null as
omission. The stop-event identifier becomes a tracing field and transport
Display hides the exhaustion cause.

The intake incorrectly labels 0025 and 0029 proposed; their records are
accepted. Its demand for named 400s therefore requires this explicit narrow
amendment, not an assumption that serde annotations suffice. No accepted text
is edited and this document supplies no operator acceptance.

## Proposed ruling

1. **Complete CA validation, one owner.** A shared uncached builder consumes
   every PEM certificate, requires at least one, installs all decoded roots
   into the existing rustls delivery client builder and completes the build.
   Any parse or root/client error refuses with a named `ca_pem` error before
   replacement, including for HTTP targets. The parser's unrelated-section
   behavior stays. PUT drops the temporary client; delivery invokes the same
   builder on a cache miss. Validation takes no delivery-cache lock, changes
   no cache counters/LRU/entries, and makes no DNS or network request.
   All targets validate before the existing single swap. Loadability does not
   promise current/future certificate validity or delivery success. No added
   expiry, hostname, revocation or network-preflight policy is authorized.
2. **Closed bodies and local 400s.** Seed accepts its existing five keys;
   webhook objects accept only `url`, `inbox`, `secret`, `ca_pem`. Optional
   webhook strings default only on omission; null and wrong types refuse.
   On seed POST and webhook PUT, all JSON data errors map to HTTP 400 with
   the native JSON error envelope and named field/path where applicable.
   This includes previously 422 missing-field, wrong-type, null and top-level
   shape refusals. Preserve all non-data extractor refusals, including syntax
   400, missing/wrong media type 415 and body-size 413. Other routes keep their
   existing mapping, including chaos's 422 and typed null-clears exception.
   Refusal precedes construction, insertion or replacement; errors cannot
   disclose credentials or secret values.
3. **Response shape is preserved.** Redacted GET and PUT echoes still contain
   null for absent option strings and withheld secrets; URL userinfo is
   removed and public CA material remains. These diagnostic responses are
   not valid PUT templates. Operators retain original configuration as
   already stated by 0041 §3. Request schemas forbid null; response schemas
   describe it. No serialization omission or new round-trip guarantee.
4. **Closed query sets with one explicit compatibility exception.** Count
   permits only `to/from/subject/since_ms`; assert permits those plus
   `count/limit`; threads, single thread and feed permit no keys. Reuse
   route-named parsing and refuse unknown keys with 400 before reads or
   subscription. Assert is still an immediate exact comparison (default 1,
   legal 0, no count cap, 200/409); its `limit` remains parsed as usize but
   inert and uncapped under accepted 0030 §3. Decline removal of that ruled
   key. Successful feeds retain subscribe-before-scan.
5. **Document compat tolerance accurately.** Compat list/search keep their
   existing open shapes, including the risk that an `inbx` typo selects the
   whole store. Existing strict latest queries remain strict. Replace the
   blanket strict-input claim with named supported surfaces and this explicit
   exception. OpenAPI describes actual query sets and the two body mappings.
6. **Observable shutdown and causes.** The final info event uses
   `webhook dispatcher stopped` as its message and keeps the five totals,
   single-event policy and sweep/join order. Exhaustion retains its per-mail
   warning and existing display prefix, adding available source causes through
   a bounded local formatter. Strip reqwest URL metadata before formatting;
   prove the resulting chain cannot disclose target userinfo or secrets.
   Retry count, backoff, budgets, redaction and ledger arithmetic do not change.

## Alternatives and consequences

Adopt simplicity's full bundle and scoped shape repairs, both panels'
uncached validation and compat documentation alternative, and robustness's
explicit cache isolation and credential-safe cause-chain proof. Reject
robustness's first-certificate-only example: it contradicts the task's
“every decoded CA certificate” requirement. Reject deleting assert limit and
omitting response nulls: they add compatibility breaks beyond the required
input refusal. Their costs are the documented inert key and non-replayable
diagnostic echoes. Reject global JSON normalization: chaos remains 422.

Previously accepted bad/trailing roots, typo keys and webhook nulls now fail.
Wrong-shaped seed/webhook requests change status from 422 to 400. Client
construction is repeated on delivery misses, also after LRU eviction; the
existing request-body bound limits input size, not a promised CPU latency or
global work budget. No new cache, validation framework, dependency, network
probe or persistent configuration is justified by this batch.

## Proof and decision lifecycle

Spec AC1–AC11 and its paired tasks require real PUT/seed/query/HTTPS/binary
regressions, complete-list and credential preservation, later-root delivery,
cache isolation, exact log position and cause/redaction tests, contract pins,
and the full gate inventory. Baseline red receipts and already-landed green
proofs must be distinguished. The awaits registry is a cumulative attribution
cap, not an active subscription limit; no false admission claim is inherited
from the panel.

Only this record and the three spec artifacts belong in the design commit.
Implementation indexes this record as proposed and may add clearly dated
proposal pointers without rewriting accepted text. Acceptance/amendment
markers claiming operator approval must wait for that approval.
