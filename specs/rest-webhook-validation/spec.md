# REST and webhook validation

Date: 2026-10-18. Design handoff for epic #120, batch 3: #122, #134, #135.
Intended branch: `epic120-rest-webhook`; inspected base: `ffded10`, a descendant
of review baseline `2a325c9` containing batches 1 and 2.
Decision: 0048 (proposed).

## Feature and rationale

Configuration must refuse values that would silently disable delivery, discard
a requested filter, or store a seed the caller did not request. Webhook PUT
must validate every certificate through the delivery TLS client construction
before replacing the target list. Native REST request shapes must describe
their actual supported inputs. Shutdown and exhausted-delivery logs must say
what happened while preserving credential redaction.

This is a semantic change to public input contracts. Proposed decision
[0048 — REST and webhook validation](../../docs/decisions/0048-rest-webhook-validation.md)
records the precise exception to accepted 0025/0029's REST body 422 posture.
It also records complete CA bundles and the newly closed shapes. Acceptance
belongs to the operator; this design does not accept or rewrite older rulings.

## Evidence and synthesis

Read both run-local positions, the complete local issue files
`.brokkr/state/epic120/{120,122,134,135}.md`, and the matching
`batch3-intake-framing.md`. The supplied issue files contain the review
evidence but no separate comments section. `.brokkr/state/tasks/` contains
three older coverage tasks, not this intake; the matching framing is under
`.brokkr/state/epic120/` and carries this run's ID. Run-local evidence is not
part of the design commit.

| Disagreement or proposal | Ruling and cost |
|---|---|
| Both panels share the delivery client builder; robustness's sample retains the first certificate only. | Adopt the shared uncached builder, but adopt simplicity's complete certificate iteration. Reject first-only parsing because the framing explicitly requires every decoded certificate. Formerly ignored invalid trailing certificates now cause refusal; valid later roots also become usable. |
| Cache warming versus build-and-discard validation. | Adopt both panels' side-effect-free validation. PUT takes no delivery-cache lock and changes no entries, LRU order or cache counters. Delivery builds again on a miss, including misses after eviction; no timing or once-per-process promise. |
| Robustness removes assert's inert `limit`; simplicity preserves it. | Preserve the explicitly accepted 0030 §3 exception. Removing it breaks a ruled compatibility contract without an issue acceptance criterion requiring that break. The losing side's cost is one documented, type-checked but ignored key; the intake's blanket “every retained key is honored” is narrowed accordingly. |
| Close compat queries versus narrow the strict-input claim. | Adopt both panels' documentation alternative expressly allowed by #134. List/search retain unknown-key tolerance; a typo such as `inbx` can still broaden the scope. Existing strict latest queries remain strict. Do not claim every native route is closed: name the actual surfaces. |
| Robustness omits absent response fields to preserve GET-to-PUT replay; simplicity preserves null echoes. | Preserve response fields and nulls. 0041 §3 already requires retaining original input for round trips, including credentials removed from URLs. Reject a new replay guarantee and serialization change. Cost: an unedited redacted echo now refuses PUT even for secretless targets; request and response schemas must distinguish nullability. |
| Body errors currently use 422, but intake and #135 require 400. | Adopt simplicity's local mapping on seed POST and webhook PUT only: all JSON data/shape errors become 400 JSON errors. Reject global normalization and changing chaos. Cost: wrong-type/missing-field errors on these two routes also move from 422 to 400; record the amendment explicitly instead of matching serde prose. |
| Final event and cause diagnostics. | Adopt the minimal explicit message and a local shared source-chain formatter. Keep per-mail exhaustion warnings, retries and ledger ordering. Cost: longer diagnostics, with new credential-safety proof obligations. |

Correct two robustness premises: blank webhook inboxes were repaired in
batch **1** (0046), and the 4,096-name awaits registry caps cumulative metric
attribution, not waiter admission. Feed uses `Store::subscribe`, not that
registry. Refusal before subscription remains required, but no registry-slot
leak or active-wait admission limit is claimed.

## Required behavior

### Webhook admission

For any supplied `ca_pem`, including on an HTTP target, consume the existing
PEM certificate iterator to completion, require at least one certificate,
add every decoded root to the existing boundary client builder, and build
successfully using the same TLS stack as delivery. Refuse any certificate
parse/root-loading error, including one after a valid certificate. Preserve
the parser's treatment of unrelated PEM sections and surrounding text; this
does not introduce a general PEM purity rule.

Validation is local: no DNS lookup, network request, hostname check, revocation
query or additional expiry policy. Success means all supplied roots load,
not that a later handshake or delivery will succeed. Keep certificate
verification enabled, built-in roots, no proxies, no redirects, and the
existing bounded delivery-client cache. Validate every target outside the
configuration write lock, then perform the existing single whole-list swap.
No rollback, second cache, prepared-target state or durable outbox is added.

Webhook objects accept exactly `url`, `inbox`, `secret`, `ca_pem`. `url` is
required; the other strings may be omitted but cannot be null or wrong-typed.
Use the existing inbox rule: empty, whitespace-only and control-bearing names
refuse; nonblank padded names remain verbatim. Keep existing URL and secret
validation, including legal empty secrets. Empty `ca_pem` still refuses.
An omitted inbox means every inbox. An empty target array clears the list.

All seed/webhook JSON data errors (unknown keys, explicit null, wrong types,
missing required fields and wrong top-level shape) answer HTTP 400 with
`{"error": "..."}`. Field defects identify the offending field; webhook
data diagnostics retain the available element/field path. Domain errors keep
their existing named 400s and target index. Error wording must not echo secret
values or URL credentials. Preserve existing syntax-error 400, media-type
415, body-size 413 and other non-data extractor behavior. Chaos retains its
existing 422 data-error behavior and per-event null-clears semantics.

Public GET and successful PUT echoes retain their current shape: withheld
`secret` is null, absent `inbox`/`ca_pem` are null, public CA material is visible,
and URL userinfo is removed. These are diagnostic responses, not replayable
configuration. No response may alter the installed credentials.

### Native REST queries and seed

Seed accepts only `to`, `from`, `subject`, `text`, `html`, retaining its current
required/optional value rules. Parse failures occur before construction or
store mutation. The following query sets are closed, using the existing
route-named 400 envelope and numeric parsing:

| Route under `/api/v1/inboxes/{inbox}` | Allowed query keys |
|---|---|
| `/count` | `to`, `from`, `subject`, `since_ms` |
| `/assert` | `to`, `from`, `subject`, `since_ms`, `count`, `limit` |
| `/threads`, `/threads/{key}`, `/feed` | none |

Count honors all filters and never paginates. Assert retains immediate exact
comparison, default count 1, legal count 0, unbounded exact count, 200 on match
and 409 otherwise. Its `limit` retains usize parsing and no page cap or effect;
`timeout_ms` and `offset` refuse. Existing list/await behavior is unchanged.
On the three view routes, admit the inbox then parse the empty query shape
before reading views or constructing the feed; an accepted feed still
subscribes before its first scan.

### Diagnostics and contracts

Ordinary dispatcher shutdown emits exactly one info event whose message is
`webhook dispatcher stopped`, with all five existing numeric totals after
the existing sweep/join sequence. `DISPATCHER_STOPPED` must not be a field.
Exhaustion warnings retain the current display prefix and include available
transport source causes. Remove reqwest's URL before formatting its chain;
neither username, password nor secret may appear in any resulting diagnostic.
Keep four attempts, backoff, budgets, accounting and per-mail warning policy.

OpenAPI and llms.txt must publish the route sets, local 400/422 distinction,
request-versus-response webhook nullability, complete CA loadability rule,
and compat list/search tolerance. Existing strict latest behavior stays
documented; “Strict input everywhere” is removed or accurately qualified.

## Acceptance Criteria

1. **AC1 / #122:** Real PUTs refuse named `ca_pem` errors with 400 for no
   certificate, empty decoded certificate, malformed PEM/base64, and valid
   PEM framing with invalid DER. A valid-first/invalid-second certificate
   bundle and valid-first/malformed-second certificate block also refuse.
2. **AC2 / #122, #135:** Starting with a working target carrying a secret and
   URL credentials, each refused PUT leaves the complete installed list
   unchanged, even when a valid target precedes the invalid one. Compare
   installed values and demonstrate that subsequent real delivery still uses
   the old target and credentials; redacted GET equality alone is insufficient.
3. **AC3 / #122:** A generated CA still enables real HTTPS delivery with
   verification enabled. A two-root bundle can deliver to a server trusted
   only by its second root. Invalid roots refuse for HTTP as well as HTTPS.
   Validation makes no outbound request and does not access/change the CA
   cache; isolated proof covers entries, build/eviction counters and LRU order.
   Existing direct-delivery, cache-cap, retries and credential tests stay green.
4. **AC4 / #135:** `secrt` and each of `inbox:null`, `secret:null`,
   `ca_pem:null` independently yield named 400s without replacing configuration.
   Omission succeeds. Empty/whitespace/control inboxes retain named 400s;
   padded legal names retain their identity. The existing blank-inbox test is
   preservation evidence on `ffded10` and regression evidence against `2a325c9`.
5. **AC5 / #134:** Seed `subjet` yields a named 400 and stores nothing; a
   subsequent count is zero and no new inbox appears. A valid seed still
   produces its requested content through ingest and is queryable on success.
6. **AC6 / #134:** Count rejects `limit`, `offset` and unknown keys; assert
   rejects `timeout_ms`, `offset` and unknown keys; threads, a single thread
   and feed reject every supplied key (including recognized filter names on
   other routes). Errors are 400 and name the key and route. Valid filters
   produce correct counts/assertions. Assert's type-checked inert `limit`,
   count 0, default 1, above-250 exact counts and immediate 200/409 remain.
7. **AC7 / #134:** Refused feed queries create no subscription. No-query
   thread/feed requests still work, including initial SSE snapshot and a
   subsequent real delivery. Existing subscribe-before-scan proofs remain.
8. **AC8 / #134, #135:** Both local JSON mappings cover the data-error classes
   above, with named fields when applicable and no effects; syntax, media
   type and size refusals retain prior behavior. Chaos still refuses a typo
   with 422. Public echo field/null shape and credential redaction stay pinned;
   replaying an echo containing null refuses without replacing the list.
9. **AC9 / #135:** A real binary shutdown proves the exact message position,
   absence of a `DISPATCHER_STOPPED=` field, one event and all five totals.
   A real exhausted connection logs a transport cause beyond `error sending
   request`; injected URL userinfo and secret markers are absent from logs.
10. **AC10 / #134:** Served OpenAPI and llms.txt agree with the parsers on
    exact route key sets, refusal statuses and webhook request/response
    shapes. Compat list/search still accept unknown keys, documented as such;
    latest retains its strict query contract.
11. **AC11 / all:** Every changed behavior has a decisive regression red
    against `2a325c9`'s behavior and green after repair, also red on `ffded10`
    when still defective there. Record existing green preservation proofs
    separately. All required final-tree gates pass with exact coverage and
    no new dependency, coverage exclusion or license-allowlist change.

## Scope boundaries

No changes to synchronous commit before acceptance, subscribe-before-scan,
oldest-only pruning or ingest extraction. Preserve the single-writer guards,
5 s SQLite busy bound, awaits metric-registry cap, envelope sender, exact
extraction pins, webhook plane, bridge, Origin guard, by-effect coverage gate,
startupProbe/chart knobs and republish guard. No persistence, schema migration,
new delivery guarantee, target limit, admission control, expiry warning,
compat query-language work or later epic delivery-policy repair is included.
