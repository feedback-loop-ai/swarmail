# 0041 — Webhook boundary: valid headers, direct delivery, shared bytes and observable loss

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance belongs to the operator.
- **Date:** 2026-10-18
- **Amends:** [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
  §5 secret validity, redacted configuration echoes and shutdown observability.
- **Scope:** run 18, issues #75, #76, #97, #98;
  [specification](../../specs/webhook-boundary/spec.md).
- **Preserves:** synchronous accept/store (0001), subscribe before check
  (0002), oldest-only pruning (0003), ingest extraction (0005), per-target
  budgets and tracked-commit drain (0029), and shutdown refusal (0024).

## Context

The current validator admits control bytes that the HTTP client refuses.
The shutdown tail exits at a lag indication even when retained mail remains.
Environment proxies receive payloads and secrets despite the configured-host
boundary. URL credentials leak through echoes and logs; request construction
copies the supposedly shared payload. Drop warnings scale with traffic, while
the final aborted count is produced after HTTP has stopped and disappears
without an observable final ledger.

These defects concern webhook delivery and observation. Stored mail is not
made asynchronous or dependent on a webhook's success by this repair.

## Ruling

1. A supplied secret is at most 4,096 UTF-8 octets and parses through the
   HTTP client's `HeaderValue::from_bytes`. Additionally reject leading or
   trailing ASCII SP/HTAB rather than silently accepting values receivers can
   trim. Empty strings, internal HTAB and parser-accepted UTF-8 remain valid.
   One helper constructs the header for validation and delivery; errors name
   the defect without echoing secret data. PUT validates all targets before
   replacing any. This supersedes §5's narrower CR/LF/NUL wording.
2. Delivery clients derive from one boundary builder: rustls, redirects
   disabled, environment proxies disabled. The extra-CA client adds its root
   without changing that policy. Existing root trust, verification, cache
   bounds and retries remain. Basic-auth target URLs are valid. This promises
   no application proxy/redirect hop, not control over DNS or host routing.
3. Public configuration echoes and every webhook URL diagnostic remove all
   userinfo, including username. The `secret` field remains redacted and
   `ca_pem` remains visible. Invalid URL display yields a fixed placeholder;
   validators and underlying error formatting must not leak the original
   URL. Internal target URLs and budget identity remain intact. Queries and
   fragments are unchanged and can expose tokens: do not store credentials
   there. This supersedes §5's “everything else stays verbatim” echo rule;
   operators retain their own original configuration for round trips.
4. A mail admitted to at least one delivery is serialized exactly once from
   a borrowed envelope into shared immutable Bytes. Every target and retry
   request body retains that allocation; no intermediate owned JSON tree or
   per-attempt full-body copy. The existing axum re-export supplies Bytes;
   no dependency/allowlist change is needed. Payload content remains fixed
   through retries and pruning. Payload-backing storage is one serialized
   buffer per distinct live admitted mail, plus reference/task/transport
   overhead; neither global byte bounds nor zero transport copying is claimed.
5. Shutdown keeps the existing stop → serve-task join → drained → dispatcher
   join ordering. The final sweep consumes until Empty/Closed, continuing
   through any number of Lagged reports. Each lag records lost mails, whose
   fan-out is unknown; each retained matching notification records aborted.
   Live and tail lag effects have one owner. Outstanding deliveries are then
   aborted and joined; delivered/exhausted outcomes are not counted twice.
   Embedder-controlled producers must quiesce before declaring drain complete;
   there is no new termination guarantee for an endlessly replenished ring.
6. After sweep and joins, ordinary dispatcher exit logs exactly one info
   event `webhook dispatcher stopped` with all five numeric ledger totals,
   even when zero. Default `serve` logging writes stdout. `/metrics` remains
   the live observation; final shutdown totals are observed in this event or
   a retained embedding ledger snapshot after stop. Do not keep HTTP alive
   or add a metrics listener. A filtered logger, crash, forced second signal
   or outright task abort cannot promise an emitted final event. The inherited
   detached-blocking-commit residual in 0024/0029 is not eliminated: totals
   cover the dispatcher's observed work, not a durable acceptance ledger.
7. Keep every dropped increment, but warn at most once per normalized sink
   key per 60-second monotonic window beginning with its last emitted warning.
   The first refusal warns; successful admissions do not reset suppression.
   State retires with its dead/drained budget. Exhaustion warnings keep their
   existing last-cause policy. Config churn can reset identity; no persistent
   rate limiter is promised.

## Alternatives and consequences

Adopt the panels' local dispatcher repairs and final log, robustness's shared
client builder and unpadded secret domain, and simplicity's Bytes re-export,
whole-userinfo redaction and warning window. The spec records each conflict.

Parser-only secret validation loses to wire fidelity, at the cost of refusing
previously accepted padded values. Duplicated client policy loses to one
owner, at the cost of a small helper. A retained username and separate log
scrubber lose to one redaction policy, sacrificing target identity in logs.
Saturation-episode warnings lose to the requested time window: rapid recoveries
must not restore per-drop spam; the price is a timestamp and no recovery log.
A build count alone cannot prove sharing after serialization, so tests must
inspect request-body allocation identity as well as received bytes and memory.

Reject credential refusal (breaks legitimate Basic auth), proxy opt-in (new
configuration surface), an outbox (new durability contract), a global delivery
budget (couples healthy targets to stalled ones), CPU offload/pooling (new
scheduling states), and HTTP shutdown reordering (reopens tracked-commit drain).
Accepted costs: transient delivery loss, target-count-dependent overhead,
serialization CPU on the dispatcher, query-token visibility and final counts
available in logs rather than a post-drain scrape. Commit precedes publication
and SMTP 250; webhook work is never awaited by acceptance and is not ordered
after the client's receipt of 250.

## Proof and landing

The specification's AC1–AC8 and paired tasks are the proof obligations:
real PUT/header delivery; exact 8/8192 lag-tail accounting; real child-process
proxy and TLS tests; credential-safe echoes/logs with functional Basic auth;
shared request bodies through retries; deterministic 60-second windows;
SIGTERM followed by final stdout totals; all six gates and child-level red/green
regressions. Follow 0020 placement and the run's 0035 coverage-by-effect rule.

Index this decision as proposed with the implementation documentation. Do not
rewrite accepted 0029 or mark this accepted. A dated amendment pointer in 0029
belongs to operator acceptance, not this design seat. This decision and the
three specification files are the only requested design-commit paths.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit. The operator accepted
this record and the later record below on 2026-10-19; the text above stays
as written. The amendment pointer the paragraph above deferred to
acceptance is now in
[0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md).
Each marker uses the later record's own verb and scope.

- *(Clarified/extended by [0048](0048-rest-webhook-validation.md) (accepted
  2026-10-19): §§2/3/6/7 — every `ca_pem` certificate is loaded through an
  uncached delivery-client build at `PUT`, webhook option strings refuse
  `null`, redacted echoes keep their shape, and the dispatcher's final stop
  event and exhaustion causes are observable. Retry count, backoff, budgets,
  redaction and ledger arithmetic do not change. This pointer was missing
  until 2026-10-19.)*
