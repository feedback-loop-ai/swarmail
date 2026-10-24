# 0062 — Refuse unsupported queries before native operations

- **Status:** proposed — acceptance belongs to the operator.
- **Date:** 2026-10-24
- **Scope:** Epic #192, issue #197.
- **Spec:** [strict native queries](../../specs/strict-native-queries/spec.md).
- **Numbering:** 0062 follows the highest existing record, 0061, on this branch; historical gaps are not reused.
- **Extends:** accepted [0044 §3](0044-compat-validation.md) and [0048 §§4–5](0048-rest-webhook-validation.md)'s method-specific closed query posture.
- **Preserves:** 0001–0005; [0011](0011-shim-delete-contract.md)'s selective body deletion and explicit global wipe; ordered mutation/refusal/shutdown; 0025/0029 domain admission; 0041 redaction; 0048 body mappings and compat tolerance.

## Context

Native handlers silently ignore queries. `inbox=a` on delete-all wipes all inboxes; a subject selector on inbox clear clears everything there. Configuration DELETEs likewise ignore apparent selectors. Broad documentation obscures the mismatch. Callers need refusal rather than success over a larger scope than they apparently intended.

Returning 400 changes a public contract. This compatible extension is recorded as proposed; it neither rewrites accepted text nor grants acceptance or weakens frozen guarantees.

## Proposed ruling

1. **Close ten empty domains.** Zero keys on DELETE `/api/v1/inboxes/{inbox}/messages`; DELETE `/api/v1/delete-all`; GET and DELETE `/api/v1/messages/{id}`; GET `/api/v1/messages/{id}/raw`; GET and DELETE `/api/v1/chaos`; GET and DELETE `/api/v1/webhooks`; POST `/api/v1/inboxes/{inbox}/seed`. Delete-all is native despite its module. Absent/empty queries retain behavior.
2. **Reuse parsing and errors.** Local ViewQuery/parse_query checks refuse every decoded key, including empty-valued, bare, encoded, repeated and empty keys. Preserve current malformed-escape/separator decoding. Return native JSON 400 naming literal method/route template and decoded key; omit values, whole URIs and stored credentials. Preserve JSON escaping. No all-key enumeration or precedence among keys; sensitive text deliberately used as a key remains named.
3. **Refuse before domain work.** Parse before lookup/rendering, store mutation, configuration access/replacement and seed construction/insertion. No rollback. Keep Origin/Host and path extraction ahead of handlers and inbox-clear admission ahead of parsing. Bad queries precede message 404 and commit 503; valid queries keep those errors.
4. **Preserve seed body precedence.** Captured extraction/json_body remain first: syntax 400, media 415, size 413 and named native data-shape 400 beat bad query. Then parse query before offloaded construction/domain validation and insertion. Deserializable domain-invalid body plus unknown query yields query refusal. Accept bounded body extraction before refusal; no new global work bound.
5. **Keep scope method-specific.** Preserve all existing closed sets, caps, null/omission and inbox rules, assert's typed inert uncapped limit, compat list/search tolerance and strict latest. Chaos/webhook PUT, selective DELETE `/api/v1/messages`, other compat aliases, MCP, health/metrics and UI/docs are excluded. Selective DELETE keeps its nonempty ids body, with no query selector.
6. **Publish and prove no effects.** README, llms and OpenAPI name each empty domain and 400, preserve path parameters/responses and document compatibility/seed precedence. Exact IDs/records/raw, full chaos/webhook configuration including credentials, seed inbox/counters and a failing-commit/restart fixture prove preservation. Retain redaction. Use real servers and existing owned handles, no new introspection endpoint.

## Alternatives and consequences

Adopt both panels' local calls and robustness's exact-state, mixed-defect and durable failure-path evidence. Reject counts-only or redacted-only proof: the saved test work cannot rule out wrong-record deletion or hidden credential loss.

Choose simplicity's existing complete Webhooks handle over new receiver/TLS credential tests for this admission-only change. Cost: no new end-to-end credential delivery proof; existing delivery tests retain that responsibility. Choose existing decoding with robustness's edge probes over a new grammar. Preserve seed body errors over earliest query rejection; bounded extraction remains.

Reject global middleware, custom extractors, duplicate query structs, filters, confirmation/force switches, rollback, new storage operations and dependencies. Local repetition buys explicit ordering. Junk-query callers must remove keys; selective removal uses the existing body-based operation. Excluded writers retain their current posture and cannot be advertised as repaired.

Synchronous acceptance, subscribe-before-check, oldest-only pruning and ingest extraction stay untouched. No durable webhook guarantee, exactly-once delivery, configuration transaction, global concurrency budget or new SMTP lifetime bound is implied.

## Proof and decision lifecycle

Spec AC1–AC10 and paired tasks require red-to-green wire proof, all ten controls, exact state, precedence/compatibility and method-aware documentation tests plus all house-rules gates. Artifact validation cannot prove behavior.

Only this proposed record and three spec artifacts belong in the design commit. Implementation indexes 0062 as proposed. Acceptance/amendment markers await the operator. Compatible work may proceed under house rules; discovered contradictions wait for the appropriate accepted amendment.
